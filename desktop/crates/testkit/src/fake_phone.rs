//! In-process fake of the phone gateway: speaks real TLP envelopes over an
//! in-memory transport, scriptable call scenarios (incoming, answer races, epoch
//! bumps) for integration tests without a device.

use tandem_core::events::PhoneEvent;
use tandem_core::model::{CallSnapshot, CallState};

use crate::fixtures;

/// Scriptable phone-side source of truth. It owns the authoritative snapshot and
/// bumps (epoch_id, state_seq) exactly as the real gateway does.
#[derive(Debug, Clone)]
pub struct FakePhone {
    snapshot: CallSnapshot,
    /// Set when another desktop has already claimed the ringing call, so a
    /// second answer loses the race.
    answered_by_other: bool,
}

impl Default for FakePhone {
    fn default() -> Self {
        Self {
            snapshot: fixtures::empty_snapshot(1),
            answered_by_other: false,
        }
    }
}

impl FakePhone {
    pub fn snapshot(&self) -> &CallSnapshot {
        &self.snapshot
    }

    fn bump(&mut self) {
        self.snapshot.version.state_seq += 1;
    }

    /// Simulates a restart: a new epoch invalidates every desktop mirror.
    pub fn restart(&mut self, new_epoch: &str) -> PhoneEvent {
        self.snapshot.version.epoch_id = new_epoch.to_string();
        self.snapshot.version.state_seq = 1;
        self.snapshot.calls.clear();
        PhoneEvent::SnapshotReplaced(self.snapshot.clone())
    }

    pub fn incoming_call(&mut self, call_id: &str) -> PhoneEvent {
        let call = fixtures::call(call_id, CallState::Ringing);
        self.bump();
        self.snapshot.calls.push(call.clone());
        PhoneEvent::IncomingCall {
            call,
            version: self.snapshot.version.clone(),
        }
    }

    /// Another desktop won the race; a later answer from this one must lose.
    pub fn answered_elsewhere(&mut self, call_id: &str) {
        self.answered_by_other = true;
        self.transition(call_id, CallState::Active);
    }

    pub fn transition(&mut self, call_id: &str, state: CallState) -> PhoneEvent {
        self.bump();
        if let Some(call) = self
            .snapshot
            .calls
            .iter_mut()
            .find(|c| c.call_id == call_id)
        {
            call.state = state;
        }
        PhoneEvent::CallStateChanged(self.snapshot.clone())
    }

    /// First valid answer wins; every later one is refused as already handled.
    pub fn try_answer(&mut self, call_id: &str) -> Result<PhoneEvent, &'static str> {
        if self.answered_by_other {
            return Err("already handled");
        }
        let ringing = self
            .snapshot
            .calls
            .iter()
            .any(|c| c.call_id == call_id && c.state == CallState::Ringing);
        if !ringing {
            return Err("invalid call state");
        }
        self.answered_by_other = true;
        Ok(self.transition(call_id, CallState::Active))
    }

    pub fn revoke(&mut self, reason: &str) -> PhoneEvent {
        PhoneEvent::Revoked {
            reason: reason.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tandem_core::controller::CallController;
    use tandem_core::events::ControllerOutput;

    #[test]
    fn an_incoming_call_reaches_the_controller_mirror() {
        let mut phone = FakePhone::default();
        let mut controller = CallController::default();

        controller.apply_phone_event(PhoneEvent::SnapshotReplaced(phone.snapshot().clone()));
        let event = phone.incoming_call("c1");
        let outputs = controller.apply_phone_event(event);

        assert!(matches!(outputs[0], ControllerOutput::MirrorUpdated(_)));
        let mirror = controller.mirror().unwrap();
        assert_eq!(mirror.calls.len(), 1);
        assert_eq!(mirror.calls[0].state, CallState::Ringing);
    }

    #[test]
    fn losing_the_answer_race_is_reported_not_applied() {
        let mut phone = FakePhone::default();
        phone.incoming_call("c1");
        phone.answered_elsewhere("c1");
        assert_eq!(phone.try_answer("c1"), Err("already handled"));
    }

    #[test]
    fn the_first_answer_wins() {
        let mut phone = FakePhone::default();
        phone.incoming_call("c1");
        assert!(phone.try_answer("c1").is_ok());
        assert_eq!(phone.try_answer("c1"), Err("already handled"));
    }

    #[test]
    fn a_phone_restart_replaces_the_whole_mirror() {
        let mut phone = FakePhone::default();
        let mut controller = CallController::default();

        controller.apply_phone_event(PhoneEvent::SnapshotReplaced(phone.snapshot().clone()));
        controller.apply_phone_event(phone.incoming_call("c1"));
        assert_eq!(controller.mirror().unwrap().calls.len(), 1);

        controller.apply_phone_event(phone.restart("epoch-0001"));
        let mirror = controller.mirror().unwrap();
        assert!(mirror.calls.is_empty());
        assert_eq!(mirror.version.epoch_id, "epoch-0001");
    }

    #[test]
    fn revocation_clears_the_mirror_and_closes_the_session() {
        let mut phone = FakePhone::default();
        let mut controller = CallController::default();
        controller.apply_phone_event(PhoneEvent::SnapshotReplaced(phone.snapshot().clone()));

        let outputs = controller.apply_phone_event(phone.revoke("removed on phone"));
        assert!(matches!(outputs[0], ControllerOutput::SessionClosed { .. }));
        assert!(controller.mirror().is_none());
    }
}
