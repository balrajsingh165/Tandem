//! End-to-end pairing tests: a real TLS phone that validates the one-time token,
//! asks for confirmation, and issues a verdict — exercising the desktop's actual
//! pairing flow including QR-pin enforcement and token replay refusal.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, ServerConfig, SignatureScheme};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, UnixTime};
use tandem_crypto::SpkiFingerprint;
use tandem_pairing::flow::{DesktopCredentials, PairingFlow, PairingState};
use tandem_pairing::{PairingError, QrPayload};
use tandem_proto::{
    envelope::Payload, Envelope, ErrorCode, PairingAwaitConfirmEvent, PairingDecision, Status,
};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug)]
struct AcceptAnyClient;

impl ClientCertVerifier for AcceptAnyClient {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

struct Issued {
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    spki: Vec<u8>,
}

fn issue(name: &str) -> Issued {
    let generated = rcgen::generate_simple_self_signed(vec![name.to_string()]).unwrap();
    let cert_der = generated.cert.der().to_vec();
    let spki = tandem_transport::tls::spki_from_certificate(&cert_der).unwrap();
    Issued {
        cert_der,
        key_der: generated.key_pair.serialize_der(),
        spki,
    }
}

/// How the scripted phone should answer a pairing attempt.
#[derive(Clone, Copy)]
enum Verdict {
    Accept { require_short_code: bool },
    Reject,
}

/// Boots a phone that accepts exactly one valid token, then refuses replays.
async fn spawn_phone(
    expected_token: &'static str,
    verdict: Verdict,
) -> (u16, Vec<u8>, Arc<AtomicUsize>) {
    let phone = issue("tandem.local");
    let spki = phone.spki.clone();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_task = attempts.clone();

    let server_config = ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(AcceptAnyClient))
        .with_single_cert(
            vec![CertificateDer::from(phone.cert_der)],
            PrivateKeyDer::try_from(phone.key_der).unwrap(),
        )
        .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    tokio::spawn(async move {
        let mut token_used = false;

        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            let Ok(tls) = acceptor.accept(tcp).await else {
                continue;
            };
            let Ok(mut ws) = tokio_tungstenite::accept_async(tls).await else {
                continue;
            };

            let Some(Ok(Message::Binary(bytes))) = ws.next().await else {
                continue;
            };
            let envelope = tandem_transport::codec::EnvelopeCodec::decode(&bytes).unwrap();
            attempts_for_task.fetch_add(1, Ordering::SeqCst);

            let Some(Payload::PairingRequest(request)) = envelope.payload else {
                continue;
            };

            // A one-time token is consumed by the first attempt that presents it,
            // so a replay is refused even with the correct value.
            let token_ok = request.pairing_token == expected_token && !token_used;
            token_used = true;

            let decision = if !token_ok {
                PairingDecision {
                    status: Some(Status {
                        code: ErrorCode::PairingRejected as i32,
                        message: "token rejected".into(),
                    }),
                    ..Default::default()
                }
            } else {
                match verdict {
                    Verdict::Reject => PairingDecision {
                        status: Some(Status {
                            code: ErrorCode::PairingRejected as i32,
                            message: "declined on the phone".into(),
                        }),
                        ..Default::default()
                    },
                    Verdict::Accept { require_short_code } => {
                        let confirm = Envelope {
                            protocol_version: 1,
                            message_id: 10,
                            in_reply_to: 0,
                            payload: Some(Payload::PairingAwaitConfirmEvent(
                                PairingAwaitConfirmEvent { require_short_code },
                            )),
                        };
                        ws.send(Message::Binary(
                            tandem_transport::codec::EnvelopeCodec::encode(&confirm).unwrap(),
                        ))
                        .await
                        .unwrap();

                        PairingDecision {
                            status: Some(Status {
                                code: ErrorCode::Ok as i32,
                                message: String::new(),
                            }),
                            desktop_device_id: "desktop-assigned-1".into(),
                            phone_device_id: "phone-1".into(),
                            phone_name: "Pixel".into(),
                            protocol_version: 1,
                            phone_bt_address: "AA:BB:CC:DD:EE:FF".into(),
                        }
                    }
                }
            };

            let reply = Envelope {
                protocol_version: 1,
                message_id: 11,
                in_reply_to: envelope.message_id,
                payload: Some(Payload::PairingDecision(decision)),
            };
            ws.send(Message::Binary(
                tandem_transport::codec::EnvelopeCodec::encode(&reply).unwrap(),
            ))
            .await
            .unwrap();
        }
    });

    (port, spki, attempts)
}

