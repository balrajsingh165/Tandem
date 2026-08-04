# Contributing to Tandem

> **Licensing and contribution status.** No license has been chosen yet, so the default applies: this repository is all-rights-reserved. Outside **code** contributions therefore cannot be merged until a license and a contribution mechanism (a CLA or a DCO) are chosen — see [docs/adr/0001-licensing-and-project-name.md](docs/adr/0001-licensing-and-project-name.md). This file currently covers the documentation and ADR process.

## Adding a documentation file

1. Name it `docs/NN-kebab-case.md` using the next unused number (00–16 are taken; next is `17-`). Single H1 title, GitHub-flavored Markdown, tight prose — every sentence must inform a builder.
2. Diagrams are Mermaid only (`flowchart`, `sequenceDiagram`, `erDiagram`, `stateDiagram-v2`, `timeline`). In `flowchart` and `erDiagram`, quote node and edge labels that contain parentheses, slashes, or `#` so they render. Never quote `sequenceDiagram` participant aliases or message text, and never quote `stateDiagram-v2` transition labels — Mermaid renders those surrounding quotes literally; [docs/10-sequence-diagrams.md](docs/10-sequence-diagrams.md) is the canonical unquoted style.
3. Tier-tag every OS-, hardware-, or vendor-gated claim with the exact tags used across the docs: `[Tier A]` `[Tier B — Linux]` `[Tier B — Win/macOS USB dongle]` `[Tier B-lite fallback]` `[Tier C — needs vendor support]` (tier definitions: [docs/00-overview.md](docs/00-overview.md)).
4. Cross-reference sibling docs instead of restating them; each fact has one owning doc (proto text lives in docs/06, permission matrix in docs/12, threat table in docs/08, interface contracts in docs/11).
5. Register the new file in [docs/REPO-STRUCTURE.md](docs/REPO-STRUCTURE.md) and add it to the reading order in docs/00-overview.md.

## Adding an ADR

1. Name it `docs/adr/NNNN-kebab-title.md` with the next four-digit number (0001–0010 exist; next is `0011`).
2. Use exactly four sections after the H1: **Context**, **Decision**, **Status** (Proposed / Accepted / Superseded by NNNN), **Consequences**.
3. Never rewrite an accepted decision — write a new ADR that supersedes it and update the old ADR's Status line.
4. Link the ADR from every doc whose content it governs, and list it in docs/REPO-STRUCTURE.md.

## The docstring rule

Every source file carries exactly one file-level docstring at the top — KDoc `/** … */` for Kotlin, `//!` for Rust, JSDoc for TypeScript/Svelte, `<!-- … -->` for XML/HTML, `#` block for scripts/TOML — stating purpose, key public types, collaborators, and non-obvious constraints, with the text kept identical to the file's entry in docs/REPO-STRUCTURE.md.
No other narrative comments anywhere: bodies self-explain through naming and small functions, and code that seems to need an inline comment gets refactored instead (rationale and exemplars: [docs/14-coding-conventions.md](docs/14-coding-conventions.md)).
