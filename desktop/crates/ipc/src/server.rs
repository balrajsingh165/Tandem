//! JSON-RPC server: accepts one or more UI connections on the local socket,
//! authenticates same-user peers, dispatches to the daemon's service
//! implementation, and pushes state events.

use crate::api::{IpcEvent, IpcRequest, IpcResponse};
use crate::error::IpcError;

/// What the daemon implements to serve the UI. Kept synchronous in signature so
/// the contract is testable without a runtime; the daemon adapts it to its own
/// async task structure.
pub trait IpcService: Send + Sync {
    fn handle(&mut self, request: IpcRequest) -> Result<IpcResponse, IpcError>;
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
