# Benchmark comparison (generated)

> **Measured, not asserted.** Every number below is read from a committed `results.json`; both directions are reported. The workload, client, and host are identical by construction (`docs/design/benchmarking.md` §3).

## Runs

| | Product | Profile | Scale | Ward | Requests | req/s | Error rate |
|---|---|---|---|--:|--:|--:|--:|
| **ehrbase-rs** | ehrbase-rs 3.0.0 | smoke | 10k | 20 | 331 | 2.8 | 0.000% |
| **ehrbase-java** | EHRbase upstream | smoke | 10k | 20 | 331 | 2.8 | 0.000% |

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
| **ehrbase-rs** | 50.3 | 23.2 |
| **ehrbase-java** | 16.9 | 4.8 |

## Latency — p99 per operation class

![p99 latency per operation class](charts/comparison-p99.svg)

## Latency — p50 per operation class

![p50 latency per operation class](charts/comparison-p50.svg)

## Per-class detail (µs)

| Class | ehrbase-rs p50 | ehrbase-java p50 | ehrbase-rs p90 | ehrbase-java p90 | ehrbase-rs p99 | ehrbase-java p99 | ehrbase-rs p99.9 | ehrbase-java p99.9 | ehrbase-rs max | ehrbase-java max | ehrbase-rs err | ehrbase-java err | p99 gap |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|---|
| aql-patient | 37535 | 42239 | 52031 | 55903 | 58143 | 225663 | 58143 | 225663 | 58143 | 225663 | 0 | 0 | 3.9× |
| aql-ward | 32159 | 17887 | 56799 | 55167 | 72959 | 201087 | 72959 | 201087 | 72959 | 201087 | 0 | 0 | 2.8× |
| comp-create-large | 50559 | 48351 | 87615 | 93311 | 87615 | 93311 | 87615 | 93311 | 87615 | 93311 | 0 | 0 | 1.1× |
| comp-create-small | 32191 | 27279 | 54559 | 45567 | 78655 | 98879 | 78655 | 98879 | 78655 | 98879 | 0 | 0 | 1.3× |
| comp-read-latest | 26095 | 24127 | 37887 | 41663 | 47935 | 64895 | 47935 | 64895 | 47935 | 64895 | 0 | 0 | 1.4× |
| comp-read-version | 25471 | 21551 | 37855 | 38783 | 47583 | 86975 | 47583 | 86975 | 47583 | 86975 | 0 | 0 | 1.8× |
| comp-update | 45055 | 34239 | 79103 | 68927 | 80959 | 165119 | 80959 | 165119 | 80959 | 165119 | 0 | 0 | 2.0× |
| contribution-commit | 34495 | 40511 | 45087 | 78719 | 63903 | 147327 | 63903 | 147327 | 63903 | 147327 | 0 | 0 | 2.3× |
| dir-read | 21247 | 19343 | 31791 | 37311 | 33279 | 53279 | 33279 | 53279 | 33279 | 53279 | 0 | 0 | 1.6× |
| dir-update | 45983 | 41087 | 75839 | 82751 | 75839 | 82751 | 75839 | 82751 | 75839 | 82751 | 0 | 0 | 1.1× |
| ehr-create | 21775 | 15703 | 46943 | 43871 | 46943 | 43871 | 46943 | 43871 | 46943 | 43871 | 0 | 0 | 1.1× |
| ehr-read | 64831 | 63551 | 72767 | 67391 | 72767 | 67391 | 72767 | 67391 | 72767 | 67391 | 0 | 0 | 1.1× |
| history-read | 26815 | 24639 | 34463 | 56735 | 35295 | 61023 | 35295 | 61023 | 35295 | 61023 | 0 | 0 | 1.7× |
| status-update | 41119 | 31775 | 49727 | 84287 | 49727 | 84287 | 49727 | 84287 | 49727 | 84287 | 0 | 0 | 1.7× |

## Resources

| | Idle RSS | Peak RSS | Mean CPU | Cold start | Storage bytes/composition |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 54 MB | 122 MB | 5.5% | 11531 ms | 28198 |
| **ehrbase-java** | 532 MB | 593 MB | 16.3% | 11827 ms | 33463 |

## Where ehrbase-rs wins (p99, computed)

- `aql-patient`: 58143 µs vs 225663 µs
- `aql-ward`: 72959 µs vs 201087 µs
- `comp-create-large`: 87615 µs vs 93311 µs
- `comp-create-small`: 78655 µs vs 98879 µs
- `comp-read-latest`: 47935 µs vs 64895 µs
- `comp-read-version`: 47583 µs vs 86975 µs
- `comp-update`: 80959 µs vs 165119 µs
- `contribution-commit`: 63903 µs vs 147327 µs
- `dir-read`: 33279 µs vs 53279 µs
- `dir-update`: 75839 µs vs 82751 µs
- `history-read`: 35295 µs vs 61023 µs
- `status-update`: 49727 µs vs 84287 µs

## Where ehrbase-java wins (p99, computed)

- `ehr-create`: 43871 µs vs 46943 µs
- `ehr-read`: 67391 µs vs 72767 µs

## Limitations

Single run per SUT (no inter-run variance yet — the ≥5-run protocol is the publication step); same host, sequential execution; see each run's own `REPORT.md` §Limitations for sampler availability.
