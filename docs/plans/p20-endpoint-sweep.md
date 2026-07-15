# P20 — endpoint SQL round-trip sweep (checklist item 34)

Owner-mandated exhaustive sweep: drive **every ITS-REST operation once** against
a real PG18, count the SQL statements (round trips) each pays, and classify the
waste. This is item 33's probe method generalized from three hot paths to the
whole surface ("find everything, not only a few high ones").

## Method

A throwaway harness (`app/ehrbase/tests/zzz_sweep_endpoints.rs`, deleted after
this record) boots the full `ehrbase-rest` router over `EhrbaseService::new(pool)`
against testcontainers `postgres:18` — exactly the binary's wiring
(`ehrbase_rest::build_with`), auth disabled, admin enabled. A `tracing` Layer
counts every event whose target starts `sqlx::query` (one per executed statement
= one DB round trip) and captures the statement `summary` field, keyed to the
op driving it. Each op is driven once via `tower::oneshot`; the counter delta is
snapshotted tightly around the awaited request.

**Config state (matches the bench, checklist header):** signing on
(`digest_default`, but the item-5 reassemble is in-memory — it adds **no** SQL,
so counts are signing-independent), eventing **on** (`outbox_enabled = true`
default — every commit pays one `event_outbox` INSERT), audit off, tenancy off,
`test_before_acquire(false)`.

**Caveats on the numbers (measured, honest):**
- The pool's `after_connect` runs `SET search_path` when a new physical
  connection opens; if that fires inside an op's window it inflates that op by
  1. Observed once (on `ehr_get_by_id`). Warm-up drives one request first to
  open the min connections; residual noise is **±1** on the first op that grows
  the pool. All write/read cores below reproduced stably across runs.
- `COMMIT` counts as one statement (a round trip, but no server work).
- Counts are for the **default eventing-on** config; subtract 1 per commit for
  the eventing-off write core.

## THE TABLE (every op, SQL round trips desc)

`min` = minimal necessary for what the op fundamentally does (plain read 1–2;
versioned read 2–3; folded write core ≈ audit + vo_version + node + COMMIT ≈ 4,
+1 precheck, +1 eventing INSERT, +2 for a representation read-back). `waste` =
`sql − min` for the success path. Waste classes use the item-33 taxonomy:
**DRR** discarded post-commit re-read · **DBR** double-built response · **RSJ**
repeated slot/meta read · **MOF** meta overfetch · **PRS** per-write side SELECT
· **SEP** separable write (foldable into a sibling statement) · **EVT**
eventing INSERT (config, not a defect) · **PRE** pre-tx existence probe.

