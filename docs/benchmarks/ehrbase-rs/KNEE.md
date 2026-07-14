# Maximum sustained throughput (knee) — ehrbase-rs 3.0.0

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Method: `docs/design/benchmark/01-measurement.md` §3, `docs/design/benchmarking.md` §2.2.

**Knee: L = 16 → 161.4 req/s at p99 33951 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%).

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 49535 | 1209 | 7 | sustained |
| 4 | 40.2 | 0.000% | 33535 | 4823 | 11 | sustained |
| 16 | 161.4 | 0.000% | 33951 | 19367 | 63 | sustained |
| 64 | 616.0 | 4.217% | 2330623 | 73915 | 128 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

