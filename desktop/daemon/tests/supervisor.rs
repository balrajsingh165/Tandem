//! End-to-end supervisor tests: a real TLS phone drives the daemon's session
//! loop, proving the mirror tracks phone truth, that the emergency list arms the
//! local pre-check, and that a revoked desktop stops retrying.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, ServerConfig, SignatureScheme};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, UnixTime};
use tandem_crypto::{InMemorySecretStore, SpkiFingerprint};
use tandem_ipc::api::ConnectionStatus;
use tandem_ipc::server::EventPublisher;
use tandem_proto::{
    envelope::Payload, CallInfo, CallSnapshot, CallStateChangedEvent, Envelope, ErrorCode,
    ResumeResponse, RevokedEvent, SessionWelcome, Status,
};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::Message;

#[path = "../src/app.rs"]
mod app;
#[path = "../src/config.rs"]
mod config;
#[path = "../src/ipc_service.rs"]
mod ipc_service;
#[path = "../src/logging.rs"]
mod logging;
#[path = "../src/session_loop.rs"]
mod session_loop;
#[path = "../src/store.rs"]
mod store;

use app::App;
use config::Config;
use ipc_service::{LinkState, SharedApp, SharedLink};
use session_loop::PhoneEndpoint;

#[derive(Debug)]
struct AcceptAnyClient;

impl ClientCertVerifier for AcceptAnyClient {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }
    fn verify_client_cert(
        &self,
        _e: &CertificateDer<'_>,
        _i: &[CertificateDer<'_>],
        _n: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            m,
            c,
            d,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            m,
            c,
            d,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// What the scripted phone does after the resume exchange.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AfterResume {
    PushRingingCall,
    Revoke,
}

async fn spawn_phone(after: AfterResume) -> (u16, Vec<u8>, Arc<AtomicUsize>) {
    let generated = rcgen::generate_simple_self_signed(vec!["tandem.local".into()]).unwrap();
    let cert_der = generated.cert.der().to_vec();
    let spki = tandem_transport::tls::spki_from_certificate(&cert_der).unwrap();
    let connections = Arc::new(AtomicUsize::new(0));
    let counter = connections.clone();

    let server_config = ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(AcceptAnyClient))
        .with_single_cert(
            vec![CertificateDer::from(cert_der)],
            PrivateKeyDer::try_from(generated.key_pair.serialize_der()).unwrap(),
        )
        .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            let Ok(tls) = acceptor.accept(tcp).await else {
                continue;
            };
            let Ok(mut ws) = tokio_tungstenite::accept_async(tls).await else {
                continue;
            };

            // SessionHello -> SessionWelcome
            let Some(Ok(Message::Binary(bytes))) = ws.next().await else {
                continue;
            };
            let hello = tandem_transport::codec::EnvelopeCodec::decode(&bytes).unwrap();
            let welcome = Envelope {
                protocol_version: 1,
                message_id: 1,
                in_reply_to: hello.message_id,
                payload: Some(Payload::SessionWelcome(SessionWelcome {
                    status: Some(Status {
                        code: ErrorCode::Ok as i32,
                        message: String::new(),
                    }),
                    protocol_version: 1,
                    phone_device_id: "phone-1".into(),
                    phone_name: "Pixel".into(),
                    epoch_id: "epoch-1".into(),
                    state_seq: 1,
                    call_log_version: 0,
                    emergency_numbers: vec!["110".into()],
                })),
            };
            send(&mut ws, welcome).await;

            // ResumeRequest -> ResumeResponse with a snapshot.
            let Some(Ok(Message::Binary(bytes))) = ws.next().await else {
                continue;
            };
            let resume = tandem_transport::codec::EnvelopeCodec::decode(&bytes).unwrap();
            let response = Envelope {
                protocol_version: 1,
                message_id: 2,
                in_reply_to: resume.message_id,
                payload: Some(Payload::ResumeResponse(ResumeResponse {
                    status: None,
                    snapshot_included: true,
                    snapshot: Some(CallSnapshot {
                        epoch_id: "epoch-1".into(),
                        state_seq: 1,
                        calls: Vec::new(),
                        audio_route: tandem_proto::AudioRoute::Earpiece as i32,
                        microphone_muted: false,
                        bt_route_address: String::new(),
                    }),
                    call_log_version: 0,
                })),
            };
            send(&mut ws, response).await;

            match after {
                AfterResume::PushRingingCall => {
                    let event = Envelope {
                        protocol_version: 1,
                        message_id: 3,
                        in_reply_to: 0,
                        payload: Some(Payload::CallStateChangedEvent(CallStateChangedEvent {
                            snapshot: Some(CallSnapshot {
                                epoch_id: "epoch-1".into(),
                                state_seq: 2,
                                calls: vec![CallInfo {
                                    call_id: "c1".into(),
                                    state: tandem_proto::CallState::Ringing as i32,
                                    remote_number: "+14155550123".into(),
                                    ..Default::default()
                                }],
                                audio_route: tandem_proto::AudioRoute::Earpiece as i32,
                                microphone_muted: false,
                                bt_route_address: String::new(),
                            }),
                        })),
                    };
                    send(&mut ws, event).await;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                AfterResume::Revoke => {
                    let event = Envelope {
                        protocol_version: 1,
                        message_id: 3,
                        in_reply_to: 0,
                        payload: Some(Payload::RevokedEvent(RevokedEvent {
                            reason: "removed on the phone".into(),
                        })),
                    };
                    send(&mut ws, event).await;
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        }
    });

    (port, spki, connections)
}

