# Tandem

Tandem lets you place, receive, and control **real SIM cellular calls from your desktop** while the SIM stays in your Android phone. Phone and PC sit in the same room, on the same LAN. Call audio reaches the desktop because the desktop presents itself to the phone as a **Bluetooth Hands-Free (HFP) headset** — the same mechanism a car kit uses. On stock non-rooted Android no app can capture or inject carrier call audio in software (the `VOICE_CALL` sources sit behind the `signature|privileged` permission `CAPTURE_AUDIO_OUTPUT`), which is exactly why Tandem bridges audio over Bluetooth HFP instead.

## The three planes

| Plane | Carries | Path |
|---|---|---|
| **Control** | dial, answer, mute, hold, merge, end, DTMF, call state, call-log sync | Tandem LAN Protocol (TLP) v1: WebSocket over mutual TLS 1.3, desktop ⇄ phone |
| **Media** | live two-way call audio | Bluetooth HFP — phone is the Audio Gateway, desktop is the Hands-Free unit; the LAN coordinates routing but never carries voice |
| **Cellular** | the phone's genuine SIM CS/VoLTE/VoWiFi call | the carrier network — Tandem drives and bridges into it, never reimplements it |

## Tier model

`[Tier A]` — control and history — works today on stock non-rooted Android: the app becomes the default dialer (`ROLE_DIALER` + `InCallService`), the desktop mirrors and drives calls over the LAN, and it ships as a complete product with zero Bluetooth audio work. Tier B adds call audio by making the desktop an HFP Hands-Free unit: software-only over BlueZ and PipeWire `[Tier B — Linux]`, or on the Windows-only no-extra-hardware track via a signed Windows Bluetooth profile-driver backend documented in [docs/17-windows-software-audio.md](docs/17-windows-software-audio.md) and ADR-0011. The dedicated USB controller design `[Tier B — Win/macOS USB dongle]` stays macOS's Tier B path, and doubles as the Windows fallback if the native-driver spike fails and the product goal changes to allow extra hardware. `[Tier B-lite fallback]` keeps the desktop on control/history while audio stays on the handset or a device paired directly to the phone. `[Tier C — needs vendor support]` is roadmap only: a sanctioned AOSP/OEM call-audio companion API that would drop in as another media backend.

> **Emergency calls are never bridged.** A desktop-originated call has no reliable caller location, so emergency numbers (911/112/…) are refused on both ends with `ERROR_CODE_EMERGENCY_NUMBER_BLOCKED` and the UI directs you to dial on the handset, which has carrier location facilities. An emergency call already active on the handset is surfaced read-only: no remote control, no audio-route changes. See docs/adr/0008-emergency-call-policy.md.

## Repository map

```text
tandem/
├── README.md          You are here.
├── CONTRIBUTING.md    How to add docs and ADRs; the docstring rule.
├── CLAUDE.md          Agent/contributor guidance: the hard invariants and coding standards.
├── LICENSE            TBD — see docs/adr/0001-licensing-and-project-name.md.
├── .gitignore         Android/Gradle, Rust/Cargo, Node/Tauri sections.
├── proto/tandem/v1/   TLP v1 wire schema, five .proto files — single source of truth.
├── docs/              Eighteen numbered docs, REPO-STRUCTURE.md, and adr/ with eleven ADRs.
├── android/           Tandem Gateway app — Kotlin, package com.tandem.gateway.
├── desktop/           Rust workspace: crates/, daemon/ "tandem-daemon", ui/ "Tauri 2 shell".
└── tools/             Proto codegen scripts, Tier A smoke tests, usb-dongle-probe.
```

The canonical file-by-file inventory with intended docstrings is [docs/REPO-STRUCTURE.md](docs/REPO-STRUCTURE.md).

## Start here

Read [docs/00-overview.md](docs/00-overview.md) first, then follow the numbered docs in order. Architecture is in docs/01, honest feasibility analysis in docs/02, the wire protocol (with the full proto text) in docs/06, and the Windows-only software-audio strategy in docs/17.

## Build status & quick start

**Status: documentation-first.** The architecture, TLP v1 protocol, module contracts, and full source layout are specified; the source trees are not yet buildable end-to-end. Developer setup, protobuf codegen, and the Tier A LAN smoke test (`tools/dev/tier-a-smoke.sh` / `.ps1`) are defined in [docs/13-build-and-setup.md](docs/13-build-and-setup.md). Short version once code lands: sideload the Android app and grant it default dialer, run `cargo run -p tandem_daemon` from `desktop/`, start the UI from `desktop/ui/`, pair via QR.
