//! Implements the IpcApi surface over the live controller and subsystems:
//! translates UI method calls into controller commands and streams state events
//! to connected UIs.

use tandem_core::error::CoreError;
use tandem_ipc::api::{
    AudioRoute as IpcAudioRoute, CallState as IpcCallState, CallView, ConnectionStatus, IpcRequest,
    IpcResponse, StatusResult,
};
use tandem_ipc::error::IpcError;
use tandem_ipc::server::IpcService;

use crate::app::App;
use tandem_core::events::UserCommand;
use tandem_core::model::{AudioRoute, Call, CallState};

/// App state shared between the IPC server and the session supervisor. A
/// std mutex is deliberate: every critical section is a short, synchronous
/// mutation, so no lock is ever held across an await.
pub type SharedApp = std::sync::Arc<std::sync::Mutex<App>>;

/// Connection facts the supervisor reports for the UI to render.
#[derive(Debug, Clone, Default)]
pub struct LinkState {
    pub connection: Option<ConnectionStatus>,
    pub phone_name: String,
}

pub type SharedLink = std::sync::Arc<std::sync::Mutex<LinkState>>;

/// What the service needs to start a pairing attempt on the user's behalf.
/// Pairing is long-running, so the request returns immediately and progress
/// arrives as `pairingProgress` events.
#[derive(Clone)]
pub struct PairingLauncher {
    pub credentials: tandem_crypto::IdentityCredentials,
    pub events: tandem_ipc::server::EventPublisher,
    pub app: SharedApp,
    pub state_path: std::path::PathBuf,
}

/// Bridges the UI-facing API to the domain, keeping every policy decision in
/// core rather than in this translation layer.
pub struct DaemonIpcService {
    app: SharedApp,
    link: SharedLink,
    pairing: Option<PairingLauncher>,
    /// The offer currently on screen. Two live offers would race for the phone's
    /// single pairing window, so showing a new code cancels the old one.
    offer_task: Option<tokio::task::JoinHandle<()>>,
    /// Set while a phone waits to be approved on this desktop; taking it is what
    /// releases the pairing exchange.
    pending_approval: PendingApproval,
}

/// The verdict slot the UI fills in and the pairing task waits on.
pub type PendingApproval = std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>;

impl DaemonIpcService {
    pub fn new(app: SharedApp, link: SharedLink) -> Self {
        Self {
            app,
            link,
            pairing: None,
            offer_task: None,
            pending_approval: PendingApproval::default(),
        }
    }

    pub fn with_pairing(mut self, launcher: PairingLauncher) -> Self {
        self.pairing = Some(launcher);
        self
    }

    pub fn shared_app(&self) -> SharedApp {
        self.app.clone()
    }

    /// Parses the QR payload and runs pairing in the background, persisting the
    /// phone identity on success so the pairing survives a restart.
    fn start_pairing(&self, qr_payload: String) -> Result<IpcResponse, IpcError> {
        let launcher = self.pairing.clone().ok_or(IpcError::Internal)?;

        let invitation = tandem_pairing::QrPayload::parse(&qr_payload)
            .map_err(|e| IpcError::PairingFailed(e.to_string()))?;

        tokio::spawn(async move {
            let mut flow = tandem_pairing::flow::PairingFlow::new(invitation);
            let credentials = tandem_pairing::flow::DesktopCredentials {
                name: launcher.credentials.identity.display_name.clone(),
                platform: platform_name().to_string(),
                cert_der: launcher.credentials.identity.cert_der.clone(),
                key_der: launcher.credentials.key_der.clone(),
            };

            let events = launcher.events.clone();
            let outcome = flow
                .run(&credentials, |state| {
                    events.publish(tandem_ipc::api::IpcEvent::PairingProgress {
                        state: describe(state),
                        short_code: short_code_of(state),
                    });
                })
                .await;

            match outcome {
                Ok(record) => {
                    {
                        let mut guard = launcher.app.lock().expect("app mutex poisoned");
                        guard
                            .store()
                            .set_paired_phone(tandem_core::model::PairedPhone {
                                device_id: record.phone_device_id.clone(),
                                name: record.phone_name.clone(),
                                spki_sha256: record.phone_spki_sha256.to_base64url(),
                                bt_address: record.phone_bt_address.clone(),
                            });
                        let _ = guard.store().save(&launcher.state_path);
                    }
                    launcher
                        .events
                        .publish(tandem_ipc::api::IpcEvent::PairingProgress {
                            state: "accepted".into(),
                            short_code: None,
                        });
                }
                Err(error) => {
                    launcher
                        .events
                        .publish(tandem_ipc::api::IpcEvent::PairingProgress {
                            state: format!("failed: {error}"),
                            short_code: None,
                        });
                }
            }
        });

        Ok(IpcResponse::Ok)
    }

