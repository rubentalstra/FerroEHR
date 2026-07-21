# Maximum sustained throughput (knee) — ehrbase-rs 3.5.0

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Latencies are coordinated-omission-corrected against planned send times.

**Knee: L = 64 → 631.5 req/s (37890 req/min) at p99 204671 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%) — sustaining 15642.0 clinical events/min.

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 50399 | 1209 | 11 | sustained |
| 2 | 20.1 | 0.000% | 29039 | 2416 | 11 | sustained |
| 4 | 40.1 | 0.000% | 26575 | 4817 | 45 | sustained |
| 8 | 80.5 | 0.000% | 26447 | 9663 | 15 | sustained |
| 16 | 160.5 | 0.000% | 16831 | 19264 | 26 | sustained |
| 32 | 316.2 | 0.008% | 50751 | 37940 | 17 | sustained |
| 64 | 631.5 | 0.032% | 204671 | 75779 | 12 | sustained |
| 72 | 557.1 | 22.509% | 26509311 | 66855 | 486 | SLO breached |
| 80 | 620.5 | 21.771% | 28196863 | 74464 | 269 | SLO breached |
| 96 | 637.1 | 34.621% | 28573695 | 76450 | 13 | SLO breached |
| 128 | 575.2 | 53.477% | 28491775 | 69022 | 107 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): a multi-run protocol with coefficient of variation is the certification bar; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

