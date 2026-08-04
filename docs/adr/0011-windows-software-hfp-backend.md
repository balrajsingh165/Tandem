# ADR-0011: Windows Software HFP Backend Before USB Hardware

## Context

The current media roadmap treats Windows and macOS desktop audio as
`[Tier B — Win/macOS USB dongle]`: Tandem owns a dedicated Bluetooth controller and implements the
HFP host stack above raw HCI. That path is technically clean but violates the Windows-only product
goal of no extra hardware.

The Android side still cannot supply call audio over LAN. Stock non-rooted Android does not give
third-party apps a carrier-call downlink capture API or cellular uplink injection API. The phone's
Bluetooth stack remains the only sanctioned way to move real SIM call audio to another device.

Windows is different from Linux: the ordinary app APIs do not expose an HFP Hands-Free role with
SCO/eSCO audio. Microsoft documents Phone Link as a Bluetooth-based PC calling product, and the
Windows Bluetooth driver stack has profile-driver interfaces for SCO links, but that is a driver
or system-component surface, not a normal Tauri/Rust application surface.

## Decision

For the Windows-only, software-only product direction, Tandem will investigate a native Windows
HFP backend before committing to any USB Bluetooth controller requirement.

The proposed backend is:

- a signed Windows Bluetooth profile driver using documented Windows Bluetooth driver-stack
  interfaces for HFP-related RFCOMM/SCO access;
- a narrow user-mode bridge exposed to `tandem-daemon`;
- a `windows_profile` implementation of the existing `BluetoothBackend` seam;
- the existing `tandem_bluetooth::hfp` protocol core and `tandem_audio` pipeline above that seam.

Tandem will not depend on Phone Link, private Windows APIs, UI automation, Android call-audio
capture, or reverse-engineered behavior. Phone Link is treated only as evidence that Windows can
support this class of user experience when the correct system integration exists.

The Windows native backend is phase-gated by docs/17-windows-software-audio.md. Until that spike
passes, the shippable Windows product remains `[Tier A]`: control and history, with call audio on
the handset or a device paired directly to the phone.

## Status

Accepted.

## Consequences

- The Windows product can keep the no-extra-hardware goal if the spike succeeds.
- The implementation cost moves from hardware ownership to Windows driver ownership: driver
  signing, installer elevation, Driver Verifier, Windows build variance, OEM Bluetooth variance,
  and a higher-severity failure surface.
- The daemon/UI architecture remains intact. TLP, pairing, phone-as-source-of-truth, emergency
  policy, and the single-command-path rule do not change.
- ADR-0010's backend seam is validated: `windows_profile` is another `BluetoothBackend`
  implementation rather than a fork of the controller or transport layers.
- The existing USB-dongle path is no longer the preferred Windows answer. It remains a fallback
  architecture only if the Windows native spike fails and the product goal changes to allow extra
  hardware.
- macOS is not solved by this ADR. A macOS software-only audio path would need its own decision.
