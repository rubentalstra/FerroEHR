# Benchmark comparison (generated)

> **Measured, not asserted.** Every number below is read from a committed `results.json`; both directions are reported. The workload, client, and host are identical by construction (`docs/design/benchmarking.md` §3).

## Runs

| | Product | Profile | Scale | Ward | Requests | req/s | req/min | Error rate |
|---|---|---|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | ehrbase-rs 3.0.3 | hour | 10k | 20 | 1209 | 0.3 | 20 | 0.000% |
| **ehrbase-java** | EHRbase upstream | hour | 10k | 20 | 1209 | 0.3 | 20 | 0.000% |

## Throughput

![Sustained throughput](charts/comparison-throughput.svg)

## Resources — app container

![App peak memory](charts/comparison-memory.svg)

![App mean CPU](charts/comparison-cpu.svg)

![App CPU over the run](charts/comparison-cpu-series.svg)

![App memory over the run](charts/comparison-rss-series.svg)

## Cold start

![Cold start](charts/comparison-coldstart.svg)

## Efficiency (computed)

| | req/s per CPU-core | req/s per GB peak RSS |
|---|--:|--:|
| **ehrbase-rs** | 47.1 | 2.6 |
| **ehrbase-java** | 11.2 | 0.6 |

## Maximum sustained throughput (knee)

> The last sustainable step on the load-factor ladder (p99 ≤ 1 s, error ≤ 0.1%), per SUT — the honest capacity signal, not peak req/s. Each SUT's own `KNEE.md` carries the full ladder and the single-run/same-host lower-bound caveat.

| | Knee L | Sustained req/s | Sustained req/min | Clinical events/min | p99 at knee |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 64 | 631.6 | 37894 | 15647 | 92.2 ms |
| **ehrbase-java** | 32 | 316.1 | 18968 | 7838 | 200.4 ms |

![Max sustained req/s at the SLO](charts/comparison-knee.svg)

## Clinical transactions (events)

> The TPC-style business-transaction metric: a clinical event (admission, medication round, lab batch, discharge…) counts **completed** only when every one of its requests succeeded. Events/min beside the per-request req/s — both directions, same workload by construction.

| Event | ehrbase-rs attempted | ehrbase-rs completed | ehrbase-rs events/min | ehrbase-java attempted | ehrbase-java completed | ehrbase-java events/min |
|---|--:|--:|--:|--:|--:|--:|
| E1 admission | 2 | 2 | 0.0 | 2 | 2 | 0.0 |
| E2 shift-vitals | 127 | 127 | 2.1 | 127 | 127 | 2.1 |
| E3 medication-round | 84 | 84 | 1.4 | 84 | 84 | 1.4 |
| E4 lab-results | 40 | 40 | 0.7 | 40 | 40 | 0.7 |
| E5 chart-review | 167 | 167 | 2.8 | 167 | 167 | 2.8 |
| E6 care-plan | 43 | 43 | 0.7 | 43 | 43 | 0.7 |
| E7 doc-correction | 21 | 21 | 0.4 | 21 | 21 | 0.4 |
| E8 ward-dashboard | 14 | 14 | 0.2 | 14 | 14 | 0.2 |
| E9 discharge | 2 | 2 | 0.0 | 2 | 2 | 0.0 |
| **total** | **500** | **500** | **8.3** | **500** | **500** | **8.3** |

**Higher total clinical-event throughput: ehrbase-rs** — 8.3 vs 8.3 events/min (1.0×).

## Latency — p99 per operation class

![p99 latency per operation class](charts/comparison-p99.svg)

## Latency — p50 per operation class

![p50 latency per operation class](charts/comparison-p50.svg)

## Per-class detail (µs)

