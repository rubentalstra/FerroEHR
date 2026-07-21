# Benchmark report — EHRbase upstream

> Generated from `results.json` (never hand-typed). Workload **hour** · scale **10k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | EHRbase upstream (foreign) |
| Base URL | http://localhost:8091/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-21T03:03:49.567359Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | 441fd98f3 |
| Workload lock | `431ac3f50ac093fc492ec6a47a47d4084daf51e597e4c59fddb1e77d28edd590` |
| Config `BENCH_DB_POOL` | `50` |
| Config `LOGGING_LEVEL_ROOT` | `WARN` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 28463 | 43775 | 43775 | 43775 | 43775 |
| ehr-read | 2 | 0 | 60863 | 62943 | 62943 | 62943 | 62943 |
| comp-create-small | 213 | 0 | 41023 | 53023 | 88831 | 109439 | 109439 |
| comp-create-large | 4 | 0 | 100991 | 148351 | 148351 | 148351 | 148351 |
| comp-update | 27 | 0 | 47711 | 76351 | 135167 | 135167 | 135167 |
| comp-read-latest | 501 | 0 | 21183 | 37951 | 74943 | 127743 | 127743 |
| comp-read-version | 167 | 0 | 19071 | 36607 | 73599 | 106559 | 106559 |
| contribution-commit | 40 | 0 | 36831 | 46719 | 90303 | 90303 | 90303 |
| aql-patient | 167 | 0 | 27727 | 41375 | 87423 | 382719 | 382719 |
| aql-ward | 14 | 0 | 18271 | 28303 | 41439 | 41439 | 41439 |
| dir-read | 43 | 0 | 18495 | 25999 | 36255 | 36255 | 36255 |
| dir-update | 6 | 0 | 37567 | 88703 | 88703 | 88703 | 88703 |
| history-read | 21 | 0 | 21343 | 26463 | 51327 | 51327 | 51327 |
| status-update | 2 | 0 | 37087 | 75519 | 75519 | 75519 | 75519 |
| opt-upload | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tpl-list | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

![Latency by operation class — p50→p99.9 range on a log scale](charts/latency.svg)

![CPU over the run](charts/cpu.svg)

![Memory (RSS) over the run](charts/rss.svg)

## 3. Throughput

Sustained **0.3 req/s (20 req/min)** over a 3600 s window (1209 measured requests, error rate 0.000%). The knee/saturation series is the multi-run publication step.

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
| benchmark-ehrbase-java-1 | 1.6% | 605.2 MiB | 518.9 MiB |
| benchmark-ehrbase-java-db-1 | 0.8% | 343.8 MiB | 316.1 MiB |

- **21.6 req/s per app CPU-core** (0.3 req/s ÷ 0.02 cores).
- **0.5 req/s per GB peak app RSS** (0.3 req/s ÷ 0.635 GB).

## 5. Storage footprint

Database on-disk size **342.2 MiB** over **10000** compositions = **35.0 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **11867 ms** (11.9 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-java --base-url http://localhost:8091/ehrbase/rest/openehr/v1 --profile hour --scale 10k --ward-size 20 --load-factor 1 --seed 2953956094
```
