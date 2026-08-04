//! Wraps WebRTC AEC3 (webrtc-audio-processing): feeds far-end reference from the
//! playback path and near-end from capture so speakerphone use on the desktop
//! does not echo into the cellular uplink. [Tier B]

use crate::error::AudioError;

/// Echo canceller state. The far-end reference must be fed even when the near-end
/// is silent, or the canceller loses its model of what is being played.
#[derive(Debug, Default)]
pub struct EchoCanceller {
    enabled: bool,
    sample_rate_hz: u32,
}

impl EchoCanceller {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self {
            enabled: true,
            sample_rate_hz,
        }
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Disabling is legitimate when the user is on a headset, where there is no
    /// acoustic path from speaker to microphone.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn push_far_end(&mut self, _playback: &[i16]) -> Result<(), AudioError> {
        if !self.enabled {
            return Ok(());
        }
        Err(AudioError::BackendUnavailable)
    }

    pub fn process_near_end(&mut self, capture: &mut [i16]) -> Result<(), AudioError> {
        if !self.enabled {
            let _ = capture;
            return Ok(());
        }
        Err(AudioError::BackendUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_canceller_passes_audio_through_untouched() {
        let mut aec = EchoCanceller::new(16_000);
        aec.set_enabled(false);
        let mut capture = [5i16, -5, 5];
        assert!(aec.push_far_end(&[1, 2, 3]).is_ok());
        assert!(aec.process_near_end(&mut capture).is_ok());
        assert_eq!(capture, [5, -5, 5]);
    }

    #[test]
    fn is_enabled_by_default_at_the_negotiated_rate() {
        let aec = EchoCanceller::new(8_000);
        assert!(aec.is_enabled());
        assert_eq!(aec.sample_rate_hz(), 8_000);
    }
}
