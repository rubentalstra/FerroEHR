# Benchmark report — ehrbase-rs 3.0.0

> Generated from `results.json` (never hand-typed). Workload **hour** · scale **100k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | ehrbase-rs 3.0.0 (ours) |
| Base URL | http://localhost:8080/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-14T03:26:59.182228Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | eda6f9ee1 |
| Workload lock | `9e9aff7ce5a3a06bd800540a343a4fe7df146dbfa7ff0fe5b7b48e8ba6075724` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 60447 | 80895 | 80895 | 80895 | 80895 |
| ehr-read | 2 | 0 | 131455 | 141695 | 141695 | 141695 | 141695 |
| comp-create-small | 213 | 0 | 41311 | 66559 | 86911 | 142463 | 142463 |
| comp-create-large | 4 | 0 | 78399 | 156671 | 156671 | 156671 | 156671 |
| comp-update | 27 | 0 | 44735 | 64479 | 77631 | 77631 | 77631 |
| comp-read-latest | 501 | 0 | 30207 | 47167 | 75903 | 163327 | 163327 |
| comp-read-version | 167 | 0 | 28543 | 46079 | 75967 | 160895 | 160895 |
| contribution-commit | 40 | 0 | 41727 | 68095 | 82751 | 82751 | 82751 |
| aql-patient | 167 | 0 | 96575 | 121151 | 183935 | 536575 | 536575 |
| aql-ward | 14 | 0 | 86911 | 146943 | 154367 | 154367 | 154367 |
| dir-read | 43 | 0 | 26623 | 44703 | 52031 | 52031 | 52031 |
| dir-update | 6 | 0 | 36159 | 147583 | 147583 | 147583 | 147583 |
| history-read | 21 | 0 | 26319 | 38207 | 46143 | 46143 | 46143 |
| status-update | 2 | 0 | 47679 | 76671 | 76671 | 76671 | 76671 |
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
| ehrbase-rs-ehrbase-1 | 0.9% | 254.6 MiB | 106.3 MiB |
| ehrbase-rs-ehrbase-postgres-1 | 2.0% | 410.3 MiB | 322.3 MiB |

- **35.7 req/s per app CPU-core** (0.3 req/s ÷ 0.01 cores).
- **1.3 req/s per GB peak app RSS** (0.3 req/s ÷ 0.267 GB).

## 5. Storage footprint

Database on-disk size **2.3 GiB** over **100000** compositions = **24.6 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **109984 ms** (110.0 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-rs --base-url http://localhost:8080/ehrbase/rest/openehr/v1 --profile hour --scale 100k --ward-size 20 --load-factor 1 --seed 2953956094
```
