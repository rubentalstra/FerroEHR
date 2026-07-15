# P20 hunt #2 — DB stratum (item 35 a / c / d)

**Owner PRIO 2026-07-15, "the second full hunt".** With round trips lean
(items 33/34, `d75dd75d4..b71b6199d`), this measures the *cost of what
remains* on the DB side: (a) per-statement EXPLAIN at the 10k shape, (c)
protocol/driver repricing, (d) the item-22 group-commit A/B.

**Method (measured, not asserted).** A throwaway integration test
(`app/ehrbase/tests/zz35db_hunt.rs`, prefixed `zz35db_`, deleted after harvest)
against a fresh **testcontainers PostgreSQL 18**, seeded to the 10k rung shape
— **200 EHRs · 10,400 vo_version rows (10,000 COMPOSITION + 200 EHR_STATUS +
200 FOLDER) · 200,400 node rows · 10,400 audit · 10,400 contribution**, then
`ANALYZE`. Every statement is the verbatim text from the source
(`storage/version_repo.rs`, `storage/node_repo.rs`, `storage/ehr_repo.rs`,
`service/ehr/meta.rs`); the two AQL queries are built through the real planner
(`ehrbase::aql::{plan, build_sql}`) so the SQL is exactly what ships.

**Contention caveat (owner note):** a concurrent agent profiled the app-CPU
stratum during part of this run; the group-commit A/B (d) especially is marked
directional. The EXPLAIN plans (a) are deterministic (row/buffer counts) and
not contention-sensitive.

**The single most important finding is a measurement correction:** the app
connects to Postgres as a **non-superuser** role (`docker/postgres/initdb/
10-ehrbase-init.sh`: `ehrbase` is `LOGIN … NOSUPERUSER`), so **Row-Level
Security is ENFORCED on every production/benchmark query** — the 17
multitenancy-scoped tables (`node`, `vo_version`, `audit`, `contribution`,
`ehr`, …) carry `ENABLE + FORCE ROW LEVEL SECURITY` from
`migrations/ehr/0004_multitenancy.sql`. `scripts/profile.sh` and all prior
EXPLAIN work connected as the `postgres` superuser, which **bypasses RLS
unconditionally** — so every plan captured to date under-represents production.
This hunt captured both passes (superuser and a `SET ROLE` non-superuser).

---

## Findings, ranked by expected knee/latency impact

### F1 (RANK 1) — `aql-patient` (the patient-dashboard query) still scans every current version in the DB: buffers scale with corpus, not result

**Evidence** (RLS-enforced plan, patient query at 10k shape):

```
Merge Join (Merge Cond: v1.vo_id = n1.vo_id)   Buffers: shared hit=337
  -> Index Scan using uq_vo_version_current on vo_version v1
         (actual rows=9802.00)   Buffers: shared hit=285     <-- scans ALL 9,802 current VOs
  -> Sort -> Bitmap Heap Scan on node n1 (rows=50)  Buffers: shared hit=52
-> Index Scan using pk_node on node n2 (loops=50)   Buffers: shared hit=1150  <-- OBSERVATION re-lookup
Execution Time: 5.592 ms   total Buffers: shared hit=1740
```

Generated SQL (verbatim, `build_sql`):
`… FROM "ehr" e0, "node" n1, "vo_version" v1, "audit" a_v1, "node" n2 WHERE
n1.rm_type IN ($7) AND n1.vo_id=v1.vo_id AND n1.sys_version=v1.sys_version AND
a_v1.id=v1.audit_id AND upper_inf(v1.sys_period) AND v1.branch_number=$8 AND
n1.ehr_id=e0.id AND n2… AND e0.is_queryable=$10 AND e0.id=CAST($11 AS uuid) …`

Two costs, both avoidable:

