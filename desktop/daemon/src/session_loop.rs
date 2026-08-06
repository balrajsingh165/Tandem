//! Supervises the LAN session: connects to the paired phone, resumes the mirror
//! against phone truth, pumps events into the controller, and reconnects with
//! backoff when the link drops. Losing the link degrades the desktop to a stale
//! mirror; it never ends a call (ADR-0007).

use std::time::Duration;

use tandem_core::events::{ControllerOutput, OutboundRequest, PhoneEvent};
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

/// Where the phone lives and how to prove it is the right one. An empty `host`
/// means "find it on the LAN": a phone's DHCP lease changes, so a paired phone is
/// located by device id rather than remembered by address.
#[derive(Debug, Clone)]
pub struct PhoneEndpoint {
    pub device_id: String,
    pub host: String,
    pub port: u16,
    pub pin: PinSource,
}

/// How long each attempt spends looking for the paired phone before backing off.
pub const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

/// Resolves the address to dial for this attempt. Discovery is only a hint; the
/// pinned-key handshake is still what decides whether the peer is the phone.
async fn resolve_endpoint(endpoint: &PhoneEndpoint) -> Result<(String, u16), TransportError> {
    if !endpoint.host.is_empty() {
        return Ok((endpoint.host.clone(), endpoint.port));
    }

    tandem_transport::discovery::find_paired_phone(&endpoint.device_id, DISCOVERY_TIMEOUT)
        .await?
        .map(|phone| (phone.host, phone.port))
        .ok_or_else(|| TransportError::ConnectFailed {
            endpoint: endpoint.device_id.clone(),
            reason: "the paired phone is not advertising on this network".into(),
        })
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
    let (host, port) = resolve_endpoint(endpoint).await?;

    WsTransportClient::connect(
        &host,
        port,
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

/// Commands travelling from the UI to the phone. The IPC service validates and
/// enqueues; the supervisor owns the socket and is the only thing that writes.
pub type CommandSender = tokio::sync::mpsc::UnboundedSender<OutboundRequest>;
pub type CommandReceiver = tokio::sync::mpsc::UnboundedReceiver<OutboundRequest>;

/// The queue between the UI and whichever session is current. Starting a session
/// takes a fresh receiver: intent queued for a session that has since been torn
/// down must not fire against a different phone.
pub struct CommandBus {
    sender: std::sync::Mutex<CommandSender>,
}

impl CommandBus {
    pub fn new() -> (std::sync::Arc<Self>, CommandReceiver) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        (
            std::sync::Arc::new(Self {
                sender: std::sync::Mutex::new(sender),
            }),
            receiver,
        )
    }

    pub fn send(&self, request: OutboundRequest) -> Result<(), ()> {
        self.sender
            .lock()
            .expect("command bus poisoned")
            .send(request)
            .map_err(|_| ())
    }

    /// Replaces the queue and hands the reading end to a new session.
    pub fn reset(&self) -> CommandReceiver {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        *self.sender.lock().expect("command bus poisoned") = sender;
        receiver
    }
}

/// Turns a validated domain request into the frame the phone expects. The domain
/// never touches generated types, so the conversion belongs here (ADR-0009).
pub fn to_payload(request: OutboundRequest) -> Payload {
    use tandem_proto as p;

    match request {
        OutboundRequest::Dial { number, sim_slot } => {
            Payload::DialRequest(p::DialRequest { number, sim_slot })
        }
        OutboundRequest::Answer { call_id } => Payload::AnswerRequest(p::AnswerRequest { call_id }),
        OutboundRequest::Reject { call_id } => Payload::RejectRequest(p::RejectRequest { call_id }),
        OutboundRequest::End { call_id } => Payload::EndRequest(p::EndRequest { call_id }),
        OutboundRequest::SetMuted { muted } => Payload::MuteRequest(p::MuteRequest { muted }),
        OutboundRequest::Hold { call_id } => Payload::HoldRequest(p::HoldRequest { call_id }),
        OutboundRequest::Unhold { call_id } => {
            Payload::UnholdRequest(p::UnholdRequest { call_id })
        }
        OutboundRequest::Merge {
            call_id,
            other_call_id,
        } => Payload::MergeRequest(p::MergeRequest {
            call_id,
            other_call_id,
        }),
        OutboundRequest::SendDtmf { call_id, digits } => {
            Payload::SendDtmfRequest(p::SendDtmfRequest { call_id, digits })
        }
        OutboundRequest::AudioRoute {
            route,
            bt_device_address,
        } => Payload::AudioRouteRequest(p::AudioRouteRequest {
            route: match route {
                AudioRoute::Earpiece => p::AudioRoute::Earpiece as i32,
                AudioRoute::Speaker => p::AudioRoute::Speaker as i32,
                AudioRoute::WiredHeadset => p::AudioRoute::WiredHeadset as i32,
                AudioRoute::Bluetooth => p::AudioRoute::Bluetooth as i32,
            },
            bt_device_address,
        }),
        OutboundRequest::SyncCallLog {
            since_ms,
            max_entries,
            before_ms,
        } => Payload::CallLogSyncRequest(p::CallLogSyncRequest {
            since_ms,
            max_entries,
            before_ms,
        }),
        OutboundRequest::SyncContacts {
            offset,
            max_entries,
        } => Payload::ContactsSyncRequest(p::ContactsSyncRequest {
            offset,
            max_entries,
        }),
        OutboundRequest::Unpair => Payload::UnpairRequest(p::UnpairRequest {
            reason: "removed on the computer".into(),
        }),
    }
}

/// Builds the resume cursor from what the desktop last saw, so the phone can
/// tell whether the mirror is contiguous or must be replaced wholesale.
pub fn resume_cursor(
    store: &Store,
    phone_id: &str,
    mirror: Option<&StateVersion>,
) -> ResumeCursor {
    ResumeCursor {
        last_epoch_id: mirror.map(|v| v.epoch_id.clone()).unwrap_or_default(),
        last_state_seq: mirror.map(|v| v.state_seq).unwrap_or(0),
        last_call_log_version: store.last_call_log_version(phone_id),
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
    commands: &mut CommandReceiver,
    state_path: &std::path::Path,
) -> Result<(), TransportError> {
    let next_id = { app.lock().expect("app mutex poisoned").next_message_id() };
    let phone_id = endpoint.device_id.clone();
    set_connection(link, &phone_id, events, ConnectionStatus::Connecting, "");

    let mut client = connect_once(endpoint, credentials, next_id).await?;
    let session = client.session().clone();

    // The emergency list is per-session: a SIM swap between sessions changes it,
    // so the local pre-check is re-armed on every connect (ADR-0008).
    {
        let mut guard = app.lock().expect("app mutex poisoned");
        guard.adopt_emergency_numbers(&phone_id, session.emergency_numbers.clone());
    }
    set_connection(link, &phone_id, events, ConnectionStatus::Resuming, &session.phone_name);

    let cursor = {
        let mut guard = app.lock().expect("app mutex poisoned");
        let mirror = guard.controller(&phone_id).version().cloned();
        resume_cursor(guard.store(), &phone_id, mirror.as_ref())
    };

    let resumed = client.resume(cursor).await?;
    apply_payload(resumed, app, link, &phone_id, events, state_path);
    set_connection(link, &phone_id, events, ConnectionStatus::Live, &session.phone_name);

    // The mirror is a projection of the phone's log. A cache whose version still
    // matches the phone needs no backfill, so only the newest page is pulled;
    // otherwise the whole log is walked, page by page, below.
    client
        .send_payload(to_payload(OutboundRequest::SyncCallLog {
            since_ms: 0,
            max_entries: SYNC_PAGE_SIZE,
            before_ms: 0,
        }))
        .await?;

    // The address book is what makes dial-by-name possible. It is small enough to
    // re-read per session rather than reconciled, and the first page is what tells
    // the store to replace what it held.
    client
        .send_payload(to_payload(OutboundRequest::SyncContacts {
            offset: 0,
            max_entries: CONTACTS_PAGE_SIZE,
        }))
        .await?;

    // Reading the phone and writing user intent have to share one task, because
    // the socket has a single writer. Commands queued while the link was down are
    // still delivered here, in order.
    // Wi-Fi power save and router idle timeouts drop a silent TCP connection
    // without either end noticing, so the link is kept warm and silence past the
    // dead-peer window is treated as a drop rather than waited on forever.
    let mut keepalive = tokio::time::interval(std::time::Duration::from_secs(
        tandem_transport::HEARTBEAT_INTERVAL_SECS,
    ));
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let dead_peer = std::time::Duration::from_secs(tandem_transport::DEAD_PEER_TIMEOUT_SECS);
    let mut heartbeat_seq: u64 = 0;

    loop {
        tokio::select! {
            biased;

            _ = keepalive.tick() => {
                heartbeat_seq += 1;
                client
                    .send_payload(Payload::Heartbeat(tandem_proto::Heartbeat {
                        seq: heartbeat_seq,
                    }))
                    .await?;
            }

            request = commands.recv() => {
                let Some(request) = request else {
                    return Err(TransportError::Closed);
                };
                client.send_payload(to_payload(request)).await?;
            }

            payload = tokio::time::timeout(dead_peer, client.next_payload()) => {
                // A peer that has said nothing for the dead-peer window is gone,
                // even though the socket still looks open.
                let payload = payload.map_err(|_| TransportError::PeerSilent)??;
                let revoked = matches!(payload, Payload::RevokedEvent(_));

                // A nudge carries no rows, so it has to be answered with a pull
                // before the desktop's history is accurate again.
                if let Payload::CallLogChangedEvent(_) = &payload {
                    let since = {
                        app.lock()
                            .expect("app mutex poisoned")
                            .store()
                            .cursor(&phone_id)
                            .newest_entry_ms
                    };
                    client
                        .send_payload(to_payload(OutboundRequest::SyncCallLog {
                            since_ms: since,
                            max_entries: SYNC_PAGE_SIZE,
                            before_ms: 0,
                        }))
                        .await?;
                }

                // Paging happens here rather than inside apply_payload, because
                // only this task may write to the socket.
                // The phone nudges rather than pushing rows; a pull is what makes
                // the change visible.
                if let Payload::ContactsChangedEvent(_) = &payload {
                    client
                        .send_payload(to_payload(OutboundRequest::SyncContacts {
                            offset: 0,
                            max_entries: CONTACTS_PAGE_SIZE,
                        }))
                        .await?;
                }

                if let Payload::ContactsSyncResponse(response) = &payload {
                    if response.has_more {
                        let held = app
                            .lock()
                            .expect("app mutex poisoned")
                            .store_ref()
                            .contacts(&phone_id)
                            .len();
                        client
                            .send_payload(to_payload(OutboundRequest::SyncContacts {
                                offset: (held + response.entries.len()) as u32,
                                max_entries: CONTACTS_PAGE_SIZE,
                            }))
                            .await?;
                    }
                }

                if let Payload::CallLogSyncResponse(response) = &payload {
                    if let Some(before_ms) = next_page_bound(app, &phone_id, response) {
                        client
                            .send_payload(to_payload(OutboundRequest::SyncCallLog {
                                since_ms: 0,
                                max_entries: SYNC_PAGE_SIZE,
                                before_ms,
                            }))
                            .await?;
                    }
                }

                apply_payload(payload, app, link, &phone_id, events, state_path);

                if revoked {
                    // The phone dropped this desktop, so the stored pairing is
                    // dead: keeping it would leave the UI offering a phone that
                    // will refuse every handshake from now on.
                    forget_phone(app, link, &phone_id, state_path);
                    set_connection(link, &phone_id, events, ConnectionStatus::Idle, "");
                    return Err(TransportError::Revoked("unpaired on the phone".into()));
                }
            }
        }
    }
}

/// Where the next backfill page should stop, or None when the walk is finished.
///
/// The phone answers newest-first, so the oldest row just received is the upper
/// bound for the next request. Paging stops at the retention bound, since rows
/// beyond it would be trimmed the moment they arrived and the walk would never
/// terminate.
fn next_page_bound(
    app: &SharedApp,
    phone_id: &str,
    response: &tandem_proto::CallLogSyncResponse,
) -> Option<i64> {
    if !response.has_more {
        return None;
    }

    let oldest = response
        .entries
        .iter()
        .map(|entry| entry.started_at_ms)
        .min()?;
    if oldest <= 0 {
        return None;
    }

    let held = app
        .lock()
        .expect("app mutex poisoned")
        .store_ref()
        .call_log(phone_id)
        .len();
    if held + response.entries.len() >= crate::store::MIRROR_MAX_ENTRIES {
        return None;
    }

    Some(oldest)
}

/// Discards the persisted pairing after the phone revoked this desktop.
fn forget_phone(
    app: &SharedApp,
    link: &SharedLink,
    phone_id: &str,
    state_path: &std::path::Path,
) {
    {
        let mut guard = app.lock().expect("app mutex poisoned");
        guard.store().remove_phone(phone_id);
        guard.forget_controller(phone_id);
        let _ = guard.store().save(state_path);
    }
    link.lock().expect("link mutex poisoned").forget(phone_id);
}

/// How many call-log rows to pull per request; the phone caps at 200.
pub const SYNC_PAGE_SIZE: u32 = 200;

/// How many contact rows to pull per request; the phone caps at 500.
pub const CONTACTS_PAGE_SIZE: u32 = 500;

/// Applies one inbound payload to the mirror and tells the UI what changed.
fn apply_payload(
    payload: Payload,
    app: &SharedApp,
    link: &SharedLink,
    phone_id: &str,
    events: &EventPublisher,
    state_path: &std::path::Path,
) {
    // The device list is link state, not call state: it survives having no call
    // and is what the UI's route picker is built from.
    if let Payload::AudioDevicesEvent(event) = &payload {
        let devices: Vec<tandem_ipc::api::AudioDeviceView> = event
            .devices
            .iter()
            .map(|device| tandem_ipc::api::AudioDeviceView {
                route: ipc_route(device.route),
                bt_device_address: device.bt_device_address.clone(),
                name: device.name.clone(),
            })
            .collect();

        {
            let mut guard = link.lock().expect("link mutex poisoned");
            let entry = guard.entry(phone_id);
            entry.audio_devices = devices.clone();
            entry.active_bt_device_address = event.active_bt_device_address.clone();
        }
        events.publish(IpcEvent::AudioDevicesChanged {
            devices,
            active_route: ipc_route(event.active_route),
            active_bt_device_address: event.active_bt_device_address.clone(),
        });
        return;
    }

    // Contacts are a directory projection, not call state.
    if let Payload::ContactsSyncResponse(response) = payload {
        let offset = {
            let guard = app.lock().expect("app mutex poisoned");
            guard.store_ref().contacts(phone_id).len() as u32
        };
        let rows: Vec<tandem_core::model::ContactRow> =
            response.entries.into_iter().map(contact_row).collect();
        let count = {
            let mut guard = app.lock().expect("app mutex poisoned");
            // The first page of a fresh sync arrives with nothing held yet, which
            // is what tells the store to replace rather than append.
            guard
                .store()
                .merge_contacts(phone_id, offset, rows, response.directory_version);
            let _ = guard.store().save(state_path);
            guard.store_ref().contacts(phone_id).len()
        };
        events.publish(IpcEvent::ContactsChanged {
            count: count as u32,
        });
        return;
    }

    // Call-log pages are not controller events: they land in the store the UI
    // reads its history from.
    if let Payload::CallLogSyncResponse(response) = payload {
        let rows: Vec<tandem_core::model::CallLogRow> =
            response.entries.into_iter().map(call_log_row).collect();
        {
            let mut guard = app.lock().expect("app mutex poisoned");
            guard.store().merge_call_log(phone_id, rows);
            guard
                .store()
                .set_call_log_version(phone_id, response.log_version);
            let _ = guard.store().save(state_path);
        }
        events.publish(IpcEvent::HistoryChanged {
            log_version: response.log_version,
        });
        return;
    }

    let Some(event) = to_phone_event(payload) else {
        return;
    };

    let outputs = {
        let mut guard = app.lock().expect("app mutex poisoned");
        guard.controller(phone_id).apply_phone_event(event)
    };

    for output in outputs {
        if let ControllerOutput::MirrorUpdated(snapshot) = output {
            events.publish(IpcEvent::CallsChanged {
                calls: snapshot.calls.iter().map(call_view).collect(),
            });
        }
    }
}

/// The wire route enum as the UI's enum. Unknown values fall back to the
/// earpiece, which every phone has.
fn ipc_route(value: i32) -> tandem_ipc::api::AudioRoute {
    use tandem_proto::AudioRoute as P;
    match P::try_from(value) {
        Ok(P::Speaker) => tandem_ipc::api::AudioRoute::Speaker,
        Ok(P::WiredHeadset) => tandem_ipc::api::AudioRoute::WiredHeadset,
        Ok(P::Bluetooth) => tandem_ipc::api::AudioRoute::Bluetooth,
        _ => tandem_ipc::api::AudioRoute::Earpiece,
    }
}

fn contact_row(entry: tandem_proto::ContactEntry) -> tandem_core::model::ContactRow {
    tandem_core::model::ContactRow {
        contact_id: entry.contact_id,
        display_name: entry.display_name,
        number: entry.number,
        label: entry.label,
        starred: entry.starred,
    }
}

fn call_log_row(entry: tandem_proto::CallLogEntry) -> tandem_core::model::CallLogRow {
    tandem_core::model::CallLogRow {
        entry_id: entry.entry_id,
        number: entry.number,
        display_name: entry.display_name,
        started_at_ms: entry.started_at_ms,
        duration_seconds: entry.duration_seconds,
        sim_slot: entry.sim_slot,
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

/// Records the link state and tells the UI, so a desktop that comes up after the
/// window was opened does not sit showing a stale "not connected".
fn set_connection(
    link: &SharedLink,
    phone_id: &str,
    events: &EventPublisher,
    connection: ConnectionStatus,
    phone_name: &str,
) {
    {
        let mut guard = link.lock().expect("link mutex poisoned");
        let entry = guard.entry(phone_id);
        entry.connection = Some(connection);
        if !phone_name.is_empty() {
            entry.phone_name = phone_name.to_string();
        }
    }
    events.publish(IpcEvent::ConnectionChanged { connection });
}

/// Keeps a session alive across drops. A transient failure backs off and retries;
/// a trust or version failure stops, because retrying cannot fix a wrong key.
pub async fn supervise(
    endpoint: PhoneEndpoint,
    credentials: IdentityCredentials,
    app: SharedApp,
    link: SharedLink,
    events: EventPublisher,
    mut commands: CommandReceiver,
    state_path: std::path::PathBuf,
) {
    let mut backoff = Backoff::new();

    loop {
        match run_one_session(
            &endpoint,
            &credentials,
            &app,
            &link,
            &events,
            &mut commands,
            &state_path,
        )
        .await {
            Ok(()) => backoff.reset(),
            Err(error) => match classify(&error) {
                AttemptOutcome::Fatal(fatal) => {
                    set_connection(&link, &endpoint.device_id, &events, ConnectionStatus::Terminated, "");
                    events.publish(IpcEvent::Revoked {
                        reason: fatal.to_string(),
                    });
                    return;
                }
                AttemptOutcome::Ended => {}
            },
        }

        set_connection(&link, &endpoint.device_id, &events, ConnectionStatus::Backoff, "");
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
        let cursor = resume_cursor(&Store::default(), "phone-1", None);
        assert_eq!(cursor.last_epoch_id, "");
        assert_eq!(cursor.last_state_seq, 0);
    }

    #[test]
    fn the_cursor_carries_the_last_seen_version_and_log() {
        let mut store = Store::default();
        store.set_call_log_version("phone-1", 9);
        let version = StateVersion {
            epoch_id: "epoch-1".into(),
            state_seq: 42,
        };

        let cursor = resume_cursor(&store, "phone-1", Some(&version));
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
