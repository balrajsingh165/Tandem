//! BluetoothBackend over BlueZ: adapter and bonding via org.bluez D-Bus, HFP HF
//! profile registration via Profile1, SCO via kernel sockets. Requires disabling
//! PipeWire's native HFP backend to avoid double-claiming the profile (docs/13).
//! [Tier B — Linux]

pub mod profile;
pub mod sco;

use crate::backend::{AdapterInfo, BackendEvent, BluetoothBackend, ScoState};
use crate::error::BluetoothError;
use crate::hfp::codec_negotiation::Codec;

/// D-Bus well-known name and object paths this backend talks to.
pub const BLUEZ_SERVICE: &str = "org.bluez";
pub const PROFILE_MANAGER_PATH: &str = "/org/bluez";

#[derive(Debug, Default)]
pub struct BluezBackend {
    sco: ScoState,
    events: Vec<BackendEvent>,
}

impl BluetoothBackend for BluezBackend {
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
