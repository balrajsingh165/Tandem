# Tandem Repository Structure

Canonical inventory of the Tandem monorepo: every directory, every source file, its one-line
purpose, and its **intended file-level docstring**. This file is the single source of truth for
file paths and docstrings; the module maps in [03-android-app.md](03-android-app.md) and
[04-desktop-app.md](04-desktop-app.md) reproduce the same paths and docstring text verbatim.

**Docstring convention** (see [14-coding-conventions.md](14-coding-conventions.md)): the quoted
text under each file below is the docstring *content*. In real files it is wrapped in the
language's header-comment form — KDoc `/** … */` for Kotlin, `//!` for Rust, `/** … */` JSDoc for
TypeScript/Svelte `<script>` blocks, `<!-- … -->` for XML/HTML, `#` line block for
scripts/TOML/properties, `//` block for Gradle KTS and `.proto`. It is the **only** narrative
comment in the file; bodies carry no repeated inline comments.

Feasibility tags used below: `[Tier A]` `[Tier B — Linux]` `[Tier B — Win/macOS USB dongle]`
`[Tier B-lite fallback]` `[Tier C — needs vendor support]`. Untagged files are tier-independent
infrastructure.

## Directory tree

```text
tandem/
├── README.md
├── CONTRIBUTING.md
├── CLAUDE.md
├── LICENSE
├── .gitignore
├── proto/
│   └── tandem/v1/
│       ├── common.proto
│       ├── call.proto
│       ├── calllog.proto
│       ├── pairing.proto
│       └── transport.proto
├── docs/
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-feasibility-and-constraints.md
│   ├── 03-android-app.md
│   ├── 04-desktop-app.md
│   ├── 05-bluetooth-hfp.md
│   ├── 06-transport-and-protocol.md
│   ├── 07-pairing-and-auth.md
│   ├── 08-security-and-encryption.md
│   ├── 09-data-models.md
│   ├── 10-sequence-diagrams.md
│   ├── 11-api-reference.md
│   ├── 12-permissions-and-platform.md
│   ├── 13-build-and-setup.md
│   ├── 14-coding-conventions.md
│   ├── 15-testing-strategy.md
│   ├── 16-roadmap.md
│   ├── 17-windows-software-audio.md
│   ├── REPO-STRUCTURE.md
│   └── adr/
│       ├── 0001-licensing-and-project-name.md
│       ├── 0002-media-via-bluetooth-hfp-not-software-capture.md
│       ├── 0003-lan-transport-choice.md
│       ├── 0004-desktop-rust-core-and-ui-toolkit.md
│       ├── 0005-android-default-dialer-role.md
│       ├── 0006-pairing-and-key-management.md
│       ├── 0007-phone-as-source-of-truth.md
│       ├── 0008-emergency-call-policy.md
│       ├── 0009-protobuf-single-source-of-truth.md
│       ├── 0010-tier-model-and-media-backend-abstraction.md
│       └── 0011-windows-software-hfp-backend.md
├── android/
│   ├── settings.gradle.kts
│   ├── build.gradle.kts
│   ├── gradle.properties
│   ├── gradle/
│   │   ├── libs.versions.toml
│   │   └── wrapper/gradle-wrapper.properties
│   └── app/
│       ├── build.gradle.kts
│       ├── proguard-rules.pro
│       └── src/
│           ├── main/
│           │   ├── AndroidManifest.xml
│           │   ├── res/values/strings.xml
│           │   ├── res/values/themes.xml
│           │   └── kotlin/com/tandem/gateway/
│           │       ├── TandemApplication.kt
│           │       ├── di/
│           │       │   ├── AppModule.kt
│           │       │   ├── TelecomModule.kt
│           │       │   ├── TransportModule.kt
│           │       │   └── DataModule.kt
│           │       ├── domain/
│           │       │   ├── model/
│           │       │   │   ├── Call.kt
│           │       │   │   ├── CallLogEntry.kt
│           │       │   │   ├── PairedDesktop.kt
│           │       │   │   ├── AudioRoute.kt
│           │       │   │   └── DeviceIdentity.kt
│           │       │   ├── port/
│           │       │   │   ├── TelecomBridge.kt
│           │       │   │   ├── CallMediaProvider.kt
│           │       │   │   ├── LanServer.kt
│           │       │   │   ├── PairingManager.kt
│           │       │   │   ├── CallLogRepository.kt
│           │       │   │   ├── PairedDeviceRepository.kt
│           │       │   │   ├── IdentityStore.kt
│           │       │   │   ├── SettingsRepository.kt
│           │       │   │   └── EmergencyNumberSource.kt
│           │       │   └── usecase/
│           │       │       ├── PlaceCall.kt
│           │       │       ├── AnswerCall.kt
│           │       │       ├── RejectCall.kt
│           │       │       ├── EndCall.kt
│           │       │       ├── SetMute.kt
│           │       │       ├── HoldCall.kt
│           │       │       ├── UnholdCall.kt
│           │       │       ├── MergeCalls.kt
│           │       │       ├── SendDtmf.kt
│           │       │       ├── RequestAudioRoute.kt
│           │       │       ├── ObserveCallState.kt
│           │       │       ├── SyncCallLog.kt
│           │       │       ├── PairDesktop.kt
│           │       │       ├── RevokeDesktop.kt
│           │       │       └── GuardEmergencyNumber.kt
│           │       ├── telecom/
│           │       │   ├── TandemInCallService.kt
│           │       │   ├── TelecomBridgeImpl.kt
│           │       │   └── CallStateMapper.kt
│           │       ├── dialer/
│           │       │   ├── DefaultDialerManager.kt
│           │       │   ├── OutgoingCallPlacer.kt
│           │       │   ├── DialIntentRouter.kt
│           │       │   └── EmergencyNumberSourceImpl.kt
│           │       ├── calllog/
│           │       │   ├── CallLogRepositoryImpl.kt
│           │       │   └── CallLogObserver.kt
│           │       ├── transport/
│           │       │   ├── LanServerImpl.kt
│           │       │   ├── NsdAdvertiser.kt
│           │       │   ├── DesktopSession.kt
│           │       │   ├── SessionRegistry.kt
│           │       │   ├── EnvelopeCodec.kt
│           │       │   ├── WebSocketFraming.kt
│           │       │   └── ControlPlaneRouter.kt
│           │       ├── pairing/
│           │       │   ├── PairingManagerImpl.kt
│           │       │   ├── PairingSession.kt
│           │       │   ├── QrPayloadCodec.kt
│           │       │   ├── DesktopOfferCodec.kt
│           │       │   └── QrImageAnalyzer.kt
│           │       ├── crypto/
│           │       │   ├── IdentityStoreImpl.kt
│           │       │   ├── DeviceCertificates.kt
│           │       │   ├── TlsServerFactory.kt
│           │       │   └── Fingerprints.kt
│           │       ├── bluetooth/
│           │       │   ├── HfpAgMonitor.kt
│           │       │   ├── HfpCallMediaProvider.kt
│           │       │   └── BondedDesktopMatcher.kt
│           │       ├── service/
│           │       │   ├── GatewayForegroundService.kt
│           │       │   ├── GatewayNotifications.kt
│           │       │   └── BootCompletedReceiver.kt
│           │       ├── data/
│           │       │   ├── db/
│           │       │   │   ├── TandemDatabase.kt
│           │       │   │   ├── PairedDesktopDao.kt
│           │       │   │   └── PairedDesktopEntity.kt
│           │       │   ├── PairedDeviceRepositoryImpl.kt
│           │       │   └── SettingsRepositoryImpl.kt
│           │       └── ui/
│           │           ├── MainActivity.kt
│           │           ├── theme/Theme.kt
│           │           ├── status/StatusScreen.kt
│           │           ├── status/StatusViewModel.kt
│           │           ├── pairing/PairingScreen.kt
│           │           ├── pairing/PairingViewModel.kt
│           │           ├── pairing/QrScannerView.kt
│           │           ├── settings/SettingsScreen.kt
│           │           ├── settings/SettingsViewModel.kt
│           │           ├── incall/InCallActivity.kt
│           │           ├── incall/InCallScreen.kt
│           │           ├── incall/InCallViewModel.kt
│           │           ├── incall/IncomingCallNotifier.kt
│           │           ├── dialpad/DialpadScreen.kt
│           │           └── dialpad/DialpadViewModel.kt
│           └── test/kotlin/com/tandem/gateway/
│               ├── transport/
│               │   └── WebSocketFramingTest.kt
│               └── testkit/
│                   ├── FakeTelecomBridge.kt
│                   ├── FakeCallMediaProvider.kt
│                   ├── FakeCallLogRepository.kt
│                   ├── FakePairedDeviceRepository.kt
│                   ├── FakeIdentityStore.kt
│                   ├── FakeSettingsRepository.kt
│                   └── InMemoryLanServer.kt
├── desktop/
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   ├── crates/
│   │   ├── proto/
│   │   │   ├── Cargo.toml
│   │   │   ├── build.rs
│   │   │   └── src/lib.rs
│   │   ├── core/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── model.rs
│   │   │       ├── events.rs
│   │   │       ├── controller.rs
│   │   │       ├── reconcile.rs
│   │   │       ├── emergency.rs
│   │   │       └── error.rs
│   │   ├── transport/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── discovery.rs
│   │   │       ├── client.rs
│   │   │       ├── codec.rs
│   │   │       ├── reconnect.rs
│   │   │       ├── tls.rs
│   │   │       └── error.rs
│   │   ├── pairing/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── qr.rs
│   │   │       ├── offer.rs
│   │   │       ├── flow.rs
│   │   │       ├── short_code.rs
│   │   │       └── error.rs
│   │   ├── crypto/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── identity.rs
│   │   │       ├── cert.rs
│   │   │       ├── pinning.rs
│   │   │       ├── secrets.rs
│   │   │       └── error.rs
│   │   ├── audio/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── backend.rs
│   │   │       ├── cpal_backend.rs
│   │   │       ├── null_backend.rs
│   │   │       ├── ring_buffer.rs
│   │   │       ├── resampler.rs
│   │   │       ├── aec.rs
│   │   │       ├── pipeline.rs
│   │   │       └── error.rs
│   │   ├── bluetooth/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── backend.rs
│   │   │       ├── error.rs
│   │   │       ├── hfp/
│   │   │       │   ├── mod.rs
│   │   │       │   ├── at.rs
│   │   │       │   ├── slc.rs
│   │   │       │   ├── indicators.rs
│   │   │       │   ├── codec_negotiation.rs
│   │   │       │   └── call_mirror.rs
│   │   │       └── backends/
│   │   │           ├── mod.rs
│   │   │           ├── null_backend.rs
│   │   │           ├── linux_bluez/
│   │   │           │   ├── mod.rs
│   │   │           │   ├── profile.rs
│   │   │           │   └── sco.rs
│   │   │           └── usb_dongle/
│   │   │               ├── mod.rs
│   │   │               ├── usb_transport.rs
│   │   │               ├── hci.rs
│   │   │               ├── l2cap.rs
│   │   │               ├── rfcomm.rs
│   │   │               ├── sdp.rs
│   │   │               ├── security.rs
│   │   │               └── sco_route.rs
│   │   ├── ipc/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── api.rs
│   │   │       ├── server.rs
│   │   │       ├── client.rs
│   │   │       ├── socket.rs
│   │   │       └── error.rs
│   │   └── testkit/
│   │       ├── Cargo.toml
│   │       └── src/
│   │           ├── lib.rs
│   │           ├── fake_phone.rs
│   │           ├── fake_ag.rs
│   │           ├── fake_audio_backend.rs
│   │           ├── fake_bluetooth_backend.rs
│   │           ├── fake_transport.rs
│   │           └── fixtures.rs
│   ├── daemon/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── app.rs
│   │       ├── config.rs
│   │       ├── ipc_service.rs
│   │       ├── logging.rs
│   │       ├── session_loop.rs
│   │       └── store.rs
│   └── ui/
│       ├── package.json
│       ├── tsconfig.json
│       ├── vite.config.ts
│       ├── svelte.config.js
│       ├── index.html
│       ├── src/
│       │   ├── main.ts
│       │   ├── App.svelte
│       │   ├── lib/
│       │   │   ├── ipc.ts
│       │   │   ├── state.ts
│       │   │   └── format.ts
│       │   ├── views/
│       │   │   ├── DialerView.svelte
│       │   │   ├── ActiveCallView.svelte
│       │   │   ├── HistoryView.svelte
│       │   │   ├── PairingView.svelte
│       │   │   └── SettingsView.svelte
│       │   └── components/
│       │       ├── DialPad.svelte
│       │       ├── CallControls.svelte
│       │       └── StatusBadge.svelte
│       └── src-tauri/
│           ├── Cargo.toml
│           ├── build.rs
│           ├── tauri.conf.json
│           ├── capabilities/default.json
│           └── src/
│               ├── main.rs
│               └── daemon_bridge.rs
└── tools/
    ├── gen-proto.sh
    ├── gen-proto.ps1
    ├── dev/
    │   ├── tier-a-smoke.sh
    │   └── tier-a-smoke.ps1
    └── usb-dongle-probe/
        ├── Cargo.toml
        └── src/main.rs
```