    /// Mints a pairing offer, returns it for the UI to draw as a QR code, and
    /// runs the exchange in the background so the request answers immediately.
    fn start_offer(&mut self) -> Result<IpcResponse, IpcError> {
        let launcher = self.pairing.clone().ok_or(IpcError::Internal)?;

        if let Some(previous) = self.offer_task.take() {
            previous.abort();
        }

        let offer = tandem_pairing::DesktopOffer::new(
            &launcher.credentials.identity.fingerprint(),
            tandem_pairing::generate_token(),
            launcher.credentials.identity.display_name.clone(),
        );
        let payload = offer.encode();

        let credentials = tandem_pairing::flow::DesktopCredentials {
            name: launcher.credentials.identity.display_name.clone(),
            platform: platform_name().to_string(),
            cert_der: launcher.credentials.identity.cert_der.clone(),
            key_der: launcher.credentials.key_der.clone(),
        };

        let approval = self.pending_approval.clone();
        self.offer_task = Some(tokio::spawn(async move {
            let events = launcher.events.clone();
            let ask = |introduction: tandem_pairing::PhoneIntroduction| {
                let events = events.clone();
                let approval = approval.clone();
                async move {
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    *approval.lock().expect("approval mutex poisoned") = Some(sender);
                    events.publish(tandem_ipc::api::IpcEvent::PairingApprovalRequested {
                        phone_name: introduction.phone_name,
                        phone_fingerprint: introduction.phone_fingerprint,
                    });
                    // A dropped sender means the offer was replaced or the UI
                    // went away, which is a refusal rather than an approval.
                    receiver.await.unwrap_or(false)
                }
            };

            let progress_events = events.clone();
            let outcome = tandem_pairing::offer::run(
                &offer,
                &credentials,
                |state| {
                    progress_events.publish(tandem_ipc::api::IpcEvent::PairingProgress {
                        state: describe_offer(state),
                        short_code: None,
                    });
                },
                ask,
            )
            .await;

            match outcome {
                Ok(record) => {
                    {
                        let mut guard = launcher.app.lock().expect("app mutex poisoned");
                        guard
                            .store()
                            .set_paired_phone(tandem_core::model::PairedPhone {
                                device_id: record.phone_device_id.clone(),
                                name: record.phone_name.clone(),
                                spki_sha256: record.phone_spki_sha256.to_base64url(),
                                bt_address: record.phone_bt_address.clone(),
                            });
                        let _ = guard.store().save(&launcher.state_path);
                    }
                    launcher
                        .events
                        .publish(tandem_ipc::api::IpcEvent::PairingProgress {
                            state: "accepted".into(),
                            short_code: None,
                        });
                }
                Err(error) => launcher
                    .events
                    .publish(tandem_ipc::api::IpcEvent::PairingProgress {
                        state: format!("failed: {error}"),
                        short_code: None,
                    }),
            }
        }));

        Ok(IpcResponse::Offer(tandem_ipc::api::OfferResult {
            payload,
            desktop_name: self
                .pairing
                .as_ref()
                .map(|p| p.credentials.identity.display_name.clone())
                .unwrap_or_default(),
        }))
    }

    fn status(&mut self) -> StatusResult {
        let mut app = self.app.lock().expect("app mutex poisoned");
        let link = self.link.lock().expect("link mutex poisoned").clone();

        let desktop_audio_available = app.desktop_audio_available();
        let mirror = app.controller().mirror().cloned();
        StatusResult {
            connection: link.connection.unwrap_or(ConnectionStatus::Idle),
            phone_name: link.phone_name,
            calls: mirror
                .as_ref()
                .map(|m| m.calls.iter().map(call_view).collect())
                .unwrap_or_default(),
            audio_route: mirror
                .as_ref()
                .map(|m| audio_route(m.audio_route))
                .unwrap_or(IpcAudioRoute::Earpiece),
            microphone_muted: mirror.as_ref().map(|m| m.microphone_muted).unwrap_or(false),
            desktop_audio_available,
        }
    }
}

