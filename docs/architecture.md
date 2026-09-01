# Architecture

A pure-Rust, **openEHR-spec-conformant** Clinical Data Repository (CDR):
ITS-REST 1.1.0 at the API, AQL 1.1 as the query language, greenfield
PostgreSQL-18-native internals. Two layers:

1. **The openEHR foundation (`openehr-*`) — generated from the official
   machine-readable specs, and done.** The spec types, canonical serialization,
   the ITS-REST contract, and the AQL front end are produced deterministically
   by `openehr-codegen`, not hand-written.
2. **The application (`ferroehr-*`) — modern idiomatic Rust of our own design
   on that foundation.** The server, storage, service layer, AQL
   execution engine, validation, and auth — proper crates, our own algorithms,
   the openEHR specifications as the authority. EHRbase and other CDRs are
   prior art, not an oracle. Shipped.

Authoritative roadmap: the public **FerroEHR Roadmap board** (a GitHub
Project view over the tracker; its readme carries the direction themes —
`.claude/rules/project-board.md`); the open-items tracker is
GitHub Issues (root `CLAUDE.md` §Issue workflow), with `docs/plans/` (deep
working plans) under it; the build record is the closed issues + PR
descriptions + `CHANGELOG.md` + git history. Per-endpoint call chains are
read from the code (router → handler → service → SQL) — there is no standing
endpoint-map document (a standing map goes stale).

Key architectural decisions (described in full in the sections below): the
spec + ITS layer is generated from the vendored machine-readable specs
(BMM/XSD/OAS) by `openehr-codegen`; the application is idiomatic Rust of our
own design on those generated crates, with its own PG18-native storage (one
`node` table + one temporal `vo_version` table) and its own typed AQL engine,
and acceptance measured by the openEHR conformance suite (EHRbase is prior art,
not an oracle); the application is four crates with zero re-exports, and its
service layer follows the openEHR SM Platform Service Model (one module per SM
chapter, concrete methods).

## The generated openEHR foundation (done)

- **Spec types** (BMM → Rust, `openehr-codegen -- emit`): `openehr-base` (BASE
  1.3.0), `openehr-rm` (RM 1.2.0 — the domain model everything consumes),
  `openehr-am` (AM 1.4 + 2.4, as `v1_4`/`v2_4`), `openehr-term` (TERM data
  classes + hand-written bundle/assets), `openehr-lang` (BMM/P_BMM model).
- **Canonical JSON** — emitted **manual serde impls** (`emit-json` → each
  defining crate's `src/json_serde.rs`; no derives, no serde attributes on
  spec types) put `_type` self-tagging on every type, over the small shared
  `openehr_base::serde_support` runtime; `openehr-its::json` is the entry
  points (`to_canonical_json`/`from_canonical_json`/`from_canonical_value`,
  refusal paths via `serde_path_to_error`) + ITS-JSON schema validation.
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
**The spec types carry no serde derives or attributes**: canonical JSON is
emitted MANUAL `serde::Serialize`/`Deserialize` impls (explicit generated
code in each defining crate's `json_serde.rs`, over the small shared
`openehr_base::serde_support` runtime); the wire contract (`_type`-first,
BMM field order, RM number typing, the STRICT reader — undeclared and
duplicate keys refused) is pinned by the canonical-output contract gate.

Fidelity is proven by gates (`openehr-its/tests/`); a `codegen-drift` CI job
regenerates everything and fails on any diff.

## Storage (ours, PG18-native)

Grounded in docs-verified PostgreSQL physics (JSONB has no partial detoast —
big single documents pay whole-document decompression per leaf access; GIN
serves no range/ordering), the storage is a **decomposed node model designed
fresh** (the diagrammed deep-dive is `docs/storage.md`):

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
- **`cold`** — the physical archival tier: FK-free mirror relations of
  `vo_version`/`node`/`vo_attestation` plus `*_all` union views.
  Admin-archived objects move there transactionally (reversibly restored, or
  thawed automatically on write); point reads retry cold only on a primary
  miss, whole-repository readers use the views, and AQL stays primary-only —
  archived content leaves the queryable store until restored.
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

## REST surface + auth (`ferroehr-rest`)

