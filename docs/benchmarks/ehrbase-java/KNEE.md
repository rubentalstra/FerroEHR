# Maximum sustained throughput (knee) — EHRbase upstream

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Method: `docs/design/benchmark/01-measurement.md` §3, `docs/design/benchmarking.md` §2.2.

**Knee: L = 16 → 160.5 req/s (9632 req/min) at p99 31583 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%) — sustaining 3981.0 clinical events/min.

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 102335 | 1209 | 12 | sustained |
| 2 | 20.1 | 0.000% | 41119 | 2416 | 14 | sustained |
| 4 | 40.1 | 0.000% | 119295 | 4817 | 23 | sustained |
| 8 | 80.5 | 0.000% | 142335 | 9663 | 24 | sustained |
| 16 | 160.5 | 0.005% | 31583 | 19263 | 36 | sustained |
| 18 | 179.0 | 1.985% | 25116671 | 21482 | 20 | SLO breached |
| 20 | 192.1 | 1.790% | 11968511 | 23049 | 271 | SLO breached |
| 24 | 215.7 | 9.868% | 43778047 | 25886 | 90 | SLO breached |
| 32 | 302.7 | 4.262% | 44072959 | 36326 | 62 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

