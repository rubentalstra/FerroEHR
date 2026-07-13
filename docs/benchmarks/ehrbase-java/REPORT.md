# Benchmark report — EHRbase upstream

> Generated from `results.json` (never hand-typed). Workload **smoke** · scale **10k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | EHRbase upstream (foreign) |
| Base URL | http://localhost:8091/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-13T18:48:58.725836Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | ec5b59182 |
| Workload lock | `afa48bd156dc31d9e22d9ac8e4a7f9425f717480b04c28d746d28ae9a9fbec26` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 22207 | 27999 | 27999 | 27999 | 27999 |
| ehr-read | 2 | 0 | 59519 | 61919 | 61919 | 61919 | 61919 |
| comp-create-small | 0 | 110 | 0 | 0 | 0 | 0 | 0 |
| comp-create-large | 0 | 24 | 0 | 0 | 0 | 0 | 0 |
| comp-update | 0 | 28 | 0 | 0 | 0 | 0 | 0 |
| comp-read-latest | 0 | 66 | 0 | 0 | 0 | 0 | 0 |
| comp-read-version | 0 | 22 | 0 | 0 | 0 | 0 | 0 |
| contribution-commit | 0 | 22 | 0 | 0 | 0 | 0 | 0 |
| aql-patient | 0 | 22 | 0 | 0 | 0 | 0 | 0 |
| aql-ward | 22 | 0 | 19423 | 26367 | 184959 | 184959 | 184959 |
| dir-read | 22 | 0 | 18847 | 24223 | 26399 | 26399 | 26399 |
| dir-update | 5 | 0 | 65439 | 75519 | 75519 | 75519 | 75519 |
| history-read | 0 | 22 | 0 | 0 | 0 | 0 | 0 |
| status-update | 2 | 0 | 49151 | 81215 | 81215 | 81215 | 81215 |
| opt-upload | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tpl-list | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

![Latency by operation class — p50→p99.9 range on a log scale](charts/latency.svg)

![CPU over the run](charts/cpu.svg)

![Memory (RSS) over the run](charts/rss.svg)

## 3. Throughput

Sustained **0.5 req/s** over a 120 s window (55 measured requests, error rate 85.175%). The knee/saturation series (register 01 §3) is the multi-run publication step.

## 4. Resource efficiency

| Container | mean CPU | peak RSS | idle RSS |
|---|--:|--:|--:|
| benchmark-ehrbase-java-1 | 15.0% | 518.7 MiB | 497.6 MiB |
| benchmark-ehrbase-java-db-1 | 0.9% | 190.4 MiB | 184.4 MiB |

- **3.0 req/s per app CPU-core** (0.5 req/s ÷ 0.15 cores).
- **0.8 req/s per GB peak app RSS** (0.5 req/s ÷ 0.544 GB).

## 5. Storage footprint

Database on-disk size **318.2 MiB** over **10000** compositions = **32.6 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **11656 ms** (11.7 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-java --base-url http://localhost:8091/ehrbase/rest/openehr/v1 --profile smoke --scale 10k --ward-size 20 --load-factor 1 --seed 2953956094
```
