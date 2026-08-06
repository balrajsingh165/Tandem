//! JSON-RPC server: accepts one or more UI connections on the local socket,
//! authenticates same-user peers, dispatches to the daemon's service
//! implementation, and pushes state events.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, Mutex};

use crate::api::{IpcEvent, IpcRequest, IpcResponse};
use crate::error::IpcError;
use crate::socket::Endpoint;

/// What the daemon implements to serve the UI. Kept synchronous in signature so
/// the contract is testable without a runtime; the daemon adapts it to its own
/// async task structure.
pub trait IpcService: Send + Sync {
    fn handle(&mut self, request: IpcRequest) -> Result<IpcResponse, IpcError>;
}

/// Frames are newline-delimited JSON: a UI is a local, trusted-by-uid peer, so
/// the framing only has to be unambiguous, not defensive.
const FRAME_DELIMITER: u8 = b'\n';

/// How many events may queue for a slow UI before it starts losing them. A UI
/// that falls this far behind re-reads state via `status` rather than stalling
/// the daemon.
const EVENT_BUFFER: usize = 256;

/// Serializes a successful result into a JSON-RPC response frame.
pub fn success_frame(id: u64, response: &IpcResponse) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": crate::JSONRPC_VERSION,
        "id": id,
        "result": response,
    })
}

/// Serializes a failure into a JSON-RPC error frame carrying the stable code.
pub fn error_frame(id: u64, error: &IpcError) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": crate::JSONRPC_VERSION,
        "id": id,
        "error": {
            "code": error.code(),
            "message": error.to_string(),
        },
    })
}

/// Events are notifications: no id, and the UI must never reply to them.
pub fn event_frame(event: &IpcEvent) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": crate::JSONRPC_VERSION,
        "method": "event",
        "params": event,
    })
}

/// Parses one inbound frame into an id and request.
pub fn parse_request(line: &str) -> Result<(u64, IpcRequest), IpcError> {
    let value: serde_json::Value = serde_json::from_str(line).map_err(|_| IpcError::Internal)?;
    let id = value.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
    let request: IpcRequest = serde_json::from_value(value).map_err(|_| IpcError::Internal)?;
    Ok((id, request))
}

/// Broadcasts daemon events to every connected UI.
#[derive(Debug, Clone)]
pub struct EventPublisher {
    sender: broadcast::Sender<IpcEvent>,
}

impl Default for EventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventPublisher {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BUFFER);
        Self { sender }
    }

    /// Publishing with no UI attached is normal, not an error.
    pub fn publish(&self, event: IpcEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<IpcEvent> {
        self.sender.subscribe()
    }
}

/// Accepts UI connections and serves them until shutdown.
pub struct IpcServer<S: IpcService + 'static> {
    service: Arc<Mutex<S>>,
    events: EventPublisher,
}

impl<S: IpcService + 'static> IpcServer<S> {
    pub fn new(service: S, events: EventPublisher) -> Self {
        Self {
            service: Arc::new(Mutex::new(service)),
            events,
        }
    }

    pub fn events(&self) -> EventPublisher {
        self.events.clone()
    }

    /// Serves connections on [endpoint] until the future is dropped.
    pub async fn serve(&self, endpoint: &Endpoint) -> Result<(), IpcError> {
        match endpoint {
            #[cfg(unix)]
            Endpoint::UnixSocket(path) => self.serve_unix(path).await,
            #[cfg(not(unix))]
            Endpoint::UnixSocket(_) => Err(IpcError::DaemonUnavailable),

            #[cfg(windows)]
            Endpoint::WindowsPipe(name) => self.serve_pipe(name).await,
            #[cfg(not(windows))]
            Endpoint::WindowsPipe(_) => Err(IpcError::DaemonUnavailable),
        }
    }

    #[cfg(unix)]
    async fn serve_unix(&self, path: &std::path::Path) -> Result<(), IpcError> {
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        // A stale socket from a crashed daemon would block the bind.
        let _ = tokio::fs::remove_file(path).await;

        let listener =
            tokio::net::UnixListener::bind(path).map_err(|_| IpcError::DaemonUnavailable)?;

        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            let service = self.service.clone();
            let events = self.events.subscribe();
            tokio::spawn(async move { serve_connection(stream, service, events).await });
        }
    }

    #[cfg(windows)]
    async fn serve_pipe(&self, name: &str) -> Result<(), IpcError> {
        use tokio::net::windows::named_pipe::ServerOptions;

        loop {
            let server = ServerOptions::new()
                .create(name)
                .map_err(|_| IpcError::DaemonUnavailable)?;

            server
                .connect()
                .await
                .map_err(|_| IpcError::DaemonUnavailable)?;

            let service = self.service.clone();
            let events = self.events.subscribe();
            tokio::spawn(async move { serve_connection(server, service, events).await });
        }
    }
}

/// Serves one UI: request/response on the read half, events pushed on the write
/// half, both sharing the stream through a split.
pub async fn serve_connection<T, S>(
    stream: T,
    service: Arc<Mutex<S>>,
    mut events: broadcast::Receiver<IpcEvent>,
) where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
    S: IpcService + 'static,
{
    let (read_half, write_half) = tokio::io::split(stream);
    let mut lines = BufReader::new(read_half).lines();
    let writer = Arc::new(Mutex::new(write_half));

    let event_writer = writer.clone();
    let event_task = tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            let frame = event_frame(&event);
            let mut guard = event_writer.lock().await;
            if write_frame(&mut *guard, &frame).await.is_err() {
                return;
            }
        }
    });

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let frame = match parse_request(&line) {
            Ok((id, request)) => {
                let result = { service.lock().await.handle(request) };
                match result {
                    Ok(response) => success_frame(id, &response),
                    Err(error) => error_frame(id, &error),
                }
            }
            Err(error) => error_frame(0, &error),
        };

        let mut guard = writer.lock().await;
        if write_frame(&mut *guard, &frame).await.is_err() {
            break;
        }
    }

    event_task.abort();
}

