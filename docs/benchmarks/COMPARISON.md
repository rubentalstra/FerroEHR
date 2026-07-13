# Benchmark comparison (generated)

> **Measured, not asserted.** Every number below is read from a committed `results.json`; both directions are reported. The workload, client, and host are identical by construction (`docs/design/benchmarking.md` §3).

## Runs

| | Product | Profile | Scale | Ward | Requests | req/s | Error rate |
|---|---|---|---|--:|--:|--:|--:|
| **ehrbase-rs** | ehrbase-rs 3.0.0 | smoke | 10k | 20 | 331 | 2.8 | 0.000% |
| **ehrbase-java** | EHRbase upstream | smoke | 10k | 20 | 309 | 2.6 | 6.647% |

## Latency — p99 per operation class

![p99 latency per operation class](charts/comparison-p99.svg)

## Latency — p50 per operation class

![p50 latency per operation class](charts/comparison-p50.svg)

## Per-class detail (µs)

| Class | ehrbase-rs p50 | ehrbase-java p50 | ehrbase-rs p99 | ehrbase-java p99 | p99 gap |
|---|--:|--:|--:|--:|---|
| aql-patient | 37535 | 30783 | 58143 | 161535 | 2.8× |
| aql-ward | 32159 | 23199 | 72959 | 192639 | 2.6× |
| comp-create-large | 50559 | 33343 | 87615 | 136191 | 1.6× |
| comp-create-small | 32191 | 26735 | 78655 | 182783 | 2.3× |
| comp-read-latest | 26095 | 25647 | 47935 | 103295 | 2.2× |
| comp-read-version | 25471 | 22847 | 47583 | 101695 | 2.1× |
| comp-update | 45055 | 35647 | 80959 | 152703 | 1.9× |
| dir-read | 21247 | 19743 | 33279 | 204671 | 6.2× |
| dir-update | 45983 | 75327 | 75839 | 124351 | 1.6× |
| ehr-create | 21775 | 10423 | 46943 | 70271 | 1.5× |
| ehr-read | 64831 | 66175 | 72767 | 122111 | 1.7× |
| history-read | 26815 | 21663 | 35295 | 91199 | 2.6× |
| status-update | 41119 | 35647 | 49727 | 65791 | 1.3× |

## Resources

| | Idle RSS | Peak RSS | Mean CPU | Cold start | Storage bytes/composition |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 54 MB | 122 MB | 5.5% | 11531 ms | 28198 |
| **ehrbase-java** | 591 MB | 622 MB | 24.3% | 16709 ms | 33500 |

## Where ehrbase-rs wins (p99, computed)

- `aql-patient`: 58143 µs vs 161535 µs
- `aql-ward`: 72959 µs vs 192639 µs
- `comp-create-large`: 87615 µs vs 136191 µs
- `comp-create-small`: 78655 µs vs 182783 µs
- `comp-read-latest`: 47935 µs vs 103295 µs
- `comp-read-version`: 47583 µs vs 101695 µs
- `comp-update`: 80959 µs vs 152703 µs
- `dir-read`: 33279 µs vs 204671 µs
- `dir-update`: 75839 µs vs 124351 µs
- `ehr-create`: 46943 µs vs 70271 µs
- `ehr-read`: 72767 µs vs 122111 µs
- `history-read`: 35295 µs vs 91199 µs
- `status-update`: 49727 µs vs 65791 µs

## Where ehrbase-java wins (p99, computed)

No class won on p99 in this run pair.

## Limitations

Single run per SUT (no inter-run variance yet — the ≥5-run protocol is the publication step); same host, sequential execution; see each run's own `REPORT.md` §Limitations for sampler availability.