Generated (not hand-authored, not listed below): `android/gradlew`, `android/gradlew.bat`
(created by `gradle wrapper`), `desktop/ui/src-tauri/icons/` (bundler icon set), protobuf
codegen output, `Cargo.lock`, `node_modules/`, build directories.

## File inventory

### Root

- **`README.md`** — One-screen orientation: what Tandem is, the three planes, the tier model,
  repo map, emergency-call callout, pointer to `docs/00-overview.md`.
- **`CONTRIBUTING.md`** — How to add a doc or ADR; the two-line docstring/no-inline-comment rule.
- **`CLAUDE.md`** — Guidance for AI coding agents: the hard invariants, layout, coding standards,
  commit convention, and doc-ownership rules.
- **`LICENSE`** — Placeholder: license TBD, see ADR-0001.
- **`.gitignore`** — Ignore rules in three sections: Android/Gradle, Rust/Cargo, Node/Tauri.

### proto/ — single source of truth for wire types

- **`proto/tandem/v1/common.proto`** — Shared enums and value types for TLP v1.
  > Shared enums and value types for the Tandem LAN Protocol (TLP) v1. Single source of truth:
  > generated into Kotlin (protobuf-gradle-plugin) and Rust (prost via tandem_proto). Never
  > hand-duplicate these types.
- **`proto/tandem/v1/call.proto`** — Call-control requests and call-plane events.
  > Call-control requests (desktop -> phone) and call-plane events (phone -> desktop) for TLP
  > v1. All user intent flows over these messages; the desktop never issues HFP AT call-control
  > commands (see docs/05).
- **`proto/tandem/v1/calllog.proto`** — Call-history sync messages.
  > Call-history sync messages for TLP v1. The phone's OS call log is the source of truth;
  > desktops hold a read-only, incrementally synced projection.
- **`proto/tandem/v1/pairing.proto`** — First-pairing handshake messages.
  > First-pairing handshake messages for TLP v1. Runs inside a provisional TLS session
  > bootstrapped by the QR/short-code secret (see docs/07). The phone owns the paired-desktop
  > list and arbitrates acceptance.
- **`proto/tandem/v1/transport.proto`** — Session layer and the Envelope frame.
  > Session layer and the Envelope frame for TLP v1. Every WebSocket binary frame carries
  > exactly one Envelope; the oneof below is the complete message catalog. Version negotiation
  > happens in SessionHello/SessionWelcome.

### docs/

Seventeen numbered documents, this file, and ten ADRs — purposes are their own H1s; see
[00-overview.md](00-overview.md) for the reading order. (Docs carry no docstrings; they are the
documentation.)

### android/ — Tandem Gateway app (Kotlin, clean architecture)

Entries below beginning `…/` are relative to `android/app/src/main/kotlin/com/tandem/gateway/`;
`…/testkit/` entries are relative to `android/app/src/test/kotlin/com/tandem/gateway/testkit/`.

#### Build files

- **`android/settings.gradle.kts`** — Gradle settings: project name, module include, repos.
  > Gradle settings for the Tandem Gateway build: single :app module, pluginManagement and
  > dependencyResolutionManagement repositories.
- **`android/build.gradle.kts`** — Root build script registering plugin versions.
  > Root Gradle build script: declares AGP, Kotlin, Hilt, and protobuf plugin versions for the
  > single-module Android build. No build logic lives here.
- **`android/gradle.properties`** — JVM/AndroidX build flags.
  > Build-wide Gradle properties: JVM args, AndroidX flag, Kotlin code style. No secrets.
- **`android/gradle/libs.versions.toml`** — Version catalog for all Android dependencies.
  > Gradle version catalog: single place pinning AGP, Kotlin, Compose BOM, Hilt, Ktor, Room,
  > DataStore, protobuf, and test dependency versions.
- **`android/gradle/wrapper/gradle-wrapper.properties`** — Pins the Gradle wrapper version.
  > Gradle wrapper pin. Regenerate with `gradle wrapper`; do not edit the distribution URL by
  > hand except to bump versions.
- **`android/app/build.gradle.kts`** — App module build: SDK levels, Compose, Hilt, protobuf codegen.
  > App module build script: applies AGP/Kotlin/Hilt/protobuf plugins, sets minSdk 29 /
  > targetSdk 35, wires Compose, Room, Ktor, and generates Kotlin protobuf bindings from
  > /proto (see ADR-0009).
- **`android/app/proguard-rules.pro`** — R8 keep rules.
  > R8/ProGuard keep rules: protobuf generated classes, Ktor reflection points, Hilt generated
  > components. Keep minimal; prefer consumer rules from libraries.
- **`android/app/src/main/AndroidManifest.xml`** — Manifest: permissions, roles, services, filters.
  > Tandem Gateway manifest: dialer-role intent filters, TandemInCallService binding,
  > GatewayForegroundService with phoneCall|connectedDevice types, and the permission set
  > justified in docs/12-permissions-and-platform.md.
- **`android/app/src/main/res/values/strings.xml`** — User-visible strings.
  > All user-visible strings for the gateway app, including the emergency-call refusal copy
  > mandated by docs/08 and ADR-0008.
- **`android/app/src/main/res/values/themes.xml`** — Material theme bootstrap.
  > Material 3 theme bootstrap for Compose interop and the splash/incoming-call window styles.

#### Application root and DI

- **`…/gateway/TandemApplication.kt`** — Application entry; Hilt root; starts the foreground service.
  > Application root for the Tandem Gateway. Hosts the Hilt component graph and schedules
  > GatewayForegroundService startup when a paired desktop exists. No business logic; wiring
  > only.
- **`…/gateway/di/AppModule.kt`** — App-scoped bindings (clock, dispatchers, app context).
  > Hilt module for app-wide primitives: coroutine dispatchers, monotonic clock, and
  > application context providers consumed across layers. Bindings only; no logic.
- **`…/gateway/di/TelecomModule.kt`** — Binds telecom/dialer/bluetooth ports to impls.
  > Hilt module binding telephony-side ports: TelecomBridge to TelecomBridgeImpl,
  > CallMediaProvider to HfpCallMediaProvider, EmergencyNumberSource to
  > EmergencyNumberSourceImpl. Bindings only; no logic.
- **`…/gateway/di/TransportModule.kt`** — Binds transport/pairing/crypto ports to impls.
  > Hilt module binding LAN-side ports: LanServer to LanServerImpl, PairingManager to
  > PairingManagerImpl, IdentityStore to IdentityStoreImpl. Bindings only; no logic.
