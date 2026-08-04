# CLAUDE.md

Guidance for Claude Code (and any AI coding agent) working in the Tandem repository.

## What this project is

Tandem lets a user place, receive, and control real SIM-based cellular calls from their desktop
while the SIM stays in their Android phone (same room, same LAN). Three planes, kept strictly
separate:

- **Control plane** — dial/answer/mute/hold/merge/end/DTMF/call-log sync over an encrypted LAN
  channel (WebSocket over mutual TLS 1.3, protobuf `Envelope` frames).
- **Media plane** — live call audio over Bluetooth HFP (phone = Audio Gateway, desktop =
  Hands-Free unit). The LAN triggers routing; it never carries voice.
- **Cellular plane** — the phone's genuine carrier call. Tandem drives it, never reimplements it.

Start with [docs/00-overview.md](docs/00-overview.md); the full doc set lives in
[docs/](docs/) and the canonical file inventory in [docs/REPO-STRUCTURE.md](docs/REPO-STRUCTURE.md).

## Hard invariants — never violate these

1. **No software capture of carrier call audio.** `VOICE_CALL`/`VOICE_DOWNLINK`/`VOICE_UPLINK`
   are gated behind the `signature|privileged` permission `CAPTURE_AUDIO_OUTPUT`; there is no
   uplink-injection API. Any change implying otherwise is wrong by design (ADR-0002).
2. **Emergency calls are forced to the handset.** Desktop-originated dials to emergency numbers
   are refused on both ends (`ERROR_CODE_EMERGENCY_NUMBER_BLOCKED`); active emergency calls are
   surfaced read-only (ADR-0008).
3. **No root, ever, for Tier A/B.** Only published mechanisms: default-dialer APIs, the public
   Bluetooth SIG HFP spec, standard LAN networking.
4. **Phone is the source of truth** for call state and call log. The desktop holds a derived
   mirror versioned by `(epoch_id, state_seq)` and reconciles via `Resume` (ADR-0007).
5. **Single command path.** User intent travels over the LAN control plane only; the desktop
   never sends HFP call-control AT commands (docs/05).
6. **Protobuf is the single wire-type source.** All cross-device types live in `/proto` and are
   generated into Kotlin and Rust. Never hand-duplicate DTOs (ADR-0009).

## Repository layout

```text
proto/     TLP v1 protobuf schema (single source of truth for wire types)
docs/      architecture docs 00-16, ADRs, REPO-STRUCTURE.md
android/   Tandem Gateway app (Kotlin, Hilt, Compose, package com.tandem.gateway)
desktop/   Rust workspace: crates/{proto,core,transport,pairing,crypto,audio,bluetooth,ipc,testkit},
           daemon/ (headless binary), ui/ (Tauri 2 + Svelte)
tools/     proto codegen scripts, Tier A smoke test, USB-dongle probe
```

## Coding standards

Full text: [docs/14-coding-conventions.md](docs/14-coding-conventions.md). Non-negotiables:

- **Layering:** `domain` (pure models + use-cases, framework-free) → `data`/`infra`
  (implementations) → `ui`. Depend on interfaces (`domain/port` on Android, traits on desktop),
  never concretions. No business logic in UI, Compose, or framework callbacks.
- **One file-level docstring per source file — and nothing more by way of narration.** KDoc
  `/** … */` for Kotlin, `//!` for Rust, JSDoc block for TS/Svelte. It states purpose, key public
  types, collaborators, non-obvious constraints. **No repeated inline comments in bodies**; code
  self-explains via naming and small functions. The intended docstring for every file is pinned
  in [docs/REPO-STRUCTURE.md](docs/REPO-STRUCTURE.md) — keep them in sync.
- **Testable seams:** every I/O boundary (telecom, Bluetooth, sockets, storage, audio) sits
  behind an interface with a fake in the testkits (`android …/testkit/`, `desktop/crates/testkit`).
- **Errors:** typed per boundary — Kotlin sealed classes, Rust `thiserror` enums (names in
  docs/11). No stringly-typed errors. Map to TLP `Status` only at transport edges.
- **Tier tags:** anything OS-specific, hardware-dependent, or vendor-gated is tagged inline with
  exactly one of: `[Tier A]`, `[Tier B — Linux]`, `[Tier B — Win/macOS USB dongle]`,
  `[Tier B-lite fallback]`, `[Tier C — needs vendor support]`.
- **Naming:** Kotlin `PascalCase` types / `camelCase` members, package `com.tandem.gateway`;
  Rust `snake_case` items / `PascalCase` types, crates `tandem_*`; docs `NN-kebab-case.md`;
  ADRs `adr/NNNN-kebab-title.md`.

## Commit convention

Conventional Commits, **single-line subject only** — no bodies unless a change genuinely needs
one. Imperative mood, lowercase, ≤ 72 chars:

```text
feat: add hfp codec negotiation state machine
fix: reconcile stale epoch on lan resume
docs: expand pairing revocation flow
chore: bump gradle wrapper
refactor: extract envelope correlation from client
test: add fake ag slc bring-up scenarios
build: pin rust toolchain
ci: add proto drift check
```

Scopes are optional (`feat(android): …`, `fix(desktop): …`, `docs(adr): …`). Do not mix
concerns in one commit; split android/desktop/proto/docs changes into separate commits.

## Common commands

```bash
# Protocol codegen (both languages) — run after any /proto change
tools/gen-proto.sh          # POSIX
tools/gen-proto.ps1         # Windows

# Android (from android/)
./gradlew assembleDebug     # build
./gradlew test              # unit tests (testkit fakes, no device)

# Desktop (from desktop/)
cargo build --workspace
cargo test --workspace
cargo run -p tandem_daemon  # headless daemon
cd ui && npm install && npm run tauri dev   # UI shell in dev

# Tier A end-to-end smoke test (phone + desktop on same LAN)
tools/dev/tier-a-smoke.sh   # or .ps1
```

Build/setup details, device sideloading, and the default-dialer dev grant:
[docs/13-build-and-setup.md](docs/13-build-and-setup.md).

## Documentation rules

- The `.proto` files are embedded verbatim **only** in
  [docs/06-transport-and-protocol.md](docs/06-transport-and-protocol.md); everywhere else,
  reference message names.
- Module maps in docs/03 and docs/04 must match [docs/REPO-STRUCTURE.md](docs/REPO-STRUCTURE.md)
  path-for-path and docstring-for-docstring. Changing a file's purpose means updating all three.
- Architecture decisions get an ADR (`docs/adr/NNNN-kebab-title.md`, Context/Decision/Status/
  Consequences); see [CONTRIBUTING.md](CONTRIBUTING.md).
- Cross-reference sibling docs instead of restating them.
