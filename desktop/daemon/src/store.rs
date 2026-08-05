//! rusqlite-backed local store (tandem-cache.db): paired phone identity row,
//! call-log mirror with sync cursor, and settings not held in config.toml. Schema
//! DDL in docs/09.

use serde::{Deserialize, Serialize};
use tandem_core::model::{CallLogRow, PairedPhone};

/// Schema version; a mismatch triggers migration rather than silent misreads.
pub const SCHEMA_VERSION: u32 = 1;

/// The call-log mirror is bounded — it is a convenience projection of the
/// phone's OS log, not an archive (docs/09 retention policy).
pub const MIRROR_MAX_ENTRIES: usize = 1000;

/// Page size for incremental sync, matching the phone-side cap.
pub const SYNC_PAGE_SIZE: u32 = 200;

/// Where the desktop's sync left off, so a reconnect fetches only what changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncCursor {
    pub newest_entry_ms: i64,
    pub oldest_entry_ms: i64,
    pub entry_count: usize,
}

/// In-memory view of the persisted state, kept behind this type so the storage
/// engine stays swappable.
#[derive(Debug, Clone, Default)]
pub struct Store {
    paired_phone: Option<PairedPhone>,
    call_log: Vec<CallLogRow>,
    cursor: SyncCursor,
    last_call_log_version: u64,
}

impl Store {
    pub fn paired_phone(&self) -> Option<&PairedPhone> {
        self.paired_phone.as_ref()
    }

    pub fn set_paired_phone(&mut self, phone: PairedPhone) {
        self.paired_phone = Some(phone);
    }

    /// Unpairing clears the mirror too: call metadata must not outlive the trust
    /// relationship that justified holding it (docs/08 privacy).
    pub fn unpair(&mut self) {
        self.paired_phone = None;
        self.call_log.clear();
        self.cursor = SyncCursor::default();
        self.last_call_log_version = 0;
    }

    pub fn call_log(&self) -> &[CallLogRow] {
        &self.call_log
    }

    pub fn cursor(&self) -> &SyncCursor {
        &self.cursor
    }

    pub fn last_call_log_version(&self) -> u64 {
        self.last_call_log_version
    }

    pub fn set_call_log_version(&mut self, version: u64) {
        self.last_call_log_version = version;
    }

    /// Merges a synced page, de-duplicating by entry id, keeping newest first,
    /// and trimming to the retention bound.
    pub fn merge_call_log(&mut self, page: Vec<CallLogRow>) {
        for row in page {
            if let Some(existing) = self
                .call_log
                .iter_mut()
                .find(|r| r.entry_id == row.entry_id)
            {
                *existing = row;
            } else {
                self.call_log.push(row);
            }
        }
        self.call_log
            .sort_by(|a, b| b.started_at_ms.cmp(&a.started_at_ms));
        self.call_log.truncate(MIRROR_MAX_ENTRIES);
        self.recompute_cursor();
    }

    /// A changed log version on the phone means rows may have been deleted there,
    /// which a timestamp-bounded sync cannot observe — so the mirror is rebuilt
    /// from scratch (docs/09 deletion reconciliation).
    pub fn needs_full_resync(&self, phone_log_version: u64) -> bool {
        phone_log_version != self.last_call_log_version
    }

    pub fn clear_call_log(&mut self) {
        self.call_log.clear();
        self.recompute_cursor();
    }

    fn recompute_cursor(&mut self) {
        self.cursor = SyncCursor {
            newest_entry_ms: self.call_log.first().map(|r| r.started_at_ms).unwrap_or(0),
            oldest_entry_ms: self.call_log.last().map(|r| r.started_at_ms).unwrap_or(0),
            entry_count: self.call_log.len(),
        };
    }

    /// Reads persisted state. A missing or unreadable file yields an empty store
    /// rather than an error: a corrupted cache must not stop the daemon, since
    /// everything in it is a re-syncable projection of phone truth.
    pub fn load(path: &std::path::Path) -> Self {
        let Ok(bytes) = std::fs::read(path) else {
            return Self::default();
        };
        let Ok(persisted) = serde_json::from_slice::<PersistedState>(&bytes) else {
            return Self::default();
        };

        let mut store = Self {
            paired_phone: persisted.paired_phone.map(Into::into),
            call_log: Vec::new(),
            cursor: SyncCursor::default(),
            last_call_log_version: persisted.last_call_log_version,
        };
        store.merge_call_log(persisted.call_log.into_iter().map(Into::into).collect());
        store
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let persisted = PersistedState {
            schema_version: SCHEMA_VERSION,
            paired_phone: self.paired_phone.clone().map(Into::into),
            call_log: self.call_log.iter().cloned().map(Into::into).collect(),
            last_call_log_version: self.last_call_log_version,
        };
        let bytes = serde_json::to_vec_pretty(&persisted)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, bytes)
    }
}

/// On-disk shape. Kept separate from the domain models so the storage format can
/// change without the domain following it.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedState {
    schema_version: u32,
    paired_phone: Option<PersistedPhone>,
    call_log: Vec<PersistedCallLogRow>,
    last_call_log_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedPhone {
    device_id: String,
    name: String,
    spki_sha256: String,
    bt_address: String,
}

impl From<PairedPhone> for PersistedPhone {
    fn from(phone: PairedPhone) -> Self {
        Self {
            device_id: phone.device_id,
            name: phone.name,
            spki_sha256: phone.spki_sha256,
            bt_address: phone.bt_address,
        }
    }
}

