//! Composition root: constructs backends per platform/config (ADR-0010
//! selection), wires controller, transport, audio, bluetooth, and IPC together
//! with channels, and supervises task lifecycles with graceful degradation
//! (audio subsystem loss never kills control).

use tandem_bluetooth::backend::BluetoothBackend;
use tandem_bluetooth::backends;
use tandem_core::controller::CallController;
use tandem_core::emergency::EmergencyNumbers;

use crate::config::Config;
use crate::store::Store;

/// Which subsystems came up. Control is required; media is optional, so a
/// desktop with no usable Bluetooth still runs as a full Tier A product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubsystemHealth {
    pub control_ready: bool,
    pub media_ready: bool,
}

impl SubsystemHealth {
    /// The daemon is usable whenever the control plane is up, regardless of
    /// media — that is the whole point of the tier model.
    pub fn is_usable(&self) -> bool {
        self.control_ready
    }
}

/// The assembled daemon. Construction never fails on media problems; it records
/// them and continues, because losing audio must not cost the user call control.
pub struct App {
    config: Config,
    controller: CallController,
    store: Store,
    bluetooth: Box<dyn BluetoothBackend>,
    health: SubsystemHealth,
    next_message_id: u64,
}

impl App {
    pub fn build(config: Config) -> Self {
        let bluetooth = backends::create(config.bluetooth_backend);
        let media_ready = bluetooth.supports_audio();

        Self {
            config,
            controller: CallController::default(),
            store: Store::default(),
            bluetooth,
            health: SubsystemHealth {
                control_ready: true,
                media_ready,
            },
            next_message_id: 1,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn controller(&mut self) -> &mut CallController {
        &mut self.controller
    }

    pub fn store(&mut self) -> &mut Store {
        &mut self.store
    }

    /// Message ids continue across sessions so the phone can deduplicate a
    /// post-reconnect retry (docs/06 framing rules).
    pub fn next_message_id(&self) -> u64 {
        self.next_message_id
    }

    pub fn set_next_message_id(&mut self, next: u64) {
        self.next_message_id = next;
    }

    pub fn health(&self) -> SubsystemHealth {
        self.health
    }

    pub fn desktop_audio_available(&self) -> bool {
        self.bluetooth.supports_audio()
    }

    /// Applies the emergency list the phone reports at session start, so the
    /// desktop-side pre-check reflects the current SIM and region (ADR-0008).
    pub fn adopt_emergency_numbers(&mut self, numbers: Vec<String>) {
        self.controller
            .set_emergency_numbers(EmergencyNumbers::from_session(numbers));
    }

    /// A media-subsystem failure degrades the daemon rather than stopping it.
    pub fn note_media_failure(&mut self) {
        self.health.media_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tandem_bluetooth::backends::BackendKind;
    use tandem_core::events::UserCommand;

    fn app() -> App {
        App::build(Config {
            bluetooth_backend: BackendKind::Null,
            ..Config::default()
        })
    }

    #[test]
    fn a_build_without_media_is_still_usable() {
        let app = app();
        assert!(app.health().is_usable());
        assert!(!app.health().media_ready);
        assert!(!app.desktop_audio_available());
    }

    #[test]
    fn losing_media_does_not_take_down_control() {
        let mut app = app();
        app.note_media_failure();
        assert!(!app.health().media_ready);
        assert!(app.health().control_ready);
        assert!(app.health().is_usable());
    }

    /// The emergency list arrives per session; adopting it must actually change
    /// what the local pre-check refuses.
    #[test]
    fn adopting_the_session_emergency_list_arms_the_local_pre_check() {
        let mut app = app();
        app.adopt_emergency_numbers(vec!["110".into()]);

        let blocked = app.controller().apply_user_command(UserCommand::Dial {
            number: "110".into(),
            sim_slot: -1,
        });
        assert!(blocked.is_err());

        let allowed = app.controller().apply_user_command(UserCommand::Dial {
            number: "+14155550123".into(),
            sim_slot: -1,
        });
        assert!(allowed.is_ok());
    }
}
