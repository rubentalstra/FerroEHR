# `benchmark` — the benchmark harness (tooling, not part of the app)

Load/latency measurement for the CDR; being overhauled for X1 (multi-SUT,
percentiles, resource footprint — `docs/plans/x1-comparison.md`).

- **Owner rule: no false claims — measured numbers only.** Every published
  number traces to a run with recorded environment (hardware, SUT
  image/config, dataset, warmup); no extrapolation, no "should be".
- Comparisons (EHRbase vs ehrbase-rs) are fairness-governed: identical
  datasets, identical request mixes, both SUTs Docker-composed on the same
  host class; document any asymmetry rather than hiding it.
- Report percentiles (p50/p95/p99), not means alone; include resource
  footprint (CPU/RSS) when the plan calls for it.
- Benchmarks inform performance optimization but never justify weakening
  conformance (correctness > speed).
- Gates: `cargo clippy -p benchmark --all-targets` +
  `cargo nextest run -p benchmark`.
