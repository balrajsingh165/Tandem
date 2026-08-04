//! TransportClient implementation: dials the phone endpoint with the
//! pinned-peer TLS config, performs SessionHello/SessionWelcome, then pumps
//! Envelope frames bidirectionally with heartbeats (5 s send / 15 s dead-peer).

use std::future::Future;

use tandem_proto::envelope::Payload;

use crate::error::TransportError;
use crate::reconnect::ResumeCursor;

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Establishes a session. Fails closed on pin mismatch — a wrong key is never
    /// retried into.
    fn connect(
        &self,
        endpoint: &str,
    ) -> impl Future<Output = Result<SessionInfo, TransportError>> + Send;

    /// Sends a request and resolves when the correlated reply arrives, or with
    /// `RequestTimeout` if the peer does not answer.
    fn request(
        &self,
        payload: Payload,
    ) -> impl Future<Output = Result<Payload, TransportError>> + Send;

    /// Reconciles the mirror against phone truth after a reconnect.
    fn resume(
        &self,
        cursor: ResumeCursor,
    ) -> impl Future<Output = Result<Payload, TransportError>> + Send;

    fn close(&self) -> impl Future<Output = ()> + Send;
}