fn credentials() -> DesktopCredentials {
    let desktop = issue("tandem-desktop");
    DesktopCredentials {
        name: "Test Desktop".into(),
        platform: "windows".into(),
        cert_der: desktop.cert_der,
        key_der: desktop.key_der,
    }
}

fn invitation(port: u16, pin_spki: &[u8], token: &str) -> QrPayload {
    QrPayload {
        host: "127.0.0.1".into(),
        port,
        fingerprint: SpkiFingerprint::from_spki_der(pin_spki),
        token: token.into(),
        phone_name: "Pixel".into(),
    }
}

#[tokio::test]
async fn a_confirmed_pairing_yields_a_persistable_record() {
    let (port, spki, _) = spawn_phone(
        "correct-token",
        Verdict::Accept {
            require_short_code: false,
        },
    )
    .await;

    let mut flow = PairingFlow::new(invitation(port, &spki, "correct-token"));
    let record = flow.run(&credentials(), |_| {}).await.unwrap();

    assert_eq!(record.desktop_device_id, "desktop-assigned-1");
    assert_eq!(record.phone_device_id, "phone-1");
    assert_eq!(record.phone_name, "Pixel");
    assert_eq!(record.protocol_version, 1);
    assert_eq!(record.phone_bt_address, "AA:BB:CC:DD:EE:FF");

    // The pinned key must be the one the QR promised, since every later session
    // authenticates against it.
    assert_eq!(
        record.phone_spki_sha256,
        SpkiFingerprint::from_spki_der(&spki)
    );
}

#[tokio::test]
async fn progress_moves_through_connecting_and_confirmation() {
    let (port, spki, _) = spawn_phone(
        "correct-token",
        Verdict::Accept {
            require_short_code: false,
        },
    )
    .await;

    let mut seen = Vec::new();
    let mut flow = PairingFlow::new(invitation(port, &spki, "correct-token"));
    flow.run(&credentials(), |state| {
        seen.push(std::mem::discriminant(state));
    })
    .await
    .unwrap();

    assert_eq!(seen.len(), 3);
    assert!(matches!(flow.state(), PairingState::Accepted(_)));
}

/// The manual path shows six digits bound to this TLS session on both screens.
#[tokio::test]
async fn the_manual_path_derives_a_six_digit_short_code() {
    let (port, spki, _) = spawn_phone(
        "correct-token",
        Verdict::Accept {
            require_short_code: true,
        },
    )
    .await;

    let mut codes = Vec::new();
    let mut flow = PairingFlow::new(invitation(port, &spki, "correct-token"));
    flow.run(&credentials(), |state| {
        if let PairingState::AwaitingConfirmation {
            short_code: Some(code),
        } = state
        {
            codes.push(code.as_str().to_string());
        }
    })
    .await
    .unwrap();

    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].len(), 6);
    assert!(codes[0].chars().all(|c| c.is_ascii_digit()));
}

#[tokio::test]
async fn a_declined_pairing_reports_user_rejection() {
    let (port, spki, _) = spawn_phone("correct-token", Verdict::Reject).await;

    let mut flow = PairingFlow::new(invitation(port, &spki, "correct-token"));
    let result = flow.run(&credentials(), |_| {}).await;

    assert_eq!(result, Err(PairingError::RejectedByUser));
    assert!(matches!(flow.state(), PairingState::Failed(_)));
}

/// A phone that cannot prove the QR's key must never receive the token.
#[tokio::test]
async fn an_impostor_phone_never_sees_the_pairing_token() {
    let (port, _real_spki, attempts) = spawn_phone(
        "correct-token",
        Verdict::Accept {
            require_short_code: false,
        },
    )
    .await;
    let impostor = issue("attacker");

    let mut flow = PairingFlow::new(invitation(port, &impostor.spki, "correct-token"));
    let result = flow.run(&credentials(), |_| {}).await;

    assert!(matches!(result, Err(PairingError::Transport(_))));
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        0,
        "the token must not reach a phone that failed the pin"
    );
}

/// One-time means one-time: a replayed token is refused even when correct.
#[tokio::test]
async fn a_replayed_token_is_refused() {
    let (port, spki, _) = spawn_phone(
        "correct-token",
        Verdict::Accept {
            require_short_code: false,
        },
    )
    .await;

    let mut first = PairingFlow::new(invitation(port, &spki, "correct-token"));
    assert!(first.run(&credentials(), |_| {}).await.is_ok());

    let mut replay = PairingFlow::new(invitation(port, &spki, "correct-token"));
    assert_eq!(
        replay.run(&credentials(), |_| {}).await,
        Err(PairingError::RejectedByUser)
    );
}
