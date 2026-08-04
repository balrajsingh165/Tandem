//! Opens and services BTPROTO_SCO sockets for call audio, honoring the
//! negotiated codec (CVSD/mSBC with transparent eSCO), and exchanges frames with
//! tandem_audio ring buffers.

use crate::error::BluetoothError;
use crate::hfp::codec_negotiation::Codec;

/// Air-mode for the SCO socket. mSBC requires transparent data so the codec
/// frames pass through untouched; CVSD is handled by the controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirMode {
    Cvsd,
    Transparent,
}

impl AirMode {
    pub fn for_codec(codec: Codec) -> Self {
        match codec {
            Codec::Cvsd => Self::Cvsd,
            Codec::Msbc => Self::Transparent,
        }
    }
}

/// MTU used for SCO reads/writes; mSBC frames are 60 bytes plus header.
pub const MSBC_FRAME_BYTES: usize = 60;

pub fn open(_address: &str, _codec: Codec) -> Result<(), BluetoothError> {
    Err(BluetoothError::BackendUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_band_requires_transparent_air_mode() {
        assert_eq!(AirMode::for_codec(Codec::Msbc), AirMode::Transparent);
        assert_eq!(AirMode::for_codec(Codec::Cvsd), AirMode::Cvsd);
    }
}
