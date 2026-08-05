//! Supervises the LAN session: connects to the paired phone, resumes the mirror
//! against phone truth, pumps events into the controller, and reconnects with
//! backoff when the link drops. Losing the link degrades the desktop to a stale
//! mirror; it never ends a call (ADR-0007).

use std::time::Duration;

use tandem_core::events::{ControllerOutput, PhoneEvent};
use tandem_core::model::{AudioRoute, Call, CallDirection, CallSnapshot, CallState, StateVersion};
use tandem_crypto::IdentityCredentials;
use tandem_ipc::api::{ConnectionStatus, IpcEvent};
use tandem_ipc::server::EventPublisher;
use tandem_proto::envelope::Payload;
use tandem_transport::client::{ClientIdentity, TransportClient, WsTransportClient};
use tandem_transport::error::TransportError;
use tandem_transport::reconnect::{Backoff, ResumeCursor};
use tandem_transport::tls::{client_config, PinSource};

use crate::ipc_service::{SharedApp, SharedLink};
use crate::store::Store;

/// Where the phone lives and how to prove it is the right one.
#[derive(Debug, Clone)]
pub struct PhoneEndpoint {
    pub host: String,
    pub port: u16,
    pub pin: PinSource,
}

/// Outcome of one connection attempt, so the supervisor can decide whether to
/// keep trying without inspecting error internals.
#[derive(Debug, PartialEq, Eq)]
pub enum AttemptOutcome {
    /// The session ran and then ended; retry after backoff.
    Ended,
    /// Trust or version failure; retrying cannot fix it.
    Fatal(TransportError),
}

/// Decides whether the supervisor keeps trying after a failure.
pub fn classify(error: &TransportError) -> AttemptOutcome {
    if error.is_retryable() {
        AttemptOutcome::Ended
    } else {
        AttemptOutcome::Fatal(error.clone())
    }
}

/// Opens one session and returns it ready to pump. Separated from the retry loop
/// so a single attempt can be driven directly in tests.
pub async fn connect_once(
    endpoint: &PhoneEndpoint,
    credentials: &IdentityCredentials,
    next_message_id: u64,
) -> Result<WsTransportClient, TransportError> {
    let chain = vec![rustls_pki_types::CertificateDer::from(
        credentials.identity.cert_der.clone(),
    )];
    let key = rustls_pki_types::PrivateKeyDer::try_from(credentials.key_der.clone())
        .map_err(|e| TransportError::TlsHandshake(e.to_string()))?;

    let tls = client_config(&endpoint.pin, chain, key)?;

    WsTransportClient::connect(
        &endpoint.host,
        endpoint.port,
        tls,
        ClientIdentity {
            device_id: credentials.identity.device_id.clone(),
            client_name: credentials.identity.display_name.clone(),
            bt_adapter_address: String::new(),
        },
        next_message_id,
    )
    .await
}

/// Builds the resume cursor from what the desktop last saw, so the phone can
/// tell whether the mirror is contiguous or must be replaced wholesale.
pub fn resume_cursor(store: &Store, mirror: Option<&StateVersion>) -> ResumeCursor {
    ResumeCursor {
        last_epoch_id: mirror.map(|v| v.epoch_id.clone()).unwrap_or_default(),
        last_state_seq: mirror.map(|v| v.state_seq).unwrap_or(0),
        last_call_log_version: store.last_call_log_version(),
    }
}