- **`…/gateway/di/DataModule.kt`** — Binds repositories and provides Room/DataStore.
  > Hilt module for persistence: provides TandemDatabase, DAOs, DataStore, and binds
  > CallLogRepository, PairedDeviceRepository, and SettingsRepository to their impls. Bindings
  > only; no logic.

#### domain/model — framework-free models

- **`…/domain/model/Call.kt`** — Call, CallState, CallDirection, DisconnectCause domain types.
  > Domain model of a live call: Call plus the CallState, CallDirection, and DisconnectCause
  > enums, mirroring Android Telecom semantics without framework types. Mapped to/from
  > tandem.v1 protos in the transport layer only.
- **`…/domain/model/CallLogEntry.kt`** — One call-history row.
  > Domain model of one call-log row as mirrored to desktops: number, cached display name,
  > type, start time, duration, SIM slot. Read-only projection of the OS call log.
- **`…/domain/model/PairedDesktop.kt`** — A trusted desktop's identity + metadata.
  > Domain model of a paired desktop: device id, display name, platform, pinned SPKI hash,
  > certificate bytes, optional Bluetooth MAC, timestamps, and revocation flag. The phone is
  > the authority for this set (ADR-0007).
- **`…/domain/model/AudioRoute.kt`** — Audio route enum + BT device address holder.
  > Domain model of the phone's call-audio route (earpiece, speaker, wired, Bluetooth with
  > device address). Mirrors android.telecom.CallAudioState routes without framework types.
- **`…/domain/model/DeviceIdentity.kt`** — This phone's identity key metadata.
  > Domain model of this phone's own identity: device id, display name, SPKI-SHA256
  > fingerprint, and certificate bytes. Private key material never leaves IdentityStore.

#### domain/port — interfaces over every I/O boundary

- **`…/domain/port/TelecomBridge.kt`** — Telephony control + observation port.
  > Port over Android Telecom: observe the authoritative call list as a Flow, and execute
  > answer/reject/end/hold/unhold/merge/mute/DTMF/dial commands. Implemented by
  > TelecomBridgeImpl; faked in tests. Contract in docs/11-api-reference.md.
- **`…/domain/port/CallMediaProvider.kt`** — Media-plane routing port (Tier abstraction seam).
  > Port over call-audio routing: request/observe the active audio route, including routing to
  > a specific Bluetooth device. Implemented today by HfpCallMediaProvider [Tier A/B]; a Tier C
  > vendor backend would implement the same port (ADR-0010).
- **`…/domain/port/LanServer.kt`** — Control-plane server port.
  > Port over the LAN control server: start/stop listening, observe inbound authenticated
  > requests, and fan events out to connected desktop sessions. Implemented by LanServerImpl;
  > faked by InMemoryLanServer in tests.
- **`…/domain/port/PairingManager.kt`** — Pairing lifecycle port.
  > Port over the pairing lifecycle: open/close a pairing window, expose the QR payload,
  > surface confirmation prompts, and finalize or reject a pairing candidate. Implemented by
  > PairingManagerImpl.
- **`…/domain/port/CallLogRepository.kt`** — Read-only call-history port.
  > Port over the OS call log: paged reads since a timestamp plus a Flow of change
  > notifications with a monotonic log version. Strictly read-only (no writes to the OS log).
- **`…/domain/port/PairedDeviceRepository.kt`** — Trusted-desktop persistence port.
  > Port over the paired-desktop store: CRUD for PairedDesktop rows, revocation flagging, and
  > lookup by pinned SPKI hash during TLS handshakes.
- **`…/domain/port/IdentityStore.kt`** — Identity-key custody port.
  > Port over identity-key custody: create-if-absent and expose this device's DeviceIdentity,
  > and sign TLS handshake material. Key material stays inside the implementation (Android
  > Keystore); callers only ever see public artifacts.
- **`…/domain/port/SettingsRepository.kt`** — User settings port.
  > Port over user settings (autostart, listening port, device display name) exposed as Flows
  > with suspend setters. Backed by DataStore in SettingsRepositoryImpl.
- **`…/domain/port/EmergencyNumberSource.kt`** — Emergency-number classification port.
  > Port answering "is this an emergency number right now" from current SIM/region data, and
  > exposing the current emergency-number list for sync to desktops. Consulted by
  > GuardEmergencyNumber before every dial (ADR-0008).

#### domain/usecase — one orchestration per user-facing capability

- **`…/domain/usecase/PlaceCall.kt`** — Dial a number after the emergency guard. `[Tier A]`
  > Use-case: place an outgoing call. Runs GuardEmergencyNumber, then delegates to
  > TelecomBridge.dial; returns a typed result the transport layer maps onto Ack statuses.
- **`…/domain/usecase/AnswerCall.kt`** — Answer with multi-desktop arbitration. `[Tier A]`
  > Use-case: answer a ringing call. Atomically arbitrates first-answer-wins across desktop
  > sessions against current telecom state, then delegates to TelecomBridge.answer.
- **`…/domain/usecase/RejectCall.kt`** — Decline a ringing call. `[Tier A]`
  > Use-case: reject a ringing call via TelecomBridge. Idempotence: rejecting a non-ringing
  > call yields InvalidCallState, never a crash.
- **`…/domain/usecase/EndCall.kt`** — Hang up a call. `[Tier A]`
  > Use-case: end an active, held, or dialing call via TelecomBridge.disconnect. Emergency
  > calls in progress are excluded (GuardEmergencyNumber policy; see docs/08).
- **`…/domain/usecase/SetMute.kt`** — Set absolute microphone mute. `[Tier A]`
  > Use-case: set the phone microphone mute state via TelecomBridge. Idempotent by design:
  > callers send the absolute target state, not a toggle.
- **`…/domain/usecase/HoldCall.kt`** — Put a call on hold. `[Tier A]`
  > Use-case: hold a call via TelecomBridge, honoring Call.can_hold capability. Holding an
  > already-held call is an OK no-op.
- **`…/domain/usecase/UnholdCall.kt`** — Resume a held call. `[Tier A]`
  > Use-case: unhold a call via TelecomBridge. Unholding an active call is an OK no-op.
- **`…/domain/usecase/MergeCalls.kt`** — Merge into a conference. `[Tier A]`
  > Use-case: merge two calls into a conference via TelecomBridge, honoring can_merge. Maps
  > telecom conference semantics onto the single is_conference flag desktops render.
- **`…/domain/usecase/SendDtmf.kt`** — Play DTMF digits into an active call. `[Tier A]`
  > Use-case: send a DTMF digit sequence into an active call via TelecomBridge, which plays
  > digits sequentially with standard Telecom tone timing.
- **`…/domain/usecase/RequestAudioRoute.kt`** — Route call audio (incl. to desktop HF). `[Tier B]`
  > Use-case: request an absolute audio route via CallMediaProvider, validating that Bluetooth
  > targets are bonded and that no emergency call is active. The LAN triggers routing; HFP
  > carries the audio (docs/05).
- **`…/domain/usecase/ObserveCallState.kt`** — The authoritative state stream. `[Tier A]`
  > Use-case: merge TelecomBridge call events, CallMediaProvider route changes, and mute state
  > into the versioned CallSnapshot stream (epoch_id, state_seq) that feeds every desktop
  > session and the handset UI alike.
- **`…/domain/usecase/SyncCallLog.kt`** — Paged history reads for desktops. `[Tier A]`
  > Use-case: serve CallLogSyncRequest pages from CallLogRepository and expose the current
  > log_version. Read-only; retention/refresh policy in docs/09-data-models.md.
- **`…/domain/usecase/PairDesktop.kt`** — Drive one pairing candidacy to a verdict.
  > Use-case: validate a PairingRequest token, await user confirmation via PairingManager,
  > persist the accepted desktop through PairedDeviceRepository, and produce the
  > PairingDecision payload.
- **`…/domain/usecase/RevokeDesktop.kt`** — Revoke a paired desktop immediately.
  > Use-case: flag a desktop revoked in PairedDeviceRepository and instruct LanServer to emit
  > RevokedEvent and close its live sessions. Takes effect before returning.
- **`…/domain/usecase/GuardEmergencyNumber.kt`** — The force-to-handset policy gate.
  > Use-case: classify a dial string via EmergencyNumberSource and refuse desktop-originated
  > emergency calls with EmergencyNumberBlocked (ADR-0008). Also flags active emergency calls
  > so remote control and audio-route requests are refused while one is live.

#### telecom — InCallService integration

- **`…/telecom/TandemInCallService.kt`** — The InCallService; telecom's callback surface. `[Tier A]`
  > android.telecom.InCallService implementation: receives Call objects and audio-state
  > callbacks while Tandem is the default dialer, forwards them to TelecomBridgeImpl, and
  > launches the handset in-call UI. No business logic in callbacks (docs/14 layering rule).
- **`…/telecom/TelecomBridgeImpl.kt`** — TelecomBridge over live telecom state. `[Tier A]`
  > TelecomBridge implementation: tracks Call objects registered by TandemInCallService, mints
  > stable call ids, executes control commands on the right Call, and emits domain call events.
  > The only class that touches android.telecom.Call directly.
- **`…/telecom/CallStateMapper.kt`** — telecom.Call → domain Call mapping. `[Tier A]`
  > Pure mapping from android.telecom.Call state, details, and capabilities to the domain Call
  > model, including DisconnectCause translation. Stateless; unit-tested exhaustively.

#### dialer — placing calls and dialer-role plumbing

- **`…/dialer/DefaultDialerManager.kt`** — ROLE_DIALER acquisition/status. `[Tier A]`
  > Wraps RoleManager: reports whether Tandem holds ROLE_DIALER and builds the role-request
  > intent for onboarding. The app is inert as a gateway until the role is granted (docs/12).
- **`…/dialer/OutgoingCallPlacer.kt`** — TelecomManager.placeCall wrapper. `[Tier A]`
  > Places outgoing calls via TelecomManager.placeCall (requires CALL_PHONE + ROLE_DIALER).
  > Invoked only by TelecomBridgeImpl after the emergency guard has passed.
