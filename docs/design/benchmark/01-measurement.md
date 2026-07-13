# Benchmark register 01 — measurement, outputs, crate design

Status: authored 2026-07-13 (W-11 A2). How the rewritten `tools/benchmark`
measures, what it emits, and the crate layout the B1 rewrite implements.

## 1. Latency

- One **HdrHistogram** (µs resolution, 3 significant digits) per operation
  class (register 00 §2 brackets: `ehr-create`, `comp-create-small`,
  `comp-create-large`, `comp-update`, `comp-read-latest`,
  `comp-read-version`, `contribution-commit`, `aql-patient`, `aql-ward`,
  `dir-read`, `dir-update`, `history-read`, `status-update`, `ehr-read`,
  `opt-upload`, `tpl-list`).
- **Coordinated-omission corrected:** the generator pre-computes planned
  send times (open loop); recorded latency = completion − *planned* send.
  A saturated SUT that delays sends cannot flatter its tail.
- Reported per class: p50 / p90 / p99 / p99.9 / max + count + error count.
  Warmup window (fixed floor per profile) discarded symmetrically.
- Errors: any non-expected status → the class error counter + excluded
  from latency; error rate > 0.1% flags the run.

## 2. Resources (CPU + RAM — compose-managed SUTs)

- Sampler polls the **Docker stats API** (`docker stats --no-stream
  --format json <app> <db>` subprocess) at ~1 Hz for the app AND db
  containers for the whole run: CPU%, memory RSS/limit.
- Emitted as a time series in `results.json` + summarized (idle baseline,
  mean, peak) in the report; derived efficiency numbers: req/s per
  CPU-core, req/s per GB peak RSS.
- **Cold start:** compose-up → first successful HTTP answer, measured per
  run. **Idle baseline:** 30 s pre-warmup sample.
- BYO SUTs: resource + storage sampling recorded as `unavailable` (honest
  gap), latency/throughput still measured.

## 3. Throughput + the knee

- Sustained req/s over the measurement window per profile run.
- The **knee series**: `hour` profile repeated at increasing load factor
  `L ∈ {1, 2, 4, 8, 16, …}` until p99 > 1 s or error rate > 0.1%;
  the report records the last sustainable `L`, its req/s, and the latency
  at that point (the capacity signal).

## 4. Storage footprint

Per scale rung, per SUT (compose-managed): `pg_total_relation_size` summed
over the SUT's schemas via `docker exec <db> psql` — bytes total and
bytes/composition. Recorded in `results.json`; compared honestly (both
sides decompose; no strawman).

## 5. Outputs (generated, never hand-typed)

```
docs/benchmarks/<sut-name>/
├── results.json     # the machine record (schema §6)
├── REPORT.md        # generated report
└── histograms/      # raw per-class HdrHistogram exports (base64 V2 format)
```

`REPORT.md` layout (benchmarking.md §7 inherited): environment block (host,
image digests, PG versions, pinned resources, harness SHA, workload.lock
hash, run count) → per-class latency table → throughput/knee → resource
efficiency → storage → cold start → "where the other side wins" (for
comparison runs) → limitations → reproduce-it commands.

## 6. `results.json` (essentials)

```json
{
  "sut": { "name", "kind", "base_url", "image_digests": {}, "versions": {} },
  "workload": { "lock", "profile", "scale", "ward_size", "load_factor", "seed" },
  "environment": { "host", "cpus", "mem", "harness_sha", "started" },
  "classes": { "<op-class>": { "count", "errors", "p50_us", "p90_us",
               "p99_us", "p999_us", "max_us", "histogram": "<HdrV2 base64>" } },
  "throughput": { "window_s", "requests", "rps", "error_rate" },
  "resources": { "app": {"idle_rss", "peak_rss", "mean_cpu", "series": []},
                  "db": {...}, "cold_start_ms": n },
  "storage": { "bytes_total", "compositions", "bytes_per_composition" }
}
```

## 7. Multi-SUT seam

Reuses the conformance crate wholesale — the client is provably the ECC
client:

- Transport/requests: `conformance::harness::{HttpRequest, HttpResponse,
  Transport}`; auth specs identical to `conformance run`.
- SUT selection: `conformance::sut` descriptors (`ehrbase-rs` compose /
  `ehrbase-java` compose / `byo --base-url`).
- Fixtures: `conformance::testdata::fixtures` for the OPT/composition
  skeletons.

## 8. CLI + entry point

```
bench run    --sut ehrbase-rs|ehrbase-java|byo [--base-url …] [--auth …]
             --profile smoke|hour|day --scale empty|10k|100k|1M
             [--ward-size N] [--load-factor L] [--knee]
             [--out docs/benchmarks]
bench seed   --sut … --scale …          # deterministic ladder seeding
bench report --from results.json        # regenerate artefacts
```

`scripts/benchmark.sh` mirrors `scripts/conformance.sh` (compose up the
selected SUT → `bench run` → down; `BENCH_SUT`/`BENCH_PROFILE`/… env).

## 9. Crate layout (B1 rewrite; files ≤ ~700 lines)

```
tools/benchmark/src/
├── lib.rs            # crate doc + BenchError
├── model/            # the register-00 workload model
│   ├── event.rs      #   event catalogue → operation sequences
│   ├── ward.rs       #   patients, admission state, staff pool
│   ├── schedule.rs   #   open-loop arrival schedule + diurnal curve
│   └── lock.rs       #   workload.lock hashing
├── gen.rs            # seeded instance-data generator over fixture skeletons
├── drive.rs          # executor: planned-time dispatch, warmup, knee series
├── measure.rs        # per-class HdrHistogram + CO correction
├── sample.rs         # docker stats poller, cold start, storage probe
├── seed.rs           # scale-ladder seeder
├── report/
│   ├── json.rs       #   results.json
│   └── markdown.rs   #   REPORT.md
└── bin/bench.rs      # clap CLI
```

Deps (all pinned in the workspace): `hdrhistogram`, `clap`, `serde`/
`serde_json`, `jiff`, `tokio`, `rand`; `conformance` for
transport/sut/fixtures. `sysinfo` only if the docker-stats subprocess
proves insufficient (prefer no extra sampling machinery).

## 10. CI

- `cargo nextest run -p benchmark` + clippy in the standard CI (model/
  generator/measure unit tests — deterministic, no Docker).
- A `bench-smoke` job (manual `workflow_dispatch` + the containers
  workflow): the `smoke` profile against the composed ehrbase-rs stack;
  asserts the run completes and artefacts parse — never publishes numbers
  from CI runners (benchmarking.md §8).
