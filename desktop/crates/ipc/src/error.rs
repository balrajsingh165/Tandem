//! IpcError: connect, protocol, timeout, and daemon-unavailable failures with
//! UI-facing retry guidance.

use thiserror::Error;

/// JSON-RPC error codes. The reserved range is standard; Tandem's own codes
/// start at -32000 and are stable, so the UI can branch on them.
pub const IPC_DAEMON_UNAVAILABLE: i32 = -32000;
pub const IPC_NOT_PAIRED: i32 = -32001;
pub const IPC_PHONE_OFFLINE: i32 = -32002;
pub const IPC_CALL_NOT_FOUND: i32 = -32003;
pub const IPC_INVALID_CALL_STATE: i32 = -32004;
pub const IPC_EMERGENCY_BLOCKED: i32 = -32005;
pub const IPC_ALREADY_HANDLED: i32 = -32006;
pub const IPC_RATE_LIMITED: i32 = -32007;
pub const IPC_AUDIO_UNAVAILABLE: i32 = -32008;
pub const IPC_PAIRING_FAILED: i32 = -32009;
pub const IPC_TIMEOUT: i32 = -32010;
pub const IPC_UNAUTHORIZED: i32 = -32011;
pub const IPC_INTERNAL: i32 = -32099;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IpcError {
    #[error("the Tandem daemon is not running")]
    DaemonUnavailable,

    #[error("no phone is paired with this desktop")]
    NotPaired,

    #[error("the phone is not reachable on this network")]
    PhoneOffline,

    #[error("no call with id {0}")]
    CallNotFound(String),

    #[error("that call cannot accept this command right now")]
    InvalidCallState,

    #[error("{number} is an emergency number; dial it on the handset")]
    EmergencyBlocked { number: String },

    #[error("another desktop already handled this call")]
    AlreadyHandled,

    #[error("too many dial attempts; wait a moment")]
    RateLimited,

    #[error("desktop audio is not available")]
    AudioUnavailable,

    #[error("pairing failed: {0}")]
    PairingFailed(String),

    #[error("the daemon did not respond in time")]
    Timeout,

    #[error("not authorized: re-pair with the phone")]
    Unauthorized,

    #[error("internal daemon error")]
    Internal,
}

impl IpcError {
    pub fn code(&self) -> i32 {
        match self {
            Self::DaemonUnavailable => IPC_DAEMON_UNAVAILABLE,
            Self::NotPaired => IPC_NOT_PAIRED,
            Self::PhoneOffline => IPC_PHONE_OFFLINE,
            Self::CallNotFound(_) => IPC_CALL_NOT_FOUND,
            Self::InvalidCallState => IPC_INVALID_CALL_STATE,
            Self::EmergencyBlocked { .. } => IPC_EMERGENCY_BLOCKED,
            Self::AlreadyHandled => IPC_ALREADY_HANDLED,
            Self::RateLimited => IPC_RATE_LIMITED,
            Self::AudioUnavailable => IPC_AUDIO_UNAVAILABLE,
            Self::PairingFailed(_) => IPC_PAIRING_FAILED,
            Self::Timeout => IPC_TIMEOUT,
            Self::Unauthorized => IPC_UNAUTHORIZED,
            Self::Internal => IPC_INTERNAL,
        }
    }

    /// Whether the UI should offer a retry. Losing a race or hitting the
    /// emergency policy is not retryable — retrying would be wrong, not just
    /// futile.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::DaemonUnavailable | Self::PhoneOffline | Self::Timeout | Self::Internal
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_distinct_so_the_ui_can_branch_on_it() {
        let codes = [
            IpcError::DaemonUnavailable.code(),
            IpcError::NotPaired.code(),
            IpcError::PhoneOffline.code(),
            IpcError::CallNotFound("c".into()).code(),
            IpcError::InvalidCallState.code(),
            IpcError::EmergencyBlocked {
                number: "911".into(),
            }
            .code(),
            IpcError::AlreadyHandled.code(),
            IpcError::RateLimited.code(),
            IpcError::AudioUnavailable.code(),
            IpcError::PairingFailed("x".into()).code(),
            IpcError::Timeout.code(),
            IpcError::Unauthorized.code(),
            IpcError::Internal.code(),
        ];
        let mut sorted = codes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len());
    }

    #[test]
    fn transient_failures_offer_retry() {
        assert!(IpcError::DaemonUnavailable.is_retryable());
        assert!(IpcError::PhoneOffline.is_retryable());
        assert!(IpcError::Timeout.is_retryable());
    }

    /// Retrying an emergency-blocked dial or a lost answer race would be wrong,
    /// not merely useless.
    #[test]
    fn policy_and_race_outcomes_are_never_retryable() {
        assert!(!IpcError::EmergencyBlocked {
            number: "911".into()
        }
        .is_retryable());
        assert!(!IpcError::AlreadyHandled.is_retryable());
        assert!(!IpcError::RateLimited.is_retryable());
    }
}
