//! rusqlite-backed local store (tandem-cache.db): paired phone identity row,
//! call-log mirror with sync cursor, and settings not held in config.toml. Schema
//! DDL in docs/09.

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
