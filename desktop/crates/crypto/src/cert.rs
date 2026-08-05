//! Generates the long-lived self-signed X.509 certificate over the identity key
//! (rcgen) used as the TLS carrier for mutual authentication.

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};

use crate::error::CryptoError;

/// Certificates are long-lived because trust is the pinned key, not expiry
/// (docs/08 key-rotation section).
pub const VALIDITY_DAYS: u32 = 3650;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfSignedCert {
    pub cert_der: Vec<u8>,
    pub spki_der: Vec<u8>,
    pub key_der: Vec<u8>,
}

/// Issues a self-signed certificate wrapping a fresh P-256 identity key.
pub fn issue(common_name: &str) -> Result<SelfSignedCert, CryptoError> {
    let key_pair =
        KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).map_err(|_| CryptoError::KeyGeneration)?;
    issue_with_key(common_name, &key_pair)
}

/// Re-issues a certificate over an existing key, so a reissue does not change
/// the pinned identity.
pub fn issue_with_key(
    common_name: &str,
    key_pair: &KeyPair,
) -> Result<SelfSignedCert, CryptoError> {
    let mut params = CertificateParams::new(vec![common_name.to_string()])
        .map_err(|_| CryptoError::Certificate)?;

    let mut name = DistinguishedName::new();
    name.push(DnType::CommonName, common_name);
    params.distinguished_name = name;

    let cert = params
        .self_signed(key_pair)
        .map_err(|_| CryptoError::Certificate)?;

    Ok(SelfSignedCert {
        cert_der: cert.der().to_vec(),
        spki_der: key_pair.public_key_der(),
        key_der: key_pair.serialize_der(),
    })
}

/// Loads a stored PKCS#8 key back into a usable keypair.
pub fn key_from_der(key_der: &[u8]) -> Result<KeyPair, CryptoError> {
    KeyPair::try_from(key_der).map_err(|_| CryptoError::KeyGeneration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pinning::SpkiFingerprint;

    #[test]
    fn issuing_produces_a_usable_certificate_and_key() {
        let issued = issue("tandem-desktop").unwrap();
        assert!(!issued.cert_der.is_empty());
        assert!(!issued.spki_der.is_empty());
        assert!(!issued.key_der.is_empty());
    }

    #[test]
    fn two_issues_yield_distinct_identities() {
        let a = issue("tandem-desktop").unwrap();
        let b = issue("tandem-desktop").unwrap();
        assert_ne!(a.spki_der, b.spki_der);
        assert_ne!(
            SpkiFingerprint::from_spki_der(&a.spki_der),
            SpkiFingerprint::from_spki_der(&b.spki_der)
        );
    }

    /// Reissuing over the same key must preserve the pin, or every reissue would
    /// break pairing.
    #[test]
    fn reissuing_over_the_same_key_preserves_the_pin() {
        let first = issue("tandem-desktop").unwrap();
        let key = key_from_der(&first.key_der).unwrap();
        let second = issue_with_key("tandem-desktop", &key).unwrap();

        assert_eq!(first.spki_der, second.spki_der);
        assert_eq!(
            SpkiFingerprint::from_spki_der(&first.spki_der),
            SpkiFingerprint::from_spki_der(&second.spki_der)
        );
    }

    #[test]
    fn a_stored_key_round_trips() {
        let issued = issue("tandem-desktop").unwrap();
        let restored = key_from_der(&issued.key_der).unwrap();
        assert_eq!(restored.public_key_der(), issued.spki_der);
    }
}