impl IpcService for DaemonIpcService {
    fn handle(&mut self, request: IpcRequest) -> Result<IpcResponse, IpcError> {
        let command = match request {
            IpcRequest::Status => return Ok(IpcResponse::Status(self.status())),
            IpcRequest::Dial { number, sim_slot } => UserCommand::Dial { number, sim_slot },
            IpcRequest::Answer { call_id } => UserCommand::Answer { call_id },
            IpcRequest::Reject { call_id } => UserCommand::Reject { call_id },
            IpcRequest::End { call_id } => UserCommand::End { call_id },
            IpcRequest::Mute { muted } => UserCommand::SetMuted { muted },
            IpcRequest::Hold { call_id } => UserCommand::Hold { call_id },
            IpcRequest::Unhold { call_id } => UserCommand::Unhold { call_id },
            IpcRequest::Merge {
                call_id,
                other_call_id,
            } => UserCommand::Merge {
                call_id,
                other_call_id,
            },
            IpcRequest::Dtmf { call_id, digits } => UserCommand::SendDtmf { call_id, digits },
            IpcRequest::AudioRoute {
                route,
                bt_device_address,
            } => {
                let audio_available = self
                    .app
                    .lock()
                    .expect("app mutex poisoned")
                    .desktop_audio_available();
                if !audio_available && matches!(route, IpcAudioRoute::Bluetooth) {
                    return Err(IpcError::AudioUnavailable);
                }
                UserCommand::RequestAudioRoute {
                    route: domain_route(route),
                    bt_device_address,
                }
            }
            IpcRequest::Pairing { qr_payload } => return self.start_pairing(qr_payload),
            IpcRequest::PairingOffer => return self.start_offer(),
            IpcRequest::PairingConfirm { accept } => {
                let verdict = self
                    .pending_approval
                    .lock()
                    .expect("approval mutex poisoned")
                    .take();
                match verdict {
                    Some(sender) => {
                        let _ = sender.send(accept);
                        return Ok(IpcResponse::Ok);
                    }
                    None => return Err(IpcError::PairingFailed("nothing to approve".into())),
                }
            }

            IpcRequest::History { .. } | IpcRequest::Settings => return Err(IpcError::Internal),
        };

        self.app
            .lock()
            .expect("app mutex poisoned")
            .controller()
            .apply_user_command(command)
            .map(|_| IpcResponse::Ok)
            .map_err(map_core_error)
    }
}

/// Core failures keep their meaning across the IPC boundary so the UI can show
/// the right guidance — the emergency refusal especially (ADR-0008).
fn map_core_error(error: CoreError) -> IpcError {
    match error {
        CoreError::CallNotFound(id) => IpcError::CallNotFound(id),
        CoreError::InvalidCallState { .. } | CoreError::EmergencyCallActive => {
            IpcError::InvalidCallState
        }
        CoreError::EmergencyBlocked { number } => IpcError::EmergencyBlocked { number },
        CoreError::StaleEpoch { .. } | CoreError::NotSynchronized => IpcError::PhoneOffline,
    }
}

fn call_view(call: &Call) -> CallView {
    CallView {
        call_id: call.call_id.clone(),
        state: call_state(call.state),
        remote_number: call.remote_number.clone(),
        remote_display_name: call.remote_display_name.clone(),
        started_at_ms: call.started_at_ms,
        is_conference: call.is_conference,
        can_hold: call.can_hold,
        can_merge: call.can_merge,
        is_emergency: call.is_emergency,
    }
}

fn call_state(state: CallState) -> IpcCallState {
    match state {
        CallState::Connecting => IpcCallState::Connecting,
        CallState::Dialing => IpcCallState::Dialing,
        CallState::Ringing => IpcCallState::Ringing,
        CallState::Active => IpcCallState::Active,
        CallState::Holding => IpcCallState::Holding,
        CallState::Disconnecting => IpcCallState::Disconnecting,
        CallState::Disconnected => IpcCallState::Disconnected,
    }
}

fn audio_route(route: AudioRoute) -> IpcAudioRoute {
    match route {
        AudioRoute::Earpiece => IpcAudioRoute::Earpiece,
        AudioRoute::Speaker => IpcAudioRoute::Speaker,
        AudioRoute::WiredHeadset => IpcAudioRoute::WiredHeadset,
        AudioRoute::Bluetooth => IpcAudioRoute::Bluetooth,
    }
}

