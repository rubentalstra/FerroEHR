# Benchmarks

EHRbase-rs ships its own benchmark instrument, built on the same principle
as the conformance suite: every published number is measured, reproducible,
and reported in both directions. There is no marketing chart in this project
that you cannot regenerate yourself with one command.

<!-- toc -->

## What the benchmark simulates

Rather than an abstract operation list, the workload models a **hospital
day** on a ward of patients: admissions, vital-signs observations every
shift, medication rounds, laboratory results arriving as CONTRIBUTION
batches, clinicians reviewing charts (repeated reads, version history, AQL
dashboards), documentation corrections producing new versions, and
discharges. The clinical documents are built from **official openEHR CKM
templates** (Vital signs, Generic lab test result, ePrescription, eReferral,
GP data set), vendored with provenance, with deterministic seeded variation
so every run — and every compared server — receives byte-identical requests.

The blend works out to roughly 70 % reads / 30 % writes, the capacity-planning
mix. Three profiles compress the day differently: `smoke` (a two-minute
self-test), `hour` (a steady-state measured hour), and `day` (a compressed
day with morning-round and shift-change peaks). Databases are pre-seeded to
a scale rung (`empty`, `10k`, `100k`, `1m` compositions) before measuring.

## What is measured

- **Latency** per operation class (16 classes, from composition creates to
  AQL dashboards): p50/p90/p99/p99.9/max from HdrHistograms, recorded
  against *planned* send times (coordinated-omission-corrected), warmup
  discarded.
- **Throughput** over the window, and the **saturation knee**: `bench knee`
  ramps the identical clinical mix at ascending load factors until p99
  crosses one second or errors exceed 0.1 %, publishing the last sustained
  step. A ladder step where the load generator itself fell behind schedule
  is flagged generator-bound, and a server that stops answering is recorded
  as having **died under load** — a first-class finding, never hidden.
- **Resources**: CPU and memory of the app *and* database containers
  sampled through the run, idle baselines, cold-start time, and storage
  bytes per composition.
- **Charts**: every report embeds generated SVGs (latency ranges, CPU/RSS
  over the run, cross-server comparisons) — committed text files, no
  external tooling.

## Running it

```bash
# our server, from the current sources
bash scripts/benchmark.sh

# the same workload against upstream EHRbase (Java)
BENCH_SUT=ehrbase-java bash scripts/benchmark.sh

# the saturation ladder
BENCH_KNEE=1 BENCH_SCALE=10k bash scripts/benchmark.sh

# the cross-server comparison from two committed runs
bench compare --from docs/benchmarks/ehrbase-rs/results.json \
              --from docs/benchmarks/ehrbase-java/results.json
```

Artefacts land in `docs/benchmarks/<sut>/` (`results.json`, `REPORT.md`,
`charts/`, raw histograms) and the comparison in
`docs/benchmarks/COMPARISON.md`.

## Fairness rules

The comparison methodology is pre-registered and enforced by construction:
the same client code drives both servers (the conformance suite's
transport), the workload is frozen and hashed (`workload.lock`), warmup is
discarded symmetrically, configuration parity is explicit (connection pools
raised in lockstep; version signing — an ehrbase-rs extension upstream does
not perform — is disabled for throughput runs and labeled), and the report
always carries a computed "where the other side wins" section plus a
limitations block. Single-host preview runs are labeled as such; publication
numbers follow a multi-run protocol on dedicated hardware.
