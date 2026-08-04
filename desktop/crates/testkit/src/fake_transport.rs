//! TransportClient fake wired to fake_phone: connect/disconnect/resume scripting
//! with deterministic timing for reconnect and reconciliation tests.

use tandem_core::events::{OutboundRequest, PhoneEvent};
use tandem_core::model::StateVersion;

use crate::fake_phone::FakePhone;

/// Connection lifecycle the fake reports, without sockets or timers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeConnectionState {
    Disconnected,
    Live,
}

/// Couples a scripted phone to a connection that tests can drop and restore.
#[derive(Debug)]
pub struct FakeTransport {
    phone: FakePhone,
    state: FakeConnectionState,
    pub sent: Vec<OutboundRequest>,
}

impl Default for FakeTransport {
    fn default() -> Self {
        Self {
            phone: FakePhone::default(),
            state: FakeConnectionState::Disconnected,
            sent: Vec::new(),
        }
    }
}

impl FakeTransport {
    pub fn phone_mut(&mut self) -> &mut FakePhone {
        &mut self.phone
    }

    pub fn state(&self) -> FakeConnectionState {
        self.state
    }

    pub fn connect(&mut self) -> PhoneEvent {
        self.state = FakeConnectionState::Live;
        PhoneEvent::SnapshotReplaced(self.phone.snapshot().clone())
    }

    /// Simulates a Wi-Fi blip. Requests attempted while down are refused rather
    /// than silently dropped.
    pub fn disconnect(&mut self) {
        self.state = FakeConnectionState::Disconnected;
    }

    pub fn send(&mut self, request: OutboundRequest) -> Result<(), &'static str> {
        if self.state != FakeConnectionState::Live {
            return Err("not connected");
        }
        self.sent.push(request);
        Ok(())
    }

    /// The resume handshake: reports the phone's current version so the
    /// controller can decide whether its mirror survived.
    pub fn resume(&mut self) -> (StateVersion, PhoneEvent) {
        self.state = FakeConnectionState::Live;
        let version = self.phone.snapshot().version.clone();
        (
            version,
            PhoneEvent::SnapshotReplaced(self.phone.snapshot().clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tandem_core::controller::CallController;
    use tandem_core::model::CallState;
    use tandem_core::reconcile::Reconciliation;

    #[test]
    fn requests_are_refused_while_disconnected() {
        let mut t = FakeTransport::default();
        assert_eq!(
            t.send(OutboundRequest::SetMuted { muted: true }),
            Err("not connected")
        );
        t.connect();
        assert!(t.send(OutboundRequest::SetMuted { muted: true }).is_ok());
    }

    /// A blip that changes nothing on the phone leaves the mirror contiguous, so
    /// no snapshot replacement is needed.
    #[test]
    fn a_quiet_blip_lets_the_mirror_continue() {
        let mut t = FakeTransport::default();
        let mut controller = CallController::default();
        controller.apply_phone_event(t.connect());

        t.disconnect();
        let (version, _) = t.resume();
        assert_eq!(
            controller.reconcile_with(&version),
            Reconciliation::Continue
        );
    }

    /// If the phone advanced while the desktop was away, the mirror is stale and
    /// must be replaced wholesale — phone truth wins (ADR-0007).
    #[test]
    fn a_blip_that_hid_events_forces_a_snapshot_replace() {
        let mut t = FakeTransport::default();
        let mut controller = CallController::default();
        controller.apply_phone_event(t.connect());

        t.disconnect();
        t.phone_mut().incoming_call("c1");

        let (version, snapshot_event) = t.resume();
        assert_eq!(
            controller.reconcile_with(&version),
            Reconciliation::ReplaceSnapshot
        );

        controller.apply_phone_event(snapshot_event);
        let mirror = controller.mirror().unwrap();
        assert_eq!(mirror.calls.len(), 1);
        assert_eq!(mirror.calls[0].state, CallState::Ringing);
        assert_eq!(
            controller.reconcile_with(&version),
            Reconciliation::Continue
        );
    }
}