1. **`v1` (vo_version) is joined to `n1` only on `(vo_id, sys_version)` — it
   carries NO `ehr_id` predicate**, even though the query is scoped to one EHR
   (`e0.id = $11`, and `n1.ehr_id = e0.id`). The planner therefore scans the
   whole `uq_vo_version_current` partial index (**9,802 rows / 285 buffers**)
   and merge-joins it to the 50 patient rows. This is a **Fix-B-class
   buffers-scale-with-corpus defect**: at 10k COMPOSITIONs it is 285 buffers;
   at 1M it is ~29,000 buffers (~220 MB touched) **per dashboard query**,
   independent of the 20-row result. The measured item-33 gap (aql median
   36.8 ms vs upstream 28.2 ms under load) is consistent with this scan.
2. **`n2` (the `CONTAINS OBSERVATION` node) is re-looked-up per composition**
   (`loops=50`, **1,150 buffers** = ~23/comp) via `pk_node` then
   `Filter: rm_type='OBSERVATION'` discarding 19 of 20 rows each — it reads the
   whole node subtree of every candidate composition to find one node.

**Proposed fix (generator, orchestrator-owned — `app/ehrbase/src/aql/sql/`):**
when a versioned-object root variable (`v1`) is join-linked to a bound EHR
(`n1.ehr_id = e0.id` and `e0.id` is a uuid-literal/`ehr_ids` scope), emit
`v1.ehr_id = e0.id` too. That lets `v1` use `idx_vo_version_ehr (ehr_id, kind)`
(≈50 rows for the patient) and the planner drive a nested loop from the 50
patient rows instead of a full-current scan — bounding the plan by result, not
corpus. This is the exact shape of the already-shipped item-24/Fix-B work
(promoted-column / ehr-scoped gating) applied to the `CONTAINS`-chain
`vo_version` join. `n2`'s per-comp subtree read is inherent to `CONTAINS`
descendant search, but scoping `v1` first shrinks the candidate set the
`n2` loop runs over. ECC AqlBasic/QueryProvisioning + the `service_aql`
byte-identity tests gate it.

### F2 (RANK 2) — the multitenancy extension taxes every write (per-row tenant FK triggers) and every read (RLS predicate), always-on even with tenancy OFF

`migrations/ehr/0004_multitenancy.sql` adds, to **17 tables incl. `node`,
`vo_version`, `audit`, `contribution`, `ehr`**: a `tenant_id uuid NOT NULL FK →
tenant` column, plus `ENABLE + FORCE ROW LEVEL SECURITY` with a
`tenant_isolation` policy. Two distinct costs:

**(a) Write path — a foreign-key trigger fires per inserted row.** The folded
commit-CTE EXPLAIN shows the trigger tail:

```
===== commit_new_version folded CTE (INSERT) =====   Execution Time: 0.569 ms
Trigger for constraint fk_audit_tenant on audit: time=0.136 ...
Trigger for constraint fk_contribution_tenant on contribution: time=0.019 ...
Trigger for constraint fk_vo_version_tenant on vo_version: time=0.028 ...
(+ fk_contribution_ehr/audit, fk_vo_version_contribution/audit/template/ehr)
```

That is **3 tenant-FK checks for the version spine alone**; the node bulk
insert adds `fk_node_tenant` **once per node row** — 20 for a vital-signs
composition, **160–400 for an IPS**. Every check re-probes the same single
`tenant` PK row (the nil default tenant). The FK triggers are unconditional —
they fire whether tenancy is on or off — and are a measurable slice of the
per-commit budget (the trigger tail here is ~0.4 ms of the 0.57 ms folded
insert, *before* the node-row multiplier).

**(b) Read path — RLS adds `tenant_id = ext.current_tenant_id()` to every
scan.** Comparing the superuser plan (RLS bypassed) with the non-superuser
plan (RLS enforced) for the same statements:

| statement | superuser | RLS-enforced | delta |
|---|---|---|---|
| current_composition_meta | 13 buf / 0.063 ms | 15 buf / 0.058 ms | ~none |
| current_version_meta_by_kind | 6 buf / 0.015 ms | 6 buf / 0.031 ms | ~none |
| read_rows (node) | 23 buf / 0.081 ms | 23 buf / 0.093 ms | ~none |
| read_current version_select | 6 buf / 0.043 ms | 6 buf / 0.030 ms | ~none |
| aql-patient | 1740 buf / 4.6 ms | 1740 buf / 5.6 ms | +planning |

