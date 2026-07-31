# Benchmarks

FerroEHR measures performance with the **same instrument family that
measures conformance** — the built-in CNF runner. There is no separate
benchmark harness: every published number is measured by a committed,
re-runnable instrument, reported in both directions, and regenerated from
committed artifacts. There is no marketing chart in this project that you
cannot regenerate yourself with one command.

<!-- toc -->

## The three instruments

- **Measured class runs** (`cnf-runner perf`, wrapped by
  `CONF_PERF_CLASS=… bash scripts/conformance.sh`) — conformance by
  measurement: the open-loop hospital-simulation workload holds a
  population-anchored offered-load floor for the normative hour (or an
  extended 2–12 h hold), and the volumetric deployment class is **earned or
  not** from the committed record. Every measurement embeds re-checkable
  HDR histograms and per-container resource telemetry. See
  [Performance](performance.md).
- **The step-load stress ladder** (`cnf-runner stress`) — exploration: the
  same workload at geometrically climbing rates until the system leaves the
  envelope, then bisection to the **maximum sustainable throughput** (the
  knee of the latency-throughput curve). Each rung embeds its own
  histograms and resource telemetry; a rung where the load *generator* fell
  behind is flagged generator-bound, never counted against the server. A
  stress report earns no class.
- **The AQL probe** (`cnf-runner aql-probe`) — diagnosis: the instrument's
  AQL set fired repeatedly against a freshly seeded corpus, with
  wire-latency percentiles and the database-side cost attributed per SQL
  statement. The optimization loop's entry point; exploration evidence
  only.

## What the workload simulates

All three instruments drive the same **hospital simulation**: clinical
journeys — admissions, shift vitals, medication rounds, laboratory results
arriving asynchronously, chart reviews, AQL ward dashboards, corrections,
discharges — expanded from a committed journey catalogue onto an open-loop
arrival schedule, with payloads built from **official openEHR CKM
templates** vendored with provenance. Every stage is its own planned
arrival instant, so latency is measured from the *planned* time
(coordinated-omission-corrected) and a stalled server cannot hide. The full
workload story lives in [Performance](performance.md#the-hospital-simulation).

## Running the instruments

```bash
# the measured class run (the conformance pipeline's perf stage)
CONF_PERF_CLASS=POC bash scripts/conformance.sh

# the step-load stress ladder (fresh compose + seed, then the climb)
cnf-runner stress --root tools/cnf-runner/artifacts \
                  --ixit tools/cnf-runner/party/ferroehr/ixit.json \
                  --out docs/conformance/ferroehr/stress.json

# the AQL optimization probe
cnf-runner aql-probe --root tools/cnf-runner/artifacts \
                     --ixit tools/cnf-runner/party/ferroehr/ixit.json \
                     --out docs/conformance/ferroehr/aql-probe.json
```

Every instrument seeds a freshly composed, empty server through the public
API and the stack is torn down afterwards — there is no seed reuse, so no
run ever measures another run's leftovers. Committed records land under
`docs/conformance/<sut>/`; the published charts regenerate from them
(`scripts/render-perf-assets.sh`, `scripts/render-comparison.sh`) and are
diff-guarded in CI.

## Fairness rules

The comparison methodology is enforced by construction: the same runner
drives both servers against the same committed catalogue and ladder, each
on its own freshly composed stack with its own committed party statement;
payload skeletons are byte-identical; database maintenance is settled
deterministically on both sides before every measured window; configuration
parity is explicit (connection pools raised in lockstep; version signing —
a FerroEHR extension upstream does not perform — disabled for throughput
comparisons and labeled). Both directions publish on equal footing: where
upstream sustains more, its curve says so exactly like the reverse — see
[Comparison](comparison.md).
