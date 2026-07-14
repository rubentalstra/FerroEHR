# Maximum sustained throughput (knee) — ehrbase-rs 3.0.0

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Method: `docs/design/benchmark/01-measurement.md` §3, `docs/design/benchmarking.md` §2.2.

**Knee: L = 32 → 319.7 req/s at p99 131327 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%).

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 16 | 162.0 | 0.000% | 23903 | 19436 | 11 | sustained |
| 24 | 236.5 | 0.007% | 150655 | 28378 | 22 | sustained |
| 32 | 319.7 | 0.005% | 131327 | 38363 | 126 | sustained |
| 40 | 398.7 | 0.234% | 891903 | 47841 | 15 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

