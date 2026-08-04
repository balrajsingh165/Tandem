# ADR-0004: Desktop — Rust Core Daemon plus Tauri UI

## Context

The desktop side must run, simultaneously: a real-time media path — 7.5 ms SCO frames,
lock-free ring buffers, acoustic echo cancellation `[Tier B — Linux]`
`[Tier B — Win/macOS USB dongle]`; a Bluetooth subsystem that on Windows/macOS reaches down to
USB HCI; a TLS LAN client with reconnect logic; and a mainstream-quality dialer UI needing
system tray, notifications, accessibility, IME support, and native theming. A single process
couples hard real-time audio deadlines to UI rendering pauses and webview garbage collection.
Rust is the core language: no GC in the audio path, memory safety in a protocol-parsing
Bluetooth stack, and the required ecosystem (tokio, rustls, tokio-tungstenite, cpal) is mature.

## Decision

Two separate processes:

- **`tandem-daemon`** — headless Rust binary owning everything real: transport, crypto,
  pairing, the call-mirror state machine, audio pipeline, and Bluetooth backends.
- **`tandem-ui`** — a Tauri 2 shell with a Svelte + TypeScript front-end. It is a pure renderer
  and command surface, talking to the daemon over JSON-RPC 2.0 on a Unix domain socket
  (`$XDG_RUNTIME_DIR/tandem/daemon.sock`) or Windows named pipe (`\\.\pipe\tandem-daemon`),
  the `IpcApi` surface defined in `tandem_ipc::api` and exported to TypeScript via ts-rs.

The media path never runs in the UI process — this is an invariant, not a preference.

**egui was considered and rejected**: an immediate-mode Rust GUI would keep everything in one
language, but its accessibility, IME, and theming support are too weak for a mainstream dialer
that must be usable by everyone. Tauri provides tray, notifications, accessibility, and small
binaries by reusing the OS webview.

## Status

Accepted.

## Consequences

- Webview jank, GC pauses, or a crashed UI cannot touch SCO deadlines; the daemon runs fully
  headless, which also enables CI protocol testing and future CLI use without any UI installed.
- The IPC boundary is a hard seam: alternative front-ends (egui revisited, CLI, a different web
  stack) remain possible without touching daemon code. Contract in docs/11-api-reference.md.
- Costs accepted: two processes to package, launch, and supervise; the UI must handle daemon
  absence and restart gracefully; the `IpcApi` becomes a versioned compatibility surface.
- The webview engine differs per OS (WebView2, WKWebView, WebKitGTK). Acceptable because
  nothing latency-critical or security-critical renders there — keys, sockets, and audio stay
  in the daemon.
- ts-rs generation keeps UI types honest without hand-duplicating DTOs, consistent with the
  single-source-of-truth rule of ADR-0009 (protobuf for the wire, `tandem_ipc::api` for IPC).
