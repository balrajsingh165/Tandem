# Windows Software Audio Strategy

This document narrows the media-plane plan for a Windows-only Tandem product whose goal is
**no extra Bluetooth dongle and no external audio hardware**. It does not weaken the existing
Android constraint: Tandem still cannot capture or inject carrier call audio in software on a
stock non-rooted phone. The only viable Windows software path is to make the Windows machine act
as a Bluetooth Hands-Free unit through the Windows Bluetooth stack itself.

## Decision frame

The Windows-only target changes the media decision from "bring hardware Tandem owns" to "own the
missing Windows integration layer." A normal desktop app is not enough: public Windows app APIs
cover RFCOMM services and ordinary audio endpoints, while SCO voice access for Bluetooth profiles
lives in the Windows Bluetooth driver stack. Phone Link proves Windows can provide PC call audio
over Bluetooth, but it is a Microsoft application and not a public Tandem integration surface.

So the Windows software-only strategy is:

1. Ship the control plane first: Android default dialer plus Windows daemon/UI over TLP v1.
2. Run a native Windows HFP feasibility spike before committing to desktop audio.
3. If the spike passes, implement a signed Windows Bluetooth profile driver plus a user-mode
   daemon backend instead of the USB-dongle backend.
4. If the spike fails, keep the Windows product honest as control + history with audio on the
   handset or a device paired directly to the phone.

This keeps the "no extra hardware" product goal intact without inventing an Android audio-capture
path or depending on private Phone Link behavior.

## Non-solutions

These are explicitly out of bounds for Windows software-only audio:

- **Android software audio relay.** `AudioPlaybackCapture`, `MediaRecorder`, and microphone APIs
  cannot capture the cellular downlink or inject the uplink for a carrier call. LAN audio is not
  a fallback.
- **Phone Link automation.** Phone Link is useful as prior art, but Tandem must not scrape its UI,
  depend on its private state, require a Microsoft account, or ask it to carry audio for a call
  Tandem controls. That would make the product fragile and non-portable across Windows releases.
- **UWP/WinRT RFCOMM-only implementation.** RFCOMM can carry the HFP service-level connection's
  AT command stream, but it does not expose the SCO/eSCO voice channel Tandem needs for duplex
  call audio.
- **Virtual microphone or loopback tricks.** Windows audio loopback records PC playback, not the
  Android cellular call. A virtual microphone can feed apps on Windows, not the phone's carrier
  uplink.
- **Private Windows APIs or reverse engineering.** The only acceptable Windows software path uses
  documented Microsoft driver interfaces and the public Bluetooth HFP specification.

## Proposed Windows media architecture

The existing plane split stays unchanged:

- TLP over mutual TLS carries dial, answer, hold, mute, DTMF, route intent, call state, and
  history.
- Android Telecom and the carrier own the cellular call.
- Bluetooth HFP carries call audio.

The Windows-specific change is below the `BluetoothBackend` seam.

```text
Android phone                         Windows PC
-------------                         ----------
Android Telecom + SIM                 tandem-ui
Android Bluetooth AG                  tandem-daemon
Tandem Gateway                        tandem_bluetooth::hfp
                                      windows_profile backend
                                      user-mode driver bridge
                                      signed KMDF Bluetooth profile driver
                                      Windows Bluetooth stack + built-in adapter
```

### Kernel/user split

The native Windows backend has two layers:

| Layer | Responsibility |
|---|---|
| Signed KMDF Bluetooth profile driver | Binds to the phone's HFP Audio Gateway service, talks to the Windows Bluetooth driver stack, opens/closes RFCOMM and SCO/eSCO links, exposes a narrow device interface to user mode. |
| `windows_profile` user-mode backend | Bridges the driver interface into `BluetoothBackend`: adapter status, bond status, RFCOMM byte channel for `tandem_bluetooth::hfp`, SCO frame streams, and backend events. |

The daemon still owns policy, LAN commands, HFP state machines, codec negotiation, audio pipeline,
and UI events. The driver should be as small as possible: Bluetooth stack access and deterministic
transport of bytes/frames across the kernel boundary, not call-control policy.

### Audio path

