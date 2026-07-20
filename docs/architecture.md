# Architecture

A pure-Rust, **openEHR-spec-conformant** Clinical Data Repository (CDR):
ITS-REST 1.1.0 at the API, AQL 1.1 as the query language, greenfield
PostgreSQL-18-native internals. Two layers:

1. **The openEHR foundation (`openehr-*`) — generated from the official
   machine-readable specs, and done.** The spec types, canonical serialization,
   the ITS-REST contract, and the AQL front end are produced deterministically
   by `openehr-codegen`, not hand-written.
2. **The application (`ehrbase-*`) — modern idiomatic Rust of our own design
   on that foundation.** The server, storage, service layer, AQL
   execution engine, validation, and auth — proper crates, our own algorithms,
   the openEHR specifications as the authority. EHRbase and other CDRs are
   prior art, not an oracle. This is the remaining Stage-1 work.

Authoritative roadmap: the root **`ROADMAP.md`**; the open-items tracker is
GitHub Issues (root `CLAUDE.md` §Issue workflow), with `docs/plans/` (deep
working plans) under it; the build record is the closed issues + PR
descriptions + `CHANGELOG.md` + git history. The per-endpoint call-chain
reference is `docs/endpoint-map.md`.

Key architectural decisions (described in full in the sections below): the
spec + ITS layer is generated from the vendored machine-readable specs
(BMM/XSD/OAS) by `openehr-codegen`; the application is idiomatic Rust of our
own design on those generated crates, with its own PG18-native storage (one
`node` table + one temporal `vo_version` table) and its own typed AQL engine,
and acceptance measured by the openEHR conformance suite (EHRbase is prior art,
not an oracle); the application is three crates with zero re-exports, and its
service layer follows the openEHR SM Platform Service Model (one module per SM
chapter, concrete methods).

## The generated openEHR foundation (done)

- **Spec types** (BMM → Rust, `openehr-codegen -- emit`): `openehr-base` (BASE
  1.3.0), `openehr-rm` (RM 1.2.0 — the domain model everything consumes),
  `openehr-am` (AM 1.4 + 2.4, as `am14`/`am24`), `openehr-term` (TERM data
  classes + hand-written bundle/assets), `openehr-lang` (BMM/P_BMM model).
- **Canonical JSON** — the native `ToJson`/`FromJson` codec (`emit-json`) puts `_type`
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
- **RM invariants** (`emit-validate`) — the BMM's invariant expressions are
  machine-classified (a pinned total tripartition) and the mechanical ones
  emitted as generated cores in `openehr-rm/src/validate/generated.rs` —
  the single source both the typed and fast validation paths call;
  terminology-backed invariants enforce at the `openehr-its` dispatcher
  against the `openehr-term` bundle; the aggregate exclusions carry
  citation-pinned adjudications in the generated register.

The generator (`tools/openehr-codegen`) is a four-stage pipeline — load
(vendored inputs verbatim) → analyze (merged include-closures, polymorphic
seams, ownership graph + back-reference cycle breaking, constructibility
proof, enumerations/constants/invariants) → plan (per-class emission
decisions + the declarative, spec-cited decision maps) → render (the only
text-producing stage). Emitter invariants (completeness — nothing a loaded
schema declares is silently dropped —, constructibility, byte-determinism,
source-package mirroring, closure correctness) are themselves a test suite.
**The spec types carry no serde**: canonical JSON is the emitted native
codec over a small hand-written runtime (the same shape as the XML codec);
the wire contract (`_type`-first, BMM field order, RM number typing,
tolerant reads) is pinned by the canonical-output contract gate.

Fidelity is proven by gates (`openehr-its/tests/`); a `codegen-drift` CI job
regenerates everything and fails on any diff.

## Storage (ours, PG18-native)

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
  migrator infrastructure.

## AQL engine (ours)

