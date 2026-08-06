# 17 — Messaging and contact sources

Scope for two features requested after the call plane was working: choosing **where
contacts come from**, and **mirroring messages to the desktop**. Contact sourcing is
buildable now. Messaging is future scope, and this document exists mainly to record
which parts of it are reachable and which are walled off, so the design is not
attempted twice.

Read alongside [12-permissions-and-platform.md](12-permissions-and-platform.md) for
the permission model and [06-transport-and-protocol.md](06-transport-and-protocol.md)
for how a new plane would be framed.

---

## 1. Contact sources — a filter, not a sync

The instinct is that phone contacts, SIM contacts and Google contacts are three
places needing three integrations. They are not. `ContactsContract` already
aggregates all of them, and each raw contact carries the account it came from:

| Source            | `RawContacts.ACCOUNT_TYPE`                  |
| ----------------- | ------------------------------------------- |
| Google account    | `com.google`                                |
| SIM               | `com.android.contacts.sim` (OEM-dependent)  |
| Device-only       | `null` account type                         |
| WhatsApp, Telegram| their own types, e.g. `com.whatsapp`        |

So "sync from Google" needs **no Google API, no OAuth, no network**. The Google
contacts are already on the phone because Android syncs them; Tandem reads the
aggregated view. A source picker is therefore a **query filter** over
`RawContacts.ACCOUNT_TYPE`, not a second sync path.

**Design.** `ContactRepository.page` gains a source filter; the phone exposes the
account types it actually has (never a hardcoded list, since SIM account types vary
by OEM) so the UI offers only real choices. Default is every source combined, which
is what a dialer does.

This also means the earlier suggestion of adding Gmail OAuth would have been strictly
worse: same data, plus a Cloud project, plus verification, plus contact data over the
network.

### Sorting

Sort belongs to the query, not the list: `DISPLAY_NAME_PRIMARY` ascending is the
default, with recency (from the call-log mirror) and starred-first as alternatives.
Sorting client-side over a paged list would reorder only the page in hand, which
looks like a bug.

---

## 2. Messaging — what is reachable

### SMS and MMS: reachable, at a price

The `Telephony` provider exposes SMS/MMS to an app holding `READ_SMS`, and fully to
the app holding **`ROLE_SMS`** (the default SMS app). Reading conversations for
mirroring needs `READ_SMS`; sending needs the role or `SEND_SMS`.

The price is real and worth stating plainly:

- `READ_SMS` and `SEND_SMS` are in Google Play's
  [restricted permissions](https://support.google.com/googleplay/android-developer/answer/10467955)
  group. A Play listing must be an eligible *default SMS handler* to declare them, or
  the listing is rejected. Tandem is a dialer, not an SMS app, so claiming them
  changes what the product is.
- Mirroring message bodies over the LAN widens the blast radius of the pairing
  considerably. Today a compromised desktop can place calls; with messages it can
  read every OTP the user receives. That deserves its own threat-model entry and
  probably its own consent, separate from call control.

**Verdict:** technically buildable, but it makes Tandem an SMS app. Not a small
follow-on.

### RCS: not reachable

There is no public API for RCS/Jibe message content. Google Messages is the only
client. Nothing to build against.

### WhatsApp: not reachable, and this is the one that surprises people

WhatsApp has **no public API for reading messages on-device**. It is not a content
provider, its database is encrypted under app-private storage, and the Business API
is a cloud product for businesses messaging customers — it cannot read a user's
personal chats.

The only mechanism that observes WhatsApp messages is a
`NotificationListenerService`, which reads *notifications* rather than messages.
That approach:

- sees only what is currently notified — no history, nothing already dismissed,
  nothing while WhatsApp is muted;
- breaks whenever WhatsApp changes its notification shape;
- requires the highly privileged "notification access" grant, which lets the app read
  **every** notification on the phone, including other messengers and banking apps;
- is what many "sync my phone" apps quietly do, and is the reason they ask for that
  permission.

**Verdict:** reading WhatsApp conversations is out of scope. It cannot be done
properly, and the notification workaround costs more privacy than the feature is
worth.

### What *is* worth building instead: hand off, don't mirror

A recent-call row can offer **actions** that open the right app on the phone, without
Tandem reading anything:

| Action        | Mechanism                                                       |
| ------------- | --------------------------------------------------------------- |
| Send SMS      | `Intent.ACTION_SENDTO` with `smsto:<number>`                     |
| WhatsApp chat | `https://wa.me/<e164>` — opens WhatsApp if installed             |
| Video/voice   | the contact's own `ContactsContract.Data` rows for installed apps |

This is how a dialer normally integrates with messengers: **launch, never read.** It
needs no new permission, no message storage, and no protocol change. The desktop can
offer the same buttons by asking the phone to fire the intent — a small, honest
addition to the control plane.

---

## 3. If message mirroring is built later

Sketch only, so it is not designed ad hoc under time pressure.

**Wire types** (`proto/tandem/v1/messaging.proto`, envelope tags 70–75):

```text
MessageThread   { thread_id, address, display_name, snippet, updated_at_ms, unread }
MessageEntry    { message_id, thread_id, body, sent_at_ms, outgoing, read }
ThreadsSyncRequest / ThreadsSyncResponse    // paged like contacts
MessagesSyncRequest / MessagesSyncResponse  // paged within a thread
MessagesChangedEvent                        // nudge, mirroring CallLogChangedEvent
SendMessageRequest                          // desktop -> phone, requires ROLE_SMS
```

**Invariants it must respect**

1. **The phone stays the source of truth.** A thread mirror versions like the call-log
   mirror and reconciles on resume (ADR-0007); the desktop never holds authoritative
   message state.
2. **Message bodies are not cached on disk by default.** The call-log mirror is
   already bounded and cleared on unpair (docs/09); message bodies deserve
   memory-only treatment unless the user opts in, because a laptop at rest is a
   different threat model from a phone.
3. **Separate consent from call control.** Pairing grants call control. Reading
   messages must be a second, revocable grant, surfaced as such on the phone.
4. **Never send on the desktop's behalf without the role.** Without `ROLE_SMS` the
   phone cannot send; the desktop must be told that rather than silently failing.

**Order of work**, if it happens: intent hand-off first (no permissions, immediate
value) → SMS thread mirroring read-only → sending, only if Tandem becomes the default
SMS app.

---

## 4. Dialer shell this assumes

The home surface these features hang off, for reference:

- **Dial** — keypad, with recents above it that filter as the user types and collapse
  once the field is long enough to be a number.
- **History** — every call, newest first, each row offering call / message / WhatsApp.
- **Contacts** — every source combined by default, sortable, with the source picker in
  settings.

Nothing on this surface may require a paired computer: Tandem holds `ROLE_DIALER`, so
it has to be a complete phone app on its own (docs/12).
