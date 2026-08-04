//! CoreError: typed domain failures (unknown call id, invalid state for command,
//! emergency blocked, stale epoch) mapped from/to TLP Status codes at the
//! boundaries.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("no call with id {0}")]
    CallNotFound(String),

    #[error("call {call_id} is in state {state:?}, which does not permit {command}")]
    InvalidCallState {
        call_id: String,
        state: crate::model::CallState,
        command: &'static str,
    },

    #[error("{number} is an emergency number; dial it on the handset")]
    EmergencyBlocked { number: String },

    #[error("remote control is refused while an emergency call is active")]
    EmergencyCallActive,

    #[error("mirror epoch {mirror} is stale against phone epoch {phone}")]
    StaleEpoch { mirror: String, phone: String },

    #[error("the phone has not sent a snapshot yet")]
    NotSynchronized,
}

impl CoreError {
    /// Wire code this failure maps onto when it crosses the transport edge.
    pub fn wire_code(&self) -> tandem_proto::ErrorCode {
        use tandem_proto::ErrorCode as E;
        match self {
            Self::CallNotFound(_) => E::CallNotFound,
            Self::InvalidCallState { .. } => E::InvalidCallState,
            Self::EmergencyBlocked { .. } => E::EmergencyNumberBlocked,
            Self::EmergencyCallActive => E::InvalidCallState,
            Self::StaleEpoch { .. } | Self::NotSynchronized => E::Internal,
        }
    }
}
