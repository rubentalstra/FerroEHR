# Benchmarking ehrbase-rs vs. EHRbase (Java) — an honest methodology

- **Status:** designed, ready to implement (2026-07-07)
- **Owner:** —
- **Purpose:** produce a **defensible, reproducible** performance and resource
  comparison between this project (`ehrbase-rs`) and the reference
  implementation (`ehrbase/ehrbase`, Java/Spring Boot) — one we could hand to a
  skeptic and have them reproduce, not a "trust me bro" marketing chart.
- **Non-negotiable:** every number in the published report is reproducible from
  committed scripts against pinned images, or it does not ship. No cherry-picked
  percentile, no un-warmed JVM strawman, no hidden config advantage.

---

## 0. The "trust me bro" failure modes we are explicitly defeating

Most CDR/database benchmark claims are worthless for one of these reasons. This
methodology is built as the point-by-point antidote:

| Failure mode | How it fakes a win | Our countermeasure |
|---|---|---|
| **Un-warmed JVM** | Measure EHRbase cold; the JIT hasn't compiled hot paths → Rust "wins" 5×. | Mandatory warmup phase (§4.2); report **warm steady-state** as the headline, cold-start separately and labelled. |
| **Cherry-picked percentile** | Report only the metric you win. | Report the **full latency distribution** (p50/p90/p99/p99.9/max) for every operation, always, both directions. |
| **Mean of a skewed distribution** | Averages hide tail latency. | Medians + percentiles + a plotted CDF; means only alongside their standard deviation. |
| **Single run** | Noise looks like signal. | ≥5 independent runs; report median-of-runs + inter-run variance (coefficient of variation); a difference inside the noise band is reported as "no measurable difference," not a win. |
| **Different DB / hardware** | Give your side PG18 + SSD, theirs PG13 + spinning disk. | Same host, same PG major where feasible, pinned CPU/RAM per container; the PG-version axis is measured *explicitly* (§3.3), never a silent advantage. |
| **Unfair config** | Bigger pool / more workers on your side. | Config parity table (§3.4), committed; deviations justified in-report. |
| **Tuned-to-win workload** | Pick only the queries your engine happens to like. | **Pre-registered workload** (§2), defined and committed *before* the first measured run; the AQL set comes from the vendored CNF corpus, not hand-picked. |
| **Toy dataset** | Benchmark on an empty DB where everything is fast. | Scale ladder: empty / 10k / 100k / 1M compositions (§2.3); AQL is measured at every rung. |
| **No raw data** | "Here's a bar chart, trust us." | Publish raw hdrhistogram logs + `results.json` + the exact commands; the report is *generated* from them. |
| **Ignoring where you lose** | Omit the operations the mature system wins. | The report has a mandatory **"Where EHRbase wins"** section; a run that hides a regression is invalid. |

If a result cannot survive these, it does not go in the report.

## 1. What we compare (and what we do not claim)

**In scope:** wire-level behaviour at the openEHR REST surface — the only fair
comparison point, since both implement ITS-REST 1.0.3. We measure the server +
its database as a black box driven over HTTP.

**Explicitly NOT claimed:**
- "Faster at everything." We report per-operation; some operations the JIT-warm
  JVM with a decade of query-planner tuning may win, and we say so.
- Micro-benchmarks of internal Rust vs. Java code — irrelevant to a deployed CDR
  and unfalsifiable at the wire. We benchmark the *system*.
- Correctness parity as a performance claim — that is the conformance suite's
  job (`docs/design/conformance-framework.md`); a fast wrong answer is a
  conformance failure, not a benchmark win. **The benchmark only runs workloads
  both servers answer correctly** (validated by a pre-flight conformance check,
  §4.1) — otherwise we would be timing an error path.

## 2. The pre-registered workload

Defined here, committed, and frozen before the first measured run. Changing it
after seeing results invalidates the run (a `workload.lock` hash is recorded in
each report).

### 2.1 Operation mix (the openEHR CDR surface)

Each is a scenario the harness drives; all payloads come from the vendored CNF
fixture corpus (`docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/`)
so neither server gets a bespoke-tuned input:

| # | Scenario | Payload source | Why it matters |
|---|---|---|---|
| W1 | EHR create | — | baseline write path + id generation |
| W2 | Composition create (small, `minimal_*`) | `compositions/CANONICAL_JSON` | the hot write path |
| W3 | Composition create (large, `all_types`, deep nesting) | `compositions/CANONICAL_JSON/nested*` | serialization + decomposition cost at size |
| W4 | Composition get by version id | (created in W2/W3) | point read / reassembly |
| W5 | Composition get latest | " | current-version resolution |
| W6 | Composition update (new version) | " | versioning write path |
| W7 | Contribution commit (multi-version) | `contributions/valid` | transactional batch write |
| W8 | AQL — simple (`SELECT c FROM COMPOSITION c`) | CNF `query/` corpus | full-scan read |
| W9 | AQL — CONTAINS chain | CNF `query/` groups B/C | the join-heavy path (our nested-set vs their schema) |
| W10 | AQL — aggregation / ORDER BY | CNF `query/` group D | sort + magnitude extraction |
| W11 | Directory (FOLDER) create + get | `directory/` | tree write/read |
| W12 | Template (OPT) upload + list | `valid_templates/` | provisioning |
| W13 | **Mixed realistic** — weighted blend of the above (70% read / 30% write) | all | the number that actually matters for capacity planning |

### 2.2 Load profiles (per scenario)

- **Latency profile:** closed-loop, 1 concurrent client, N=10k iterations after
  warmup — isolates per-request cost without queueing effects.
- **Throughput profile:** open-loop, ramping concurrency {1, 2, 4, 8, 16, 32,
  64, 128}, fixed duration per step, measuring sustained req/s and the latency
  at each concurrency (the throughput-vs-latency "knee" is the real capacity
  signal, not peak req/s).
- **Saturation:** ramp until p99 latency exceeds a fixed SLO (e.g. 1 s) or error
  rate > 0.1%; record the throughput at that point.

### 2.3 Data-scale ladder (the AQL scenarios especially)

Every read/query scenario is measured at four database sizes, seeded
deterministically (same seed, same compositions, both servers):

`empty` · `10k` · `100k` · `1M` compositions across `N` EHRs.

AQL performance *only matters at scale* — the empty-DB number is a warm-path
sanity check; the 100k/1M numbers are the headline. Storage footprint (§3.5) is
measured at each rung too.

## 3. Fairness controls (the heart of it)

### 3.1 Identical host, isolated resources

Both stacks run on the **same physical host**, one at a time (never
simultaneously — no noisy-neighbour contention). Each container is pinned:
`--cpus` and `--memory` fixed and equal (e.g. 4 CPU / 8 GB app, 4 CPU / 8 GB
db), recorded in the report. The load generator runs on a **separate** host (or
at minimum separate pinned cores) so client cost never steals from the server.

### 3.2 Identical client

One load generator drives both, byte-identical requests (same headers, same
auth mode, same `Prefer`, same body). The generator is our own Rust harness
(§5) reusing the conformance `SutClient` — so the client code path is provably
the same for both targets.

### 3.3 Database version — measured, not assumed

EHRbase (Java) targets PostgreSQL 13–16; ehrbase-rs targets PG 18. This is a
real confound. We handle it honestly with **two runs**:
- **"Recommended" run:** each server on its own recommended/supported PG
  (EHRbase on PG16, ehrbase-rs on PG18). This is the "as you'd actually deploy
  it" comparison — the headline for a product claim.
- **"Controlled" run:** both on **PG16** (the newest both support), isolating
  the *application/engine* difference from the *database-version* difference.
  If EHRbase cannot run on PG18 and we cannot run on PG16, we say so and the
  recommended run stands alone with that caveat stated.

The delta between the two runs quantifies how much of any difference is "our
engine" vs. "our newer database" — a number a serious reader wants and a
marketing chart omits.

### 3.4 Config parity

A committed parity table; every asymmetry is either eliminated or justified:

| Axis | Setting | Notes |
|---|---|---|
| Connection pool size | equal (e.g. 16) | the dominant tunable; must match |
| Server worker threads | equal where the model allows | tokio workers vs Spring/Tomcat threads — matched to CPU count, documented as not-identical-by-design |
| Auth | Basic, same credentials, RBAC on both (or off both) | our RBAC on = a *cost we pay*; we do not turn it off to win |
| Signing | **off** for the throughput runs (or on for both) | version signing is our feature; benchmarking with it on-for-us/absent-for-them is unfair — measure both configurations and label them |
| JVM heap / GC | EHRbase default + one tuned variant | report both; do not hobble the JVM with a tiny heap |
| Logging | equal, minimal (warn) | logging at debug skews everything |
| PG `shared_buffers`/`work_mem` | equal, documented | same DB tuning both sides |
| Template cache | on both (`moka` / Caffeine) | equal |