/// Reported to the phone so its paired-devices list can label this machine.
fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn describe(state: &tandem_pairing::flow::PairingState) -> String {
    use tandem_pairing::flow::PairingState as S;
    match state {
        S::Scanned => "scanned".into(),
        S::Connecting => "connecting".into(),
        S::AwaitingConfirmation { .. } => "awaitingConfirmation".into(),
        S::Accepted(_) => "accepted".into(),
        S::Failed(error) => format!("failed: {error}"),
    }
}

fn describe_offer(state: &tandem_pairing::OfferState) -> String {
    use tandem_pairing::OfferState as S;
    match state {
        S::Waiting => "waitingForScan".into(),
        S::Retrying { reason } => format!("retrying: {reason}"),
        S::Connecting { phone_name } => format!("connecting:{phone_name}"),
        S::AwaitingLocalApproval(phone) => format!("approve:{}", phone.phone_name),
        S::AwaitingConfirmation => "awaitingConfirmation".into(),
        S::Accepted(_) => "accepted".into(),
        S::Failed(error) => format!("failed: {error}"),
    }
}

fn short_code_of(state: &tandem_pairing::flow::PairingState) -> Option<String> {
    match state {
        tandem_pairing::flow::PairingState::AwaitingConfirmation { short_code } => {
            short_code.as_ref().map(|c| c.as_str().to_string())
        }
        _ => None,
    }
}

fn domain_route(route: IpcAudioRoute) -> AudioRoute {
    match route {
        IpcAudioRoute::Earpiece => AudioRoute::Earpiece,
        IpcAudioRoute::Speaker => AudioRoute::Speaker,
        IpcAudioRoute::WiredHeadset => AudioRoute::WiredHeadset,
        IpcAudioRoute::Bluetooth => AudioRoute::Bluetooth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tandem_bluetooth::backends::BackendKind;

    fn service() -> DaemonIpcService {
        service_with_link(LinkState::default())
    }

    fn service_with_link(link: LinkState) -> DaemonIpcService {
        let mut app = App::build(Config {
            bluetooth_backend: BackendKind::Null,
            ..Config::default()
        });
        app.adopt_emergency_numbers(vec!["911".into(), "112".into()]);
        DaemonIpcService::new(
            std::sync::Arc::new(std::sync::Mutex::new(app)),
            std::sync::Arc::new(std::sync::Mutex::new(link)),
        )
    }

    #[test]
    fn status_reports_media_availability_so_the_ui_can_explain_itself() {
        let mut s = service_with_link(LinkState {
            connection: Some(ConnectionStatus::Live),
            phone_name: "Pixel".into(),
        });
        let response = s.handle(IpcRequest::Status).unwrap();

        match response {
            IpcResponse::Status(status) => {
                assert_eq!(status.connection, ConnectionStatus::Live);
                assert_eq!(status.phone_name, "Pixel");
                assert!(!status.desktop_audio_available);
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    /// The refusal must survive translation with its number intact, since the UI
    /// copy names the number it refused.
    #[test]
    fn an_emergency_dial_is_refused_with_the_number_preserved() {
        let mut s = service();
        let err = s
            .handle(IpcRequest::Dial {
                number: "911".into(),
                sim_slot: -1,
            })
            .unwrap_err();
        assert_eq!(
            err,
            IpcError::EmergencyBlocked {
                number: "911".into()
            }
        );
        assert_eq!(err.code(), tandem_ipc::error::IPC_EMERGENCY_BLOCKED);
    }

    #[test]
    fn an_ordinary_dial_passes_through() {
        let mut s = service();
        assert_eq!(
            s.handle(IpcRequest::Dial {
                number: "+14155550123".into(),
                sim_slot: -1,
            })
            .unwrap(),
            IpcResponse::Ok
        );
    }

    /// Routing to Bluetooth on a Tier B-lite build must fail with a specific
    /// reason rather than a generic error.
    #[test]
    fn bluetooth_routing_is_refused_when_the_build_has_no_audio_path() {
        let mut s = service();
        assert_eq!(
            s.handle(IpcRequest::AudioRoute {
                route: IpcAudioRoute::Bluetooth,
                bt_device_address: "AA:BB".into(),
            })
            .unwrap_err(),
            IpcError::AudioUnavailable
        );
    }

    #[test]
    fn commands_against_an_unsynced_mirror_report_the_phone_offline() {
        let mut s = service();
        assert_eq!(
            s.handle(IpcRequest::Answer {
                call_id: "c1".into()
            })
            .unwrap_err(),
            IpcError::PhoneOffline
        );
    }
}
