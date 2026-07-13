# Benchmark report — EHRbase upstream

> Generated from `results.json` (never hand-typed). Workload **smoke** · scale **10k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | EHRbase upstream (foreign) |
| Base URL | http://localhost:8091/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-13T19:45:02.805811Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | 327554d09 |
| Workload lock | `afa48bd156dc31d9e22d9ac8e4a7f9425f717480b04c28d746d28ae9a9fbec26` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 10423 | 70271 | 70271 | 70271 | 70271 |
| ehr-read | 2 | 0 | 66175 | 122111 | 122111 | 122111 | 122111 |
| comp-create-small | 90 | 0 | 26735 | 57375 | 182783 | 182783 | 182783 |
| comp-create-large | 4 | 0 | 33343 | 136191 | 136191 | 136191 | 136191 |
| comp-update | 28 | 0 | 35647 | 99775 | 152703 | 152703 | 152703 |
| comp-read-latest | 66 | 0 | 25647 | 46559 | 103295 | 103295 | 103295 |
| comp-read-version | 22 | 0 | 22847 | 38815 | 101695 | 101695 | 101695 |
| contribution-commit | 0 | 22 | 0 | 0 | 0 | 0 | 0 |
| aql-patient | 22 | 0 | 30783 | 60799 | 161535 | 161535 | 161535 |
| aql-ward | 22 | 0 | 23199 | 36447 | 192639 | 192639 | 192639 |
| dir-read | 22 | 0 | 19743 | 47967 | 204671 | 204671 | 204671 |
| dir-update | 5 | 0 | 75327 | 124351 | 124351 | 124351 | 124351 |
| history-read | 22 | 0 | 21663 | 54975 | 91199 | 91199 | 91199 |
| status-update | 2 | 0 | 35647 | 65791 | 65791 | 65791 | 65791 |
| opt-upload | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tpl-list | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

![Latency by operation class — p50→p99.9 range on a log scale](charts/latency.svg)

![CPU over the run](charts/cpu.svg)

![Memory (RSS) over the run](charts/rss.svg)

## 3. Throughput

Sustained **2.6 req/s** over a 120 s window (309 measured requests, error rate 6.647%). The knee/saturation series (register 01 §3) is the multi-run publication step.

## 4. Resource efficiency

| Container | mean CPU | peak RSS | idle RSS |
|---|--:|--:|--:|
| benchmark-ehrbase-java-1 | 24.3% | 622.5 MiB | 590.7 MiB |
| benchmark-ehrbase-java-db-1 | 4.2% | 234.0 MiB | 217.6 MiB |

- **10.6 req/s per app CPU-core** (2.6 req/s ÷ 0.24 cores).
- **3.9 req/s per GB peak app RSS** (2.6 req/s ÷ 0.653 GB).

## 5. Storage footprint

Database on-disk size **319.5 MiB** over **10000** compositions = **32.7 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **16709 ms** (16.7 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-java --base-url http://localhost:8091/ehrbase/rest/openehr/v1 --profile smoke --scale 10k --ward-size 20 --load-factor 1 --seed 2953956094
```
