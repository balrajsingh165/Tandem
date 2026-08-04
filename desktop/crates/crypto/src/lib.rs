//! tandem_crypto: desktop device identity (P-256), self-signed certificate
//! management, SPKI pinning helpers, and OS-keychain-backed secret storage.
//! Trust is pinned keys, never chains (ADR-0006).

pub mod cert;
pub mod error;
pub mod identity;
pub mod pinning;
pub mod secrets;

pub use error::CryptoError;
pub use identity::DeviceIdentity;
pub use pinning::SpkiFingerprint;
