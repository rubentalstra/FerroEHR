# Benchmark report — ehrbase-rs 3.0.0

> Generated from `results.json` (never hand-typed). Workload **smoke** · scale **empty** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | ehrbase-rs 3.0.0 (ours) |
| Base URL | http://localhost:8080/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-13T17:36:42.473396Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | cffa0e152 |
| Workload lock | `afa48bd156dc31d9e22d9ac8e4a7f9425f717480b04c28d746d28ae9a9fbec26` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 17839 | 34527 | 34527 | 34527 | 34527 |
| ehr-read | 2 | 0 | 62271 | 65055 | 65055 | 65055 | 65055 |
| comp-create-small | 90 | 0 | 20847 | 29295 | 97919 | 97919 | 97919 |
| comp-create-large | 4 | 0 | 44223 | 78463 | 78463 | 78463 | 78463 |
| comp-update | 0 | 28 | 0 | 0 | 0 | 0 | 0 |
| comp-read-latest | 0 | 66 | 0 | 0 | 0 | 0 | 0 |
| comp-read-version | 0 | 22 | 0 | 0 | 0 | 0 | 0 |
| contribution-commit | 22 | 0 | 20575 | 26591 | 33215 | 33215 | 33215 |
| aql-patient | 22 | 0 | 21231 | 31279 | 35935 | 35935 | 35935 |
| aql-ward | 22 | 0 | 15503 | 17471 | 19359 | 19359 | 19359 |
| dir-read | 22 | 0 | 14599 | 20079 | 24527 | 24527 | 24527 |
| dir-update | 0 | 25 | 0 | 0 | 0 | 0 | 0 |
| history-read | 0 | 22 | 0 | 0 | 0 | 0 | 0 |
| status-update | 2 | 0 | 29631 | 43359 | 43359 | 43359 | 43359 |
| opt-upload | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tpl-list | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## 3. Throughput

Sustained **1.6 req/s** over a 120 s window (188 measured requests, error rate 46.439%). The knee/saturation series (register 01 §3) is the multi-run publication step.

## 4. Resource efficiency

| Container | mean CPU | peak RSS | idle RSS |
|---|--:|--:|--:|
| ehrbase-rs-ehrbase-1 | 3.6% | 121.0 MiB | 4.0 MiB |
| ehrbase-rs-ehrbase-postgres-1 | 1.7% | 94.3 MiB | 36.1 MiB |

- **43.8 req/s per app CPU-core** (1.6 req/s ÷ 0.04 cores).
- **12.3 req/s per GB peak app RSS** (1.6 req/s ÷ 0.127 GB).

## 5. Storage footprint

Database on-disk size **18.5 MiB** over **0** compositions = **0 B/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

Compose-up → first successful HTTP answer: **11592 ms** (11.6 s).

## 7. Limitations

- Storage measured on an empty (unseeded) database — bytes/composition is not meaningful at this scale.
- Templates excluded for this SUT: none (all provisioning uploads accepted).
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-rs --base-url http://localhost:8080/ehrbase/rest/openehr/v1 --profile smoke --scale empty --ward-size 20 --load-factor 1 --seed 2953956094
```
