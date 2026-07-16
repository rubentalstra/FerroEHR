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

<!-- populated from the full route inventory — one row per endpoint,
     columns: # | METHOD path | handler (file:line) | L-verdict | E-verdict | receipt -->

PENDING — inventory sweep in flight; populated in the next commit.

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

## 4. Fix waves

Filed after the probe pass; each wave lists its rows, the change, the gate
receipts, and the re-measured numbers.

## 5. Exit criteria

- [ ] §1 has one row per endpoint (the count matches the router inventory)
      and every row carries an L and E verdict with a receipt.
- [ ] §2/§3 rows all verdicted; every ladder error at lf≤64 named.
- [ ] All `OPT`/`DEFECT` rows fixed or explicitly deferred with a reason the
      owner can read (no silent deferrals).
- [ ] Fresh benchmark pair (knee + hour, both SUTs' shape) committed —
      numbers only claimed from that run.
- [ ] ECC zero-drift run committed (baseline 370·335·0 or better).
- [ ] WORKLIST W-14 row closed with the merged PR link.
