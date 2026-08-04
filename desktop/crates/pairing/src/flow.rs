//! Pairing flow driver: provisional TLS connect (pin from QR), PairingRequest
//! submission, PairingAwaitConfirmEvent handling, and PairingDecision
//! finalization — persisting the phone identity and this desktop's assigned
//! device id.

use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use tandem_crypto::SpkiFingerprint;
use tandem_proto::{envelope::Payload, ErrorCode, PairingRequest};
use tandem_transport::client::WsTransportClient;
use tandem_transport::tls::{client_config, PinSource};

use crate::error::PairingError;
use crate::qr::QrPayload;
use crate::short_code::ShortCode;

/// Observable pairing progress, surfaced to the UI as it advances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingState {
    Scanned,
    Connecting,
    AwaitingConfirmation { short_code: Option<ShortCode> },
    Accepted(PairedPhoneRecord),
    Failed(PairingError),
}

/// What the desktop persists once the phone accepts it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedPhoneRecord {
    pub desktop_device_id: String,
    pub phone_device_id: String,
    pub phone_name: String,
    pub protocol_version: u32,
    pub phone_bt_address: String,
    /// The phone key this desktop pins for every later session.
    pub phone_spki_sha256: SpkiFingerprint,
}

/// Version window this desktop advertises in `PairingRequest`.
pub const PROTOCOL_MIN: u32 = 1;
pub const PROTOCOL_MAX: u32 = 1;

/// This desktop's identity as presented during pairing.
#[derive(Debug, Clone)]
pub struct DesktopCredentials {
    pub name: String,
    pub platform: String,
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
}

