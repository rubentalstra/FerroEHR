# Benchmark report — ehrbase-rs 3.0.0

> Generated from `results.json` (never hand-typed). Workload **smoke** · scale **empty** · ward **20** · load factor **1** · seed `2953956094`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.

## 1. Environment

> **Load generator:** Apple M2 (8 logical CPUs, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| SUT | ehrbase-rs 3.0.0 (ours) |
| Base URL | http://localhost:8080/ehrbase/rest/openehr/v1 |
| Run start | 2026-07-13T17:53:26.283104Z |
| Load-gen host | 8 logical CPUs, 16384 MiB RAM |
| Harness rev | fdc0c539e |
| Workload lock | `afa48bd156dc31d9e22d9ac8e4a7f9425f717480b04c28d746d28ae9a9fbec26` |

> A report with a different load-generator line is not directly comparable.

## 2. Latency — per operation class

p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.

| Class | count | errors | p50 | p90 | p99 | p99.9 | max |
|---|--:|--:|--:|--:|--:|--:|--:|
| ehr-create | 2 | 0 | 19471 | 20703 | 20703 | 20703 | 20703 |
| ehr-read | 2 | 0 | 60511 | 66047 | 66047 | 66047 | 66047 |
| comp-create-small | 90 | 0 | 19519 | 21759 | 199807 | 199807 | 199807 |
| comp-create-large | 4 | 0 | 34335 | 71871 | 71871 | 71871 | 71871 |
| comp-update | 28 | 0 | 21631 | 69247 | 73151 | 73151 | 73151 |
| comp-read-latest | 66 | 0 | 16351 | 18351 | 20319 | 20319 | 20319 |
| comp-read-version | 22 | 0 | 14831 | 16799 | 17327 | 17327 | 17327 |
| contribution-commit | 22 | 0 | 19839 | 22287 | 23391 | 23391 | 23391 |
| aql-patient | 22 | 0 | 20335 | 22063 | 25599 | 25599 | 25599 |
| aql-ward | 22 | 0 | 14679 | 16495 | 18063 | 18063 | 18063 |
| dir-read | 22 | 0 | 15463 | 18719 | 22751 | 22751 | 22751 |
| dir-update | 5 | 0 | 20623 | 68287 | 68287 | 68287 | 68287 |
| history-read | 22 | 0 | 14855 | 17983 | 24127 | 24127 | 24127 |
| status-update | 2 | 0 | 26719 | 32687 | 32687 | 32687 | 32687 |
| opt-upload | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tpl-list | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## 3. Throughput

Sustained **2.8 req/s** over a 120 s window (331 measured requests, error rate 0.000%). The knee/saturation series (register 01 §3) is the multi-run publication step.

## 4. Resource efficiency

| Container | mean CPU | peak RSS | idle RSS |
|---|--:|--:|--:|
| ehrbase-rs-ehrbase-1 | 4.2% | 124.2 MiB | 9.4 MiB |
| ehrbase-rs-ehrbase-postgres-1 | 1.7% | 111.3 MiB | 42.8 MiB |

- **65.4 req/s per app CPU-core** (2.8 req/s ÷ 0.04 cores).
- **21.2 req/s per GB peak app RSS** (2.8 req/s ÷ 0.130 GB).

## 5. Storage footprint

Database on-disk size **19.0 MiB** over **0** compositions = **0 B/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).

## 6. Cold start

_Cold start not measured for this run (BYO SUT or compose unmanaged)._

## 7. Limitations

- Cold start not measured — compose lifecycle was not managed by the harness.
- Storage measured on an empty (unseeded) database — bytes/composition is not meaningful at this scale.
- Templates excluded for this SUT: none (all provisioning uploads accepted).
- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.

## 8. Reproduce it

```bash
cargo run -q -p benchmark --bin bench -- run --sut ehrbase-rs --base-url http://localhost:8080/ehrbase/rest/openehr/v1 --profile smoke --scale empty --ward-size 20 --load-factor 1 --seed 2953956094
```
