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

## Tasks

- [ ] T1. **Profiling harness**: `pg_stat_statements` enabled in the compose
      PG + a `scripts/profile.sh` that runs one capacity step (L=32/L=40)
      against the composed stack and dumps the top statements by total/mean
      time + `EXPLAIN (ANALYZE, BUFFERS)` for the AQL dashboard shapes —
      committed evidence file per run (`docs/benchmarks/profiles/`). Never
      optimize without a before/after pair.
- [ ] T2. **AQL hot-path indexes** (ADR-008 reserved exactly this): promoted
      column or `IMMUTABLE`-expression btree for the dashboard ORDER-BY
      shape (context start_time per versioned object), and whatever T1
      shows for the CONTAINS + ehr_id join order. Target: patient/ward AQL
      p99 back under upstream at 10k AND 100k.
- [ ] T3. **Per-commit statement budget**: measure the write-path statement
      sequence (T1), then collapse it — validation reads cached per
      template, vo_version+audit+contribution round trips reduced (CTE
      pipeline / batched statements), subject sync only on EHR_STATUS
      change. Targets from T1 numbers, not intuition.
- [ ] T4. **PG tuning as config parity** (both SUTs identically, documented
      in the parity table): `shared_buffers`, `max_wal_size`, checkpoint
      spacing; `synchronous_commit` stays ON (clinical durability — never
      traded). Temporal-GiST maintenance cost quantified before any schema
      move. PG18 AIO (`io_method`) evaluated here.
- [ ] T5. **Re-ladder + publish**: identical fine ladder both SUTs, hour
      profiles at 10k/100k re-run, README + COMPARISON refreshed with
      whatever the numbers say. Exit: the saturation row flips, or the
      honest residual gap is recorded with the next bottleneck named.
      Full ECC zero-drift run at close.

## Superseded original scope (2026-07 draft, kept as input)

PG18 AIO tuning; pipelined hot-read path (`deadpool-postgres`) if measured
to help; `JSON_TABLE` codegen in the AQL SQL generator; criterion/divan
micro-benches. Each folds into T1–T5 only when the profiles point at it.
