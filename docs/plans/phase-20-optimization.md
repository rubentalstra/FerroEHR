# P20 — Optimization (re-planned 2026-07-14 from measured evidence)

- Status: in-progress (W-5 executed: the stale plan below-the-fold is
  replaced by this measurement-driven one; owner go 2026-07-14)
- Mission (owner): **beat upstream EHRbase on max sustained throughput
  too** — the one benchmark row we lose. Honest baseline to beat
  (identical fine ladder, 10k rung, single-host preview, W-11):
  **ehrbase-rs 320 req/s @ p99 131 ms · upstream 956 req/s @ p99 497 ms.**
  We already hold every other measured row (memory ~6×, CPU ~2×, all
  clinical-load p99 classes, storage/composition).
- Discipline: profile → fix → re-ladder. Every step ends with the identical
  fine ladder re-run; only measured improvements are claimed; ECC zero-drift
  at close; behavioural changes out of scope (conformance stays green).

## The measured evidence (W-11 overnight, committed)

1. **Write path saturates first.** At L=40 (~400 req/s offered) p99 blows to
   892 ms with only 0.23% errors — latency, not shedding, is the ceiling.
   Server logs under load: pool-acquire waits 2→15 s; `vo_version` INSERT
   1.6 s; `node` INSERT 2.2 s; single-row version SELECTs ~2 s — *everything*
   slows together ⇒ PG-level contention, not one bad statement.
2. **AQL plans degrade with scale.** The patient-dashboard query (CONTAINS +
   ehr_id filter + ORDER BY time) runs 2 s+ at the 10k rung and flips the
   AQL rows to upstream at 100k (patient 184 vs 126 ms, ward 155 vs 83 ms).
   The ORDER BY re-extracts jsonb per candidate row via a correlated
   subquery; no expression index serves it.
3. **Node writes are already single-statement** (`node_repo::write_nodes`
   multi-row `push_values`) — the per-commit cost is spread across the
   *statement sequence* (validation reads, vo_version + audit + contribution
   inserts, subject sync, temporal-GiST maintenance) and PG configuration
   (container-default `shared_buffers` etc. — equal for both SUTs, but our
   write path leans on PG harder).
4. Upstream survives ~3× higher offered write load at ~3.8× our knee
   latency — their per-commit path is cheaper; ours does more per write
   (RM validation depth, temporal constraints).
5. **(T1 profile, 2026-07-14)** `template_store` content is re-read from PG
   on **every** composition commit (10,206 calls / 120 s window, #2
   statement by total time) — the validation path bypasses the template
   cache. → T3 first slice.
