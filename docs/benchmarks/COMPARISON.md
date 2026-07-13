# Benchmark comparison (generated)

> **Measured, not asserted.** Every number below is read from a committed `results.json`; both directions are reported. The workload, client, and host are identical by construction (`docs/design/benchmarking.md` §3).

## Runs

| | Product | Profile | Scale | Ward | Requests | req/s | Error rate |
|---|---|---|---|--:|--:|--:|--:|
| **ehrbase-rs** | ehrbase-rs 3.0.0 | smoke | 10k | 20 | 331 | 2.8 | 0.000% |
| **ehrbase-java** | EHRbase upstream | smoke | 10k | 20 | 77 | 0.6 | 79.245% |

## Latency — p99 per operation class

![p99 latency per operation class](charts/comparison-p99.svg)

## Latency — p50 per operation class

![p50 latency per operation class](charts/comparison-p50.svg)

## Per-class detail (µs)

| Class | ehrbase-rs p50 | ehrbase-java p50 | ehrbase-rs p99 | ehrbase-java p99 | p99 gap |
|---|--:|--:|--:|--:|---|
| aql-patient | 37183 | 33823 | 73727 | 125631 | 1.7× |
| aql-ward | 37087 | 21823 | 56927 | 44767 | 1.3× |
| dir-read | 16895 | 24319 | 64287 | 58495 | 1.1× |
| dir-update | 69951 | 76159 | 78911 | 106111 | 1.3× |
| ehr-create | 21519 | 36191 | 39647 | 51519 | 1.3× |
| ehr-read | 66623 | 65599 | 75135 | 69311 | 1.1× |
| status-update | 29199 | 40415 | 41215 | 93503 | 2.3× |

## Resources

| | Idle RSS | Peak RSS | Mean CPU | Cold start | Storage bytes/composition |
|---|--:|--:|--:|--:|--:|
| **ehrbase-rs** | 144 MB | 191 MB | 5.4% | 11491 ms | 28236 |
| **ehrbase-java** | 568 MB | 593 MB | 15.9% | 950 ms | 33355 |

## Where ehrbase-rs wins (p99, computed)

- `aql-patient`: 73727 µs vs 125631 µs
- `dir-update`: 78911 µs vs 106111 µs
- `ehr-create`: 39647 µs vs 51519 µs
- `status-update`: 41215 µs vs 93503 µs

## Where ehrbase-java wins (p99, computed)

- `aql-ward`: 44767 µs vs 56927 µs
- `dir-read`: 58495 µs vs 64287 µs
- `ehr-read`: 69311 µs vs 75135 µs

## Limitations

Single run per SUT (no inter-run variance yet — the ≥5-run protocol is the publication step); same host, sequential execution; see each run's own `REPORT.md` §Limitations for sampler availability.
