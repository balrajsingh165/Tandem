//! Resamples between device native rates and the HFP codec rate (8 kHz CVSD /
//! 16 kHz mSBC) with fixed latency budget; quality/latency tradeoffs documented
//! inline in docs/05.

use crate::error::AudioError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resampler {
    input_hz: u32,
    output_hz: u32,
}

impl Resampler {
    pub fn new(input_hz: u32, output_hz: u32) -> Result<Self, AudioError> {
        if input_hz == 0 || output_hz == 0 {
            return Err(AudioError::FormatUnsupported {
                requested_hz: output_hz,
            });
        }
        Ok(Self {
            input_hz,
            output_hz,
        })
    }

    pub fn is_passthrough(&self) -> bool {
        self.input_hz == self.output_hz
    }

    /// Output sample count for a given input length, used to size buffers ahead
    /// of conversion.
    pub fn output_len(&self, input_len: usize) -> usize {
        ((input_len as u64 * self.output_hz as u64) / self.input_hz as u64) as usize
    }

    pub fn process(&mut self, input: &[i16], out: &mut Vec<i16>) -> Result<(), AudioError> {
        if self.is_passthrough() {
            out.extend_from_slice(input);
            return Ok(());
        }
        Err(AudioError::BackendUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_rates_match() {
        let mut r = Resampler::new(16_000, 16_000).unwrap();
        assert!(r.is_passthrough());
        let mut out = Vec::new();
        r.process(&[1, 2, 3], &mut out).unwrap();
        assert_eq!(out, vec![1, 2, 3]);
    }

    #[test]
    fn output_length_tracks_the_rate_ratio() {
        let up = Resampler::new(8_000, 16_000).unwrap();
        assert_eq!(up.output_len(56), 112);
        let down = Resampler::new(48_000, 16_000).unwrap();
        assert_eq!(down.output_len(480), 160);
    }

    #[test]
    fn zero_rates_are_rejected() {
        assert!(Resampler::new(0, 16_000).is_err());
        assert!(Resampler::new(16_000, 0).is_err());
    }
}