### 3.5 Storage-footprint measurement

At each scale rung, `pg_total_relation_size` of the schema (both servers) —
bytes per composition stored. Our decomposed-node model vs. EHRbase's
row-per-locatable is a legitimate, honest storage comparison (both decompose;
neither is a single-JSONB strawman).

## 4. Protocol (how a run executes)

### 4.1 Pre-flight conformance gate
Before any timing, the harness runs a **correctness check**: every workload
scenario is executed once against each server and the response asserted (status
+ payload shape via the conformance `assert` layer). A server that answers a
scenario incorrectly is **excluded from that scenario's timing** with a loud
note — we never publish "X is faster at W9" if X returns the wrong result for
W9. This ties the benchmark to the conformance suite so we cannot accidentally
time an error path.

### 4.2 Warmup
Per scenario, per server: run the operation until latency stabilises (a rolling
p50 within 5% over a 30 s window, or a fixed 60 s / 5k-iteration floor,
whichever is longer). **Warmup samples are discarded.** This is the single most
important fairness step for the JVM — and we apply the identical rule to
ehrbase-rs (page cache, connection pool fill, PG plan cache) so it is symmetric,
not a JVM handicap.

### 4.3 Measure
The measurement window: fixed iteration count (latency profile) or fixed
duration (throughput profile). Latencies recorded in an **HdrHistogram**
(coordinated-omission-corrected — the harness records intended vs. actual send
time so a stalled server cannot hide its tail). Resource stats (CPU%, RSS)
sampled from cgroup/`docker stats` at 1 Hz throughout.

### 4.4 Repeat + cooldown
≥5 runs per (scenario × server × scale × PG-config). Between runs: restart both
containers to a clean state, re-seed deterministically, cooldown to idle. Report
**median across runs** with the inter-run coefficient of variation; if CoV >
~10% the result is flagged "high variance" and the run count increased.

### 4.5 Order randomisation
Scenario and server order randomised across runs (seeded, recorded) so
thermal/cache drift doesn't systematically favour whichever went first.

## 5. Implementation — a Rust harness, not a shell script

A new workspace crate **`tools/benchmark`** (a binary; never a dependency
of the server), reusing the conformance infrastructure so the client path is
identical to what we already trust:

```
tools/benchmark/src/
├── main.rs        # clap CLI: bench run / report / seed
├── workload.rs    # the W1..W13 scenarios (pre-registered), from CNF fixtures
├── driver.rs      # closed/open-loop drivers over the conformance SutClient
├── seed.rs        # deterministic scale-ladder seeding (empty/10k/100k/1M)
├── measure.rs     # HdrHistogram + coordinated-omission correction
├── stats.rs       # per-scenario sampler for CPU/RSS (cgroup) + pg size
├── target.rs      # Target = { EhrbaseRs | EhrbaseJava }, base_url + auth + PG handle
└── report.rs      # results.json + REPORT.md + CDF/throughput plot data + workload.lock
```

Dependencies: `reqwest` (via the conformance client), `hdrhistogram`, `clap`,
`serde`/`serde_json`, `jiff`, `tokio`, `sysinfo` or direct cgroup reads. The
scale-seeder reuses the conformance fixtures + `SutClient` — the same code that
drives conformance drives the benchmark, so "the client is fair" is provable,
not asserted.

**Why our own harness over k6/wrk/vegeta:** (1) coordinated-omission handling is
non-negotiable and most tools get it wrong; (2) openEHR payloads + AQL + auth +
the fixture corpus are already modelled in our conformance crate — reuse beats
re-encoding scenarios in a foreign DSL; (3) byte-identical requests to both
targets is guaranteed by construction. (We *cross-check* a subset with `oha` or
`k6` as an independent sanity control — if our harness and an off-the-shelf tool
disagree materially on the same scenario, we investigate before publishing.)

## 6. The comparison environment

