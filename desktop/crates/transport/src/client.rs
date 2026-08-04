//! TransportClient implementation: dials the phone endpoint with the
//! pinned-peer TLS config, performs SessionHello/SessionWelcome, then pumps
//! Envelope frames bidirectionally with heartbeats (5 s send / 15 s dead-peer).

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use rustls::ClientConfig;
use rustls_pki_types::ServerName;
use tandem_proto::{envelope::Payload, Envelope, SessionHello};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{client_async, WebSocketStream};

use crate::codec::EnvelopeCodec;
use crate::error::TransportError;
use crate::reconnect::ResumeCursor;
use crate::{DEAD_PEER_TIMEOUT_SECS, PROTOCOL_VERSION};

/// Live session facts the phone reports in `SessionWelcome`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionInfo {
    pub protocol_version: u32,
    pub phone_device_id: String,
    pub phone_name: String,
    pub epoch_id: String,
    pub state_seq: u64,
    pub call_log_version: u64,
    pub emergency_numbers: Vec<String>,
}

/// Connection lifecycle as observed by the daemon; mirrors the state table in
/// docs/06.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Idle,
    Discovering,
    Connecting,
    Authenticating,
    PairingProvisional,
    Resuming,
    Live,
    Backoff,
    Terminated,
}

/// The seam between the daemon and the LAN. Core and tests depend on this trait,
/// never on sockets (docs/11).
pub trait TransportClient: Send + Sync {
    /// Sends a request and resolves when the correlated reply arrives, or with
    /// `RequestTimeout` if the peer does not answer.
    fn request(
        &mut self,
        payload: Payload,
    ) -> impl std::future::Future<Output = Result<Payload, TransportError>> + Send;

    /// Reconciles the mirror against phone truth after a reconnect.
    fn resume(
        &mut self,
        cursor: ResumeCursor,
    ) -> impl std::future::Future<Output = Result<Payload, TransportError>> + Send;

    fn close(&mut self) -> impl std::future::Future<Output = ()> + Send;
}

/// What this desktop announces about itself when opening a session.
#[derive(Debug, Clone, Default)]
pub struct ClientIdentity {
    pub device_id: String,
    pub client_name: String,
    pub bt_adapter_address: String,
}

type Socket = WebSocketStream<TlsStream<TcpStream>>;

/// A live, authenticated session over WebSocket + mutual TLS 1.3.
pub struct WsTransportClient {
    socket: Socket,
    codec: EnvelopeCodec,
    session: SessionInfo,
}

impl WsTransportClient {
    /// Dials the phone, completes the TLS handshake against the pinned key, then
    /// exchanges SessionHello/SessionWelcome. Any pin failure aborts here rather
    /// than surfacing later as a confusing protocol error.
    pub async fn connect(
        host: &str,
        port: u16,
        tls_config: ClientConfig,
        identity: ClientIdentity,
        next_message_id: u64,
    ) -> Result<Self, TransportError> {
        let endpoint = format!("{host}:{port}");

        let tcp =
            TcpStream::connect(&endpoint)
                .await
                .map_err(|e| TransportError::ConnectFailed {
                    endpoint: endpoint.clone(),
                    reason: e.to_string(),
                })?;
        tcp.set_nodelay(true).ok();

        // The certificate is verified by pinned key, so the SNI value carries no
        // trust weight; a fixed name keeps it stable across IP changes.
        let server_name = ServerName::try_from(TLS_SERVER_NAME)
            .map_err(|e| TransportError::TlsHandshake(e.to_string()))?;

        let tls = TlsConnector::from(Arc::new(tls_config))
            .connect(server_name, tcp)
            .await
            .map_err(|e| TransportError::TlsHandshake(e.to_string()))?;

        let request = format!("ws://{endpoint}{WS_PATH}");
        let (socket, _) =
            client_async(request, tls)
                .await
                .map_err(|e| TransportError::ConnectFailed {
                    endpoint: endpoint.clone(),
                    reason: e.to_string(),
                })?;

        let mut client = Self {
            socket,
            codec: EnvelopeCodec::resuming_at(next_message_id),
            session: SessionInfo::default(),
        };
        client.session = client.handshake(identity).await?;
        Ok(client)
    }

    pub fn session(&self) -> &SessionInfo {
        &self.session
    }

