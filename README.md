# EHRbase-rs

[![CI](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange.svg?logo=rust)](rust-toolchain.toml)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg?logo=rust)](Cargo.toml)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-18-336791.svg?logo=postgresql&logoColor=white)](docs/VERSIONS.md)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)

[![EHRbase Logo](ehrbase.png)](ehrbase.png)

> [!IMPORTANT]
> **This is a hard fork of [`ehrbase/ehrbase`](https://github.com/ehrbase/ehrbase) — a from-scratch, pure-Rust reimplementation.**
> The upstream project is a Java/Spring Boot application. This fork re-implements the
> same [openEHR](https://www.openehr.org/) Clinical Data Repository in **Rust**, with no
> JVM, no `archie`/openEHR-SDK, and no ANTLR runtime. It is an active work in progress
> (see [Status](#status)); it is **not** a drop-in replacement for upstream EHRbase yet.

**EHRbase-rs** is a pure-Rust [openEHR](https://www.openehr.org/) Clinical Data Repository:
a standards-based backend for interoperable clinical applications. It targets the openEHR
Reference Model, the [openEHR REST API](https://specifications.openehr.org/releases/ITS-REST/latest/),
and model-based queries via the [Archetype Query Language (AQL)](https://specifications.openehr.org/releases/QUERY/latest/AQL.html) —
the same surface as upstream EHRbase, but built natively in Rust.

The goal, in one line:

> A faithful, 1:1, pure-Rust reimplementation of EHRbase that behaves identically at the
> openEHR REST API surface, backed by a natively-generated openEHR stack.

The authoritative plan is [`PORT_MASTER_PLAN.md`](PORT_MASTER_PLAN.md).

----

## Why a Rust fork?

- **Pure Rust, no JVM.** The Reference Model, serialization, terminology, ADL/AOM, and
  AQL are all implemented in Rust — no Java runtime and no foreign-language bindings in
  the running server.
- **Spec-driven, not hand-written.** The openEHR **specification** crates are *generated
  deterministically* from openEHR's machine-readable BMM meta-model (see
  [ADR-004](docs/ADRs/ADR-004-spec-driven-codegen.md)), so a spec-version bump is a
  re-run, not a rewrite.
- **Modern stack.** Rust 1.96 / edition 2024, `tokio` + `axum`, `sqlx` + `sea-query`,
  PostgreSQL 18.

## Status

> [!WARNING]
> Early-stage. The openEHR **spec layer** is generated and passing its fidelity gate;
> the **EHRbase application** port (REST, persistence, AQL engine) is underway. There is
> no runnable server yet. Per the port plan, phases P1–P16 are not expected to compile
> in full until the make-it-compile phase (P17).

What works today:

- **Generated openEHR spec crates** — `openehr-base` (BASE 1.3.0), `openehr-rm` (RM 1.2.0),
  `openehr-am` (AM 1.4.0 + 2.4.0), `openehr-term` (TERM 3.1.0), `openehr-lang` (LANG 1.1.0)
  — all compile clean and clippy-clean, emitted by `openehr-codegen` from the vendored BMM.
- **Fidelity gate** — the generated Reference Model **reads and losslessly round-trips**
  the real EHRbase / openEHR_SDK canonical-JSON corpus (`openehr-its/tests/fidelity.rs`).
- **AQL parser** — a hand-written `logos` + `chumsky` parser for the full AQL grammar
  (`openehr-query`), validated against the official example corpus.

## Architecture

Two families of crates, per [ADR-004](docs/ADRs/ADR-004-spec-driven-codegen.md):

| Prefix | Meaning | Source |
|---|---|---|
| `openehr-*` | the openEHR **specification** | generated from the vendored BMM meta-model (or hand-written runtime parsers) |
| `ehrbase-*` | the ported **EHRbase application** | ported file-by-file from the upstream Java |

```
crates/
├── openehr-codegen    # BMM → Rust emitter (the generator)
├── openehr-derive     # #[derive(OpenEhrType)] — canonical-JSON _type (de)serialization
├── openehr-base       # BASE: foundation + base types        ┐
├── openehr-rm         # RM: Reference Model                  │ generated
├── openehr-am         # AM: ADL/AOM (am14 + am24)             │ from BMM
├── openehr-term       # TERM: terminology (hand-written)     ┘
├── openehr-lang       # LANG: BMM / ODIN / EL object model
├── openehr-query      # QUERY: AQL lexer + parser (logos + chumsky)
├── openehr-its        # ITS: canonical JSON/XML + REST contract + fidelity gate
├── openehr-flat       # FLAT / STRUCTURED / Web Template (SDT)
├── ehrbase-rest       # openEHR REST API surface (axum)      ┐
├── ehrbase-compat     # EHRbase-specific endpoints, admin    │ ported from
└── ehrbase            # the server binary                    ┘ EHRbase Java
```

## Tech stack

| Layer | Choice |
|---|---|
| Language | Rust stable **1.96** (MSRV 1.96), **edition 2024** |
| Database | **PostgreSQL 18** (target 18.4+) |
| Web / async | `axum` 0.8, `tower`, `hyper` 1, `tokio` 1 |
| Persistence | `sqlx` 0.9 + `sea-query` 0.32 |
| Serialization | `serde` / `serde_json`, `quick-xml` |
| Parsers | `logos` (lexer) + `chumsky` (parser) — **no ANTLR runtime** |
| Codegen | `openehr-codegen` (BMM → Rust) + `openehr-derive` |

The authoritative, fully-pinned dependency set lives in the root `Cargo.toml`
`[workspace.dependencies]`; version pins are recorded in [`docs/VERSIONS.md`](docs/VERSIONS.md).

## Building and testing

You will need the Rust toolchain pinned in [`rust-toolchain.toml`](rust-toolchain.toml)
(stable 1.96, installed automatically by `rustup`). PostgreSQL 18 (via Docker or local)
is needed for the database integration tests.

```shell
# Build the workspace
cargo build --workspace

# Run the tests (unit + integration)
cargo nextest run --workspace
# Doctests
cargo test --workspace --doc

# Lint & format
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check

# Regenerate the openEHR spec crates from the vendored BMM meta-model
cargo run -p openehr-codegen -- emit
```

> [!NOTE]
> The `openehr-base` / `openehr-rm` / `openehr-am` / `openehr-term` / `openehr-lang`
> crates are **generated** (`// @generated … DO NOT EDIT`). To change one, edit the
> emitter (`crates/openehr-codegen/src/emit.rs`) or a sibling `*_impl.rs` and regenerate
> — never hand-edit a generated file. See [ADR-004](docs/ADRs/ADR-004-spec-driven-codegen.md).

## Documentation

- [`PORT_MASTER_PLAN.md`](PORT_MASTER_PLAN.md) — the authoritative port plan.
- [`docs/ADRs/ADR-004-spec-driven-codegen.md`](docs/ADRs/ADR-004-spec-driven-codegen.md) — why the spec layer is generated from BMM.
- [`docs/VERSIONS.md`](docs/VERSIONS.md) — the pinned language, database, and openEHR spec versions.
- [`docs/PORTING.md`](docs/PORTING.md) / [`docs/ROSETTA.md`](docs/ROSETTA.md) — the Java↔Rust and spec↔Rust mapping rules.

For openEHR concepts and the upstream reference implementation, see the
[EHRbase documentation](https://docs.ehrbase.org) and the
[openEHR specifications](https://specifications.openehr.org/).

## Relationship to upstream EHRbase

This fork tracks EHRbase as its behavioural reference (imported at v2.33.0) and aims for
parity at the openEHR REST surface. It deliberately pins the **latest** openEHR spec
versions (RM 1.2.0, BASE 1.3.0, AM 1.4.0 + 2.4.0), which diverge from the RM 1.1.0-era
wire format stock EHRbase emits — a known Stage-1 REST-parity consideration tracked in
[`docs/VERSIONS.md`](docs/VERSIONS.md). It is not affiliated with or endorsed by the
upstream EHRbase project or vitagroup.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md).
Code must be `cargo fmt` clean and pass `cargo clippy … -D warnings`; the same checks run
in CI on every pull request.

## Acknowledgments

This project is a fork of, and owes its architecture to, **EHRbase**, jointly developed by
[vitasystems GmbH](https://www.vitagroup.ag/) and the
[Peter L. Reichertz Institute (PLRI)](https://www.plri.de/). Upstream EHRbase in turn
contains code derived from EtherCIS and relies on the openEHR Reference Model implementation
[Archie](https://github.com/openEHR/archie) by Nedap.

The openEHR specifications and the machine-readable BMM meta-model that this project
generates from are published by the [openEHR Foundation](https://www.openehr.org/).

## License

EHRbase-rs is licensed under the [Apache License, Version 2.0](LICENSE), the same license
as upstream EHRbase.
