# ADR-0001: Licensing and Project Name

## Context

The initial documentation pass needs a stable project name, and the repository needs a `LICENSE`
file. The name appears in package identifiers (`com.tandem.gateway`), crate names (`tandem_*`),
the protocol name (Tandem LAN Protocol, TLP), and the mDNS service type (`_tandem._tcp`), so it
must be fixed now even though no trademark clearance has been done.

The license choice is genuinely open. It depends on decisions not yet made: distribution channels
(Google Play for the gateway, per-OS packaging for the desktop), whether external contributions
will be accepted and under what agreement, and license-compatibility constraints from
dependencies in the Tier B media path (for example the WebRTC AEC3 component named in
docs/04-desktop-app.md). Picking a license casually now would be hard to reverse once external
contributions exist, because relicensing then requires every contributor's consent.

## Decision

- **Tandem** is the working codename, used consistently everywhere: repo, docs, protocol name,
  Android package, Rust crates, service type. It is not trademark-cleared; a rename before any
  public release is possible and is treated as a mechanical find-and-replace plus re-pairing
  (the service type and package identifiers change with it).
- License selection is **deliberately deferred**. The root `LICENSE` file is a placeholder that
  states only that the license is to be determined and points to this ADR. It grants no rights
  and invents no terms.
- The deferral has an expiry: a license must be chosen before the first public binary release or
  before merging the first external contribution, whichever comes first. The choice at that
  point also decides the contribution mechanism (CLA or DCO).

## Status

Accepted. (What is accepted is the deferral itself; the license remains open. This is the only
sanctioned "TBD" in the Tandem documentation set.)

## Consequences

- Until a license is chosen, default copyright applies: the code is all-rights-reserved and
  external contributions must not be merged, since no inbound-rights agreement exists.
- Every document may reference "Tandem" freely; none may state or imply license terms. Anything
  describing reuse rights must point here.
- The rename risk is contained: identifiers derived from the name are enumerated above, so a
  future rename is a bounded, mechanical change while the project is pre-release.
- `CONTRIBUTING.md` documents process (how to add docs and ADRs) but cannot promise that outside
  code will be accepted until this ADR's deferral is resolved.
