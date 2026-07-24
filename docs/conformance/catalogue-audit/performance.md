# Catalogue audit — PERFORMANCE chapter

Issue #231 · audited 2026-07-24 · 4 cases · verdicts: 4 ok / 0 defects / 0 ambiguities

The vendored CNF component carries no performance test-schedule chapter (the
2017 schedule lineage is referenced from the guide overview only), so this
chapter is an authored proposal by design; every case carries the explicit
authoring flag in `spec_refs` ("this proposal; 2017 schedule lineage") and the
normative internal ground is the permanent design record
`docs/conformance/cnf-design.md` §8.14 — deliberately NOT cited from the YAML
per the durable-citation hard rule (internal markdown is never a `spec_refs`
target; the proposal flag is the adjudication).

| case | verdict | evidence | resolution |
|---|---|---|---|
| PERF-hospital_sim-class_POC | ok | Floor 2/s, corpus `cnf.scale.10k`, warmup PT5M, duration PT1H, thresholds {p99 ≤ 1000 ms on every operation, error_rate 0, offered ≥ 2} all match the §8.14 class table + threshold template (cnf-design.md:1729–1732); journey shares (18 journeys) sum to 100.0% and every journey resolves in `vocab/journey_catalogue.yaml`; the committed measured record (`docs/conformance/ehrbase-rs/results.json` measurements: offered 2.033, warmup_s 300, duration_s 3600) was produced from exactly this case | none |
| PERF-hospital_sim-class_S | ok | Floor 15/s, corpus `cnf.scale.100k` — matches §8.14 (POC 2 · S 15 · L 150 · R 1,500) and the `scale_ladder.md` corpus contract; identical journey table and thresholds | none |
| PERF-hospital_sim-class_L | ok | Floor 150/s, corpus `cnf.scale.1m`; same structure verified | none |
| PERF-hospital_sim-class_R | ok | Floor 1500/s, corpus `cnf.scale.10m`; same structure verified | none |

Checks common to all four:
- **Ground (dim 1):** authored-proposal adjudication present in `spec_refs`; no official schedule row exists to diverge from.
- **Expectations (dim 2):** thresholds derive from §8.14 (population-anchored floors, p99 ≤ 1 s SLO, zero error budget for the measured window — distinct from the stress instrument's 0.1% exploration budget); the open-loop/coordinated-omission-free requirement is stated in the case preamble and implemented by the driver, not asserted per-row.
- **Citations (dim 3):** proposal flag, see header note.
- **Fixtures (dim 4):** all four `cnf.scale.*` corpus keys are defined in `tools/cnf-runner/artifacts/corpus/recipes/scale_ladder.md` (10k/100k/1m/10m EHR counts).
- **Captures (dim 5):** performance cases bind no step captures; the journey decomposition resolves through `vocab/journey_catalogue.yaml` (all 18 journeys present; validator enforces the 10:1..50:1 write-share derivation band).
- **Ambiguity tags (dim 6):** none carried; none needed.

Related adjudication landed separately (runner machinery, not the catalogue): the measured-window driver accepts the spec-legal `201|204` minimal-return created family (ITS-REST overview `Requests_and_responses.md` §Prefer) since PR #267 — no case YAML change was implied.