Base path `/ferroehr/rest/openehr/v1`, implementing the generated ITS-REST
1.1.0 server traits over `axum` with a `tower-http` middleware stack and
content negotiation (canonical JSON/XML via `openehr-its`). Extensions: admin
API (plus the own-design activity-report, archive/restore and dump/load routes under
the same gate), the `/message` group realizing the SM Message component
(EHR-Extract export/import + TDD import — the release publishes no such API),
the always-on public health family (`/health`, `/health/liveness`,
`/health/readiness`), `/rest/status`, the ops-introspection `/management/*`
surface, item tags (there is no EhrScape surface). **Auth:** Basic +
OAuth2/OIDC via `argon2`/`jsonwebtoken`/`oauth2`/`openidconnect`; authorization is the
shipped RBAC/ABAC `access` module in `ferroehr-rest`.

### The access layer (`ferroehr-rest::extensions::access`) — our own design

ITS-REST makes authentication a `SHOULD` and mandates no scheme, and the SM
places authorization out of band, so **only the 401-vs-403 split is
spec-grounded**: a missing or rejected credential is `401` with a
`WWW-Authenticate` challenge, an authenticated-but-refused caller is `403`.
Everything finer is our own design, built to the IETF OAuth2/JOSE RFCs and
expressed in the vocabulary of NIST SP 800-162 (ABAC) and ANSI/INCITS 359
(Core RBAC).

**Evaluation order** — each stage can only narrow what the previous one allowed,
which is what makes an optional stage safe to disable:

1. **Authentication** — Basic (Argon2id PHC, verified-credential cache) or
   OAuth2/OIDC bearer. Produces a `Principal` (subject, roles, scopes, the
   validated claim set) or a typed refusal. A malformed `Authorization` header is
   a `400`, not a `401`: the server never read a credential.
