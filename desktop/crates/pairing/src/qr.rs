//! Parses and validates the pairing QR payload (host, port, SPKI fingerprint,
//! one-time token, name); rejects unknown versions and malformed fingerprints
//! before any network activity.

use serde::{Deserialize, Serialize};
use tandem_crypto::SpkiFingerprint;

use crate::error::PairingError;

/// Wire form of the QR payload the phone displays. Field names are the compact
/// keys pinned in docs/07 and must not be renamed.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawQrPayload {
    v: u32,
    host: String,
    port: u16,
    fp: String,
    tok: String,
    name: String,
}

/// Validated pairing invitation. Construction implies the version is supported
/// and the fingerprint is well formed, so no network step re-checks those.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrPayload {
    pub host: String,
    pub port: u16,
    pub fingerprint: SpkiFingerprint,
    pub token: String,
    pub phone_name: String,
}

/// Highest QR payload version this desktop understands.
pub const SUPPORTED_QR_VERSION: u32 = 1;

impl QrPayload {
    pub fn parse(scanned: &str) -> Result<Self, PairingError> {
        let raw: RawQrPayload =
            serde_json::from_str(scanned.trim()).map_err(|_| PairingError::InvalidQr)?;

        if raw.v != SUPPORTED_QR_VERSION {
            return Err(PairingError::UnsupportedQrVersion(raw.v));
        }
        if raw.host.is_empty() || raw.port == 0 || raw.tok.is_empty() {
            return Err(PairingError::InvalidQr);
        }

        let fingerprint =
            SpkiFingerprint::from_base64url(&raw.fp).map_err(|_| PairingError::InvalidQr)?;

        Ok(Self {
            host: raw.host,
            port: raw.port,
            fingerprint,
            token: raw.tok,
            phone_name: raw.name,
        })
    }

    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json(fp: &str) -> String {
        format!(
            r#"{{"v":1,"host":"192.168.1.20","port":46521,"fp":"{fp}","tok":"tok123","name":"Pixel"}}"#
        )
    }

    fn fp() -> String {
        SpkiFingerprint::from_spki_der(b"phone-key").to_base64url()
    }

    #[test]
    fn parses_a_valid_payload() {
        let payload = QrPayload::parse(&valid_json(&fp())).unwrap();
        assert_eq!(payload.endpoint(), "192.168.1.20:46521");
        assert_eq!(payload.token, "tok123");
        assert_eq!(payload.phone_name, "Pixel");
        assert_eq!(
            payload.fingerprint,
            SpkiFingerprint::from_spki_der(b"phone-key")
        );
    }

    #[test]
    fn rejects_unknown_versions_before_touching_the_network() {
        let json = valid_json(&fp()).replace(r#""v":1"#, r#""v":2"#);
        assert_eq!(
            QrPayload::parse(&json),
            Err(PairingError::UnsupportedQrVersion(2))
        );
    }

    #[test]
    fn rejects_malformed_fingerprints() {
        assert_eq!(
            QrPayload::parse(&valid_json("not-a-fingerprint")),
            Err(PairingError::InvalidQr)
        );
    }

    #[test]
    fn rejects_non_tandem_and_empty_fields() {
        assert_eq!(QrPayload::parse("https://example.com"), Err(PairingError::InvalidQr));
        let json = valid_json(&fp()).replace(r#""port":46521"#, r#""port":0"#);
        assert_eq!(QrPayload::parse(&json), Err(PairingError::InvalidQr));
    }
}
