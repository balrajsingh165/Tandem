//! Desktop-issued pairing offer: the payload this computer renders as a QR code
//! for the phone's camera, and the exchange that follows. The phone authenticates
//! this desktop by scanning its key fingerprint; this desktop learns the phone's
//! key on connect and pins it from then on, with the short code as the check
//! against a machine in the middle (docs/07).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tandem_crypto::SpkiFingerprint;
use tandem_proto::{envelope::Payload, ErrorCode, PairingRequest};
use tandem_transport::client::WsTransportClient;
use tandem_transport::discovery;
use tandem_transport::tls::{client_config_observing, PinSource};

use crate::error::PairingError;
use crate::flow::{DesktopCredentials, PairedPhoneRecord, PROTOCOL_MAX, PROTOCOL_MIN};

/// How long the desktop hunts for a phone advertising itself on the LAN.
pub const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the code on screen stays live. The phone only opens its side after
/// the user scans, so the desktop has to keep offering itself until then.
pub const OFFER_LIFETIME: Duration = Duration::from_secs(900);

/// Pause between connection attempts while waiting to be scanned.
pub const RETRY_INTERVAL: Duration = Duration::from_secs(2);

/// Wire form of the offer. Compact keys keep the QR small enough to scan from a
/// screen at arm's length.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopOffer {
    /// Payload version, so a future format can be refused rather than misread.
    pub v: u32,
    /// base64url SPKI-SHA256 of this desktop; the phone pins it after scanning.
    pub fp: String,
    /// One-time token proving the connecting desktop is the one on screen.
    pub tok: String,
    /// Display name the phone shows in its confirmation sheet.
    pub name: String,
}

pub const OFFER_VERSION: u32 = 1;

impl DesktopOffer {
    pub fn new(fingerprint: &SpkiFingerprint, token: String, name: String) -> Self {
        Self {
            v: OFFER_VERSION,
            fp: fingerprint.to_base64url(),
            tok: token,
            name,
        }
    }

    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn parse(raw: &str) -> Result<Self, PairingError> {
        let offer: Self = serde_json::from_str(raw.trim()).map_err(|_| PairingError::InvalidQr)?;
        if offer.v != OFFER_VERSION {
            return Err(PairingError::UnsupportedQrVersion(offer.v));
        }
        if offer.fp.is_empty() || offer.tok.is_empty() {
            return Err(PairingError::InvalidQr);
        }
        Ok(offer)
    }
}

/// The phone that scanned the code, as this desktop's own user must see it
/// before anything is shared. Discovery names the phone; the key is what the
/// desktop will pin, so both are shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneIntroduction {
    pub phone_name: String,
    pub phone_fingerprint: String,
}

/// Progress the UI renders while the user points their phone at the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfferState {
    /// Showing the code, nothing has scanned it yet.
    Waiting,
    /// An attempt came back without reaching the phone. Carries why, so a
    /// pairing that never completes can be diagnosed instead of guessed at.
    Retrying { reason: String },
    /// A phone was found on the network; opening a session.
    Connecting { phone_name: String },
    /// A phone answered; this desktop's user has to approve it before the
    /// pairing request is sent.
    AwaitingLocalApproval(PhoneIntroduction),
    /// Waiting for the tap on the phone.
    AwaitingConfirmation,
    Accepted(PairedPhoneRecord),
    Failed(PairingError),
}

/// Runs the desktop side of a scan-to-pair exchange to a verdict.
///
/// Nothing tells this desktop when the code was scanned, and the phone rejects
/// connections until it has been, so attempts repeat until one gets through or
/// the offer expires. Only a refusal the phone actually sent ends it early.
pub async fn run<C, F>(
    offer: &DesktopOffer,
    credentials: &DesktopCredentials,
    mut on_progress: impl FnMut(&OfferState),
    confirm: C,
) -> Result<PairedPhoneRecord, PairingError>
where
    C: Fn(PhoneIntroduction) -> F,
    F: std::future::Future<Output = bool>,
{
    let deadline = std::time::Instant::now() + OFFER_LIFETIME;
    let mut last;

    on_progress(&OfferState::Waiting);

    loop {
        match attempt(offer, credentials, &mut on_progress, &confirm).await {
            Ok(record) => return Ok(record),
            Err(error) if is_final(&error) => return Err(error),
            Err(error) => {
                on_progress(&OfferState::Retrying {
                    reason: error.to_string(),
                });
                last = error;
            }
        }

        if std::time::Instant::now() >= deadline {
            return Err(last);
        }
        tokio::time::sleep(RETRY_INTERVAL).await;
    }
}

/// Whether an attempt failed for a reason retrying cannot fix. A phone that
/// answered and said no is final; anything short of an answer is just a phone
/// that has not been pointed at the screen yet.
fn is_final(error: &PairingError) -> bool {
    matches!(
        error,
        PairingError::RejectedByUser | PairingError::VersionNegotiationFailed { .. }
    )
}

