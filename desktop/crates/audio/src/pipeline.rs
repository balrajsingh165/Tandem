//! Assembles the duplex audio graph: capture → AEC → resample → SCO uplink, and
//! SCO downlink → resample → playback, with end-to-end latency accounting
//! surfaced to the UI. [Tier B]

use crate::error::AudioError;
use crate::ring_buffer::RingBuffer;
use crate::{NARROW_BAND_HZ, SCO_FRAME_MS, WIDE_BAND_HZ};

/// Latency the UI reports, split so a user can tell a slow device from a slow
/// radio link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LatencyBudget {
    pub capture_ms: u32,
    pub playback_ms: u32,
    pub link_ms: u32,
}

impl LatencyBudget {
    pub fn total_ms(&self) -> u32 {
        self.capture_ms + self.playback_ms + self.link_ms
    }

    /// docs/05 expects roughly 40–80 ms added end to end; beyond that the UI
    /// warns rather than silently degrading the call.
    pub fn is_within_expected(&self) -> bool {
        self.total_ms() <= 80
    }
}

/// Duplex buffers between the OS audio callbacks and the SCO pump.
#[derive(Debug)]
pub struct AudioPipeline {
    uplink: RingBuffer,
    downlink: RingBuffer,
    sample_rate_hz: u32,
    latency: LatencyBudget,
}

impl AudioPipeline {
    /// Sizes each buffer to `buffered_frames` SCO frames at the negotiated rate.
    pub fn new(sample_rate_hz: u32, buffered_frames: usize) -> Result<Self, AudioError> {
        if sample_rate_hz != NARROW_BAND_HZ && sample_rate_hz != WIDE_BAND_HZ {
            return Err(AudioError::FormatUnsupported {
                requested_hz: sample_rate_hz,
            });
        }
        let frame_samples = (sample_rate_hz as usize * SCO_FRAME_MS as usize) / 1000;
        let capacity = frame_samples * buffered_frames.max(1);
        Ok(Self {
            uplink: RingBuffer::with_capacity(capacity),
            downlink: RingBuffer::with_capacity(capacity),
            sample_rate_hz,
            latency: LatencyBudget::default(),
        })
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn uplink(&mut self) -> &mut RingBuffer {
        &mut self.uplink
    }

    pub fn downlink(&mut self) -> &mut RingBuffer {
        &mut self.downlink
    }

    pub fn latency(&self) -> LatencyBudget {
        self.latency
    }

    pub fn set_latency(&mut self, latency: LatencyBudget) {
        self.latency = latency;
    }

    /// Non-zero once either direction has starved or overflowed; the daemon
    /// surfaces this rather than letting quality degrade unexplained.
    pub fn dropped_samples(&self) -> u64 {
        self.uplink.dropped() + self.downlink.dropped()
    }

    /// Called when SCO drops. The buffers are cleared so a later reattach does
    /// not replay stale audio; the cellular call itself is untouched.
    pub fn reset(&mut self) {
        self.uplink.clear();
        self.downlink.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_hfp_codec_rates_are_accepted() {
        assert!(AudioPipeline::new(WIDE_BAND_HZ, 4).is_ok());
        assert!(AudioPipeline::new(NARROW_BAND_HZ, 4).is_ok());
        assert!(AudioPipeline::new(44_100, 4).is_err());
    }

    #[test]
    fn buffers_are_sized_from_the_sco_frame_and_depth() {
        let mut p = AudioPipeline::new(WIDE_BAND_HZ, 4).unwrap();
        assert_eq!(p.uplink().capacity(), 112 * 4);
        assert_eq!(p.downlink().capacity(), 112 * 4);
    }

    #[test]
    fn latency_budget_flags_excursions_beyond_expectations() {
        let good = LatencyBudget {
            capture_ms: 20,
            playback_ms: 20,
            link_ms: 30,
        };
        assert_eq!(good.total_ms(), 70);
        assert!(good.is_within_expected());

        let bad = LatencyBudget {
            capture_ms: 60,
            playback_ms: 40,
            link_ms: 30,
        };
        assert!(!bad.is_within_expected());
    }

    #[test]
    fn reset_clears_buffers_after_an_sco_drop() {
        let mut p = AudioPipeline::new(WIDE_BAND_HZ, 1).unwrap();
        p.uplink().push(&[1, 2, 3]);
        p.reset();
        assert!(p.uplink().is_empty());
        assert!(p.downlink().is_empty());
    }

    #[test]
    fn drop_tally_aggregates_both_directions() {
        let mut p = AudioPipeline::new(NARROW_BAND_HZ, 1).unwrap();
        let capacity = p.uplink().capacity();
        p.uplink().push(&vec![0i16; capacity + 5]);
        p.downlink().push(&vec![0i16; capacity + 3]);
        assert_eq!(p.dropped_samples(), 8);
    }
}
