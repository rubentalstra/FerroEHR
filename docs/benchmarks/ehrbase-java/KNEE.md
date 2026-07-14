# Maximum sustained throughput (knee) — EHRbase upstream

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Method: `docs/design/benchmark/01-measurement.md` §3, `docs/design/benchmarking.md` §2.2.

**Knee: L = 64 → 643.0 req/s at p99 46783 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%).

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 70527 | 1209 | 14 | sustained |
| 4 | 40.2 | 0.000% | 40127 | 4823 | 12 | sustained |
| 16 | 161.4 | 0.000% | 27103 | 19367 | 25 | sustained |
| 64 | 643.0 | 0.018% | 46783 | 77155 | 61 | sustained |
| 128 | 1268.8 | 0.318% | 758783 | 152259 | 17 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