| Class | ehrbase-rs p50 | ehrbase-java p50 | ehrbase-rs p90 | ehrbase-java p90 | ehrbase-rs p99 | ehrbase-java p99 | ehrbase-rs p99.9 | ehrbase-java p99.9 | ehrbase-rs max | ehrbase-java max | ehrbase-rs err | ehrbase-java err | p99 gap |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|---|
| aql-patient | 28463 | 51551 | 49279 | 103743 | 65247 | 136575 | 112639 | 409343 | 112639 | 409343 | 0 | 0 | 2.1× |
| aql-ward | 19567 | 30783 | 38975 | 115327 | 39391 | 126591 | 39391 | 126591 | 39391 | 126591 | 0 | 0 | 3.2× |
| comp-create-large | 79615 | 112639 | 127999 | 249087 | 127999 | 249087 | 127999 | 249087 | 127999 | 249087 | 0 | 0 | 1.9× |
| comp-create-small | 39935 | 54431 | 67135 | 126591 | 93887 | 162175 | 99647 | 196991 | 99647 | 196991 | 0 | 0 | 1.7× |
| comp-read-latest | 22015 | 34783 | 49279 | 92607 | 74623 | 132479 | 147583 | 141951 | 147583 | 141951 | 0 | 0 | 1.8× |
| comp-read-version | 20495 | 32111 | 47903 | 91711 | 61503 | 131583 | 145151 | 146303 | 145151 | 146303 | 0 | 0 | 2.1× |
| comp-update | 41247 | 56703 | 73343 | 108479 | 81471 | 205439 | 81471 | 205439 | 81471 | 205439 | 0 | 0 | 2.5× |
| contribution-commit | 33279 | 48031 | 53759 | 108543 | 75327 | 186879 | 75327 | 186879 | 75327 | 186879 | 0 | 0 | 2.5× |
| dir-read | 18127 | 25583 | 35615 | 82559 | 49023 | 112895 | 49023 | 112895 | 49023 | 112895 | 0 | 0 | 2.3× |
| dir-update | 31039 | 47135 | 90559 | 166655 | 90559 | 166655 | 90559 | 166655 | 90559 | 166655 | 0 | 0 | 1.8× |
| ehr-create | 36127 | 72511 | 38591 | 89407 | 38591 | 89407 | 38591 | 89407 | 38591 | 89407 | 0 | 0 | 2.3× |
| ehr-read | 82431 | 116479 | 91135 | 147327 | 91135 | 147327 | 91135 | 147327 | 91135 | 147327 | 0 | 0 | 1.6× |
| history-read | 13463 | 32351 | 29119 | 65023 | 33439 | 86463 | 33439 | 86463 | 33439 | 86463 | 0 | 0 | 2.6× |
| status-update | 29903 | 47999 | 48799 | 86079 | 48799 | 86079 | 48799 | 86079 | 48799 | 86079 | 0 | 0 | 1.8× |

## Resources

| | Idle RSS | Peak RSS | Mean CPU | Cold start | Storage bytes/composition |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 11 MB | 132 MB | 0.7% | 11563 ms | 24911 |
| **ehrbase-java** | 547 MB | 624 MB | 3.0% | 17184 ms | 35894 |

## Where ehrbase-rs wins (p99, computed)

- `aql-patient`: 65247 µs vs 136575 µs
- `aql-ward`: 39391 µs vs 126591 µs
- `comp-create-large`: 127999 µs vs 249087 µs
- `comp-create-small`: 93887 µs vs 162175 µs
- `comp-read-latest`: 74623 µs vs 132479 µs
- `comp-read-version`: 61503 µs vs 131583 µs
- `comp-update`: 81471 µs vs 205439 µs
- `contribution-commit`: 75327 µs vs 186879 µs
- `dir-read`: 49023 µs vs 112895 µs
- `dir-update`: 90559 µs vs 166655 µs
- `ehr-create`: 38591 µs vs 89407 µs
- `ehr-read`: 91135 µs vs 147327 µs
- `history-read`: 33439 µs vs 86463 µs
- `status-update`: 48799 µs vs 86079 µs

## Where ehrbase-java wins (p99, computed)

No class won on p99 in this run pair.

## Limitations

Single run per SUT (no inter-run variance yet — the ≥5-run protocol is the publication step); same host, sequential execution; see each run's own `REPORT.md` §Limitations for sampler availability.
