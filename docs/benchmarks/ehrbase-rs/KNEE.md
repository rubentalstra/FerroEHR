# Maximum sustained throughput (knee) — ehrbase-rs 3.0.0

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Method: `docs/design/benchmark/01-measurement.md` §3, `docs/design/benchmarking.md` §2.2.

**Knee: L = 40 → 396.9 req/s (23814 req/min) at p99 174463 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%) — sustaining 9832.0 clinical events/min.

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 32 | 318.3 | 0.050% | 788479 | 38191 | 14 | sustained |
| 40 | 396.9 | 0.010% | 174463 | 47629 | 14 | sustained |
| 42 | 415.9 | 2.409% | 2660351 | 49912 | 54 | SLO breached |
| 44 | 448.0 | 1.173% | 1887231 | 53763 | 63 | SLO breached |
| 48 | 453.7 | 3.989% | 3489791 | 54448 | 54 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

