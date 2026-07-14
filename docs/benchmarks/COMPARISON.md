# Benchmark comparison (generated)

> **Measured, not asserted.** Every number below is read from a committed `results.json`; both directions are reported. The workload, client, and host are identical by construction (`docs/design/benchmarking.md` §3).

## Runs

| | Product | Profile | Scale | Ward | Requests | req/s | req/min | Error rate |
|---|---|---|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | ehrbase-rs 3.0.0 | hour | 100k | 20 | 1209 | 0.3 | 20 | 0.000% |
| **ehrbase-java** | EHRbase upstream | hour | 100k | 20 | 1209 | 0.3 | 20 | 0.000% |

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
| **ehrbase-rs** | 35.7 | 1.4 |
| **ehrbase-java** | 16.3 | 0.5 |

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
| **total** | **0** | **0** | **0.0** | **0** | **0** | **0.0** |

## Latency — p99 per operation class

![p99 latency per operation class](charts/comparison-p99.svg)

## Latency — p50 per operation class

![p50 latency per operation class](charts/comparison-p50.svg)

## Per-class detail (µs)

| Class | ehrbase-rs p50 | ehrbase-java p50 | ehrbase-rs p90 | ehrbase-java p90 | ehrbase-rs p99 | ehrbase-java p99 | ehrbase-rs p99.9 | ehrbase-java p99.9 | ehrbase-rs max | ehrbase-java max | ehrbase-rs err | ehrbase-java err | p99 gap |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|---|
| aql-patient | 96575 | 39711 | 121151 | 86335 | 183935 | 126271 | 536575 | 298751 | 536575 | 298751 | 0 | 0 | 1.5× |
| aql-ward | 86911 | 30031 | 146943 | 66175 | 154367 | 82879 | 154367 | 82879 | 154367 | 82879 | 0 | 0 | 1.9× |
| comp-create-large | 78399 | 63679 | 156671 | 145535 | 156671 | 145535 | 156671 | 145535 | 156671 | 145535 | 0 | 0 | 1.1× |
| comp-create-small | 41311 | 33087 | 66559 | 92223 | 86911 | 117631 | 142463 | 129983 | 142463 | 129983 | 0 | 0 | 1.4× |
| comp-read-latest | 30207 | 23487 | 47167 | 73535 | 75903 | 93567 | 163327 | 99455 | 163327 | 99455 | 0 | 0 | 1.2× |
| comp-read-version | 28543 | 21247 | 46079 | 70655 | 75967 | 91967 | 160895 | 96831 | 160895 | 96831 | 0 | 0 | 1.2× |
| comp-update | 44735 | 45983 | 64479 | 91391 | 77631 | 140543 | 77631 | 140543 | 77631 | 140543 | 0 | 0 | 1.8× |
| contribution-commit | 41727 | 40383 | 68095 | 90687 | 82751 | 122815 | 82751 | 122815 | 82751 | 122815 | 0 | 0 | 1.5× |
| dir-read | 26623 | 22895 | 44703 | 73087 | 52031 | 80639 | 52031 | 80639 | 52031 | 80639 | 0 | 0 | 1.5× |
| dir-update | 36159 | 54431 | 147583 | 142591 | 147583 | 142591 | 147583 | 142591 | 147583 | 142591 | 0 | 0 | 1.0× |
| ehr-create | 60447 | 78655 | 80895 | 93375 | 80895 | 93375 | 80895 | 93375 | 80895 | 93375 | 0 | 0 | 1.2× |
| ehr-read | 131455 | 113407 | 141695 | 116223 | 141695 | 116223 | 141695 | 116223 | 141695 | 116223 | 0 | 0 | 1.2× |
| history-read | 26319 | 34047 | 38207 | 60959 | 46143 | 85951 | 46143 | 85951 | 46143 | 85951 | 0 | 0 | 1.9× |
| status-update | 47679 | 68031 | 76671 | 85567 | 76671 | 85567 | 76671 | 85567 | 76671 | 85567 | 0 | 0 | 1.1× |

## Resources

| | Idle RSS | Peak RSS | Mean CPU | Cold start | Storage bytes/composition |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 106 MB | 255 MB | 0.9% | 109984 ms | 25147 |
| **ehrbase-java** | 640 MB | 678 MB | 2.1% | 17678 ms | 31836 |

## Where ehrbase-rs wins (p99, computed)

- `comp-create-small`: 86911 µs vs 117631 µs
- `comp-read-latest`: 75903 µs vs 93567 µs
- `comp-read-version`: 75967 µs vs 91967 µs
- `comp-update`: 77631 µs vs 140543 µs
- `contribution-commit`: 82751 µs vs 122815 µs
- `dir-read`: 52031 µs vs 80639 µs
- `ehr-create`: 80895 µs vs 93375 µs
- `history-read`: 46143 µs vs 85951 µs
- `status-update`: 76671 µs vs 85567 µs

## Where ehrbase-java wins (p99, computed)

- `aql-patient`: 126271 µs vs 183935 µs
- `aql-ward`: 82879 µs vs 154367 µs
- `comp-create-large`: 145535 µs vs 156671 µs
- `dir-update`: 142591 µs vs 147583 µs
- `ehr-read`: 116223 µs vs 141695 µs

## Limitations

Single run per SUT (no inter-run variance yet — the ≥5-run protocol is the publication step); same host, sequential execution; see each run's own `REPORT.md` §Limitations for sampler availability.
