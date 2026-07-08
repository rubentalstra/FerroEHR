# Architecture

A pure-Rust, **openEHR-spec-conformant** Clinical Data Repository (CDR):
ITS-REST 1.0.3 at the API, AQL 1.1 as the query language, greenfield
PostgreSQL-18-native internals (ADR-008). Two layers:

1. **The openEHR foundation (`openehr-*`) — generated from the official
   machine-readable specs, and done.** The spec types, canonical serialization,
   the ITS-REST contract, and the AQL front end are produced deterministically
   by `openehr-codegen` (ADR-004/005), not hand-written.
2. **The application (`ehrbase-*`) — modern idiomatic Rust of our own design
   on that foundation (ADR-006/008).** The server, storage, service layer, AQL
   execution engine, validation, and auth — proper crates, our own algorithms,
   the openEHR specifications as the authority. EHRbase and other CDRs are
   prior art, not an oracle. This is the remaining Stage-1 work.

Authoritative roadmap: `docs/plans/` (phases `00–20, 99` + SM phases) +
`docs/PROGRESS.md`.
Decisions: `docs/ADRs/ADR-004` (spec codegen), `ADR-005` (ITS codegen),
`ADR-006` (application philosophy), **`ADR-008` (greenfield storage + AQL +
conformance target — read first)**, `ADR-010` (SM-aligned service
architecture + the `app/*`/`crates/*` two-layer workspace layout; design set
at `docs/design/sm-platform/`).

## The generated openEHR foundation (done)

- **Spec types** (BMM → Rust, `openehr-codegen -- emit`): `openehr-base` (BASE
  1.3.0), `openehr-rm` (RM 1.2.0 — the domain model everything consumes),
  `openehr-am` (AM 1.4 + 2.4, as `am14`/`am24`), `openehr-term` (TERM data
  classes + hand-written bundle/assets), `openehr-lang` (BMM/P_BMM model).
- **Canonical JSON** — `#[derive(OpenEhrType)]` (`openehr-derive`) puts `_type`
  self-tagging on every type; `openehr-its::json` is the entry points +
  ITS-JSON schema validation.
- **Canonical XML** (`emit-xml`) — generated `ToXml`/`FromXml` over a
  hand-written `quick-xml` runtime, from the vendored XSDs + BMM field model
  (`openehr-its`).
- **ITS-REST contract** (`emit-rest`) — generated DTOs + `#[async_trait]`
  server traits + route tables per API group, from the vendored OpenAPI
  (`openehr-its`); the application implements the traits.
- **AQL front end** — hand-written `logos`+`chumsky` lexer/parser/AST
  (`openehr-query`), corpus-validated.

Fidelity is proven by gates (`openehr-its/tests/`); a `codegen-drift` CI job
regenerates everything and fails on any diff.

## Storage (ours, PG18-native — ADR-008, built at P10)

Grounded in docs-verified PostgreSQL physics (JSONB has no partial detoast —
big single documents pay whole-document decompression per leaf access; GIN
serves no range/ordering), the storage is a **decomposed node model designed
fresh**:

- **`node`** — one unified table for all versioned-object content
  (COMPOSITION / EHR_STATUS / FOLDER). One row per RM structure node with a
  **nested-set index** (`num`, `num_cap`, `parent_num`, `citem_num`): AQL
  CONTAINS is an integer interval join, never a JSON walk. Promoted predicate
  columns (`rm_type` — full RM type names, no alias compaction —,
  `archetype`, `name`, `path COLLATE "C"`, `ehr_id`) and a **canonical
  openEHR JSON fragment** in `data jsonb` (verbatim `openehr-its` encoding:
  zero translation between storage and API, no synthetic fields).
- **`vo_version`** — one temporal version table (PG18
  `PRIMARY KEY … WITHOUT OVERLAPS`, `sys_period tstzrange`, `uuidv7()` keys)
  instead of current+`_history` pairs; current = `upper_inf(sys_period)`
  partial index. `LATEST_VERSION` and **`ALL_VERSIONS`** both supported.
- **`ehr`, `contribution`, `audit`, `template_store`, `stored_query`,
  `item_tag`** — supporting tables; every write emits contribution + audit in
  the same transaction (openEHR requirement).
- **`ext`** — our own `IMMUTABLE` helper functions (e.g.
  `openehr_magnitude(jsonb)` for DV_ORDERED ordering semantics), usable in
  btree **expression indexes** for measured hot paths.
- Migrations via `sqlx migrate add` (official CLI); `sqlx` pool + two-schema
  migrator infrastructure from P09.

## AQL engine (ours — ADR-008, built at P16)

Parsed AST (`openehr-query`, done) → **path analysis + typing against a
BMM-generated RM attribute model** (attribute→types, multiplicity,
abstract→concrete descendants — generated, not reflected, not hand-written)
→ **our typed query IR** → SQL via `sea-query` (nested-set interval joins for
CONTAINS chains; `jsonb_path_query_first` + jsonpath item methods +
`openehr_magnitude` for typed leaf extraction/comparison/ordering;
`JSON_TABLE` for array unnesting; GIN `jsonb_ops` `$.**` equality anchors as
document pre-filters) → execute (`sqlx`) → `RESULT_SET` (1.0.3). The feature
envelope is documented per construct; rejections are explicit typed errors.

