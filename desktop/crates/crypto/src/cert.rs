//! Generates the long-lived self-signed X.509 certificate over the identity key
//! (rcgen) used as the TLS carrier for mutual authentication.

use crate::error::CryptoError;

/// Certificates are long-lived because trust is the pinned key, not expiry
/// (docs/08 key-rotation section).
pub const VALIDITY_DAYS: u32 = 3650;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfSignedCert {
    pub cert_der: Vec<u8>,
    pub spki_der: Vec<u8>,
}

/// Issues a self-signed certificate wrapping the identity key.
pub fn issue(_common_name: &str) -> Result<SelfSignedCert, CryptoError> {
    Err(CryptoError::Certificate)
}

/// Extracts the SubjectPublicKeyInfo from a DER certificate for pinning.
pub fn spki_from_cert_der(_cert_der: &[u8]) -> Result<Vec<u8>, CryptoError> {
    Err(CryptoError::Certificate)
}
