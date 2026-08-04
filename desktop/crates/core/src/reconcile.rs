//! Reconciliation after reconnect: compares (epoch_id, state_seq) against
//! ResumeResponse, decides snapshot-replace vs continue, and never lets stale
//! mirror state override phone truth.

use crate::model::StateVersion;

/// What the controller must do with its mirror after a resume handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reconciliation {
    /// Mirror is contiguous with phone truth; keep it and apply deltas.
    Continue,
    /// Epoch changed or a gap was detected; discard the mirror wholesale.
    ReplaceSnapshot,
}

/// The phone restarted (new epoch) or advanced past what the mirror last saw, so
/// only a full snapshot can restore truth. A mirror ahead of the phone is also
/// stale, not authoritative — that direction means the phone lost state.
pub fn decide(mirror: Option<&StateVersion>, phone: &StateVersion) -> Reconciliation {
    match mirror {
        None => Reconciliation::ReplaceSnapshot,
        Some(m) if m.epoch_id != phone.epoch_id => Reconciliation::ReplaceSnapshot,
        Some(m) if m.state_seq != phone.state_seq => Reconciliation::ReplaceSnapshot,
        Some(_) => Reconciliation::Continue,
    }
}

/// True when an inbound event may be applied in order on top of the mirror.
pub fn is_contiguous(mirror: &StateVersion, incoming: &StateVersion) -> bool {
    mirror.epoch_id == incoming.epoch_id && incoming.state_seq > mirror.state_seq
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(epoch: &str, seq: u64) -> StateVersion {
        StateVersion {
            epoch_id: epoch.into(),
            state_seq: seq,
        }
    }

    #[test]
    fn empty_mirror_needs_a_snapshot() {
        assert_eq!(
            decide(None, &version("e1", 7)),
            Reconciliation::ReplaceSnapshot
        );
    }

    #[test]
    fn phone_restart_invalidates_the_mirror() {
        assert_eq!(
            decide(Some(&version("e1", 7)), &version("e2", 1)),
            Reconciliation::ReplaceSnapshot
        );
    }

    #[test]
    fn gap_within_an_epoch_needs_a_snapshot() {
        assert_eq!(
            decide(Some(&version("e1", 4)), &version("e1", 9)),
            Reconciliation::ReplaceSnapshot
        );
    }

    #[test]
    fn mirror_ahead_of_the_phone_is_stale_not_authoritative() {
        assert_eq!(
            decide(Some(&version("e1", 9)), &version("e1", 4)),
            Reconciliation::ReplaceSnapshot
        );
    }

    #[test]
    fn matching_version_continues() {
        assert_eq!(
            decide(Some(&version("e1", 7)), &version("e1", 7)),
            Reconciliation::Continue
        );
    }

    #[test]
    fn contiguity_requires_same_epoch_and_forward_motion() {
        assert!(is_contiguous(&version("e1", 7), &version("e1", 8)));
        assert!(!is_contiguous(&version("e1", 7), &version("e1", 7)));
        assert!(!is_contiguous(&version("e1", 7), &version("e2", 8)));
    }
}
