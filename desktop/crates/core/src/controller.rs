//! CallController: consumes phone events and user commands, maintains the
//! mirrored CallSnapshot (phone is the source of truth — ADR-0007), and emits UI
//! state plus outbound requests. Pure transition function; side effects live in
//! the daemon.

use crate::emergency::EmergencyNumbers;
use crate::error::CoreError;
use crate::events::{ControllerOutput, OutboundRequest, PhoneEvent, UserCommand};
use crate::model::{CallSnapshot, CallState, StateVersion};
use crate::reconcile::{self, Reconciliation};

#[derive(Debug, Default)]
pub struct CallController {
    mirror: Option<CallSnapshot>,
    emergency: EmergencyNumbers,
}

impl CallController {
    pub fn new(emergency: EmergencyNumbers) -> Self {
        Self {
            mirror: None,
            emergency,
        }
    }

    pub fn mirror(&self) -> Option<&CallSnapshot> {
        self.mirror.as_ref()
    }

    pub fn version(&self) -> Option<&StateVersion> {
        self.mirror.as_ref().map(|s| &s.version)
    }

    pub fn set_emergency_numbers(&mut self, emergency: EmergencyNumbers) {
        self.emergency = emergency;
    }

    /// Phone truth always wins: a snapshot replaces the mirror, and an
    /// out-of-order event triggers a replace rather than a partial apply.
    pub fn apply_phone_event(&mut self, event: PhoneEvent) -> Vec<ControllerOutput> {
        match event {
            PhoneEvent::SnapshotReplaced(snapshot) | PhoneEvent::CallStateChanged(snapshot) => {
                self.mirror = Some(snapshot.clone());
                vec![ControllerOutput::MirrorUpdated(snapshot)]
            }
            PhoneEvent::IncomingCall { call, version } => match self.mirror.as_mut() {
                Some(mirror) if reconcile::is_contiguous(&mirror.version, &version) => {
                    mirror.calls.retain(|c| c.call_id != call.call_id);
                    mirror.calls.push(call);
                    mirror.version = version;
                    vec![ControllerOutput::MirrorUpdated(mirror.clone())]
                }
                _ => Vec::new(),
            },
            PhoneEvent::AudioRouteChanged {
                route,
                bt_device_address,
                version,
            } => match self.mirror.as_mut() {
                Some(mirror) if reconcile::is_contiguous(&mirror.version, &version) => {
                    mirror.audio_route = route;
                    mirror.bt_route_address = bt_device_address;
                    mirror.version = version;
                    vec![ControllerOutput::MirrorUpdated(mirror.clone())]
                }
                _ => Vec::new(),
            },
            PhoneEvent::CallLogChanged { .. } => {
                vec![ControllerOutput::SendRequest(
                    OutboundRequest::SyncCallLog {
                        since_ms: 0,
                        max_entries: 200,
                        before_ms: 0,
                    },
                )]
            }
            PhoneEvent::Revoked { reason } => {
                self.mirror = None;
                vec![ControllerOutput::SessionClosed { reason }]
            }
        }
    }

    /// Validates intent against the mirror before it reaches the wire. The phone
    /// re-validates authoritatively; this only prevents obviously invalid and
    /// emergency-policy-violating traffic.
    pub fn apply_user_command(
        &mut self,
        command: UserCommand,
    ) -> Result<ControllerOutput, CoreError> {
        if let Some(mirror) = self.mirror.as_ref() {
            if mirror.has_active_emergency() && !matches!(command, UserCommand::Dial { .. }) {
                return Err(CoreError::EmergencyCallActive);
            }
        }

        let request = match command {
            UserCommand::Dial { number, sim_slot } => {
                self.emergency.guard(&number)?;
                OutboundRequest::Dial { number, sim_slot }
            }
            UserCommand::Answer { call_id } => {
                self.expect_state(&call_id, CallState::accepts_answer, "answer")?;
                OutboundRequest::Answer { call_id }
            }
            UserCommand::Reject { call_id } => {
                self.expect_state(&call_id, CallState::accepts_answer, "reject")?;
                OutboundRequest::Reject { call_id }
            }
            UserCommand::End { call_id } => {
                self.expect_state(&call_id, |s| !s.is_terminal(), "end")?;
                OutboundRequest::End { call_id }
            }
            UserCommand::SetMuted { muted } => OutboundRequest::SetMuted { muted },
            UserCommand::Hold { call_id } => OutboundRequest::Hold { call_id },
            UserCommand::Unhold { call_id } => OutboundRequest::Unhold { call_id },
            UserCommand::Merge {
                call_id,
                other_call_id,
            } => OutboundRequest::Merge {
                call_id,
                other_call_id,
            },
            UserCommand::SendDtmf { call_id, digits } => {
                self.expect_state(&call_id, |s| s == CallState::Active, "dtmf")?;
                OutboundRequest::SendDtmf { call_id, digits }
            }
            UserCommand::RequestAudioRoute {
                route,
                bt_device_address,
            } => OutboundRequest::AudioRoute {
                route,
                bt_device_address,
            },
        };

        Ok(ControllerOutput::SendRequest(request))
    }

    /// Reconciliation entry point used by the transport after a reconnect.
    pub fn reconcile_with(&self, phone: &StateVersion) -> Reconciliation {
        reconcile::decide(self.version(), phone)
    }

    fn expect_state(
        &self,
        call_id: &str,
        predicate: impl Fn(CallState) -> bool,
        command: &'static str,
    ) -> Result<(), CoreError> {
        let mirror = self.mirror.as_ref().ok_or(CoreError::NotSynchronized)?;
        let call = mirror
            .call(call_id)
            .ok_or_else(|| CoreError::CallNotFound(call_id.to_string()))?;
        if predicate(call.state) {
            Ok(())
        } else {
            Err(CoreError::InvalidCallState {
                call_id: call_id.to_string(),
                state: call.state,
                command,
            })
        }
    }
}
