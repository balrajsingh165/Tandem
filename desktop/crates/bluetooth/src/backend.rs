//! BluetoothBackend trait (docs/11): adapter lifecycle, bonding state, RFCOMM
//! channel to the AG, SCO audio open/close, and backend events. The seam that
//! makes Tier B Linux, Tier B dongle, Tier B-lite, and a future Tier C backend
//! interchangeable (ADR-0010).

use crate::error::BluetoothError;
use crate::hfp::codec_negotiation::Codec;

/// State of the synchronous audio link that carries voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScoState {
    #[default]
    Closed,
    Opening,
    Open {
        codec: Codec,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterInfo {
    pub address: String,
    pub name: String,
    pub powered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    AdapterChanged(AdapterInfo),
    BondingChanged {
        address: String,
        bonded: bool,
    },
    RfcommConnected {
        address: String,
    },
    RfcommDisconnected {
        address: String,
    },
    ScoStateChanged(ScoState),
    /// The link dropped mid-call. The daemon reports this and lets the phone fall
    /// back to its earpiece; it never ends the cellular call (docs/05).
    LinkLost {
        reason: String,
    },
}

/// Every OS-specific implementation sits behind this trait, so the HFP core and
/// the daemon are written once.
pub trait BluetoothBackend: Send + Sync {
    fn adapter(&self) -> Result<AdapterInfo, BluetoothError>;
    fn is_bonded(&self, address: &str) -> Result<bool, BluetoothError>;

    /// Opens the RFCOMM channel carrying the SLC's AT stream.
    fn connect_rfcomm(&mut self, address: &str) -> Result<(), BluetoothError>;
    fn write_at(&mut self, line: &str) -> Result<(), BluetoothError>;
    fn read_at_line(&mut self) -> Result<Option<String>, BluetoothError>;

    /// Opens the SCO voice link for the negotiated codec.
    fn open_sco(&mut self, codec: Codec) -> Result<(), BluetoothError>;
    fn close_sco(&mut self);
    fn sco_state(&self) -> ScoState;

    fn read_sco(&mut self, out: &mut [i16]) -> Result<usize, BluetoothError>;
    fn write_sco(&mut self, samples: &[i16]) -> Result<(), BluetoothError>;

    fn drain_events(&mut self) -> Vec<BackendEvent>;
    fn disconnect(&mut self);

    /// Whether this backend can carry audio at all. False for the null backend,
    /// which lets the product run control-only (Tier B-lite).
    fn supports_audio(&self) -> bool {
        true
    }
}