2. **`EHR_ACCESS`** — always on and unconditional, from the RM's own gateway
   clause ("All access decisions to data in the EHR must be made in accordance
   with the policies and rules in this object").
3. **RBAC** — the coarse operation class (public / clinical / admin /
   management) against the caller's roles, plus the read-only restriction, which
   overrides any grant. Roles come from the RFC 9068 §2.2.3.1 claim carriers; an
   OAuth2 scope is NOT a role (RFC 6749 §3.3).
4. **ABAC** — off by default. A policy engine (embedded Cedar, or an external
   PDP) decides over subject attributes (subject, roles, scopes, organization,
   patient) and resource attributes (patient, template), fanned out over
   multi-valued attributes as a cartesian product with **all-must-permit** and
   short-circuit deny.
5. **SMART scopes** — off by default, AND-composed onto the ABAC decision, so it
   can only narrow. Disabled SMART produces zero wire drift.

**Deny-by-default and fail-closed, in both senses.** No stage permits by
omission: a gate reached without a principal refuses; an unconfigured resource
kind on the external PDP denies (and the missing rule is a boot error); Cedar is
deny-by-default with `forbid` overriding `permit`. And a stage that cannot
DECIDE is never read as a decision — an unreachable token issuer is `503`, a
policy server answering `5xx` or a Cedar policy that errors during evaluation is
a fail-closed `500`. Silence is never consent, and a broken control never looks
like a policy outcome.

Configuration is boot-validated rather than degraded at the first request: a
mandatory audience list, an `https` issuer, an algorithm set bound to its key
source, an HMAC entropy floor, the OWASP Argon2id parameter floor, and a policy
rule per resource kind the PEP consults.

## Templates, validation, FLAT

OPT 1.4 XML ingestion → `openehr-am`; WebTemplate builder (`moka`-cached);
composition validation (walker over WebTemplate + RM invariants + terminology
binding via `openehr-term`); FLAT/STRUCTURED/Web-Template JSON in
`openehr_its::flat`, whose only authority is the vendored Simplified Formats
spec text — there is no vendor-quirk mode and no feature flag; vendor
implementations are prior art only. Simplified Formats is a STABLE ITS-REST 1.1.0 sub-specification, so it
lives in `openehr-its` beside the other ITS surfaces.

## Spec version policy

One pin per openEHR component, always the newest generation we have
vendored (`docs/VERSIONS.md` is the ladder). openEHR's own release strategy
guarantees within-major compatibility (minor releases are additive), so the
newer-generation pin accepts every valid older-minor instance — no version
negotiation and no parallel generations, with exactly one exception: **AM
ships both extant majors** (`v1_4` + `v2_4`), because the spec itself keeps
ADL 1.4 and ADL 2 side by side. A future major release triggers a
per-component decision (dual generation via the version-module codegen
pattern only if the ecosystem runs both; otherwise cutover). ITS-REST is
single-version by owner ruling: the CDR implements the latest released REST
API, nothing else. **ITS-XML is not a second generation either**: its 2.0.0
restructure changed the schemas' target namespace while leaving every element
name and `xsi:type` spelled identically, so one generated codec serves both
published lineages and the wire namespace is a serialize-time choice — v2 by default
(owner ruling 2026-08-03, issue #1666: only the v2 bundle's schemas model the
RM 1.2.0 the server emits — register AMB-185), v1 when a request selects it
with the `version` media-type parameter on `application/xml` (our own
extension; no openEHR spec governs namespace selection on the REST wire). The two bundles are not interchangeable as
VALIDATORS, though — the v1 one is frozen at an older RM generation, so a
correct RM 1.2.0 document can fail against it (`docs/VERSIONS.md` §Spec
version policy; register AMB-185). Upstream spec changes and releases are detected
automatically by the scheduled watcher workflows and filed as `spec-update`
issues for triage.

## PostgreSQL 18

We target **PG 18** (18.6+): `uuidv7()`, temporal `WITHOUT OVERLAPS`
constraints, `RETURNING OLD/NEW`, `JSON_TABLE` + SQL/JSON functions and
jsonpath item methods (PG 17), B-tree skip scan, async I/O, STORED generated
columns for hot extractions. See `docs/postgres-features.md`.

## Workspace layout

Three physical directories (consolidated 2026-07-16):
**`app/*`** holds the application — `ferroehr` (the platform **library**),
`ferroehr-rest` (the ITS-REST protocol adapter, which calls the concrete
`FerroEhrService` directly), `ferroehr-server` (the wiring-only binary; the
bin is still named `ferroehr`), `ferroehr-ext` (the feature-gated
optional-integration crate — FHIR conversion core, events transport,
multimedia store — one additive cargo feature per integration, default all-on,
slim builds compile them out with loud boot refusals for enabled-but-unbuilt
integrations), and `ferroehr-viewer` (the Leptos SSR viewer — its own
binary/OCI image, consuming the CDR strictly over ITS-REST);
**`tools/*`** holds the dev/verification
tooling that is *not* part of the shipped application
(`testkit` — the shared test-database harness, and
`openehr-codegen` — the BMM/XSD/OAS → Rust generator); **`crates/*`** holds the
generated openEHR spec layer + its tooling (`openehr-*`). Root
workspace `members = ["crates/*", "app/*", "tools/*"]`. Arrows:
`ferroehr-server → {ferroehr-rest, ferroehr}`, `ferroehr-rest → ferroehr`,
`ferroehr → ferroehr-ext` (optional, feature-forwarded),
`app/* → crates/openehr-*`. The SM Platform Service Model is realized as the
*structure* of `ferroehr::service` — one module per SM chapter, concrete
methods (no trait catalog), SM call semantics as the design authority — with
zero re-exports anywhere (every import names its defining module).

### SM platform component map

The service layer realizes the openEHR **SM Platform Service Model**
(vendored SM spec `docs/specs/openehr/SM/docs/openehr_platform/`). One
`ferroehr::service` module per SM component; the SM interfaces map to concrete
`FerroEhrService` methods (there is no trait catalog):

| SM component | SM interface(s) | Realization (`ferroehr::service`) | Status |
|---|---|---|---|
| EHR | `I_EHR_SERVICE`, `I_EHR_STATUS`, `I_EHR_COMPOSITION`, `I_EHR_DIRECTORY`, `I_EHR_CONTRIBUTION` | `service::ehr` (status/composition/directory/contributions/tags/access modules) | implemented |
| Definitions | `I_DEFINITION_ADL14`/`ADL2`/`QUERY` | `service::definition` (adl14/adl2/query_store/wire modules) | implemented |
| Demographic | `I_DEMOGRAPHIC_SERVICE`, `I_PARTY`, `I_PARTY_RELATIONSHIP` | `service::demographic` | implemented |
| Query | `I_QUERY_SERVICE` | `service::query` | implemented |
| Validity checking | `I_VALIDITY_CHECKER` | `service::validity` | implemented |
| System Log | `I_SYSTEM_LOG` (stub; "IHE ATNA-compliant") | `ferroehr::system_log` (dual DICOM PS3.15 + FHIR `AuditEvent`/BALP rendering; local Audit Record Repository in the `audit` schema, on by default; syslog + ITI-20 ATX:FHIR Feed forwarding sinks; the ITI-81 retrieval as the read side; ITI-19 mTLS via `[server.tls]`) | implemented |
| Admin | `I_ADMIN_SERVICE` (+archive/dump-load) | `service::admin` | implemented |
| EHR Index | `I_EHR_INDEX` | `service::ehr_index` | implemented |
| Terminology | `I_TERMINOLOGY_SERVICE` | `service::terminology` (in-process `openehr-term` bundle + N simultaneously-materialised FHIR R4B servers, selected per call by an explicit terminology→provider route map with a `default` fallback; optional OAuth2 client-credentials per server; commit-time ac-code constraint-binding resolution) | implemented |
| Message | `I_MESSAGE_SERVICE`, `I_EHR_EXTRACT_SERVICE`, `I_TDD_SERVICE` | `service::message` | implemented |
| Subject Proxy | `I_SUBJECT_PROXY_SERVICE`, `I_DATA_BINDING` | `service::subject_proxy` | implemented |

| Crate | Role | Kind |
|---|---|---|
| `openehr-base` | BASE 1.3.0 | generated |
| `openehr-rm` | RM 1.2.0 — the domain model | generated |
| `openehr-am` | AM 1.4 + 2.4 (`v1_4`/`v2_4`) | generated |
| `openehr-term` | TERM classes + terminology bundle | generated + hand-written |
| `openehr-lang` | BMM/P_BMM object model | generated |
| `openehr-its` | Canonical JSON/XML + ITS-REST contract + runtimes + gates + Simplified Formats (`flat`: FLAT / STRUCTURED / Web Template) | generated + hand-written |
| `openehr-query` | AQL 1.1 lexer + parser + AST | hand-written |
| `openehr-adl` | ADL 2.4 engine: ADL2/cADL/ODIN parser, AOM2 validation, flattener, OPT2, ADL 1.4→2 conversion | hand-written |
| `openehr-codegen` | BMM/XSD/OAS → Rust generator (+ `emit-rm-model`) | tooling |
| `ferroehr-rest` | ITS-REST protocol adapter (axum) + auth + ATNA audit middleware; `access` module = RBAC/ABAC authz; calls the concrete `FerroEhrService` | application |
| `ferroehr` | The platform library: storage, service layer (one module per SM chapter), AQL engine, versioning, the full config tree, telemetry, `signing` + `system_log` | application |
| `ferroehr-server` | The wiring-only binary (config → pool → migrations → service → serve); bin name `ferroehr` | application |
| `ferroehr-ext` | Optional integrations behind additive features (`fhir`, `events`, `multimedia`): FHIR mapping/reverse/feeder-audit cores, the AMQP events transport, the content-addressed multimedia store | application |
| `testkit` | Shared test-database harness: one PG18 server + template-database cloning (`tools/*`) | tooling |

## Build state

The openEHR-conformant CDR is shipped: the generated spec/ITS foundation plus
the full application (persistence, greenfield storage, REST + auth, the SM
service layer, templates, WebTemplate/FLAT/STRUCTURED, validation, the AQL
engine, conformance). Everything not yet built — the plugin system, further
enterprise capabilities, refinement, performance — is ordinary tracker work:
an issue with a milestone, built as compiling, tested increments of our own
design. The build record is the closed issues + PR descriptions.

## Verification

- **Fidelity gates** (spec/serialization): canonical JSON read + lossless
  round-trip + ITS-JSON schema validation; XML round-trips.
- **Conformance pipeline** (`scripts/conformance.sh`):
  [Veredictum](https://github.com/rubentalstra/Veredictum), the independent
  CNF 2.0 reference runner, over its committed machine-readable catalogue
  (Docker-composed SUT on fresh volumes) — the acceptance instrument, pinned
  in `scripts/lib/veredictum.sh`;
  results → pure-function verdicts → report/statement/certificate + badges,
  all under `docs/conformance/<sut>/` (the baseline lives ONLY in those
  committed artifacts).
- **Measured performance** (the same instrument): `veredictum perf` earns the
  volumetric deployment classes (POC/S/L/R) by open-loop,
  coordinated-omission-free sustained runs (normative hour, extendable
  2–12 h — never shorter) whose re-checkable HDR-V2 records land in
  `results.json` `measurements`, environment-bound; `veredictum stress` is
  the separate step-load exploration instrument (maximum sustainable
  throughput, `stress.json`, never a conformance record). Published SVGs +
  summaries regenerate FROM the committed artifacts
  (`scripts/render/perf-assets.sh`, CI diff-guarded).
- **Drift check** (`scripts/checks/codegen-drift.sh` + CI): the generated layer
  is always in sync with the vendored specs.
