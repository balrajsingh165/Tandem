//! SPKI-SHA256 fingerprint computation, base64url rendering, and constant-time
//! pin comparison used by transport TLS verification on both the pairing and
//! paired paths.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};

use crate::error::CryptoError;

/// A pinned peer key. Trust is this value, never a certificate chain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpkiFingerprint([u8; 32]);

impl SpkiFingerprint {
    /// Hashes the DER-encoded SubjectPublicKeyInfo, which is stable across
    /// certificate reissues of the same key.
    pub fn from_spki_der(spki_der: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(spki_der);
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        Self(out)
    }

    pub fn from_base64url(encoded: &str) -> Result<Self, CryptoError> {
        let raw = URL_SAFE_NO_PAD
            .decode(encoded.trim())
            .map_err(|_| CryptoError::MalformedFingerprint)?;
        let bytes: [u8; 32] = raw
            .try_into()
            .map_err(|_| CryptoError::MalformedFingerprint)?;
        Ok(Self(bytes))
    }

    pub fn to_base64url(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Constant-time comparison so a mismatching pin cannot be probed by timing.
    pub fn matches(&self, other: &Self) -> bool {
        let mut diff = 0u8;
        for (a, b) in self.0.iter().zip(other.0.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }

    pub fn verify(&self, presented: &Self) -> Result<(), CryptoError> {
        if self.matches(presented) {
            Ok(())
        } else {
            Err(CryptoError::PinMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_round_trips_through_base64url() {
        let fp = SpkiFingerprint::from_spki_der(b"example-spki-der");
        let encoded = fp.to_base64url();
        assert_eq!(SpkiFingerprint::from_base64url(&encoded).unwrap(), fp);
    }

    #[test]
    fn base64url_encoding_is_unpadded_and_url_safe() {
        let encoded = SpkiFingerprint::from_spki_der(b"x").to_base64url();
        assert!(!encoded.contains('='));
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
    }

    #[test]
    fn distinct_keys_do_not_match() {
        let a = SpkiFingerprint::from_spki_der(b"key-a");
        let b = SpkiFingerprint::from_spki_der(b"key-b");
        assert!(!a.matches(&b));
        assert_eq!(a.verify(&b), Err(CryptoError::PinMismatch));
    }

    #[test]
    fn identical_keys_match() {
        let a = SpkiFingerprint::from_spki_der(b"key-a");
        let b = SpkiFingerprint::from_spki_der(b"key-a");
        assert!(a.matches(&b));
        assert!(a.verify(&b).is_ok());
    }

    #[test]
    fn malformed_fingerprints_are_rejected() {
        assert_eq!(
            SpkiFingerprint::from_base64url("not-base64!!"),
            Err(CryptoError::MalformedFingerprint)
        );
        assert_eq!(
            SpkiFingerprint::from_base64url("c2hvcnQ"),
            Err(CryptoError::MalformedFingerprint)
        );
    }
}