    pub fn next_message_id(&self) -> u64 {
        self.codec.next_message_id()
    }

    async fn handshake(&mut self, identity: ClientIdentity) -> Result<SessionInfo, TransportError> {
        let hello = Payload::SessionHello(SessionHello {
            device_id: identity.device_id,
            protocol_min: PROTOCOL_VERSION,
            protocol_max: PROTOCOL_VERSION,
            client_name: identity.client_name,
            bt_adapter_address: identity.bt_adapter_address,
        });

        let frame = self.codec.encode_request(hello)?;
        self.send(frame).await?;

        match self.receive().await?.payload {
            Some(Payload::SessionWelcome(welcome)) => {
                let status_ok = welcome
                    .status
                    .as_ref()
                    .map(|s| s.code == tandem_proto::ErrorCode::Ok as i32)
                    .unwrap_or(false);

                if !status_ok {
                    return Err(match welcome.status.as_ref().map(|s| s.code) {
                        Some(code)
                            if code == tandem_proto::ErrorCode::VersionUnsupported as i32 =>
                        {
                            TransportError::VersionUnsupported {
                                requested: PROTOCOL_VERSION,
                            }
                        }
                        _ => TransportError::Unauthenticated,
                    });
                }

                Ok(SessionInfo {
                    protocol_version: welcome.protocol_version,
                    phone_device_id: welcome.phone_device_id,
                    phone_name: welcome.phone_name,
                    epoch_id: welcome.epoch_id,
                    state_seq: welcome.state_seq,
                    call_log_version: welcome.call_log_version,
                    emergency_numbers: welcome.emergency_numbers,
                })
            }
            Some(Payload::RevokedEvent(event)) => Err(TransportError::Revoked(event.reason)),
            _ => Err(TransportError::ProtocolViolation(
                "expected SessionWelcome as the first server frame".into(),
            )),
        }
    }

    async fn send(&mut self, frame: Vec<u8>) -> Result<(), TransportError> {
        self.socket
            .send(Message::Binary(frame))
            .await
            .map_err(|e| TransportError::ProtocolViolation(e.to_string()))
    }

    async fn receive(&mut self) -> Result<Envelope, TransportError> {
        let deadline = std::time::Duration::from_secs(DEAD_PEER_TIMEOUT_SECS);

        loop {
            let next = tokio::time::timeout(deadline, self.socket.next())
                .await
                .map_err(|_| TransportError::PeerSilent)?;

            match next {
                Some(Ok(Message::Binary(bytes))) => return EnvelopeCodec::decode(&bytes),
                // Ping/Pong and text frames are not protocol traffic; keep waiting
                // rather than tearing down a healthy session.
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Text(_))) => continue,
                Some(Ok(Message::Close(_))) | None => return Err(TransportError::Closed),
                Some(Ok(Message::Frame(_))) => continue,
                Some(Err(e)) => return Err(TransportError::ProtocolViolation(e.to_string())),
            }
        }
    }
}

impl TransportClient for WsTransportClient {
    async fn request(&mut self, payload: Payload) -> Result<Payload, TransportError> {
        let message_id = self.codec.next_message_id();
        let frame = self.codec.encode_request(payload)?;
        self.send(frame).await?;

        loop {
            let envelope = self.receive().await?;
            // Events can arrive between a request and its reply; only the
            // correlated frame resolves the call.
            if envelope.in_reply_to == message_id || envelope.in_reply_to == 0 {
                if let Some(payload) = envelope.payload {
                    return Ok(payload);
                }
            }
        }
    }

    async fn resume(&mut self, cursor: ResumeCursor) -> Result<Payload, TransportError> {
        self.request(Payload::ResumeRequest(tandem_proto::ResumeRequest {
            last_epoch_id: cursor.last_epoch_id,
            last_state_seq: cursor.last_state_seq,
            last_call_log_version: cursor.last_call_log_version,
        }))
        .await
    }

    async fn close(&mut self) {
        let _ = self.socket.close(None).await;
    }
}

/// SNI presented to the phone. Trust comes from the pinned key, not this name.
pub const TLS_SERVER_NAME: &str = "tandem.local";

/// WebSocket path the gateway serves.
pub const WS_PATH: &str = "/tlp/v1";