impl DesktopCredentials {
    fn rustls_parts(
        &self,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), PairingError> {
        let cert = CertificateDer::from(self.cert_der.clone());
        let key = PrivateKeyDer::try_from(self.key_der.clone())
            .map_err(|e| PairingError::Transport(format!("unusable device key: {e}")))?;
        Ok((vec![cert], key))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingFlow {
    invitation: QrPayload,
    state: PairingState,
}

impl PairingFlow {
    pub fn new(invitation: QrPayload) -> Self {
        Self {
            invitation,
            state: PairingState::Scanned,
        }
    }

    pub fn state(&self) -> &PairingState {
        &self.state
    }

    pub fn invitation(&self) -> &QrPayload {
        &self.invitation
    }

    pub fn begin_connect(&mut self) {
        self.state = PairingState::Connecting;
    }

    pub fn await_confirmation(&mut self, short_code: Option<ShortCode>) {
        self.state = PairingState::AwaitingConfirmation { short_code };
    }

    pub fn accept(&mut self, record: PairedPhoneRecord) {
        self.state = PairingState::Accepted(record);
    }

    pub fn fail(&mut self, error: PairingError) {
        self.state = PairingState::Failed(error);
    }

    /// The phone picks the highest mutually supported version; anything outside
    /// this desktop's window aborts pairing rather than guessing.
    pub fn accept_version(&self, chosen: u32) -> Result<u32, PairingError> {
        if (PROTOCOL_MIN..=PROTOCOL_MAX).contains(&chosen) {
            Ok(chosen)
        } else {
            Err(PairingError::VersionNegotiationFailed {
                desktop_min: PROTOCOL_MIN,
                desktop_max: PROTOCOL_MAX,
            })
        }
    }

    /// Runs the exchange to a verdict. The QR fingerprint pins the TLS peer, so
    /// a phone that cannot prove that key never sees the pairing token.
    pub async fn run(
        &mut self,
        credentials: &DesktopCredentials,
        mut on_progress: impl FnMut(&PairingState),
    ) -> Result<PairedPhoneRecord, PairingError> {
        self.begin_connect();
        on_progress(&self.state);

        let pin = PinSource::PairingBootstrap(self.invitation.fingerprint.clone());
        let (chain, key) = credentials.rustls_parts()?;
        let tls =
            client_config(&pin, chain, key).map_err(|e| PairingError::Transport(e.to_string()))?;

        let mut client = WsTransportClient::connect_provisional(
            &self.invitation.host,
            self.invitation.port,
            tls,
            1,
        )
        .await
        .map_err(|e| self.record_failure(PairingError::Transport(e.to_string())))?;

        let request = Payload::PairingRequest(PairingRequest {
            pairing_token: self.invitation.token.clone(),
            desktop_cert_der: credentials.cert_der.clone(),
            desktop_name: credentials.name.clone(),
            desktop_platform: credentials.platform.clone(),
            protocol_min: PROTOCOL_MIN,
            protocol_max: PROTOCOL_MAX,
        });

        client
            .send_payload(request)
            .await
            .map_err(|e| self.record_failure(PairingError::Transport(e.to_string())))?;

        loop {
            let payload = client
                .next_payload()
                .await
                .map_err(|e| self.record_failure(PairingError::Transport(e.to_string())))?;

            match payload {
                Payload::PairingAwaitConfirmEvent(event) => {
                    // The short code is only meaningful on the manual path; on the
                    // QR path the scan already bound both identities.
                    let code = if event.require_short_code {
                        Some(self.derive_short_code(&client, credentials)?)
                    } else {
                        None
                    };
                    self.await_confirmation(code);
                    on_progress(&self.state);
                }

                Payload::PairingDecision(decision) => {
                    let ok = decision
                        .status
                        .as_ref()
                        .map(|s| s.code == ErrorCode::Ok as i32)
                        .unwrap_or(false);

                    if !ok {
                        let error = match decision.status.as_ref().map(|s| s.code) {
                            Some(code) if code == ErrorCode::PairingRejected as i32 => {
                                PairingError::RejectedByUser
                            }
                            _ => PairingError::RejectedByUser,
                        };
                        return Err(self.record_failure(error));
                    }

                    let version = self
                        .accept_version(decision.protocol_version)
                        .map_err(|e| self.record_failure(e))?;

                    let record = PairedPhoneRecord {
                        desktop_device_id: decision.desktop_device_id,
                        phone_device_id: decision.phone_device_id,
                        phone_name: decision.phone_name,
                        protocol_version: version,
                        phone_bt_address: decision.phone_bt_address,
                        phone_spki_sha256: self.invitation.fingerprint.clone(),
                    };
                    self.accept(record.clone());
                    on_progress(&self.state);
                    return Ok(record);
                }

                Payload::RevokedEvent(_) => {
                    return Err(self.record_failure(PairingError::RejectedByUser))
                }

                // Anything else during pairing is noise from a confused peer.
                _ => continue,
            }
        }
    }

    /// Binds the displayed digits to this TLS session and to both identities, so
    /// a matching code on both screens rules out a machine in the middle.
    fn derive_short_code(
        &self,
        client: &WsTransportClient,
        credentials: &DesktopCredentials,
    ) -> Result<ShortCode, PairingError> {
        let exporter = client
            .tls_exporter(
                crate::short_code::EXPORTER_LABEL,
                crate::short_code::EXPORTER_LENGTH,
            )
            .map_err(|e| PairingError::Transport(e.to_string()))?;

        let desktop_spki = tandem_transport::tls::spki_from_certificate(&credentials.cert_der)
            .map_err(|e| PairingError::Transport(e.to_string()))?;

        Ok(ShortCode::derive(
            &exporter,
            &self.invitation.fingerprint,
            &SpkiFingerprint::from_spki_der(&desktop_spki),
        ))
    }

    fn record_failure(&mut self, error: PairingError) -> PairingError {
        self.fail(error.clone());
        error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tandem_crypto::SpkiFingerprint;

    fn flow() -> PairingFlow {
        PairingFlow::new(QrPayload {
            host: "192.168.1.20".into(),
            port: 46521,
            fingerprint: SpkiFingerprint::from_spki_der(b"phone-key"),
            token: "tok".into(),
            phone_name: "Pixel".into(),
        })
    }

    #[test]
    fn advances_through_the_happy_path() {
        let mut f = flow();
        assert_eq!(*f.state(), PairingState::Scanned);
        f.begin_connect();
        assert_eq!(*f.state(), PairingState::Connecting);
        f.await_confirmation(None);
        assert!(matches!(
            f.state(),
            PairingState::AwaitingConfirmation { .. }
        ));
        f.accept(PairedPhoneRecord {
            desktop_device_id: "d1".into(),
            phone_device_id: "p1".into(),
            phone_name: "Pixel".into(),
            protocol_version: 1,
            phone_bt_address: String::new(),
            phone_spki_sha256: SpkiFingerprint::from_spki_der(b"phone-key"),
        });
        assert!(matches!(f.state(), PairingState::Accepted(_)));
    }

    #[test]
    fn unsupported_version_aborts_rather_than_guessing() {
        assert_eq!(flow().accept_version(1).unwrap(), 1);
        assert_eq!(
            flow().accept_version(2),
            Err(PairingError::VersionNegotiationFailed {
                desktop_min: 1,
                desktop_max: 1
            })
        );
    }
}
