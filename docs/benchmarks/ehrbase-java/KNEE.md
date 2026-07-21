# Maximum sustained throughput (knee) — EHRbase upstream

> Generated from `knee.json` (never hand-typed). Scale **10k**. The `hour` rate shape is driven at an ascending load-factor ladder on short fixed windows; the ladder stops at the first step past the SLO (p99 > 1 s) or the 0.1% error-rate flag. Latencies are coordinated-omission-corrected against planned send times.

**Knee: L = 48 → 475.0 req/s (28500 req/min) at p99 575487 µs** (the last sustainable step; SLO p99 ≤ 1 s, error ≤ 0.1%) — sustaining 11755.0 clinical events/min.

## Ladder

| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |
|--:|--:|--:|--:|--:|--:|---|
| 1 | 10.1 | 0.000% | 60575 | 1209 | 13 | sustained |
| 2 | 20.1 | 0.000% | 39743 | 2416 | 9 | sustained |
| 4 | 40.1 | 0.000% | 34335 | 4817 | 14 | sustained |
| 8 | 80.5 | 0.000% | 33023 | 9663 | 24 | sustained |
| 16 | 160.5 | 0.005% | 108671 | 19263 | 44 | sustained |
| 32 | 316.2 | 0.011% | 44607 | 37939 | 12 | sustained |
| 48 | 475.0 | 0.061% | 575487 | 56999 | 48 | sustained |
| 52 | 486.9 | 6.807% | 26361855 | 58422 | 10 | SLO breached |
| 56 | 478.3 | 13.586% | 28426239 | 57398 | 11 | SLO breached |
| 64 | 540.6 | 14.420% | 28934143 | 64872 | 26 | SLO breached |

![Knee — sustained req/s vs p99 latency](charts/knee.svg)

## Limitations

- **Single run per step** (no inter-run variance): a multi-run protocol with coefficient of variation is the certification bar; these numbers are indicative, not certified.
- **Same-host load generator:** the generator competes for CPU with the SUT at high load, so the measured knee is a **lower bound** on the SUT's real capacity — an isolated load generator would push it higher.
- Provisioning is re-applied idempotently at each step; scale seeding runs once before the ladder.

