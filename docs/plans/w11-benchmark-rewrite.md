# W-11 — `tools/benchmark` complete rewrite: the hospital-day stress instrument

- Status: in-progress
- Started: 2026-07-13   Owner: Ruben (session: orchestrator)
- Owner directive (2026-07-13, PRIO 1 after W-10): complete rewrite of
  `tools/benchmark` — "test the performance with real data, stress test it
  for real: templates, generating, reads and writes and updating and
  versions and everything that happens during the day when the system runs
  inside a hospital. CPU and RAM and also latency, but also everything in
  between that makes sense."
- Absorbs the benchmark half of X1 (`docs/plans/x1-comparison.md` §benchmark;
  the ECC half shipped with W-10). The methodology foundation
  (`docs/design/benchmarking.md`, 2026-07-07) **stands** — fairness controls,
  coordinated-omission correction, pre-registration, warmup, ≥5 runs,
  "where EHRbase wins" — and this rewrite replaces its abstract W1–W13
  operation list with a modelled clinical workload.

## Mission

1. **A workload model that simulates a hospital day, not an operation
   list.** A ward of patients and staff with a daily clinical rhythm:
   admissions, shift observations (vitals), medication rounds, lab results
   arriving as contributions, clinician chart reviews (latest reads +
   version history + AQL dashboards), corrections producing new versions,
   discharges. Real templates (vendored corpus OPTs), generated realistic
   instance data (varying values/subjects/times, deterministic seed).
2. **Full-fidelity measurement:** latency percentiles per operation class
   (p50/p90/p99/p99.9/max, HdrHistogram, coordinated-omission-corrected),
   sustained throughput + the saturation knee, error rate, **CPU% and RSS
   of both the app and DB containers** sampled ~1 Hz, storage footprint
   (bytes/composition) per scale rung, cold-start + idle baselines.
3. **Multi-SUT like the ECC runner:** ehrbase-rs compose (default), upstream
   EHRbase Java (fairness-governed comparison DATA), BYO endpoint by URL.
   One client code path (the conformance transport) for every SUT.
4. **Generated, honest outputs:** `docs/benchmarks/<sut>/results.json` +
   `REPORT.md` (+ comparison page input), environment block, raw histogram
   data, mandatory "where the other side wins" and limitations sections —
   measured numbers only (owner hard rule).

## Constraints

- Methodology invariants from `docs/design/benchmarking.md` §0/§3/§4 are
  law: identical client, config-parity table, warmup discarded
  symmetrically, ≥5 runs + CoV, pre-registered workload (`workload.lock`),
  raw data published, no cherry-picking.
- Reuse the conformance crate's transport/fixtures (`conformance::harness`,
  `conformance::testdata`) — the client is provably identical to the
  trusted ECC path. Never a dependency of the app.
- Pinned crates only (`hdrhistogram`, `clap`, `serde`, `jiff`, `tokio`,
  `sysinfo`/cgroup reads via docker stats). Official CLIs. Max 2 concurrent
  workers. Files ≤ ~700 lines.
- Gates: `cargo clippy -p benchmark --all-targets` + `cargo nextest run -p
  benchmark`; a smoke run against the composed ehrbase-rs stack must
  complete end-to-end before close.

## The hospital-day workload model (design summary)

**Actors on a ward of `P` patients over a simulated day** (rates scaled by
a load factor so a "day" compresses into a benchmark window):

| Clinical event | CDR operations | Default weight |
|---|---|---|
| Admission | EHR create + EHR_STATUS read + first assessment composition | low |
| Shift observations (vitals q4h) | composition create (small, hot path) | **high** |
| Medication administration | composition create + occasional update (missed dose correction → new version) | high |
| Lab results arriving | CONTRIBUTION commit (multi-version batch) | medium |
| Chart review (ward round) | composition get-latest ×N + versioned read + AQL patient dashboard (CONTAINS + ORDER BY) | **high — reads dominate** |
| Care-plan / directory use | FOLDER read, occasional directory update version | low |
| Corrections | composition update (new version) + version-history read | medium |
| Ward dashboard / reporting | AQL aggregation across the ward population | medium |
| Discharge | status update + discharge-summary composition (large) | low |
| Provisioning (rare) | OPT upload/list | rare |

Read:write ≈ 70:30 overall (capacity-planning blend, benchmarking.md W13
lineage). Templates: vendored corpus OPTs (vitals/minimal/nested/all-types)
+ generated instance data with per-patient variation (deterministic seed).
Profiles: `smoke` (CI, minutes), `hour` (steady-state hour), `day`
(full diurnal curve with morning-round and shift-change peaks). Scale
ladder: empty / 10k / 100k / 1M compositions, seeded deterministically.