1. The phone and PC complete normal Bluetooth bonding.
2. `SessionHello.bt_adapter_address` reports the built-in Windows adapter address.
3. The phone stores that address against the paired desktop.
4. The Windows profile backend establishes HFP SLC with the phone's AG.
5. The user requests desktop audio over the LAN.
6. Android routes the call to the bonded Windows HF endpoint.
7. The Windows driver receives SCO/eSCO voice packets and passes frames to user mode.
8. `tandem_audio` renders downlink through WASAPI/cpal and captures uplink from the selected
   Windows communication microphone.

The single-command-path rule still applies: Tandem does not answer, hang up, dial, hold, merge, or
send DTMF through HFP AT commands. Those commands remain TLP requests to the phone gateway.

## Feasibility spike

No Windows desktop-audio development should proceed until this spike passes on a real Windows 11
machine and a real Android phone.

### Spike scope

- Target Windows 11 first. Windows 10 support is deferred until the Windows 11 path works.
- Use the built-in PC Bluetooth adapter, not a USB dongle dedicated to Tandem.
- Use test signing during development; document the production signing path separately.
- Use one known Android phone model first, then expand to a small device matrix.

### Pass criteria

The spike passes only if all of these are demonstrated:

1. The driver can bind to the phone's HFP Audio Gateway service through the Windows Bluetooth stack.
2. Tandem can complete HFP SLC against Android and keep it stable for 30 minutes idle.
3. A real carrier call can be routed from Android to the Windows PC without Phone Link.
4. SCO/eSCO audio frames cross the driver/user-mode boundary in both directions.
5. The user-mode audio pipeline can render downlink and feed uplink with tolerable latency and no
   call drop on driver restart, daemon restart, or Bluetooth disconnect.
6. The phone falls back to handset audio when the Windows media path fails.
7. The implementation uses documented Windows driver interfaces and public Bluetooth SIG behavior.

### Fail criteria

The spike fails if any of these are true:

- SCO/eSCO voice cannot be accessed without replacing the built-in adapter driver or adding a
  second controller.
- The required driver cannot be packaged and signed for ordinary users.
- The approach needs private Phone Link, private Windows Bluetooth APIs, UI automation, or
  reverse-engineered protocol behavior.
- Audio can be received but not transmitted to the phone uplink.
- A backend crash can drop the cellular call instead of falling back to the handset.

## Product implications

The Windows software-only path removes the extra-dongle requirement, but it adds a real platform
cost:

- The installer needs administrator elevation for the driver.
- Production distribution needs Microsoft driver signing.
- Microsoft Store distribution is unlikely for the full-audio build.
- Kernel bugs are higher severity than daemon bugs, so the driver must be tiny, fuzzed, and
  covered by Driver Verifier during development.
- Support must account for Windows build differences, Bluetooth radio firmware quirks, OEM driver
  stacks, and enterprise machines that block third-party drivers.

That trade is still better aligned with the user's product goal than requiring a Tandem-specific
USB controller.

## Documentation changes implied by a successful spike

If the spike passes, update the architecture documents as follows:

- Replace the Windows part of `[Tier B - Win/macOS USB dongle]` with a Windows native profile
  backend.
- Keep the USB-dongle backend only as an optional fallback or remove it from the Windows roadmap.
- Drop macOS from the committed Tier B roadmap unless a separate macOS software strategy is chosen.
- Add `windows_profile` to the `BluetoothBackend` implementations in docs/04, docs/05, docs/11,
  and docs/REPO-STRUCTURE.md.
- Add driver signing, installation, and rollback to docs/12 and docs/13.
- Add a Windows driver test tier to docs/15: Driver Verifier, HLK-relevant checks, stress
  reconnects, and crash/fallback behavior.

## References checked

- Microsoft Phone Link documents that PC calling uses Bluetooth between Windows and the phone:
  <https://support.microsoft.com/en-us/windows/apps/phonelink/setting-up-calls-in-the-phone-link>
- Microsoft Windows app documentation exposes RFCOMM APIs, which are useful for byte streams but
  not sufficient for SCO voice:
  <https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/send-or-receive-files-with-rfcomm>
- Microsoft Bluetooth profile-driver documentation describes driver-stack access to SCO links:
  <https://learn.microsoft.com/en-us/windows-hardware/drivers/bluetooth/bluetooth-profile-drivers-overview>
- Android/AOSP declares `CAPTURE_AUDIO_OUTPUT` outside ordinary third-party app reach, which is
  why the phone-side audio relay is not a viable design:
  <https://android.googlesource.com/platform/frameworks/base/+/refs/heads/android16-qpr2-release/core/res/AndroidManifest.xml>
