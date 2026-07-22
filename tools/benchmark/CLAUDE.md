# `benchmark` — the benchmark harness (tooling, not part of the app) — CONDEMNED

Load/latency measurement for the CDR (`bench` bin: open-loop driver, seeding,
sampling, HDR histograms, render). **This crate is condemned (owner ruling
2026-07-22):** the durable measurement instrument is `cnf-runner` (`perf` /
`stress` / `perf-assets`). Nothing consumes `cnf-runner` as a library, and **no
new capability is ever added to `benchmark`** — its unique features migrate INTO
`cnf-runner`, after which the crate is DELETED (tracked upstream). Do not extend
it; do not build on it.

While it still exists and is run via `scripts/benchmark.sh`, the fairness rules
that govern any published measurement still bind:

- **No false claims — measured numbers only.** Every published number traces to
  a run with recorded environment (hardware, SUT image/config, dataset, warmup);
  no extrapolation, no "should be".
- Comparisons (EHRbase vs ehrbase-rs) are fairness-governed: identical datasets,
  identical request mixes, both SUTs Docker-composed on the same host class;
  document any asymmetry rather than hiding it.
- Report percentiles (p50/p95/p99), not means alone; include resource footprint
  (CPU/RSS) where the plan calls for it.
- Benchmarks inform performance work but never justify weakening conformance
  (correctness > speed).
- Payload variation stays inside AOM constraints by construction (jitter in FLAT
  space against each template's WebTemplate).
- Gates while it exists: `cargo clippy -p benchmark --all-targets` +
  `cargo nextest run -p benchmark`.
