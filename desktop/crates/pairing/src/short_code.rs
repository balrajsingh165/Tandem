//! Derives the 6-digit short authentication code via HKDF-SHA256 over both SPKI
//! hashes and the TLS exporter, byte-identical to the phone's Fingerprints
//! implementation (docs/07).

use hkdf::Hkdf;
use sha2::Sha256;
use tandem_crypto::SpkiFingerprint;

/// Salt and exporter label are part of the wire contract with the phone; changing
/// either makes both sides derive different codes (docs/07).
const SALT: &[u8] = b"tandem-pairing-short-code-v1";
pub const EXPORTER_LABEL: &[u8] = b"EXPORTER-tandem-pairing-v1";
pub const EXPORTER_LENGTH: usize = 32;

/// Six digits the user compares across both screens on the manual pairing path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortCode(String);

impl ShortCode {
    /// `info` is the phone's SPKI hash followed by the desktop's, so the code is
    /// bound to both identities and to this TLS session's exporter.
    pub fn derive(tls_exporter: &[u8], phone: &SpkiFingerprint, desktop: &SpkiFingerprint) -> Self {
        let mut info = Vec::with_capacity(64);
        info.extend_from_slice(phone.as_bytes());
        info.extend_from_slice(desktop.as_bytes());

        let hk = Hkdf::<Sha256>::new(Some(SALT), tls_exporter);
        let mut okm = [0u8; 4];
        hk.expand(&info, &mut okm)
            .expect("4 bytes is within HKDF-SHA256 output limits");

        let value = u32::from_be_bytes(okm) & 0x7fff_ffff;
        Self(format!("{:06}", value % 1_000_000))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Constant-time comparison; a mismatch means the session is under attack and
    /// pairing must abort.
    pub fn matches(&self, other: &Self) -> bool {
        let (a, b) = (self.0.as_bytes(), other.0.as_bytes());
        if a.len() != b.len() {
            return false;
        }
        let mut diff = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        diff == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone() -> SpkiFingerprint {
        SpkiFingerprint::from_spki_der(b"phone-key")
    }

    fn desktop() -> SpkiFingerprint {
        SpkiFingerprint::from_spki_der(b"desktop-key")
    }

    #[test]
    fn code_is_always_six_digits() {
        for i in 0..64u8 {
            let code = ShortCode::derive(&[i; 32], &phone(), &desktop());
            assert_eq!(code.as_str().len(), 6);
            assert!(code.as_str().chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn derivation_is_deterministic() {
        let a = ShortCode::derive(&[7u8; 32], &phone(), &desktop());
        let b = ShortCode::derive(&[7u8; 32], &phone(), &desktop());
        assert_eq!(a, b);
        assert!(a.matches(&b));
    }

    #[test]
    fn a_different_exporter_yields_a_different_code() {
        let a = ShortCode::derive(&[7u8; 32], &phone(), &desktop());
        let b = ShortCode::derive(&[8u8; 32], &phone(), &desktop());
        assert_ne!(a, b);
    }

    #[test]
    fn identity_order_is_load_bearing() {
        let forward = ShortCode::derive(&[7u8; 32], &phone(), &desktop());
        let swapped = ShortCode::derive(&[7u8; 32], &desktop(), &phone());
        assert_ne!(forward, swapped);
    }

    #[test]
    fn substituting_either_identity_changes_the_code() {
        let baseline = ShortCode::derive(&[7u8; 32], &phone(), &desktop());
        let attacker = SpkiFingerprint::from_spki_der(b"attacker-key");
        assert_ne!(
            baseline,
            ShortCode::derive(&[7u8; 32], &attacker, &desktop())
        );
        assert_ne!(baseline, ShortCode::derive(&[7u8; 32], &phone(), &attacker));
    }
}
