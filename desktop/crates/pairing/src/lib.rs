//! tandem_pairing: desktop side of first pairing — QR payload parsing, the
//! pairing state machine, and short-code derivation. Produces the persisted
//! PairedPhone identity on success (docs/07).

pub mod error;
pub mod flow;
pub mod qr;
pub mod short_code;

pub use error::PairingError;
pub use qr::QrPayload;
pub use short_code::ShortCode;
