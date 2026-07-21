# Maximum sustained throughput (knee) — ehrbase-rs 3.5.0

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Latencies are coordinated-omission-corrected against planned send times.

**Knee: L = 64 → 622.4 req/s (37341 req/min) at p99 91199 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%) — sustaining 15419.5 clinical events/min.

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 38047 | 1209 | 9 | sustained |
| 2 | 20.1 | 0.000% | 33023 | 2416 | 5 | sustained |
| 4 | 40.1 | 0.000% | 25343 | 4817 | 4 | sustained |
| 8 | 80.5 | 0.000% | 22991 | 9663 | 16 | sustained |
| 16 | 160.5 | 0.005% | 15599 | 19263 | 30 | sustained |
| 32 | 316.2 | 0.008% | 18159 | 37940 | 18 | sustained |
| 48 | 479.8 | 0.005% | 50815 | 57573 | 11 | sustained |
| 64 | 622.4 | 0.021% | 91199 | 74682 | 16 | sustained |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): the ≥5-run protocol (benchmarking.md §4.4) is the publication step; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

