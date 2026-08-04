# ADR-0008: Emergency Calls Are Forced to the Handset

## Context

Emergency calling depends on capabilities only the handset has: carrier and OS location
facilities, priority network handling, and a device physically with the caller. A
desktop-originated emergency call adds failure modes (LAN, session state, audio routing) and,
critically, has no reliable caller location. Silently bridging such a call — or letting it fail
somewhere between desktop and phone — is a safety hazard. The policy must hold even if one end
is buggy or stale.

## Decision

Tandem never places or manipulates emergency calls from the desktop.

- **Both-ends enforcement.** The desktop pre-checks every dial string against the
  emergency-number list the phone syncs to it and blocks locally with clear UX before any
  request is sent (`tandem_core::emergency`). The phone authoritatively checks every
  `DialRequest` against `TelephonyManager.isEmergencyNumber()` (via `GuardEmergencyNumber` and
  `EmergencyNumberSource`) and refuses matches with `ERROR_CODE_EMERGENCY_NUMBER_BLOCKED`. The
  desktop check is defense-in-depth and UX; the phone check is the guarantee.
- **Mandatory UX copy.** Every refusal, on both surfaces, must explicitly instruct the user to
  dial on the handset, which has carrier location facilities. This copy is a requirement, not a
  suggestion; the Android strings live in `res/values/strings.xml`.
- **Active emergency calls are read-only.** If an emergency call is live on the phone (placed
  from the handset), Tandem surfaces it with `CallInfo.is_emergency = true` only: remote
  control commands are refused, `AudioRouteRequest` is refused, and the OS owns audio routing.
- **The handset path is untouched.** Emergency numbers dial normally from the handset dialpad —
  the handset is the sanctioned emergency path; the guard applies only to desktop-originated
  intent.

## Status

Accepted.

## Consequences

- The guard sits in front of `TelecomBridge.dial` for every desktop-originated call, and the
  same use-case flags live emergency calls so route and control requests fail closed while one
  is active.
- `EmergencyNumberSourceImpl` needs a conservative static fallback (112/911) for moments when
  telephony data is unavailable, and refreshes on SIM/carrier-config change — refusing a
  non-emergency number is acceptable; missing an emergency number is not.
- Keeping the desktop's emergency-number list current adds a small sync surface; the phone-side
  authoritative check means a stale desktop list degrades UX (later refusal) but never safety.
- The policy is a named safety control in the threat model (docs/08-security-and-encryption.md)
  and is restated in README, docs/00-overview.md, docs/02-feasibility-and-constraints.md, and
  docs/16-roadmap.md; flow (j) in docs/10-sequence-diagrams.md shows the refusal end-to-end.