- **`…/dialer/DialIntentRouter.kt`** — Handles ACTION_DIAL/tel: intents. `[Tier A]`
  > Routes external ACTION_DIAL and tel: intents into the handset dialpad UI with the number
  > prefilled, fulfilling the default-dialer contract. Never auto-places calls from intents.
- **`…/dialer/EmergencyNumberSourceImpl.kt`** — TelephonyManager-backed emergency data. `[Tier A]`
  > EmergencyNumberSource implementation over TelephonyManager.isEmergencyNumber and
  > getEmergencyNumberList, with a conservative static fallback (112/911) when telephony is
  > unavailable. Refreshes on SIM/carrier config change.

#### calllog — history mirroring

- **`…/calllog/CallLogRepositoryImpl.kt`** — CallLog provider reads. `[Tier A]`
  > CallLogRepository implementation querying android.provider.CallLog.Calls with paged,
  > timestamp-bounded projections (READ_CALL_LOG). Read-only by design; never writes or
  > deletes OS call-log rows.
- **`…/calllog/CallLogObserver.kt`** — Change detection + version bump. `[Tier A]`
  > ContentObserver on the CallLog provider: bumps the persisted monotonic log_version and
  > emits change notifications that become CallLogChangedEvent fan-outs.

#### transport — LAN control-plane server

- **`…/transport/LanServerImpl.kt`** — Ktor WS-over-mTLS listener. `[Tier A]`
  > LanServer implementation: Ktor (CIO) WebSocket endpoint over mutual TLS 1.3 built by
  > TlsServerFactory, accepting paired desktops (pinned SPKI) and provisional pairing sessions.
  > Delegates frames to DesktopSession; owns nothing about message semantics.
- **`…/transport/NsdAdvertiser.kt`** — mDNS/DNS-SD advertisement. `[Tier A]`
  > Advertises _tandem._tcp via NsdManager with TXT records for protocol version, device id,
  > and display name. Re-registers on network change; advertisement carries no secrets.
- **`…/transport/DesktopSession.kt`** — One connected desktop's session actor. `[Tier A]`
  > Per-connection session actor: performs SessionHello/SessionWelcome and Resume, tracks
  > (epoch_id, state_seq) delivery, applies per-session rate limits, and serializes outbound
  > events. One coroutine per session; no shared mutable state.
- **`…/transport/SessionRegistry.kt`** — Live session registry + fan-out. `[Tier A]`
  > Registry of live DesktopSessions: broadcast fan-out of call/log events, revocation
  > enforcement, and the atomic claim primitive AnswerCall uses for first-answer-wins
  > arbitration.
- **`…/transport/EnvelopeCodec.kt`** — Envelope ↔ domain mapping. `[Tier A]`
  > Encodes/decodes tandem.v1 Envelope frames and maps between generated proto types and
  > domain models. The only Android file that imports generated proto classes (ADR-0009).
- **`…/transport/WebSocketFraming.kt`** — RFC 6455 handshake and frame codec. `[Tier A]`
  > RFC 6455 handshake and frame codec for the gateway's WebSocket endpoint,
  > written against raw streams so the TLS socket can be created from an SSLContext
  > backed by non-exportable Android Keystore keys (ADR-0006). Pure byte
  > manipulation; no I/O policy and no protocol semantics.
- **`…/transport/ControlPlaneRouter.kt`** — Request dispatch to use-cases. `[Tier A]`
  > Routes decoded control-plane requests to their use-cases and maps results onto Ack/typed
  > responses. Pure dispatch: authentication happened at TLS accept, policy lives in
  > use-cases.

#### pairing — trust establishment

- **`…/pairing/PairingManagerImpl.kt`** — Pairing window + confirmation orchestration.
  > PairingManager implementation: opens a 120 s single-use pairing window, validates tokens,
  > drives the user confirmation sheet, and finalizes via PairDesktop. Enforces one pairing
  > candidate at a time.
- **`…/pairing/PairingSession.kt`** — One pairing candidate's state machine.
  > State machine for one pairing candidacy: TokenPresented, AwaitingConfirm (with optional
  > short-code comparison), Accepted, Rejected, Expired. Emits the PairingAwaitConfirmEvent
  > and PairingDecision payloads.
- **`…/pairing/QrPayloadCodec.kt`** — QR payload build/parse.
  > Builds the pairing QR payload (host, port, SPKI fingerprint, one-time token, name) and
  > renders it for display. Format is pinned in docs/07-pairing-and-auth.md; token TTL 120 s.
- **`…/pairing/DesktopOfferCodec.kt`** — Scanned desktop offer parsing.
  > Parses the pairing offer a desktop renders on screen and this phone scans with its camera:
  > version, the desktop's SPKI fingerprint to expect, the one-time token to accept, and the
  > name to show in the confirmation sheet. Mirror of tandem_pairing::DesktopOffer; the compact
  > keys are wire contract.
- **`…/pairing/QrImageAnalyzer.kt`** — Camera-frame QR decoding.
  > CameraX ImageAnalysis.Analyzer that decodes QR codes from the preview stream with ZXing,
  > reading the luminance plane directly so no bitmap is allocated per frame. Reports the first
  > successful decode once and then ignores the rest.

#### crypto — identity, certs, TLS

- **`…/crypto/IdentityStoreImpl.kt`** — Keystore-backed identity.
  > IdentityStore implementation over Android Keystore (StrongBox when available): generates
  > the non-exportable P-256 identity key on first run and exposes DeviceIdentity. Private key
  > operations never leave the Keystore.
- **`…/crypto/DeviceCertificates.kt`** — Self-signed device cert management.
  > Creates and persists the long-lived self-signed X.509 certificate wrapping the identity
  > key. Certificates are TLS carriers only; trust is pinned SPKI hashes, never chains
  > (ADR-0006).
- **`…/crypto/TlsServerFactory.kt`** — mTLS server context assembly.
  > Builds the TLS 1.3-only server context for LanServerImpl: presents the device cert,
  > requires client certs, and verifies peers against PairedDeviceRepository pins — accepting
  > unknown peers only into the provisional pairing path.
- **`…/crypto/Fingerprints.kt`** — SPKI hashing + short-code derivation.
  > Pure helpers: SPKI-SHA256 fingerprints, base64url rendering, and the 6-digit pairing short
  > code derived via HKDF from both SPKI hashes and the TLS exporter (docs/07). No I/O.

#### bluetooth — HFP AG-side coordination `[Tier B]`

- **`…/bluetooth/HfpAgMonitor.kt`** — Headset-profile connection observer. `[Tier B]`
  > Observes BluetoothHeadset profile state via BluetoothProfile proxy (BLUETOOTH_CONNECT):
  > which bonded devices have an HFP link, SCO audio state changes. The AG itself is Android's
  > Bluetooth stack — Tandem observes and steers, never reimplements it.
- **`…/bluetooth/HfpCallMediaProvider.kt`** — CallMediaProvider over InCallService routing. `[Tier B]`
  > CallMediaProvider implementation: executes AudioRouteRequest by calling
  > InCallService.setAudioRoute / requestBluetoothAudio toward the desktop's bonded HF device
  > and reports route reality from CallAudioState callbacks. Falls back to earpiece
  > automatically if SCO drops — the call itself is never touched (docs/05).
- **`…/bluetooth/BondedDesktopMatcher.kt`** — Maps paired desktops to bonded BT devices. `[Tier B]`
  > Resolves a paired desktop's stored BT MAC to a live BluetoothDevice among current bonds,
  > so routing targets the right HF. Reports unbonded desktops so UX can prompt Bluetooth
  > pairing.

#### service — process lifecycle

- **`…/service/GatewayForegroundService.kt`** — The long-lived gateway process host. `[Tier A]`
  > Foreground service (types phoneCall|connectedDevice) keeping the LAN server, NSD
  > advertisement, and telecom observation alive; legal for phoneCall type because Tandem
  > holds ROLE_DIALER. Doze/battery behavior documented in docs/02.
- **`…/service/GatewayNotifications.kt`** — Notification channels + builders. `[Tier A]`
  > Builds the persistent gateway status notification and its channel set (POST_NOTIFICATIONS)
  > showing connected desktops and audio-route state. Incoming-call notifications live in
  > IncomingCallNotifier, not here.
- **`…/service/BootCompletedReceiver.kt`** — Optional autostart hook. `[Tier A]`
  > BroadcastReceiver for BOOT_COMPLETED (RECEIVE_BOOT_COMPLETED): starts
  > GatewayForegroundService when the user has opted into autostart in settings. Disabled by
  > default.

#### data — persistence

- **`…/data/db/TandemDatabase.kt`** — Room database definition.
  > Room database (tandem.db, v1) hosting the paired-desktop table. Schema DDL and migration
  > policy documented in docs/09-data-models.md.
- **`…/data/db/PairedDesktopDao.kt`** — Paired-desktop table access.
  > Room DAO for paired_desktop rows: upsert, revoke-flag update, lookup by SPKI hash for TLS
  > accept, and an observable list for the settings UI.
- **`…/data/db/PairedDesktopEntity.kt`** — Room entity for a paired desktop.
  > Room entity mirroring domain PairedDesktop one-to-one (docs/09 schema). Mapping to domain
  > lives in PairedDeviceRepositoryImpl, keeping Room out of the domain layer.
- **`…/data/PairedDeviceRepositoryImpl.kt`** — Repository over the DAO.
  > PairedDeviceRepository implementation bridging PairedDesktopDao rows and domain models.
  > Owns the entity/domain mapping; enforces that revocation is a flag-set, never a hard
  > delete, so audit history survives.
- **`…/data/SettingsRepositoryImpl.kt`** — DataStore-backed settings.
  > SettingsRepository implementation over Preferences DataStore: autostart, port override,
  > device display name. Exposes Flows; all writes are suspend and transactional.

