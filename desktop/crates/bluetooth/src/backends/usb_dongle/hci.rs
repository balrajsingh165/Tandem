//! Minimal HCI host: command/event flow, ACL and SCO data paths, controller
//! init, inquiry/paging, and connection management — only the subset HFP-HF
//! requires.

use crate::error::BluetoothError;

/// HCI packet type indicators prefixed to every USB transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Command,
    Acl,
    Sco,
    Event,
}

impl PacketType {
    pub fn indicator(self) -> u8 {
        match self {
            Self::Command => 0x01,
            Self::Acl => 0x02,
            Self::Sco => 0x03,
            Self::Event => 0x04,
        }
    }

    pub fn from_indicator(value: u8) -> Result<Self, BluetoothError> {
        match value {
            0x01 => Ok(Self::Command),
            0x02 => Ok(Self::Acl),
            0x03 => Ok(Self::Sco),
            0x04 => Ok(Self::Event),
            other => Err(BluetoothError::Rfcomm(format!(
                "unknown HCI packet indicator {other:#04x}"
            ))),
        }
    }
}

/// Opcodes the host stack issues, composed from OGF and OCF per the Core spec.
pub const OPCODE_RESET: u16 = 0x0C03;
pub const OPCODE_SETUP_SYNC_CONNECTION: u16 = 0x0428;
pub const OPCODE_ACCEPT_SYNC_CONNECTION: u16 = 0x0429;
pub const OPCODE_WRITE_VOICE_SETTING: u16 = 0x0C26;

/// Composes an opcode from its OGF and OCF fields.
pub const fn opcode(ogf: u16, ocf: u16) -> u16 {
    (ogf << 10) | ocf
}

pub fn reset() -> Result<(), BluetoothError> {
    Err(BluetoothError::BackendUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_indicators_round_trip() {
        for pt in [
            PacketType::Command,
            PacketType::Acl,
            PacketType::Sco,
            PacketType::Event,
        ] {
            assert_eq!(PacketType::from_indicator(pt.indicator()).unwrap(), pt);
        }
    }

    #[test]
    fn unknown_indicators_are_rejected() {
        assert!(PacketType::from_indicator(0x09).is_err());
    }

    #[test]
    fn opcode_composition_matches_the_core_spec() {
        assert_eq!(opcode(0x03, 0x003), OPCODE_RESET);
        assert_eq!(opcode(0x01, 0x028), OPCODE_SETUP_SYNC_CONNECTION);
        assert_eq!(opcode(0x03, 0x026), OPCODE_WRITE_VOICE_SETTING);
    }
}