/// Converts an inbound payload into a controller event. Returns None for frames
/// the controller has no opinion about, such as acks.
pub fn to_phone_event(payload: Payload) -> Option<PhoneEvent> {
    match payload {
        Payload::CallStateChangedEvent(event) => {
            Some(PhoneEvent::CallStateChanged(snapshot_from(event.snapshot?)))
        }
        Payload::ResumeResponse(response) if response.snapshot_included => Some(
            PhoneEvent::SnapshotReplaced(snapshot_from(response.snapshot?)),
        ),
        Payload::IncomingCallEvent(event) => Some(PhoneEvent::IncomingCall {
            call: call_from(event.call?),
            version: StateVersion {
                epoch_id: event.epoch_id,
                state_seq: event.state_seq,
            },
        }),
        Payload::AudioRouteChangedEvent(event) => Some(PhoneEvent::AudioRouteChanged {
            route: route_from(event.route),
            bt_device_address: event.bt_device_address,
            version: StateVersion {
                epoch_id: event.epoch_id,
                state_seq: event.state_seq,
            },
        }),
        Payload::CallLogChangedEvent(event) => Some(PhoneEvent::CallLogChanged {
            log_version: event.log_version,
        }),
        Payload::RevokedEvent(event) => Some(PhoneEvent::Revoked {
            reason: event.reason,
        }),
        _ => None,
    }
}

fn snapshot_from(proto: tandem_proto::CallSnapshot) -> CallSnapshot {
    CallSnapshot {
        version: StateVersion {
            epoch_id: proto.epoch_id,
            state_seq: proto.state_seq,
        },
        calls: proto.calls.into_iter().map(call_from).collect(),
        audio_route: route_from(proto.audio_route),
        microphone_muted: proto.microphone_muted,
        bt_route_address: proto.bt_route_address,
    }
}

fn call_from(proto: tandem_proto::CallInfo) -> Call {
    Call {
        call_id: proto.call_id,
        state: state_from(proto.state),
        direction: if proto.direction == tandem_proto::CallDirection::Outgoing as i32 {
            CallDirection::Outgoing
        } else {
            CallDirection::Incoming
        },
        remote_number: proto.remote_number,
        remote_display_name: proto.remote_display_name,
        started_at_ms: proto.started_at_ms,
        is_conference: proto.is_conference,
        can_hold: proto.can_hold,
        can_merge: proto.can_merge,
        is_emergency: proto.is_emergency,
        sim_slot: proto.sim_slot,
    }
}

fn state_from(value: i32) -> CallState {
    use tandem_proto::CallState as P;
    match P::try_from(value) {
        Ok(P::Dialing) => CallState::Dialing,
        Ok(P::Ringing) => CallState::Ringing,
        Ok(P::Active) => CallState::Active,
        Ok(P::Holding) => CallState::Holding,
        Ok(P::Disconnecting) => CallState::Disconnecting,
        Ok(P::Disconnected) => CallState::Disconnected,
        _ => CallState::Connecting,
    }
}

fn route_from(value: i32) -> AudioRoute {
    use tandem_proto::AudioRoute as P;
    match P::try_from(value) {
        Ok(P::Speaker) => AudioRoute::Speaker,
        Ok(P::WiredHeadset) => AudioRoute::WiredHeadset,
        Ok(P::Bluetooth) => AudioRoute::Bluetooth,
        _ => AudioRoute::Earpiece,
    }
}

/// Delay before the next attempt, advancing the backoff. `entropy` supplies the
/// jitter so the caller owns randomness and this stays deterministic.
pub fn next_delay(backoff: &mut Backoff, entropy: f64) -> Duration {
    let delay = backoff.jittered(entropy);
    backoff.advance();
    delay
}

/// Jitter source. The clock is sufficient here: the goal is only to stop a fleet
/// of desktops retrying in lockstep, not to resist prediction.
fn clock_entropy() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    f64::from(nanos % 1_000) / 1_000.0
}