6. **F5 — the W-11 workload committed empty compositions (found via the T1
   profile's rows=0 anomaly).** 5 of 6 vendored CKM example skeletons
   (`tools/benchmark/templates/ckm/*.example.json`) have **no `content`**:
   the `/example` generator emits only mandatory nodes, and the CKM
   templates' content items are optional. Every ward-sim CKM write was an
   empty COMPOSITION and every patient-dashboard AQL measured a zero-row
   result set. The published W-11 comparison stays *fair* (both SUTs got
   byte-identical payloads) but not *realistic* — the numbers MUST be
   re-measured at T5 with populated documents, and README/COMPARISON/book
   refreshed from whatever the honest re-run says. Fix: the openehr-flat
   example builder populates optional content (bounded), pack regenerated.
7. **F6 — `SELECT c/uid/value` returns null (normative AQL defect).**
   QUERY master03 (identified-paths table) lists `COMPOSITION.uid.value` →
   `/uid/value`; our engine extracts from the stored fragment, but the
   OBJECT_VERSION_ID is injected only at the REST read path
   (`service/ehr/meta.rs`), never stored. Verified live: CONTAINS chains
   are correct (the zero rows were F5's data absence); only uid paths
   resolve null. Folded into T2 (same seam); ECC case follows.

## Tasks

- [x] T1. **Profiling harness**: `pg_stat_statements` enabled in the compose
      PG + a `scripts/profile.sh` that runs one capacity step (L=32/L=40)
      against the composed stack and dumps the top statements by total/mean
      time + `EXPLAIN (ANALYZE, BUFFERS)` for the AQL dashboard shapes —
      committed evidence file per run (`docs/benchmarks/profiles/`). Never
      optimize without a before/after pair. *(Done 2026-07-14: harness
      committed; first profile `20260714T070026Z-L32-10k.md` — its rows=0
      anomaly surfaced F5/F6, its statement table surfaced the template-cache
      bypass (finding 5, fixed as the T3 first slice) and confirmed the AQL
      extraction as the #1 statement.)*
- [x] T2. **AQL hot-path indexes**: promoted column or `IMMUTABLE`-expression
      btree for the dashboard ORDER-BY shape (context start_time per
      versioned object), and whatever T1 shows for the CONTAINS + ehr_id
      join order. Target: patient/ward AQL p99 back under upstream at 10k
      AND 100k. *(Done 2026-07-14: promoted-leaf registry
      (`storage/promoted.rs`) + `node.context_start` (migration 0008,
      backfilled, partial `(ehr_id, context_start)` btree) + the AQL
      column-substitution fast path; EXPLAIN-verified `Index Scan Backward
      using idx_node_context_start`; the p99-vs-upstream target is measured
      at T5's re-ladder.)*
- [ ] T3. **Per-commit statement budget**: measure the write-path statement
      sequence (T1), then collapse it — validation reads cached per
      template, vo_version+audit+contribution round trips reduced (CTE
      pipeline / batched statements), subject sync only on EHR_STATUS
      change. Targets from T1 numbers, not intuition.
      - [x] T3a. Template re-read eliminated (2026-07-14): `web_template_for`
        consulted `template_store` BEFORE the WebTemplate cache — 10,206
        redundant reads/120 s (the #2 statement). Now cache-first; template
        delete evicts (422 instead of an FK 500 for a racing commit).
      - [ ] T3b. Round-trip collapse from the T1 profile's remaining
        sequence (per commit: BEGIN, EHR EXISTS, advisory lock, 2×
        version-tree reads (~19k+20k calls/window), audit INSERT,
        contribution INSERT, vo_version INSERT, node INSERT,
        event_outbox INSERT): merge the audit+contribution+vo_version
        round trips (CTE pipeline / one multi-statement), fold the EXISTS
        into an existing read. Versioning semantics (RM common master06)
        must be byte-identical — orchestrator-reviewed.
- [ ] T4. **PG tuning as config parity** (both SUTs identically, documented
      in the parity table): `shared_buffers`, `max_wal_size`, checkpoint
      spacing; `synchronous_commit` stays ON (clinical durability — never
      traded). Temporal-GiST maintenance cost quantified before any schema
      move. PG18 AIO (`io_method`) evaluated here.
- [x] T2b. **F6 — uid projection** (folded into T2, same seam): `SELECT
      c/uid/value` returns the OBJECT_VERSION_ID wire string (QUERY
      master03 identified-paths table); ECC AqlBasic case added; verified
      live on the composed stack. *(Done 2026-07-14: engine synthesis from
      vo_version (version-correct under ALL_VERSIONS, live-verified) +
      **ECC-QRY-025** — the spine case asserted only the projected column
      path, the new case pins the projected CELL against the committed
      OBJECT_VERSION_ID.)*
- [x] T2c. **F5 — populated example generation**: the openehr-flat example
      builder emits optional content items (one instance each, bounded,
      recursion-guarded); the CKM pack examples regenerated from the
      composed server; each commits clean and `CONTAINS OBSERVATION`
      matches; snapshot deltas reviewed honestly. *(Done 2026-07-14:
      `medium` redesigned as the fully-populated single-instance
      committable level with constraint-aware leaves — temporal patterns,
      C_DURATION allowed fields, media-type code lists, container
      cardinality caps; `complete` = medium + second occurrences. All six
      CKM skeletons regenerated at medium, all commit 201, dashboard AQL
      returns rows. The corpus-wide committability guard now covers
      Required AND Medium across 40+ templates.)*
- [x] T2d. **F7 — validator name-blind sibling admission** (found by the
      widened F5 corpus guard; a server-side false-rejection conformance
      defect): templates reusing one archetype under a container with
      name-differentiated siblings had instances routed to the wrong
      sibling overlay and their children rejected as Unexpected. Routing
      now implements name-based differentiation (RM common master03
      §LOCATABLE; AOM 1.4 master04 §node_id; BASE master11 §Name-based
      Predicate): name-qualified siblings match strictly, the unqualified
      sibling admits the residual, cross-contamination still rejected.
      Reproduced + pinned on the vendored neurologist-examination template
      (three synthetic routing tests + the real-fixture test).
- [ ] T5. **Re-ladder + publish**: identical fine ladder both SUTs, hour
      profiles at 10k/100k re-run **with the F5-fixed populated workload**,
      README + COMPARISON refreshed with whatever the numbers say (the
      pre-F5 numbers measured empty CKM writes and zero-row dashboards —
      supersede them explicitly, both directions). Exit: the saturation row
      flips, or the honest residual gap is recorded with the next
      bottleneck named. Full ECC zero-drift run at close.

## State (2026-07-14, after the first optimization wave)

Committed on `claude/p20-optimization`: T1 (harness + first profile), T2/T2b/
T2c/T2d (promoted `context_start` + AQL fast path, uid projection + ECC-QRY-025,
populated examples + regenerated CKM pack, the F7 validator routing fix), T3a
(template-cache read eliminated). **Next, in order:** (1) full ECC run —
zero-drift gate + the new case live (baseline ratchets to include
ECC-QRY-025); (2) T3b round-trip collapse; (3) T4 PG parity tuning; (4) T5
re-ladder with the now-populated workload — the ONLY honest source of new
published numbers (all pre-F5 numbers measured empty CKM payloads and are
superseded, both directions: heavier real payloads may LOWER both knees).

## Superseded original scope (2026-07 draft, kept as input)

PG18 AIO tuning; pipelined hot-read path (`deadpool-postgres`) if measured
to help; `JSON_TABLE` codegen in the AQL SQL generator; criterion/divan
micro-benches. Each folds into T1–T5 only when the profiles point at it.
