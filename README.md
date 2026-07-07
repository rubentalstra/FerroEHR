# EHRbase-rs

**A pure-Rust openEHR Clinical Data Repository** — ITS-REST 1.0.3 at the API,
AQL 1.1 as the query language, PostgreSQL-18-native storage designed from
first principles.

[![CI](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/ci.yml)
[![Containers](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/containers.yml/badge.svg?branch=develop)](https://github.com/rubentalstra/ehrbase-rs/actions/workflows/containers.yml)
[![Last commit](https://img.shields.io/github/last-commit/rubentalstra/ehrbase-rs/develop?logo=github)](https://github.com/rubentalstra/ehrbase-rs/commits/develop)
[![Open issues](https://img.shields.io/github/issues/rubentalstra/ehrbase-rs?logo=github)](https://github.com/rubentalstra/ehrbase-rs/issues)

[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange.svg?logo=rust)](rust-toolchain.toml)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg?logo=rust)](Cargo.toml)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-18-336791.svg?logo=postgresql&logoColor=white)](docs/VERSIONS.md)
[![openEHR](https://img.shields.io/badge/openEHR-ITS--REST_1.0.3_%C2%B7_AQL_1.1-1F6FEB.svg)](https://specifications.openehr.org/)
[![GHCR](https://img.shields.io/badge/ghcr.io-ehrbase--rs-2496ED.svg?logo=docker&logoColor=white)](https://github.com/rubentalstra/ehrbase-rs/pkgs/container/ehrbase-rs)

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Security policy](https://img.shields.io/badge/security-policy-yellow.svg)](SECURITY.md)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)

> [!IMPORTANT]
> This is a hard fork of [`ehrbase/ehrbase`](https://github.com/ehrbase/ehrbase),
> reimplemented from scratch in Rust — no JVM, no `archie`/openEHR-SDK, no ANTLR
> runtime. Its conformance target is the **openEHR specifications** (verified by
> the official CNF conformance framework), not bug-for-bug parity with upstream.
> Pre-release software: not yet production-ready (see [Status](#status--roadmap)).

An [openEHR](https://www.openehr.org/) CDR is a standards-based backend for
interoperable clinical applications: applications store and query structured
clinical data through the vendor-neutral
[openEHR REST API](https://specifications.openehr.org/releases/ITS-REST/latest/)
and the [Archetype Query Language](https://specifications.openehr.org/releases/QUERY/latest/AQL.html).
EHRbase-rs implements that surface natively in Rust, in two layers:

1. **The openEHR foundation (`openehr-*` crates)** — the Reference Model, canonical
   JSON/XML serialization, the ITS-REST contract, and the AQL parser, **generated
   deterministically from openEHR's machine-readable specifications** (BMM/XSD/OpenAPI).
   A spec-version bump is a re-run, not a rewrite ([ADR-004](docs/ADRs/ADR-004-spec-driven-codegen.md),
   [ADR-005](docs/ADRs/ADR-005-its-codegen.md)).
2. **The application (`ehrbase-*` crates)** — storage, versioning, services, the AQL
   engine, validation, and auth: modern idiomatic Rust of our own design on that
   foundation ([ADR-006](docs/ADRs/ADR-006-application-port-philosophy.md),
   [ADR-008](docs/ADRs/ADR-008-greenfield-pg18-storage.md)).

## Highlights

- **The openEHR REST API (ITS-REST 1.0.3)** — EHR, EHR_STATUS, COMPOSITION,
  DIRECTORY/FOLDER, CONTRIBUTION, QUERY, and DEFINITION (OPT 1.4 templates), served
  from server traits and routes generated from the official OpenAPI contract.
- **A full AQL 1.1 engine** — hand-written parser, path analysis against a
  BMM-generated RM attribute model, a typed query IR lowered to SQL. CONTAINS is a
  nested-set integer interval join, never a JSON tree walk. `LATEST_VERSION` **and**
  `ALL_VERSIONS` are supported.
- **PG18-native storage** — one unified `node` table (a row per RM structure node,
  canonical JSON fragments, no key aliasing) plus one temporal `vo_version` table
  (`PRIMARY KEY … WITHOUT OVERLAPS`) instead of current/history table pairs; full
  time-travel reads; contribution + audit written in the same transaction as every
  change.
- **Composition validation** — RM invariants + terminology + WebTemplate constraint
  checking → ITS-REST 422 problem details.
- **SDT surface** — WebTemplate (`wt+json`), FLAT, and STRUCTURED formats with
  Better-semantics parity.
- **Authentication** — Basic and OAuth2/OIDC (Keycloak-style) bearer auth out of the
  box; RBAC/ABAC authorization is in active development
  ([design](docs/enterprise/access-control.md)).
- **IHE ATNA audit trail** — every API operation emits a DICOM AuditMessage over
  RFC 5424 syslog (UDP or TLS), with a total-coverage guard so no endpoint ships
  unaudited.
- **Observability** — `tracing` + OTLP traces, Prometheus metrics, a management
  surface that is off by default, and a one-command local Grafana LGTM stack.
  PHI never enters telemetry.
- **Small, safe deployment** — a distroless, non-root, shell-less container
  (~pure-Rust binary with rustls; no OpenSSL), multi-arch (amd64 + arm64).

## Quickstart (Docker)

Two published images, mirroring EHRbase's app + preconfigured-postgres model:
`ghcr.io/rubentalstra/ehrbase-rs` and `ghcr.io/rubentalstra/ehrbase-rs-postgres`
(PostgreSQL 18 with the role, database, schemas, and extensions pre-created — the
server runs its own migrations at boot).

```shell
docker compose up --build          # build + start server and database

# Probe the public status endpoint:
curl http://localhost:8080/ehrbase/rest/status

# Create an EHR (dev Basic credentials: ehrbase / ehrbase):
curl -u ehrbase:ehrbase -X POST -i \
  http://localhost:8080/ehrbase/rest/openehr/v1/ehr

# Run an AQL query:
curl -u ehrbase:ehrbase -H 'Content-Type: application/json' \
  -d '{"q":"SELECT e/ehr_id/value FROM EHR e"}' \
  http://localhost:8080/ehrbase/rest/openehr/v1/query/aql

docker compose down -v             # stop and remove the data volume
```

The dev credentials come from [`docker/ehrbase.dev.toml`](docker/ehrbase.dev.toml)
— **development only**; configure real credentials or OIDC for anything else.
All configuration is environment-driven (`EHRBASE_*`).

<details>
<summary>Local observability stack (Grafana LGTM)</summary>

The [`docker-compose.observability.yml`](docker-compose.observability.yml) overlay
adds a single-container OTLP collector + Prometheus + Tempo + Loki + Grafana and
points the app at it:

```shell
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build
# Grafana: http://localhost:3000 — provisioned "ehrbase-rs — service overview" dashboard
```

Dashboards and an alert starter pack live in
[`docker/observability/`](docker/observability/); the design contract is
[`docs/design/observability.md`](docs/design/observability.md).
</details>

## Status & roadmap

**Stage 1 (a complete, conformant CDR) is most of the way through.** Foundation
phases 00–08 (generated spec layer, serialization, contracts, parsers) and
application phases 09–16 are done:

| Done | Delivered |
|---|---|
| P09–P10 | persistence infrastructure + the greenfield node/temporal-version storage with a lossless codec |
| P11–P12 | the full REST surface (~96 operations) + auth, and the service layer (versioned CRUD, contributions, time-travel, tags, stored queries) |
| P13–P15 | OPT 1.4 template ingestion, WebTemplate/FLAT/STRUCTURED, composition validation |
| P16 | the AQL engine, ATNA audit trail, container images, observability |

Remaining before the first release: **P17** FLAT/EhrScape compatibility wiring,
**P18** workspace integration, **P19** openEHR **CNF conformance** (the acceptance
instrument), **P20** optimization. In parallel, enterprise capabilities are being
restored, starting with [RBAC/ABAC access control](docs/enterprise/access-control.md).
Progress is tracked in [`docs/PROGRESS.md`](docs/PROGRESS.md) and
[`docs/plans/`](docs/plans/).

Quality gates on every commit: build + 600+ tests against real PostgreSQL 18
(testcontainers), clippy `-D warnings`, rustfmt, `cargo deny`/`audit`/`machete`,
codegen drift (the generated layer must match the vendored specs), and a container
smoke test that boots both images and exercises the API.

## Architecture

```
crates/
├── openehr-codegen    # BMM/XSD/OAS → Rust generator            ┐ tooling
├── openehr-derive     # #[derive(OpenEhrType)] canonical JSON   ┘
├── openehr-base       # BASE 1.3.0: foundation + base types     ┐
├── openehr-rm         # RM 1.2.0: the Reference Model           │ generated
├── openehr-am         # AM 1.4 + 2.4: ADL/AOM (am14 + am24)     │ from the
├── openehr-lang       # LANG: BMM / ODIN object model           ┘ vendored specs
├── openehr-term       # TERM 3.1.0: terminology bundle (hand-written)
├── openehr-query      # AQL 1.1 lexer + parser (logos + chumsky)
├── openehr-its        # canonical JSON/XML + generated ITS-REST contract + fidelity gates
├── openehr-flat       # WebTemplate / FLAT / STRUCTURED (SDT)
├── ehrbase-rest       # the axum REST server + auth              ┐
├── ehrbase-compat     # EhrScape / admin / compatibility surface │ the application
├── ehrbase-audit      # IHE ATNA audit trail                     │ (our own design)
└── ehrbase            # binary: storage, services, AQL engine    ┘
```

Dependencies point one way: application → specification, never the reverse. The
generated crates carry `// @generated` headers and are never hand-edited — changes
go through the emitter and a regeneration, enforced by a CI drift check. The full
picture is in [`docs/architecture.md`](docs/architecture.md).

## Tech stack

| Layer | Choice |
|---|---|
| Language | Rust stable **1.96**, edition 2024 |
| Database | **PostgreSQL 18** (18.4+): `uuidv7()`, temporal `WITHOUT OVERLAPS` keys, `RETURNING OLD/NEW`, `JSON_TABLE`, skip scan |
| Web / async | `axum` · `tower-http` · `hyper` · `tokio` |
| Persistence | `sqlx` + `sea-query` (dynamic AQL SQL; not an ORM) |
| Auth | `jsonwebtoken` · `oauth2` · `openidconnect` · `argon2` |
| Serialization | `serde`/`serde_json` · `quick-xml` |
| Parsers | `logos` + `chumsky` — no ANTLR runtime |
| Observability | `tracing` · OpenTelemetry OTLP · `metrics` + Prometheus |

Every version is pinned: the manifest (`[workspace.dependencies]`) is authoritative,
with the platform/spec matrix in [`docs/VERSIONS.md`](docs/VERSIONS.md).

## Building from source

Prerequisites: the pinned Rust toolchain (installed automatically by `rustup` from
[`rust-toolchain.toml`](rust-toolchain.toml)), Docker (for the PostgreSQL 18
integration tests via testcontainers), and `xmllint` (`libxml2-utils`) for the
canonical-XML parity tests.

```shell
cargo build --workspace                # build everything
cargo nextest run --workspace          # unit + integration tests
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check

# Regenerate the openEHR spec + ITS layer from the vendored specifications:
cargo run -p openehr-codegen -- emit        # spec crates from BMM
cargo run -p openehr-codegen -- emit-xml    # canonical-XML impls
cargo run -p openehr-codegen -- emit-rest   # ITS-REST contract
```

## Documentation

| Where | What |
|---|---|
| [`docs/architecture.md`](docs/architecture.md) | the system design |
| [`docs/plans/`](docs/plans/) · [`docs/PROGRESS.md`](docs/PROGRESS.md) | the roadmap and phase history |
| [`docs/ADRs/`](docs/ADRs/) | decision records — start with [ADR-008](docs/ADRs/ADR-008-greenfield-pg18-storage.md) (storage + conformance pivot) and [ADR-004](docs/ADRs/ADR-004-spec-driven-codegen.md)/[005](docs/ADRs/ADR-005-its-codegen.md) (spec codegen) |
| [`docs/design/`](docs/design/) | subsystem designs: [AQL engine](docs/design/aql-engine.md) · [observability](docs/design/observability.md) · [container images](docs/design/container-images.md) |
| [`docs/enterprise/`](docs/enterprise/) | enterprise capabilities: [ATNA audit](docs/enterprise/atna-audit.md) · [access control](docs/enterprise/access-control.md) |
| [`docs/specs/openehr/`](docs/specs/openehr/) | the vendored openEHR specification text + CNF conformance schedule (the oracle) |

For openEHR itself, see the [openEHR specifications](https://specifications.openehr.org/)
and the upstream [EHRbase documentation](https://docs.ehrbase.org).

## Relationship to upstream EHRbase

This project began as an EHRbase fork (imported at v2.33.0) and keeps that history
in git. Since [ADR-008](docs/ADRs/ADR-008-greenfield-pg18-storage.md) its internals
are greenfield designs and its compatibility target is the **openEHR specifications**
— verified by the official CNF conformance framework — rather than EHRbase parity.
It pins the *latest* published spec versions (RM 1.2.0, BASE 1.3.0, TERM 3.1.0,
AM 1.4.0 + 2.4.0). EHRbase remains valued prior art. This project is not affiliated
with or endorsed by the upstream EHRbase project or vitagroup.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md).
Every pull request must pass the full CI gate (build, tests, clippy, rustfmt,
supply-chain checks, codegen drift).

## Acknowledgments

This project is a fork of, and owes its architecture to, **EHRbase**, jointly
developed by [vitasystems GmbH](https://www.vitagroup.ag/) and the
[Peter L. Reichertz Institute (PLRI)](https://www.plri.de/). Upstream EHRbase in
turn contains code derived from EtherCIS and relies on the openEHR Reference Model
implementation [Archie](https://github.com/openEHR/archie) by Nedap. The openEHR
specifications and the machine-readable BMM meta-model this project generates from
are published by the [openEHR Foundation](https://www.openehr.org/).

## License

EHRbase-rs is licensed under the [Apache License, Version 2.0](LICENSE), the same
license as upstream EHRbase.