/// Runs one session to completion: connect, reconcile, then pump events until
/// the link ends.
pub async fn run_one_session(
    endpoint: &PhoneEndpoint,
    credentials: &IdentityCredentials,
    app: &SharedApp,
    link: &SharedLink,
    events: &EventPublisher,
) -> Result<(), TransportError> {
    let next_id = { app.lock().expect("app mutex poisoned").next_message_id() };
    set_connection(link, ConnectionStatus::Connecting, "");

    let mut client = connect_once(endpoint, credentials, next_id).await?;
    let session = client.session().clone();

    // The emergency list is per-session: a SIM swap between sessions changes it,
    // so the local pre-check is re-armed on every connect (ADR-0008).
    {
        let mut guard = app.lock().expect("app mutex poisoned");
        guard.adopt_emergency_numbers(session.emergency_numbers.clone());
    }
    set_connection(link, ConnectionStatus::Resuming, &session.phone_name);

    let cursor = {
        let mut guard = app.lock().expect("app mutex poisoned");
        let mirror = guard.controller().version().cloned();
        resume_cursor(guard.store(), mirror.as_ref())
    };

    let resumed = client.resume(cursor).await?;
    apply_payload(resumed, app, events);
    set_connection(link, ConnectionStatus::Live, &session.phone_name);

    loop {
        let payload = client.next_payload().await?;
        let revoked = matches!(payload, Payload::RevokedEvent(_));
        apply_payload(payload, app, events);

        if revoked {
            set_connection(link, ConnectionStatus::Terminated, &session.phone_name);
            return Err(TransportError::Revoked("unpaired on the phone".into()));
        }
    }
}

/// Applies one inbound payload to the mirror and tells the UI what changed.
fn apply_payload(payload: Payload, app: &SharedApp, events: &EventPublisher) {
    let Some(event) = to_phone_event(payload) else {
        return;
    };

    let outputs = {
        let mut guard = app.lock().expect("app mutex poisoned");
        guard.controller().apply_phone_event(event)
    };

    for output in outputs {
        if let ControllerOutput::MirrorUpdated(snapshot) = output {
            events.publish(IpcEvent::CallsChanged {
                calls: snapshot.calls.iter().map(call_view).collect(),
            });
        }
    }
}

fn call_view(call: &Call) -> tandem_ipc::api::CallView {
    tandem_ipc::api::CallView {
        call_id: call.call_id.clone(),
        state: match call.state {
            CallState::Connecting => tandem_ipc::api::CallState::Connecting,
            CallState::Dialing => tandem_ipc::api::CallState::Dialing,
            CallState::Ringing => tandem_ipc::api::CallState::Ringing,
            CallState::Active => tandem_ipc::api::CallState::Active,
            CallState::Holding => tandem_ipc::api::CallState::Holding,
            CallState::Disconnecting => tandem_ipc::api::CallState::Disconnecting,
            CallState::Disconnected => tandem_ipc::api::CallState::Disconnected,
        },
        remote_number: call.remote_number.clone(),
        remote_display_name: call.remote_display_name.clone(),
        started_at_ms: call.started_at_ms,
        is_conference: call.is_conference,
        can_hold: call.can_hold,
        can_merge: call.can_merge,
        is_emergency: call.is_emergency,
    }
}

fn set_connection(link: &SharedLink, connection: ConnectionStatus, phone_name: &str) {
    let mut guard = link.lock().expect("link mutex poisoned");
    guard.connection = Some(connection);
    if !phone_name.is_empty() {
        guard.phone_name = phone_name.to_string();
    }
}

