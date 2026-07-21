# Maximum sustained throughput (knee) — EHRbase upstream

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Latencies are coordinated-omission-corrected against planned send times.

**Knee: L = 44 → 434.2 req/s (26052 req/min) at p99 872959 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%) — sustaining 10744.0 clinical events/min.

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 64767 | 1209 | 11 | sustained |
| 2 | 20.1 | 0.000% | 45119 | 2416 | 9 | sustained |
| 4 | 40.1 | 0.000% | 36351 | 4817 | 30 | sustained |
| 8 | 80.5 | 0.000% | 26975 | 9663 | 7 | sustained |
| 16 | 160.5 | 0.005% | 18751 | 19263 | 12 | sustained |
| 32 | 316.2 | 0.008% | 24767 | 37940 | 18 | sustained |
| 40 | 400.4 | 0.017% | 68735 | 48054 | 45 | sustained |
| 44 | 434.2 | 0.073% | 872959 | 52103 | 14 | sustained |
| 46 | 461.5 | 0.115% | 949759 | 55385 | 36 | SLO breached |
| 48 | 477.9 | 0.389% | 20742143 | 57352 | 15 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

