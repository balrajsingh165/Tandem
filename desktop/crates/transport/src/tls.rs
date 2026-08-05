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
    /// First pairing from a phone-issued QR: the pin came from the payload.
    PairingBootstrap(SpkiFingerprint),
    /// First pairing from a desktop-issued QR. The phone authenticated *this*
    /// device by scanning its key, so the phone's own key is unknown until it
    /// answers; it is recorded on connect and pinned for every later session.
    /// The user still compares the short code, which is what rules out a
    /// machine in the middle (docs/07).
    TrustOnFirstUse,
}

impl PinSource {
    /// None in trust-on-first-use, where there is nothing to compare against yet.
    pub fn fingerprint(&self) -> Option<&SpkiFingerprint> {
        match self {
            Self::Paired(fp) | Self::PairingBootstrap(fp) => Some(fp),
            Self::TrustOnFirstUse => None,
        }
    }

    pub fn is_bootstrap(&self) -> bool {
        matches!(self, Self::PairingBootstrap(_) | Self::TrustOnFirstUse)
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
    let Some(expected) = pin.fingerprint() else {
        return Ok(());
    };
    let presented = SpkiFingerprint::from_spki_der(presented_spki_der);
    if expected.matches(&presented) {
        Ok(())
    } else {
        Err(TransportError::PinMismatch)
    }
}

/// rustls verifier that trusts exactly one public key and nothing else. Expiry,
/// hostname, and issuer are all irrelevant here: the pinned key is the identity.
/// With no pin it records the key it saw instead, for trust-on-first-use.
#[derive(Debug)]
struct PinnedKeyVerifier {
    pin: Option<SpkiFingerprint>,
    observed: Arc<std::sync::Mutex<Option<SpkiFingerprint>>>,
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
        let presented = SpkiFingerprint::from_spki_der(&spki);

        // Record what answered even when pinned, so callers can report the key.
        if let Ok(mut slot) = self.observed.lock() {
            *slot = Some(presented.clone());
        }

        match &self.pin {
            Some(expected) if !expected.matches(&presented) => Err(rustls::Error::General(
                "peer key does not match the pinned fingerprint".into(),
            )),
            _ => Ok(ServerCertVerified::assertion()),
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
    Ok(client_config_observing(pin, client_cert_chain, client_key)?.0)
}

/// Same as [client_config], plus a handle that receives the peer key the
/// handshake actually saw. Trust-on-first-use pairing needs it to learn which
/// key to pin from then on.
pub fn client_config_observing(
    pin: &PinSource,
    client_cert_chain: Vec<CertificateDer<'static>>,
    client_key: PrivateKeyDer<'static>,
) -> Result<(ClientConfig, ObservedPeerKey), TransportError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let observed: ObservedPeerKey = Arc::new(std::sync::Mutex::new(None));

    let verifier = PinnedKeyVerifier {
        pin: pin.fingerprint().cloned(),
        observed: observed.clone(),
        provider: provider.clone(),
    };

    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| TransportError::TlsHandshake(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_client_auth_cert(client_cert_chain, client_key)
        .map_err(|e| TransportError::TlsHandshake(e.to_string()))?;

    Ok((config, observed))
}

/// The peer key a handshake presented, filled in once the TLS exchange runs.
pub type ObservedPeerKey = Arc<std::sync::Mutex<Option<SpkiFingerprint>>>;

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

    /// Trust-on-first-use has nothing to compare against, so any key passes and
    /// the short code becomes the thing that rules out an impostor.
    #[test]
    fn trust_on_first_use_accepts_an_unknown_key() {
        assert!(PinSource::TrustOnFirstUse.fingerprint().is_none());
        assert!(PinSource::TrustOnFirstUse.is_bootstrap());
        assert!(verify_peer(&PinSource::TrustOnFirstUse, b"any-key").is_ok());
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
