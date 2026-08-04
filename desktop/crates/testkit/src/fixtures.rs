//! Canonical fixtures: sample CallSnapshots, call-log pages, QR payloads,
//! certificates, and keys used across unit and integration tests.

use tandem_core::model::{
    AudioRoute, Call, CallDirection, CallLogRow, CallSnapshot, CallState, StateVersion,
};

pub const PHONE_DEVICE_ID: &str = "phone-0000-0000-0000";
pub const DESKTOP_DEVICE_ID: &str = "desktop-0000-0000-0000";
pub const EPOCH: &str = "epoch-0000";
pub const REMOTE_NUMBER: &str = "+14155550123";

pub fn version(state_seq: u64) -> StateVersion {
    StateVersion {
        epoch_id: EPOCH.into(),
        state_seq,
    }
}

pub fn call(call_id: &str, state: CallState) -> Call {
    Call {
        call_id: call_id.into(),
        state,
        direction: CallDirection::Incoming,
        remote_number: REMOTE_NUMBER.into(),
        remote_display_name: "Alex".into(),
        started_at_ms: 1_700_000_000_000,
        is_conference: false,
        can_hold: true,
        can_merge: false,
        is_emergency: false,
        sim_slot: 0,
    }
}

/// An emergency call as the phone would surface it: read-only, never remotely
/// controllable (ADR-0008).
pub fn emergency_call(call_id: &str) -> Call {
    Call {
        is_emergency: true,
        remote_number: "911".into(),
        remote_display_name: String::new(),
        direction: CallDirection::Outgoing,
        ..call(call_id, CallState::Active)
    }
}

pub fn empty_snapshot(state_seq: u64) -> CallSnapshot {
    CallSnapshot {
        version: version(state_seq),
        calls: Vec::new(),
        audio_route: AudioRoute::Earpiece,
        microphone_muted: false,
        bt_route_address: String::new(),
    }
}

pub fn snapshot_with(calls: Vec<Call>, state_seq: u64) -> CallSnapshot {
    CallSnapshot {
        calls,
        ..empty_snapshot(state_seq)
    }
}

pub fn call_log_page(count: usize) -> Vec<CallLogRow> {
    (0..count)
        .map(|i| CallLogRow {
            entry_id: format!("entry-{i}"),
            number: REMOTE_NUMBER.into(),
            display_name: "Alex".into(),
            started_at_ms: 1_700_000_000_000 - (i as i64 * 60_000),
            duration_seconds: 42,
            sim_slot: 0,
        })
        .collect()
}

/// A well-formed pairing QR payload; `fp` must be a valid base64url SPKI hash.
pub fn qr_payload(fingerprint_b64url: &str) -> String {
    format!(
        r#"{{"v":1,"host":"192.168.1.20","port":46521,"fp":"{fingerprint_b64url}","tok":"token-0000","name":"Pixel"}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_carry_a_consistent_version() {
        let snapshot = snapshot_with(vec![call("c1", CallState::Ringing)], 5);
        assert_eq!(snapshot.version.state_seq, 5);
        assert_eq!(snapshot.version.epoch_id, EPOCH);
        assert_eq!(snapshot.calls.len(), 1);
    }

    #[test]
    fn emergency_fixture_is_flagged_and_active() {
        let snapshot = snapshot_with(vec![emergency_call("c9")], 1);
        assert!(snapshot.has_active_emergency());
    }

    #[test]
    fn call_log_pages_are_ordered_newest_first() {
        let page = call_log_page(3);
        assert_eq!(page.len(), 3);
        assert!(page[0].started_at_ms > page[1].started_at_ms);
    }
}
