# ADR-0003: LAN Transport — WebSocket over Mutual TLS 1.3

## Context

The control plane needs an authenticated, encrypted, bidirectional message channel on the LAN.
The phone is the server (it owns state — ADR-0007, and it must accept multiple desktops); the
desktops are clients. Traffic is tiny: under 10 messages per second at worst, with envelopes
capped at 256 KiB. The channel security model is fixed by ADR-0006: mutual TLS 1.3 with pinned
self-signed device certificates, so first-class mTLS support in Ktor (the phone's embedded CIO
server) and rustls (desktop) is a hard requirement.

Candidates: raw TCP with custom framing, WebSocket over TLS, gRPC bidirectional streaming, QUIC.

## Decision

TLP v1 runs over **WebSocket over mutual TLS 1.3**: binary frames, exactly one protobuf
`Envelope` per frame, maximum frame size 256 KiB, default TCP port 46521 (the actual port is
carried in the mDNS SRV record; see docs/06-transport-and-protocol.md).

Rejected alternatives:

- **gRPC** — drags in HTTP/2 plus its own TLS configuration surface; the streaming-RPC model
  adds nothing over a single `oneof`-based Envelope catalog; embedding a gRPC server with
  custom-pinned mTLS in an Android app is markedly heavier than Ktor WebSockets.
- **QUIC** — UDP plus ALPN plus comparatively immature mTLS-with-pinning support in the mobile
  and embedded-server ecosystems. Its headline benefit, removing TCP head-of-line blocking, is
  irrelevant at control-plane rates, and voice never rides this channel (ADR-0002).
- **Raw TCP** — reinvents framing, keepalive plumbing, and close semantics that WebSocket
  provides for free, with no offsetting benefit.

## Status

Accepted.

## Consequences

- Mature, boring libraries on both ends: Ktor (CIO) server on Android, tokio-tungstenite plus
  rustls on the desktop; mTLS with SPKI pinning is well-trodden in both stacks.
- Message framing arrives free: one WebSocket binary frame equals one `Envelope`, so the codec
  layers (`EnvelopeCodec.kt`, `tandem_transport::codec`) contain no length-prefix logic.
- Liveness stays application-visible: `Heartbeat`/`HeartbeatAck` ride TLP (5 s interval, 15 s
  dead-peer) rather than relying on WebSocket ping frames, so both peers time out identically
  regardless of library behavior.
- TCP head-of-line blocking is accepted deliberately: at fewer than 10 small messages per
  second it is unmeasurable, and the media plane is Bluetooth, not this socket.
- Version negotiation lives in `SessionHello`/`SessionWelcome` payloads, not in ALPN, keeping
  the protocol portable across any future transport swap (see docs/06-transport-and-protocol.md,
  Message Catalog).
