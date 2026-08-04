//! tandem_audio: microphone/speaker I/O for the HFP voice path — AudioBackend
//! trait, lock-free ring buffers, resampling, and echo cancellation.
//! Consumes/produces 8 or 16 kHz mono frames against the Bluetooth SCO clock.
//! [Tier B]

pub mod aec;
pub mod backend;
pub mod cpal_backend;
pub mod error;
pub mod null_backend;
pub mod pipeline;
pub mod resampler;
pub mod ring_buffer;

pub use backend::{AudioBackend, StreamFormat};
pub use error::AudioError;
pub use ring_buffer::RingBuffer;

/// Narrow-band CVSD runs at 8 kHz; wide-band mSBC at 16 kHz.
pub const NARROW_BAND_HZ: u32 = 8_000;
pub const WIDE_BAND_HZ: u32 = 16_000;

/// SCO carries 7.5 ms frames; the pipeline is sized in multiples of this.
pub const SCO_FRAME_MS: u32 = 7;
