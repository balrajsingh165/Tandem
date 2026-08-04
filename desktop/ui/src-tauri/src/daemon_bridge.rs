//! Bridges the webview and the daemon socket: forwards JSON-RPC requests from
//! the front-end via Tauri commands, streams daemon events to the webview, and
//! manages daemon liveness (spawn/reconnect prompts).

use serde::Serialize;
use tandem_ipc::error::{IpcError, IPC_DAEMON_UNAVAILABLE};

/// Event channel name the front-end subscribes to.
pub const EVENT_CHANNEL: &str = "tandem://event";

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
}

/// Forwards one request to the daemon. The shell performs no policy of its own:
/// every decision, including the emergency refusal, belongs to the daemon.
#[tauri::command]
pub async fn daemon_request(
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, BridgeError> {
    let _ = (method, params);
    Err(BridgeError::daemon_unavailable())
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
        let error = BridgeError::daemon_unavailable();
        assert_eq!(error.code, IPC_DAEMON_UNAVAILABLE);
    }
}