| op | method | status | sql | min | waste | class |
|---|---|---:|---:|---:|---:|---|
| `ehr_status_update` | PUT | 200 | 18 | ~10 | ~8 | MOF + DRR + DBR + EVT |
| `person_update` | PUT | 200 | 18 | ~10 | ~8 | RSJ×2 + DBR + EVT |
| `composition_update` | PUT | 200 | 17 | ~10 | ~7 | MOF + PRS + DBR + EVT |
| `directory_update` | PUT | 200 | 15 | ~10 | ~5 | RSJ + PRS + EVT |
| `person_delete` | DELETE | 204 | 14 | ~8 | ~6 | RSJ×2 + EVT |
| `ehr_create_with_id` | PUT | 201 | 12 | ~10 | ~2 | SEP + EVT + PRE |
| `directory_create` | POST | 201 | 12 | ~9 | ~3 | RSJ + PRS + EVT |
| `ehr_create` (repr) | POST | 201 | 11 | ~10 | ~1 | SEP + EVT |
| `ehr_create` (minimal) | POST | 201 | 11 | ~10 | ~1 | SEP + EVT |
| `composition_delete` | DELETE | 204 | 11 | ~8 | ~3 | PRS + EVT |
| `directory_delete` | DELETE | 204 | 11 | ~8 | ~3 | RSJ + PRS + EVT |
| `contribution_create` | POST | 201 | 10 | ~8 | ~2 | DRR + EVT + PRE |
| `person_create` | POST | 201 | 9 | ~6 | ~3 | DBR + EVT |
| `organisation_create` (sibling) | POST | 201 | 9 | ~6 | ~3 | DBR + EVT |
| `composition_create` (repr) | POST | 201 | 8 | ~8 | 0 | EVT + PRE (optimal) |
| `composition_create` (minimal) | POST | 201 | 6 | ~6 | 0 | EVT + PRE (optimal) |
| `composition_tags_update` | PUT | 200 | 6 | ~4 | ~2 | RSJ |
| `ehr_status_tags_update` | PUT | 200 | 6 | ~4 | ~2 | RSJ |
| `ehr_get_by_id` | GET | 200 | 5 (±1) | ~3 | ~2 | RSJ |
| `definition_query_store` (PUT) | PUT | 200 | 4 | ~2 | ~2 | RSJ |
| `person_get` | GET | 200 | 4 | ~3 | ~1 | RSJ |
| `admin_ehr_delete` | DELETE | 204 | 4 | ~3 | ~1 | — |
| `ehr_status_get_at_time` | GET | 200 | 3 | ~2 | ~1 | RSJ |
| `versioned_ehr_status_revision_history` | GET | 200 | 3 | ~3 | 0 | — |
| `versioned_ehr_status_version_get_at_time` | GET | 200 | 3 | ~3 | 0 | — |
| `versioned_composition_get` | GET | 200 | 3 | ~3 | 0 | — |
| `directory_get_at_time` | GET | 200 | 3 | ~2 | ~1 | RSJ |
| `definition_template_adl1.4_upload` | POST | 201 | 3 | ~2 | ~1 | — |
| `query_execute_stored_query` (GET) | GET | 200 | 3 | ~3 | 0 | — |
| `query_execute_stored_query_version` (GET) | GET | 200 | 3 | ~3 | 0 | — |
| `versioned_party_version_get_at_time` | GET | 200 | 3 | ~3 | 0 | — |
| `ehr_status_get_by_version_id` | GET | 200 | 2 | ~2 | 0 | — |
| `versioned_ehr_status_get` | GET | 200 | 2 | ~2 | 0 | — |
| `versioned_ehr_status_version_get_by_id` | GET | 200 | 2 | ~2 | 0 | — |
| `composition_get` (latest) | GET | 200 | 2 | ~2 | 0 | — |
| `composition_get` (by version) | GET | 200 | 2 | ~2 | 0 | — |
| `versioned_composition_revision_history` | GET | 200 | 2 | ~2 | 0 | — |
| `versioned_composition_version_get_at_time` | GET | 200 | 2 | ~2 | 0 | — |
| `versioned_composition_version_get_by_id` | GET | 200 | 2 | ~2 | 0 | — |
| `directory_get_by_version_id` | GET | 200 | 2 | ~2 | 0 | — |
| `contribution_get` | GET | 200 | 2 | ~2 | 0 | — |
| `definition_template_adl1.4_example_get` | GET | 200 | 2 | ~2 | 0 | — |
| `query_execute_adhoc_query` (GET, cold) | GET | 200 | 2 | ~2 | 0 | — |
| `query_execute_adhoc_query` (GET, warm cache) | GET | 200 | 2 | ~2 | 0 | — |
| `query_execute_adhoc_query_body` (POST) | POST | 200 | 2 | ~2 | 0 | — |
| `versioned_party_get` | GET | 200 | 2 | ~2 | 0 | — |
| `versioned_party_revision_history` | GET | 200 | 2 | ~2 | 0 | — |
| `ehr_get_by_subject` (miss) | GET | 404 | 1 | 1 | 0 | — |
| `ehr_tags_get` | GET | 200 | 1 | 1 | 0 | — |
| `composition_tags_get` | GET | 200 | 1 | 1 | 0 | — |
| `composition_tags_delete` | DELETE | 204 | 1 | 1 | 0 | — |
| `ehr_status_tags_get` | GET | 200 | 1 | 1 | 0 | — |
| `ehr_status_tags_delete` | DELETE | 204 | 1 | 1 | 0 | — |
| `definition_template_adl1.4_list` | GET | 200 | 1 | 1 | 0 | — |
| `definition_template_adl1.4_get` | GET | 200 | 1 | 1 | 0 | — |
| `definition_template_adl2_list` | GET | 200 | 1 | 1 | 0 | — |
| `definition_query_list` | GET | 200 | 1 | 1 | 0 | — |
| `definition_query_version_get` | GET | 200 | 1 | 1 | 0 | — |
| `demographic_tags_get` | GET | 200 | 1 | 1 | 0 | — |