#### ui — Compose screens (status, pairing, settings, handset dialer + in-call)

- **`…/ui/MainActivity.kt`** — Single-activity Compose host. `[Tier A]`
  > Launcher activity hosting the Compose navigation graph (status, pairing, settings,
  > dialpad). Receives DialIntentRouter forwards; holds no state beyond navigation.
- **`…/ui/theme/Theme.kt`** — Compose theme.
  > Material 3 Compose theme (colors, typography, shapes) for all gateway screens, light and
  > dark.
- **`…/ui/status/StatusScreen.kt`** — Gateway status dashboard. `[Tier A]`
  > Compose screen showing gateway health: dialer-role status, LAN listener state, connected
  > desktops, BT audio state, and the emergency-policy notice. Renders StatusViewModel state;
  > no logic.
- **`…/ui/status/StatusViewModel.kt`** — Status screen state holder. `[Tier A]`
  > ViewModel deriving StatusScreen state from ObserveCallState, LanServer status, and
  > repositories. UI state only; commands delegate to use-cases.
- **`…/ui/pairing/PairingScreen.kt`** — QR scanner + confirmation sheet.
  > Compose screen for pairing: requests the camera, scans the code shown on the desktop,
  > reports progress while that desktop connects, and renders the accept/reject confirmation
  > sheet with its name and fingerprint.
- **`…/ui/pairing/QrScannerView.kt`** — CameraX preview for scanning.
  > Camera preview composable for scanning a desktop's pairing code: binds a CameraX preview
  > plus QrImageAnalyzer to the current lifecycle, releases the camera on disposal, and reports
  > the first decoded payload to its caller.
- **`…/ui/pairing/PairingViewModel.kt`** — Pairing flow state holder.
  > ViewModel driving PairingScreen from PairingManager events: scanning a desktop's pairing
  > code, the legacy show-a-code window, candidate arrival, short-code display, and verdict
  > submission.
- **`…/ui/settings/SettingsScreen.kt`** — Settings + paired-desktop management.
  > Compose screen for settings: paired desktop list with revoke actions, autostart toggle,
  > port override, device name. Revocation confirmation copy warns it is immediate.
- **`…/ui/settings/SettingsViewModel.kt`** — Settings state holder.
  > ViewModel binding SettingsScreen to SettingsRepository and RevokeDesktop. UI state only.
- **`…/ui/incall/InCallActivity.kt`** — Handset in-call window. `[Tier A]`
  > Activity shown over the lock screen for active calls (launched by TandemInCallService and
  > IncomingCallNotifier full-screen intent). Hosts InCallScreen; window flags only, no call
  > logic.
- **`…/ui/incall/InCallScreen.kt`** — Handset in-call controls. `[Tier A]`
  > Compose in-call UI on the handset: answer/reject/end, mute, hold, merge, DTMF pad, audio
  > route picker. The default-dialer contract requires this to be fully usable without any
  > desktop.
- **`…/ui/incall/InCallViewModel.kt`** — In-call state holder. `[Tier A]`
  > ViewModel projecting ObserveCallState snapshots into in-call UI state and dispatching
  > control actions through the same use-cases the LAN path uses — one command path for both
  > surfaces.
- **`…/ui/incall/IncomingCallNotifier.kt`** — Ringing notification/full-screen intent. `[Tier A]`
  > Posts the incoming-call notification (USE_FULL_SCREEN_INTENT + POST_NOTIFICATIONS) with
  > answer/decline actions and launches InCallActivity when ringing. The only surface allowed
  > to use a full-screen intent.
- **`…/ui/dialpad/DialpadScreen.kt`** — Handset dialpad. `[Tier A]`
  > Compose dialpad for placing calls from the handset, including numbers prefilled by
  > DialIntentRouter. Emergency numbers dial normally here — the handset is the sanctioned
  > emergency path (ADR-0008).
- **`…/ui/dialpad/DialpadViewModel.kt`** — Dialpad state holder. `[Tier A]`
  > ViewModel for DialpadScreen: dial-string editing and PlaceCall dispatch. Note the
  > emergency guard applies only to desktop-originated dials; handset dials pass through.

#### test — unit tests and deterministic fakes (test seams)

- **`…/transport/WebSocketFramingTest.kt`** — RFC 6455 codec tests.
  > Unit tests for the RFC 6455 handshake and frame codec: upgrade parsing, the
  > Sec-WebSocket-Accept derivation, masked client frames, extended payload
  > lengths, and the protocol violations the gateway must refuse.


- **`…/testkit/FakeTelecomBridge.kt`** — Scriptable telecom fake.
  > In-memory TelecomBridge fake: tests script call arrivals and state transitions and assert
  > on received commands. Backs use-case and router unit tests without Android Telecom.
- **`…/testkit/FakeCallMediaProvider.kt`** — Scriptable media-route fake.
  > In-memory CallMediaProvider fake: records route requests and lets tests simulate route
  > changes and SCO drops, including the fall-back-to-earpiece path.
- **`…/testkit/FakeCallLogRepository.kt`** — In-memory call-log fake.
  > In-memory CallLogRepository fake seeded with fixture entries; supports paging bounds and
  > log-version bumps for sync tests.
- **`…/testkit/FakePairedDeviceRepository.kt`** — In-memory trust-store fake.
  > In-memory PairedDeviceRepository fake for pairing, revocation, and TLS-pin lookup tests.
- **`…/testkit/FakeIdentityStore.kt`** — Deterministic identity fake.
  > IdentityStore fake with a fixed test keypair and fingerprint so pairing and TLS tests are
  > deterministic.
- **`…/testkit/FakeSettingsRepository.kt`** — In-memory settings fake.
  > In-memory SettingsRepository fake with mutable Flows for settings-dependent behavior
  > tests.
- **`…/testkit/InMemoryLanServer.kt`** — Loopback LanServer fake.
  > LanServer fake that connects in-process desktop sessions, letting protocol round-trip
  > tests run the real router/use-case path with no sockets or TLS.

### desktop/ — Rust workspace

#### Workspace roots

- **`desktop/Cargo.toml`** — Workspace manifest.
  > Cargo workspace for the Tandem desktop: member crates (proto, core, transport, pairing,
  > crypto, audio, bluetooth, ipc, testkit), the daemon and Tauri shell binaries, and shared
  > workspace dependency versions.
- **`desktop/rust-toolchain.toml`** — Pinned Rust toolchain.
  > Pins the Rust toolchain version and components (rustfmt, clippy) so all contributors and
  > CI build identically.

#### crates/proto — generated wire types

- **`desktop/crates/proto/Cargo.toml`** — Proto crate manifest.
  > Manifest for tandem_proto: prost/prost-build dependencies and the build-script hookup that
  > compiles /proto at build time (ADR-0009).
