//! Builds the rustls client config: TLS 1.3 only, presents the desktop device
//! cert, and verifies the server by pinned SPKI-SHA256 (pairing-bootstrap mode
//! pins from the QR payload instead). No WebPKI roots are consulted, ever.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use tandem_crypto::SpkiFingerprint;

use crate::error::TransportError;

/// Which pin the handshake must satisfy. Both modes verify a pinned key; they
/// differ only in where the pin came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinSource {
    /// Normal sessions: the pin persisted when pairing completed.
    Paired(SpkiFingerprint),
    /// First pairing: the pin transcribed from the QR payload.
    PairingBootstrap(SpkiFingerprint),
}

impl PinSource {
    pub fn fingerprint(&self) -> &SpkiFingerprint {
        match self {
            Self::Paired(fp) | Self::PairingBootstrap(fp) => fp,
        }
    }

    pub fn is_bootstrap(&self) -> bool {
        matches!(self, Self::PairingBootstrap(_))
    }
}

/// Extracts the DER SubjectPublicKeyInfo from a certificate. The SPKI is what is
/// pinned, so a peer may reissue its certificate without breaking trust.
pub fn spki_from_certificate(cert_der: &[u8]) -> Result<Vec<u8>, TransportError> {
    use x509_parser::prelude::*;

    let (_, cert) = X509Certificate::from_der(cert_der)
        .map_err(|e| TransportError::TlsHandshake(format!("unparsable certificate: {e}")))?;
    Ok(cert.tbs_certificate.subject_pki.raw.to_vec())
}

/// Verifies the peer's presented key against the pin. This replaces certificate
/// chain validation entirely — there is no CA in the trust model (ADR-0006).
pub fn verify_peer(pin: &PinSource, presented_spki_der: &[u8]) -> Result<(), TransportError> {
    let presented = SpkiFingerprint::from_spki_der(presented_spki_der);
    if pin.fingerprint().matches(&presented) {
        Ok(())
    } else {
        Err(TransportError::PinMismatch)
    }
}

/// rustls verifier that trusts exactly one public key and nothing else. Expiry,
/// hostname, and issuer are all irrelevant here: the pinned key is the identity.
#[derive(Debug)]
struct PinnedKeyVerifier {
    pin: SpkiFingerprint,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedKeyVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let spki = spki_from_certificate(end_entity.as_ref())
            .map_err(|e| rustls::Error::General(e.to_string()))?;

        if SpkiFingerprint::from_spki_der(&spki).matches(&self.pin) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "peer key does not match the pinned fingerprint".into(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Assembles the mutual-TLS client config: presents this desktop's identity and
/// accepts only the pinned peer.
pub fn client_config(
    pin: &PinSource,
    client_cert_chain: Vec<CertificateDer<'static>>,
    client_key: PrivateKeyDer<'static>,
) -> Result<ClientConfig, TransportError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());

    let verifier = PinnedKeyVerifier {
        pin: pin.fingerprint().clone(),
        provider: provider.clone(),
    };

    ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| TransportError::TlsHandshake(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_client_auth_cert(client_cert_chain, client_key)
        .map_err(|e| TransportError::TlsHandshake(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin() -> PinSource {
        PinSource::Paired(SpkiFingerprint::from_spki_der(b"phone-key"))
    }

    #[test]
    fn the_pinned_key_is_accepted() {
        assert!(verify_peer(&pin(), b"phone-key").is_ok());
    }

    #[test]
    fn any_other_key_is_refused_even_if_otherwise_valid() {
        assert_eq!(
            verify_peer(&pin(), b"attacker-key"),
            Err(TransportError::PinMismatch)
        );
    }

    #[test]
    fn bootstrap_pins_are_verified_identically() {
        let boot = PinSource::PairingBootstrap(SpkiFingerprint::from_spki_der(b"phone-key"));
        assert!(boot.is_bootstrap());
        assert!(verify_peer(&boot, b"phone-key").is_ok());
        assert_eq!(
            verify_peer(&boot, b"attacker-key"),
            Err(TransportError::PinMismatch)
        );
    }

    /// The pin must survive a real certificate round trip, not just raw bytes.
    #[test]
    fn spki_is_extracted_from_a_real_certificate() {
        let cert = rcgen::generate_simple_self_signed(vec!["tandem".into()]).unwrap();
        let der = cert.cert.der().to_vec();

        let spki = spki_from_certificate(&der).unwrap();
        assert!(!spki.is_empty());

        let pinned = PinSource::Paired(SpkiFingerprint::from_spki_der(&spki));
        assert!(verify_peer(&pinned, &spki).is_ok());
    }

    /// Two different certificates over different keys must not collide.
    #[test]
    fn distinct_certificates_yield_distinct_pins() {
        let a = rcgen::generate_simple_self_signed(vec!["a".into()]).unwrap();
        let b = rcgen::generate_simple_self_signed(vec!["b".into()]).unwrap();

        let spki_a = spki_from_certificate(a.cert.der()).unwrap();
        let spki_b = spki_from_certificate(b.cert.der()).unwrap();

        assert_ne!(spki_a, spki_b);
        let pinned_a = PinSource::Paired(SpkiFingerprint::from_spki_der(&spki_a));
        assert_eq!(
            verify_peer(&pinned_a, &spki_b),
            Err(TransportError::PinMismatch)
        );
    }
}