`ext.current_tenant_id()` is `STABLE PARALLEL SAFE`
(`migrations/ext/0002_tenant_context.sql`), so the predicate folds to a
**constant** evaluated once per query — buffers are unchanged and per-row
execution cost is negligible in single-tenant mode. The RLS costs that *are*
real: (i) it **defeats index-only scans** — the superuser `aql-patient` used
`Index Only Scan on pk_audit (Heap Fetches: 50)`; under RLS it becomes an
`Index Scan` with a `tenant_id` filter, forcing 50 heap visits (150 buffers)
because `tenant_id` is not in the index; (ii) **planning-time overhead** —
`aql-patient` planning was 0.74 ms (superuser) vs **7.26 ms (RLS)** on first
plan (amortized by the per-connection prepared-statement cache thereafter, see
F4); (iii) 16 bytes/row of `tenant_id` storage on every node.

**Proposed fix (greenfield mandate applies):** the RLS `WITH CHECK` +
`NOT NULL` already constrain `tenant_id`; the `fk_<table>_tenant` foreign key is
belt-and-suspenders that costs a trigger invocation per row. **Drop the per-row
tenant FKs on the hot write tables** (`node` above all — it is the per-node
multiplier — plus `vo_version`/`audit`/`contribution`); keep the column + RLS
policy + NOT NULL. That removes 20–400 trigger calls per composition commit at
zero isolation cost (RLS still enforces visibility; a bad tenant_id is
impossible under the auto-stamping DEFAULT + WITH CHECK). Separately, for
single-node deployments where tenancy is never used, a config to **not FORCE
RLS / connect as a BYPASSRLS role** would recover index-only scans and the
planning overhead — but that is a deployment choice, not a schema change.
Owner call required (multitenancy is an E2 extension). ECC + the E2
`tenant_isolation` integration test gate any change.

### F3 (RANK 3 — informational, healthy) — every lean single-object read is index-served and O(result); no seq scans on a hot path at scale

All the item-33/34 lean meta reads are tight, index-served, and buffer-bounded
by the result (RLS-enforced numbers):

| statement | plan | buffers | exec |
|---|---|---|---|
| current_composition_meta | pk_vo_version → pk_audit → idx_ehr → pk_node | 15 | 0.058 ms |
| current_version_meta_by_kind (EHR_STATUS) | idx_vo_version_ehr → pk_audit | 6 | 0.031 ms |
| current_version_meta_scoped | uq_vo_version_current → pk_audit | 6 | 0.016 ms |
| directory_current_meta | ehr_folder → uq_vo_version_current → pk_audit → ehr | 11 | 0.070 ms |
| read_current (version_select + LATERAL attest) | uq_vo_version_current → pk_audit → LATERAL | 6 | 0.030 ms |
| read_rows (node by version) | bitmap pk_node, 20 rows | 23 | 0.093 ms |
| first_version_root | pk_node | 7 | 0.027 ms |
| all_version_meta | pk_vo_version → pk_audit | 6 | 0.009 ms |
| version_at / directory_get_at_time | pk_vo_version | 3 | 0.006 ms |
| read_subtrees_canonical (1 anchor) | unnest → pk_node interval | 23 | 0.091 ms |
| aql-ward (LIMIT 50) | **pk_ehr index scan, Limit 50** | 2 | 0.035 ms |

`aql-ward` deserves a note: with the service-composed `LIMIT` it is a bounded
`pk_ehr` top-N (2 buffers) — O(limit), not O(EHRs). (The first pass, which
omitted the limit, seq-scanned + sorted all EHRs; the ward query is only
healthy *because* the LIMIT reaches the SQL — confirmed it does.) No hot-path
statement seq-scans a large table: the `Seq Scan on ehr` seen in a few plans is
the planner's choice at 200 rows and flips to `pk_ehr` as the table grows (the
RLS `aql-ward` already chose `pk_ehr`). **Verified not a problem.**

