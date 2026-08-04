//! SDP: publishes the Hands-Free service record (UUID 0x111E, RFCOMM channel)
//! and queries the AG's record for its channel number during connection setup.

use crate::error::BluetoothError;

/// Attribute identifiers from the SDP specification used by the HFP records.
pub const ATTR_SERVICE_CLASS_ID_LIST: u16 = 0x0001;
pub const ATTR_PROTOCOL_DESCRIPTOR_LIST: u16 = 0x0004;
pub const ATTR_SUPPORTED_FEATURES: u16 = 0x0311;

/// The hands-free record Tandem publishes so the AG can find it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandsFreeRecord {
    pub service_uuid: u16,
    pub rfcomm_channel: u8,
    pub profile_version: u16,
    pub supported_features: u32,
}

impl Default for HandsFreeRecord {
    fn default() -> Self {
        Self {
            service_uuid: crate::HFP_HF_UUID,
            rfcomm_channel: 1,
            profile_version: 0x0108,
            supported_features: crate::hfp::HF_FEATURES,
        }
    }
}

/// What Tandem needs from the AG's published record to open the SLC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioGatewayRecord {
    pub rfcomm_channel: u8,
    pub profile_version: u16,
}

/// Extracts the AG's RFCOMM server channel from a queried record.
pub fn parse_audio_gateway_record(
    rfcomm_channel: Option<u8>,
    profile_version: Option<u16>,
) -> Result<AudioGatewayRecord, BluetoothError> {
    let channel = rfcomm_channel.ok_or_else(|| {
        BluetoothError::Rfcomm("audio gateway record has no RFCOMM channel".into())
    })?;
    if channel == 0 || channel > 30 {
        return Err(BluetoothError::Rfcomm(format!(
            "audio gateway advertised invalid RFCOMM channel {channel}"
        )));
    }
    Ok(AudioGatewayRecord {
        rfcomm_channel: channel,
        profile_version: profile_version.unwrap_or(0x0105),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_record_declares_the_hands_free_uuid() {
        let record = HandsFreeRecord::default();
        assert_eq!(record.service_uuid, 0x111E);
        assert_eq!(record.profile_version, 0x0108);
    }

    #[test]
    fn parses_a_usable_gateway_record() {
        let ag = parse_audio_gateway_record(Some(3), Some(0x0108)).unwrap();
        assert_eq!(ag.rfcomm_channel, 3);
        assert_eq!(ag.profile_version, 0x0108);
    }

    #[test]
    fn defaults_the_version_when_the_gateway_omits_it() {
        assert_eq!(
            parse_audio_gateway_record(Some(1), None).unwrap().profile_version,
            0x0105
        );
    }

    #[test]
    fn missing_or_invalid_channels_are_rejected() {
        assert!(parse_audio_gateway_record(None, Some(0x0108)).is_err());
        assert!(parse_audio_gateway_record(Some(0), None).is_err());
        assert!(parse_audio_gateway_record(Some(99), None).is_err());
    }
}
