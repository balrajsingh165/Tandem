//! BluetoothBackend driving a dedicated USB Bluetooth controller directly
//! (bypassing the OS stack, which does not expose the HF role to apps): full host
//! stack from HCI up. Scoped to one vetted controller family at a time (docs/05).
//! [Tier B — Win/macOS USB dongle]

pub mod hci;
pub mod l2cap;
pub mod rfcomm;
pub mod sco_route;
pub mod sdp;
pub mod security;
pub mod usb_transport;

use crate::backend::{AdapterInfo, BackendEvent, BluetoothBackend, ScoState};
use crate::error::BluetoothError;
use crate::hfp::codec_negotiation::Codec;

#[derive(Debug, Default)]
pub struct UsbDongleBackend {
    sco: ScoState,
    events: Vec<BackendEvent>,
}

impl BluetoothBackend for UsbDongleBackend {
    fn adapter(&self) -> Result<AdapterInfo, BluetoothError> {
        Err(BluetoothError::BackendUnavailable)
    }

    fn is_bonded(&self, _address: &str) -> Result<bool, BluetoothError> {
        Err(BluetoothError::BackendUnavailable)
    }

    fn connect_rfcomm(&mut self, _address: &str) -> Result<(), BluetoothError> {
        Err(BluetoothError::BackendUnavailable)
    }

    fn write_at(&mut self, _line: &str) -> Result<(), BluetoothError> {
        Err(BluetoothError::BackendUnavailable)
    }

    fn read_at_line(&mut self) -> Result<Option<String>, BluetoothError> {
        Err(BluetoothError::BackendUnavailable)
    }

    fn open_sco(&mut self, _codec: Codec) -> Result<(), BluetoothError> {
        Err(BluetoothError::BackendUnavailable)
    }

    fn close_sco(&mut self) {
        self.sco = ScoState::Closed;
    }

    fn sco_state(&self) -> ScoState {
        self.sco
    }

    fn read_sco(&mut self, _out: &mut [i16]) -> Result<usize, BluetoothError> {
        Err(BluetoothError::BackendUnavailable)
    }

    fn write_sco(&mut self, _samples: &[i16]) -> Result<(), BluetoothError> {
        Err(BluetoothError::BackendUnavailable)
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut self.events)
    }

    fn disconnect(&mut self) {
        self.close_sco();
    }
}
