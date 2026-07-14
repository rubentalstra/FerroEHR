# Maximum sustained throughput (knee) — EHRbase upstream

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Method: `docs/design/benchmark/01-measurement.md` §3, `docs/design/benchmarking.md` §2.2.

**Knee: L = 96 → 956.0 req/s at p99 496639 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%).

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 16 | 162.0 | 0.000% | 18431 | 19436 | 11 | sustained |
| 24 | 236.5 | 0.004% | 21567 | 28379 | 42 | sustained |
| 32 | 319.7 | 0.000% | 28799 | 38365 | 10 | sustained |
| 40 | 399.6 | 0.002% | 26335 | 47952 | 12 | sustained |
| 48 | 476.3 | 0.016% | 40415 | 57158 | 17 | sustained |
| 64 | 656.8 | 0.016% | 512511 | 78817 | 14 | sustained |
| 96 | 956.0 | 0.084% | 496639 | 114720 | 17 | sustained |
| 128 | 871.1 | 30.806% | 32866303 | 104534 | 224 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

