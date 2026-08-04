//! Builds the rustls client config: TLS 1.3 only, presents the desktop device
//! cert, and verifies the server by pinned SPKI-SHA256 (pairing-bootstrap mode
//! pins from the QR payload instead). No WebPKI roots are consulted, ever.

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
}