Parsed AST (`openehr-query`, done) → **path analysis + typing against a
BMM-generated RM attribute model** (attribute→types, multiplicity,
abstract→concrete descendants — generated, not reflected, not hand-written)
→ **our typed query IR** → SQL via `sea-query` (nested-set interval joins for
CONTAINS chains; `jsonb_path_query_first` + jsonpath item methods +
`openehr_magnitude` for typed leaf extraction/comparison/ordering;
`JSON_TABLE` for array unnesting; GIN `jsonb_ops` `$.**` equality anchors as
document pre-filters) → execute (`sqlx`) → `RESULT_SET` (1.1.0). The feature
envelope is documented per construct; rejections are explicit typed errors.

## REST surface + auth (`ehrbase-rest`)

Base path `/ehrbase/rest/openehr/v1`, implementing the generated ITS-REST
1.1.0 server traits over `axum` with a `tower-http` middleware stack and
content negotiation (canonical JSON/XML via `openehr-its`). Extensions: admin
API, `/rest/status`, `/management/*`, item tags, EhrScape compatibility
(a feature-gated `ehrscape` adapter module in `ehrbase-rest`). **Auth (Stage 1):** Basic + OAuth2/OIDC via
`argon2`/`jsonwebtoken`/`oauth2`/`openidconnect`; RBAC is Stage 2.

## Templates, validation, FLAT

OPT 1.4 XML ingestion → `openehr-am`; WebTemplate builder (`moka`-cached);
composition validation (walker over WebTemplate + RM invariants + terminology
binding via `openehr-term`); FLAT/STRUCTURED/Web-Template JSON in
`openehr-flat` (Better `web-template` semantics; quirks behind a feature
flag).

## Spec version policy

One pin per openEHR component, always the newest generation we have
vendored (`docs/VERSIONS.md` is the ladder). openEHR's own release strategy
guarantees within-major compatibility (minor releases are additive), so the
newer-generation pin accepts every valid older-minor instance — no version
negotiation and no parallel generations, with exactly one exception: **AM
ships both extant majors** (`am14` + `am24`), because the spec itself keeps
ADL 1.4 and ADL 2 side by side. A future major release triggers a
per-component decision (dual generation via the `am14`/`am24` codegen
pattern only if the ecosystem runs both; otherwise cutover). ITS-REST is
single-version by owner ruling: the CDR implements the latest released REST
API, nothing else. Upstream spec changes and releases are detected
automatically by the scheduled watcher workflows and filed as `spec-update`
issues for triage.

## PostgreSQL 18

We target **PG 18** (18.4+): `uuidv7()`, temporal `WITHOUT OVERLAPS`
constraints, `RETURNING OLD/NEW`, `JSON_TABLE` + SQL/JSON functions and
jsonpath item methods (PG 17), B-tree skip scan, async I/O, STORED generated
columns for hot extractions. See `docs/postgres-features.md`.

## Workspace layout

Three physical directories (consolidated 2026-07-16):
**`app/*`** holds the application — `ehrbase` (the platform **library**),
`ehrbase-rest` (the ITS-REST protocol adapter, which calls the concrete
`EhrbaseService` directly), and `ehrbase-server` (the wiring-only binary; the
bin is still named `ehrbase`); **`tools/*`** holds the dev/verification
tooling that is *not* part of the shipped application (`conformance` — the
ECC runner, `benchmark`, `testkit` — the shared test-database harness, and
`openehr-codegen` — the BMM/XSD/OAS → Rust generator); **`crates/*`** holds the
generated openEHR spec layer + its tooling (`openehr-*`). Root
workspace `members = ["crates/*", "app/*", "tools/*"]`. Arrows:
`ehrbase-server → {ehrbase-rest, ehrbase}`, `ehrbase-rest → ehrbase`,
`app/* → crates/openehr-*`. The former `ehrbase-sm` trait catalog is deleted:
the SM Platform Service Model survives as the *structure* of
`ehrbase::service` — one module per SM chapter, concrete methods, SM call
semantics as the design authority — with zero re-exports anywhere (every
import names its defining module).

### SM platform component map

The service layer realizes the openEHR **SM Platform Service Model**
(vendored SM spec `docs/specs/openehr/SM/docs/openehr_platform/`). One
`ehrbase::service` module per SM component; the SM interfaces map to concrete
`EhrbaseService` methods (there is no trait catalog):

