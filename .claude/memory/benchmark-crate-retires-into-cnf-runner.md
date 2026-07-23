---
name: benchmark-crate-retires-into-cnf-runner
description: "Owner rulings 2026-07-22/23 — nothing consumes cnf-runner as a lib, ever; the benchmark lab is DELETED (2026-07-23), all measurement lives in cnf-runner's perf/stress/aql-probe/stress-compare"
metadata: 
  node_type: memory
  type: project
  originSessionId: 6ac52978-20dc-48c9-8e32-4ecb66b5f383
  modified: 2026-07-23T12:00:00.000Z
---

Owner ruling (2026-07-22): **nothing consumes `cnf-runner` as a library
dependency — ever** (it is the terminal instrument: lib consumed only by
its own bin/tests). Completed 2026-07-23 with owner sign-off:
`tools/benchmark`, `scripts/benchmark.sh`, `docker/benchmark/`,
`.github/workflows/benchmark.yml`, and `docs/benchmarks/**` are DELETED —
all measurement is native to cnf-runner (`perf` / `stress` / `aql-probe` /
`stress-compare`), the comparison page consumes committed
`docs/conformance/<sut>/stress.json`, and every instrument always seeds a
freshly composed empty SUT (no seed reuse).

**Why:** one perf pipeline owned by one instrument; a dep arrow (either
direction) creates a half-coupled interim state the owner explicitly
rejected.

**How to apply:** never resurrect a second measurement harness or a
lib-consumer of cnf-runner; new measurement capability lands as a
cnf-runner subcommand. Related: [[ecc-own-conformance-framework]].
