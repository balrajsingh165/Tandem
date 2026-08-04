//! Routes SCO audio over the controller's USB isochronous endpoints (HCI SCO
//! packets), pacing against the Bluetooth clock and bridging frames into
//! tandem_audio ring buffers.

use crate::error::BluetoothError;
use crate::hfp::codec_negotiation::Codec;

/// HCI Voice Setting air-coding values. Transparent data is required for mSBC so
/// the controller does not transcode wide-band frames.
pub const VOICE_SETTING_CVSD: u16 = 0x0060;
pub const VOICE_SETTING_TRANSPARENT: u16 = 0x0063;

/// eSCO retransmission effort: favour quality for voice without unbounded delay.
pub const RETRANSMISSION_EFFORT_QUALITY: u8 = 0x02;

pub fn voice_setting_for(codec: Codec) -> u16 {
    match codec {
        Codec::Cvsd => VOICE_SETTING_CVSD,
        Codec::Msbc => VOICE_SETTING_TRANSPARENT,
    }
}

/// Bytes per SCO packet for a codec, used to size isochronous transfers.
pub fn packet_bytes_for(codec: Codec) -> usize {
    match codec {
        Codec::Cvsd => 48,
        Codec::Msbc => 60,
    }
}

pub fn open(_codec: Codec) -> Result<(), BluetoothError> {
    Err(BluetoothError::BackendUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_band_requires_transparent_voice_setting() {
        assert_eq!(voice_setting_for(Codec::Msbc), VOICE_SETTING_TRANSPARENT);
        assert_eq!(voice_setting_for(Codec::Cvsd), VOICE_SETTING_CVSD);
    }

    #[test]
    fn packet_sizes_differ_per_codec() {
        assert_eq!(packet_bytes_for(Codec::Msbc), 60);
        assert_eq!(packet_bytes_for(Codec::Cvsd), 48);
    }
}
