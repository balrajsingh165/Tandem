//! Compares the HFP indicator view of call state with the LAN CallSnapshot
//! mirror, flags divergence for logging/telemetry, and always resolves in favor
//! of LAN truth (single-command-path rule, docs/05).

use crate::hfp::indicators::HfpCallView;

/// Coarse LAN-side view, reduced from the authoritative CallSnapshot so this
/// module does not depend on the core domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanCallView {
    Idle,
    Incoming,
    Outgoing,
    Active,
    HeldOnly,
}

/// Outcome of a consistency check. Divergence is reported, never acted on: the
/// LAN mirror is the source of truth and HFP only reflects reality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorCheck {
    Consistent,
    Diverged {
        lan: LanCallView,
        hfp: HfpCallView,
    },
}

impl MirrorCheck {
    pub fn is_diverged(self) -> bool {
        matches!(self, Self::Diverged { .. })
    }
}

/// Compares the two views. A transient mismatch is expected while an indicator
/// update is in flight, so callers debounce before reporting.
pub fn compare(lan: LanCallView, hfp: HfpCallView) -> MirrorCheck {
    let equivalent = matches!(
        (lan, hfp),
        (LanCallView::Idle, HfpCallView::Idle)
            | (LanCallView::Incoming, HfpCallView::Incoming)
            | (LanCallView::Outgoing, HfpCallView::Outgoing)
            | (LanCallView::Active, HfpCallView::Active)
            | (LanCallView::HeldOnly, HfpCallView::HeldOnly)
    );

    if equivalent {
        MirrorCheck::Consistent
    } else {
        MirrorCheck::Diverged { lan, hfp }
    }
}

/// The resolution rule, stated in code so it cannot drift: whatever HFP reports,
/// the LAN view wins.
pub fn resolve(lan: LanCallView, _hfp: HfpCallView) -> LanCallView {
    lan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_views_are_consistent() {
        assert_eq!(
            compare(LanCallView::Active, HfpCallView::Active),
            MirrorCheck::Consistent
        );
        assert_eq!(
            compare(LanCallView::Idle, HfpCallView::Idle),
            MirrorCheck::Consistent
        );
    }

    #[test]
    fn mismatched_views_are_reported_as_divergence() {
        let check = compare(LanCallView::Active, HfpCallView::Idle);
        assert!(check.is_diverged());
        assert_eq!(
            check,
            MirrorCheck::Diverged {
                lan: LanCallView::Active,
                hfp: HfpCallView::Idle
            }
        );
    }

    #[test]
    fn lan_truth_always_wins_regardless_of_hfp() {
        assert_eq!(
            resolve(LanCallView::Active, HfpCallView::Idle),
            LanCallView::Active
        );
        assert_eq!(
            resolve(LanCallView::Idle, HfpCallView::Active),
            LanCallView::Idle
        );
        assert_eq!(
            resolve(LanCallView::HeldOnly, HfpCallView::Outgoing),
            LanCallView::HeldOnly
        );
    }
}
