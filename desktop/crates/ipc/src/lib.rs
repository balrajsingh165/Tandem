//! tandem_ipc: the daemon-to-UI contract — JSON-RPC 2.0 over a local socket,
//! with request, response, and event types defined once in api.rs and exported
//! to TypeScript via ts-rs (docs/11).

pub mod api;
pub mod client;
pub mod error;
pub mod server;
pub mod socket;

pub use api::{IpcEvent, IpcRequest, IpcResponse};
pub use error::IpcError;

/// JSON-RPC version string every frame carries.
pub const JSONRPC_VERSION: &str = "2.0";
