//! TransportError: discovery, TLS/pinning, session-handshake, timeout, and
//! protocol-violation failures, with retryability annotations consumed by
//! reconnect.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransportError {
    #[error("no paired phone found on this network")]
    PhoneNotDiscovered,

    #[error("could not reach {endpoint}: {reason}")]
    ConnectFailed { endpoint: String, reason: String },

    #[error("the phone presented an unexpected key; refusing to connect")]
    PinMismatch,

    #[error("TLS handshake failed: {0}")]
    TlsHandshake(String),

    #[error("this desktop is not authenticated; re-pair with the phone")]
    Unauthenticated,

    #[error("phone rejected protocol version {requested}")]
    VersionUnsupported { requested: u32 },

    #[error("request {message_id} timed out")]
    RequestTimeout { message_id: u64 },

    #[error("peer went silent for longer than the dead-peer timeout")]
    PeerSilent,

    #[error("protocol violation: {0}")]
    ProtocolViolation(String),

    #[error("frame of {size} bytes exceeds the {max} byte limit")]
    FrameTooLarge { size: usize, max: usize },

    #[error("this desktop's authorization was revoked: {0}")]
    Revoked(String),

    #[error("connection closed")]
    Closed,
}

impl TransportError {
    /// Whether the reconnect loop should keep trying. Trust and version failures
    /// are terminal: retrying cannot fix a wrong key or an unsupported protocol.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::PhoneNotDiscovered
            | Self::ConnectFailed { .. }
            | Self::TlsHandshake(_)
            | Self::RequestTimeout { .. }
            | Self::PeerSilent
            | Self::Closed => true,

            Self::PinMismatch
            | Self::Unauthenticated
            | Self::VersionUnsupported { .. }
            | Self::ProtocolViolation(_)
            | Self::FrameTooLarge { .. }
            | Self::Revoked(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_network_failures_are_retryable() {
        assert!(TransportError::PeerSilent.is_retryable());
        assert!(TransportError::Closed.is_retryable());
        assert!(TransportError::PhoneNotDiscovered.is_retryable());
    }

    #[test]
    fn trust_failures_are_terminal() {
        assert!(!TransportError::PinMismatch.is_retryable());
        assert!(!TransportError::Revoked("removed".into()).is_retryable());
        assert!(!TransportError::Unauthenticated.is_retryable());
        assert!(!TransportError::VersionUnsupported { requested: 9 }.is_retryable());
    }
}
