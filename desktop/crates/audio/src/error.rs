//! AudioError: device-unavailable, format-negotiation, stream, and xrun
//! failures; states which are recoverable by pipeline rebuild vs fatal to the
//! audio session.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AudioError {
    #[error("no audio device named {0}")]
    DeviceNotFound(String),

    #[error("no usable {direction} device is available")]
    NoDevice { direction: &'static str },

    #[error("device cannot provide {requested_hz} Hz mono")]
    FormatUnsupported { requested_hz: u32 },

    #[error("audio stream failed: {0}")]
    Stream(String),

    #[error("buffer under/overrun on the {direction} path")]
    Xrun { direction: &'static str },

    #[error("audio backend is not available in this build")]
    BackendUnavailable,
}

impl AudioError {
    /// Recoverable failures justify rebuilding the pipeline; the rest end the
    /// audio session. Either way the cellular call is never touched — audio loss
    /// falls back to the handset (docs/05).
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::Xrun { .. } | Self::Stream(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xruns_and_stream_faults_justify_a_rebuild() {
        assert!(AudioError::Xrun {
            direction: "capture"
        }
        .is_recoverable());
        assert!(AudioError::Stream("device reset".into()).is_recoverable());
    }

    #[test]
    fn missing_devices_and_formats_are_not_recoverable_by_retry() {
        assert!(!AudioError::NoDevice {
            direction: "playback"
        }
        .is_recoverable());
        assert!(!AudioError::FormatUnsupported {
            requested_hz: 16_000
        }
        .is_recoverable());
        assert!(!AudioError::BackendUnavailable.is_recoverable());
    }
}
