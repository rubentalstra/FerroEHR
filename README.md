# EHRbase-rs

[![CI](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange.svg?logo=rust)](rust-toolchain.toml)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg?logo=rust)](Cargo.toml)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-18-336791.svg?logo=postgresql&logoColor=white)](docs/VERSIONS.md)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)


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

> A pure-Rust, **openEHR-spec-conformant** CDR — a natively-generated openEHR
> stack (ITS-REST 1.0.3, AQL 1.1) with a modern idiomatic Rust application and
> greenfield PostgreSQL-18-native internals of our own design (ADR-008).

The roadmap lives in [`docs/plans/`](docs/plans/) + [`docs/PROGRESS.md`](docs/PROGRESS.md);
the design decisions are [ADR-004/005/006/008](docs/ADRs/). See
[`docs/architecture.md`](docs/architecture.md) for the full picture.

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
> Early-stage. The openEHR **spec + serialization + REST-contract foundation** is generated
> and passing its fidelity gates; the **EHRbase application** (REST, persistence, service,
> AQL engine, auth) is the remaining Stage-1 work and is built as compiling, tested
> increments (`docs/plans/` phases 09–20). There is no runnable server yet.

What works today (the generated foundation, all compiling + clippy-clean):

- **Generated spec crates** — `openehr-base` (BASE 1.3.0), `openehr-rm` (RM 1.2.0),
  `openehr-am` (AM 1.4.0 + 2.4.0), `openehr-term` (TERM 3.1.0), `openehr-lang` (LANG 1.1.0),
  emitted by `openehr-codegen` from the vendored BMM.
- **Canonical JSON + XML + the ITS-REST contract** — JSON `_type` self-tagging + validation,
  generated XML `ToXml`/`FromXml`, and generated ITS-REST DTOs/server-traits/routes, all in
  `openehr-its` (ADR-005).
- **Fidelity gates** — the generated RM **reads, losslessly round-trips, and validates**
  the real EHRbase / openEHR_SDK canonical-JSON corpus against the ITS-JSON schema; XML
  round-trips (48 compositions + real EHRbase XML fixtures). A `codegen-drift` CI job keeps
  the generated layer in sync with the specs.
- **AQL parser** — a hand-written `logos` + `chumsky` parser for the full AQL grammar
  (`openehr-query`), validated against the official example corpus.

## Architecture

Two families of crates (ADR-004/005/006):

| Prefix | Meaning | Source |
|---|---|---|
| `openehr-*` | the openEHR **specification + serialization + REST contract** | generated from the vendored BMM/XSD/OAS (plus hand-written runtimes + the AQL parser) |
| `ehrbase-*` | the **EHRbase application** | modern idiomatic Rust on the generated crates, with EHRbase's Java as the behavioural reference |

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
├── ehrbase-rest       # openEHR REST API surface (axum) + auth  ┐
├── ehrbase-compat     # EHRbase-specific endpoints, admin       │ idiomatic app on
└── ehrbase            # the server binary + service + AQL engine ┘ the openehr-* crates
```

## Tech stack

| Layer | Choice |
|---|---|
| Language | Rust stable **1.96** (MSRV 1.96), **edition 2024** |
| Database | **PostgreSQL 18** (target 18.4+) |
| Web / async | `axum` 0.8, `tower`, `hyper` 1, `tokio` 1 |
| Persistence | `sqlx` 0.9 + `sea-query` 1.0 (not sea-orm) |
| Auth | `jsonwebtoken`, `oauth2`, `openidconnect`, `argon2` — Basic + OAuth2/OIDC |
| Serialization | `serde` / `serde_json`, `quick-xml` |
| Parsers | `logos` (lexer) + `chumsky` (parser) — **no ANTLR runtime** |
| Codegen | `openehr-codegen` (BMM/XSD/OAS → Rust) + `openehr-derive` |

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

# Regenerate the openEHR spec + ITS layer from the vendored specs
cargo run -p openehr-codegen -- emit        # spec crates from BMM
cargo run -p openehr-codegen -- emit-xml    # canonical-XML impls (openehr-its)
cargo run -p openehr-codegen -- emit-rest   # ITS-REST contract (openehr-its)
```

> [!NOTE]
> The `openehr-base` / `openehr-rm` / `openehr-am` / `openehr-term` / `openehr-lang`
> crates are **generated** (`// @generated … DO NOT EDIT`). To change one, edit the
> emitter (`crates/openehr-codegen/src/emit.rs`) or a sibling `*_impl.rs` and regenerate
> — never hand-edit a generated file. See [ADR-004](docs/ADRs/ADR-004-spec-driven-codegen.md).

## Run with Docker

Two images (mirroring EHRbase's app + preconfigured-postgres model): the
`ehrbase` server and a PostgreSQL 18 image with the role, database, schemas, and
extensions pre-created. The quickstart [`docker-compose.yml`](docker-compose.yml)
builds and runs both from the [`docker/`](docker/) Dockerfiles:

```shell
docker compose up --build          # build + start both services

# The server is on http://localhost:8080; probe the public status endpoint:
curl http://localhost:8080/ehrbase/rest/status

# Create an EHR (Basic auth; dev default credentials ehrbase / ehrbase):
curl -u ehrbase:ehrbase -X POST -i \
  http://localhost:8080/ehrbase/rest/openehr/v1/ehr

docker compose down -v             # stop and remove the data volume
```

Published images: `ghcr.io/rubentalstra/ehrbase-rs` and
`ghcr.io/rubentalstra/ehrbase-rs-postgres`. The dev Basic-auth user
(`ehrbase`/`ehrbase`) comes from [`docker/ehrbase.dev.toml`](docker/ehrbase.dev.toml)
— **dev only**; configure real credentials (or OIDC) for production. The
postgres image bakes no migration state; the server runs its sqlx migrations at
boot (see [`docker/postgres/README.md`](docker/postgres/README.md)).

## Documentation

- [`docs/plans/`](docs/plans/) + [`docs/PROGRESS.md`](docs/PROGRESS.md) — the roadmap (what's done, what's next).
- [`docs/architecture.md`](docs/architecture.md) — the system design.
- [ADR-004](docs/ADRs/ADR-004-spec-driven-codegen.md) / [ADR-005](docs/ADRs/ADR-005-its-codegen.md) — why the spec + ITS layers are generated.
- [ADR-006](docs/ADRs/ADR-006-application-port-philosophy.md) — the application-layer philosophy (idiomatic Rust, not a 1:1 Java port; auth; stack).
- [`docs/VERSIONS.md`](docs/VERSIONS.md) + [`docs/postgres-features.md`](docs/postgres-features.md) — pinned versions + the PG 17/18 features we use.

For openEHR concepts and the upstream reference implementation, see the
[EHRbase documentation](https://docs.ehrbase.org) and the
[openEHR specifications](https://specifications.openehr.org/).

## Relationship to upstream EHRbase

This project began as an EHRbase fork (imported at v2.33.0) and keeps that history in
git, but since [ADR-008](docs/ADRs/ADR-008-greenfield-pg18-storage.md) its internals are
greenfield designs and its compatibility target is the **openEHR specifications** (the
CNF conformance framework), not EHRbase parity. EHRbase remains valued prior art. This
project pins the **latest** openEHR spec versions (RM 1.2.0, BASE 1.3.0, AM 1.4.0 +
2.4.0; see [`docs/VERSIONS.md`](docs/VERSIONS.md)). It is not affiliated with or
endorsed by the upstream EHRbase project or vitagroup.

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
