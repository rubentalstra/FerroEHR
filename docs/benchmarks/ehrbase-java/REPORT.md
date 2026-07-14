# Benchmark report — EHRbase upstream

> Generated from `results.json` (never hand-typed). Workload **hour** · scale **10k** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | EHRbase upstream (foreign) |
| Base URL | http://localhost:8091/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-14T01:41:20.350827Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | 24bc076ce |
| Workload lock | `9e9aff7ce5a3a06bd800540a343a4fe7df146dbfa7ff0fe5b7b48e8ba6075724` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 25663 | 33919 | 33919 | 33919 | 33919 |
| ehr-read | 2 | 0 | 63135 | 65727 | 65727 | 65727 | 65727 |
| comp-create-small | 213 | 0 | 30287 | 88127 | 105983 | 110015 | 110015 |
| comp-create-large | 4 | 0 | 54527 | 86911 | 86911 | 86911 | 86911 |
| comp-update | 27 | 0 | 39039 | 78399 | 110719 | 110719 | 110719 |
| comp-read-latest | 501 | 0 | 22463 | 60287 | 89279 | 104895 | 104895 |
| comp-read-version | 167 | 0 | 19119 | 58335 | 88255 | 107711 | 107711 |
| contribution-commit | 40 | 0 | 41311 | 66623 | 138239 | 138239 | 138239 |
| aql-patient | 167 | 0 | 32351 | 74559 | 104511 | 295935 | 295935 |
| aql-ward | 14 | 0 | 26207 | 76031 | 103935 | 103935 | 103935 |
| dir-read | 43 | 0 | 22671 | 57887 | 116927 | 116927 | 116927 |
| dir-update | 6 | 0 | 43551 | 103423 | 103423 | 103423 | 103423 |
| history-read | 21 | 0 | 31967 | 59167 | 71167 | 71167 | 71167 |
| status-update | 2 | 0 | 57023 | 57247 | 57247 | 57247 | 57247 |
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
| benchmark-ehrbase-java-1 | 1.7% | 606.1 MiB | 515.3 MiB |
| benchmark-ehrbase-java-db-1 | 0.9% | 196.1 MiB | 184.3 MiB |

- **19.3 req/s per app CPU-core** (0.3 req/s ÷ 0.02 cores).
- **0.5 req/s per GB peak app RSS** (0.3 req/s ÷ 0.636 GB).

## 5. Storage footprint

Database on-disk size **319.3 MiB** over **10000** compositions = **32.7 KiB/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **11621 ms** (11.6 s).

## 7. Limitations

- Templates excluded for this SUT: none (all provisioning uploads accepted).
- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-java --base-url http://localhost:8091/ehrbase/rest/openehr/v1 --profile hour --scale 10k --ward-size 20 --load-factor 1 --seed 2953956094
```
