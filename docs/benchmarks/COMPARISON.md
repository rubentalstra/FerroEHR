# Benchmark comparison (generated)

> **Measured, not asserted.** Every number below is read from a committed `results.json`; both directions are reported. The workload, client, and host are identical by construction (`docs/design/benchmarking.md` §3).

## Runs

| | Product | Profile | Scale | Ward | Requests | req/s | req/min | Error rate |
|---|---|---|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | ehrbase-rs 3.0.1 | hour | 10k | 20 | 1209 | 0.3 | 20 | 0.000% |
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
| **ehrbase-rs** | 57.4 | 2.6 |
| **ehrbase-java** | 17.2 | 0.5 |

## Maximum sustained throughput (knee)

> The last sustainable step on the load-factor ladder (p99 ≤ 1 s, error ≤ 0.1%), per SUT — the honest capacity signal, not peak req/s. Each SUT's own `KNEE.md` carries the full ladder and the single-run/same-host lower-bound caveat.

| | Knee L | Sustained req/s | Sustained req/min | Clinical events/min | p99 at knee |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 26 | 262.2 | 15733 | 6498 | 195.1 ms |
| **ehrbase-java** | 16 | 160.5 | 9632 | 3981 | 31.6 ms |

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
| aql-patient | 30927 | 37119 | 54303 | 84223 | 64063 | 112191 | 71167 | 310015 | 71167 | 310015 | 0 | 0 | 1.8× |
| aql-ward | 36799 | 28159 | 48735 | 61823 | 76287 | 100991 | 76287 | 100991 | 76287 | 100991 | 0 | 0 | 1.3× |
| comp-create-large | 73983 | 123967 | 162943 | 196607 | 162943 | 196607 | 162943 | 196607 | 162943 | 196607 | 0 | 0 | 1.2× |
| comp-create-small | 31055 | 42015 | 54463 | 84351 | 76351 | 121983 | 125567 | 131967 | 125567 | 131967 | 0 | 0 | 1.6× |
| comp-read-latest | 21007 | 25887 | 40767 | 73343 | 56703 | 102591 | 65055 | 141311 | 65055 | 141311 | 0 | 0 | 1.8× |
| comp-read-version | 19519 | 23295 | 39007 | 75775 | 56927 | 101631 | 62911 | 142463 | 62911 | 142463 | 0 | 0 | 1.8× |
| comp-update | 37439 | 53695 | 66495 | 98175 | 74495 | 134399 | 74495 | 134399 | 74495 | 134399 | 0 | 0 | 1.8× |
| contribution-commit | 32399 | 42911 | 61503 | 90879 | 75647 | 151935 | 75647 | 151935 | 75647 | 151935 | 0 | 0 | 2.0× |
| dir-read | 14831 | 22111 | 32063 | 57247 | 38495 | 87935 | 38495 | 87935 | 38495 | 87935 | 0 | 0 | 2.3× |
| dir-update | 38303 | 34111 | 123903 | 83583 | 123903 | 83583 | 123903 | 83583 | 123903 | 83583 | 0 | 0 | 1.5× |
| ehr-create | 43935 | 43167 | 67391 | 55327 | 67391 | 55327 | 67391 | 55327 | 67391 | 55327 | 0 | 0 | 1.2× |
| ehr-read | 59103 | 64607 | 109183 | 111423 | 109183 | 111423 | 109183 | 111423 | 109183 | 111423 | 0 | 0 | 1.0× |
| history-read | 12679 | 29647 | 33439 | 58847 | 40127 | 76095 | 40127 | 76095 | 40127 | 76095 | 0 | 0 | 1.9× |
| status-update | 44159 | 48351 | 57663 | 95295 | 57663 | 95295 | 57663 | 95295 | 57663 | 95295 | 0 | 0 | 1.7× |

## Resources

| | Idle RSS | Peak RSS | Mean CPU | Cold start | Storage bytes/composition |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 11 MB | 133 MB | 0.6% | 11431 ms | 24906 |
| **ehrbase-java** | 579 MB | 645 MB | 1.9% | 17171 ms | 35880 |

## Where ehrbase-rs wins (p99, computed)

- `aql-patient`: 64063 µs vs 112191 µs
- `aql-ward`: 76287 µs vs 100991 µs
- `comp-create-large`: 162943 µs vs 196607 µs
- `comp-create-small`: 76351 µs vs 121983 µs
- `comp-read-latest`: 56703 µs vs 102591 µs
- `comp-read-version`: 56927 µs vs 101631 µs
- `comp-update`: 74495 µs vs 134399 µs
- `contribution-commit`: 75647 µs vs 151935 µs
- `dir-read`: 38495 µs vs 87935 µs
- `ehr-read`: 109183 µs vs 111423 µs
- `history-read`: 40127 µs vs 76095 µs
- `status-update`: 57663 µs vs 95295 µs

## Where ehrbase-java wins (p99, computed)

- `dir-update`: 83583 µs vs 123903 µs
- `ehr-create`: 55327 µs vs 67391 µs

## Limitations

Single run per SUT (no inter-run variance yet — the ≥5-run protocol is the publication step); same host, sequential execution; see each run's own `REPORT.md` §Limitations for sampler availability.