/// Keeps a session alive across drops. A transient failure backs off and retries;
/// a trust or version failure stops, because retrying cannot fix a wrong key.
pub async fn supervise(
    endpoint: PhoneEndpoint,
    credentials: IdentityCredentials,
    app: SharedApp,
    link: SharedLink,
    events: EventPublisher,
) {
    let mut backoff = Backoff::new();

    loop {
        match run_one_session(&endpoint, &credentials, &app, &link, &events).await {
            Ok(()) => backoff.reset(),
            Err(error) => match classify(&error) {
                AttemptOutcome::Fatal(fatal) => {
                    set_connection(&link, ConnectionStatus::Terminated, "");
                    events.publish(IpcEvent::Revoked {
                        reason: fatal.to_string(),
                    });
                    return;
                }
                AttemptOutcome::Ended => {}
            },
        }

        set_connection(&link, ConnectionStatus::Backoff, "");
        events.publish(IpcEvent::ConnectionChanged {
            connection: ConnectionStatus::Backoff,
        });
        tokio::time::sleep(next_delay(&mut backoff, clock_entropy())).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tandem_proto::{CallInfo, CallSnapshot as ProtoSnapshot, ResumeResponse};

    #[test]
    fn transient_failures_keep_the_supervisor_retrying() {
        assert_eq!(classify(&TransportError::PeerSilent), AttemptOutcome::Ended);
        assert_eq!(classify(&TransportError::Closed), AttemptOutcome::Ended);
    }

    /// A wrong key or unsupported version cannot be fixed by waiting.
    #[test]
    fn trust_failures_stop_the_supervisor() {
        assert!(matches!(
            classify(&TransportError::PinMismatch),
            AttemptOutcome::Fatal(_)
        ));
        assert!(matches!(
            classify(&TransportError::Revoked("removed".into())),
            AttemptOutcome::Fatal(_)
        ));
    }

    #[test]
    fn an_empty_mirror_resumes_from_zero() {
        let cursor = resume_cursor(&Store::default(), None);
        assert_eq!(cursor.last_epoch_id, "");
        assert_eq!(cursor.last_state_seq, 0);
    }

    #[test]
    fn the_cursor_carries_the_last_seen_version_and_log() {
        let mut store = Store::default();
        store.set_call_log_version(9);
        let version = StateVersion {
            epoch_id: "epoch-1".into(),
            state_seq: 42,
        };

        let cursor = resume_cursor(&store, Some(&version));
        assert_eq!(cursor.last_epoch_id, "epoch-1");
        assert_eq!(cursor.last_state_seq, 42);
        assert_eq!(cursor.last_call_log_version, 9);
    }

    /// A resume that includes a snapshot must replace the mirror, not merge into
    /// it — phone truth wins (ADR-0007).
    #[test]
    fn an_included_snapshot_becomes_a_replace_event() {
        let payload = Payload::ResumeResponse(ResumeResponse {
            status: None,
            snapshot_included: true,
            snapshot: Some(ProtoSnapshot {
                epoch_id: "epoch-2".into(),
                state_seq: 5,
                calls: vec![CallInfo {
                    call_id: "c1".into(),
                    state: tandem_proto::CallState::Ringing as i32,
                    ..Default::default()
                }],
                audio_route: tandem_proto::AudioRoute::Bluetooth as i32,
                microphone_muted: true,
                bt_route_address: "AA:BB".into(),
            }),
            call_log_version: 3,
        });

        match to_phone_event(payload) {
            Some(PhoneEvent::SnapshotReplaced(snapshot)) => {
                assert_eq!(snapshot.version.epoch_id, "epoch-2");
                assert_eq!(snapshot.version.state_seq, 5);
                assert_eq!(snapshot.calls.len(), 1);
                assert_eq!(snapshot.calls[0].state, CallState::Ringing);
                assert_eq!(snapshot.audio_route, AudioRoute::Bluetooth);
                assert!(snapshot.microphone_muted);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    /// A resume with no snapshot means the mirror is contiguous; nothing to apply.
    #[test]
    fn a_resume_without_a_snapshot_produces_no_event() {
        let payload = Payload::ResumeResponse(ResumeResponse {
            status: None,
            snapshot_included: false,
            snapshot: None,
            call_log_version: 3,
        });
        assert!(to_phone_event(payload).is_none());
    }

    #[test]
    fn revocation_reaches_the_controller() {
        let payload = Payload::RevokedEvent(tandem_proto::RevokedEvent {
            reason: "removed on phone".into(),
        });
        assert!(matches!(
            to_phone_event(payload),
            Some(PhoneEvent::Revoked { .. })
        ));
    }

    #[test]
    fn acks_are_not_controller_events() {
        assert!(to_phone_event(Payload::Ack(tandem_proto::Ack { status: None })).is_none());
    }

    #[test]
    fn backoff_grows_between_attempts() {
        let mut backoff = Backoff::new();
        let first = next_delay(&mut backoff, 0.5);
        let second = next_delay(&mut backoff, 0.5);
        assert!(second > first);
    }
}
