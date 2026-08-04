//! Null AudioBackend: accepts and discards frames, produces silence. Serves
//! Tier B-lite fallback builds and deterministic tests.
//! [Tier B-lite fallback]

use crate::backend::{AudioBackend, AudioEvent, StreamFormat};
use crate::error::AudioError;

/// Lets the daemon keep its full composition shape where no desktop audio path
/// exists, so control and history run unchanged.
#[derive(Debug, Default)]
pub struct NullAudioBackend {
    capture_open: bool,
    playback_open: bool,
}

impl AudioBackend for NullAudioBackend {
    fn open_capture(&mut self, _format: StreamFormat) -> Result<(), AudioError> {
        self.capture_open = true;
        Ok(())
    }

    fn open_playback(&mut self, _format: StreamFormat) -> Result<(), AudioError> {
        self.playback_open = true;
        Ok(())
    }

    fn read_capture(&mut self, out: &mut [i16]) -> Result<usize, AudioError> {
        out.fill(0);
        Ok(out.len())
    }

    fn write_playback(&mut self, _samples: &[i16]) -> Result<(), AudioError> {
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<AudioEvent> {
        Vec::new()
    }

    fn close(&mut self) {
        self.capture_open = false;
        self.playback_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_silence_and_swallows_playback() {
        let mut backend = NullAudioBackend::default();
        backend
            .open_capture(StreamFormat::for_rate(16_000))
            .unwrap();
        backend
            .open_playback(StreamFormat::for_rate(16_000))
            .unwrap();

        let mut out = [7i16; 16];
        assert_eq!(backend.read_capture(&mut out).unwrap(), 16);
        assert!(out.iter().all(|&s| s == 0));
        assert!(backend.write_playback(&[1, 2, 3]).is_ok());
        assert!(backend.drain_events().is_empty());
        backend.close();
    }
}