async fn send<S>(ws: &mut tokio_tungstenite::WebSocketStream<S>, envelope: Envelope)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let bytes = tandem_transport::codec::EnvelopeCodec::encode(&envelope).unwrap();
    ws.send(Message::Binary(bytes)).await.unwrap();
}

fn harness(
    port: u16,
    spki: &[u8],
) -> (
    PhoneEndpoint,
    tandem_crypto::IdentityCredentials,
    SharedApp,
    SharedLink,
) {
    let credentials =
        tandem_crypto::identity::load_or_create(&InMemorySecretStore::default(), "Desk").unwrap();

    let app = App::build(Config {
        bluetooth_backend: tandem_bluetooth::backends::BackendKind::Null,
        ..Config::default()
    });

    (
        PhoneEndpoint {
            // A fixed host bypasses discovery, keeping the test to a real socket.
            device_id: "phone-under-test".into(),
            host: "127.0.0.1".into(),
            port,
            pin: tandem_transport::tls::PinSource::Paired(SpkiFingerprint::from_spki_der(spki)),
        },
        credentials,
        Arc::new(Mutex::new(app)),
        Arc::new(Mutex::new(LinkState::default())),
    )
}

/// The core of ADR-0007: the desktop mirror follows phone truth.
#[tokio::test]
async fn the_mirror_tracks_phone_truth() {
    let (port, spki, _) = spawn_phone(AfterResume::PushRingingCall).await;
    let (endpoint, credentials, app, link) = harness(port, &spki);
    let events = EventPublisher::new();

    let session = tokio::spawn({
        let app = app.clone();
        let link = link.clone();
        async move {
            let (_bus, mut commands) = session_loop::CommandBus::new();
            let state = std::env::temp_dir().join("tandem-test-state.json");
            session_loop::run_one_session(
                &endpoint,
                &credentials,
                &app,
                &link,
                &events,
                &mut commands,
                &state,
            )
            .await
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    let mut guard = app.lock().unwrap();
    let mirror = guard
        .controller()
        .mirror()
        .cloned()
        .expect("mirror present");
    assert_eq!(mirror.version.epoch_id, "epoch-1");
    assert_eq!(mirror.calls.len(), 1);
    assert_eq!(mirror.calls[0].call_id, "c1");
    assert_eq!(
        mirror.calls[0].state,
        tandem_core::model::CallState::Ringing
    );

    session.abort();
}

/// The session must reach Live, and report the phone's name for the UI.
#[tokio::test]
async fn the_link_reaches_live_and_names_the_phone() {
    let (port, spki, _) = spawn_phone(AfterResume::PushRingingCall).await;
    let (endpoint, credentials, app, link) = harness(port, &spki);
    let events = EventPublisher::new();

    let session = tokio::spawn({
        let app = app.clone();
        let link = link.clone();
        async move {
            let (_bus, mut commands) = session_loop::CommandBus::new();
            let state = std::env::temp_dir().join("tandem-test-state.json");
            session_loop::run_one_session(
                &endpoint,
                &credentials,
                &app,
                &link,
                &events,
                &mut commands,
                &state,
            )
            .await
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    let state = link.lock().unwrap().clone();
    assert_eq!(state.connection, Some(ConnectionStatus::Live));
    assert_eq!(state.phone_name, "Pixel");

    session.abort();
}

/// The per-session emergency list must arm the local pre-check, so a regional
/// number the phone reports is refused before any frame is sent (ADR-0008).
#[tokio::test]
async fn the_session_emergency_list_arms_the_local_pre_check() {
    let (port, spki, _) = spawn_phone(AfterResume::PushRingingCall).await;
    let (endpoint, credentials, app, link) = harness(port, &spki);
    let events = EventPublisher::new();

    let session = tokio::spawn({
        let app = app.clone();
        let link = link.clone();
        async move {
            let (_bus, mut commands) = session_loop::CommandBus::new();
            let state = std::env::temp_dir().join("tandem-test-state.json");
            session_loop::run_one_session(
                &endpoint,
                &credentials,
                &app,
                &link,
                &events,
                &mut commands,
                &state,
            )
            .await
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    let refused = app.lock().unwrap().controller().apply_user_command(
        tandem_core::events::UserCommand::Dial {
            number: "110".into(),
            sim_slot: -1,
        },
    );
    assert!(refused.is_err(), "the phone's list must be in force");

    session.abort();
}

/// Revocation is terminal: the supervisor must stop rather than reconnect, or a
/// removed desktop would hammer the phone forever.
#[tokio::test]
async fn revocation_stops_the_supervisor() {
    let (port, spki, connections) = spawn_phone(AfterResume::Revoke).await;
    let (endpoint, credentials, app, link) = harness(port, &spki);
    let events = EventPublisher::new();

    let (_bus, commands) = session_loop::CommandBus::new();
    let supervisor = tokio::spawn(session_loop::supervise(
        endpoint,
        credentials,
        app,
        link.clone(),
        events,
        commands,
        std::env::temp_dir().join("tandem-test-state.json"),
    ));

    tokio::time::timeout(std::time::Duration::from_secs(5), supervisor)
        .await
        .expect("supervisor must return after revocation")
        .unwrap();

    assert_eq!(
        link.lock().unwrap().connection,
        Some(ConnectionStatus::Terminated)
    );
    assert_eq!(
        connections.load(Ordering::SeqCst),
        1,
        "a revoked desktop must not reconnect"
    );
}
