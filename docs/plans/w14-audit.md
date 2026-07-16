# W-14 — full code audit: every endpoint, every path (latency + error + optimization register)

Owner directive 2026-07-16 (expanded same day): **a FULL audit — not only the
high-p99 outliers. Probe everything: every endpoint, every request path, every
background path — find every place we can still optimize, and name every error
source.** Register-first like W-3c/W-3f: every row gets a receipt (measurement,
file:line, or an explicit "audited — nothing found" verdict) before any fix
wave starts. Branch: `claude/w14-audit`. Close: fix waves → fresh measured
pair → ECC zero-drift.

Standing rules apply: measured numbers only; spec citations only (never ADRs);
no test weakening; ECC baseline ratchets only upward.

**Owner rules for this phase (2026-07-16):**

1. **Greenfield — nuking rewrites welcome.** This is a greenfield application:
   when a probe shows a subsystem is structurally slow or structurally lossy,
   the fix is a proper redesign/rewrite of that path (W-3c/W-3f style), never
   a patch bolted onto a bad shape. No quick fixes; no deferrals dressed as
   fixes. A fix wave may nuke and re-author an entire module if that is the
   right shape.
2. **Spec triage on every finding.** `docs/specs/openehr/` is the oracle
   (`/spec-lookup`). Every OPT/DEFECT row gets a spec triage before its fix
   ships: (a) does the spec MANDATE the behaviour (then the fix must preserve
   it — cite file + section); (b) is the spec SILENT (then flag "no openEHR
   spec governs this — our own design" and optimize freely); (c) does the
   current code DIVERGE from the spec (then the fix is the spec-true
   behaviour, and it may supersede the perf concern). Status-code and wire
   findings triage against ITS-REST; versioning against RM common; AQL
   against QUERY. No fix merges without its triage line in the receipt.

## 0. Method

Three passes over the full surface, in order:

1. **Inventory** (this file, §1–§3): every HTTP endpoint, every background
   path, every cache, the full error surface — enumerated with file:line.
2. **Probe** (per-row): each row audited on both tracks —
   - **L (latency):** DB round-trips per request (count + shape), serial
     awaits that could parallelize, per-request allocation/serialization
     weight, lock/cache contention, N+1 loops, missing indexes vs the query
     shape, oversized responses.
   - **E (errors):** swallowed/discarded errors, wrong REST status mappings,
     `unwrap`-adjacent fallbacks, lossy `map_err`, error paths that
     log/allocate excessively, and the *named* sources of the measured ladder
     error rates (0.0052% at lf=16 → 0.0185% at lf=64 — pre-saturation, so
     these are real defects or admission artefacts, not overload).
3. **Verdict** per row: `OPT` (optimization filed, with expected win),
   `DEFECT` (error-handling defect filed), `CLEAN` (audited, receipt states
   what was checked), or `N/A` (not a runtime path). Fixes batch into waves;
   each wave ends with scoped gates; the phase ends with a fresh benchmark
   pair + ECC zero-drift run.

### Measured seeds (v3.0.3 pair, 2026-07-16 — the starting evidence)

Per-class p99 (hour run, ehrbase-rs, ms):

| class | n | p50 | p99 | max | note |
|---|---|---|---|---|---|
| comp-create-large | 4 | 79.6 | 128.0 | 128.0 | worst p99 — first probe target |
| comp-create-small | 213 | 39.9 | 93.9 | 99.6 | high n, p99 ~94 — write path |
| ehr-read | 2 | 82.4 | 91.1 | 91.1 | n=2, weak signal — re-measure |
| dir-update | 6 | 31.0 | 90.6 | 90.6 | |
| comp-update | 27 | 41.2 | 81.5 | 81.5 | |
| contribution-commit | 40 | 33.3 | 75.3 | 75.3 | |
| comp-read-latest | 501 | 22.0 | 74.6 | 147.6 | max 147 — tail outlier source? |
| aql-patient | 167 | 28.5 | 65.2 | 112.6 | |
| comp-read-version | 167 | 20.5 | 61.5 | 145.2 | max 145 — same tail question |
| dir-read | 43 | 18.1 | 49.0 | 49.0 | |
| status-update | 2 | 29.9 | 48.8 | 48.8 | |
| aql-ward | 14 | 19.6 | 39.4 | 39.4 | |
| ehr-create | 2 | 36.1 | 38.6 | 38.6 | |
| history-read | 21 | 13.5 | 33.4 | 33.4 | |

Knee-ladder error rates (pre-saturation errors are unnamed — E-track must
name every one): lf=16 → 0.0052%, lf=32 → 0.0079%, lf=64 (knee) → 0.0185%;
beyond the knee the shed layer answers cleanly (25–54% 503s, no OOM — W-12
behaviour, by design).

## 1. Endpoint register

Full route inventory (2026-07-16): **159 route mounts / 155 distinct handler
operations** — 127 API-subtree operations behind auth+overload, 32
public/operational. `BP` = `/ehrbase/rest/openehr/v1`, `RR` = `/ehrbase/rest`.
Every `api/**` handler runs the same spine: `into_parts` → `guarded_dispatch`
(`ehr_access::enforce` → `pep::pre_check` → group dispatcher →
`pep::post_check` → `AuditOpId`). Columns: L = latency verdict, E = error
verdict; ☐ = unprobed.

### 1a. Middleware / cross-cutting rows (audit once, applies to all)

| # | Layer / concern | Where | L | E | Receipt |
|---|---|---|---|---|---|
| M-1 | guarded_dispatch spine cost (enforce + pre/post PEP per request) | `api/mod.rs` | ◐ | ☐ | L probed → F-8/F-11 (§4b): near-zero ABAC-off; ABAC-on double-read = F-8 |
| M-2 | authn middleware (Basic/Bearer per request; argon2 cost on Basic!) | `router.rs:103-109` | ◐ | ☐ | L probed → F-11 clean (verified-cred cache + JWKS cache); E pending P-3 |
| M-3 | ATNA audit middleware (always installed, early-out if off) | `router.rs:113-116` | ☐ | ☐ | |
| M-4 | tenant middleware (when enabled) | `router.rs:95-102` | ☐ | ☐ | |
| M-5 | http_metrics + root_span | `router.rs:118-119` | ☐ | ☐ | |
| M-6 | overload shed layer — API subtree only; management/docs/status never shed | `router.rs:129` | ☐ | ☐ | |
| M-7 | tower-http stack order (request-id, trace, catch-panic, CORS, 16 MiB body limit, 30 s timeout, compression) | `router.rs:141-158` | ☐ | ☐ | |
| M-8 | timeout → 408 special-case mapping | `overview/error.rs:167` | ☐ | ☐ | |
| M-9 | content negotiation + body parse seam (`overview/negotiate.rs` — also the top unwrap-density file, 35) | `overview/negotiate.rs` | ☐ | ☐ | |
| M-10 | config-gated extensions mounted but handler-gated (disabled ⇒ 404 inside handler — routing slots always occupied) | inventory note | ☐ | ☐ | |

### 1b. EHR API (33 ops, `api/ehr/openapi_routes.rs`)

| # | Op | Where | L | E | Receipt |
|---|---|---|---|---|---|
| EHR-1 | GET BP/ehr (by subject) | `:74` | ☐ | ☐ | |
| EHR-2 | POST BP/ehr | `:93` | ☐ | ☐ | |
| EHR-3 | GET BP/ehr/{ehr_id} | `:116` | ◐ | ☐ | L probed → F-7 (§4b); seed: ehr-read p99 91 ms |
| EHR-4 | PUT BP/ehr/{ehr_id} | `:136` | ☐ | ☐ | |
| EHR-5 | GET …/ehr_status/{version_uid} | `:165` | ☐ | ☐ | |
| EHR-6 | GET …/ehr_status (at time) | `:189` | ☐ | ☐ | |
| EHR-7 | PUT …/ehr_status | `:209` | ☐ | ☐ | seed: status-update p99 49 ms |
| EHR-8 | GET …/versioned_ehr_status | `:235` | ☐ | ☐ | |
| EHR-9 | GET …/versioned_ehr_status/revision_history | `:259` | ☐ | ☐ | |
| EHR-10 | GET …/versioned_ehr_status/version (at time) | `:283` | ☐ | ☐ | |
| EHR-11 | GET …/versioned_ehr_status/version/{version_uid} | `:310` | ☐ | ☐ | |
| EHR-12 | POST …/composition | `:332` | ◐ | ☐ | L probed → F-1/F-3/F-4/F-5 (§4a); seed: comp-create-large p99 128 ms, small 94 ms |
| EHR-13 | GET …/composition/{uid_based_id} | `:359` | ◐ | ☐ | L probed → F-8/F-9 (§4b); seed: comp-read-latest p99 74.6, max 147.6 |
| EHR-14 | PUT …/composition/{uid_based_id} | `:383` | ◐ | ☐ | L probed → F-1/F-2/F-3/F-5 (§4a); seed: comp-update p99 81.5 |
| EHR-15 | DELETE …/composition/{uid_based_id} | `:407` | ☐ | ☐ | |
| EHR-16 | GET …/versioned_composition/{vo_uid} | `:436` | ☐ | ☐ | |
| EHR-17 | GET …/versioned_composition/{vo_uid}/revision_history | `:465` | ☐ | ☐ | seed: history-read p99 33 ms |
| EHR-18 | GET …/versioned_composition/{vo_uid}/version (at time) | `:494` | ☐ | ☐ | |
| EHR-19 | GET …/versioned_composition/{vo_uid}/version/{version_uid} | `:524` | ◐ | ☐ | L probed → F-9 (§4b); seed: comp-read-version p99 61.5, max 145 |
| EHR-20 | GET …/directory (at time) | `:550` | ◐ | ☐ | L probed → F-11 clean, 3 RT (§4b); seed: dir-read p99 49 ms |
| EHR-21 | PUT …/directory | `:570` | ◐ | ☐ | L probed → §4b dir-update note, F-1/F-2 apply; seed: dir-update p99 90.6 ms |
| EHR-22 | POST …/directory | `:590` | ☐ | ☐ | |
| EHR-23 | DELETE …/directory | `:610` | ☐ | ☐ | |
| EHR-24 | GET …/directory/{version_uid} | `:637` | ☐ | ☐ | |
| EHR-25 | POST …/contribution | `:659` | ☐ | ☐ | seed: contribution-commit p99 75.3 |
| EHR-26 | GET …/contribution/{contribution_uid} | `:686` | ☐ | ☐ | |
| EHR-27 | GET …/tags | `:711` | ☐ | ☐ | |
| EHR-28 | GET …/composition/{id}/tags | `:738` | ☐ | ☐ | |
| EHR-29 | PUT …/composition/{id}/tags | `:762` | ☐ | ☐ | |
| EHR-30 | DELETE …/composition/{id}/tags/{key} | `:787` | ☐ | ☐ | |
| EHR-31 | GET …/ehr_status/{id}/tags | `:814` | ☐ | ☐ | |
| EHR-32 | PUT …/ehr_status/{id}/tags | `:838` | ☐ | ☐ | |
| EHR-33 | DELETE …/ehr_status/{id}/tags/{key} | `:863` | ☐ | ☐ | |

### 1c. QUERY API (6 ops, `api/query/openapi_routes.rs`)

| # | Op | Where | L | E | Receipt |
|---|---|---|---|---|---|
| QRY-1 | GET BP/query/aql | `:39` | ◐ | ☐ | L probed → F-10 (§4b); seed: aql-patient p99 65, aql-ward 39 |
| QRY-2 | POST BP/query/aql | `:58` | ☐ | ☐ | |
| QRY-3 | GET BP/query/{name} | `:81` | ☐ | ☐ | |
| QRY-4 | POST BP/query/{name} | `:104` | ☐ | ☐ | |
| QRY-5 | GET BP/query/{name}/{version} | `:130` | ☐ | ☐ | |
| QRY-6 | POST BP/query/{name}/{version} | `:156` | ☐ | ☐ | |

### 1d. DEFINITION API (13 ops, `api/definition/openapi_routes.rs`)

| # | Op | Where | L | E | Receipt |
|---|---|---|---|---|---|
| DEF-1 | GET BP/definition/template/adl1.4 | `:52` | ☐ | ☐ | tpl-list class n=0 in hour run — measure separately |
| DEF-2 | POST BP/definition/template/adl1.4 | `:71` | ☐ | ☐ | opt-upload n=0 — measure |
| DEF-3 | GET …/adl1.4/{template_id} | `:94` | ☐ | ☐ | |
| DEF-4 | GET …/adl1.4/{template_id}/example | `:117` | ☐ | ☐ | example generation cost |
| DEF-5 | GET BP/definition/template/adl2 | `:136` | ☐ | ☐ | |
| DEF-6 | POST BP/definition/template/adl2 | `:155` | ☐ | ☐ | |
| DEF-7 | GET …/adl2/{template_id} | `:178` | ☐ | ☐ | |
| DEF-8 | GET …/adl2/{template_id}/example | `:201` | ☐ | ☐ | |
| DEF-9 | GET …/adl2/{template_id}/{version} | `:227` | ☐ | ☐ | |
| DEF-10 | GET BP/definition/query/{name} | `:250` | ☐ | ☐ | |
| DEF-11 | PUT BP/definition/query/{name} | `:264` | ☐ | ☐ | |
| DEF-12 | GET BP/definition/query/{name}/{version} | `:290` | ☐ | ☐ | |
| DEF-13 | PUT BP/definition/query/{name}/{version} | `:313` | ☐ | ☐ | |

### 1e. DEMOGRAPHIC API (42 standard + 8 relationship ext, `api/demographic/`)

Party CRUD is one code path (`party::run(kind, action)`) × 5 kinds — audit the
shared path once (DEM-P), spot-check per-kind divergence. Same for tags
(`tags::run`, DEM-T) and versioned-party reads (DEM-V).

| # | Op group | Where | L | E | Receipt |
|---|---|---|---|---|---|
| DEM-P | POST/GET/PUT/DELETE party ×5 kinds (20 ops, `openapi_routes.rs:80-365`) | `party.rs` | ☐ | ☐ | |
| DEM-V | versioned_party get / revision_history / version-at-time / version-by-id (4 ops, `:384-450`) | `versioned.rs` | ☐ | ☐ | |
| DEM-C | POST/GET contribution (2 ops, `:471,488`) | `contribution.rs` | ☐ | ☐ | |
| DEM-T | tags collection + per-kind get/update/delete (16 ops, `:506-758`) | `tags.rs` | ☐ | ☐ | |
| DEM-R | party_relationship ext: CRUD + versioned reads (8 ops, `relationship.rs:68-212`) | `relationship.rs` | ☐ | ☐ | |

### 1f. ADMIN + extensions (25 ops)

| # | Op group | Where | L | E | Receipt |
|---|---|---|---|---|---|
| ADM-1 | DELETE BP/admin/ehr/all (raw query walk for repeated ehr_id) | `api/admin/openapi_routes.rs:36` | ☐ | ☐ | |
| ADM-2 | DELETE BP/admin/ehr/{ehr_id} | `:56` | ☐ | ☐ | ties to Q-3/Q-4 delete loops |
| TRM-1..6 | terminology ext (6 ops) | `extensions/terminology.rs:88-193` | ☐ | ☐ | |
| EVT-1..5 | event_subscription ext (5 ops) | `extensions/event_subscription.rs:60-121` | ☐ | ☐ | |
| TEN-1..5 | tenant ext (5 ops) | `extensions/tenant_routes.rs:55-115` | ☐ | ☐ | whole-map cache clear on write (C-5) |
| FHR-1..7 | FHIR ext: ingest/search + mapping CRUD (7 ops) | `extensions/fhir.rs:102-198` | ☐ | ☐ | E-3 lossy sites live here |

### 1g. Public / operational surface (32 mounts)

| # | Op group | Where | L | E | Receipt |
|---|---|---|---|---|---|
| SYS-1 | OPTIONS BP + OPTIONS / (manifest) | `api/system/options.rs:189`, `router.rs:186-187` | ☐ | ☐ | 10 unwrap-class hits in options.rs |
| SYS-2 | GET RR/status, /health, RR/status/health | `overview/status.rs:37-60` | ☐ | ☐ | static — likely CLEAN |
| SMT-1 | GET RR/.well-known/smart-configuration | `smart/discovery.rs:258` | ☐ | ☐ | doc rebuilt per request from config |
| DOC-1 | openapi.json + 12 family docs + swagger UI (15 mounts) | `extensions/openapi.rs:209-488` | ☐ | ☐ | `extensions_document(cfg)` per request — cache? |
| MGT-1..11 | management surface (11 method-routes) | `extensions/management/mod.rs:274-427` | ☐ | ☐ | never shed — DoS surface? own AccessGuard |

Unshed/unauthed surface note (probe M-6): status/health/SMART/docs/management
sit outside both auth and the overload layer — verify none does unbounded work.

## 2. Background / long-running path register

One row per non-request execution path. Verdicts: OPT / DEFECT / CLEAN / N/A.

| # | Path | Where | Trigger | Risk noted at inventory | L | E | Receipt |
|---|---|---|---|---|---|---|---|
| B-1 | ATNA audit drain task | `system_log/sender.rs:134` | startup spawn | request path only `try_send`, drop on full queue — is the drop counted/logged? | ☐ | ☐ | |
| B-2 | Event outbox publisher loop | `extensions/events/publisher.rs:86,120-173` | startup + interval | drain loop hammers outbox until short batch; `sync_subscriptions` re-declares AMQP queues **every cycle** | ☐ | ☐ | |
| B-3 | FHIR outbound emitter loop | `extensions/fhir/outbound.rs:95,143-178` | startup + interval | builds+POSTs per COMPOSITION version; network-bound; cursor advance | ☐ | ☐ | |
| B-4 | Telemetry DB sampler | `telemetry/samplers.rs:45-50` | interval | pool/DB stat queries per tick | ☐ | ☐ | |
| B-5 | DB migrations at startup | `main.rs:172` → `db/migrate.rs:39-53` | startup | sequential EXT then EHR on one conn — cold-start cost (11.6 s measured) | ☐ | ☐ | |
| B-6 | S3 multimedia offload (commit path) | `extensions/multimedia/offload.rs:171-175` | per-request | tree rewrite synchronous; failed upload aborts commit; `offload.rs:149` drops source error | ☐ | ☐ | |
| B-7 | S3 blob put/get/delete | `extensions/multimedia/store.rs:111-154` | per-request | network I/O in request path | ☐ | ☐ | |
| B-8 | Health indicator probes | `main.rs:194-201` | per probe | — | ☐ | ☐ | |
| B-9 | Telemetry shutdown flush | `telemetry/mod.rs:215` | shutdown | spawn_blocking | ☐ | ☐ | |
| B-10 | Template load (lazy, no warm) | `templates/store.rs`, WebTemplateCache | first request per template | cold-first-hit latency; is a startup warm worth it? | ☐ | ☐ | |

### 2b. Known N+1 / per-item query loops (from inventory — probe each)

| # | Site | Shape |
|---|---|---|
| Q-1 | `service/message/export.rs:204-205` | one `read_version_by_ordinal` per selected version (export N+1) |
| Q-2 | `service/ehr_index/conflicts.rs:80-82` | query per EHR row |
| Q-3 | `service/admin/delete.rs:117-123` | `still_referenced` fetch_one per blob candidate |
| Q-4 | `service/admin/archive.rs:31-82` | four loops, exists/insert per ehr_id/party_id |
| Q-5 | `service/admin/dump_load.rs:592-657` | five per-item insert loops in tx (bulk load — batch-insert candidate) |

sqlx call-site density (probe order for the L-track): `storage/version_repo.rs` 92 ·
`definition/query_store.rs` 33 · `admin/dump_load.rs` 32 · `subject_proxy/store.rs` 30 ·
`definition/adl14.rs` 30 · `admin/delete.rs` 28 · `definition/adl2.rs` 24 ·
`subject_proxy/mod.rs` 24 · `ehr_repo.rs` 20 · `query/execute.rs` 16 · `ehr_index/index.rs` 16.

### 2c. Caches (probe: hit rates, invalidation correctness, sizing)

| # | Cache | Where | Note from inventory | Verdict | Receipt |
|---|---|---|---|---|---|
| C-1 | `created_ehr_repr` (moka, 4096, TTL 30 s) | `service/mod.rs:122` | TTL-only, no invalidate — stale-read window on status update? | ☐ | |
| C-2 | `web_templates` (WebTemplateCache) | `service/mod.rs:64` | invalidated on template op (`adl14.rs:261`) — ADL2 ops too? | ☐ | |
| C-3 | `ehr_access` (moka, single-flight) | `service/ehr/access.rs:167` | capacity-bounded, no TTL; invalidate via CommitEnv hook | ☐ | |
| C-4 | `plan_cache` (moka, keyed by query text) | `query/plan_cache.rs:69` | insert-only LRU; param-normalization? key = raw text | ☐ | |
| C-5 | `tenant_cache` (RwLock\<HashMap\>) | `service/mod.rs:50` | whole-map clear on any tenant op; RwLock on hot path | ☐ | |

## 3. Error-surface register

### 3a. Status-mapping seam (audit the whole match)

Central: `CallStatusType → ApiError` at `ehrbase-rest/src/overview/error.rs:64-97`;
`IntoResponse` at `:155` (timeout special-case `:167`); `StorageError → SmError`
bridge at `storage/error.rs:52` (Exception + `e.to_string()` — lossy, DEFECT
candidate). Probe: every variant maps to the ITS-REST-correct status; nothing
collapses to 500 that has a defined 4xx.

### 3b. Lossy `map_err` sites (source discarded into string/generic)

| # | Site | Verdict | Receipt |
|---|---|---|---|
| E-1 | `storage/error.rs:52` — StorageError→SmError Exception via `to_string()` (EVERY storage error crosses here) | ☐ | |
| E-2 | `extensions/multimedia/offload.rs:149` — `map_err(\|_\| …)` drops source entirely | ☐ | |
| E-3 | `extensions/fhir/mod.rs:320,428,494,566,572` — five string-collapse sites | ☐ | |
| E-4 | `system_log/{message.rs:187, sender.rs:123, mod.rs:79}` — Xml/Transport `to_string()` | ☐ | |
| E-5 | `templates/{runtime.rs:96, ingest.rs:42}` | ☐ | |
| E-6 | `telemetry/{mod.rs:130,150, layers.rs:56,68-69}` (String error type) | ☐ | |
| E-7 | `service/admin/dump_load.rs:355` | ☐ | |
| E-8 | `extensions/fhir/reverse.rs:44` | ☐ | |
| E-9 | `config/{loader.rs:326, mod.rs:161}` | ☐ | |
| E-10 | `extensions/multimedia/store.rs:71` | ☐ | |

### 3c. unwrap/expect/`let _`/`.ok()`/`unwrap_or_default` density (probe each file, worst first)

`overview/negotiate.rs` 35 · `multimedia/offload.rs` 21 · `terminology/bundle.rs` 20 ·
`fhir/mapping.rs` 14 · `aql/terminology.rs` 14 · `object_version_id.rs` 13 ·
`versioning/contribution.rs` 13 · `system_log/message.rs` 13 · `access/authn/mod.rs` 13 ·
`storage/codec.rs` 11 · `config/mod.rs` 11 · `authz/cedar.rs` 11 ·
`query_store.rs` 10 · `overview/params.rs` 10 · `authn/jwt.rs` 10 ·
`api/system/options.rs` 10 · then the ~30-file tail (≤9 each) — every file gets a
pass; receipt = "all benign (parses of static data / infallible)" or a DEFECT row.

### 3d. Ladder error naming (the measured 0.0052–0.0185% at lf 16–64)

- [ ] Instrument or log-mine a knee re-run: capture every non-2xx/timeout at
      lf=16/32/64 with status + path + error body → name each source here.

### 3e. Error enum inventory (reference)

`AuditError` `StorageError` `MultimediaError` `EventError` `DbError`
`TelemetryError` `ServiceError` `IndexError` `AqlError`+4 `KeyError`
`SigningError`/`SignError` `TerminologyRelationError` `SmError`/`CallStatusType`
`ServeError` `AuthzConfigError` `AuthzError` `AuthError` `RestError(ApiError)`.

### 3f. Existing PERF markers (fold into L-track)

- `service/admin/{mod.rs:34, archive.rs:15}` — cold-tier storage movement (spec-silent)
- `service/ehr/composition_validate.rs:39` — scans + reassembles every live COMPOSITION
- `service/ehr/composition.rs:281` — full version read-back after write (spec-fidelity)

## 4. Filed findings

One row per probed finding. Spec-triage column per owner rule 2:
**M** = spec-mandated (cite) / **S** = spec-silent (our design, free) /
**D** = spec-divergent (spec-true fix wins). Fix column names the wave.

### 4a. Write path (probe P-1, 2026-07-16 — POST/PUT composition traced handler→SQL)

| # | Finding | Evidence | Triage | Fix |
|---|---|---|---|---|
| F-1 | **Default-on digest signing bypasses the P20 one-CTE commit fold.** `sig_preknown = client_signature.is_some() \|\| !signer.enabled()` and digest signing defaults to enabled → every default deployment runs the SPLIT path: `write_contribution` + `insert_vo_version` as two statements (signature computed over the returned `time_committed` between them) instead of the folded `commit_new_version` CTE. The measured v3.0.3 numbers are the slow path. Create = 6 RT, but the fold only fires with signing off. | `versioning/change.rs:552,589-645`, `signature/config.rs:54,71-81`, `signer.rs:107-113`, `version_repo.rs:304,346,578` | **S (triaged 2026-07-16)** — `VERSION.signature` is 0..1 and server-side signing is nowhere mandated (RM common master06 §Digital Signature: "can be made", conditioned on PKI "in place"; serialisation explicitly TBD; no SM/ITS-REST/CNF obligation). `canonical_form` does include `commit_audit.time_committed`, and master06 §Contributions says time_committed "should be computed on the server" — the Rust process IS the server, so assigning it app-side (not DB `now()`) keeps conformance AND makes the signature pre-computable → the one-CTE fold fires with signing on. Redesign on that basis | ☐ |
| F-2 | **Update pays a serial 3-read placement trio in-tx**: `advisory_lock` → `lineage_tip` → `next_ordinal` are three sequential round-trips; the two reads are independent of each other and read the same `vo_version` rows — collapsible into one statement (the tip-close UPDATE may fold too). Update ≈ 11 RT total vs create's 6. | `change.rs:258-325`, `version_repo.rs:179,1110,1144,506` | ☐ spec-silent (storage mechanics) — flag our-own-design | ☐ |
| F-3 | **Decompose→reassemble double transform per commit (signing on)**: the just-decomposed rows are fully reassembled (`O(N log N)` sort + per-row path attach) solely to feed the signature, then dropped — and `return=representation` responses re-read + re-reassemble from the DB although that served form was in memory. | `change.rs:611`, `codec.rs:176-236`, rest `api/ehr/composition.rs:283-325` | ☐ spec-silent (mechanics); representation body itself ITS-REST-governed (content must be the committed version) | ☐ |
| F-4 | **`reject_duplicate_persistent` is a serial N+1 that scales with EHR size on the write path**: for persistent compositions, one `read_current` (2 statements + full reassembly) per live persistent vo_id, sequentially. Event compositions skip it (0 RT). | `composition_validate.rs:28-60`, PERF marker `:39` | **S (triaged 2026-07-16)** — spec-SILENT and SEC-undecided: RM ehr master04 §Persistent Compositions explicitly allows "more than one instance of some"; COMPOSITION has no uniqueness invariant; CNF master07 `create_composition-same_opt_twice` notes "lack of information in the openEHR specifications" and its Robot case is tagged `future` (not baseline). Our rejection stays as a PORT-NOTE'd choice; the fix is free to batch the check into ONE query (no per-vo reassembly) | ☐ |
| F-5 | **Large-composition penalty = ~6–7 full passes over the node set per create**: decompose walk + num_cap reverse pass + reassemble-for-signing (the only super-linear term, sort) + 2 validation walks + canonical serialize + SHA-256 + O(N·cols) insert bind. Pass consolidation is the comp-create-large lever. | `codec.rs:38-45,52-155,176-236`, `composition_validate.rs:86,93-97`, `integrity.rs:51-70`, `node_repo.rs:58-89` | ☐ validation walks spec-mandated (AM/RM conformance); *number of passes* spec-silent | ☐ |
| F-6 | CLEAN (partial): create/update under `Prefer: return=minimal` does **no** read-back (metadata-only response from memory — the old read-back already removed); create takes no advisory lock; no FOR UPDATE/SERIALIZABLE anywhere; `write_nodes` already one bulk statement. PERF marker at `composition.rs:281` sits on `template_of_version` (ABAC helper), NOT the create path — marker text to verify/reword. | `composition.rs:63-69,245-246`, `meta.rs:77-90` | n/a | note |

### 4b. Read path / dispatch spine (probe P-2, 2026-07-16 — GET composition/EHR/directory + AQL + authn/PEP traced)

| # | Finding | Evidence | Triage | Fix |
|---|---|---|---|---|
| F-7 | **EHR GET runs 4 serial DB round-trips, and the created-EHR cache is never consulted on read**: `ehr_header` → `current_version_meta_by_kind(EHR_STATUS)` → `current_vo(EhrAccess)` → `live_folder_hierarchies`, each a sequential pool-acquire+query; `created_ehr_repr` (C-1) is popped only by the post-create representation path. This is the ehr-read p99 91 ms. Redesign: one merged query (or concurrent reads) for the whole EHR summary. | `service/ehr/service.rs:209-289,446-459`, `ehr_repo.rs:150,175`, `version_repo.rs:1524,1585` | ☐ EHR representation ITS-REST-governed; query shape spec-silent | ☐ |
| F-8 | **ABAC-on composition GET pays a SECOND full composition reassembly**: `post_check` → `template_of_version` resolver does a whole 2-query read + reassemble just to learn the template id (the PERF-marked helper from F-6). Redesign: promote template_id to a version-row column (it is already known at commit). | `pep.rs:226-293,406-421`, `service/ehr/composition.rs:281-300` | ☐ spec-silent (authz out of band per SM) | ☐ |
| F-9 | Composition read shape: 2 sequential round-trips (version row, then nodes) — mergeable to 1; `reassemble` re-sorts rows already `ORDER BY num` from SQL (O(N log N) redundant) and re-splits the materialized path string per row. | `version_repo.rs:947-983`, `node_repo.rs:96-121`, `codec.rs:176-236` | ☐ spec-silent | ☐ |
| F-10 | AQL: terminology-expanded queries are **never plan-cached** (re-parse + re-expand + re-lower every call); `fetch_all` materializes the entire result page in memory (bounded only by paging); missing-id scan O(n·m). Whole-object projection already batched (1 subtree query per page — clean). | `execute.rs:180-236`, `exec.rs:56-129` | ☐ AQL semantics QUERY-governed; caching spec-silent | ☐ |
| F-11 | CLEAN receipts: dispatch spine near-zero when ABAC/RBAC off (two Option checks); `ehr_access` enforce is a moka hit steady-state (miss pre-warmed at create, negative-cached); Basic auth argon2 behind SHA-256-keyed verified cache (TTL-gated), JWT remote JWKS moka 5 min single-flight; composition ETag needs no extra query; `expand_multimedia` no-ops unless flag+engine; deleted version short-circuits to 204 pre-reassembly; directory read = 3 RT with in-memory subtree select; directory update pre-read already one merged join; AQL param-regex once per query, rows moved (not cloned) into RESULT_SET; EHR-scoped work on composition reads: none. | probe P-2 trace | n/a | note |

Directory-update note (EHR-21 seed 90.6 ms): the write decomposes + bulk-inserts
the entire folder tree per update — inherent to whole-tree versioning; the F-1
signing-fold and F-2 trio findings apply to its commit too (shared `update` path).

### 4c. Error surface (probe P-3, 2026-07-16 — status seam vs ITS-REST, storage bridge, 7-file unwrap sweep, ladder naming)

| # | Finding | Evidence | Triage | Fix |
|---|---|---|---|---|
| F-12 | **DEFECT: malformed `If-Match` is silently discarded** — `expected_from_if_match` maps unparseable values to `None` via `.ok()`, so a garbage If-Match runs as if NO precondition was sent (lost-update window) instead of 400/412. | `versioning/object_version_id.rs:255,257` | **D** — ITS-REST `Requests_and_responses.md` §If-Match (`:203`) requires the precondition honoured; fix = reject unparseable If-Match (also blueprint row 13's If-Match hardening) | ☐ |
| F-13 | **DEFECT: the storage→SmError blanket bridge loses SQLSTATE/constraint detail and mis-statuses whole error classes.** Only 2 constraints are hand-sniffed at call sites; every other `sqlx::Error` → stringified `Exception` → 500: serialization failure 40001 + deadlock 40P01 (conflict-class, belong 409/retry), any other unique/constraint violation (→ 409), and **`PoolTimedOut` under load → 500** (a prime named ladder-error source; overload semantics belong 503 + Retry-After per our W-12 contract). No structured log at the bridge — constraint/code dropped before any tracing. Redesign: central sqlx classifier (SQLSTATE → typed StorageError variants) at the bridge, not per call site. | `storage/error.rs:48-54`, `ehr_repo.rs:68-71`, `db/pool.rs:23-31` | **S** — HTTP mapping ITS-REST-governed (409/412 rows), overload semantics spec-silent (our W-12 contract) | ☐ |
| F-14 | **Error body shape has no spec anchor**: we emit `{error, message}` unconditionally; the spec's (MAY, `Prefer: return=representation`-gated) illustrative body is `{message, code, errors[DV_CODED_TEXT]}` (`Requests_and_responses.md:242-272`). 422's `{message, validationErrors[]}` is a documented PORT NOTE; the generic shape is not. Decide: adopt spec-example shape or PORT-NOTE the divergence. | `overview/error.rs:107-114,174-195` | ☐ **M(MAY)** — triage the shape decision vs ITS-REST + ECC goldens before touching (wire change!) | ☐ |
| F-15 | Note: SM-level `AuthFailure` always → 403; no 401 route exists from the service layer (401 only from authn middleware). Matches "authenticated-but-unauthorized → 403" discipline — verify intent, then PORT-NOTE. | `overview/error.rs:71` | ☐ | ☐ |
| F-16 | CLEAN: every other status mapping spec-correct (404/412/409/422/400/501 rows verified against `Requests_and_responses.md:218-235`); 408 for execution timeout is the SPEC'S OWN code (`:229` — 504/503 absent from the spec subset); Success/FileNotWritable/Exception→500 defensible. Unwrap sweep of the 7 worst-density files (negotiate, offload, bundle, object_version_id, contribution, authn, codec): **exactly one defect** (F-12); all other hits infallible/optional-header/server-data/test-only. | probe P-3 | n/a | note |
| F-17 | **Instrument finding (tools/benchmark)**: error counting is asymmetric — successes are warmup-filtered, errors are counted unconditionally (`measure.rs:106-127`), slightly overstating error_rate; and "error" conflates server-side non-expected status with generator-side 2 s dependency-misses (`drive.rs:38,874-878`). Split server vs generator errors + warmup-filter both, or the W-14 close pair mis-attributes. | `tools/benchmark/src/{measure.rs,drive.rs}` | **S** (our instrument) | ☐ |

## 5. Fix waves

Filed after the probe pass; each wave lists its rows, the change, the gate
receipts, and the re-measured numbers.

## 6. Exit criteria

- [ ] §1 has one row per endpoint (the count matches the router inventory)
      and every row carries an L and E verdict with a receipt.
- [ ] §2/§3 rows all verdicted; every ladder error at lf≤64 named.
- [ ] All `OPT`/`DEFECT` rows fixed or explicitly deferred with a reason the
      owner can read (no silent deferrals).
- [ ] Fresh benchmark pair (knee + hour, both SUTs' shape) committed —
      numbers only claimed from that run.
- [ ] ECC zero-drift run committed (baseline 370·335·0 or better).
- [ ] WORKLIST W-14 row closed with the merged PR link.