- **`desktop/crates/proto/build.rs`** — prost codegen from /proto.
  > Build script compiling proto/tandem/v1/*.proto with prost-build into Rust modules. The
  > repo-root proto directory is the only schema source; no vendored copies.
- **`desktop/crates/proto/src/lib.rs`** — Generated-module exports.
  > tandem_proto: re-exports the prost-generated tandem.v1 types as the single Rust wire-type
  > surface. No hand-written types beyond include! glue.

#### crates/core — domain + call-controller state machine

- **`desktop/crates/core/Cargo.toml`** — Core crate manifest.
  > Manifest for tandem_core: pure domain crate; depends only on tandem_proto and std
  > ecosystem basics — no I/O, no async runtime types in public APIs.
- **`desktop/crates/core/src/lib.rs`** — Crate root and public surface.
  > tandem_core: framework-free domain for the desktop — call mirror model, controller state
  > machine, reconciliation, and emergency pre-check. Everything here is deterministic and
  > unit-testable with no I/O (docs/14 layering).
- **`desktop/crates/core/src/model.rs`** — Domain models mirrored from phone truth.
  > Domain models of the mirrored call plane: Call, CallState, AudioRoute, CallLogRow,
  > PairedPhone, plus (epoch_id, state_seq) versioning. Converted from tandem.v1 protos at the
  > transport boundary only.
- **`desktop/crates/core/src/events.rs`** — Controller input/output event types.
  > Event vocabulary between transport and controller: inbound phone events (snapshot,
  > incoming call, route change, log change) and outbound UI-facing state deltas. Pure data;
  > no channels or runtime types.
- **`desktop/crates/core/src/controller.rs`** — The call-controller state machine.
  > CallController: consumes phone events and user commands, maintains the mirrored
  > CallSnapshot (phone is the source of truth — ADR-0007), and emits UI state plus outbound
  > requests. Pure transition function; side effects live in the daemon.
- **`desktop/crates/core/src/reconcile.rs`** — Resume/gap reconciliation rules.
  > Reconciliation after reconnect: compares (epoch_id, state_seq) against ResumeResponse,
  > decides snapshot-replace vs continue, and never lets stale mirror state override phone
  > truth.
- **`desktop/crates/core/src/emergency.rs`** — Client-side emergency pre-check.
  > Desktop-side emergency-number pre-check against the list synced from the phone: blocks
  > the dial locally with clear UX before any request is sent. Defense in depth — the phone
  > enforces the same policy authoritatively (ADR-0008).
- **`desktop/crates/core/src/error.rs`** — Core error enum.
  > CoreError: typed domain failures (unknown call id, invalid state for command, emergency
  > blocked, stale epoch) mapped from/to TLP Status codes at the boundaries.

#### crates/transport — LAN client

- **`desktop/crates/transport/Cargo.toml`** — Transport crate manifest.
  > Manifest for tandem_transport: tokio, tokio-tungstenite, rustls, mdns-sd, and tandem_proto
  > dependencies for the LAN control-plane client.
- **`desktop/crates/transport/src/lib.rs`** — Crate root; TransportClient trait.
  > tandem_transport: discovery, connection, and codec for TLP v1 over WebSocket + mutual TLS
  > 1.3. Exposes the TransportClient trait (docs/11) so core and tests never touch sockets
  > directly.
- **`desktop/crates/transport/src/discovery.rs`** — mDNS browse for _tandem._tcp.
  > Browses _tandem._tcp via mdns-sd, parses TXT records (version, device id, name), and
  > emits candidate endpoints — filtered against the paired phone's identity before any
  > connection attempt.
- **`desktop/crates/transport/examples/discover.rs`** — mDNS reachability probe.
  > Development probe: browses `_tandem._tcp` for a few seconds and prints the first phone it
  > finds, so mDNS reachability can be checked without pairing.
- **`desktop/crates/transport/src/client.rs`** — WS-over-mTLS session client.
  > TransportClient implementation: dials the phone endpoint with the pinned-peer TLS config,
  > performs SessionHello/SessionWelcome, then pumps Envelope frames bidirectionally with
  > heartbeats (5 s send / 15 s dead-peer).
- **`desktop/crates/transport/src/codec.rs`** — Envelope framing + correlation.
  > Encodes/decodes Envelope frames (one per binary WS message, 256 KiB cap), assigns
  > monotonic message_ids, and matches in_reply_to responses to pending requests with
  > timeouts.
- **`desktop/crates/transport/src/reconnect.rs`** — Backoff + resume orchestration.
  > Reconnect loop: exponential backoff (0.5 s to 30 s, jittered), immediate retry on
  > network-change signals, and ResumeRequest emission with the last seen (epoch_id,
  > state_seq, call_log_version) so core can reconcile (docs/10 flow h).
- **`desktop/crates/transport/src/tls.rs`** — rustls config with SPKI pinning.
  > Builds the rustls client config: TLS 1.3 only, presents the desktop device cert, and
  > verifies the server by pinned SPKI-SHA256 (pairing-bootstrap mode pins from the QR
  > payload instead). No WebPKI roots are consulted, ever.
- **`desktop/crates/transport/src/error.rs`** — Transport error enum.
  > TransportError: discovery, TLS/pinning, session-handshake, timeout, and protocol-violation
  > failures, with retryability annotations consumed by reconnect.

#### crates/pairing — trust bootstrap (desktop side)

- **`desktop/crates/pairing/Cargo.toml`** — Pairing crate manifest.
  > Manifest for tandem_pairing: depends on tandem_transport, tandem_crypto, and tandem_proto
  > for the first-pairing flow.
- **`desktop/crates/pairing/src/lib.rs`** — Crate root.
  > tandem_pairing: desktop side of first pairing — QR payload parsing, the pairing state
  > machine, and short-code derivation. Produces the persisted PairedPhone identity on
  > success (docs/07).
- **`desktop/crates/pairing/src/qr.rs`** — QR payload parsing.
  > Parses and validates the pairing QR payload (host, port, SPKI fingerprint, one-time
  > token, name); rejects unknown versions and malformed fingerprints before any network
  > activity.
- **`desktop/crates/pairing/src/offer.rs`** — Scan-to-pair offer + exchange.
  > Desktop-issued pairing offer: the payload this computer renders as a QR code for the
  > phone's camera, and the exchange that follows. The phone authenticates this desktop by
  > scanning its key fingerprint; this desktop learns the phone's key on connect and pins it
  > from then on, with the short code as the check against a machine in the middle (docs/07).
- **`desktop/crates/pairing/src/flow.rs`** — Pairing state machine.
  > Pairing flow driver: provisional TLS connect (pin from QR), PairingRequest submission,
  > PairingAwaitConfirmEvent handling, and PairingDecision finalization — persisting the
  > phone identity and this desktop's assigned device id.
- **`desktop/crates/pairing/src/short_code.rs`** — 6-digit comparison code.
  > Derives the 6-digit short authentication code via HKDF-SHA256 over both SPKI hashes and
  > the TLS exporter, byte-identical to the phone's Fingerprints implementation (docs/07).
- **`desktop/crates/pairing/src/error.rs`** — Pairing error enum.
  > PairingError: invalid QR, token expired, fingerprint mismatch, user rejection, and
  > version-negotiation failures, each mapped to actionable UI copy.

#### crates/crypto — identity + secrets (desktop side)

- **`desktop/crates/crypto/Cargo.toml`** — Crypto crate manifest.
  > Manifest for tandem_crypto: ring/rcgen for keys and certs, keyring for OS secret storage.
- **`desktop/crates/crypto/src/lib.rs`** — Crate root.
  > tandem_crypto: desktop device identity (P-256), self-signed certificate management, SPKI
  > pinning helpers, and OS-keychain-backed secret storage. Trust is pinned keys, never
  > chains (ADR-0006).
- **`desktop/crates/crypto/src/identity.rs`** — Device keypair lifecycle.
  > Creates the desktop's P-256 identity keypair on first run and loads it thereafter; the
  > private key lives in the OS secret store via secrets.rs, with an encrypted-file fallback.
- **`desktop/crates/crypto/src/cert.rs`** — Self-signed cert generation.
  > Generates the long-lived self-signed X.509 certificate over the identity key (rcgen) used
  > as the TLS carrier for mutual authentication.
- **`desktop/crates/crypto/src/pinning.rs`** — SPKI pin computation/verification.
  > SPKI-SHA256 fingerprint computation, base64url rendering, and constant-time pin
  > comparison used by transport TLS verification on both the pairing and paired paths.
- **`desktop/crates/crypto/src/secrets.rs`** — OS secret-store access.
  > Stores/loads identity-key material via the OS secret service (macOS Keychain, Windows
  > Credential Manager, Linux Secret Service) through keyring, with an encrypted-file
  > fallback for headless Linux sessions.
- **`desktop/crates/crypto/src/error.rs`** — Crypto error enum.
  > CryptoError: key-generation, secret-store access, certificate, and pin-verification
  > failures. Never carries key material in messages.

#### crates/audio — low-latency call audio `[Tier B]`

- **`desktop/crates/audio/Cargo.toml`** — Audio crate manifest.
  > Manifest for tandem_audio: cpal for device I/O, webrtc-audio-processing for AEC, plus
  > resampling dependencies. [Tier B]
- **`desktop/crates/audio/src/lib.rs`** — Crate root; AudioBackend trait export.
  > tandem_audio: microphone/speaker I/O for the HFP voice path — AudioBackend trait,
  > lock-free ring buffers, resampling, and echo cancellation. Consumes/produces 8 or 16 kHz
  > mono frames against the Bluetooth SCO clock. [Tier B]
- **`desktop/crates/audio/src/backend.rs`** — AudioBackend trait.
  > AudioBackend trait (docs/11): open capture/playback streams at a negotiated sample rate,
  > push/pull frames with bounded latency, report device changes. Implementations: cpal
  > (real), null (Tier B-lite / tests).
- **`desktop/crates/audio/src/cpal_backend.rs`** — Real device I/O via cpal.
  > AudioBackend implementation over cpal: device enumeration, stream setup at native rates,
  > and frame exchange with the pipeline through ring buffers. All OS-audio quirks
  > (WASAPI/CoreAudio/ALSA-PipeWire) isolate here. [Tier B]
- **`desktop/crates/audio/src/null_backend.rs`** — Silent no-op backend.
  > Null AudioBackend: accepts and discards frames, produces silence. Serves Tier B-lite
  > fallback builds and deterministic tests. [Tier B-lite fallback]
- **`desktop/crates/audio/src/ring_buffer.rs`** — Lock-free SPSC ring buffer.
  > Lock-free single-producer single-consumer ring buffer for audio frames between the
  > real-time OS callback and the SCO pump. Fixed capacity; overruns drop oldest and count,
  > never block the RT thread.
- **`desktop/crates/audio/src/resampler.rs`** — Rate conversion.
  > Resamples between device native rates and the HFP codec rate (8 kHz CVSD / 16 kHz mSBC)
  > with fixed latency budget; quality/latency tradeoffs documented inline in docs/05.
- **`desktop/crates/audio/src/aec.rs`** — Echo cancellation wrapper.
  > Wraps WebRTC AEC3 (webrtc-audio-processing): feeds far-end reference from the playback
  > path and near-end from capture so speakerphone use on the desktop does not echo into the
  > cellular uplink. [Tier B]
- **`desktop/crates/audio/src/pipeline.rs`** — Capture/playback graph assembly.
  > Assembles the duplex audio graph: capture → AEC → resample → SCO uplink, and SCO downlink
  > → resample → playback, with end-to-end latency accounting surfaced to the UI. [Tier B]
- **`desktop/crates/audio/src/error.rs`** — Audio error enum.
  > AudioError: device-unavailable, format-negotiation, stream, and xrun failures; states
  > which are recoverable by pipeline rebuild vs fatal to the audio session.

#### crates/bluetooth — HFP Hands-Free subsystem `[Tier B]`

- **`desktop/crates/bluetooth/Cargo.toml`** — Bluetooth crate manifest.
  > Manifest for tandem_bluetooth: zbus (BlueZ D-Bus) behind the linux_bluez feature, nusb
  > behind the usb_dongle feature, shared HFP core always compiled. [Tier B]
- **`desktop/crates/bluetooth/src/lib.rs`** — Crate root; backend selection.
  > tandem_bluetooth: the HFP Hands-Free unit — OS-independent HFP protocol core plus
  > pluggable backends (linux_bluez, usb_dongle, null). Implements the public Bluetooth SIG
  > HFP v1.8 spec; no product's proprietary protocol is involved (docs/05). [Tier B]
- **`desktop/crates/bluetooth/src/backend.rs`** — BluetoothBackend trait.
  > BluetoothBackend trait (docs/11): adapter lifecycle, bonding state, RFCOMM channel to the
  > AG, SCO audio open/close, and backend events. The seam that makes Tier B Linux, Tier B
  > dongle, Tier B-lite, and a future Tier C backend interchangeable (ADR-0010).
- **`desktop/crates/bluetooth/src/error.rs`** — Bluetooth error enum.
  > BluetoothError: adapter, bonding, RFCOMM, SCO, and HFP-protocol failures with
  > degradation guidance (audio loss never ends the call — docs/05).
- **`desktop/crates/bluetooth/src/hfp/mod.rs`** — HFP core module root.
  > OS-independent HFP v1.8 Hands-Free implementation: SLC bring-up, indicator tracking, and
  > codec negotiation as pure protocol logic over a byte channel supplied by a backend.
  > Call-control AT commands are deliberately not sent — LAN is the intent path (docs/05).
- **`desktop/crates/bluetooth/src/hfp/at.rs`** — AT command tokenizer/serializer.
  > Parser and serializer for the HFP AT command subset (BRSF, CIND, CMER, CIEV, BAC, BCS,
  > CLCC, CLIP, VGS, VGM and friends), line-discipline aware, tolerant of AG quirks.
- **`desktop/crates/bluetooth/src/hfp/slc.rs`** — Service-level connection state machine.
  > SLC establishment state machine per HFP v1.8 §4.2: BRSF exchange, CIND read, CMER enable,
  > CHLD query, then connected-idle. Emits typed SLC events; drives at.rs over the backend's
  > RFCOMM channel.
- **`desktop/crates/bluetooth/src/hfp/indicators.rs`** — AG indicator tracking.
  > Tracks AG indicators (call, callsetup, callheld, service, signal, battchg) from +CIEV and
  > periodic +CLCC polls, producing the HFP-view call state used for consistency checks
  > against LAN truth.
- **`desktop/crates/bluetooth/src/hfp/codec_negotiation.rs`** — CVSD/mSBC selection.
  > Wide-band speech negotiation: advertises mSBC via AT+BAC, answers +BCS codec selection,
  > and configures the SCO path for the agreed codec (CVSD fallback always supported).
- **`desktop/crates/bluetooth/src/hfp/call_mirror.rs`** — HFP-view vs LAN-truth reconciliation.
  > Compares the HFP indicator view of call state with the LAN CallSnapshot mirror, flags
  > divergence for logging/telemetry, and always resolves in favor of LAN truth (single-
  > command-path rule, docs/05).
- **`desktop/crates/bluetooth/src/backends/mod.rs`** — Backend registry/selection.
  > Compile-time and runtime backend selection: picks linux_bluez, usb_dongle, or null by
  > platform, feature flags, and configuration; exposes a uniform constructor to the daemon.
- **`desktop/crates/bluetooth/src/backends/null_backend.rs`** — No-op backend. `[Tier B-lite fallback]`
  > Null BluetoothBackend: reports no adapter and rejects audio-route attach cleanly, letting
  > the product run control-plane-only while the user pairs commodity earbuds directly to
  > the phone. [Tier B-lite fallback]
- **`desktop/crates/bluetooth/src/backends/linux_bluez/mod.rs`** — BlueZ backend root. `[Tier B — Linux]`
  > BluetoothBackend over BlueZ: adapter and bonding via org.bluez D-Bus, HFP HF profile
  > registration via Profile1, SCO via kernel sockets. Requires disabling PipeWire's native
  > HFP backend to avoid double-claiming the profile (docs/13). [Tier B — Linux]
- **`desktop/crates/bluetooth/src/backends/linux_bluez/profile.rs`** — Profile1 registration. `[Tier B — Linux]`
  > Registers the Hands-Free profile (UUID 0x111E) with BlueZ via ProfileManager1, receives
  > the RFCOMM fd for the SLC on NewConnection, and adapts it to the HFP core's byte-channel
  > interface.
- **`desktop/crates/bluetooth/src/backends/linux_bluez/sco.rs`** — SCO socket audio. `[Tier B — Linux]`
  > Opens and services BTPROTO_SCO sockets for call audio, honoring the negotiated codec
  > (CVSD/mSBC with transparent eSCO), and exchanges frames with tandem_audio ring buffers.
- **`desktop/crates/bluetooth/src/backends/usb_dongle/mod.rs`** — Dongle backend root. `[Tier B — Win/macOS USB dongle]`
  > BluetoothBackend driving a dedicated USB Bluetooth controller directly (bypassing the OS
  > stack, which does not expose the HF role to apps): full host stack from HCI up. Scoped to
  > one vetted controller family at a time (docs/05). [Tier B — Win/macOS USB dongle]
- **`desktop/crates/bluetooth/src/backends/usb_dongle/usb_transport.rs`** — USB HCI transport. `[Tier B — Win/macOS USB dongle]`
  > USB transport for HCI (interrupt/bulk/isochronous endpoints per the Bluetooth USB
  > transport spec) via WinUSB/IOKit through nusb; owns exclusive device claim and hotplug
  > detection.
- **`desktop/crates/bluetooth/src/backends/usb_dongle/hci.rs`** — HCI host layer. `[Tier B — Win/macOS USB dongle]`
  > Minimal HCI host: command/event flow, ACL and SCO data paths, controller init, inquiry/
  > paging, and connection management — only the subset HFP-HF requires.
- **`desktop/crates/bluetooth/src/backends/usb_dongle/l2cap.rs`** — L2CAP layer. `[Tier B — Win/macOS USB dongle]`
  > L2CAP channel management over ACL: signaling, fixed and dynamic channels, and the
  > single-session multiplexing RFCOMM and SDP need. No ERTM; basic mode only.
- **`desktop/crates/bluetooth/src/backends/usb_dongle/rfcomm.rs`** — RFCOMM layer. `[Tier B — Win/macOS USB dongle]`
  > RFCOMM (TS 07.10 subset) over L2CAP: multiplexer session, DLCI management, credit-based
  > flow control — enough to carry the HFP SLC byte stream.
- **`desktop/crates/bluetooth/src/backends/usb_dongle/sdp.rs`** — SDP records/queries. `[Tier B — Win/macOS USB dongle]`
  > SDP: publishes the Hands-Free service record (UUID 0x111E, RFCOMM channel) and queries
  > the AG's record for its channel number during connection setup.
- **`desktop/crates/bluetooth/src/backends/usb_dongle/security.rs`** — Bonding/link keys. `[Tier B — Win/macOS USB dongle]`
  > SSP bonding for the dongle path: numeric-comparison pairing with the phone, link-key
  > generation and encrypted storage via tandem_crypto secrets, and authentication/encryption
  > enforcement on the ACL.
- **`desktop/crates/bluetooth/src/backends/usb_dongle/sco_route.rs`** — SCO over USB. `[Tier B — Win/macOS USB dongle]`
  > Routes SCO audio over the controller's USB isochronous endpoints (HCI SCO packets),
  > pacing against the Bluetooth clock and bridging frames into tandem_audio ring buffers.

#### crates/ipc — daemon ⇄ UI contract

- **`desktop/crates/ipc/Cargo.toml`** — IPC crate manifest.
  > Manifest for tandem_ipc: serde/serde_json, ts-rs for TypeScript type export, and the
  > platform socket dependencies.
- **`desktop/crates/ipc/src/lib.rs`** — Crate root.
  > tandem_ipc: the daemon-to-UI contract — JSON-RPC 2.0 over a local socket, with request,
  > response, and event types defined once in api.rs and exported to TypeScript via ts-rs
  > (docs/11).
- **`desktop/crates/ipc/src/api.rs`** — The IpcApi type vocabulary.
  > IpcApi: every method (dial, answer, reject, end, mute, hold, unhold, merge, dtmf,
  > audio-route, history, pairing, settings, status) with its params, results, and event
  > payloads. Single source for both the Rust server and the generated TS client types.
- **`desktop/crates/ipc/src/server.rs`** — Daemon-side dispatcher.
  > JSON-RPC server: accepts one or more UI connections on the local socket, authenticates
  > same-user peers, dispatches to the daemon's service implementation, and pushes state
  > events.
- **`desktop/crates/ipc/src/client.rs`** — UI-side client.
  > JSON-RPC client used by the Tauri shell: request/response with timeouts, event
  > subscription, and automatic reconnect to a restarted daemon.
- **`desktop/crates/ipc/src/socket.rs`** — Platform socket abstraction.
  > Local-socket abstraction: Unix domain socket at $XDG_RUNTIME_DIR/tandem/daemon.sock and
  > Windows named pipe \\.\pipe\tandem-daemon, with same-user peer checks on both.
- **`desktop/crates/ipc/src/error.rs`** — IPC error enum.
  > IpcError: connect, protocol, timeout, and daemon-unavailable failures with UI-facing
  > retry guidance.

#### crates/testkit — desktop fakes

- **`desktop/crates/testkit/Cargo.toml`** — Testkit manifest.
  > Manifest for tandem_testkit: dev-dependency crate providing fakes and fixtures; never
  > shipped in release binaries.
- **`desktop/crates/testkit/src/lib.rs`** — Crate root.
  > tandem_testkit: deterministic fakes for every desktop I/O seam (transport, Bluetooth,
  > audio, phone peer, HFP AG) plus shared fixtures, backing the test pyramid in docs/15.
- **`desktop/crates/testkit/src/fake_phone.rs`** — Scripted TLP phone peer.
  > In-process fake of the phone gateway: speaks real TLP envelopes over an in-memory
  > transport, scriptable call scenarios (incoming, answer races, epoch bumps) for
  > integration tests without a device.
- **`desktop/crates/testkit/src/fake_ag.rs`** — Scripted HFP Audio Gateway.
  > Fake HFP AG speaking the AT protocol over an in-memory byte channel: drives SLC
  > bring-up, indicator sequences, codec negotiation, and SCO open/close for hfp core tests
  > (docs/15 integration tier).
- **`desktop/crates/testkit/src/fake_audio_backend.rs`** — Deterministic AudioBackend.
  > AudioBackend fake producing synthetic frames and capturing playback for assertion;
  > deterministic clocking for pipeline and latency tests.
- **`desktop/crates/testkit/src/fake_bluetooth_backend.rs`** — Scriptable BluetoothBackend.
  > BluetoothBackend fake: scripted adapter/bond/RFCOMM/SCO behavior including mid-call SCO
  > drops, backing controller and degradation tests.
- **`desktop/crates/testkit/src/fake_transport.rs`** — In-memory TransportClient.
  > TransportClient fake wired to fake_phone: connect/disconnect/resume scripting with
  > deterministic timing for reconnect and reconciliation tests.
- **`desktop/crates/testkit/src/fixtures.rs`** — Shared test data.
  > Canonical fixtures: sample CallSnapshots, call-log pages, QR payloads, certificates, and
  > keys used across unit and integration tests.

#### daemon — headless binary

- **`desktop/daemon/Cargo.toml`** — Daemon manifest.
  > Manifest for tandem-daemon: assembles core, transport, pairing, crypto, audio, bluetooth,
  > and ipc into the headless desktop service binary.
- **`desktop/daemon/src/main.rs`** — Entry point.
  > tandem-daemon entry point: parses CLI flags, loads config, initializes logging, and runs
  > the app supervisor until shutdown signal. No logic beyond bootstrapping app.rs.
- **`desktop/daemon/src/app.rs`** — Composition root + supervisor.
  > Composition root: constructs backends per platform/config (ADR-0010 selection), wires
  > controller, transport, audio, bluetooth, and IPC together with channels, and supervises
  > task lifecycles with graceful degradation (audio subsystem loss never kills control).
- **`desktop/daemon/src/config.rs`** — Config file + flags.
  > Loads and validates config.toml (paired-phone endpoint hints, backend selection, audio
  > devices, log level) with CLI overrides; documents every key in docs/09.
- **`desktop/daemon/src/ipc_service.rs`** — IpcApi implementation.
  > Implements the IpcApi surface over the live controller and subsystems: translates UI
  > method calls into controller commands and streams state events to connected UIs.
- **`desktop/daemon/src/logging.rs`** — tracing setup.
  > Initializes tracing subscribers (stderr + rolling file), with call metadata redaction in
  > release builds per the privacy policy in docs/08.
- **`desktop/daemon/src/session_loop.rs`** — LAN session supervision and reconnect.
  > Supervises the LAN session: connects to the paired phone, resumes the mirror
  > against phone truth, pumps events into the controller, and reconnects with
  > backoff when the link drops. Losing the link degrades the desktop to a stale
  > mirror; it never ends a call (ADR-0007).
- **`desktop/daemon/src/store.rs`** — SQLite mirror + identity persistence.
  > rusqlite-backed local store (tandem-cache.db): paired phone identity row, call-log mirror
  > with sync cursor, and settings not held in config.toml. Schema DDL in docs/09.

#### ui — Tauri shell + Svelte front-end

- **`desktop/ui/package.json`** — Front-end manifest.
  > npm manifest for the Tandem UI: Svelte + TypeScript + Vite toolchain and Tauri CLI
  > scripts (dev, build, tauri dev, tauri build).
- **`desktop/ui/tsconfig.json`** — TypeScript config.
  > TypeScript compiler options for the Svelte front-end: strict mode on, ES2022 target,
  > path alias to generated IPC types.
- **`desktop/ui/vite.config.ts`** — Vite build config.
  > Vite configuration: Svelte plugin, dev-server port for tauri dev, and build output
  > consumed by the Tauri bundler.
- **`desktop/ui/svelte.config.js`** — Svelte compiler config.
  > Svelte configuration: vitePreprocess for TypeScript in components. No SvelteKit; this is
  > a plain Vite + Svelte SPA inside Tauri.
- **`desktop/ui/index.html`** — SPA entry document.
  > Single-page entry for the Tauri webview: mounts App.svelte into #app; no external
  > resources (all assets bundled).
- **`desktop/ui/src/main.ts`** — Front-end bootstrap.
  > Front-end entry: instantiates App.svelte, initializes the IPC client connection to the
  > daemon, and installs global error reporting.
- **`desktop/ui/src/App.svelte`** — Root component + navigation.
  > Root component: view switching (dialer, active call, history, pairing, settings),
  > connection status header, and the emergency-notice surface required by ADR-0008 UX copy.
- **`desktop/ui/src/lib/ipc.ts`** — Daemon IPC client wrapper.
  > Typed wrapper over the JSON-RPC client for the daemon socket, using ts-rs-generated
  > types from tandem_ipc::api. The only module that talks to the daemon; views never do.
- **`desktop/ui/src/lib/state.ts`** — Front-end state stores.
  > Svelte stores derived from daemon events: mirrored call snapshot, connection state,
  > history cache, pairing progress. Read-only projections; commands go through ipc.ts.
- **`desktop/ui/src/lib/format.ts`** — Display formatting helpers.
  > Pure formatting helpers: phone-number display, call duration, timestamps, and BT/route
  > labels. No state, no IPC.
- **`desktop/ui/src/views/DialerView.svelte`** — Dialer screen. `[Tier A]`
  > Dialer view: number entry via DialPad, recent-call shortcuts, and dial dispatch. Shows
  > the emergency-block explanation when core/emergency refuses a number (ADR-0008).
- **`desktop/ui/src/views/ActiveCallView.svelte`** — Live-call screen. `[Tier A]`
  > Active-call view: caller identity, call timer, CallControls, DTMF pad, and the audio
  > route indicator with attach/detach-to-desktop action where a Tier B backend is present.
- **`desktop/ui/src/views/HistoryView.svelte`** — Call-history screen. `[Tier A]`
  > History view: the read-only mirrored call log with incremental loading and call-back
  > actions; displays the sync freshness state from state.ts.
- **`desktop/ui/src/views/PairingView.svelte`** — Pairing wizard.
  > Pairing view: QR-scan instructions and manual entry path, live pairing progress, the
  > 6-digit short-code comparison step, and success/failure outcomes.
- **`desktop/ui/src/views/SettingsView.svelte`** — Settings screen.
  > Settings view: paired phone identity and fingerprint display, audio device pickers,
  > Bluetooth backend status, autostart, and unpair (with the re-pairing consequence spelled
  > out).
- **`desktop/ui/src/components/DialPad.svelte`** — Dial pad component.
  > Reusable 12-key dial pad emitting digit events; used by DialerView for dialing and
  > ActiveCallView for DTMF. Presentation only.
- **`desktop/ui/src/components/CallControls.svelte`** — Call control buttons.
  > Reusable call-control cluster (mute, hold, merge, end) rendering capability-gated
  > buttons from the mirrored call state; emits intents upward, never calls IPC itself.
- **`desktop/ui/src/components/StatusBadge.svelte`** — Connection/route badge.
  > Small status badge for connection and audio-route states with accessible labels; used in
  > the header and settings.

#### ui/src-tauri — Tauri shell

- **`desktop/ui/src-tauri/Cargo.toml`** — Tauri shell manifest.
  > Manifest for the tandem-ui Tauri shell: tauri 2.x, tandem_ipc client dependency, and
  > bundler metadata.
- **`desktop/ui/src-tauri/build.rs`** — Tauri build script.
  > Standard tauri-build invocation generating the shell's compile-time context. Do not add
  > logic here.
- **`desktop/ui/src-tauri/tauri.conf.json`** — Tauri app config.
  > Tauri configuration: window defaults, bundle identifiers com.tandem.desktop, updater
  > disabled in v1, and CSP locked to bundled assets only.
- **`desktop/ui/src-tauri/capabilities/default.json`** — Tauri capability grants.
  > Tauri v2 capability file: minimal permission set for the main window (shell events,
  > window control); no filesystem or network capabilities — all I/O goes through the daemon
  > IPC.
- **`desktop/ui/src-tauri/src/main.rs`** — Shell entry point.
  > Tauri shell entry: creates the window, tray icon, and notification bridge, and spawns
  > daemon_bridge for IPC forwarding. Contains no call logic (docs/14 layering).
- **`desktop/ui/src-tauri/src/daemon_bridge.rs`** — Webview ⇄ daemon forwarder.
  > Bridges the webview and the daemon socket: forwards JSON-RPC requests from the front-end
  > via Tauri commands, streams daemon events to the webview, and manages daemon liveness
  > (spawn/reconnect prompts).

### tools/ — codegen and developer scripts

- **`tools/gen-proto.sh`** — Proto codegen (POSIX).
  > Regenerates protocol bindings on POSIX systems: runs protoc/gradle protobuf task checks
  > and cargo build -p tandem_proto so both languages compile from /proto in one step.
- **`tools/gen-proto.ps1`** — Proto codegen (Windows).
  > Windows equivalent of gen-proto.sh: verifies protoc availability and triggers Kotlin and
  > Rust generation from /proto.
- **`tools/dev/tier-a-smoke.sh`** — Tier A end-to-end smoke test (POSIX).
  > Scripted Tier A smoke test: discovers the phone via mDNS, confirms pairing, places a
  > test call to an operator-provided number, asserts CallStateChanged round-trip, and syncs
  > the call log. Exit code is the CI gate described in docs/13.
- **`tools/dev/tier-a-smoke.ps1`** — Tier A smoke test (Windows).
  > Windows equivalent of tier-a-smoke.sh with identical steps and exit semantics.
- **`tools/usb-dongle-probe/Cargo.toml`** — Dongle probe manifest.
  > Manifest for the usb-dongle-probe developer tool. [Tier B — Win/macOS USB dongle]
- **`tools/usb-dongle-probe/src/main.rs`** — Dongle capability probe.
  > Developer CLI probing a USB Bluetooth controller for Tandem compatibility: HCI version,
  > SCO-over-USB support, mSBC capability, and exclusive-claim viability; prints a
  > supported/unsupported verdict used in docs/13 bring-up. [Tier B — Win/macOS USB dongle]