**Note on the warm-cache query row:** the AQL plan cache (item 9) is a
parse/plan cache, not a SQL cache — cold and warm ad-hoc GET both execute 2
statements (the lowered SELECT + the per-page subtree reload). The cache saves
CPU (lex/parse/type/lower), not round trips.

## Top-15 outliers — where the waste lives (file:line call chains)

Ranked by wasted statements. The write paths (`aql/`, `service/ehr/status.rs`,
`service/ehr/service.rs`, `storage/ehr_repo.rs`, baseline migration) are being
edited concurrently under item 34's sibling work — line numbers are as of this
sweep (branch `claude/p20-hotpaths`).

### 1. `ehr_status_update` — 18 stmts, ~8 wasted (MOF + DRR + DBR)
The single worst op. Statement trace: 2 identical pre-tx version reads → advisory
lock + `ehr/kind` read + `MAX(sys_version)` + close-old `UPDATE vo_version` +
audit CTE + `INSERT vo_version` + `INSERT node` + `INSERT event_outbox` +
`UPDATE ehr` (subject sync) + COMMIT → **5** post-commit read statements
(two identical reassembly pairs + a version-meta read).
- **MOF double pre-read:** `replace_ehr_status` reads the version for If-Match at
  `app/ehrbase/src/service/ehr/status.rs:445` (`ehr_status_meta`), then
  `status_update` reads the *same* version again at `status.rs:69` (`current_vo`).
- **DRR discarded re-read:** `status_update` reassembles the whole updated
  EHR_STATUS at `status.rs:95` (`read_current` → `version_response`), but
  `replace_ehr_status` keeps **only the uid** from it via `version_uid(...)` at
  `status.rs:453` — the reassembled body is thrown away.
- **DBR double-built response:** the REST layer then rebuilds that exact
  representation from scratch at
  `app/ehrbase-rest/src/api/ehr/ehr_status.rs:98` (`get_ehr_status`).
- **Fix shape:** collapse the two pre-reads to one; have `status_update` return
  the already-reassembled body (or a uid under minimal) so the REST layer stops
  re-reading. Fold `sync_ehr_subject` (`status.rs:314`, a separate `UPDATE ehr`)
  into the write.

### 2. `person_update` — 18 stmts, ~8 wasted (RSJ×2 + DBR)
The demographic dispatcher resolves the party (SELECT `kind` + full read) before
calling the service, then `update_party` resolves it **again**:
- `app/ehrbase/src/service/demographic/party.rs:74` (`update_party` →
  `ensure_party`) re-runs `SELECT kind FROM vo_version` + read after the
  dispatcher already did.
- **DBR:** post-commit `read_party` at `party.rs:98` rebuilds the representation
  (SELECT `kind` + version + nodes) — a third full read of the same object.
- Trace: `SELECT kind` ×3 across the op, full version+node read ×3.
- **Fix shape:** thread the resolved `(vo_id, kind)` from the dispatcher into the
  service call; return the post-commit body instead of re-reading.

### 3. `composition_update` — 17 stmts, ~7 wasted (MOF + PRS + DBR)
- **MOF/pre-read:** the CONTAINS-uid resolution + `read_current` pre-read at
  `app/ehrbase/src/service/ehr/composition.rs:184` fetch the current version;
  the trace shows two full reassembly reads before the tx.
- **PRS per-write side SELECT:** `ensure_content_writable` at `composition.rs:193`
  is a standalone `SELECT (n.data->>'is_modifiable')…` that could ride the
  pre-read.
- **DBR:** REST re-reads for representation at
  `app/ehrbase-rest/src/api/ehr/composition.rs:306-310` (`get_composition_at_version`).

### 4. `person_delete` — 14 stmts, ~6 wasted (RSJ×2)
Double pre-read (dispatcher resolve + `delete_party`'s `load_party_version` at
`app/ehrbase/src/service/demographic/party.rs:113`), each re-fetching `kind`.
No post-read (204). Same dispatcher-vs-service duplication as #2.

