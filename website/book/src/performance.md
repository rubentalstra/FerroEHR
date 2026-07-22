# Performance

EHRbase-rs makes the same discipline it applies to conformance apply to
performance: a *class* is not a marketing label a vendor writes down, it is a
verdict a server **earns by measurement** on a stated environment, or does
not. The performance chapter of the CNF suite runs an open-loop clinical
workload at a published offered-load floor, records the result as a
re-checkable histogram, and lets the verdict pipeline recompute — earned or
not earned — from that artifact. Nothing on this page is hand-typed; every
number comes from the committed measurement records or the generated assets
below.

<!-- toc -->

## The volumetric class ladder

Performance conformance is graded on a small, closed ladder of deployment
classes — proof-of-concept, small, large, and regional. Each class fixes an
**offered-load floor** (the peak API arrival rate the server must sustain), a
**latency budget** (a p99 service-level objective), and an **error budget**
(zero: a failed request under load is a failed class). A class is *earned*
only when a measured run holds every threshold; a class is never *declared*.

Crucially, a class verdict is **environment-bound**: it is meaningful only
alongside the hardware, core count, memory, storage class, and topology it was
measured on, which the runner records in the measurement's environment block
and stamps into every asset. The same binary earns different classes on
different hardware, and the artifact always says which.

![The performance class ladder](perf-assets/perf-class-ladder.svg)

## Where the floors come from

The offered-load floors are not chosen for effect — they are **anchored to
population**, so a class corresponds to a real catchment a deployment might
serve. The derivation is a short chain of published, official activity
statistics:

- **Clinical documents per person per year.** Summing the major encounter
  types that each commit a clinical document gives roughly forty-six
  documents per capita per year: primary- and specialist-care consultations
  (OECD, *Health at a Glance 2023*), inpatient discharges (OECD/Eurostat
  hospital discharge statistics), emergency-department visits (OECD
  emergency-care indicators), laboratory reports (Royal College of
  Pathologists activity data), diagnostic-imaging events (NHS England
  *Diagnostic Imaging Dataset*, 2023/24), and dispensed prescriptions (NHS
  Business Services Authority *Prescription Cost Analysis*, 2024/25).
- **Average write rate.** Multiplying a class's served population by that
  per-capita rate and dividing by the number of seconds in a year gives the
  average sustained document-write rate for the class.
- **Busy-hour peak.** Real clinical traffic is not flat: it concentrates in
  ward rounds and clinic hours. Following the ITU-T E.500 busy-hour
  engineering convention, the average is scaled to a busy-hour peak by a peak
  factor of eight.
- **Read multiplier.** A CDR is a read-heavy OLTP system — charts are read far
  more often than they are written. Following the read-heavy OLTP convention
  used by standard database benchmarks (YCSB, OLTP-Bench), the offered load
  applies a read-to-write ratio of 10:1 on top of the write rate.

The floors that fall out of this chain are the published defaults the runner
enforces; the concrete rates per class are carried in the class ladder above
and the summary table below, never re-typed into this prose.

## How a measured run works

A performance run is deliberately **open-loop**: the runner plays a seeded
arrival schedule — request *i* is *due* at a planned instant computed before
the run starts — rather than a closed loop of virtual users that would slow
its own offered load down whenever the server stalls. This makes the run
**coordinated-omission-free**: each request's latency is measured from its
*planned* arrival instant, so a server that pauses cannot hide the queue it
built up behind a handful of fast replies.

A run has two phases: a discarded **warmup** window (caches fill, pools warm,
the JIT of a compared server settles) followed by the **sustained measurement
window** at the class's offered-load floor. Latencies are accumulated into
per-operation **HDR V2 histograms**, which are base64-encoded verbatim into
the measurement record in `docs/conformance/<sut>/results.json`. Because the
full histogram is embedded — not just a handful of pre-computed percentiles —
any consumer can re-derive every percentile and re-check every threshold from
the committed artifact. The class verdict itself is then **recomputed by the
verdict pipeline** from those records: the runner never writes a verdict it
cannot reproduce from the data.

The measured corpus is seeded strictly through the public write path (create
EHR, commit composition) — never a database backdoor — so what the run reads
is exactly what the server's own write path produced. The corpus contract is
the scale-ladder recipe in the runner's committed artifacts.

## Reproducing it

The measured run is a stage of the conformance pipeline. Selecting a class
seeds the matching scale corpus, plays the open-loop schedule against the
composed SUT, and merges the measurement records into `results.json`:

```bash
# seed the class corpus, run the open-loop schedule, merge the record
CONF_PERF_CLASS=POC bash scripts/conformance.sh
```

The runner subcommand can also be driven directly against a running SUT — the
`ixit` topology file supplies the mandatory environment block:

```bash
# a full measured run of the proof-of-concept class
cnf-runner perf --root tools/cnf-runner/artifacts \
                --ixit <ixit.json> --results <results.json> --class POC

# an exploratory smoke run — tiny corpus, seconds-long window, NEVER
# persisted (the record would not realize the case's workload)
cnf-runner perf --root tools/cnf-runner/artifacts \
                --ixit <ixit.json> --results <results.json> \
                --class POC --smoke
```

The published assets are rendered from the committed `results.json` by
`cnf-runner perf-assets` (wrapped by `scripts/render-perf-assets.sh`); the
docs CI job re-renders and `git diff`s them, so a hand-edited or stale asset
fails the build.

## The latest measured run

The per-operation percentiles below are re-derived at build time from the
committed HDR V2 histograms for the proof-of-concept class:

![Proof-of-concept class latency, re-derived from the committed histograms](perf-assets/perf-latency-class-POC.svg)

{{#include ../generated/perf-summary.md}}
