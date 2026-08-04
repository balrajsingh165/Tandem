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

/// Bridges the UI-facing API to the domain, keeping every policy decision in
/// core rather than in this translation layer.
pub struct DaemonIpcService {
    app: App,
    connection: ConnectionStatus,
    phone_name: String,
}

impl DaemonIpcService {
    pub fn new(app: App) -> Self {
        Self {
            app,
            connection: ConnectionStatus::Idle,
            phone_name: String::new(),
        }
    }

    pub fn set_connection(&mut self, connection: ConnectionStatus, phone_name: &str) {
        self.connection = connection;
        self.phone_name = phone_name.to_string();
    }

    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    fn status(&mut self) -> StatusResult {
        let desktop_audio_available = self.app.desktop_audio_available();
        let mirror = self.app.controller().mirror().cloned();
        StatusResult {
            connection: self.connection,
            phone_name: self.phone_name.clone(),
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
                if !self.app.desktop_audio_available() && matches!(route, IpcAudioRoute::Bluetooth)
                {
                    return Err(IpcError::AudioUnavailable);
                }
                UserCommand::RequestAudioRoute {
                    route: domain_route(route),
                    bt_device_address,
                }
            }
            IpcRequest::History { .. } | IpcRequest::Pairing { .. } | IpcRequest::Settings => {
                return Err(IpcError::Internal)
            }
        };

        self.app
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
        let mut app = App::build(Config {
            bluetooth_backend: BackendKind::Null,
            ..Config::default()
        });
        app.adopt_emergency_numbers(vec!["911".into(), "112".into()]);
        DaemonIpcService::new(app)
    }

    #[test]
    fn status_reports_media_availability_so_the_ui_can_explain_itself() {
        let mut s = service();
        s.set_connection(ConnectionStatus::Live, "Pixel");
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
