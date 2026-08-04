//! PairingError: invalid QR, token expired, fingerprint mismatch, user
//! rejection, and version-negotiation failures, each mapped to actionable UI
//! copy.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PairingError {
    #[error("this QR code is not a Tandem pairing code")]
    InvalidQr,

    #[error("pairing code version {0} is not supported by this desktop")]
    UnsupportedQrVersion(u32),

    #[error("the pairing code has expired; generate a new one on the phone")]
    TokenExpired,

    #[error("the phone presented a different key than the QR code promised")]
    FingerprintMismatch,

    #[error("the short codes do not match; do not continue")]
    ShortCodeMismatch,

    #[error("pairing was declined on the phone")]
    RejectedByUser,

    #[error("no mutually supported protocol version: desktop {desktop_min}..={desktop_max}")]
    VersionNegotiationFailed { desktop_min: u32, desktop_max: u32 },

    #[error("pairing failed: {0}")]
    Transport(String),
}
