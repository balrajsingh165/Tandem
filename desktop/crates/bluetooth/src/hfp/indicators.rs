//! Tracks AG indicators (call, callsetup, callheld, service, signal, battchg)
//! from +CIEV and periodic +CLCC polls, producing the HFP-view call state used
//! for consistency checks against LAN truth.

use std::collections::HashMap;

/// Indicator names defined by HFP v1.8. The AG chooses the ordering, so names —
/// not positions — are the stable identifiers.
pub const CALL: &str = "call";
pub const CALLSETUP: &str = "callsetup";
pub const CALLHELD: &str = "callheld";
pub const SERVICE: &str = "service";
pub const SIGNAL: &str = "signal";
pub const BATTCHG: &str = "battchg";

/// Coarse call state as the HFP link sees it. This is a consistency check
/// against LAN truth, never a source of truth itself (docs/05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HfpCallView {
    Idle,
    Incoming,
    Outgoing,
    Active,
    HeldOnly,
}

/// Indicator values keyed by the AG's declared ordering.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Indicators {
    order: Vec<String>,
    values: HashMap<String, u8>,
}

impl Indicators {
    /// Records the ordering from `+CIND: (...)`; subsequent `+CIEV` indices are
    /// resolved through it.
    pub fn set_order(&mut self, names: Vec<String>) {
        self.order = names;
    }

    pub fn order(&self) -> &[String] {
        &self.order
    }

    /// Applies `+CIND?` values positionally against the declared ordering.
    pub fn set_values(&mut self, values: &[u8]) {
        for (position, value) in values.iter().enumerate() {
            if let Some(name) = self.order.get(position) {
                self.values.insert(name.clone(), *value);
            }
        }
    }

    /// Applies a `+CIEV: <index>,<value>` update. Indices are 1-based; an index
    /// outside the declared ordering is ignored rather than treated as fatal,
    /// since AG quirks must not tear down a working link.
    pub fn apply_ciev(&mut self, index: usize, value: u8) -> bool {
        match self.order.get(index.wrapping_sub(1)) {
            Some(name) if index > 0 => {
                self.values.insert(name.clone(), value);
                true
            }
            _ => false,
        }
    }

    pub fn get(&self, name: &str) -> Option<u8> {
        self.values.get(name).copied()
    }

    pub fn has_service(&self) -> bool {
        self.get(SERVICE).unwrap_or(0) > 0
    }

    /// Derives the coarse view from the call/callsetup/callheld triple.
    pub fn call_view(&self) -> HfpCallView {
        let call = self.get(CALL).unwrap_or(0);
        let callsetup = self.get(CALLSETUP).unwrap_or(0);
        let callheld = self.get(CALLHELD).unwrap_or(0);

        if call == 0 && callsetup == 0 {
            return if callheld > 0 {
                HfpCallView::HeldOnly
            } else {
                HfpCallView::Idle
            };
        }
        if call == 1 {
            return if callheld == 2 {
                HfpCallView::HeldOnly
            } else {
                HfpCallView::Active
            };
        }
        match callsetup {
            1 => HfpCallView::Incoming,
            2 | 3 => HfpCallView::Outgoing,
            _ => HfpCallView::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indicators() -> Indicators {
        let mut i = Indicators::default();
        i.set_order(vec![
            SERVICE.into(),
            CALL.into(),
            CALLSETUP.into(),
            CALLHELD.into(),
        ]);
        i.set_values(&[1, 0, 0, 0]);
        i
    }

    #[test]
    fn values_are_resolved_through_the_declared_ordering() {
        let i = indicators();
        assert_eq!(i.get(SERVICE), Some(1));
        assert_eq!(i.get(CALL), Some(0));
        assert!(i.has_service());
    }

    #[test]
    fn ciev_updates_by_one_based_index() {
        let mut i = indicators();
        assert!(i.apply_ciev(2, 1));
        assert_eq!(i.get(CALL), Some(1));
    }

    #[test]
    fn out_of_range_indices_are_ignored_not_fatal() {
        let mut i = indicators();
        assert!(!i.apply_ciev(0, 1));
        assert!(!i.apply_ciev(99, 1));
        assert_eq!(i.get(CALL), Some(0));
    }

    #[test]
    fn derives_incoming_and_outgoing_from_callsetup() {
        let mut i = indicators();
        i.apply_ciev(3, 1);
        assert_eq!(i.call_view(), HfpCallView::Incoming);
        i.apply_ciev(3, 2);
        assert_eq!(i.call_view(), HfpCallView::Outgoing);
        i.apply_ciev(3, 3);
        assert_eq!(i.call_view(), HfpCallView::Outgoing);
    }

    #[test]
    fn derives_active_and_held() {
        let mut i = indicators();
        i.apply_ciev(2, 1);
        i.apply_ciev(3, 0);
        assert_eq!(i.call_view(), HfpCallView::Active);

        i.apply_ciev(4, 2);
        assert_eq!(i.call_view(), HfpCallView::HeldOnly);
    }

    #[test]
    fn idle_when_nothing_is_in_progress() {
        assert_eq!(indicators().call_view(), HfpCallView::Idle);
    }
}
