# Sequence Diagrams

The ten canonical end-to-end flows of Tandem, one Mermaid `sequenceDiagram` each. Every named
message that crosses the LAN exists verbatim in `proto/tandem/v1/` — see
[06-transport-and-protocol.md](06-transport-and-protocol.md), Message Catalog — and travels as one
protobuf `Envelope` per WebSocket binary frame over mutual TLS 1.3. Arrows between the desktop UI
and the desktop daemon are JSON-RPC 2.0 IPC, not TLP (see [04-desktop-app.md](04-desktop-app.md)).
HFP and AT-command names follow [05-bluetooth-hfp.md](05-bluetooth-hfp.md).

## Conventions

- Participants carry their plane in the label, drawn from the canonical set: `User`,
  `Desktop UI [control]`, `Desktop Daemon [control]`, `Desktop HF (BT) [media]`,
  `Phone Gateway [control]`, `Android Telecom [cellular]`, `Phone BT stack (AG) [media]`,
  `Carrier [cellular]`.
- Solid arrows are requests and events; dashed arrows are responses. Every response `Envelope`
  sets `in_reply_to` to the request's `message_id`; correlation is elided except where arbitration
  depends on it.
- `Ack{X}` abbreviates an `Ack` whose `Status.code` is `X`. Braces list the proto fields that
  matter at that step; omitted fields keep proto3 defaults.
- Single-command-path rule (see [05-bluetooth-hfp.md](05-bluetooth-hfp.md)): all user intent
  travels over the LAN control plane. The desktop HF never issues HFP call-control AT commands
  (`ATA`, `AT+CHUP`, `ATD`, `AT+CHLD` as a user action, `AT+VTS`). The HFP link carries audio
  (SCO/eSCO), codec negotiation (`AT+BAC` / `+BCS`), indicator mirroring (`+CIEV`, `AT+CLCC`) as a
  consistency check, and volume sync (`AT+VGS` / `AT+VGM`).
- The phone is the source of truth (ADR-0007). An `Ack` only confirms that a request was accepted;
  the `CallSnapshot` inside `CallStateChangedEvent` is what the desktop renders.

