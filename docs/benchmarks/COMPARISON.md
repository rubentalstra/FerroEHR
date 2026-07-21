# Benchmark comparison (generated)

> **Measured, not asserted.** Every number below is read from a committed `results.json`; both directions are reported. The workload, client, and host are identical by construction.

## Runs

| | Product | Profile | Scale | Ward | Requests | req/s | req/min | Error rate |
|---|---|---|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | ehrbase-rs 3.5.0 | hour | 10k | 20 | 1209 | 0.3 | 20 | 0.000% |
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
| **ehrbase-rs** | 78.4 | 2.6 |
| **ehrbase-java** | 21.6 | 0.6 |

## Maximum sustained throughput (knee)

> The last sustainable step on the load-factor ladder (p99 ≤ 1 s, error ≤ 0.1%), per SUT — the honest capacity signal, not peak req/s. Each SUT's own `KNEE.md` carries the full ladder and the single-run/same-host lower-bound caveat.

| | Knee L | Sustained req/s | Sustained req/min | Clinical events/min | p99 at knee |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 64 | 631.5 | 37890 | 15642 | 204.7 ms |
| **ehrbase-java** | 48 | 475.0 | 28500 | 11755 | 575.5 ms |

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
| aql-patient | 15679 | 27727 | 24223 | 41375 | 35807 | 87423 | 39007 | 382719 | 39007 | 382719 | 0 | 0 | 2.4× |
| aql-ward | 11719 | 18271 | 14127 | 28303 | 15359 | 41439 | 15359 | 41439 | 15359 | 41439 | 0 | 0 | 2.7× |
| comp-create-large | 60511 | 100991 | 110399 | 148351 | 110399 | 148351 | 110399 | 148351 | 110399 | 148351 | 0 | 0 | 1.3× |
| comp-create-small | 26911 | 41023 | 35647 | 53023 | 66303 | 88831 | 73727 | 109439 | 73727 | 109439 | 0 | 0 | 1.3× |
| comp-read-latest | 13527 | 21183 | 24015 | 37951 | 35775 | 74943 | 43455 | 127743 | 43455 | 127743 | 0 | 0 | 2.1× |
| comp-read-version | 11511 | 19071 | 22543 | 36607 | 33983 | 73599 | 35231 | 106559 | 35231 | 106559 | 0 | 0 | 2.2× |
| comp-update | 30767 | 47711 | 40959 | 76351 | 75199 | 135167 | 75199 | 135167 | 75199 | 135167 | 0 | 0 | 1.8× |
| contribution-commit | 23887 | 36831 | 31231 | 46719 | 45823 | 90303 | 45823 | 90303 | 45823 | 90303 | 0 | 0 | 2.0× |
| dir-read | 11807 | 18495 | 15239 | 25999 | 25039 | 36255 | 25039 | 36255 | 25039 | 36255 | 0 | 0 | 1.4× |
| dir-update | 20015 | 37567 | 67199 | 88703 | 67199 | 88703 | 67199 | 88703 | 67199 | 88703 | 0 | 0 | 1.3× |
| ehr-create | 19711 | 28463 | 20527 | 43775 | 20527 | 43775 | 20527 | 43775 | 20527 | 43775 | 0 | 0 | 2.1× |
| ehr-read | 60895 | 60863 | 64703 | 62943 | 64703 | 62943 | 64703 | 62943 | 64703 | 62943 | 0 | 0 | 1.0× |
| history-read | 10423 | 21343 | 14927 | 26463 | 16295 | 51327 | 16295 | 51327 | 16295 | 51327 | 0 | 0 | 3.1× |
| status-update | 28175 | 37087 | 30447 | 75519 | 30447 | 75519 | 30447 | 75519 | 30447 | 75519 | 0 | 0 | 2.5× |

## Resources

| | Idle RSS | Peak RSS | Mean CPU | Cold start | Storage bytes/composition |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 16 MB | 134 MB | 0.4% | 11524 ms | 27662 |
| **ehrbase-java** | 519 MB | 605 MB | 1.6% | 11867 ms | 35884 |

## Where ehrbase-rs wins (p99, computed)

- `aql-patient`: 35807 µs vs 87423 µs
- `aql-ward`: 15359 µs vs 41439 µs
- `comp-create-large`: 110399 µs vs 148351 µs
- `comp-create-small`: 66303 µs vs 88831 µs
- `comp-read-latest`: 35775 µs vs 74943 µs
- `comp-read-version`: 33983 µs vs 73599 µs
- `comp-update`: 75199 µs vs 135167 µs
- `contribution-commit`: 45823 µs vs 90303 µs
- `dir-read`: 25039 µs vs 36255 µs
- `dir-update`: 67199 µs vs 88703 µs
- `ehr-create`: 20527 µs vs 43775 µs
- `history-read`: 16295 µs vs 51327 µs
- `status-update`: 30447 µs vs 75519 µs

## Where ehrbase-java wins (p99, computed)

- `ehr-read`: 62943 µs vs 64703 µs

## Limitations

Single run per SUT (no inter-run variance yet — the ≥5-run protocol is the publication step); same host, sequential execution; see each run's own `REPORT.md` §Limitations for sampler availability.
