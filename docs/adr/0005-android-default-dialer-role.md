# ADR-0005: Android App Takes the Default-Dialer Role

## Context

To observe and control real SIM calls, an app needs Android Telecom to bind its
`InCallService`. Telecom binds an in-call UI service only for the app holding a dialer role — for
Tandem that is `ROLE_DIALER`, the user's default phone app (the car-mode dialer role is out of
scope). Without the role, `TandemInCallService` is never bound: no `Call` objects, no
answer/reject/hold/merge/mute/DTMF, no audio-route control, no authoritative state stream.
Outgoing calls go through `TelecomManager.placeCall` with `CALL_PHONE`, which the role turns into
a first-class dialer operation instead of a redirected intent. `[Tier A]` does not exist without
the role.

Alternatives considered and rejected:

- **`ConnectionService` + `MANAGE_OWN_CALLS`** — the wrong tool. Self-managed
  `ConnectionService` exists for apps hosting their own VoIP calls. Tandem drives
  carrier-managed SIM calls; for that, default-dialer + `InCallService` +
  `TelecomManager.placeCall` is sufficient and correct. Tandem implements no
  `ConnectionService` and never requests `MANAGE_OWN_CALLS`.
- **Notification-listener / `ANSWER_PHONE_CALLS` hacks** — partial coverage, no hold/merge/DTMF,
  no route control, fragile across OEMs, and contrary to the rule-abiding stance.

## Decision

Tandem Gateway requests `ROLE_DIALER` via `RoleManager` during onboarding and operates as the
user's default phone app. It therefore ships a complete handset dialer and in-call experience —
dialpad, lock-screen incoming-call UI, full in-call controls — fully usable with no desktop
present. The gateway is inert (no LAN control) until the role is granted.

## Status

Accepted.

## Consequences

- **Play Store policy cost.** Google Play restricts `READ_CALL_LOG` and related permissions to
  the user's default handler; the app must genuinely be a dialer to pass review, and store
  friction is expected. Details and the degradation matrix live in
  docs/12-permissions-and-platform.md.
- **User friction.** Users must replace the dialer they trust. Onboarding must say plainly what
  changes, and un-setting the role must cleanly restore the previous dialer with no data loss.
- **Scope cost.** The handset UX (`InCallActivity`, dialpad, incoming-call notifications) is
  mandatory, not optional — a substantial share of the Android UI surface exists to honor the
  role contract. The upside: it makes `[Tier A]` a complete, independently shippable product
  rather than a companion stub.
- **Sanctioned-API benefit.** Everything rides stable public APIs, and the `phoneCall` type in
  `android:foregroundServiceType="phoneCall|connectedDevice"` for `GatewayForegroundService` is
  legal precisely because the app holds `ROLE_DIALER` (see docs/03-android-app.md).
- Losing the role at runtime (user switches dialer) must drop the gateway to a clearly
  signalled inert state, never a half-working one.