| Flow | Scenario | Tier | Primary TLP messages |
|---|---|---|---|
| [a](#flow-a--app-launch-auto-discovery-connect) | Launch, discovery, connect | `[Tier A]` | `SessionHello`, `SessionWelcome`, `ResumeRequest`, `ResumeResponse`, `Heartbeat` |
| [b](#flow-b--pairing-qr-primary-manual-short-code-fallback) | First pairing | `[Tier A]` | `PairingRequest`, `PairingAwaitConfirmEvent`, `PairingDecision` |
| [c](#flow-c--outgoing-call-from-the-desktop) | Outgoing call | `[Tier A]` | `DialRequest`, `Ack`, `CallStateChangedEvent` |
| [d](#flow-d--incoming-cellular-call-surfaced-to-the-desktop) | Incoming call | `[Tier A]` | `IncomingCallEvent`, `AnswerRequest`, `CallStateChangedEvent` |
| [e](#flow-e--audio-attachdetach-over-hfp) | Audio attach and detach | `[Tier B — Linux]` `[Tier B — Win/macOS USB dongle]` | `AudioRouteRequest`, `AudioRouteChangedEvent` |
| [f](#flow-f--mute-hold-unhold-merge-end-round-trips) | In-call control | `[Tier A]` | `MuteRequest`, `HoldRequest`, `UnholdRequest`, `MergeRequest`, `EndRequest` |
| [g](#flow-g--call-log-sync) | History sync | `[Tier A]` | `CallLogChangedEvent`, `CallLogSyncRequest`, `CallLogSyncResponse` |
| [h](#flow-h--lan-reconnect-after-a-wi-fi-blip-phone-as-source-of-truth) | Reconnect and re-sync | `[Tier A]` | `ResumeRequest`, `ResumeResponse` |
| [i](#flow-i--multi-desktop-incoming-fan-out-and-answer-arbitration) | Fan-out and arbitration | `[Tier A]` | `IncomingCallEvent`, `AnswerRequest`, `Ack{ERROR_CODE_ALREADY_HANDLED}` |
| [j](#flow-j--emergency-number-attempt-from-the-desktop-forced-to-handset) | Emergency refusal | `[Tier A]` | `DialRequest`, `Ack{ERROR_CODE_EMERGENCY_NUMBER_BLOCKED}` |

## Flow a — App launch, auto-discovery, connect

`[Tier A]`. The daemon browses mDNS for `_tandem._tcp`, matches the TXT `id` against its stored
paired phone, and opens a mutual-TLS session in which both peers verify pinned SPKI-SHA256 hashes.
`SessionHello`/`SessionWelcome` negotiate the protocol version and hand the desktop the phone's
current `(epoch_id, state_seq)` head plus the emergency-number list. `ResumeRequest`/
`ResumeResponse` then initialize the mirror from the authoritative snapshot; the connection state
table is in [06-transport-and-protocol.md](06-transport-and-protocol.md).

```mermaid
sequenceDiagram
    participant U as User
    participant DU as Desktop UI [control]
    participant DD as Desktop Daemon [control]
    participant PG as Phone Gateway [control]

    U->>DU: launch tandem-ui
    DU->>DD: IPC connect over UDS or named pipe
    Note over PG: NsdAdvertiser has registered _tandem._tcp with TXT keys v, id, name
    DD->>PG: mDNS browse _tandem._tcp
    PG-->>DD: PTR, SRV and TXT answer, id equals the paired phone_device_id, port 46521
    Note over DD: TXT id matches the stored paired phone, so connect to the SRV target
    DD->>PG: TCP connect then TLS 1.3 mutual handshake
    Note over DD,PG: Each side verifies the peer against its pinned SPKI-SHA256, no CA is involved, and the phone refuses revoked pins
    DD->>PG: SessionHello{device_id, protocol_min, protocol_max, client_name, bt_adapter_address}
    PG-->>DD: SessionWelcome{status ERROR_CODE_OK, protocol_version, phone_device_id, phone_name, epoch_id, state_seq, call_log_version, emergency_numbers}
    Note over DD: emergency_numbers seeds the local pre-check used in flow j and is refreshed on every new session
    DD->>PG: ResumeRequest{last_epoch_id, last_state_seq, last_call_log_version}
    PG-->>DD: ResumeResponse{status ERROR_CODE_OK, snapshot_included true, snapshot, call_log_version}
    Note over DD: A fresh launch means an unknown or differing epoch, so the snapshot is included and replaces the mirror
    DD->>DU: IPC state push with calls, audio route and call-log version
    loop every 5 s each way, peer declared dead after 15 s of silence
        DD->>PG: Heartbeat{seq}
        PG-->>DD: HeartbeatAck{seq}
    end
```

If `SessionWelcome.status` is `ERROR_CODE_VERSION_UNSUPPORTED` the phone closes the session and the
desktop surfaces an upgrade prompt instead of retrying; if it is `ERROR_CODE_UNAUTHENTICATED` the
stored pairing is stale and the user must re-pair (flow b).

## Flow b — Pairing (QR primary, manual short-code fallback)

`[Tier A]`. Pairing runs inside a provisional TLS session bootstrapped by the QR payload's
fingerprint and one-time token (TTL 120 s, single use); on the manual path the user types host,
port and token instead, and trust is deferred to a 6-digit short code derived via HKDF-SHA256 over
both SPKI hashes and the TLS exporter binding. The phone owns the verdict: the user confirms on a
sheet showing the desktop's name and fingerprint. Payload format, persisted fields and revocation
are specified in [07-pairing-and-auth.md](07-pairing-and-auth.md).

```mermaid
sequenceDiagram
    participant U as User
    participant DU as Desktop UI [control]
    participant DD as Desktop Daemon [control]
    participant PG as Phone Gateway [control]

    U->>PG: open the Pairing screen and pick QR or manual mode
    Note over PG: PairingManager opens a 120 s single-use window, mints the one-time token and admits one candidate at a time
    alt QR path
        Note over PG: Phone renders the QR payload with v, host, port, fp, tok and name
        U->>DU: scan the QR on the desktop
    else manual short-code path
        Note over PG: Phone displays host, port and token for typing
        U->>DU: type host, port and token
    end
    DU->>DD: IPC start pairing with the parsed payload
    DD->>PG: TLS 1.3 connect, provisional session
    Note over DD,PG: QR path pins the phone cert against fp before proceeding. Manual path has no fingerprint yet, so trust rests on the short code. Either way the desktop presents its freshly generated P-256 device cert
    DD->>PG: PairingRequest{pairing_token, desktop_cert_der, desktop_name, desktop_platform, protocol_min, protocol_max}
    Note over PG: Token is valid, unexpired and unused, so the candidate is accepted for confirmation
    PG-->>DD: PairingAwaitConfirmEvent{require_short_code}
    opt require_short_code is true on the manual path
        Note over DU,PG: Both screens show the same 6-digit code and the user compares them before confirming
    end
    PG->>U: confirmation sheet with the desktop name and fingerprint
    U->>PG: tap Confirm
    PG-->>DD: PairingDecision{status ERROR_CODE_OK, desktop_device_id, phone_device_id, phone_name, protocol_version, phone_bt_address}
    Note over PG: Persist the paired_desktop row, schema in 09-data-models.md
    Note over DD: Persist phone device id and name, pinned SPKI hash, cert bytes, phone_bt_address and last endpoint
    Note over DD,PG: Normal sessions may now be opened, which is flow a
```

A rejected or expired candidacy yields `PairingDecision{status ERROR_CODE_PAIRING_REJECTED}` and
the provisional session closes; the token is burned either way. Bluetooth bonding for
`[Tier B — Linux]` and `[Tier B — Win/macOS USB dongle]` is a separate, later step and never gates
LAN pairing.

## Flow c — Outgoing call from the desktop

`[Tier A]`. A desktop dial clears the local emergency pre-check, then travels as `DialRequest` to
the phone, where `GuardEmergencyNumber` re-checks authoritatively and the per-session dial rate
limit applies before `TelecomManager.placeCall` runs. Each subsequent telecom transition fans out
as a `CallStateChangedEvent` carrying the full `CallSnapshot`, so the desktop converges with no
delta bookkeeping. The blocking path for emergency numbers is flow j.

```mermaid
sequenceDiagram
    participant U as User
    participant DU as Desktop UI [control]
    participant DD as Desktop Daemon [control]
    participant PG as Phone Gateway [control]
    participant TEL as Android Telecom [cellular]
    participant CAR as Carrier [cellular]

    U->>DU: enter a number and press Call
    DU->>DD: IPC dial with the dial string and SIM slot
    Note over DD: emergency.rs pre-check passes, the number is not on the synced emergency list
    DD->>PG: DialRequest{number, sim_slot}
    Note over PG: GuardEmergencyNumber passes because TelephonyManager.isEmergencyNumber is false, and the session is under the 5 dials per minute limit
    PG->>TEL: TelecomManager.placeCall with the tel URI
    PG-->>DD: Ack{ERROR_CODE_OK}
    TEL->>PG: onCallAdded into TandemInCallService, state CALL_STATE_CONNECTING
    PG->>DD: CallStateChangedEvent, snapshot has the call in CALL_STATE_CONNECTING
    TEL->>CAR: originate the SIM call over CS, VoLTE or VoWiFi
    TEL->>PG: onStateChanged to CALL_STATE_DIALING
    PG->>DD: CallStateChangedEvent, snapshot in CALL_STATE_DIALING
    CAR-->>TEL: remote party answers
    TEL->>PG: onStateChanged to CALL_STATE_ACTIVE
    PG->>DD: CallStateChangedEvent, snapshot in CALL_STATE_ACTIVE with started_at_ms set
    DD->>DU: IPC state push, active-call view
    Note over U,CAR: Under [Tier A] and [Tier B-lite fallback] the voice path stays on the handset or a headset paired to the phone. Flow e attaches desktop audio
```

Failure mapping: a refused or failed `placeCall` returns `Ack{ERROR_CODE_TELECOM_FAILURE}`, an
over-quota session returns `Ack{ERROR_CODE_RATE_LIMITED}`, and no `CallStateChangedEvent` follows
either. `DialRequest` is not idempotent — a retry after reconnect is deduplicated by
`message_id` (see [11-api-reference.md](11-api-reference.md)).

## Flow d — Incoming cellular call surfaced to the desktop

`[Tier A]`. Ringing is reported to `TandemInCallService`, raised on the handset by
`IncomingCallNotifier`, and fanned out to every authenticated session as `IncomingCallEvent`
followed by the ringing snapshot. Answering from the desktop is an `AnswerRequest` the phone
arbitrates before touching telecom. The contested two-desktop case is flow i.

```mermaid
sequenceDiagram
    participant CAR as Carrier [cellular]
    participant TEL as Android Telecom [cellular]
    participant PG as Phone Gateway [control]
    participant DD as Desktop Daemon [control]
    participant DU as Desktop UI [control]
    participant U as User

    CAR->>TEL: incoming call on the SIM
    TEL->>PG: onCallAdded into TandemInCallService, state CALL_STATE_RINGING
    Note over PG: IncomingCallNotifier posts the handset full-screen incoming-call UI in parallel, the handset stays fully usable on its own
    PG->>DD: IncomingCallEvent{call, epoch_id, state_seq}
    PG->>DD: CallStateChangedEvent, snapshot in CALL_STATE_RINGING
    DD->>DU: IPC incoming-call push
    DU->>U: desktop notification with caller display name or number, Answer and Decline actions
    U->>DU: click Answer
    DU->>DD: IPC answer with call_id
    DD->>PG: AnswerRequest{call_id}
    Note over PG: AnswerCall claims the call atomically, and with a single session the claim succeeds
    PG->>TEL: Call.answer with VideoProfile.STATE_AUDIO_ONLY
    PG-->>DD: Ack{ERROR_CODE_OK}
    TEL->>PG: onStateChanged to CALL_STATE_ACTIVE
    PG->>DD: CallStateChangedEvent, snapshot in CALL_STATE_ACTIVE
    DD->>DU: IPC state push, active-call view
```

Declining from the desktop is the same shape with `RejectRequest{call_id}`, ending in a
`CALL_STATE_DISCONNECTED` snapshot carrying `DISCONNECT_CAUSE_REJECTED`. An `AnswerRequest` for a
call that has already stopped ringing returns `Ack{ERROR_CODE_INVALID_CALL_STATE}`, and an unknown
id returns `Ack{ERROR_CODE_CALL_NOT_FOUND}`.

## Flow e — Audio attach/detach over HFP

`[Tier B — Linux]` `[Tier B — Win/macOS USB dongle]`. The LAN carries routing intent; HFP carries
the voice. `AudioRouteRequest` makes the phone call `InCallService.requestBluetoothAudio` toward
the desktop's bonded HF, after which Android's Bluetooth stack — the AG, never Tandem code —
selects a codec and opens eSCO, and `AudioRouteChangedEvent` reports the route that actually
happened. Under `[Tier B-lite fallback]` the desktop runs the null Bluetooth backend, this flow
never executes, and the user pairs a commodity Bluetooth speakerphone or earbuds to the phone
instead; SLC, codecs and degradation behavior are detailed in
[05-bluetooth-hfp.md](05-bluetooth-hfp.md).

```mermaid
sequenceDiagram
    participant U as User
    participant DU as Desktop UI [control]
    participant DD as Desktop Daemon [control]
    participant DHF as Desktop HF (BT) [media]
    participant PG as Phone Gateway [control]
    participant TEL as Android Telecom [cellular]
    participant AG as Phone BT stack (AG) [media]

    Note over DHF,AG: Precondition, the desktop adapter is bonded and the SLC came up at BT connect with AT+BRSF feature exchange, AT+CIND indicator map, AT+CMER reporting enabled and AT+BAC advertising CVSD plus mSBC
    Note over TEL,AG: Precondition, one call is in CALL_STATE_ACTIVE with audio on AUDIO_ROUTE_EARPIECE
    U->>DU: click Use this computer for audio
    DU->>DD: IPC set audio route to the desktop HF
    DD->>PG: AudioRouteRequest{route AUDIO_ROUTE_BLUETOOTH, bt_device_address}
    Note over PG: RequestAudioRoute checks that BondedDesktopMatcher resolves the address to a live bond and that no emergency call is active
    PG->>TEL: InCallService.requestBluetoothAudio for the desktop HF device
    PG-->>DD: Ack{ERROR_CODE_OK}, the route change is still pending
    TEL->>AG: hand call audio to the HFP device
    AG->>DHF: +BCS:2 selecting mSBC
    DHF->>AG: AT+BCS=2
    AG->>DHF: eSCO connection request in Transparent air mode
    DHF->>AG: accept, SCO link up
    DHF->>DD: SCO up at 16 kHz mono, start the duplex pipeline
    Note over DHF,AG: Voice flows in 7.5 ms frames. +CIEV and AT+CLCC mirror call state as a consistency check only, and AT+VGS with AT+VGM keep volume in sync
    TEL->>PG: onCallAudioStateChanged, route is Bluetooth
    PG->>DD: AudioRouteChangedEvent{route AUDIO_ROUTE_BLUETOOTH, bt_device_address, epoch_id, state_seq}
    DD->>DU: IPC route update, audio-on-desktop badge
    Note over U,AG: The user talks through desktop mic and speakers, with lock-free ring buffers and AEC3 in tandem_audio adding roughly 40 to 80 ms
    U->>DU: click Move audio back to the phone
    DU->>DD: IPC set audio route to earpiece
    DD->>PG: AudioRouteRequest{route AUDIO_ROUTE_EARPIECE}
    PG->>TEL: InCallService.setAudioRoute to the earpiece route
    PG-->>DD: Ack{ERROR_CODE_OK}, the route change is still pending
    TEL->>AG: release the HFP call-audio route
    AG->>DHF: SCO disconnect
    DHF->>DD: SCO down, stop the pipeline and flush the ring buffers
    TEL->>PG: onCallAudioStateChanged, route is earpiece
    PG->>DD: AudioRouteChangedEvent{route AUDIO_ROUTE_EARPIECE}
    Note over U,AG: Attach and detach move only where audio renders. The cellular call stays in Android Telecom throughout, and an unexpected SCO loss falls back to the earpiece rather than dropping the call
```

`AudioRouteRequest` is idempotent — it names an absolute target route, so a repeat during a pending
transition is an OK no-op. If the address is not bonded or the AG refuses the audio connection the
phone answers `Ack{ERROR_CODE_AUDIO_ROUTE_UNAVAILABLE}` and the call keeps its previous route.

## Flow f — Mute, hold, unhold, merge, end round-trips

`[Tier A]`. Each control command is one request, one `Ack`, and one or more `CallStateChangedEvent`
snapshots once telecom reality changes — the snapshot, not the `Ack`, is what the UI renders.
`MuteRequest`, `HoldRequest` and `UnholdRequest` carry absolute target state and are idempotent;
`MergeRequest` and `EndRequest` are not, and are deduplicated by `message_id` on retry (see
[11-api-reference.md](11-api-reference.md)). The UI-to-daemon IPC leg before each command is
identical to flow c and elided.

```mermaid
sequenceDiagram
    participant DD as Desktop Daemon [control]
    participant PG as Phone Gateway [control]
    participant TEL as Android Telecom [cellular]

    Note over DD,TEL: Precondition, call c1 is CALL_STATE_ACTIVE and a second call c2 is CALL_STATE_HOLDING by the merge step
    DD->>PG: MuteRequest{muted true}
    PG->>TEL: InCallService.setMuted true
    PG-->>DD: Ack{ERROR_CODE_OK}
    TEL->>PG: onCallAudioStateChanged, microphone muted
    PG->>DD: CallStateChangedEvent, snapshot has microphone_muted true

    DD->>PG: HoldRequest{call_id c1}
    PG->>TEL: Call.hold, honoring the can_hold capability
    PG-->>DD: Ack{ERROR_CODE_OK}
    TEL->>PG: onStateChanged to CALL_STATE_HOLDING
    PG->>DD: CallStateChangedEvent, snapshot has c1 in CALL_STATE_HOLDING

    DD->>PG: UnholdRequest{call_id c1}
    PG->>TEL: Call.unhold
    PG-->>DD: Ack{ERROR_CODE_OK}
    TEL->>PG: onStateChanged to CALL_STATE_ACTIVE
    PG->>DD: CallStateChangedEvent, snapshot has c1 in CALL_STATE_ACTIVE

    DD->>PG: MergeRequest{call_id c1, other_call_id c2}
    PG->>TEL: Call.conference with c2, honoring the can_merge capability
    PG-->>DD: Ack{ERROR_CODE_OK}
    TEL->>PG: conference established
    PG->>DD: CallStateChangedEvent, snapshot has is_conference true

    DD->>PG: EndRequest{call_id c1}
    PG->>TEL: Call.disconnect
    PG-->>DD: Ack{ERROR_CODE_OK}
    TEL->>PG: onStateChanged to CALL_STATE_DISCONNECTING
    PG->>DD: CallStateChangedEvent, snapshot has c1 in CALL_STATE_DISCONNECTING
    TEL->>PG: onCallRemoved
    PG->>DD: CallStateChangedEvent, snapshot has c1 in CALL_STATE_DISCONNECTED with DISCONNECT_CAUSE_LOCAL_HANGUP
```

A command whose capability flag is false — `HoldRequest` on a call with `can_hold` false,
`MergeRequest` with `can_merge` false — returns `Ack{ERROR_CODE_INVALID_CALL_STATE}` and produces
no snapshot. `SendDtmfRequest{call_id, digits}` follows the same request/`Ack` shape but emits no
snapshot, because DTMF changes no telecom state. The handset in-call UI dispatches through the same
use-cases, so both surfaces share one command path.

## Flow g — Call-log sync

`[Tier A]`. `CallLogObserver` watches the OS call-log provider, bumps the phone-persisted
`log_version`, and nudges every session with `CallLogChangedEvent`; desktops then page through
`CallLogSyncRequest`/`CallLogSyncResponse` at 200 entries per page. The desktop cache is a
read-only projection — no TLP message can write, edit or delete an OS call-log row. Retention and
refresh policy are in [09-data-models.md](09-data-models.md).

```mermaid
sequenceDiagram
    participant TEL as Android Telecom [cellular]
    participant PG as Phone Gateway [control]
    participant DD as Desktop Daemon [control]
    participant DU as Desktop UI [control]

    TEL->>PG: a call disconnects and the OS appends a CallLog row
    Note over PG: CallLogObserver fires and bumps the persisted log_version from 41 to 42
    PG->>DD: CallLogChangedEvent{log_version 42}
    Note over DD: The cached last_call_log_version is 41, so the mirror is stale and an incremental sync starts
    DD->>PG: CallLogSyncRequest{since_ms just after the newest cached entry, max_entries 200}
    PG-->>DD: CallLogSyncResponse{status ERROR_CODE_OK, entries, log_version 42, has_more true}
    loop while has_more is true
        DD->>PG: CallLogSyncRequest{since_ms advanced past the last received entry, max_entries 200}
        PG-->>DD: CallLogSyncResponse{status ERROR_CODE_OK, entries, log_version 42, has_more false}
    end
    Note over DD: Upsert CallLogEntry rows into the SQLite mirror by entry_id, then store log_version 42
    DD->>DU: IPC history updated
    Note over PG,DD: The same request path serves a cold first sync, with since_ms 0 and paging until has_more is false
```

If `READ_CALL_LOG` is denied the phone still answers `CallLogSyncResponse`, with a non-OK `Status`
and no `entries`, and the desktop shows history as unavailable while call control keeps working
(see [12-permissions-and-platform.md](12-permissions-and-platform.md)).

## Flow h — LAN reconnect after a Wi-Fi blip, phone as source of truth

`[Tier A]`. Fifteen seconds of silence — three missed heartbeats — declares the peer dead; the
desktop then backs off from 0.5 s doubling to 30 s with ±20 % jitter and retries immediately on an
OS network-change signal. The phone kept working throughout, so its `state_seq` advanced past what
the desktop last saw and the stale `ResumeRequest` forces a full snapshot that replaces the mirror
wholesale. Stale desktop state never overrides phone truth (ADR-0007).

```mermaid
sequenceDiagram
    participant DU as Desktop UI [control]
    participant DD as Desktop Daemon [control]
    participant DHF as Desktop HF (BT) [media]
    participant PG as Phone Gateway [control]

    DD->>PG: Heartbeat{seq 17}
    PG-->>DD: HeartbeatAck{seq 17}
    Note over DD,PG: Wi-Fi blip, frames stop flowing in both directions
    Note over DD: 15 s of silence, so the peer is declared dead and the mirror is marked stale
    DD->>DU: IPC status reconnecting, controls disabled
    Note over DHF: Bluetooth is a separate radio and plane, so an attached SCO link keeps carrying voice through the whole control outage
    Note over PG: The phone closes the dead session, the handset keeps working, and a call answered there advances state_seq from 52 to 58
    loop backoff 0.5 s doubling to 30 s max with plus or minus 20 percent jitter
        DD--xPG: TCP connect attempt fails
    end
    Note over DD: OS network-change signal on Wi-Fi restore triggers an immediate retry, re-resolving mDNS in case the phone address changed
    DD->>PG: TLS 1.3 mutual handshake, pins verified on both sides
    DD->>PG: SessionHello{device_id, protocol_min, protocol_max, client_name, bt_adapter_address}
    PG-->>DD: SessionWelcome{status ERROR_CODE_OK, protocol_version, epoch_id unchanged, state_seq 58, call_log_version, emergency_numbers}
    DD->>PG: ResumeRequest{last_epoch_id, last_state_seq 52, last_call_log_version}
    Note over PG: Same epoch but 52 lags the head of 58, so a gap is detected and the snapshot is included, exactly as a changed epoch_id would force
    PG-->>DD: ResumeResponse{status ERROR_CODE_OK, snapshot_included true, snapshot, call_log_version}
    Note over DD: reconcile.rs replaces the mirrored call plane from the snapshot instead of applying deltas
    DD->>DU: IPC state push, the UI converges on current reality
    loop heartbeats resume
        DD->>PG: Heartbeat{seq}
        PG-->>DD: HeartbeatAck{seq}
    end
```

A restarted gateway process mints a new `epoch_id`, so `ResumeResponse` includes a snapshot on that
basis alone and every desktop-side sequence counter resets. If `call_log_version` also moved, the
desktop chains into flow g after reconciling.

## Flow i — Multi-desktop incoming fan-out and answer arbitration

`[Tier A]`. The phone accepts multiple concurrent authenticated sessions and fans every call-plane
event out to all of them. The first valid `AnswerRequest` wins an atomic claim in `SessionRegistry`
arbitrated against current telecom state; later answers get
`Ack{ERROR_CODE_ALREADY_HANDLED}`. Both desktops then receive the resulting
`CallStateChangedEvent`, so the loser converges on the active call rather than guessing from its
error.

```mermaid
sequenceDiagram
    participant CAR as Carrier [cellular]
    participant TEL as Android Telecom [cellular]
    participant PG as Phone Gateway [control]
    participant DA as Desktop A Daemon [control]
    participant DB as Desktop B Daemon [control]

    CAR->>TEL: incoming call on the SIM
    TEL->>PG: onCallAdded into TandemInCallService, state CALL_STATE_RINGING
    par fan-out to every authenticated session
        PG->>DA: IncomingCallEvent{call, epoch_id, state_seq}
    and
        PG->>DB: IncomingCallEvent{call, epoch_id, state_seq}
    end
    Note over DA,DB: Both desktops ring, and so does the handset
    DA->>PG: AnswerRequest{call_id}
    Note over PG: AnswerCall wins the atomic claim on call_id, first valid answer wins
    PG->>TEL: Call.answer with VideoProfile.STATE_AUDIO_ONLY
    PG-->>DA: Ack{ERROR_CODE_OK}
    DB->>PG: AnswerRequest{call_id} moments later
    Note over PG: The claim is already held and telecom no longer reports the call as ringing
    PG-->>DB: Ack{ERROR_CODE_ALREADY_HANDLED}
    TEL->>PG: onStateChanged to CALL_STATE_ACTIVE
    par snapshot fan-out
        PG->>DA: CallStateChangedEvent, snapshot in CALL_STATE_ACTIVE
    and
        PG->>DB: CallStateChangedEvent, snapshot in CALL_STATE_ACTIVE
    end
    Note over DB: Desktop B dismisses its ringing UI and renders the active call, because the snapshot and not the Ack is the state authority
```

The same arbitration covers the handset: if the user answers on the phone, both desktops receive
`Ack{ERROR_CODE_ALREADY_HANDLED}` for any in-flight `AnswerRequest` and the active snapshot right
after. Audio can be attached from only one desktop at a time — a second `AudioRouteRequest`
retargets the route rather than duplicating the SCO link.

## Flow j — Emergency-number attempt from the desktop, forced to handset

`[Tier A]`. Both ends refuse desktop-originated emergency calls (ADR-0008): the desktop pre-checks
against the list synced in `SessionWelcome` and blocks locally, and the phone's
`GuardEmergencyNumber` is the authoritative backstop via `TelephonyManager.isEmergencyNumber`,
answering `Ack{ERROR_CODE_EMERGENCY_NUMBER_BLOCKED}` without ever invoking
`TelecomManager.placeCall`. A desktop-originated emergency call has no reliable caller location, so
it is never silently bridged. The policy as a safety control is in
[08-security-and-encryption.md](08-security-and-encryption.md).

```mermaid
sequenceDiagram
    participant U as User
    participant DU as Desktop UI [control]
    participant DD as Desktop Daemon [control]
    participant PG as Phone Gateway [control]
    participant TEL as Android Telecom [cellular]

    U->>DU: enter an emergency number such as 112 or 911 and press Call
    DU->>DD: IPC dial with the dial string
    alt normal case, the desktop pre-check catches it
        Note over DD: emergency.rs matches the number against SessionWelcome.emergency_numbers from flow a
        DD-->>DU: blocked locally, no DialRequest is ever sent
    else defense in depth, the local list was stale or incomplete
        DD->>PG: DialRequest{number, sim_slot}
        Note over PG: GuardEmergencyNumber finds TelephonyManager.isEmergencyNumber true and refuses before TelecomManager.placeCall is reached
        PG-->>DD: Ack{ERROR_CODE_EMERGENCY_NUMBER_BLOCKED}
        DD-->>DU: dial refused by the phone, same blocking UX
    end
    DU->>U: emergency block dialog that directs the user to the handset
    Note over U,TEL: The handset dialpad places emergency calls normally, it is the sanctioned path
    Note over PG,TEL: An emergency call already active on the handset is surfaced read-only with is_emergency true, so control requests and AudioRouteRequest are refused while it lives and the OS owns audio routing
```

The dialog copy is fixed in `strings.xml` on the phone and its desktop counterpart, and both say
the same thing:

> Emergency calls cannot be placed from this computer. Dial the number on your phone — it can share
> your location with emergency services.

Mid-session staleness of the synced list (a SIM swap, a region change) is acceptable precisely
because the phone-side guard is authoritative; the desktop pre-check exists to fail fast and to
give the user the handset instruction without a network round-trip.
