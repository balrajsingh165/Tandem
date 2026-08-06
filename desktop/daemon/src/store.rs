//! Local store (tandem-cache.json): paired phone identities, a per-phone
//! call-log mirror with its own sync cursor, and settings not held in config.toml.
//! Schema DDL in docs/09.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tandem_core::model::{CallLogRow, PairedPhone};

/// Schema version; a mismatch triggers migration rather than silent misreads.
pub const SCHEMA_VERSION: u32 = 2;

/// The call-log mirror is bounded — it is a projection of the phone's OS log, not
/// an archive (docs/09 retention policy). Large enough to hold a typical phone's
/// entire log so "recents" is really the whole history, small enough that the
/// cache stays a file the daemon can rewrite cheaply.
pub const MIRROR_MAX_ENTRIES: usize = 5000;

/// Page size for incremental sync, matching the phone-side cap.
pub const SYNC_PAGE_SIZE: u32 = 200;

/// Where the desktop's sync left off, so a reconnect fetches only what changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncCursor {
    pub newest_entry_ms: i64,
    pub oldest_entry_ms: i64,
    pub entry_count: usize,
}

/// One phone's mirrored log. Held per phone because each has its own OS log and
/// its own version counter; merging them would make deletions unreconcilable.
#[derive(Debug, Clone, Default)]
pub struct PhoneLog {
    rows: Vec<CallLogRow>,
    cursor: SyncCursor,
    version: u64,
}

/// In-memory view of the persisted state, kept behind this type so the storage
/// engine stays swappable.
#[derive(Debug, Clone, Default)]
pub struct Store {
    phones: Vec<PairedPhone>,
    logs: HashMap<String, PhoneLog>,
}

impl Store {
    pub fn phones(&self) -> &[PairedPhone] {
        &self.phones
    }

    pub fn phone(&self, device_id: &str) -> Option<&PairedPhone> {
        self.phones.iter().find(|p| p.device_id == device_id)
    }

    /// Re-pairing the same phone replaces its record rather than duplicating it,
    /// since the device id is the identity the desktop keys everything by.
    pub fn add_phone(&mut self, phone: PairedPhone) {
        match self
            .phones
            .iter_mut()
            .find(|p| p.device_id == phone.device_id)
        {
            Some(existing) => *existing = phone,
            None => self.phones.push(phone),
        }
    }

    /// Removing a phone clears its mirror too: call metadata must not outlive the
    /// trust relationship that justified holding it (docs/08 privacy).
    pub fn remove_phone(&mut self, device_id: &str) {
        self.phones.retain(|p| p.device_id != device_id);
        self.logs.remove(device_id);
    }

    pub fn call_log(&self, device_id: &str) -> &[CallLogRow] {
        self.logs
            .get(device_id)
            .map(|log| log.rows.as_slice())
            .unwrap_or_default()
    }

    /// Every phone's rows newest-first, for a combined recents list.
    pub fn all_call_log(&self) -> Vec<(&str, &CallLogRow)> {
        let mut merged: Vec<(&str, &CallLogRow)> = self
            .logs
            .iter()
            .flat_map(|(id, log)| log.rows.iter().map(move |row| (id.as_str(), row)))
            .collect();
        merged.sort_by(|a, b| b.1.started_at_ms.cmp(&a.1.started_at_ms));
        merged
    }

    pub fn cursor(&self, device_id: &str) -> SyncCursor {
        self.logs
            .get(device_id)
            .map(|log| log.cursor.clone())
            .unwrap_or_default()
    }

    pub fn last_call_log_version(&self, device_id: &str) -> u64 {
        self.logs.get(device_id).map(|log| log.version).unwrap_or(0)
    }

    pub fn set_call_log_version(&mut self, device_id: &str, version: u64) {
        self.logs.entry(device_id.to_string()).or_default().version = version;
    }

    /// Merges a synced page, de-duplicating by entry id, keeping newest first,
    /// and trimming to the retention bound.
    pub fn merge_call_log(&mut self, device_id: &str, page: Vec<CallLogRow>) {
        let log = self.logs.entry(device_id.to_string()).or_default();

        for row in page {
            match log.rows.iter_mut().find(|r| r.entry_id == row.entry_id) {
                Some(existing) => *existing = row,
                None => log.rows.push(row),
            }
        }
        log.rows.sort_by(|a, b| b.started_at_ms.cmp(&a.started_at_ms));
        log.rows.truncate(MIRROR_MAX_ENTRIES);
        log.cursor = cursor_for(&log.rows);
    }

