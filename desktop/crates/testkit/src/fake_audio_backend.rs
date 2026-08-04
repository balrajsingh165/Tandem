//! AudioBackend fake producing synthetic frames and capturing playback for
//! assertion; deterministic clocking for pipeline and latency tests.

use tandem_audio::backend::{AudioBackend, AudioEvent, StreamFormat};
use tandem_audio::error::AudioError;

/// Deterministic backend: capture yields a repeating ramp so tests can assert on
/// exact sample values, and playback is recorded rather than rendered.
#[derive(Debug, Default)]
pub struct FakeAudioBackend {
    format: Option<StreamFormat>,
    capture_open: bool,
    playback_open: bool,
    next_sample: i16,
    pub played: Vec<i16>,
    events: Vec<AudioEvent>,
}

impl FakeAudioBackend {
    pub fn format(&self) -> Option<StreamFormat> {
        self.format
    }

    pub fn push_event(&mut self, event: AudioEvent) {
        self.events.push(event);
    }
}

impl AudioBackend for FakeAudioBackend {
    fn open_capture(&mut self, format: StreamFormat) -> Result<(), AudioError> {
        self.format = Some(format);
        self.capture_open = true;
        Ok(())
    }

    fn open_playback(&mut self, format: StreamFormat) -> Result<(), AudioError> {
        self.format = Some(format);
        self.playback_open = true;
        Ok(())
    }

    fn read_capture(&mut self, out: &mut [i16]) -> Result<usize, AudioError> {
        if !self.capture_open {
            return Err(AudioError::NoDevice {
                direction: "capture",
            });
        }
        for slot in out.iter_mut() {
            *slot = self.next_sample;
            self.next_sample = self.next_sample.wrapping_add(1);
        }
        Ok(out.len())
    }

    fn write_playback(&mut self, samples: &[i16]) -> Result<(), AudioError> {
        if !self.playback_open {
            return Err(AudioError::NoDevice {
                direction: "playback",
            });
        }
        self.played.extend_from_slice(samples);
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<AudioEvent> {
        std::mem::take(&mut self.events)
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
    fn capture_is_deterministic_across_reads() {
        let mut b = FakeAudioBackend::default();
        b.open_capture(StreamFormat::for_rate(16_000)).unwrap();

        let mut first = [0i16; 4];
        b.read_capture(&mut first).unwrap();
        assert_eq!(first, [0, 1, 2, 3]);

        let mut second = [0i16; 3];
        b.read_capture(&mut second).unwrap();
        assert_eq!(second, [4, 5, 6]);
    }

    #[test]
    fn playback_is_recorded_for_assertion() {
        let mut b = FakeAudioBackend::default();
        b.open_playback(StreamFormat::for_rate(8_000)).unwrap();
        b.write_playback(&[9, 8, 7]).unwrap();
        assert_eq!(b.played, vec![9, 8, 7]);
    }

    #[test]
    fn reading_or_writing_a_closed_stream_fails() {
        let mut b = FakeAudioBackend::default();
        let mut out = [0i16; 2];
        assert!(b.read_capture(&mut out).is_err());
        assert!(b.write_playback(&[1]).is_err());
    }

    #[test]
    fn events_are_drained_once() {
        let mut b = FakeAudioBackend::default();
        b.push_event(AudioEvent::DefaultDeviceChanged);
        assert_eq!(b.drain_events().len(), 1);
        assert!(b.drain_events().is_empty());
    }
}
