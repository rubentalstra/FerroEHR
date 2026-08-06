# Contributing to FerroEHR

Thank you for your interest in contributing. This document covers the practical
rules; the architectural ground rules live in
[`docs/architecture.md`](docs/architecture.md) and the root `CLAUDE.md`, and
the project's position — why it exists, and what it asks of the companies that
build on it — is
[Why FerroEHR exists](https://ferroehr.eu/docs/latest/why-ferroehr.html).

There is no contributor licence agreement and no copyright assignment: you keep
your copyright, and the licence stays MIT for everyone. Contributions are not
only code — a bug report with a reproducing request, a conformance case for
uncovered behaviour, a specification ambiguity you had to resolve, a
documentation correction, or measurement from your own hardware all count.

## Before you start

- **Read [`docs/architecture.md`](docs/architecture.md) first** — it explains
  the two-layer split (generated `openehr-*` spec crates, hand-written
  `ferroehr-*` application) every change must respect.
- For anything spec-facing (RM semantics, REST wire behaviour, AQL, canonical
  JSON/XML, templates, terminology), **the vendored openEHR specification text at
  [`docs/specs/openehr/`](docs/specs/openehr/) is the authority** — cite the spec
  file and section in your PR description. EHRbase and other CDRs are prior art,
  not oracles.

## Setup

- The Rust toolchain is pinned by [`rust-toolchain.toml`](rust-toolchain.toml);
  `rustup` installs it automatically on first build.
- Docker is required for the PostgreSQL 18 integration tests (testcontainers).
- `xmllint` (`libxml2-utils` on Debian/Ubuntu, part of libxml2 on macOS) is
  required by the canonical-XML parity tests.
- Install the shared git hooks once: `bash scripts/install-hooks.sh`.

## The gates (every PR must pass all of them)

```shell
cargo build --workspace
cargo nextest run --workspace          # unit + integration (real PG 18 via Docker)
cargo test --workspace --doc
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo deny check && cargo machete      # deny subsumes cargo-audit (same RustSec DB + yanked/licenses/bans/sources)
bash scripts/checks/codegen-drift.sh    # generated layer matches the vendored specs
```

CI runs the same set; nothing is advisory.

## Hard rules

- **Never hand-edit a generated file.** Everything under a `// @generated … DO NOT
  EDIT` header (the `openehr-base`/`openehr-rm`/`openehr-am` crates, the generated
  parts of `openehr-its`) is produced by `openehr-codegen`. Change the emitter (or
  a sibling `*_impl.rs`) and regenerate:
  `cargo run -p openehr-codegen -- emit` / `emit-xml` / `emit-rest`.
- **Never weaken, skip, or delete a test** to make a build pass, and never edit a
  test to route around a bug it exposes.
- Dependencies are added only from the root `Cargo.toml`
  `[workspace.dependencies]` (`dep.workspace = true`); don't hand-pin versions in
  member crates, and don't hand-roll what a pinned crate already provides.
- `thiserror` in library crates, `anyhow` only in the binary; no
  `unwrap`/`expect` outside tests.
- Application crates (`ferroehr-*`) consume the generated `openehr-*` types
  directly — never re-model the RM or re-serialize.

## Pull requests

- Branch from `develop`; PRs target `develop`.
- Keep PRs focused; describe **what** changed and **why**, citing spec sections
  for conformance-relevant behaviour.
- Tests accompany behaviour changes. Snapshot changes (`insta`) must be reviewed,
  not blindly accepted.
- Commit messages describe the change itself (conventional-commit style subjects
  like `feat: …`, `fix: …`, `docs: …` are used throughout the history).

## Reporting issues

Use the GitHub issue tracker. For suspected security vulnerabilities, **do not
open a public issue** — see [SECURITY.md](SECURITY.md).

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
