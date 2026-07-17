# Maximum sustained throughput (knee) — EHRbase upstream

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Method: `docs/design/benchmark/01-measurement.md` §3, `docs/design/benchmarking.md` §2.2.

**Knee: L = 32 → 316.1 req/s (18968 req/min) at p99 36447 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%) — sustaining 7838.0 clinical events/min.

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 110207 | 1209 | 12 | sustained |
| 2 | 20.1 | 0.000% | 65407 | 2416 | 11 | sustained |
| 4 | 40.1 | 0.000% | 51071 | 4817 | 9 | sustained |
| 8 | 80.5 | 0.000% | 43583 | 9663 | 127 | sustained |
| 16 | 160.5 | 0.005% | 31119 | 19263 | 13 | sustained |
| 32 | 316.1 | 0.016% | 36447 | 37937 | 9 | sustained |
| 36 | 348.4 | 1.219% | 27508735 | 41807 | 16 | SLO breached |
| 40 | 261.6 | 34.422% | 84213759 | 31387 | 548 | SLO breached |
| 48 | 437.4 | 7.964% | 33292287 | 52492 | 46 | SLO breached |
| 64 | 383.7 | 37.739% | 43843583 | 46048 | 81 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

