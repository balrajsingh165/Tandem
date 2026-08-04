//! AudioBackend implementation over cpal: device enumeration, stream setup at
//! native rates, and frame exchange with the pipeline through ring buffers. All
//! OS-audio quirks (WASAPI/CoreAudio/ALSA-PipeWire) isolate here. [Tier B]

use crate::backend::{AudioBackend, AudioEvent, StreamFormat};
use crate::error::AudioError;

/// Device selection, empty meaning the OS default communication device.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceSelection {
    pub capture: String,
    pub playback: String,
}

#[derive(Debug, Default)]
pub struct CpalAudioBackend {
    selection: DeviceSelection,
}

impl CpalAudioBackend {
    pub fn new(selection: DeviceSelection) -> Self {
        Self { selection }
    }

    pub fn selection(&self) -> &DeviceSelection {
        &self.selection
    }
}

impl AudioBackend for CpalAudioBackend {
    fn open_capture(&mut self, _format: StreamFormat) -> Result<(), AudioError> {
        Err(AudioError::BackendUnavailable)
    }

    fn open_playback(&mut self, _format: StreamFormat) -> Result<(), AudioError> {
        Err(AudioError::BackendUnavailable)
    }

    fn read_capture(&mut self, _out: &mut [i16]) -> Result<usize, AudioError> {
        Err(AudioError::BackendUnavailable)
    }

    fn write_playback(&mut self, _samples: &[i16]) -> Result<(), AudioError> {
        Err(AudioError::BackendUnavailable)
    }

    fn drain_events(&mut self) -> Vec<AudioEvent> {
        Vec::new()
    }

    fn close(&mut self) {}
}
