//! tandem_transport: discovery, connection, and codec for TLP v1 over WebSocket +
//! mutual TLS 1.3. Exposes the TransportClient trait (docs/11) so core and tests
//! never touch sockets directly.

pub mod client;
pub mod codec;
pub mod discovery;
pub mod error;
pub mod reconnect;
pub mod tls;

pub use client::TransportClient;
pub use codec::EnvelopeCodec;
pub use error::TransportError;

/// Default TLP port; the SRV record carries the actual port when it differs.
pub const DEFAULT_PORT: u16 = 46521;

/// Protocol version this build speaks.
pub const PROTOCOL_VERSION: u32 = 1;

/// Heartbeat cadence and the silence after which a peer is considered dead.
pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;
pub const DEAD_PEER_TIMEOUT_SECS: u64 = 15;

/// Frames larger than this are a protocol violation, not a fragmentation hint.
pub const MAX_ENVELOPE_BYTES: usize = 256 * 1024;
