//! tandem_pairing: desktop side of first pairing — QR payload parsing, the
//! pairing state machine, and short-code derivation. Produces the persisted
//! PairedPhone identity on success (docs/07).

pub mod error;
pub mod flow;
pub mod offer;
pub mod qr;
pub mod short_code;

pub use error::PairingError;
pub use offer::{DesktopOffer, OfferState, PhoneIntroduction};
pub use qr::QrPayload;
pub use short_code::ShortCode;

/// Generates the one-time secret carried in a desktop pairing offer.
pub fn generate_token() -> String {
    use base64::Engine as _;
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("system RNG must be available");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
