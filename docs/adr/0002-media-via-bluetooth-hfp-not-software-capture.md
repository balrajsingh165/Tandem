# ADR-0002: Call Media via Bluetooth HFP, Not Software Capture

## Context

Tandem must deliver live two-way call audio to the desktop. The obvious-seeming design — capture
call audio on the phone in software and stream it over the LAN — is impossible on stock,
non-rooted Android: the `VOICE_CALL`, `VOICE_DOWNLINK`, and `VOICE_UPLINK` audio sources are
gated behind `CAPTURE_AUDIO_OUTPUT`, a `signature|privileged` permission unavailable to
installable apps, and there is no API at all for injecting audio into the cellular uplink. Even
a hypothetical downlink capture would therefore yield one-way audio. Root, hidden APIs, and
accessibility-service abuse are excluded by the project's rule-abiding stance.

What stock Android does sanction: routing call audio to a Bluetooth Hands-Free device. The
phone's own Bluetooth stack implements the HFP Audio Gateway role for any bonded HF unit — this
is exactly how every car kit and headset receives call audio today.

## Decision

Call audio reaches the desktop **exclusively via Bluetooth HFP**: the phone acts as Audio
Gateway (implemented by Android's Bluetooth stack, not by Tandem), the desktop acts as the
Hands-Free unit. The LAN control plane coordinates routing but never carries voice, and Tandem
software on the phone never touches call audio.

Desktop HF implementations sit behind the media-backend seam (ADR-0010):

- `[Tier B — Linux]` software-only HF via BlueZ, audio via PipeWire; no special hardware.
- `[Tier B — Win/macOS USB dongle]` the OS stacks do not expose the HF role to applications, so
  the daemon drives a dedicated USB Bluetooth controller directly and implements HFP against the
  published Bluetooth SIG specification — a legitimate protocol implementation, not reverse
  engineering of any product.
- `[Tier B-lite fallback]` no desktop audio at all: the user pairs any commodity Bluetooth
  speakerphone or earbuds to the phone; Tandem does control and history only.

Single-command-path rule: all user intent travels over the LAN control plane; the desktop never
issues HFP call-control AT commands. HFP carries audio, codec negotiation, indicator mirroring,
and volume sync only (see docs/05-bluetooth-hfp.md).

## Status

Accepted.

## Consequences

- `[Tier A]` control and history ship with zero audio work; audio is strictly additive, and
  every audio failure degrades to Tier A behavior — if SCO or the HFP link drops, the call
  continues on the handset and is never dropped by Tandem.
- Tier B complexity is real and accepted. The dongle path means implementing HCI, L2CAP, RFCOMM,
  SDP, and SCO routing in `tandem_bluetooth` and requires the user to buy a supported USB
  controller `[Tier B — Win/macOS USB dongle]`.
- Audio quality is bounded by HFP codecs — CVSD (8 kHz) and mSBC (16 kHz wideband); telephony
  grade, roughly 40–80 ms added latency (see docs/05-bluetooth-hfp.md). Acceptable: the cellular
  leg is narrowband anyway.
- A future sanctioned platform "call-audio companion" API drops in as another media backend
  `[Tier C — needs vendor support]` without architectural change.
- Every document discussing media must restate the no-software-capture reality; nothing in
  Tandem may imply carrier call audio can be captured in software on stock Android.
