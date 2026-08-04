# ADR-0010: Tier Model and the Media-Backend Abstraction

## Context

The control plane is identical everywhere `[Tier A]`, but the media plane varies radically by
platform: software-only HFP HF via BlueZ/PipeWire `[Tier B — Linux]`; a dedicated USB Bluetooth
controller with Tandem's own HFP stack `[Tier B — Win/macOS USB dongle]`; no desktop audio at
all, with a commodity speakerphone paired to the phone `[Tier B-lite fallback]`; and a
hypothetical sanctioned platform API `[Tier C — needs vendor support]`. Without a deliberate
seam, these differences leak conditional logic into the controller, transport, and UI layers
and turn each tier into a fork.

## Decision

One narrow media seam on each side of the LAN; everything above it is tier-blind.

- **Android**: all call-audio routing sits behind the `CallMediaProvider` domain port. Today's
  implementation, `HfpCallMediaProvider`, executes `AudioRouteRequest` by steering
  `InCallService` audio routing toward the desktop's bonded HF device and reports route
  reality from `CallAudioState` callbacks. A future Tier C vendor backend implements the same
  port unchanged.
- **Desktop**: two traits — `BluetoothBackend` (`tandem_bluetooth`) and `AudioBackend`
  (`tandem_audio`). Bluetooth backends: `linux_bluez`, `usb_dongle`, and `null_backend`
  (Tier B-lite: control and history with no local audio). Audio backends: `cpal_backend` and
  `null_backend`. The HFP protocol logic itself — AT parsing, SLC, indicators, codec
  negotiation, call-state mirroring — lives above the backends in `tandem_bluetooth::hfp` and
  is backend-agnostic.
- **Tier A depends on none of this.** Selecting the null backends *is* Tier B-lite; no core,
  transport, or UI code branches on tier — tiers are configuration, not code paths.
- **Where Tier C would attach** `[Tier C — needs vendor support]`: on the phone side it implements
  `CallMediaProvider` unchanged. On the desktop it does *not* fit `BluetoothBackend`, which is
  deliberately HFP-shaped (adapter, bonding, RFCOMM channel, SCO open/close); it slots in as a peer
  of the Bluetooth backends under the same selection logic in `daemon/src/app.rs`, feeding
  `AudioBackend` directly and bypassing `tandem_bluetooth::hfp`. Naming this now keeps the audio
  pipeline from assuming a SCO clock is the only frame source.

## Status

Accepted.

## Consequences

- Tiers are interchangeable at the seam: shipping Tier B-lite first, adding Linux, then the
  dongle path, then a Tier C backend requires no architectural change — the roadmap phases in
  docs/16-roadmap.md map one-to-one onto backend implementations.
- Testability follows directly: `FakeCallMediaProvider` on Android and `fake_bluetooth_backend`,
  `fake_audio_backend`, and `fake_ag` in `tandem_testkit` implement the same seams, so the HFP
  state machine and routing logic are tested without any radio or hardware
  (docs/15-testing-strategy.md).
- Accepted costs: trait indirection, and the interfaces must stay lowest-common-denominator —
  dongle-specific capabilities are expressed generically or kept private to the backend, never
  surfaced upward.
- The seam enforces the plane separation of ADR-0002: backends move audio and mirror
  indicators; user intent never enters through them (single-command-path rule,
  docs/05-bluetooth-hfp.md).
- Contracts for `CallMediaProvider`, `BluetoothBackend`, and `AudioBackend` are specified in
  docs/11-api-reference.md.
