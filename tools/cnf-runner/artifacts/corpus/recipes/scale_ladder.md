# Recipe `scale_ladder` — the volumetric scale-class corpora

Deterministic generated-corpus contract for the `cnf.scale.*` keys the
performance schedule's cases reference (`cnf.scale.10k` / `cnf.scale.100k` /
`cnf.scale.1m` / `cnf.scale.10m`).

- Shape: N EHRs at 100 committed COMPOSITION versions each, where N is the
  key's EHR count (10k → 10,000 · 100k → 100,000 · 1m → 1,000,000 · 10m →
  10,000,000). The ~100-versions-per-EHR volume assumption honours the
  published lesson that per-EHR data volume dominates query cost.
- Seeding is strictly through the public API (create EHR, commit
  composition) — never a database backdoor; what the measured run reads is
  exactly what the SUT's own write path produced.
- EHR e (0-based creation order) receives compositions j = 0..100 in commit
  order; commit t (global 0-based task order, t = e·100 + j) carries the
  `bp_series(t mod 10)` payload (contract: `corpus/recipes/bp_series.md`)
  against the `cnf.blood_pressure` template (`cnf.opt.blood_pressure`).
- The seeded index (EHR ids + committed version uids, in order) is the
  measured run's addressing pool: reads and per-EHR queries cycle it
  deterministically (stride 2,654,435,761 — the 32-bit Fibonacci-hashing
  multiplier — over the pool, by arrival index).
- The ad-hoc query the `adhoc_query` workload operation executes over this
  corpus is the blood-pressure read scoped to one EHR:
  `SELECT c/uid/value, o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude
   FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o
   [openEHR-EHR-OBSERVATION.blood_pressure.v2]
   WHERE e/ehr_id/value = $ehr_id LIMIT 10`.

Implementation: `cnf-runner` `perf_run::seed_scale_ladder` (the reference
seeder); any runner reproducing this contract produces an equivalent corpus.
