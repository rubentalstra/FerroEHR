# Maximum sustained throughput (knee) — ehrbase-rs 3.0.3

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Method: `docs/design/benchmark/01-measurement.md` §3, `docs/design/benchmarking.md` §2.2.

**Knee: L = 64 → 631.6 req/s (37894 req/min) at p99 92223 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%) — sustaining 15647.0 clinical events/min.

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 100031 | 1209 | 28 | sustained |
| 2 | 20.1 | 0.000% | 36735 | 2416 | 13 | sustained |
| 4 | 40.1 | 0.000% | 30959 | 4817 | 10 | sustained |
| 8 | 80.5 | 0.000% | 27599 | 9663 | 13 | sustained |
| 16 | 160.5 | 0.005% | 18959 | 19263 | 19 | sustained |
| 32 | 316.2 | 0.008% | 21807 | 37940 | 14 | sustained |
| 64 | 631.6 | 0.018% | 92223 | 75789 | 11 | sustained |
| 72 | 540.9 | 25.188% | 29540351 | 64908 | 294 | SLO breached |
| 80 | 589.3 | 25.713% | 29474815 | 70716 | 190 | SLO breached |
| 96 | 469.4 | 51.905% | 29032447 | 56332 | 182 | SLO breached |
| 128 | 579.3 | 53.986% | 28491775 | 69514 | 55 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