/// One connection attempt against whichever phone is currently advertising.
async fn attempt<C, F>(
    offer: &DesktopOffer,
    credentials: &DesktopCredentials,
    on_progress: &mut impl FnMut(&OfferState),
    confirm: &C,
) -> Result<PairedPhoneRecord, PairingError>
where
    C: Fn(PhoneIntroduction) -> F,
    F: std::future::Future<Output = bool>,
{
    // The phone advertises itself; without that there is nothing to dial, since
    // a QR shown on a screen cannot carry the phone's own address.
    let phone = discovery::find_any_phone(DISCOVERY_TIMEOUT)
        .await
        .map_err(|e| PairingError::Transport(e.to_string()))?
        .ok_or_else(|| PairingError::Transport("no Tandem phone found on this network".into()))?;

    let cert = rustls_pki_types::CertificateDer::from(credentials.cert_der.clone());
    let key = rustls_pki_types::PrivateKeyDer::try_from(credentials.key_der.clone())
        .map_err(|e| PairingError::Transport(format!("unusable device key: {e}")))?;

    let (tls, observed) =
        client_config_observing(&PinSource::TrustOnFirstUse, vec![cert], key)
            .map_err(|e| PairingError::Transport(e.to_string()))?;

    // A phone that has not been pointed at the screen refuses the handshake, so
    // reaching this line — not merely finding the phone — is what "connecting"
    // means. Reporting it any earlier would claim a scan that never happened.
    let mut client = WsTransportClient::connect_provisional(&phone.host, phone.port, tls, 1)
        .await
        .map_err(|e| PairingError::Transport(e.to_string()))?;

    on_progress(&OfferState::Connecting {
        phone_name: phone.display_name.clone(),
    });

    // Whatever key answered is the one this desktop will pin, so a later session
    // reaching a different device fails closed.
    let phone_key = observed
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .ok_or_else(|| PairingError::Transport("no peer key observed".into()))?;

    // Nothing has been shared yet: the desktop's own user gets to see which
    // phone picked up the code before its certificate goes across.
    let introduction = PhoneIntroduction {
        phone_name: phone.display_name.clone(),
        phone_fingerprint: phone_key.to_base64url(),
    };
    on_progress(&OfferState::AwaitingLocalApproval(introduction.clone()));
    if !confirm(introduction).await {
        return Err(PairingError::RejectedByUser);
    }

    client
        .send_payload(Payload::PairingRequest(PairingRequest {
            pairing_token: offer.tok.clone(),
            desktop_cert_der: credentials.cert_der.clone(),
            desktop_name: credentials.name.clone(),
            desktop_platform: credentials.platform.clone(),
            protocol_min: PROTOCOL_MIN,
            protocol_max: PROTOCOL_MAX,
        }))
        .await
        .map_err(|e| PairingError::Transport(e.to_string()))?;

    loop {
        let payload = client
            .next_payload()
            .await
            .map_err(|e| PairingError::Transport(e.to_string()))?;

        match payload {
            Payload::PairingAwaitConfirmEvent(_) => {
                on_progress(&OfferState::AwaitingConfirmation);
            }

            Payload::PairingDecision(decision) => {
                let accepted = decision
                    .status
                    .as_ref()
                    .map(|s| s.code == ErrorCode::Ok as i32)
                    .unwrap_or(false);
                if !accepted {
                    return Err(PairingError::RejectedByUser);
                }

                let record = PairedPhoneRecord {
                    desktop_device_id: decision.desktop_device_id,
                    phone_device_id: decision.phone_device_id,
                    phone_name: decision.phone_name,
                    protocol_version: decision.protocol_version.max(PROTOCOL_MIN),
                    phone_bt_address: decision.phone_bt_address,
                    phone_spki_sha256: phone_key,
                };
                on_progress(&OfferState::Accepted(record.clone()));
                return Ok(record);
            }

            Payload::RevokedEvent(_) => return Err(PairingError::RejectedByUser),
            _ => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The phone refuses every connection until the user scans, so a desktop
    /// that gave up on the first refusal could never be paired with.
    #[test]
    fn only_an_answer_from_the_phone_ends_the_offer() {
        assert!(is_final(&PairingError::RejectedByUser));
        assert!(is_final(&PairingError::VersionNegotiationFailed {
            desktop_min: 1,
            desktop_max: 1,
        }));
        assert!(!is_final(&PairingError::Transport("refused".into())));
        assert!(!is_final(&PairingError::Transport(
            "no Tandem phone found on this network".into()
        )));
    }

    /// The code stays on screen far longer than one attempt, and retries have to
    /// be frequent enough that a scan is answered while the phone still waits.
    #[test]
    fn the_offer_outlives_a_single_attempt() {
        assert!(OFFER_LIFETIME > DISCOVERY_TIMEOUT + RETRY_INTERVAL);
        assert!(RETRY_INTERVAL < Duration::from_secs(5));
    }

    fn offer() -> DesktopOffer {
        DesktopOffer::new(
            &SpkiFingerprint::from_spki_der(b"desktop-key"),
            "one-time-token".into(),
            "Balraj PC".into(),
        )
    }

    #[test]
    fn an_offer_round_trips_through_its_payload() {
        let parsed = DesktopOffer::parse(&offer().encode()).unwrap();
        assert_eq!(parsed, offer());
        assert_eq!(parsed.name, "Balraj PC");
    }

    /// The payload has to stay small enough to scan comfortably from a screen.
    #[test]
    fn the_payload_stays_compact() {
        assert!(offer().encode().len() < 160, "{}", offer().encode());
    }

    #[test]
    fn an_unknown_version_is_refused_rather_than_guessed() {
        let raw = offer().encode().replace("\"v\":1", "\"v\":9");
        assert_eq!(
            DesktopOffer::parse(&raw),
            Err(PairingError::UnsupportedQrVersion(9))
        );
    }

    #[test]
    fn a_payload_missing_its_secret_is_invalid() {
        let raw = offer().encode().replace("one-time-token", "");
        assert_eq!(DesktopOffer::parse(&raw), Err(PairingError::InvalidQr));
        assert_eq!(DesktopOffer::parse("not json"), Err(PairingError::InvalidQr));
    }

    #[test]
    fn the_offer_carries_this_desktop_key() {
        let fingerprint = SpkiFingerprint::from_spki_der(b"desktop-key");
        assert_eq!(offer().fp, fingerprint.to_base64url());
    }
}
