//! Event vocabulary between transport and controller: inbound phone events
//! (snapshot, incoming call, route change, log change) and outbound UI-facing
//! state deltas. Pure data; no channels or runtime types.

use crate::model::{AudioRoute, Call, CallSnapshot, StateVersion};

/// Everything the phone can tell the desktop about the call plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneEvent {
    SnapshotReplaced(CallSnapshot),
    IncomingCall {
        call: Call,
        version: StateVersion,
    },
    CallStateChanged(CallSnapshot),
    AudioRouteChanged {
        route: AudioRoute,
        bt_device_address: String,
        version: StateVersion,
    },
    CallLogChanged {
        log_version: u64,
    },
    Revoked {
        reason: String,
    },
}

/// User intent arriving from the UI, before it becomes a TLP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserCommand {
    Dial {
        number: String,
        sim_slot: i32,
    },
    Answer {
        call_id: String,
    },
    Reject {
        call_id: String,
    },
    End {
        call_id: String,
    },
    SetMuted {
        muted: bool,
    },
    Hold {
        call_id: String,
    },
    Unhold {
        call_id: String,
    },
    Merge {
        call_id: String,
        other_call_id: String,
    },
    SendDtmf {
        call_id: String,
        digits: String,
    },
    RequestAudioRoute {
        route: AudioRoute,
        bt_device_address: String,
    },
}

/// What the controller emits after a transition: state for the UI, requests for
/// the transport, or a refusal that never reaches the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerOutput {
    MirrorUpdated(CallSnapshot),
    SendRequest(OutboundRequest),
    EmergencyRefused { number: String },
    SessionClosed { reason: String },
}

/// Controller-level description of a TLP request; the codec turns this into an
/// Envelope so the domain never touches generated types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboundRequest {
    Dial {
        number: String,
        sim_slot: i32,
    },
    Answer {
        call_id: String,
    },
    Reject {
        call_id: String,
    },
    End {
        call_id: String,
    },
    SetMuted {
        muted: bool,
    },
    Hold {
        call_id: String,
    },
    Unhold {
        call_id: String,
    },
    Merge {
        call_id: String,
        other_call_id: String,
    },
    SendDtmf {
        call_id: String,
        digits: String,
    },
    AudioRoute {
        route: AudioRoute,
        bt_device_address: String,
    },
    SyncCallLog {
        since_ms: i64,
        max_entries: u32,
    },
}

impl OutboundRequest {
    /// Idempotent requests carry an absolute target state and are safe to repeat;
    /// the rest are deduplicated by (device id, message_id) — docs/11 section 5.
    pub fn is_idempotent(&self) -> bool {
        matches!(
            self,
            Self::SetMuted { .. }
                | Self::Hold { .. }
                | Self::Unhold { .. }
                | Self::AudioRoute { .. }
        )
    }
}