## REST surface + auth (`ehrbase-rest`, `ehrbase-compat`, built at P11/P12)

Base path `/ehrbase/rest/openehr/v1`, implementing the generated ITS-REST
1.0.3 server traits over `axum` with a `tower-http` middleware stack and
content negotiation (canonical JSON/XML via `openehr-its`). Extensions: admin
API, `/rest/status`, `/management/*`, item tags, EhrScape compatibility
(`ehrbase-compat`). **Auth (Stage 1):** Basic + OAuth2/OIDC via
`argon2`/`jsonwebtoken`/`oauth2`/`openidconnect`; RBAC is Stage 2.

## Templates, validation, FLAT (P13–P15, P17)

OPT 1.4 XML ingestion → `openehr-am`; WebTemplate builder (`moka`-cached);
composition validation (walker over WebTemplate + RM invariants + terminology
binding via `openehr-term`); FLAT/STRUCTURED/Web-Template JSON in
`openehr-flat` (Better `web-template` semantics; quirks behind a feature
flag).

## PostgreSQL 18

We target **PG 18** (18.4+): `uuidv7()`, temporal `WITHOUT OVERLAPS`
constraints, `RETURNING OLD/NEW`, `JSON_TABLE` + SQL/JSON functions and
jsonpath item methods (PG 17), B-tree skip scan, async I/O, STORED generated
columns for hot extractions. See `docs/postgres-features.md`.

## Workspace layout

Three physical directories (ADR-010; move executed 2026-07-08):
**`app/*`** holds the application crates (`ehrbase`, `ehrbase-sm` [SM-1],
`ehrbase-rest`, `ehrbase-compat`, `ehrbase-audit`, `ehrbase-authz`,
`ehrbase-signing`); **`tools/*`** holds the dev/verification tooling that is
*not* part of the shipped application (`conformance` — the ECC runner,
`benchmark`); **`crates/*`** holds the generated openEHR spec layer + its
tooling (`openehr-*`, `openehr-codegen`, `openehr-derive`). Root workspace
`members = ["crates/*", "app/*", "tools/*"]`.
Dependencies point downward only: `tools/* → app/* → crates/openehr-*`. The service seam is the SM-aligned native API
(`ehrbase-sm`, ADR-010): one trait per SM Platform Service Model interface,
with `ehrbase-rest` as the ITS-REST protocol adapter — see
`docs/design/sm-platform/08-target-architecture.md`.

| Crate | Role | Kind |
|---|---|---|
| `openehr-base` | BASE 1.3.0 | generated |
| `openehr-rm` | RM 1.2.0 — the domain model | generated |
| `openehr-am` | AM 1.4 + 2.4 (`am14`/`am24`) | generated |
| `openehr-term` | TERM classes + terminology bundle | generated + hand-written |
| `openehr-lang` | BMM/P_BMM object model | generated |
| `openehr-its` | Canonical JSON/XML + ITS-REST contract + runtimes + gates | generated + hand-written |
| `openehr-query` | AQL 1.1 lexer + parser + AST | hand-written |
| `openehr-flat` | FLAT / STRUCTURED / Web Template | hand-written |
| `openehr-codegen` | BMM/XSD/OAS → Rust generator (+ `emit-rm-model`, P16) | tooling |
| `openehr-derive` | `#[derive(OpenEhrType)]` proc-macro | tooling |
| `ehrbase-rest` | ITS-REST server (axum) + auth | application |
| `ehrbase-compat` | EhrScape, admin, WebTemplate/FLAT endpoints | application |
| `ehrbase` | Binary: storage, service layer, AQL engine, versioning, CLI | application |

## Build sequence & stages

- **Stage 1** (`docs/plans/`): foundation done (00–08); the application is
  built as compiling, tested increments — **P09** persistence infra ✅ →
  **P10** storage foundation ✅ → **P11** REST+auth ✅ → **P12** service ✅ →
  **P13** templates ✅ → **P14** WebTemplate/FLAT/STRUCTURED ✅ →
  **P15** validation ✅ → **P16** AQL engine (current) →
  **P17** FLAT/EhrScape → **P18** integration →
  **P19** openEHR conformance → **P20** optimization → **P99**
  cleanup/release.
- **Stage 2**: enterprise capabilities — RBAC/attribute authz, plugin system,
  multi-tenancy (`reference/v1` archaeology).
- **Stage 3**: refinement, performance, new capabilities.

## Verification

- **Fidelity gates** (spec/serialization): canonical JSON read + lossless
  round-trip + ITS-JSON schema validation; XML round-trips.
- **Conformance suite** (`scripts/conformance.sh` — **planned P19**, not yet in
  the tree): the openEHR **CNF Platform Conformance Test Schedule** (REST+JSON)
  plus corpus suites — the acceptance instrument (ADR-008).
- **Drift check** (`scripts/check-codegen-drift.sh` + CI): the generated layer
  is always in sync with the vendored specs.
