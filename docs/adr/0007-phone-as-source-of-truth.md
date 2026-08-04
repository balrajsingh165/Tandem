# ADR-0007: Phone Is the Source of Truth for Call State

## Context

Call-plane reality lives in Android Telecom on the phone. Remote hangups, carrier events, and
handset-side user actions all originate there regardless of what any desktop believes. Multiple
desktops can be connected at once, and LAN links blip. Any design in which desktops hold
authoritative or negotiated state invites split-brain — a desktop rendering "held" after the
user unheld the call on the handset, or two desktops disagreeing about who answered.

## Decision

The phone gateway is the **single source of truth** for call-plane state and the call log; each
desktop holds a derived, disposable mirror. Mechanics:

- Every call-plane event carries `(epoch_id, state_seq)`: `epoch_id` is a UUID minted at each
  gateway process start; `state_seq` is monotonic within an epoch.
- `CallStateChangedEvent` carries the full `CallSnapshot`, not deltas, so desktops converge on
  every transition without delta bookkeeping.
- On every (re)connect the desktop sends
  `ResumeRequest{last_epoch_id, last_state_seq, last_call_log_version}`; the phone replies
  `ResumeResponse` with `snapshot_included = true` whenever the epoch differs or a gap is
  detected, and that snapshot **replaces** all desktop-side call state. Stale mirror state
  never overrides phone truth (`tandem_core::reconcile`).
- Desktop user actions are requests, not state mutations: the UI renders mirror state plus
  pending affordances, and any request may be refused with an `Ack` carrying an error `Status`.
- The call log follows the same model: a read-only projection versioned by `call_log_version`;
  the phone never writes its OS call log on a desktop's behalf.

## Status

Accepted.

## Consequences

- Multi-desktop is safe by construction: identical truth fans out to every session, and
  answer arbitration happens on the phone atomically against telecom state — first valid
  `AnswerRequest` wins, losers get `ERROR_CODE_ALREADY_HANDLED` plus the resulting
  `CallStateChangedEvent`.
- Desktop core code collapses to a pure mirror plus a command layer (`CallController` is a
  deterministic transition function); there is no conflict resolution and no reconciliation
  logic beyond snapshot-replace.
- Bandwidth cost of full snapshots is accepted: with at most a handful of concurrent calls, a
  `CallSnapshot` is far below the 256 KiB envelope cap.
- User-visible latency: desktop actions round-trip the LAN before the UI reflects them. The UI
  must show pending states honestly rather than optimistically mutating the mirror.
- The handset UI consumes the same authoritative stream through the same use-cases, so both
  surfaces can never disagree (see docs/01-architecture.md, data-ownership rules).
