//! CryptoError: key-generation, secret-store access, certificate, and
//! pin-verification failures. Never carries key material in messages.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CryptoError {
    #[error("identity key generation failed")]
    KeyGeneration,

    #[error("no device identity has been created yet")]
    IdentityMissing,

    #[error("secret store unavailable: {0}")]
    SecretStoreUnavailable(String),

    #[error("certificate could not be parsed or generated")]
    Certificate,

    #[error("peer key does not match the pinned fingerprint")]
    PinMismatch,

    #[error("fingerprint is malformed")]
    MalformedFingerprint,
}