    /// A changed log version on the phone means rows may have been deleted there,
    /// which a timestamp-bounded sync cannot observe — so the mirror is rebuilt
    /// from scratch (docs/09 deletion reconciliation).
    pub fn needs_full_resync(&self, device_id: &str, phone_log_version: u64) -> bool {
        phone_log_version != self.last_call_log_version(device_id)
    }

    pub fn clear_call_log(&mut self, device_id: &str) {
        if let Some(log) = self.logs.get_mut(device_id) {
            log.rows.clear();
            log.cursor = SyncCursor::default();
        }
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

        let mut store = Self::default();

        // A v1 file held a single phone and an untagged log; adopting it under that
        // phone's own id keeps a working pairing across the upgrade.
        if let Some(legacy) = persisted.paired_phone {
            let phone: PairedPhone = legacy.into();
            let id = phone.device_id.clone();
            store.add_phone(phone);
            store.set_call_log_version(&id, persisted.last_call_log_version);
            store.merge_call_log(&id, persisted.call_log.into_iter().map(Into::into).collect());
        }

        for entry in persisted.paired_phones {
            let phone: PairedPhone = entry.phone.into();
            let id = phone.device_id.clone();
            store.add_phone(phone);
            store.set_call_log_version(&id, entry.last_call_log_version);
            store.merge_call_log(&id, entry.call_log.into_iter().map(Into::into).collect());
        }

        store
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let persisted = PersistedState {
            schema_version: SCHEMA_VERSION,
            paired_phone: None,
            call_log: Vec::new(),
            last_call_log_version: 0,
            paired_phones: self
                .phones
                .iter()
                .map(|phone| PersistedPairing {
                    phone: phone.clone().into(),
                    call_log: self
                        .call_log(&phone.device_id)
                        .iter()
                        .cloned()
                        .map(Into::into)
                        .collect(),
                    last_call_log_version: self.last_call_log_version(&phone.device_id),
                })
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&persisted)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, bytes)
    }
}

fn cursor_for(rows: &[CallLogRow]) -> SyncCursor {
    SyncCursor {
        newest_entry_ms: rows.first().map(|r| r.started_at_ms).unwrap_or(0),
        oldest_entry_ms: rows.last().map(|r| r.started_at_ms).unwrap_or(0),
        entry_count: rows.len(),
    }
}

/// On-disk shape. Kept separate from the domain models so the storage format can
/// change without the domain following it.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedState {
    schema_version: u32,
    /// v1 only: one phone and an untagged log. Read on load, never written.
    #[serde(default)]
    paired_phone: Option<PersistedPhone>,
    #[serde(default)]
    call_log: Vec<PersistedCallLogRow>,
    #[serde(default)]
    last_call_log_version: u64,
    #[serde(default)]
    paired_phones: Vec<PersistedPairing>,
}

