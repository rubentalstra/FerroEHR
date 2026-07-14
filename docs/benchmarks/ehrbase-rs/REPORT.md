# Benchmark report — ehrbase-rs 3.0.0

> Generated from `results.json` (never hand-typed). Workload **hour** · scale **10k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | ehrbase-rs 3.0.0 (ours) |
| Base URL | http://localhost:8080/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-14T00:39:23.458873Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | 24bc076ce |
| Workload lock | `9e9aff7ce5a3a06bd800540a343a4fe7df146dbfa7ff0fe5b7b48e8ba6075724` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 58367 | 61375 | 61375 | 61375 | 61375 |
| ehr-read | 2 | 0 | 127231 | 129983 | 129983 | 129983 | 129983 |
| comp-create-small | 213 | 0 | 38303 | 69247 | 95615 | 135167 | 135167 |
| comp-create-large | 4 | 0 | 81279 | 142591 | 142591 | 142591 | 142591 |
| comp-update | 27 | 0 | 44831 | 72383 | 79359 | 79359 | 79359 |
| comp-read-latest | 501 | 0 | 25439 | 42591 | 59167 | 64063 | 64063 |
| comp-read-version | 167 | 0 | 26207 | 40991 | 58079 | 59295 | 59295 |
| contribution-commit | 40 | 0 | 37535 | 75903 | 90175 | 90175 | 90175 |
| aql-patient | 167 | 0 | 39103 | 57503 | 66751 | 69951 | 69951 |
| aql-ward | 14 | 0 | 50303 | 70143 | 78399 | 78399 | 78399 |
| dir-read | 43 | 0 | 26927 | 46495 | 65727 | 65727 | 65727 |
| dir-update | 6 | 0 | 37151 | 131327 | 131327 | 131327 | 131327 |
| history-read | 21 | 0 | 22143 | 44927 | 48703 | 48703 | 48703 |
| status-update | 2 | 0 | 54975 | 80255 | 80255 | 80255 | 80255 |
| opt-upload | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tpl-list | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

![Latency by operation class — p50→p99.9 range on a log scale](charts/latency.svg)

![CPU over the run](charts/cpu.svg)

![Memory (RSS) over the run](charts/rss.svg)

## 3. Throughput

Sustained **0.3 req/s** over a 3600 s window (1209 measured requests, error rate 0.000%). The knee/saturation series (register 01 §3) is the multi-run publication step.

## 4. Resource efficiency

| Container | mean CPU | peak RSS | idle RSS |
|---|--:|--:|--:|
| ehrbase-rs-ehrbase-1 | 0.9% | 187.8 MiB | 47.1 MiB |
| ehrbase-rs-ehrbase-postgres-1 | 1.4% | 287.7 MiB | 198.4 MiB |

- **36.9 req/s per app CPU-core** (0.3 req/s ÷ 0.01 cores).
- **1.7 req/s per GB peak app RSS** (0.3 req/s ÷ 0.197 GB).

## 5. Storage footprint

Database on-disk size **269.2 MiB** over **10000** compositions = **27.6 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **11403 ms** (11.4 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-rs --base-url http://localhost:8080/ehrbase/rest/openehr/v1 --profile hour --scale 10k --ward-size 20 --load-factor 1 --seed 2953956094
```
