//! AudioBackend trait (docs/11): open capture/playback streams at a negotiated
//! sample rate, push/pull frames with bounded latency, report device changes.
//! Implementations: cpal (real), null (Tier B-lite / tests).

use crate::error::AudioError;

/// Negotiated stream shape. HFP voice is always mono at the codec's rate; the
/// backend resamples if the device cannot run natively at that rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFormat {
    pub sample_rate_hz: u32,
    pub frame_samples: usize,
}

impl StreamFormat {
    pub fn for_rate(sample_rate_hz: u32) -> Self {
        let frame_samples = (sample_rate_hz as usize * crate::SCO_FRAME_MS as usize) / 1000;
        Self {
            sample_rate_hz,
            frame_samples,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioEvent {
    DefaultDeviceChanged,
    DeviceLost { name: String },
    Xrun { direction: &'static str },
}

/// The seam every audio implementation sits behind, so the pipeline is testable
/// without hardware.
pub trait AudioBackend: Send + Sync {
    fn open_capture(&mut self, format: StreamFormat) -> Result<(), AudioError>;
    fn open_playback(&mut self, format: StreamFormat) -> Result<(), AudioError>;

    /// Reads captured uplink samples; returns how many were available.
    fn read_capture(&mut self, out: &mut [i16]) -> Result<usize, AudioError>;

    /// Queues downlink samples for playback.
    fn write_playback(&mut self, samples: &[i16]) -> Result<(), AudioError>;

    fn drain_events(&mut self) -> Vec<AudioEvent>;
    fn close(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_size_follows_the_sco_cadence() {
        assert_eq!(StreamFormat::for_rate(8_000).frame_samples, 56);
        assert_eq!(StreamFormat::for_rate(16_000).frame_samples, 112);
    }
}