Two micro-notes (not knee-relevant, recorded for completeness):
- `current_vo(ehr, 'COMPOSITION')` bitmap-scans all 50 of an EHR's
  compositions (52 buffers) because COMPOSITION is not singleton per EHR — but
  the callers only pass singleton kinds (EHR_STATUS → ~3 buffers). No live
  mis-call; noted so a future COMPOSITION caller does not regress.
- `directory_current_meta` / `ehr_exists` seq-scan `ehr`/`ehr_folder` at 200
  rows; both are PK/`pk_ehr_folder`-served at scale.

### F4 (RANK 4) — the folded commit CTE plan is correct and cheap; the residual is WAL + FK triggers, not statement shape

`commit_new_version` executes as one `INSERT … WITH a, c, v` — audit +
contribution + vo_version in a single statement (15 buffers, 0.57 ms incl.
triggers). The tip-close `UPDATE` (skipped on create) is a single
`uq_vo_version_tree` index scan (2 buffers). The **one-open-row partial-index
maintenance does NOT dominate** — the CTE is a flat insert; the cost sinks are
(i) the FK triggers (F2a) and (ii) WAL fsync (F6/d). This confirms item-20's
close note: the write path's residual is un-pipelinable (WAL + index
maintenance), not round-trip count.

---

## (c) Protocol / driver repricing at the new statement counts

**Raw round-trip latency** (warm connection, prepared `SELECT 1`, 2000 iters):
**179 µs/RTT** on the testcontainers TCP socket (Docker-Desktop-VM inflated; a
bare-metal Unix socket is ~30–50 µs — treat 179 µs as an upper bound).

