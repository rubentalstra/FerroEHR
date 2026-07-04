# Architecture

A pure-Rust openEHR Clinical Data Repository (CDR) that is **openEHR-spec-
conformant and behavior-compatible with EHRbase** at the REST/AQL surface.
Two layers:

1. **The openEHR foundation (`openehr-*`) — generated from the official
   machine-readable specs, and done.** The spec types, canonical serialization,
   the ITS-REST contract, and the AQL front end are produced deterministically
   by `openehr-codegen` (ADR-004/005), not hand-written.
2. **The EHRbase application (`ehrbase-*`) — modern idiomatic Rust built on that
   foundation (ADR-006).** The server, persistence, service layer, AQL execution
   engine, validation, and auth — using proper crates, consuming the generated
   `openehr-*` types directly, following EHRbase's Java as the *behavioural
   reference* (not a class-by-class port). This is the remaining Stage-1 work.

Authoritative roadmap: `docs/plans/` (phases `00–20, 99`) + `docs/PROGRESS.md`.
Decisions: `docs/ADRs/ADR-004` (spec codegen), `ADR-005` (ITS codegen),
`ADR-006` (application philosophy).

## The generated openEHR foundation (done)

- **Spec types** (BMM → Rust, `openehr-codegen -- emit`): `openehr-base` (BASE
  1.3.0), `openehr-rm` (RM 1.2.0 — the domain model everything consumes),
  `openehr-am` (AM 1.4 + 2.4, as `am14`/`am24`), `openehr-term` (TERM data
  classes + hand-written bundle/assets), `openehr-lang` (BMM/P_BMM model).
- **Canonical JSON** — `#[derive(OpenEhrType)]` (`openehr-derive`) puts `_type`
  self-tagging on every type; `openehr-its::json` is the entry points +
  ITS-JSON schema validation.
- **Canonical XML** (`emit-xml`) — generated `ToXml`/`FromXml` over a hand-written
  `quick-xml` runtime, from the vendored XSDs + BMM field model (`openehr-its`).
- **ITS-REST contract** (`emit-rest`) — generated DTOs + `#[async_trait]` server
  traits + route tables per API group, from the vendored OpenAPI (`openehr-its`);
  the application implements the traits.
- **AQL front end** — hand-written `logos`+`chumsky` lexer/parser/AST
  (`openehr-query`), corpus-validated.

Fidelity is proven by gates (`openehr-its/tests/`): the openEHR_SDK canonical-
JSON corpus reads + round-trips losslessly + validates against the ITS-JSON
schema; XML round-trips (48 compositions + real EHRbase XML fixtures). A
`codegen-drift` CI job regenerates everything and fails on any diff — the
generated layer can never silently drift from the specs.

## What the application does (EHRbase behaviour, idiomatic Rust)

### REST surface (`ehrbase-rest`, `ehrbase-compat`)

Base path `/ehrbase/rest/openehr/v1`, ITS-REST 1.0.3: **EHR** / **EHR_STATUS**,
**COMPOSITION** (versioned), **DIRECTORY/FOLDER**, **CONTRIBUTION** (the
audit/versioning envelope), **QUERY** (ad-hoc `/aql` + stored), **DEFINITION**
(templates). EHRbase extensions: Admin API (`/rest/admin`), `/rest/status`,
`/management/*`, Item Tags, and the EhrScape API (`/rest/ecis/v1/*`, in
`ehrbase-compat`). The server is `axum` implementing the generated ITS-REST
server traits, with a `tower-http` middleware stack and content negotiation
(canonical JSON/XML via `openehr-its`).

### Authentication (Stage 1)

