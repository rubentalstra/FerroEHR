# Benchmark register 00 — the hospital-day workload model

Status: authored 2026-07-13 (W-11 A1). The pre-registered workload of the
rewritten `tools/benchmark`. This register IS the workload definition: the
crate implements this table, and `workload.lock` hashes this model (event
catalogue + rates + template set + generator seed), frozen before the first
measured run (`docs/design/benchmarking.md` §2 discipline — that document's
fairness/methodology sections remain law; its W1–W13 list is superseded by
this model).

## 1. Simulation shape

A **ward** of `P` admitted patients driven through a simulated clinical day
by a deterministic event generator (seeded; same seed → byte-identical
request sequence):

- Every patient is one **EHR**; admission state machine:
  `admitted → on-ward → discharged` (a day turns over ~10% of the ward).
- Staff are implicit: event *rates* model nurses/physicians/lab systems.
- The generator produces an **open-loop arrival schedule** (event class,
  patient, planned send time) ahead of the run — coordinated-omission
  correction then measures against *planned* time, so a slow SUT cannot
  slow the workload down and hide its tail.
- No think-time model beyond arrival rates: the CDR sees request arrival,
  not user typing.

## 2. Event catalogue (the clinical day → CDR operations)

Payloads come from the vendored corpus OPTs + owned corrected fixtures
(the same set the ECC suite provisions) with **generated instance data**
(§4). Operation classes in `[brackets]` key the latency histograms
(register 01 §1).

| # | Clinical event | Operation sequence | Per patient-day |
|---|---|---|---|
| E1 | Admission | `POST /ehr` (EHR_STATUS with subject id) `[ehr-create]` → `GET /ehr/{id}` `[ehr-read]` → admission assessment composition (nested template) `[comp-create-large]` | ward turnover only (~0.1) |
| E2 | Shift vitals (q4h) | `POST …/composition` small vitals `[comp-create-small]` | 6 |
| E3 | Medication round | `POST …/composition` medication `[comp-create-small]`; 5% → correction `PUT` `[comp-update]` | 4 |
| E4 | Lab results arrive | `POST …/contribution` batch of 1–3 result compositions `[contribution-commit]` | 2 |
| E5 | Chart review (round) | 3× `GET …/composition/{object}` latest `[comp-read-latest]` → 1× `GET …/composition/{ovid}` `[comp-read-version]` → patient-scoped AQL (CONTAINS OBSERVATION + WHERE ehr_id + ORDER BY time) `[aql-patient]` | 8 |
| E6 | Care-plan / directory | `GET /ehr/{id}/directory` `[dir-read]`; 10% → directory update `PUT` `[dir-update]` | 2 |
| E7 | Documentation correction | `PUT …/composition/{object}` new version `[comp-update]` → `GET …/versioned_composition/{uid}/revision_history` `[history-read]` | 1 |
| E8 | Ward dashboard / reporting | population AQL (aggregation / ORDER BY across the ward, no ehr_id filter) `[aql-ward]` | 0.5 |
| E9 | Discharge | discharge-summary composition (large) `[comp-create-large]` → `PUT …/ehr_status` `[status-update]` | ward turnover (~0.1) |
| E10 | Provisioning | OPT upload + template list `[opt-upload]`, `[tpl-list]` | rare: fixed 2/run, outside the measured mix |

Resulting op blend (E1–E9 weighted): **≈70% read / 30% write** — the
capacity-planning number (benchmarking.md §2.1 W13 lineage). The exact
per-class expected counts for a given `(P, profile)` are printed into the
report and covered by `workload.lock`.

## 3. Profiles (time compression)

A real patient-day at ward size `P` compresses into a benchmark window by
scaling arrival rates; the *mix* never changes, only the clock:

| Profile | Window | Shape | Purpose |
|---|---|---|---|
| `smoke` | ~2 min | fixed small event count, steady rates | CI + harness self-test; not a publishable number |
| `hour` | 60 min | steady state at daily-mean rates × load factor `L` | the standard measured run; `L` ramps for the throughput/knee series |
| `day` | configurable (default 60 min compressed day) | diurnal curve: morning-round peak (~08:00), afternoon peak (~14:00), shift-change bumps (~07:00/15:00/23:00), night trough | the realism profile — tail latency under load swings |

The knee/saturation series (register 01 §3) reuses `hour` at increasing
`L` until the SLO (p99 > 1 s or error rate > 0.1%) breaks.

## 4. Data generation (real templates, generated instances)

- **Templates:** two packs, provisioned once per run, identically on every
  SUT: (a) the vendored corpus OPTs already exercised by ECC —
  vitals/minimal (small), nested (large/deep), persistent (directory,
  care plan); (b) the **official CKM pack** (owner directive 2026-07-13):
  five templates vendored as CKM's own OPT exports with provenance
  (`tools/benchmark/templates/ckm/` — vital-signs, generic lab result,
  medication order, International Patient Summary [1.9 MB], clinical
  synopsis). CKM-pack composition skeletons come from the SUT's own
  `GET /definition/template/adl1.4/{id}/example`, so any CKM template is
  usable without hand-authored fixtures; a template a SUT rejects at
  upload is excluded loudly for that SUT (fairness note in the report).
- **Instances:** per-event canonical-JSON compositions produced from the
  fixture skeletons with seeded variation: numeric leaf values within the
  template ranges, event `time` advancing along the simulated day,
  per-patient `subject` ids, composer rotation from a staff pool. The
  generator is deterministic (`rand` with a fixed seed recorded in the
  lock) — both SUTs receive **byte-identical** request sequences.
- No hand-tuned payloads: every skeleton is a committed fixture; variation
  touches values, never structure.

## 5. Scale ladder

Read/query performance only matters against stored volume. The day runs on
top of a pre-seeded database:

`empty` · `10k` · `100k` · `1M` compositions (deterministic seeder, same
seed both SUTs; storage footprint measured per rung — register 01 §4).

W-11 close requires `smoke`+`hour` at `empty`+`10k`; the full ladder and
the ≥5-run publication protocol are the X1 publication step.

## 6. Fairness inheritance

Everything in `docs/design/benchmarking.md` §§0/3/4 applies unchanged:
identical client (the conformance transport), config-parity table, warmup
discarded symmetrically, ≥5 runs + CoV for published numbers, order
randomisation, pre-flight correctness gate (a scenario a SUT answers
incorrectly is excluded from its timing, loudly), raw data published,
mandatory "where the other side wins" + limitations sections.
