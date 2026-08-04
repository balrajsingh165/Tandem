//! Reconnect loop: exponential backoff (0.5 s to 30 s, jittered), immediate retry
//! on network-change signals, and ResumeRequest emission with the last seen
//! (epoch_id, state_seq, call_log_version) so core can reconcile (docs/10 flow h).

use std::time::Duration;

/// Backoff parameters from docs/06; jitter keeps a fleet of desktops from
/// retrying in lockstep after a router reboot.
pub const INITIAL_BACKOFF_MS: u64 = 500;
pub const MAX_BACKOFF_MS: u64 = 30_000;
pub const JITTER_FRACTION: f64 = 0.20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backoff {
    current_ms: u64,
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}

impl Backoff {
    pub fn new() -> Self {
        Self {
            current_ms: INITIAL_BACKOFF_MS,
        }
    }

    /// Base delay before jitter. Callers apply `jittered` with their own entropy
    /// source so this crate stays deterministic and testable.
    pub fn current(&self) -> Duration {
        Duration::from_millis(self.current_ms)
    }

    pub fn advance(&mut self) {
        self.current_ms = (self.current_ms.saturating_mul(2)).min(MAX_BACKOFF_MS);
    }

    /// A successful connection or an explicit network-change signal resets the
    /// delay so recovery is immediate rather than waiting out a long backoff.
    pub fn reset(&mut self) {
        self.current_ms = INITIAL_BACKOFF_MS;
    }

    /// Applies +/- JITTER_FRACTION to the base delay. `entropy` is in [0.0, 1.0).
    pub fn jittered(&self, entropy: f64) -> Duration {
        let clamped = entropy.clamp(0.0, 1.0);
        let spread = (clamped * 2.0) - 1.0;
        let factor = 1.0 + (spread * JITTER_FRACTION);
        Duration::from_millis((self.current_ms as f64 * factor).round() as u64)
    }
}

/// What the desktop asks the phone for after reconnecting, so the mirror can be
/// reconciled against the source of truth.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResumeCursor {
    pub last_epoch_id: String,
    pub last_state_seq: u64,
    pub last_call_log_version: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_saturates_at_the_cap() {
        let mut b = Backoff::new();
        assert_eq!(b.current(), Duration::from_millis(500));
        b.advance();
        assert_eq!(b.current(), Duration::from_millis(1000));
        for _ in 0..20 {
            b.advance();
        }
        assert_eq!(b.current(), Duration::from_millis(MAX_BACKOFF_MS));
    }

    #[test]
    fn reset_restores_immediate_recovery() {
        let mut b = Backoff::new();
        for _ in 0..10 {
            b.advance();
        }
        b.reset();
        assert_eq!(b.current(), Duration::from_millis(INITIAL_BACKOFF_MS));
    }

    #[test]
    fn jitter_stays_within_twenty_percent() {
        let mut b = Backoff::new();
        b.advance();
        let base = 1000.0;
        for step in 0..=10 {
            let entropy = step as f64 / 10.0;
            let ms = b.jittered(entropy).as_millis() as f64;
            assert!(ms >= base * 0.8 - 1.0, "{ms} below lower bound");
            assert!(ms <= base * 1.2 + 1.0, "{ms} above upper bound");
        }
    }

    #[test]
    fn jitter_extremes_hit_both_bounds() {
        let b = Backoff::new();
        assert_eq!(b.jittered(0.0), Duration::from_millis(400));
        assert_eq!(b.jittered(1.0), Duration::from_millis(600));
        assert_eq!(b.jittered(0.5), Duration::from_millis(500));
    }
}
