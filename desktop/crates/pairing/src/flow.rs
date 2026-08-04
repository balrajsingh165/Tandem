//! Pairing flow driver: provisional TLS connect (pin from QR), PairingRequest
//! submission, PairingAwaitConfirmEvent handling, and PairingDecision
//! finalization — persisting the phone identity and this desktop's assigned
//! device id.

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
}

/// Version window this desktop advertises in `PairingRequest`.
pub const PROTOCOL_MIN: u32 = 1;
pub const PROTOCOL_MAX: u32 = 1;

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
