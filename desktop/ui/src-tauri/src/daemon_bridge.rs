//! Bridges the webview and the daemon socket: forwards JSON-RPC requests from
//! the front-end via Tauri commands, streams daemon events to the webview, and
//! manages daemon liveness (spawn/reconnect prompts).

use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use tandem_ipc::error::{IpcError, IPC_DAEMON_UNAVAILABLE};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Event channel name the front-end subscribes to.
pub const EVENT_CHANNEL: &str = "tandem://event";

/// Correlates requests on a connection. Each call opens its own connection, so
/// this only has to be unique per call rather than globally ordered.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Error shape the webview receives. `code` is the stable JSON-RPC code from
/// tandem_ipc, so the UI branches on cause rather than message text.
#[derive(Debug, Clone, Serialize)]
pub struct BridgeError {
    pub code: i32,
    pub message: String,
}

impl From<IpcError> for BridgeError {
    fn from(error: IpcError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
        }
    }
}

impl BridgeError {
    pub fn daemon_unavailable() -> Self {
        Self {
            code: IPC_DAEMON_UNAVAILABLE,
            message: IpcError::DaemonUnavailable.to_string(),
        }
    }

    fn protocol(detail: impl std::fmt::Display) -> Self {
        Self {
            code: tandem_ipc::error::IPC_INTERNAL,
            message: detail.to_string(),
        }
    }
}

/// Forwards one request to the daemon. The shell performs no policy of its own:
/// every decision, including the emergency refusal, belongs to the daemon.
#[tauri::command]
pub async fn daemon_request(
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, BridgeError> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    let mut frame = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
    });
    // Unit-variant methods carry no params; sending an empty object would fail
    // the tagged-enum decode on the daemon side.
    if !is_empty_params(&params) {
        frame["params"] = params;
    }

    let line = serde_json::to_string(&frame).map_err(BridgeError::protocol)?;
    let reply = round_trip(&line).await?;

    let value: serde_json::Value = serde_json::from_str(&reply).map_err(BridgeError::protocol)?;

    if let Some(error) = value.get("error") {
        return Err(BridgeError {
            code: error.get("code").and_then(|c| c.as_i64()).unwrap_or(-32099) as i32,
            message: error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("daemon request failed")
                .to_string(),
        });
    }

    Ok(unwrap_result(value.get("result")))
}

/// `IpcResponse` is a tagged enum, so it arrives as `{"result":"status",
/// "data":{…}}`. The webview wants the payload itself; variants without a body
/// (a plain Ok) yield null.
fn unwrap_result(result: Option<&serde_json::Value>) -> serde_json::Value {
    match result {
        Some(serde_json::Value::Object(map)) => map
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        Some(other) => other.clone(),
        None => serde_json::Value::Null,
    }
}

fn is_empty_params(params: &serde_json::Value) -> bool {
    match params {
        serde_json::Value::Null => true,
        serde_json::Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

/// Opens a connection, sends one line, and reads the correlated reply. Events
/// interleaved on the same connection are skipped rather than mistaken for the
/// response.
#[cfg(windows)]
async fn round_trip(line: &str) -> Result<String, BridgeError> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let pipe = ClientOptions::new()
        .open(r"\\.\pipe\tandem-daemon")
        .map_err(|_| BridgeError::daemon_unavailable())?;

    exchange(pipe, line).await
}

#[cfg(unix)]
async fn round_trip(line: &str) -> Result<String, BridgeError> {
    let path = std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("tandem/daemon.sock");

    let stream = tokio::net::UnixStream::connect(path)
        .await
        .map_err(|_| BridgeError::daemon_unavailable())?;

    exchange(stream, line).await
}

async fn exchange<S>(stream: S, line: &str) -> Result<String, BridgeError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (read_half, mut write_half) = tokio::io::split(stream);

    write_half
        .write_all(format!("{line}\n").as_bytes())
        .await
        .map_err(|_| BridgeError::daemon_unavailable())?;
    write_half
        .flush()
        .await
        .map_err(|_| BridgeError::daemon_unavailable())?;

    let mut lines = BufReader::new(read_half).lines();
    let deadline = std::time::Duration::from_secs(5);

    loop {
        let next = tokio::time::timeout(deadline, lines.next_line())
            .await
            .map_err(|_| BridgeError::from(IpcError::Timeout))?
            .map_err(|_| BridgeError::daemon_unavailable())?;

        let Some(candidate) = next else {
            return Err(BridgeError::daemon_unavailable());
        };

        // Notifications have no id; only a correlated frame is the answer.
        let parsed: serde_json::Value = match serde_json::from_str(&candidate) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if parsed.get("id").is_some() {
            return Ok(candidate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_errors_keep_their_stable_code_across_the_bridge() {
        let bridged: BridgeError = IpcError::EmergencyBlocked {
            number: "911".into(),
        }
        .into();
        assert_eq!(bridged.code, tandem_ipc::error::IPC_EMERGENCY_BLOCKED);
        assert!(bridged.message.contains("handset"));
    }

    #[test]
    fn an_absent_daemon_is_reported_with_its_own_code() {
        assert_eq!(
            BridgeError::daemon_unavailable().code,
            IPC_DAEMON_UNAVAILABLE
        );
    }

    #[test]
    fn a_tagged_result_is_unwrapped_to_its_payload() {
        let framed = serde_json::json!({ "result": "status", "data": { "phoneName": "Pixel" } });
        assert_eq!(unwrap_result(Some(&framed))["phoneName"], "Pixel");
    }

    /// A body-less Ok must not look like a malformed payload to the webview.
    #[test]
    fn a_bodyless_ok_unwraps_to_null() {
        let framed = serde_json::json!({ "result": "ok" });
        assert!(unwrap_result(Some(&framed)).is_null());
        assert!(unwrap_result(None).is_null());
    }

    /// Unit-variant methods must not carry a params object.
    #[test]
    fn empty_params_are_recognized() {
        assert!(is_empty_params(&serde_json::Value::Null));
        assert!(is_empty_params(&serde_json::json!({})));
        assert!(!is_empty_params(&serde_json::json!({ "muted": true })));
    }
}
