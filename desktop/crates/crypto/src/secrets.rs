//! Stores/loads identity-key material via the OS secret service (macOS Keychain,
//! Windows Credential Manager, Linux Secret Service) through keyring, with an
//! encrypted-file fallback for headless Linux sessions.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::CryptoError;

/// Secret custody boundary: implementations hold key material, callers hold only
/// handles and public artifacts.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError>;
    fn set(&self, key: &str, value: &[u8]) -> Result<(), CryptoError>;
    fn delete(&self, key: &str) -> Result<(), CryptoError>;
}

/// Process-lifetime store used by tests and by headless runs before an OS
/// backend is selected. Never used for production key custody.
#[derive(Debug, Default)]
pub struct InMemorySecretStore {
    entries: Mutex<HashMap<String, Vec<u8>>>,
}

impl SecretStore for InMemorySecretStore {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| CryptoError::SecretStoreUnavailable("poisoned".into()))?;
        Ok(entries.get(key).cloned())
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), CryptoError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| CryptoError::SecretStoreUnavailable("poisoned".into()))?;
        entries.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), CryptoError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| CryptoError::SecretStoreUnavailable("poisoned".into()))?;
        entries.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_round_trips_and_deletes() {
        let store = InMemorySecretStore::default();
        assert_eq!(store.get("identity").unwrap(), None);
        store.set("identity", b"material").unwrap();
        assert_eq!(store.get("identity").unwrap(), Some(b"material".to_vec()));
        store.delete("identity").unwrap();
        assert_eq!(store.get("identity").unwrap(), None);
    }
}
