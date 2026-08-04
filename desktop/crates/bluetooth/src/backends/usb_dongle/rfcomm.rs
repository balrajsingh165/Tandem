//! RFCOMM (TS 07.10 subset) over L2CAP: multiplexer session, DLCI management,
//! credit-based flow control — enough to carry the HFP SLC byte stream.

use crate::error::BluetoothError;

/// DLCI 0 is the multiplexer control channel; data channels derive from the
/// server channel number the AG publishes in its SDP record.
pub const DLCI_CONTROL: u8 = 0;

/// Initial credits granted to the peer when a data channel opens.
pub const INITIAL_CREDITS: u8 = 7;

/// Computes the DLCI for a server channel. The direction bit distinguishes the
/// initiator, which for Tandem is always the hands-free side.
pub fn dlci_for_server_channel(server_channel: u8, initiator: bool) -> Result<u8, BluetoothError> {
    if server_channel == 0 || server_channel > 30 {
        return Err(BluetoothError::Rfcomm(format!(
            "server channel {server_channel} outside the valid range 1..=30"
        )));
    }
    Ok((server_channel << 1) | u8::from(!initiator))
}

/// Credit-based flow control state for one data channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditWindow {
    remaining: u8,
}

impl Default for CreditWindow {
    fn default() -> Self {
        Self {
            remaining: INITIAL_CREDITS,
        }
    }
}

impl CreditWindow {
    pub fn remaining(&self) -> u8 {
        self.remaining
    }

    pub fn can_send(&self) -> bool {
        self.remaining > 0
    }

    /// Consumes one credit for an outbound frame; sending without credit is a
    /// protocol violation, not a queueing decision.
    pub fn consume(&mut self) -> Result<(), BluetoothError> {
        if self.remaining == 0 {
            return Err(BluetoothError::Rfcomm("no RFCOMM credits remaining".into()));
        }
        self.remaining -= 1;
        Ok(())
    }

    pub fn grant(&mut self, credits: u8) {
        self.remaining = self.remaining.saturating_add(credits);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dlci_encodes_channel_and_direction() {
        assert_eq!(dlci_for_server_channel(1, true).unwrap(), 2);
        assert_eq!(dlci_for_server_channel(1, false).unwrap(), 3);
        assert_ne!(DLCI_CONTROL, dlci_for_server_channel(1, true).unwrap());
    }

    #[test]
    fn invalid_server_channels_are_rejected() {
        assert!(dlci_for_server_channel(0, true).is_err());
        assert!(dlci_for_server_channel(31, true).is_err());
    }

    #[test]
    fn credits_gate_sending_and_can_be_replenished() {
        let mut w = CreditWindow::default();
        for _ in 0..INITIAL_CREDITS {
            assert!(w.can_send());
            w.consume().unwrap();
        }
        assert!(!w.can_send());
        assert!(w.consume().is_err());
        w.grant(3);
        assert_eq!(w.remaining(), 3);
        assert!(w.can_send());
    }
}