### 5. `directory_update` — 15 stmts, ~5 wasted (RSJ + PRS)
- The `ehr_folder` → `vo_id` join runs **twice**: pre-tx at
  `app/ehrbase/src/storage/ehr_repo.rs:157` and **again** post-commit at
  `ehr_repo.rs:135` when building the representation (trace stmts 1 and 13 are
  the identical `SELECT f.vo_id FROM ehr_folder`).
- **PRS:** standalone `is_modifiable` SELECT (`ehr_repo.rs:178`).
- **Fix shape:** thread the resolved directory `vo_id` from the pre-read into the
  post-commit representation build (item-33 did this for the *inner write* but
  the representation read still re-joins `ehr_folder`).

### 6. `directory_create` — 12 stmts, ~3 wasted (RSJ + PRS)
`EXISTS` precheck + `is_modifiable` SELECT + `ehr_folder` join pre-tx, then the
`ehr_folder` join **repeats** post-commit (`ehr_repo.rs:135`, trace stmts 3 and
10 identical). Same RSJ as #5.

### 7. `composition_delete` — 11 stmts, ~3 wasted (PRS)
Full pre-read (version+nodes) + standalone `is_modifiable` SELECT
(`composition.rs:361` region, `ehr_repo.rs:206` combined `EXISTS`+is_modifiable)
+ advisory + close-old + audit + `INSERT vo_version` (tombstone) + `event_outbox`
+ COMMIT. The pre-read reassembles nodes only to check state — a `kind`/version
meta read would suffice.

### 8. `directory_delete` — 11 stmts, ~3 wasted (RSJ + PRS)
Same `ehr_folder`-join + `is_modifiable` side-SELECT pattern as #5/#6 on the
delete path.

### 9–10. `ehr_create` / `ehr_create_with_id` — 11 / 12 stmts, ~1–2 wasted (SEP)
Already well-folded (item-33 fix E: representation built from `Committed`, so
`return=representation` and `return=minimal` both cost 11 — no discarded
read-back). Residual: `INSERT INTO ehr` is immediately followed by a separate
`UPDATE ehr SET subject_id…` (`sync_ehr_subject`, `status.rs:314`) — **SEP**,
foldable into the initial INSERT's column list. `ehr_create_with_id` adds one
pre-existence `SELECT vo_id…` (PRE, legitimate).

### 11. `contribution_create` — 10 stmts, ~2 wasted (DRR)
`EXISTS` + `is_modifiable` + audit CTE + `INSERT audit/vo_version/node` +
`event_outbox` + COMMIT, then post-commit `SELECT audit` + `SELECT vo_version`
(2 stmts) to build the contribution response — a representation re-read of data
partly present in the commit result.

### 12–14. `person_create` / `organisation_create` — 9 stmts, ~3 wasted (DBR)
Write core is 5 (audit CTE + `INSERT vo_version` + `INSERT node` +
`event_outbox` + COMMIT — **no** `EXISTS` precheck, unlike composition), then
`read_party` rebuilds the representation post-commit (`SELECT kind` + version +
nodes = 4). The party create could serve the representation from the committed
data (as `ehr_create` now does). Siblings (`agent`/`group`/`role`) are
byte-identical — same `create` + `read_party` path keyed by `PartyKind`.

### 15. `ehr_get_by_id` — 5 stmts (±1), ~2 wasted (RSJ)
`SELECT system_id,time_created` (`ehr_repo.rs:109`) + `current_vo` for
EHR_STATUS (`service/ehr/service.rs:196`) + `version_creating_system_id`
(`service.rs:203`) + a **second** `current_vo`-shaped read of the same status
version + `ehr_folder` join (`ehr_repo.rs:135`). The status version metadata is
fetched twice; the per-version `creating_system_id` is a separate round trip
that could join the version read.

## Cross-cutting structural findings

These recur across the whole write surface — fixing them once (in the shared
`update`/`delete`/read helpers and the REST write-response path) drains most of
the table:

1. **Dispatcher-then-service double resolve (RSJ).** Both the demographic and
   directory paths resolve the target `vo_id`/`kind` in the REST/dispatch layer
   *and* again inside the service method. Thread the resolved handle through.
2. **Post-commit representation is built 1–3× (DRR + DBR).** The service commit
   helpers reassemble the served body, the SM adapter keeps only the uid
   (`version_uid(...)`), then the REST layer re-reads for `Prefer:
   return=representation`. One reassembly should flow end-to-end; under
   `return=minimal` none is needed. `composition_create` already does this
   correctly (0 waste) — it is the template for the rest.
3. **`is_modifiable` / existence as standalone side-SELECTs (PRS/PRE).** Every
   content write pays a separate `SELECT (n.data->>'is_modifiable')` and/or
   `EXISTS`. These can fold into the concurrency pre-read (one row already being
   fetched) or into the write CTE.
4. **`ehr_folder` join repeated post-commit (RSJ).** Directory create/update/
   delete re-run the `ehr_folder → vo_id` join to build the response after the
   write already knew the `vo_id`.
5. **`event_outbox` INSERT per commit (EVT).** Present on **every** write in the
   default (eventing-on) config (`versioning/change.rs:694,881`). Not a defect,
   but it is a guaranteed +1 round trip inside the held write tx on every
   commit; with no subscribers it is pure overhead (item 12 gated the consumer,
   not the INSERT). Worth an eventing-off fast path.
6. **`INSERT ehr` + `UPDATE ehr SET subject_id` split (SEP).** The EHR create
   writes the row then immediately updates its subject columns; fold into one
   INSERT.

## Ops probed vs skipped

**Generated ITS-REST catalogue** (from `crates/openehr-its/src/rest/generated/*`
`ROUTES` tables): ehr 34, demographic 45, query 6, definition 13, admin 2 =
**100 operation ids**.

**Driven directly:** 64 requests covering **~65 distinct operation ids**
(every ehr op; person full lifecycle + versioned_party ×4 + one sibling create;
5 of 6 query ops; 10 of 13 definition ops; 1 of 2 admin ops).

**Covered by dispatcher equivalence (not driven, identical measured path):** the
4 remaining demographic party types (`agent`/`group`/`organisation`/`role`)
CRUD + all party `*_tags_*` ops — the demographic group routes through one
`DemographicService` dispatcher keyed by `PartyKind`, and all item-tag ops share
one `ItemTagAdapter` (measured via `composition_tags_*` and `ehr_status_tags_*`).
`admin_ehr_delete_all` shares the `admin_ehr_delete` repo path. This accounts for
the ~35 op ids not individually driven.

**Skipped — could not drive to success (reason):**
- `definition_template_adl2_upload` → **400, 0 SQL.** The 2-line ADL2 stub
  source (borrowed from the Mock-backed `definition_adl2_http` test) is rejected
  by the real ADL2 parser before any DB access. Needs a valid ADL2 operational
  template fixture to measure the true store cost. (Consequently
  `adl2_get`/`adl2_example`/`adl2_version_get` for that id return 404.)
- `query_execute_stored_query_body` (POST) → **400, 0 SQL.** POST body `{}`
  rejected at parameter parsing before DB. The GET stored-query variants (3
  stmts) measure the execution cost; the POST-body variant differs only in
  parameter parsing.

**Not probed — extension surfaces (config/feature-gated off in
`RestConfig::default` / require `build_full`):** `/management/*` (observability,
only mounted via `build_full`), `/terminology/*` (config-gated off),
`/fhir/*` (feature-gated off), event-subscription routes, PARTY_RELATIONSHIP
extension routes, tenant routes (tenancy off). These are our-own-design
extensions, not core ITS-REST conformance surface; measuring them needs their
config/feature flags enabled and is out of scope for the core sweep. Flagged
here so the gap is explicit.

## Reproduction

Harness (throwaway, since deleted): `app/ehrbase/tests/zzz_sweep_endpoints.rs`.
`CARGO_TARGET_DIR=$PWD/target/agent-t1 cargo test -p ehrbase --test
zzz_sweep_endpoints -- --nocapture` → prints THE TABLE + per-statement
breakdown for the outliers. Requires Docker (testcontainers `postgres:18`).
