# Benchmark report — ehrbase-rs 3.0.0

> Generated from `results.json` (never hand-typed). Workload **smoke** · scale **10k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | ehrbase-rs 3.0.0 (ours) |
| Base URL | http://localhost:8080/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-13T19:10:08.513531Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | 17d3a3530 |
| Workload lock | `afa48bd156dc31d9e22d9ac8e4a7f9425f717480b04c28d746d28ae9a9fbec26` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 21519 | 39647 | 39647 | 39647 | 39647 |
| ehr-read | 2 | 0 | 66623 | 75135 | 75135 | 75135 | 75135 |
| comp-create-small | 90 | 0 | 28575 | 45631 | 79999 | 79999 | 79999 |
| comp-create-large | 4 | 0 | 43231 | 87999 | 87999 | 87999 | 87999 |
| comp-update | 28 | 0 | 38943 | 76735 | 89535 | 89535 | 89535 |
| comp-read-latest | 66 | 0 | 24463 | 44799 | 52447 | 52447 | 52447 |
| comp-read-version | 22 | 0 | 25999 | 42015 | 43999 | 43999 | 43999 |
| contribution-commit | 22 | 0 | 35615 | 54527 | 97343 | 97343 | 97343 |
| aql-patient | 22 | 0 | 37183 | 55583 | 73727 | 73727 | 73727 |
| aql-ward | 22 | 0 | 37087 | 50655 | 56927 | 56927 | 56927 |
| dir-read | 22 | 0 | 16895 | 41695 | 64287 | 64287 | 64287 |
| dir-update | 5 | 0 | 69951 | 78911 | 78911 | 78911 | 78911 |
| history-read | 22 | 0 | 19679 | 28527 | 51583 | 51583 | 51583 |
| status-update | 2 | 0 | 29199 | 41215 | 41215 | 41215 | 41215 |
| opt-upload | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tpl-list | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

![Latency by operation class — p50→p99.9 range on a log scale](charts/latency.svg)

![CPU over the run](charts/cpu.svg)

![Memory (RSS) over the run](charts/rss.svg)

## 3. Throughput

Sustained **2.8 req/s** over a 120 s window (331 measured requests, error rate 0.000%). The knee/saturation series (register 01 §3) is the multi-run publication step.

## 4. Resource efficiency

| Container | mean CPU | peak RSS | idle RSS |
|---|--:|--:|--:|
| ehrbase-rs-ehrbase-1 | 5.4% | 190.9 MiB | 144.2 MiB |
| ehrbase-rs-ehrbase-postgres-1 | 6.8% | 275.3 MiB | 202.9 MiB |

- **51.5 req/s per app CPU-core** (2.8 req/s ÷ 0.05 cores).
- **13.8 req/s per GB peak app RSS** (2.8 req/s ÷ 0.200 GB).

## 5. Storage footprint

Database on-disk size **269.3 MiB** over **10000** compositions = **27.6 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **11491 ms** (11.5 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-rs --base-url http://localhost:8080/ehrbase/rest/openehr/v1 --profile smoke --scale 10k --ward-size 20 --load-factor 1 --seed 2953956094
```
