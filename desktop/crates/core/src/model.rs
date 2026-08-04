//! Domain models of the mirrored call plane: Call, CallState, AudioRoute,
//! CallLogRow, PairedPhone, plus (epoch_id, state_seq) versioning. Converted from
//! tandem.v1 protos at the transport boundary only.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallState {
    Connecting,
    Dialing,
    Ringing,
    Active,
    Holding,
    Disconnecting,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioRoute {
    Earpiece,
    Speaker,
    WiredHeadset,
    Bluetooth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub call_id: String,
    pub state: CallState,
    pub direction: CallDirection,
    pub remote_number: String,
    pub remote_display_name: String,
    pub started_at_ms: i64,
    pub is_conference: bool,
    pub can_hold: bool,
    pub can_merge: bool,
    pub is_emergency: bool,
    pub sim_slot: i32,
}

/// Versioning pair that makes the desktop mirror reconcilable against phone truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateVersion {
    pub epoch_id: String,
    pub state_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSnapshot {
    pub version: StateVersion,
    pub calls: Vec<Call>,
    pub audio_route: AudioRoute,
    pub microphone_muted: bool,
    pub bt_route_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallLogRow {
    pub entry_id: String,
    pub number: String,
    pub display_name: String,
    pub started_at_ms: i64,
    pub duration_seconds: u32,
    pub sim_slot: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedPhone {
    pub device_id: String,
    pub name: String,
    pub spki_sha256: String,
    pub bt_address: String,
}

impl CallState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Disconnected)
    }

    pub fn accepts_answer(self) -> bool {
        matches!(self, Self::Ringing)
    }
}

impl CallSnapshot {
    pub fn call(&self, call_id: &str) -> Option<&Call> {
        self.calls.iter().find(|c| c.call_id == call_id)
    }

    pub fn has_active_emergency(&self) -> bool {
        self.calls
            .iter()
            .any(|c| c.is_emergency && !c.state.is_terminal())
    }
}

impl From<tandem_proto::CallState> for CallState {
    fn from(value: tandem_proto::CallState) -> Self {
        use tandem_proto::CallState as P;
        match value {
            P::Unspecified | P::Connecting => Self::Connecting,
            P::Dialing => Self::Dialing,
            P::Ringing => Self::Ringing,
            P::Active => Self::Active,
            P::Holding => Self::Holding,
            P::Disconnecting => Self::Disconnecting,
            P::Disconnected => Self::Disconnected,
        }
    }
}

impl From<tandem_proto::AudioRoute> for AudioRoute {
    fn from(value: tandem_proto::AudioRoute) -> Self {
        use tandem_proto::AudioRoute as P;
        match value {
            P::Speaker => Self::Speaker,
            P::WiredHeadset => Self::WiredHeadset,
            P::Bluetooth => Self::Bluetooth,
            P::Unspecified | P::Earpiece => Self::Earpiece,
        }
    }
}
