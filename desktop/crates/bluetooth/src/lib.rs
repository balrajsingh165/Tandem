//! tandem_bluetooth: the HFP Hands-Free unit — OS-independent HFP protocol core
//! plus pluggable backends (linux_bluez, usb_dongle, null). Implements the public
//! Bluetooth SIG HFP v1.8 spec; no product's proprietary protocol is involved
//! (docs/05). [Tier B]

pub mod backend;
pub mod backends;
pub mod error;
pub mod hfp;

pub use backend::{BluetoothBackend, ScoState};
pub use error::BluetoothError;

/// Hands-Free Profile service UUID, from the Bluetooth SIG assigned numbers.
pub const HFP_HF_UUID: u16 = 0x111E;

/// Audio Gateway service UUID, published by the phone.
pub const HFP_AG_UUID: u16 = 0x111F;