## Tasks

### A — Design register
- [x] A1. Plan file (this) + workload-model register
      `docs/design/benchmark/00-workload-model.md`: the clinical event
      table above made exact — per-event operation sequences, payload
      sources, arrival-rate model (diurnal curve), read/write budget,
      scale ladder, profile definitions, `workload.lock` coverage.
- [x] A2. Measurement register `docs/design/benchmark/01-measurement.md`:
      metric set (latency classes, throughput, CPU/RSS sampling via the
      Docker stats API, storage footprint, cold-start), sampling
      architecture, results.json schema, report layout, multi-SUT seam
      (reuse `conformance::sut`), CLI surface, CI binding (smoke profile).

### B — Rewrite
- [x] B1. Fresh authoring of the crate per A1/A2 (workers ≤2, disjoint
      files; orchestrator owns lib.rs + the workload model core).
- [x] B2. ONE fix pass → crate clippy-zero, `cargo nextest run -p
      benchmark` green, zero TODOs. *Closed 2026-07-13: 0 warnings, 57/57,
      fmt clean.*
- [x] B3. **Official CKM template pack** (owner directive 2026-07-13:
      "fetch from the CKM online the needed archetypes — the official
      ones — as many as we need"): 5 official openEHR CKM templates
      vendored as CKM's own OPT exports with provenance
      (`tools/benchmark/templates/ckm/`, `scripts/vendor-ckm-templates.sh`
      — vital-signs, generic-lab-test-result, medication-order,
      international-patient-summary [1.9 MB deep-stress payload],
      clinical-synopsis). Wire into the workload: upload at provisioning;
      composition skeletons obtained from the SUT's own
      `GET /definition/template/adl1.4/{id}/example` (so any CKM template
      is usable without hand-authored fixtures), then seeded variation as
      register 00 §4. Templates a SUT rejects at upload are excluded
      loudly for that SUT (fairness note in the report), never silently.

### C — Runs
- [x] C1. Smoke profile end-to-end vs composed ehrbase-rs (empty rung):
      artefacts under `docs/benchmarks/ehrbase-rs/`. *Closed 2026-07-13:
      331 requests · 0 errors; the smoke pass caught two real instrument
      defects first (weak-ETag uid parsing; FOLDER payload missing the
      RM-mandatory archetype_node_id) — both fixed spec-cited.*
- [ ] C2. `hour` profile vs ehrbase-rs at the 10k rung — the first real
      committed baseline (latency + CPU/RSS + storage numbers).
- [x] C3. Upstream EHRbase Java runs the byte-identical workload cleanly —
      **331/331 requests · 0 errors on BOTH SUTs** (smoke @ 10k preview) —
      after four spec-verified driver/renderer tolerances (weak ETag parse,
      FOLDER archetype_node_id, 204 create-success on seed + contribution)
      and two upstream findings recorded (ORDER-BY-must-be-projected
      "Not implemented"; 204-without-representation). COMPARISON.md +
      grouped charts generated; smoke numbers are a preview — the hour
      profile + ladder + ≥5-run protocol is the X1 publication step.

### O — Overnight autonomous run (owner directive 2026-07-13 23:xx, asleep;
### full autonomy granted: keep testing/benchmarking; amazing-but-honest SVGs
### on the root README; measured numbers only)
- [x] O1. W-12 backpressure fix lands (worker in flight) → gates → commit →
      rebuild image → **full ECC run** (server changed; zero-drift gate; the
      shed layer must not disturb conformance — it only bites >256
      concurrent) → commit ratcheted baseline if changed.
- [ ] O2. Rerun BOTH knee ladders (pool parity 50 + shedding active;
      steps 1,4,16,64,128,256,512,1024) → commit knee artefacts; our server
      must now shed (503s → error-rate breach) instead of dying — if it
      still dies, record the finding loudly and continue (W-12 iterates
      later; never hide it).
- [x] O3. C2/C3 proper: `hour` profile @ 10k, ehrbase-rs then ehrbase-java
      (sequential, quiet host) → commit both results + regenerate
      COMPARISON.md with hour data + knees.
- [~] O4. (deferred in favour of completing O5/O6 before morning; the 100k/1M ladder is the X1 publication step) If the night allows: 100k rung seed + hour, both SUTs → commit.
- [x] O5. Root README: a "Measured against EHRbase (Java)" section embedding
      committed comparison SVGs (relative paths; the charts live in
      docs/benchmarks/**) with the honest caveats line (single-run,
      same-host preview; link to COMPARISON.md + the methodology) — the
      wording must survive the no-false-claims rule: charts show measured
      data, both directions, never an unqualified "faster than Java".
- [ ] O6. D-phase docs: book benchmark page, blueprint touch-ups,
      PROGRESS/WORKLIST rows, changelog verify → push; PR opened (merge
      when CI green, per the standing autonomous flow).

### D — Close
- [ ] D1. Workspace gates; scripts/benchmark.sh entry point; CI smoke job.
- [ ] D2. Docs: `docs/design/benchmarking.md` updated (workload section
      superseded by the register), book page addition, changelog,
      WORKLIST/PROGRESS/blueprint touch-ups; PR + merge.

## Real defect found by the CKM pack (triaged 2026-07-13, 4-way check)

The server's own `/example` for the official CKM **International Patient
Summary** OPT is rejected by the server's own validator (422: "unexpected
node 'at0001' under 'description'" at the Medication-Summary
`ACTION.medication`). Four-way triage, never assumed:

1. **Spec** (spec-researcher, verbatim citations): an *unconstrained*
   attribute admits anything RM-valid (ADL 1.4 `master05-cadl.adoc` §"Any"
   Constraints; CNF `master15-content_tc_composition.adoc` L38); a
   *constrained* one is governed by the AOM 1.4 `valid_value` cascade;
   RM `ACTION.description` is existence 1..1. CNF has **no** case
   committing a server-generated example — spec silence, flagged.
2. **The OPT** (parsed XML, both ACTION.medication occurrences): it DOES
   constrain `description` to `ITEM_TREE[at0017]` (+ `protocol
   ITEM_TREE[at0030]`, four ISM_TRANSITION alternatives).
3. **Validator**: `check_closure` rejects `at0001` where only `at0017` is
   admissible — **spec-correct** (and its capture rule already leaves
   wholly-unconstrained attributes open, matching 1).
4. **Generator** — **the defect**: `openehr-flat` `from_flat`'s per-ENTRY
   mandatory-structure synthesis stamps a blind `ITEM_TREE[at0001]`
   placeholder for missing structural attributes even when the template
   constrains them. Fix in flight: the WebTemplate captures structural
   stubs (attr → rm_type/node_id/term-definition name) and the synthesis
   consults them first; the `at0001` placeholder survives only for
   genuinely unconstrained attributes (where it is spec-legal per 1).
   Plus a new ECC TPL case ("ADL 1.4 example round-trip", the IPS OPT
   vendored into the ECC owned-fixture set) so the defect class stays
   probed — owner directive.

## Measured P20 fuel (knee ladders, 2026-07-14 overnight — single-host preview)

With pool parity 50, shedding 256, signing off (parity): **our knee = L16
(161 req/s, p99 34 ms); L64 breaches (p99 2.3 s, 4.2% errors incl. shed
503s). Upstream rode L64 clean (643 req/s, p99 22 ms) and breached at L128.**
The server survives (no OOM — W-12 works); the gap is engine-side write-path
cost under concurrency + the AQL dashboard plan (server log at load: node/
vo_version INSERTs 1.6–2.2 s, the patient AQL 2 s+ at the 10k rung, every
statement slowed → PG-level contention). Published as measured (the
comparison's "where the other side wins"); the optimization work is P20's,
with this data as its opening evidence. Candidates recorded there: AQL-path
indexes for the ORDER-BY/CONTAINS dashboard shape; per-commit round-trip
reduction; temporal-GiST maintenance cost under write concurrency.

## Decisions made this phase

- **Read:write budget counts a CONTRIBUTION as its N committed
  compositions** (a batch commit of N compositions is N clinical writes —
  register 00 §2 capacity lineage); measured blend ≈73% read, inside
  70±5. The literal per-HTTP-op blend is ~76% read; E5's rate is the
  lever if the op-count blend ever becomes the law.
- **E1 admission extended** beyond the register's literal sequence: it
  also seeds an initial vitals composition and establishes the directory,
  so E5/E6/E7 always have pre-existing state (per-patient ordering
  invariant holds structurally; PORT NOTE in `model/event.rs`).
- **`TemplateKind::Vitals` renders `composition_evaluation_test`** (a
  proven both-server event composition) until the CKM `vital-signs`
  template lands at B3 (PORT NOTE in `render.rs`).
- **Driver tolerances:** `dir-read` accepts 404 and `opt-upload` accepts
  409/204 as measured successes (open-loop schedules cannot guarantee a
  prior directory write; provisioning is idempotent — documented in
  `drive.rs`); `status-update` performs one unmeasured GET to build a
  correct If-Match when the uid is not cached (identical cost every SUT).
- **`workload.lock`** hashes an ordered, extensible template-source list —
  the B3 CKM pack shifts the lock without an API change.
