//! Creates the desktop's P-256 identity keypair on first run and loads it
//! thereafter; the private key lives in the OS secret store via secrets.rs, with
//! an encrypted-file fallback.

use crate::cert;
use crate::error::CryptoError;
use crate::pinning::SpkiFingerprint;
use crate::secrets::SecretStore;

/// Secret-store keys. The private key and its public artifacts are stored
/// separately so a reader of the public parts never touches key material.
const KEY_PRIVATE: &str = "tandem.identity.key";
const KEY_CERT: &str = "tandem.identity.cert";
const KEY_SPKI: &str = "tandem.identity.spki";

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

/// The identity plus the private key needed to present it in a TLS handshake.
/// Only the transport layer needs this; everything else takes `DeviceIdentity`.
#[derive(Debug, Clone)]
pub struct IdentityCredentials {
    pub identity: DeviceIdentity,
    pub key_der: Vec<u8>,
}

/// Loads the existing identity or creates one on first run. Creation is
/// idempotent: a second call returns the same key rather than rotating it, since
/// rotating would silently break every existing pairing.
pub fn load_or_create(
    store: &dyn SecretStore,
    display_name: &str,
) -> Result<IdentityCredentials, CryptoError> {
    if let Some(credentials) = load(store, display_name)? {
        return Ok(credentials);
    }

    let issued = cert::issue(COMMON_NAME)?;
    store.set(KEY_PRIVATE, &issued.key_der)?;
    store.set(KEY_CERT, &issued.cert_der)?;
    store.set(KEY_SPKI, &issued.spki_der)?;

    Ok(build(
        display_name,
        issued.spki_der,
        issued.cert_der,
        issued.key_der,
    ))
}

/// Returns the stored identity, or None when this desktop has none yet.
pub fn load(
    store: &dyn SecretStore,
    display_name: &str,
) -> Result<Option<IdentityCredentials>, CryptoError> {
    let (Some(key_der), Some(cert_der), Some(spki_der)) = (
        store.get(KEY_PRIVATE)?,
        store.get(KEY_CERT)?,
        store.get(KEY_SPKI)?,
    ) else {
        return Ok(None);
    };

    Ok(Some(build(display_name, spki_der, cert_der, key_der)))
}

/// Discards the identity. Every pairing that pinned it becomes unusable, so this
/// is the "re-pair from scratch" path in docs/07, not a routine operation.
pub fn reset(store: &dyn SecretStore) -> Result<(), CryptoError> {
    store.delete(KEY_PRIVATE)?;
    store.delete(KEY_CERT)?;
    store.delete(KEY_SPKI)?;
    Ok(())
}

/// The device id is derived from the pinned key, so it is stable across restarts
/// without needing its own persisted value. The phone assigns its own id for the
/// desktop at pairing; this one identifies the key locally.
fn build(
    display_name: &str,
    spki_der: Vec<u8>,
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
) -> IdentityCredentials {
    let device_id = SpkiFingerprint::from_spki_der(&spki_der).to_base64url();
    IdentityCredentials {
        identity: DeviceIdentity {
            device_id,
            display_name: display_name.to_string(),
            spki_der,
            cert_der,
        },
        key_der,
    }
}

const COMMON_NAME: &str = "Tandem Desktop";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::InMemorySecretStore;

    #[test]
    fn first_run_creates_an_identity() {
        let store = InMemorySecretStore::default();
        let created = load_or_create(&store, "Workstation").unwrap();

        assert!(!created.identity.spki_der.is_empty());
        assert!(!created.identity.cert_der.is_empty());
        assert!(!created.key_der.is_empty());
        assert_eq!(created.identity.display_name, "Workstation");
    }

    /// Rotating on every start would silently break every existing pairing.
    #[test]
    fn a_second_run_returns_the_same_key() {
        let store = InMemorySecretStore::default();
        let first = load_or_create(&store, "Workstation").unwrap();
        let second = load_or_create(&store, "Workstation").unwrap();

        assert_eq!(first.identity.fingerprint(), second.identity.fingerprint());
        assert_eq!(first.key_der, second.key_der);
        assert_eq!(first.identity.device_id, second.identity.device_id);
    }

    #[test]
    fn nothing_is_stored_before_first_run() {
        let store = InMemorySecretStore::default();
        assert!(load(&store, "Workstation").unwrap().is_none());
    }

    #[test]
    fn the_device_id_is_derived_from_the_pinned_key() {
        let store = InMemorySecretStore::default();
        let created = load_or_create(&store, "Workstation").unwrap();

        assert_eq!(
            created.identity.device_id,
            created.identity.fingerprint().to_base64url()
        );
    }

    #[test]
    fn renaming_the_desktop_does_not_change_its_key() {
        let store = InMemorySecretStore::default();
        let before = load_or_create(&store, "Old Name").unwrap();
        let after = load_or_create(&store, "New Name").unwrap();

        assert_eq!(after.identity.display_name, "New Name");
        assert_eq!(before.identity.fingerprint(), after.identity.fingerprint());
    }

    /// Reset is the re-pair-from-scratch path: the new identity must differ.
    #[test]
    fn reset_forces_a_new_identity() {
        let store = InMemorySecretStore::default();
        let before = load_or_create(&store, "Workstation").unwrap();

        reset(&store).unwrap();
        assert!(load(&store, "Workstation").unwrap().is_none());

        let after = load_or_create(&store, "Workstation").unwrap();
        assert_ne!(before.identity.fingerprint(), after.identity.fingerprint());
    }

    /// A half-written store must not yield a broken identity.
    #[test]
    fn a_partial_store_is_treated_as_absent() {
        let store = InMemorySecretStore::default();
        load_or_create(&store, "Workstation").unwrap();
        store.delete(KEY_CERT).unwrap();

        assert!(load(&store, "Workstation").unwrap().is_none());
    }
}