`docker/benchmark/` — a compose setup standing up, one at a time:

- **ehrbase-rs stack:** `ghcr.io/rubentalstra/ehrbase-rs` + `…-postgres` (our
  published images, by digest — the exact artifact users run).
- **EHRbase Java stack:** official `ehrbase/ehrbase:<pinned>` +
  `ehrbase/ehrbase-v2-postgres:<pinned>` (their published images — the exact
  artifact *their* users run; no hand-built "crippled" variant).

Both pinned by digest in the report. Same templates uploaded to both before
seeding. The harness targets whichever is up via `--target`.

## 7. The report (generated, honest, reproducible)

`docs/benchmarks/REPORT.md` + `results.json` + plot data, **generated** from the
run — never hand-typed. Contents:

1. **Environment block:** host CPU/RAM/kernel, both image digests, PG versions,
   pinned resources, harness commit SHA, `workload.lock` hash, run date, run
   count.
2. **Per-scenario table:** for W1–W13 × scale × PG-config — p50/p90/p99/p99.9/max
   latency, throughput at the knee, error rate, for **both** servers side by
   side, with the inter-run CoV. Winner marked only where the difference exceeds
   the noise band.
3. **Latency CDFs** and **throughput-vs-concurrency curves** (the plot data;
   rendered via the repo's dataviz conventions).
4. **Resource efficiency:** req/s per CPU-core, req/s per GB-RAM, idle + loaded
   RSS, container image size, cold-start time.
5. **Storage footprint:** bytes/composition at each scale rung.
6. **"Where EHRbase wins":** mandatory. Every scenario/scale where the reference
   implementation is faster or lighter, stated plainly with the number.
7. **Methodology limitations:** the PG-version confound residual, JVM-tuning
   sensitivity, single-host caveat, workload representativeness, and anything the
   run could not control. A benchmark that claims no limitations is lying.
8. **Reproduce-it block:** the exact commands. A reader with Docker + the repo
   reruns the whole thing.

### 7.1 What a headline claim may say
Only claims of this shape are permitted, each traceable to a table cell:

> "On the controlled PG16 run, ehrbase-rs served the mixed 70/30 workload (W13)
> at a median of X req/s at a p99 of Y ms on 4 cores, vs. EHRbase's Z req/s at W
> ms p99 — a K× throughput difference; at 1M compositions the AQL CONTAINS
> scenario (W9) p99 was A ms vs. B ms. EHRbase was faster at W12 template
> listing (C vs. D ms). Full data and reproduction: [link]."

Numbers with provenance and both directions. Never "ehrbase-rs is faster."

## 8. CI and cadence

- **Not** on every PR (too slow, too noisy on shared runners — CI runners are
  the *wrong* environment for absolute numbers).
- A **manual / scheduled** workflow (`workflow_dispatch` + monthly) on a
  dedicated/consistent runner or a documented bare-metal box, publishing the
  generated `docs/benchmarks/REPORT.md` as an artifact and (on a tagged release)
  committing it. Each report carries its environment block so cross-run
  comparison is honest.
- A lightweight **regression micro-bench** *can* run in CI against ehrbase-rs
  alone (self-hosted, not vs. Java) to catch our own perf regressions between
  releases — separate from the comparative benchmark, clearly labelled, never
  conflated with a "vs. EHRbase" claim.

## 9. Implementation plan

1. `tools/benchmark` scaffold: CLI, `Target`, the `SutClient` reuse, the
   deterministic seeder (empty→1M), HdrHistogram measurement with
   coordinated-omission correction. Prove it against the self-hosted ehrbase-rs.
2. Workload W1–W13 from the CNF fixtures; the pre-flight conformance gate;
   `workload.lock`. Freeze the workload.
3. `docker/benchmark/` dual-stack compose + the pinned EHRbase Java images; the
   config-parity table realised as compose env.
4. The two PG-config runs (recommended + controlled); ≥5 runs each; the
   report generator (`results.json` → `REPORT.md` + plot data).
5. First published report to `docs/benchmarks/` with the full environment block,
   the "where EHRbase wins" section, and the reproduce-it commands.

Discipline throughout: pre-register before measuring; publish raw data;
report both directions; state every limitation. If we cannot defend a number to
someone running EHRbase in production, it does not ship.
