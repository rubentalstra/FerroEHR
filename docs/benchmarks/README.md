# Benchmarks and measured performance

Performance is reported by two complementary instruments that share one
workload payload and one measurement discipline. Every published figure is
derived from a committed artifact; nothing here is hand-typed.

## The benchmark lab (profile-first)

The benchmark harness (`tools/benchmark`, driven by `scripts/benchmark.sh`)
models a **hospital day** — admissions, shift vitals, medication rounds,
laboratory contributions, chart reviews, corrections, discharges — built
from official openEHR CKM templates with seeded determinism, so every run
and every compared server receives byte-identical requests. It measures
per-operation latency (coordinated-omission-corrected), the **saturation
knee** (the highest sustained load holding the latency objective inside the
error budget), resource footprint, cold start, and storage per composition,
and it publishes a **cross-SUT comparison** against upstream EHRbase (Java)
in both directions.

Its artifacts live per system under this directory
(`docs/benchmarks/<sut>/` — `results.json`, `REPORT.md`, `charts/`, raw
histograms); the head-to-head comparison against upstream is published on the
website comparison page and in `../conformance/COMPARISON.md`.

## The CNF measured class runs (conformance-by-measurement)

The CNF conformance runner (`tools/cnf-runner`) grades performance as a
**verdict earned by measurement** on a stated environment: an open-loop
offered-load run against a volumetric deployment class, held to a published
latency and error budget. Each run embeds re-checkable HDR histograms so any
consumer re-derives the percentiles, and the class verdict — earned or not —
is recomputed by the verdict pipeline from the records.

Its measurement records live in the conformance artifacts
(`docs/conformance/<sut>/results.json`, the `measurements` block); the method
and reproduction steps are the website's Performance chapter.

## The step-load stress instrument (exploration)

`cnf-runner stress` climbs short, intense load steps (geometric doubling
with bisection refinement) to the **maximum sustainable throughput** inside
a latency budget — the knee of the latency-throughput curve. It is
exploration, never conformance: its report
(`docs/conformance/<sut>/stress.json`, schema-published, environment-bound,
per-step re-checkable histograms) earns no class and never touches the
conformance results; the class floors appear in it as context only.

## The canonical commands

```bash
# the conformance pipeline (compose fresh → catalogue → verdicts → badges)
bash scripts/conformance.sh

# + the measured class stage (hour-plus, exclusive SUT; extended holds)
CONF_PERF_CLASS=POC CONF_PERF_HOURS=1 bash scripts/conformance.sh

# the step-load stress ladder (exploration only)
cargo run -p cnf-runner -- stress --root tools/cnf-runner/artifacts \
  --ixit tools/cnf-runner/party/<sut>/ixit.json \
  --out docs/conformance/<sut>/stress.json --skip-seed

# regenerate every published perf/stress visual FROM committed artifacts
bash scripts/render-perf-assets.sh

# the benchmark lab (hospital-day profile + cross-SUT compare)
bash scripts/benchmark.sh
```

## One pipeline, one payload provenance

Both instruments seed strictly through the public write path (create EHR,
commit composition) — never a database backdoor — and both draw their
clinical documents from the same provenance-stamped template corpus. The
benchmark lab answers *how fast, versus whom*; the CNF class runs answer
*which deployment class this build has earned, on this hardware*. Together
they are the measured half of the project's honest-publication rule: a claim
without a committed measurement does not ship.
