# `cnf-runner` — the CNF 2.0 reference runner (tooling, not part of the app)

The ground-up rebuild of the conformance instrument (#202, W1–W7): a
data-driven interpreter over the CNF 2.0 machine-readable schedule. The
schedule is the ONLY DSL — no cucumber, no Hurl, no second test-definition
language, ever.

- **Artifact families are the contract** (case cores, operation bindings,
  vocabularies incl. the capability→family→tier matrix, corpus manifest,
  ambiguity register, party artifacts statement/results/ixit). The committed
  JSON Schemas under `schemas/` are the published norm — emission is
  deterministic and drift-guarded by a nextest test; never hand-edit an
  emitted schema.
- **Every closed vocabulary is a Rust enum/newtype** (outcome kinds,
  selectors, header matchers, capture sources, the `${…}` reference grammar,
  dispositions, sentinels): illegal states unrepresentable. New vocabulary
  values enter only by schedule release, never ad hoc.
- **Cases speak SM + outcome kinds only** — nothing wire-level (no HTTP
  status, header, or media type) in a case core; wire lives in per-ITS
  operation bindings, each mapping cited to its OAS source.
- **Expectations trace to spec text** (`docs/specs/openehr/CNF/`, `QUERY`,
  `ITS-REST`) — never to observed SUT behaviour; EHRbase and the retired ECC
  harness (its catalogue preserved at `comparison/ecc-catalog.tsv`) are
  prior art, not oracles. Spec silences go through the ambiguity register
  with a typed `disposition`, never private resolution.
- **Verdicts are computed, never asserted** — pure functions of
  (statement, results, catalogue, capability matrix).
- **The measurement machinery is conformance-by-measurement** (`perf.rs`,
  `perf_run.rs`, `perf_assets.rs`, the `perf`/`perf-assets` subcommands):
  OPEN-LOOP offered load only (a deterministic seeded arrival schedule;
  latency from the PLANNED arrival instant so coordinated omission cannot
  hide stalls); every measurement embeds its base64 HDR V2 histograms + the
  ixit environment block; class verdicts (earned | not-earned) re-derive
  from the DECODED histograms in the verdict pipeline — the stored verdict
  and summary percentiles are tamper-checked, never trusted. The scale
  corpora seed strictly through the public API per
  `artifacts/corpus/recipes/scale_ladder.md`; published SVGs/summary tables
  render FROM committed results.json (`scripts/render-perf-assets.sh`,
  CI regenerate-and-diff guarded). A `--smoke` run is exploratory wiring
  proof and is NEVER persisted.
- Gates: `cargo clippy -p cnf-runner --all-targets` +
  `cargo nextest run -p cnf-runner` (schema drift, pilot acceptance,
  seeded-defect rejection, the §8.13-derived cross-artifact guards).
- **Red-run triage follows the attribution law**
  (`.claude/rules/cnf-triage.md`; delegate to the `cnf-triage` agent): the
  vendored spec text is ALWAYS right and never a suspect — every red row is
  attributed to exactly one of {application, runner machinery, catalogue
  artifact} by three-way comparison (spec-required vs catalogue-expected vs
  SUT-observed), each attribution carrying the spec citation + the actual
  wire exchange. Never adjust an expectation to match observed SUT
  behaviour; never change app code without a reproduced exchange; spec
  silence goes through `artifacts/registers/ambiguities.yaml`.
