//! End-to-end transport tests: a real rustls server speaking TLP over WebSocket,
//! exercising the pinned-key handshake, the SessionHello/SessionWelcome exchange,
//! and request correlation against the actual client implementation.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, ServerConfig, SignatureScheme};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, UnixTime};
use tandem_crypto::SpkiFingerprint;
use tandem_proto::{envelope::Payload, Ack, Envelope, ErrorCode, SessionWelcome, Status};
use tandem_transport::client::{ClientIdentity, TransportClient, WsTransportClient};
use tandem_transport::codec::EnvelopeCodec;
use tandem_transport::error::TransportError;
use tandem_transport::tls::{client_config, spki_from_certificate, PinSource};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::Message;

/// Accepts any client certificate: these tests exercise the desktop's pinning of
/// the phone, while the phone-side pin lives in the Android gateway.
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
    cert_der: CertificateDer<'static>,
    key_der: PrivateKeyDer<'static>,
    spki: Vec<u8>,
}

fn issue(name: &str) -> Issued {
    let generated = rcgen::generate_simple_self_signed(vec![name.to_string()]).unwrap();
    let cert_der = CertificateDer::from(generated.cert.der().to_vec());
    let key_der = PrivateKeyDer::try_from(generated.key_pair.serialize_der()).expect("PKCS#8 key");
    let spki = spki_from_certificate(cert_der.as_ref()).unwrap();
    Issued {
        cert_der,
        key_der,
        spki,
    }
}

/// Boots a TLS + WebSocket server that answers one session, then returns its
/// port and the SPKI a client must pin to reach it.
async fn spawn_phone(welcome: SessionWelcome) -> (u16, Vec<u8>) {
    let phone = issue("tandem.local");
    let spki = phone.spki.clone();

    let server_config = ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(AcceptAnyClient))
        .with_single_cert(vec![phone.cert_der], phone.key_der)
        .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let tls = acceptor.accept(tcp).await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(tls).await.unwrap();

        // Read SessionHello, answer SessionWelcome.
        let hello = match ws.next().await.unwrap().unwrap() {
            Message::Binary(bytes) => EnvelopeCodec::decode(&bytes).unwrap(),
            other => panic!("unexpected first frame: {other:?}"),
        };
        assert!(matches!(hello.payload, Some(Payload::SessionHello(_))));

        let reply = Envelope {
            protocol_version: 1,
            message_id: 1,
            in_reply_to: hello.message_id,
            payload: Some(Payload::SessionWelcome(welcome)),
        };
        ws.send(Message::Binary(EnvelopeCodec::encode(&reply).unwrap()))
            .await
            .unwrap();

        // Answer any further requests with an OK ack so correlation can be tested.
        while let Some(Ok(Message::Binary(bytes))) = ws.next().await {
            let request = EnvelopeCodec::decode(&bytes).unwrap();
            let ack = Envelope {
                protocol_version: 1,
                message_id: request.message_id + 1000,
                in_reply_to: request.message_id,
                payload: Some(Payload::Ack(Ack {
                    status: Some(Status {
                        code: ErrorCode::Ok as i32,
                        message: String::new(),
                    }),
                })),
            };
            ws.send(Message::Binary(EnvelopeCodec::encode(&ack).unwrap()))
                .await
                .unwrap();
        }
    });

    (port, spki)
}

fn welcome_ok() -> SessionWelcome {
    SessionWelcome {
        status: Some(Status {
            code: ErrorCode::Ok as i32,
            message: String::new(),
        }),
        protocol_version: 1,
        phone_device_id: "phone-1".into(),
        phone_name: "Pixel".into(),
        epoch_id: "epoch-1".into(),
        state_seq: 7,
        call_log_version: 3,
        emergency_numbers: vec!["911".into(), "112".into()],
    }
}

async fn connect_pinning(
    port: u16,
    pin_spki: &[u8],
    next_id: u64,
) -> Result<WsTransportClient, TransportError> {
    let desktop = issue("tandem-desktop");
    let pin = PinSource::Paired(SpkiFingerprint::from_spki_der(pin_spki));
    let config = client_config(&pin, vec![desktop.cert_der], desktop.key_der).unwrap();

    WsTransportClient::connect(
        "127.0.0.1",
        port,
        config,
        ClientIdentity {
            device_id: "desktop-1".into(),
            client_name: "Test Desktop".into(),
            bt_adapter_address: String::new(),
        },
        next_id,
    )
    .await
}

#[tokio::test]
async fn a_pinned_session_completes_the_handshake() {
    let (port, spki) = spawn_phone(welcome_ok()).await;
    let client = connect_pinning(port, &spki, 1).await.unwrap();

    let session = client.session();
    assert_eq!(session.phone_device_id, "phone-1");
    assert_eq!(session.phone_name, "Pixel");
    assert_eq!(session.epoch_id, "epoch-1");
    assert_eq!(session.state_seq, 7);
    assert_eq!(session.call_log_version, 3);
}

/// The emergency list must survive the handshake: the desktop's local pre-check
/// is armed from it (ADR-0008).
#[tokio::test]
async fn the_session_carries_the_emergency_number_list() {
    let (port, spki) = spawn_phone(welcome_ok()).await;
    let client = connect_pinning(port, &spki, 1).await.unwrap();

    assert_eq!(client.session().emergency_numbers, vec!["911", "112"]);
}

/// The whole trust model: a phone presenting an unpinned key must be refused
/// before any protocol traffic flows.
#[tokio::test]
async fn a_wrong_pin_aborts_before_the_session_opens() {
    let (port, _real_spki) = spawn_phone(welcome_ok()).await;
    let impostor = issue("attacker");

    let result = connect_pinning(port, &impostor.spki, 1).await;

    match result {
        Err(TransportError::TlsHandshake(_)) => {}
        Err(other) => panic!("expected a TLS handshake failure, got {other:?}"),
        Ok(_) => panic!("an unpinned key must never establish a session"),
    }
}

#[tokio::test]
async fn a_rejected_version_surfaces_as_version_unsupported() {
    let mut welcome = welcome_ok();
    welcome.status = Some(Status {
        code: ErrorCode::VersionUnsupported as i32,
        message: "unsupported".into(),
    });

    let (port, spki) = spawn_phone(welcome).await;
    let result = connect_pinning(port, &spki, 1).await;

    assert!(matches!(
        result,
        Err(TransportError::VersionUnsupported { requested: 1 })
    ));
}

#[tokio::test]
async fn requests_are_correlated_to_their_replies() {
    let (port, spki) = spawn_phone(welcome_ok()).await;
    let mut client = connect_pinning(port, &spki, 1).await.unwrap();

    let reply = client
        .request(Payload::MuteRequest(tandem_proto::MuteRequest {
            muted: true,
        }))
        .await
        .unwrap();

    match reply {
        Payload::Ack(ack) => assert_eq!(ack.status.unwrap().code, ErrorCode::Ok as i32),
        other => panic!("unexpected reply: {other:?}"),
    }
}

/// Ids resume from the persisted counter rather than restarting, which is what
/// makes cross-reconnect dedup possible (docs/06).
#[tokio::test]
async fn message_ids_resume_from_the_persisted_counter() {
    let (port, spki) = spawn_phone(welcome_ok()).await;
    let client = connect_pinning(port, &spki, 500).await.unwrap();

    // The handshake consumed id 500, so the next allocation continues from 501.
    assert_eq!(client.next_message_id(), 501);
}
