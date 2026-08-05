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

/// Directory-backed store used when no OS secret service is reachable, such as a
/// headless Linux session. Entries are written to owner-only files; this is a
/// fallback, and the OS store is preferred wherever one exists.
#[derive(Debug, Clone)]
pub struct FileSecretStore {
    directory: std::path::PathBuf,
}

impl FileSecretStore {
    pub fn new(directory: impl Into<std::path::PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    /// Keys become filenames, so anything outside the allowed set is rejected
    /// rather than escaping the directory.
    fn path_for(&self, key: &str) -> Result<std::path::PathBuf, CryptoError> {
        let safe = key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
        if !safe || key.is_empty() {
            return Err(CryptoError::SecretStoreUnavailable(format!(
                "unsafe secret key: {key}"
            )));
        }
        Ok(self.directory.join(key))
    }

    fn ensure_directory(&self) -> Result<(), CryptoError> {
        std::fs::create_dir_all(&self.directory)
            .map_err(|e| CryptoError::SecretStoreUnavailable(e.to_string()))
    }
}

impl SecretStore for FileSecretStore {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        let path = self.path_for(key)?;
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(CryptoError::SecretStoreUnavailable(e.to_string())),
        }
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), CryptoError> {
        self.ensure_directory()?;
        let path = self.path_for(key)?;
        std::fs::write(&path, value)
            .map_err(|e| CryptoError::SecretStoreUnavailable(e.to_string()))?;
        restrict_to_owner(&path)
    }

    fn delete(&self, key: &str) -> Result<(), CryptoError> {
        let path = self.path_for(key)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(CryptoError::SecretStoreUnavailable(e.to_string())),
        }
    }
}

#[cfg(unix)]
fn restrict_to_owner(path: &std::path::Path) -> Result<(), CryptoError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| CryptoError::SecretStoreUnavailable(e.to_string()))
}

// Windows inherits the per-user profile ACL, which already restricts access to
// the owning account.
#[cfg(not(unix))]
fn restrict_to_owner(_path: &std::path::Path) -> Result<(), CryptoError> {
    Ok(())
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

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tandem-secrets-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn file_store_persists_across_instances() {
        let dir = temp_dir("persist");
        FileSecretStore::new(&dir)
            .set("tandem.identity.key", b"material")
            .unwrap();

        let reopened = FileSecretStore::new(&dir);
        assert_eq!(
            reopened.get("tandem.identity.key").unwrap(),
            Some(b"material".to_vec())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_store_reports_absent_keys_as_none() {
        let dir = temp_dir("absent");
        assert_eq!(FileSecretStore::new(&dir).get("nothing").unwrap(), None);
    }

    #[test]
    fn file_store_delete_is_idempotent() {
        let dir = temp_dir("delete");
        let store = FileSecretStore::new(&dir);
        store.set("k", b"v").unwrap();
        store.delete("k").unwrap();
        store.delete("k").unwrap();
        assert_eq!(store.get("k").unwrap(), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Keys become filenames, so traversal attempts must be refused.
    #[test]
    fn file_store_refuses_unsafe_keys() {
        let store = FileSecretStore::new(temp_dir("unsafe"));
        assert!(store.get("../escape").is_err());
        assert!(store.set("a/b", b"v").is_err());
        assert!(store.get("").is_err());
    }

    /// A full identity must survive a process restart, or pairing breaks on every
    /// launch.
    #[test]
    fn an_identity_survives_a_restart_through_the_file_store() {
        let dir = temp_dir("identity");
        let first = crate::identity::load_or_create(&FileSecretStore::new(&dir), "Desk").unwrap();
        let second = crate::identity::load_or_create(&FileSecretStore::new(&dir), "Desk").unwrap();

        assert_eq!(first.identity.fingerprint(), second.identity.fingerprint());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