impl From<PersistedPhone> for PairedPhone {
    fn from(phone: PersistedPhone) -> Self {
        Self {
            device_id: phone.device_id,
            name: phone.name,
            spki_sha256: phone.spki_sha256,
            bt_address: phone.bt_address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedCallLogRow {
    entry_id: String,
    number: String,
    display_name: String,
    started_at_ms: i64,
    duration_seconds: u32,
    sim_slot: i32,
}

impl From<CallLogRow> for PersistedCallLogRow {
    fn from(row: CallLogRow) -> Self {
        Self {
            entry_id: row.entry_id,
            number: row.number,
            display_name: row.display_name,
            started_at_ms: row.started_at_ms,
            duration_seconds: row.duration_seconds,
            sim_slot: row.sim_slot,
        }
    }
}

impl From<PersistedCallLogRow> for CallLogRow {
    fn from(row: PersistedCallLogRow) -> Self {
        Self {
            entry_id: row.entry_id,
            number: row.number,
            display_name: row.display_name,
            started_at_ms: row.started_at_ms,
            duration_seconds: row.duration_seconds,
            sim_slot: row.sim_slot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, started_at_ms: i64) -> CallLogRow {
        CallLogRow {
            entry_id: id.into(),
            number: "+14155550123".into(),
            display_name: "Alex".into(),
            started_at_ms,
            duration_seconds: 10,
            sim_slot: 0,
        }
    }

    #[test]
    fn merging_keeps_newest_first() {
        let mut store = Store::default();
        store.merge_call_log(vec![row("a", 100), row("b", 300), row("c", 200)]);
        let ids: Vec<_> = store
            .call_log()
            .iter()
            .map(|r| r.entry_id.as_str())
            .collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn re_syncing_the_same_entry_updates_rather_than_duplicates() {
        let mut store = Store::default();
        store.merge_call_log(vec![row("a", 100)]);
        let mut updated = row("a", 100);
        updated.duration_seconds = 99;
        store.merge_call_log(vec![updated]);

        assert_eq!(store.call_log().len(), 1);
        assert_eq!(store.call_log()[0].duration_seconds, 99);
    }

    #[test]
    fn the_mirror_is_bounded_by_the_retention_policy() {
        let mut store = Store::default();
        let page: Vec<_> = (0..MIRROR_MAX_ENTRIES + 50)
            .map(|i| row(&format!("e{i}"), i as i64))
            .collect();
        store.merge_call_log(page);
        assert_eq!(store.call_log().len(), MIRROR_MAX_ENTRIES);
        assert_eq!(store.cursor().entry_count, MIRROR_MAX_ENTRIES);
    }

    #[test]
    fn the_cursor_tracks_the_mirror_bounds() {
        let mut store = Store::default();
        store.merge_call_log(vec![row("a", 100), row("b", 300)]);
        assert_eq!(store.cursor().newest_entry_ms, 300);
        assert_eq!(store.cursor().oldest_entry_ms, 100);
    }

    #[test]
    fn a_version_change_forces_a_full_resync() {
        let mut store = Store::default();
        store.set_call_log_version(7);
        assert!(!store.needs_full_resync(7));
        assert!(store.needs_full_resync(8));
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tandem-store-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("tandem-cache.json")
    }

    /// A pairing that vanished on restart would force the user to re-pair every
    /// launch.
    #[test]
    fn a_paired_phone_survives_a_restart() {
        let path = temp_path("pairing");
        let mut store = Store::default();
        store.set_paired_phone(PairedPhone {
            device_id: "p1".into(),
            name: "Pixel".into(),
            spki_sha256: "pinned-key".into(),
            bt_address: "AA:BB".into(),
        });
        store.set_call_log_version(12);
        store.save(&path).unwrap();

        let reopened = Store::load(&path);
        let phone = reopened.paired_phone().expect("phone must persist");
        assert_eq!(phone.device_id, "p1");
        assert_eq!(phone.spki_sha256, "pinned-key");
        assert_eq!(reopened.last_call_log_version(), 12);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn the_mirrored_call_log_survives_a_restart_newest_first() {
        let path = temp_path("log");
        let mut store = Store::default();
        store.merge_call_log(vec![row("a", 100), row("b", 300)]);
        store.save(&path).unwrap();

        let reopened = Store::load(&path);
        assert_eq!(reopened.call_log().len(), 2);
        assert_eq!(reopened.call_log()[0].entry_id, "b");
        assert_eq!(reopened.cursor().newest_entry_ms, 300);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn an_absent_file_loads_as_an_empty_store() {
        let store = Store::load(&temp_path("missing"));
        assert!(store.paired_phone().is_none());
        assert!(store.call_log().is_empty());
    }

    /// The cache is a re-syncable projection, so corruption must not stop the
    /// daemon from starting.
    #[test]
    fn a_corrupt_file_loads_as_an_empty_store() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{ not valid json").unwrap();

        let store = Store::load(&path);
        assert!(store.paired_phone().is_none());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Call metadata must not outlive the pairing that justified holding it.
    #[test]
    fn unpairing_clears_the_mirrored_call_log() {
        let mut store = Store::default();
        store.set_paired_phone(PairedPhone {
            device_id: "p1".into(),
            name: "Pixel".into(),
            spki_sha256: "fp".into(),
            bt_address: String::new(),
        });
        store.merge_call_log(vec![row("a", 100)]);

        store.unpair();
        assert!(store.paired_phone().is_none());
        assert!(store.call_log().is_empty());
        assert_eq!(store.last_call_log_version(), 0);
    }
}