**(i) Pipelining (`tokio-postgres`/`deadpool-postgres` vs sqlx).** With the
folded CTE, a signing-off composition **create** runs ~2 data statements inside
the tx (`commit_new_version` CTE + `write_nodes` bulk insert) plus `BEGIN` +
`COMMIT` ≈ 4 network flushes. The theoretical pipelining saving is bounded by
the *dependent* structure: `write_nodes` FK-depends on the `vo_version` row the
CTE just wrote, and `COMMIT` depends on both — so only `BEGIN` (and, marginally,
the send of the node insert) can overlap. Ceiling ≈ **1 RTT saved ≈ 179 µs
(Docker) / ~40 µs (bare-metal) per create**, against a commit whose real cost
is WAL fsync (F6) and the FK-trigger tail (F2a). The cross-driver surface cost
(a second pool + connection lifecycle, duplicating the `jiff`/`uuid`/`search_path`
setup, and losing sqlx's compile-time-checked binds) is high for a sub-RTT win.
**Repricing confirms item-20's skip decision holds at the new counts** — the
independent statements were already collapsed into the CTE; what is left is
sequential-by-dependency, which pipelining cannot overlap. Verified not worth it.

**(ii) sqlx prepared-statement reuse.** `statement_cache_capacity` is left at
the sqlx default (100) per physical connection (`db/pool.rs` sets none). The
fixed hot-path statement set is ~30 distinct static SQL texts — comfortably
under 100 — so each is prepared once per connection and **reused across pool
checkouts** (sqlx's per-connection cache is persistent). Critically, the AQL
generated SQL text is **stable across patients**: item-24 made the `ehr_id`
literal a bound parameter (`e0.id = CAST($11 AS uuid)` in the captured SQL, not
an inlined literal), so `aql-patient` is **one** prepared statement text, not
one-per-patient. F12's "each unique AQL re-PREPAREs" churn therefore does *not*
bite the benchmark's fixed query set; it would only matter for a tenant issuing
>100 *structurally distinct* ad-hoc AQL texts per connection. **Verified not a
problem for the measured workload**; if heavy ad-hoc AQL ever appears, bump
`statement_cache_capacity` — cheap, no schema change.

**(iii) Pool acquire at saturation.** `acquire_timeout = 30 s`, `max = 50`
(bench). Post item-3, a create holds **one** pool connection end-to-end, so
hold time is short and the pool turns over fast. The tower admission shed
(256/2048) sits *in front* of the pool and converts overload into 503s before
acquisitions queue for 30 s — so the acquire timeout is not the knee mechanism;
shedding is. A fast-fail-and-retry acquire policy would only re-implement the
admission layer one level down. **Directional: not the knee**; leave the
timeout, rely on the admission shed. (No change proposed.)

---

## (d) Group-commit A/B (item 22) — executed on the current write path

32 concurrent commit loops × 40 commits each (the write-path statement shape:
audit+contribution+vo_version CTE + 20-row node insert per commit), 3 trials,
best + mean reported. `synchronous_commit` stays ON throughout (durability
intact). **Directional — testcontainers on Docker-Desktop disk, and a
concurrent CPU-profiling agent shared the host.**

| config | best | mean | commits/s (best) |
|---|---|---|---|
| **commit_delay=0, wal_compression=off** (default) | 612 ms | 704 ms | **2091** |
| commit_delay=1000 µs, commit_siblings=5 | 643 ms | 668 ms | 1991 |
| commit_delay=2000 µs, commit_siblings=5 | 650 ms | 688 ms | 1968 |
| commit_delay=1000 µs + **wal_compression=on** | 1454 ms | 1511 ms | **880** |

**Verdict — item 22 is a "verified not a win; do not enable":**
- `commit_delay` (1000–2000 µs) **lowers best-case throughput slightly**
  (2091 → ~1970 commits/s) while **narrowing the mean/best spread** (it trades a
  small latency floor for less tail) — a wash at best on this setup, no clear
  win. The research's "modest gains" did not materialize here; the write
  concurrency (32) already amortizes fsync without a delay.
- **`wal_compression=on` is a large regression (2.4× slower, 2091 → 880
  commits/s)** — the CPU cost of compressing many small WAL records dominates
  on a fast local device. This is the clearest signal in the A/B.

**Recommendation for the `BENCH_PG` floor and production default:** keep
`commit_delay=0` and **`wal_compression=off`**. Do not adopt item-22's
group-commit tuning — it is not a win on this write profile, and
`wal_compression` actively hurts. (Caveat: on a *slow/network* WAL device with
*low* write concurrency, `commit_delay` can help; the bench floor is neither, so
the default stands. Re-test only if the deployment target has slow fsync.)

---

## Summary — what to action vs what is settled

| # | Finding | Status / action |
|---|---|---|
| F1 | `aql-patient` scans all current VOs (buffers ∝ corpus) + per-comp OBSERVATION re-read | **FIX (orchestrator, generator):** emit `v1.ehr_id = e0.id` to ehr-scope the `vo_version` join on a `CONTAINS` chain — bounds the plan by result. Highest expected knee impact. |
| F2 | Multitenancy: per-row tenant FK triggers on every write (node ×20–400/commit) + RLS on every read | **FIX candidate (owner call, greenfield):** drop `fk_node_tenant`/spine tenant FKs, keep column + RLS + NOT NULL. RLS read cost itself is ~nil in single-tenant (STABLE predicate) but defeats index-only scans + adds planning overhead. |
| F3 | All lean single-object reads index-served, O(result), sub-0.1 ms | **Verified not a problem.** No hot-path seq scan at scale. |
| F4 | Folded commit CTE plan correct; residual is WAL + FK triggers, not statement shape | Confirms item-20; no statement-shape change needed. |
| c-i | Pipelining second driver | **Skip confirmed** at the new counts: ceiling ~1 RTT, un-pipelinable dependent tail. |
| c-ii | sqlx prepared-statement reuse | **Verified not a problem:** stable SQL text (incl. AQL param-bound), ~30 texts < cache 100. |
| c-iii | Pool acquire at saturation | **Not the knee:** admission shed fronts the pool. No change. |
| d | Group-commit A/B (item 22) | **Verified not a win; do not enable.** Keep `commit_delay=0`, `wal_compression=off`. |

**Verification owed on any fix:** ECC zero-drift + the fresh-hour pair + a knee
re-ladder (per item-35 exit). F1 additionally gated by `service_aql`
byte-identity + AqlBasic/QueryProvisioning ECC; F2 by the E2 tenant-isolation
test.

*Throwaway instrument `app/ehrbase/tests/zz35db_hunt.rs` — DELETE after harvest
(not committed). Raw EXPLAIN output archived in the run log.*