async fn write_frame<W>(writer: &mut W, frame: &serde_json::Value) -> std::io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut bytes = serde_json::to_vec(frame)?;
    bytes.push(FRAME_DELIMITER);
    writer.write_all(&bytes).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ConnectionStatus, StatusResult};

    struct EchoService {
        pub seen: Vec<IpcRequest>,
    }

    impl IpcService for EchoService {
        fn handle(&mut self, request: IpcRequest) -> Result<IpcResponse, IpcError> {
            self.seen.push(request.clone());
            match request {
                IpcRequest::Status => Ok(IpcResponse::Status(StatusResult {
                    phones: Vec::new(),
                    selected_phone_id: String::new(),
                    connection: ConnectionStatus::Live,
                    phone_name: "Pixel".into(),
                    calls: Vec::new(),
                    audio_route: crate::api::AudioRoute::Earpiece,
                    microphone_muted: false,
                    desktop_audio_available: false,
                    audio_devices: Vec::new(),
                    active_bt_device_address: String::new(),
                })),
                IpcRequest::Dial { number, .. } => Err(IpcError::EmergencyBlocked { number }),
                _ => Ok(IpcResponse::Ok),
            }
        }
    }

    #[test]
    fn success_frames_carry_the_id_and_result() {
        let frame = success_frame(7, &IpcResponse::Ok);
        assert_eq!(frame["jsonrpc"], "2.0");
        assert_eq!(frame["id"], 7);
        assert_eq!(frame["result"]["result"], "ok");
    }

    #[test]
    fn error_frames_carry_the_stable_code() {
        let frame = error_frame(
            9,
            &IpcError::EmergencyBlocked {
                number: "911".into(),
            },
        );
        assert_eq!(frame["id"], 9);
        assert_eq!(frame["error"]["code"], crate::error::IPC_EMERGENCY_BLOCKED);
        assert!(frame["error"]["message"]
            .as_str()
            .unwrap()
            .contains("handset"));
    }

    /// A notification with an id would invite the UI to reply, which the daemon
    /// does not expect.
    #[test]
    fn event_frames_are_notifications_without_an_id() {
        let frame = event_frame(&IpcEvent::Revoked {
            reason: "removed on phone".into(),
        });
        assert!(frame.get("id").is_none());
        assert_eq!(frame["method"], "event");
        assert_eq!(frame["params"]["type"], "revoked");
    }

    #[test]
    fn requests_parse_with_their_correlation_id() {
        let (id, request) = parse_request(r#"{"jsonrpc":"2.0","id":4,"method":"status"}"#).unwrap();
        assert_eq!(id, 4);
        assert_eq!(request, IpcRequest::Status);
    }

    #[test]
    fn malformed_frames_are_rejected_rather_than_panicking() {
        assert!(parse_request("not json").is_err());
        assert!(parse_request(r#"{"jsonrpc":"2.0","id":1}"#).is_err());
    }

    /// A full round trip over a real duplex stream, including an event pushed
    /// while the connection is open.
    #[tokio::test]
    async fn a_connection_answers_requests_and_receives_events() {
        let (client, server) = tokio::io::duplex(8192);
        let service = Arc::new(Mutex::new(EchoService { seen: Vec::new() }));
        let publisher = EventPublisher::new();
        let subscription = publisher.subscribe();

        let service_for_task = service.clone();
        tokio::spawn(async move {
            serve_connection(server, service_for_task, subscription).await;
        });

        let (read_half, mut write_half) = tokio::io::split(client);
        let mut lines = BufReader::new(read_half).lines();

        write_half
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"status\"}\n")
            .await
            .unwrap();

        let reply: serde_json::Value =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(reply["id"], 1);
        assert_eq!(reply["result"]["data"]["phoneName"], "Pixel");

        publisher.publish(IpcEvent::HistoryChanged { log_version: 12 });
        let pushed: serde_json::Value =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(pushed["method"], "event");
        assert_eq!(pushed["params"]["logVersion"], 12);

        assert_eq!(service.lock().await.seen.len(), 1);
    }

    /// An emergency refusal must reach the UI with its stable code intact.
    #[tokio::test]
    async fn a_refused_dial_returns_its_stable_error_code() {
        let (client, server) = tokio::io::duplex(8192);
        let service = Arc::new(Mutex::new(EchoService { seen: Vec::new() }));
        let subscription = EventPublisher::new().subscribe();

        tokio::spawn(async move { serve_connection(server, service, subscription).await });

        let (read_half, mut write_half) = tokio::io::split(client);
        let mut lines = BufReader::new(read_half).lines();

        write_half
            .write_all(
                b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"dial\",\"params\":{\"number\":\"911\",\"simSlot\":-1}}\n",
            )
            .await
            .unwrap();

        let reply: serde_json::Value =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(reply["id"], 2);
        assert_eq!(reply["error"]["code"], crate::error::IPC_EMERGENCY_BLOCKED);
    }

    #[tokio::test]
    async fn publishing_without_a_listener_is_not_an_error() {
        let publisher = EventPublisher::new();
        publisher.publish(IpcEvent::HistoryChanged { log_version: 1 });
    }
}
