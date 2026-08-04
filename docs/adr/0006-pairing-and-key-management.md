# ADR-0006: Pairing and Key Management

## Context

Phone and desktops must mutually authenticate on a home/office LAN with no CA and no account
server. Requirements: strong per-device identity, a user-visible trust decision on the phone,
immediate revocation, and defense against man-in-the-middle at first contact. For the channel,
the realistic candidates were a Noise handshake (XX/IK) over TCP versus mutual TLS 1.3. Noise
is elegant and minimal, but it would mean hand-integrating a custom handshake into Ktor and
every desktop I/O path; TLS 1.3 is native in every stack Tandem already uses (Ktor/OkHttp,
rustls), and mTLS with key pinning delivers the same properties — mutual authentication and
forward secrecy — with far better tooling and reviewability.

## Decision

- **Identity**: each device generates a **P-256 keypair** at first run. Phone: Android Keystore
  (StrongBox when available), non-exportable. Desktop: OS secret store via `keyring` (macOS
  Keychain, Windows Credential Manager/DPAPI, Linux Secret Service), with an encrypted-file
  fallback.
- **Certificates are carriers only**: each device wraps its public key in a long-lived
  (3650-day) self-signed X.509 certificate used solely so TLS has something to present. Trust
  is **pinned SPKI-SHA256**, never certificate chains and never expiry.
- **Channel**: mutual TLS 1.3 only — no TLS 1.2 fallback. Noise rejected for the tooling
  reasons above.
- **Bootstrap**: QR is primary — the phone displays `{v, host, port, fp, tok, name}` where
  `fp` is its SPKI fingerprint and `tok` a 128-bit one-time token, TTL 120 s, single use. The
  manual short-code fallback additionally has both screens display a 6-digit code derived via
  HKDF-SHA256 over both SPKI hashes plus the TLS exporter binding, which the user compares.
- **Trust authority**: the phone owns the paired-desktop list, confirms each `PairingRequest`
  with the user, and assigns the `desktop_device_id` (UUIDv4) in `PairingDecision`.
- **Revocation**: immediate — the row is flagged revoked, any live session receives
  `RevokedEvent` and is closed, and future TLS handshakes from that SPKI are rejected. Desktop
  key loss means a fresh pairing with a new identity; the stale entry is revoked manually by
  the user (flow shown in docs/07-pairing-and-auth.md).

## Status

Accepted.

## Consequences

- No CA, no expiry lifecycle, no renewal outages. The tradeoff — long-lived certs — is honest
  because expiry does nothing when trust is a pinned key; rotation is re-pairing (desktop) or
  app reset (phone), covered in docs/08-security-and-encryption.md.
- MITM at first contact is defeated by the QR-carried fingerprint, or on the manual path by the
  short-code comparison bound to the actual TLS session via the exporter.
- Unknown SPKIs are only ever admitted into the provisional pairing path, and only while a
  pairing window is open; all other handshakes require a pinned, non-revoked peer.
- Bluetooth bonding is a **separate, optional** trust system `[Tier B — Linux]`
  `[Tier B — Win/macOS USB dongle]`: LAN pairing performs no bonding. The desktop reports its
  adapter address in `SessionHello.bt_adapter_address`, the phone stores it on the paired row, and
  the user completes standard Bluetooth bonding before any `AudioRouteRequest` can target that
  address. Holding one credential never confers the other.
- Exact persisted fields per paired device are specified in docs/07-pairing-and-auth.md with
  storage schema in docs/09-data-models.md.
