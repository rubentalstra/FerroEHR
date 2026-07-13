# Benchmark report — ehrbase-rs 3.0.0

> Generated from `results.json` (never hand-typed). Workload **smoke** · scale **10k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | ehrbase-rs 3.0.0 (ours) |
| Base URL | http://localhost:8080/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-13T18:41:26.664814Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | f68ee1e6b |
| Workload lock | `afa48bd156dc31d9e22d9ac8e4a7f9425f717480b04c28d746d28ae9a9fbec26` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 29871 | 38207 | 38207 | 38207 | 38207 |
| ehr-read | 2 | 0 | 61727 | 69247 | 69247 | 69247 | 69247 |
| comp-create-small | 90 | 0 | 26863 | 32623 | 92927 | 92927 | 92927 |
| comp-create-large | 4 | 0 | 43487 | 78527 | 78527 | 78527 | 78527 |
| comp-update | 28 | 0 | 33247 | 75327 | 80063 | 80063 | 80063 |
| comp-read-latest | 66 | 0 | 18991 | 25935 | 27999 | 27999 | 27999 |
| comp-read-version | 22 | 0 | 21183 | 25807 | 27039 | 27039 | 27039 |
| contribution-commit | 22 | 0 | 27199 | 32735 | 39231 | 39231 | 39231 |
| aql-patient | 22 | 0 | 32079 | 39711 | 42495 | 42495 | 42495 |
| aql-ward | 22 | 0 | 28783 | 36543 | 42015 | 42015 | 42015 |
| dir-read | 22 | 0 | 20015 | 23167 | 25887 | 25887 | 25887 |
| dir-update | 5 | 0 | 37791 | 74303 | 74303 | 74303 | 74303 |
| history-read | 22 | 0 | 20751 | 28191 | 46623 | 46623 | 46623 |
| status-update | 2 | 0 | 35327 | 41695 | 41695 | 41695 | 41695 |
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
| ehrbase-rs-ehrbase-1 | 4.7% | 166.4 MiB | 67.1 MiB |
| ehrbase-rs-ehrbase-postgres-1 | 3.1% | 311.2 MiB | 243.7 MiB |

- **58.2 req/s per app CPU-core** (2.8 req/s ÷ 0.05 cores).
- **15.8 req/s per GB peak app RSS** (2.8 req/s ÷ 0.174 GB).

## 5. Storage footprint

Database on-disk size **268.6 MiB** over **10000** compositions = **27.5 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **13705 ms** (13.7 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-rs --base-url http://localhost:8080/ehrbase/rest/openehr/v1 --profile smoke --scale 10k --ward-size 20 --load-factor 1 --seed 2953956094
```
