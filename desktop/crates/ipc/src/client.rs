//! JSON-RPC client used by the Tauri shell: request/response with timeouts,
//! event subscription, and automatic reconnect to a restarted daemon.

use crate::api::IpcRequest;
use crate::error::IpcError;

/// Default time the UI waits for the daemon before surfacing a timeout.
pub const REQUEST_TIMEOUT_MS: u64 = 5_000;

/// Allocates JSON-RPC request ids for correlation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestIdAllocator {
    next: u64,
}

impl RequestIdAllocator {
    pub fn allocate(&mut self) -> u64 {
        self.next = self.next.saturating_add(1);
        self.next
    }
}

/// Builds the outbound JSON-RPC frame for a request.
pub fn request_frame(id: u64, request: &IpcRequest) -> serde_json::Value {
    let mut frame = serde_json::json!({
        "jsonrpc": crate::JSONRPC_VERSION,
        "id": id,
    });
    if let serde_json::Value::Object(request_fields) =
        serde_json::to_value(request).unwrap_or(serde_json::Value::Null)
    {
        if let Some(object) = frame.as_object_mut() {
            for (key, value) in request_fields {
                object.insert(key, value);
            }
        }
    }
    frame
}

/// Maps a JSON-RPC error code back onto the typed error the UI branches on.
pub fn error_from_code(code: i32, message: &str) -> IpcError {
    use crate::error::*;
    match code {
        IPC_DAEMON_UNAVAILABLE => IpcError::DaemonUnavailable,
        IPC_NOT_PAIRED => IpcError::NotPaired,
        IPC_PHONE_OFFLINE => IpcError::PhoneOffline,
        IPC_CALL_NOT_FOUND => IpcError::CallNotFound(message.to_string()),
        IPC_INVALID_CALL_STATE => IpcError::InvalidCallState,
        IPC_EMERGENCY_BLOCKED => IpcError::EmergencyBlocked {
            number: message.to_string(),
        },
        IPC_ALREADY_HANDLED => IpcError::AlreadyHandled,
        IPC_RATE_LIMITED => IpcError::RateLimited,
        IPC_AUDIO_UNAVAILABLE => IpcError::AudioUnavailable,
        IPC_PAIRING_FAILED => IpcError::PairingFailed(message.to_string()),
        IPC_TIMEOUT => IpcError::Timeout,
        IPC_UNAUTHORIZED => IpcError::Unauthorized,
        _ => IpcError::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_start_at_one_and_increase() {
        let mut alloc = RequestIdAllocator::default();
        assert_eq!(alloc.allocate(), 1);
        assert_eq!(alloc.allocate(), 2);
    }

    #[test]
    fn request_frames_are_valid_json_rpc() {
        let frame = request_frame(
            3,
            &IpcRequest::Dtmf {
                call_id: "c1".into(),
                digits: "123".into(),
            },
        );
        assert_eq!(frame["jsonrpc"], "2.0");
        assert_eq!(frame["id"], 3);
        assert_eq!(frame["method"], "dtmf");
        assert_eq!(frame["params"]["digits"], "123");
    }

    #[test]
    fn known_codes_map_back_to_typed_errors() {
        assert_eq!(
            error_from_code(crate::error::IPC_ALREADY_HANDLED, ""),
            IpcError::AlreadyHandled
        );
        assert_eq!(
            error_from_code(crate::error::IPC_EMERGENCY_BLOCKED, "911"),
            IpcError::EmergencyBlocked {
                number: "911".into()
            }
        );
    }

    #[test]
    fn unknown_codes_degrade_to_internal_rather_than_panicking() {
        assert_eq!(error_from_code(-1, "mystery"), IpcError::Internal);
    }
}