| SM component | SM interface(s) | Realization (`ehrbase::service`) | Status |
|---|---|---|---|
| EHR | `I_EHR_SERVICE`, `I_EHR_STATUS`, `I_EHR_COMPOSITION`, `I_EHR_DIRECTORY`, `I_EHR_CONTRIBUTION` | `service::ehr` (status/composition/directory/contributions/tags/access modules) | implemented |
| Definitions | `I_DEFINITION_ADL14`/`ADL2`/`QUERY` | `service::definition` (adl14/adl2/query_store/wire modules) | implemented |
| Demographic | `I_DEMOGRAPHIC_SERVICE`, `I_PARTY`, `I_PARTY_RELATIONSHIP` | `service::demographic` | implemented |
| Query | `I_QUERY_SERVICE` | `service::query` | implemented |
| Validity checking | `I_VALIDITY_CHECKER` | `service::validity` | implemented |
| System Log | `I_SYSTEM_LOG` (stub; "IHE ATNA-compliant") | `ehrbase::system_log` (dual DICOM PS3.15 + FHIR `AuditEvent`/BALP rendering; local Audit Record Repository in the `audit` schema, on by default; syslog + ITI-20 ATX:FHIR Feed forwarding sinks; the ITI-81 retrieval as the read side; ITI-19 mTLS via `[server.tls]`) | implemented |
| Admin | `I_ADMIN_SERVICE` (+archive/dump-load) | `service::admin` | implemented |
| EHR Index | `I_EHR_INDEX` | `service::ehr_index` | implemented |
| Terminology | `I_TERMINOLOGY_SERVICE` | `service::terminology` | implemented |
| Message | `I_MESSAGE_SERVICE`, `I_EHR_EXTRACT_SERVICE`, `I_TDD_SERVICE` | `service::message` | implemented |
| Subject Proxy | `I_SUBJECT_PROXY_SERVICE`, `I_DATA_BINDING` | `service::subject_proxy` | implemented |

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
| `openehr-codegen` | BMM/XSD/OAS → Rust generator (+ `emit-rm-model`) | tooling |
| `ehrbase-rest` | ITS-REST protocol adapter (axum) + auth + ATNA audit middleware; `access` module = RBAC/ABAC authz; calls the concrete `EhrbaseService` | application |
| `ehrbase` | The platform library: storage, service layer (one module per SM chapter), AQL engine, versioning, the full config tree, telemetry, `signing` + `system_log` | application |
| `ehrbase-server` | The wiring-only binary (config → pool → migrations → service → serve); bin name `ehrbase` | application |
| `conformance` | ECC conformance runner (`tools/*`) | tooling |
| `benchmark` | Benchmark harness (`tools/*`) | tooling |
| `testkit` | Shared test-database harness: one PG18 server + template-database cloning (`tools/*`) | tooling |

## Build sequence & stages

- **Stage 1**: the openEHR-conformant CDR — the generated spec/ITS
  foundation plus the full application (persistence, greenfield storage,
  REST + auth, the SM service layer, templates, WebTemplate/FLAT/STRUCTURED,
  validation, the AQL engine, conformance, optimization, cutover). Built as
  compiling, tested increments; the per-phase build record is the closed
  issues + PR descriptions (+ the retired `docs/PROGRESS.md` in git
  history).
- **Stage 2**: enterprise capabilities — RBAC/attribute authz, plugin system,
  multi-tenancy (`reference/v1` archaeology).
- **Stage 3**: refinement, performance, new capabilities.

## Verification

- **Fidelity gates** (spec/serialization): canonical JSON read + lossless
  round-trip + ITS-JSON schema validation; XML round-trips.
- **Conformance suite** (`scripts/conformance.sh` — present): the ECC catalogue
  (Docker-composed SUT, both formats) — the acceptance instrument;
  the standing baseline: 341 executed · 315 passed · 0 failed,
  CORE/STANDARD PASS.
- **Drift check** (`scripts/check-codegen-drift.sh` + CI): the generated layer
  is always in sync with the vendored specs.
