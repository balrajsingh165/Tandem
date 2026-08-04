//! Encodes/decodes Envelope frames (one per binary WS message, 256 KiB cap),
//! assigns monotonic message_ids, and matches in_reply_to responses to pending
//! requests with timeouts.

use prost::Message as _;
use tandem_proto::{envelope::Payload, Envelope};

use crate::error::TransportError;
use crate::{MAX_ENVELOPE_BYTES, PROTOCOL_VERSION};

/// Allocates `message_id`s. The counter is monotonic per device and persisted
/// across sessions — it never restarts — so (device id, message_id) stays unique
/// for the life of the pairing and post-reconnect retries can be deduplicated
/// (docs/06 framing rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageIdAllocator {
    next: u64,
}

impl Default for MessageIdAllocator {
    fn default() -> Self {
        Self::starting_at(1)
    }
}

impl MessageIdAllocator {
    /// Resumes from the highest id already used by this device.
    pub fn starting_at(next: u64) -> Self {
        Self { next: next.max(1) }
    }

    pub fn allocate(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    pub fn peek(&self) -> u64 {
        self.next
    }
}

#[derive(Debug, Default)]
pub struct EnvelopeCodec {
    ids: MessageIdAllocator,
}

impl EnvelopeCodec {
    pub fn resuming_at(next_message_id: u64) -> Self {
        Self {
            ids: MessageIdAllocator::starting_at(next_message_id),
        }
    }

    pub fn next_message_id(&self) -> u64 {
        self.ids.peek()
    }

    pub fn encode_request(&mut self, payload: Payload) -> Result<Vec<u8>, TransportError> {
        let envelope = Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: self.ids.allocate(),
            in_reply_to: 0,
            payload: Some(payload),
        };
        Self::encode(&envelope)
    }

    /// Retries after a reconnect reuse the original id so the phone's dedup
    /// ledger recognizes them; a fresh id would execute the request twice.
    pub fn encode_retry(
        &self,
        payload: Payload,
        original_message_id: u64,
    ) -> Result<Vec<u8>, TransportError> {
        let envelope = Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: original_message_id,
            in_reply_to: 0,
            payload: Some(payload),
        };
        Self::encode(&envelope)
    }

    pub fn encode(envelope: &Envelope) -> Result<Vec<u8>, TransportError> {
        let bytes = envelope.encode_to_vec();
        if bytes.len() > MAX_ENVELOPE_BYTES {
            return Err(TransportError::FrameTooLarge {
                size: bytes.len(),
                max: MAX_ENVELOPE_BYTES,
            });
        }
        Ok(bytes)
    }

    pub fn decode(frame: &[u8]) -> Result<Envelope, TransportError> {
        if frame.len() > MAX_ENVELOPE_BYTES {
            return Err(TransportError::FrameTooLarge {
                size: frame.len(),
                max: MAX_ENVELOPE_BYTES,
            });
        }
        let envelope = Envelope::decode(frame)
            .map_err(|e| TransportError::ProtocolViolation(e.to_string()))?;
        if envelope.payload.is_none() {
            return Err(TransportError::ProtocolViolation(
                "envelope carries no payload".into(),
            ));
        }
        Ok(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tandem_proto::DialRequest;

    fn dial() -> Payload {
        Payload::DialRequest(DialRequest {
            number: "+14155550123".into(),
            sim_slot: -1,
        })
    }

    #[test]
    fn message_ids_start_at_one_and_increase() {
        let mut codec = EnvelopeCodec::default();
        let first = EnvelopeCodec::decode(&codec.encode_request(dial()).unwrap()).unwrap();
        let second = EnvelopeCodec::decode(&codec.encode_request(dial()).unwrap()).unwrap();
        assert_eq!(first.message_id, 1);
        assert_eq!(second.message_id, 2);
    }

    #[test]
    fn ids_resume_across_sessions_rather_than_restarting() {
        let mut codec = EnvelopeCodec::resuming_at(42);
        let frame = codec.encode_request(dial()).unwrap();
        assert_eq!(EnvelopeCodec::decode(&frame).unwrap().message_id, 42);
    }

    #[test]
    fn a_retry_reuses_the_original_id_so_dedup_can_fire() {
        let mut codec = EnvelopeCodec::default();
        let original = EnvelopeCodec::decode(&codec.encode_request(dial()).unwrap()).unwrap();
        let retry =
            EnvelopeCodec::decode(&codec.encode_retry(dial(), original.message_id).unwrap())
                .unwrap();
        assert_eq!(retry.message_id, original.message_id);
    }

    #[test]
    fn round_trip_preserves_the_payload() {
        let mut codec = EnvelopeCodec::default();
        let decoded = EnvelopeCodec::decode(&codec.encode_request(dial()).unwrap()).unwrap();
        match decoded.payload {
            Some(Payload::DialRequest(d)) => {
                assert_eq!(d.number, "+14155550123");
                assert_eq!(d.sim_slot, -1);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
        assert_eq!(decoded.protocol_version, PROTOCOL_VERSION);
    }

    #[test]
    fn oversized_and_empty_frames_are_protocol_violations() {
        assert!(matches!(
            EnvelopeCodec::decode(&vec![0u8; MAX_ENVELOPE_BYTES + 1]),
            Err(TransportError::FrameTooLarge { .. })
        ));
        let empty = Envelope {
            protocol_version: PROTOCOL_VERSION,
            message_id: 1,
            in_reply_to: 0,
            payload: None,
        };
        assert!(matches!(
            EnvelopeCodec::decode(&empty.encode_to_vec()),
            Err(TransportError::ProtocolViolation(_))
        ));
    }

    #[test]
    fn garbage_does_not_panic_the_decoder() {
        let garbage = vec![0xff, 0xff, 0xff, 0xff, 0x07];
        assert!(EnvelopeCodec::decode(&garbage).is_err());
    }
}
