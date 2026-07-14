# Benchmark report — EHRbase upstream

> Generated from `results.json` (never hand-typed). Workload **hour** · scale **100k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | EHRbase upstream (foreign) |
| Base URL | http://localhost:8091/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-14T04:40:14.112001Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | eda6f9ee1 |
| Workload lock | `9e9aff7ce5a3a06bd800540a343a4fe7df146dbfa7ff0fe5b7b48e8ba6075724` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 78655 | 93375 | 93375 | 93375 | 93375 |
| ehr-read | 2 | 0 | 113407 | 116223 | 116223 | 116223 | 116223 |
| comp-create-small | 213 | 0 | 33087 | 92223 | 117631 | 129983 | 129983 |
| comp-create-large | 4 | 0 | 63679 | 145535 | 145535 | 145535 | 145535 |
| comp-update | 27 | 0 | 45983 | 91391 | 140543 | 140543 | 140543 |
| comp-read-latest | 501 | 0 | 23487 | 73535 | 93567 | 99455 | 99455 |
| comp-read-version | 167 | 0 | 21247 | 70655 | 91967 | 96831 | 96831 |
| contribution-commit | 40 | 0 | 40383 | 90687 | 122815 | 122815 | 122815 |
| aql-patient | 167 | 0 | 39711 | 86335 | 126271 | 298751 | 298751 |
| aql-ward | 14 | 0 | 30031 | 66175 | 82879 | 82879 | 82879 |
| dir-read | 43 | 0 | 22895 | 73087 | 80639 | 80639 | 80639 |
| dir-update | 6 | 0 | 54431 | 142591 | 142591 | 142591 | 142591 |
| history-read | 21 | 0 | 34047 | 60959 | 85951 | 85951 | 85951 |
| status-update | 2 | 0 | 68031 | 85567 | 85567 | 85567 | 85567 |
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
| benchmark-ehrbase-java-1 | 2.1% | 677.5 MiB | 639.5 MiB |
| benchmark-ehrbase-java-db-1 | 1.1% | 325.3 MiB | 299.2 MiB |

- **16.3 req/s per app CPU-core** (0.3 req/s ÷ 0.02 cores).
- **0.5 req/s per GB peak app RSS** (0.3 req/s ÷ 0.710 GB).

## 5. Storage footprint

Database on-disk size **3.0 GiB** over **100000** compositions = **31.1 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **17678 ms** (17.7 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-java --base-url http://localhost:8091/ehrbase/rest/openehr/v1 --profile hour --scale 100k --ward-size 20 --load-factor 1 --seed 2953956094
```