**Basic auth + OAuth2/OIDC** (Keycloak-style) as an `axum`/`tower` middleware —
`argon2`, `jsonwebtoken`, `oauth2`, `openidconnect`. Fine-grained
**RBAC/attribute authorization is Stage 2** (matches EHRbase's own layering).

### AQL engine (the crown jewel — `openehr-query` front end → `ehrbase` engine)

Parsed AST (done) → semantic/path analysis vs WebTemplates → **ASL** (an
abstract-SQL IR) → PostgreSQL (`sea-query`: JSONB path extraction, array
unnesting, `current` + `_history` `UNION`, `JSON_TABLE` where viable) → execute
(`sqlx`) → assemble the `RESULT_SET` (1.0.3). Built idiomatically **following
EHRbase's proven ASL approach** (its `aql-engine` Java is the reference); the
ASL IR is kept as a distinct pass — it is what makes the hard cases tractable.

### Persistence (v2 schema — `sqlx` + `sea-query`, not sea-orm)

The **real EHRbase v2 schema** is reused verbatim (the vendored Flyway SQL in
`crates/ehrbase/migrations/`, run via `sqlx migrate`). It decomposes each
composition **row-per-locatable**: every LOCATABLE node is a row with leaf
attributes as JSONB. Key tables: `ehr.comp_data`/`_history` (decomposed rows),
`ehr.comp_version`, `ehr.ehr`/`ehr_status_data`/`ehr_folder_data`,
`ehr.contribution` + `ehr.audit_details` (every write inserts both),
`ehr.template_store`, `ehr.stored_query`, `ehr.item_tag`. Versioning uses paired
`current`/`_history` tables. The RM↔JSONB bridge (`rm-db-format`) lives in
`ehrbase/src/rm_db_format/`, over the generated `openehr-rm` types.

### Templates, validation, FLAT

OPT 1.4 XML ingestion → `openehr-am`; a WebTemplate builder (`moka`-cached);
composition validation (a validation walker over the WebTemplate + terminology
binding via `openehr-term`); FLAT/STRUCTURED/Web-Template JSON in `openehr-flat`
(Better `web-template` semantics; EHRbase quirks behind a feature flag).

## PostgreSQL 18

We target **PG 18** (EHRbase Java targets PG 15/16) to use two majors of new
capability — `uuidv7()`, temporal `WITHOUT OVERLAPS` constraints, `RETURNING
OLD/NEW`, `JSON_TABLE` + SQL/JSON functions, B-tree skip scan, async I/O,
generated columns, OAuth. See `docs/postgres-features.md` for the full feature
delta and where each phase uses it.

## Workspace layout (13 crates)

Dependencies point downward only: app (`ehrbase-*`) → spec (`openehr-*`).

```
openehr-derive (proc-macro)        openehr-codegen (BMM/XSD/OAS → Rust generator)
        │                                   │  (generates ▼)
        └───────────────► openehr-base ─────┴─► openehr-rm ─► openehr-am
                               │                    │            (am14/am24)
                               ▼                    ▼
                          openehr-term         openehr-lang
                               │                    │
                               ▼                    ▼
   openehr-query ◄─── openehr-rm ───► openehr-its ───► openehr-flat
        │                                 │                 │
        └──────────► ehrbase-rest ◄───────┴─────────────────┘
                          │
                          ▼
                    ehrbase-compat ─► ehrbase (binary)
```

| Crate | Role | Kind |
|---|---|---|
| `openehr-base` | BASE 1.3.0 (foundation + base types + identification) | generated |
| `openehr-rm` | RM 1.2.0 — the domain model | generated |
| `openehr-am` | AM 1.4 + 2.4 (AOM) as `am14`/`am24` | generated |
| `openehr-term` | TERM data classes + terminology bundle/assets | generated + hand-written |
| `openehr-lang` | BMM/P_BMM object model | generated |
| `openehr-its` | Canonical JSON + generated XML (`ToXml`/`FromXml`) + generated ITS-REST contract + runtimes + fidelity gates | generated + hand-written |
| `openehr-query` | AQL 1.1.0 lexer + parser + AST | hand-written |
| `openehr-flat` | FLAT / STRUCTURED / Web Template | hand-written |
| `openehr-codegen` | The BMM/XSD/OAS → Rust generator (`emit`/`emit-xml`/`emit-rest`) | tooling |
| `openehr-derive` | `#[derive(OpenEhrType)]` proc-macro | tooling |
| `ehrbase-rest` | ITS-REST server (`axum`) + auth; implements the generated traits | application |
| `ehrbase-compat` | EhrScape, admin API, WebTemplate/FLAT endpoints | application |
| `ehrbase` | Binary: persistence, service layer, AQL engine, versioning, config, CLI | application |

The `ehrbase-*` crates carry EHRbase's Java in-tree as the **read-only
behavioural reference**, deleted per-subsystem as each reaches parity (P99).

## Build sequence & stages

- **Stage 1** (`docs/plans/`): the generated foundation is done; the application
  is built as **compiling, tested increments** in dependency order — **P09**
  persistence → **P10** rm-db-format → **P11** REST+auth → **P12** service →
  **P13** templates → **P14** WebTemplate → **P15** validation → **P16** AQL
  engine → **P17** FLAT/EhrScape → **P18** integration → **P19** conformance &
  parity (≥99% at the REST surface vs stock EHRbase) → **P20** optimization →
  **P99** cutover.
- **Stage 2** (after parity holds): restore the enterprise capabilities EHRbase
  removed pre-v2 — RBAC/attribute authz (highest priority), the plugin system,
  multi-tenancy — from the `reference/v1` archaeology.
- **Stage 3**: idiomatic refinement, performance, new capabilities.

## Verification

- **Fidelity gates** (spec/serialization): JSON read + lossless round-trip +
  ITS-JSON schema validation; XML round-trip + real EHRbase-XML fixtures.
- **Parity harness** (`scripts/parity.sh`, P19): drives our server and a stock
  EHRbase with identical requests and diffs responses; the `USE_REFERENCE_EHRBASE=1`
  negative gate keeps parity tests honest. This — not class-level mirroring — is
  how "behavior-compatible" is proven.
- **Drift check** (`scripts/check-codegen-drift.sh` + CI): the generated layer is
  always in sync with the vendored specs.
