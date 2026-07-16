# Benchmark report — ehrbase-rs 3.0.3

> Generated from `results.json` (never hand-typed). Workload **hour** · scale **10k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | ehrbase-rs 3.0.3 (ours) |
| Base URL | http://localhost:8080/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-16T01:18:57.73996Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | 117fad2e1 |
| Workload lock | `431ac3f50ac093fc492ec6a47a47d4084daf51e597e4c59fddb1e77d28edd590` |
| Config `BENCH_DB_POOL` | `50` |
| Config `LOGGING_LEVEL_ROOT` | `WARN` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 36127 | 38591 | 38591 | 38591 | 38591 |
| ehr-read | 2 | 0 | 82431 | 91135 | 91135 | 91135 | 91135 |
| comp-create-small | 213 | 0 | 39935 | 67135 | 93887 | 99647 | 99647 |
| comp-create-large | 4 | 0 | 79615 | 127999 | 127999 | 127999 | 127999 |
| comp-update | 27 | 0 | 41247 | 73343 | 81471 | 81471 | 81471 |
| comp-read-latest | 501 | 0 | 22015 | 49279 | 74623 | 147583 | 147583 |
| comp-read-version | 167 | 0 | 20495 | 47903 | 61503 | 145151 | 145151 |
| contribution-commit | 40 | 0 | 33279 | 53759 | 75327 | 75327 | 75327 |
| aql-patient | 167 | 0 | 28463 | 49279 | 65247 | 112639 | 112639 |
| aql-ward | 14 | 0 | 19567 | 38975 | 39391 | 39391 | 39391 |
| dir-read | 43 | 0 | 18127 | 35615 | 49023 | 49023 | 49023 |
| dir-update | 6 | 0 | 31039 | 90559 | 90559 | 90559 | 90559 |
| history-read | 21 | 0 | 13463 | 29119 | 33439 | 33439 | 33439 |
| status-update | 2 | 0 | 29903 | 48799 | 48799 | 48799 | 48799 |
| opt-upload | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tpl-list | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

![Latency by operation class — p50→p99.9 range on a log scale](charts/latency.svg)

![CPU over the run](charts/cpu.svg)

![Memory (RSS) over the run](charts/rss.svg)

## 3. Throughput

Sustained **0.3 req/s (20 req/min)** over a 3600 s window (1209 measured requests, error rate 0.000%). The knee/saturation series (register 01 §3) is the multi-run publication step.

### Clinical transactions (events)

A clinical event (admission, medication round, lab batch, discharge…) is a multi-request business transaction, counted **completed** only when every one of its steps succeeded within the measured window (warmup applied per event by its last step — symmetric with the per-request warmup discard). The TPC-style events/min analogue of the req/s above.

| Event | attempted | completed | events/min |
|---|--:|--:|--:|
| E1 admission | 2 | 2 | 0.0 |
| E2 shift-vitals | 127 | 127 | 2.1 |
| E3 medication-round | 84 | 84 | 1.4 |
| E4 lab-results | 40 | 40 | 0.7 |
| E5 chart-review | 167 | 167 | 2.8 |
| E6 care-plan | 43 | 43 | 0.7 |
| E7 doc-correction | 21 | 21 | 0.4 |
| E8 ward-dashboard | 14 | 14 | 0.2 |
| E9 discharge | 2 | 2 | 0.0 |
| **total** | **500** | **500** | **8.3** |

## 4. Resource efficiency

| Container | mean CPU | peak RSS | idle RSS |
|---|--:|--:|--:|
| ehrbase-rs-ehrbase-1 | 0.7% | 131.7 MiB | 10.8 MiB |
| ehrbase-rs-ehrbase-postgres-1 | 1.6% | 354.1 MiB | 283.1 MiB |

- **47.1 req/s per app CPU-core** (0.3 req/s ÷ 0.01 cores).
- **2.4 req/s per GB peak app RSS** (0.3 req/s ÷ 0.138 GB).

## 5. Storage footprint

Database on-disk size **237.6 MiB** over **10000** compositions = **24.3 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **11563 ms** (11.6 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-rs --base-url http://localhost:8080/ehrbase/rest/openehr/v1 --profile hour --scale 10k --ward-size 20 --load-factor 1 --seed 2953956094
```
