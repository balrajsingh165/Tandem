//! Null BluetoothBackend: reports no adapter and rejects audio-route attach
//! cleanly, letting the product run control-plane-only while the user pairs
//! commodity earbuds directly to the phone. [Tier B-lite fallback]

use crate::backend::{AdapterInfo, BackendEvent, BluetoothBackend, ScoState};
use crate::error::BluetoothError;
use crate::hfp::codec_negotiation::Codec;

/// Refuses cleanly rather than pretending: the daemon keeps its full composition
/// shape and the UI can explain that audio stays on the phone.
#[derive(Debug, Default)]
pub struct NullBluetoothBackend;

impl BluetoothBackend for NullBluetoothBackend {
    fn adapter(&self) -> Result<AdapterInfo, BluetoothError> {
        Err(BluetoothError::NoAdapter)
    }

    fn is_bonded(&self, _address: &str) -> Result<bool, BluetoothError> {
        Ok(false)
    }

    fn connect_rfcomm(&mut self, _address: &str) -> Result<(), BluetoothError> {
        Err(BluetoothError::NoAdapter)
    }

    fn write_at(&mut self, _line: &str) -> Result<(), BluetoothError> {
        Err(BluetoothError::NoAdapter)
    }

    fn read_at_line(&mut self) -> Result<Option<String>, BluetoothError> {
        Ok(None)
    }

    fn open_sco(&mut self, _codec: Codec) -> Result<(), BluetoothError> {
        Err(BluetoothError::NoAdapter)
    }

    fn close_sco(&mut self) {}

    fn sco_state(&self) -> ScoState {
        ScoState::Closed
    }

    fn read_sco(&mut self, _out: &mut [i16]) -> Result<usize, BluetoothError> {
        Err(BluetoothError::NoAdapter)
    }

    fn write_sco(&mut self, _samples: &[i16]) -> Result<(), BluetoothError> {
        Err(BluetoothError::NoAdapter)
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        Vec::new()
    }

    fn disconnect(&mut self) {}

    fn supports_audio(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_itself_audio_incapable() {
        assert!(!NullBluetoothBackend.supports_audio());
        assert_eq!(NullBluetoothBackend.sco_state(), ScoState::Closed);
    }

    #[test]
    fn refuses_attachment_cleanly_rather_than_pretending() {
        let mut b = NullBluetoothBackend;
        assert_eq!(b.adapter().unwrap_err(), BluetoothError::NoAdapter);
        assert_eq!(
            b.connect_rfcomm("AA:BB:CC:DD:EE:FF").unwrap_err(),
            BluetoothError::NoAdapter
        );
        assert_eq!(
            b.open_sco(Codec::Cvsd).unwrap_err(),
            BluetoothError::NoAdapter
        );
        assert!(!b.is_bonded("AA:BB:CC:DD:EE:FF").unwrap());
    }

    /// Refusal must not be mistaken for a link failure, which would make the UI
    /// suggest a degradation that never happened.
    #[test]
    fn its_refusal_is_not_a_degradation_path() {
        assert!(!BluetoothError::NoAdapter.degrades_to_handset());
    }
}