/// One phone and the mirror belonging to it.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedPairing {
    phone: PersistedPhone,
    #[serde(default)]
    call_log: Vec<PersistedCallLogRow>,
    #[serde(default)]
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

    const P: &str = "phone-1";

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
        store.merge_call_log(P, vec![row("a", 100), row("b", 300), row("c", 200)]);
        let ids: Vec<_> = store
            .call_log("phone-1")
            .iter()
            .map(|r| r.entry_id.as_str())
            .collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn re_syncing_the_same_entry_updates_rather_than_duplicates() {
        let mut store = Store::default();
        store.merge_call_log(P, vec![row("a", 100)]);
        let mut updated = row("a", 100);
        updated.duration_seconds = 99;
        store.merge_call_log(P, vec![updated]);

        assert_eq!(store.call_log(P).len(), 1);
        assert_eq!(store.call_log(P)[0].duration_seconds, 99);
    }

    #[test]
    fn the_mirror_is_bounded_by_the_retention_policy() {
        let mut store = Store::default();
        let page: Vec<_> = (0..MIRROR_MAX_ENTRIES + 50)
            .map(|i| row(&format!("e{i}"), i as i64))
            .collect();
        store.merge_call_log("phone-1", page);
        assert_eq!(store.call_log(P).len(), MIRROR_MAX_ENTRIES);
        assert_eq!(store.cursor(P).entry_count, MIRROR_MAX_ENTRIES);
    }

    #[test]
    fn the_cursor_tracks_the_mirror_bounds() {
        let mut store = Store::default();
        store.merge_call_log(P, vec![row("a", 100), row("b", 300)]);
        assert_eq!(store.cursor(P).newest_entry_ms, 300);
        assert_eq!(store.cursor(P).oldest_entry_ms, 100);
    }

    #[test]
    fn a_version_change_forces_a_full_resync() {
        let mut store = Store::default();
        store.set_call_log_version(P, 7);
        assert!(!store.needs_full_resync(P, 7));
        assert!(store.needs_full_resync(P, 8));
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
        store.add_phone(PairedPhone {
            device_id: "p1".into(),
            name: "Pixel".into(),
            spki_sha256: "pinned-key".into(),
            bt_address: "AA:BB".into(),
        });
        store.set_call_log_version("p1", 12);
        store.save(&path).unwrap();

        let reopened = Store::load(&path);
        let phone = reopened.phone("p1").expect("phone must persist");
        assert_eq!(phone.device_id, "p1");
        assert_eq!(phone.spki_sha256, "pinned-key");
        assert_eq!(reopened.last_call_log_version("p1"), 12);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    fn phone(id: &str) -> PairedPhone {
        PairedPhone {
            device_id: id.into(),
            name: format!("Phone {id}"),
            spki_sha256: format!("pin-{id}"),
            bt_address: String::new(),
        }
    }

    #[test]
    fn the_mirrored_call_log_survives_a_restart_newest_first() {
        let path = temp_path("log");
        let mut store = Store::default();
        store.add_phone(phone(P));
        store.merge_call_log(P, vec![row("a", 100), row("b", 300)]);
        store.save(&path).unwrap();

        let reopened = Store::load(&path);
        assert_eq!(reopened.call_log(P).len(), 2);
        assert_eq!(reopened.call_log(P)[0].entry_id, "b");
        assert_eq!(reopened.cursor(P).newest_entry_ms, 300);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn an_absent_file_loads_as_an_empty_store() {
        let store = Store::load(&temp_path("missing"));
        assert!(store.phones().is_empty());
        assert!(store.call_log(P).is_empty());
    }

    /// The cache is a re-syncable projection, so corruption must not stop the
    /// daemon from starting.
    #[test]
    fn a_corrupt_file_loads_as_an_empty_store() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{ not valid json").unwrap();

        let store = Store::load(&path);
        assert!(store.phones().is_empty());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Call metadata must not outlive the pairing that justified holding it.
    #[test]
    fn removing_a_phone_clears_its_mirrored_call_log() {
        let mut store = Store::default();
        store.add_phone(phone("p1"));
        store.merge_call_log("p1", vec![row("a", 100)]);
        store.set_call_log_version("p1", 4);

        store.remove_phone("p1");
        assert!(store.phone("p1").is_none());
        assert!(store.call_log("p1").is_empty());
        assert_eq!(store.last_call_log_version("p1"), 0);
    }

    /// Two phones must not see each other's calls, or recents would attribute a
    /// call to the wrong SIM.
    #[test]
    fn each_phone_keeps_its_own_log_and_version() {
        let mut store = Store::default();
        store.add_phone(phone("p1"));
        store.add_phone(phone("p2"));
        store.merge_call_log("p1", vec![row("a", 100)]);
        store.merge_call_log("p2", vec![row("b", 300)]);
        store.set_call_log_version("p1", 7);

        assert_eq!(store.call_log("p1").len(), 1);
        assert_eq!(store.call_log("p2").len(), 1);
        assert_eq!(store.last_call_log_version("p1"), 7);
        assert_eq!(store.last_call_log_version("p2"), 0);

        // The combined view is what Recents renders, newest first across phones.
        let merged = store.all_call_log();
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].1.entry_id, "b");
        assert_eq!(merged[0].0, "p2");
    }

    /// Re-pairing an existing phone must not create a second row for it.
    #[test]
    fn re_adding_a_phone_replaces_its_record() {
        let mut store = Store::default();
        store.add_phone(phone("p1"));
        let mut renamed = phone("p1");
        renamed.name = "Renamed".into();
        store.add_phone(renamed);

        assert_eq!(store.phones().len(), 1);
        assert_eq!(store.phone("p1").unwrap().name, "Renamed");
    }

    /// A v1 cache held one phone and an untagged log; the upgrade must keep the
    /// pairing rather than silently dropping it.
    #[test]
    fn a_v1_cache_is_adopted_under_the_phone_id() {
        let path = temp_path("legacy");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            br#"{"schema_version":1,
                 "paired_phone":{"device_id":"old","name":"Pixel",
                                 "spki_sha256":"pinned","bt_address":""},
                 "call_log":[{"entry_id":"a","number":"+1","display_name":"Alex",
                              "started_at_ms":500,"duration_seconds":3,"sim_slot":0}],
                 "last_call_log_version":9}"#,
        )
        .unwrap();

        let store = Store::load(&path);
        assert_eq!(store.phones().len(), 1);
        assert_eq!(store.phone("old").unwrap().spki_sha256, "pinned");
        assert_eq!(store.call_log("old").len(), 1);
        assert_eq!(store.last_call_log_version("old"), 9);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
