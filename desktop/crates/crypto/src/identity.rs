//! Creates the desktop's P-256 identity keypair on first run and loads it
//! thereafter; the private key lives in the OS secret store via secrets.rs, with
//! an encrypted-file fallback.

use crate::error::CryptoError;
use crate::pinning::SpkiFingerprint;
use crate::secrets::SecretStore;

/// The desktop's public identity. Private key material never appears here; it
/// stays inside the secret store and is used only through signing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub display_name: String,
    pub spki_der: Vec<u8>,
    pub cert_der: Vec<u8>,
}

impl DeviceIdentity {
    pub fn fingerprint(&self) -> SpkiFingerprint {
        SpkiFingerprint::from_spki_der(&self.spki_der)
    }
}

/// Loads the existing identity or creates one on first run.
pub fn load_or_create(
    _store: &dyn SecretStore,
    _display_name: &str,
) -> Result<DeviceIdentity, CryptoError> {
    Err(CryptoError::KeyGeneration)
}
