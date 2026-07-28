# `cnf-runner` — the CNF 2.0 reference runner (tooling, not part of the app)

The ground-up rebuild of the conformance instrument (#202, W1–W7): a
data-driven interpreter over the CNF 2.0 machine-readable schedule. The
schedule is the ONLY DSL — no cucumber, no Hurl, no second test-definition
language, ever.

## The canonical CLI (use EXACTLY these forms — never improvise flags)

| Command | Purpose |
|---|---|
| `bash scripts/conformance.sh` | THE pipeline (compose fresh → run → verdicts → badges). Env: `CONF_SUT=ehrbase-rs\|ehrbase-java\|byo`, `CONF_PERF_CLASS=POC\|S\|L\|R` (adds the measured stage), `CONF_PERF_HOURS=1\|2\|4\|6\|8\|12`, `CONF_NO_COMPOSE=1`, `SKIP_BUILD=1` |
| `cargo run -p cnf-runner -- validate --root tools/cnf-runner/artifacts --specs docs/specs/openehr` | every machine gate over the artifact tree (zero findings = green) |
| `cargo run -p cnf-runner -- run --root tools/cnf-runner/artifacts --ixit <party>/ixit.json --out <dir> --sut-name N --sut-version V --statement <party>/statement.json [--filter SUBSTR]` | execute the functional catalogue against a live SUT |
| `cargo run -p cnf-runner -- verdicts --statement F --results F --root tools/cnf-runner/artifacts --out <dir>` | the pure verdict pipeline + report/statement/certificate |
| `cargo run -p cnf-runner -- perf --root tools/cnf-runner/artifacts --ixit F --results F --class POC\|S\|L\|R [--hours 1\|2\|4\|6\|8\|12]` | the measured class run (conformance-by-measurement; merges into results.json) |
| `cargo run -p cnf-runner -- stress --root tools/cnf-runner/artifacts --ixit F --out stress.json [--corpus-class POC] [--step-secs 120] [--bisections 3] [--max-rate 4096]` | the step-load stress ladder → maximum sustainable throughput (exploration only; NEVER touches results.json) |
| `cargo run -p cnf-runner -- aql-probe --root tools/cnf-runner/artifacts --ixit F --out aql-probe.json [--corpus-class POC] [--requests 20]` | the seeded-corpus AQL optimization probe: wire percentiles + pg_stat_statements attribution (exploration only; NEVER touches results.json) |
| `cargo run -p cnf-runner -- stress-compare --left F --left-label S --right F --right-label S --out F.svg` | the cross-SUT stress overlay FROM two committed stress.json reports (driven by scripts/render-comparison.sh) |
| `bash scripts/render-perf-assets.sh` (env `CONF_SUT`) | regenerate the published SVGs + summary FROM committed artifacts (CI diffs them) |
| `bash scripts/render-conformance-assets.sh` (env `CONF_SUT`) | regenerate the conformance visuals (capability heat grid + per-chapter outcome bars) FROM committed verdicts/results (CI diffs them) |
| `cargo run -p cnf-runner -- emit-schemas --out tools/cnf-runner/schemas` | regenerate the published JSON Schemas after a schema.rs change (drift-tested) |

Every instrument seeds a freshly composed, empty SUT and the stack is torn
down afterwards — there is no seed reuse (the `--skip-seed`/sidecar scheme
is retired). Credentials for direct `run`/`perf`/`stress`/`aql-probe`
invocations come from the env
the ixit references: `SUT_USER/SUT_PASS` (+ `SUT_ADMIN_*`, `SUT_RO_*`) —
the dev-compose defaults are exported by `scripts/conformance.sh`.

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
  operation bindings, each mapping cited to its source under the oracle
  order (owner rulings 2026-07-24 + 2026-07-28): the ITS-REST docs text
  first and on every conflict; the released OAS only for behaviour the docs
  text is silent on, cited AS the OAS.
- **Expectations trace to the released spec** (`docs/specs/openehr/CNF/`,
  `QUERY`, `ITS-REST` docs text; the released OAS fills docs-text silence
  per the 2026-07-28 ruling and loses every conflict) — never to observed
  SUT behaviour; EHRbase and the retired ECC
  harness (its final catalogue in git history; retired 2026-07-22) are
  prior art, not oracles. Spec silences go through the ambiguity register
  with a typed `disposition`, never private resolution.
- **Coverage is a mandate, not just pass rate** — the catalogue must exercise
  EVERY wire behaviour the spec defines (every operation, status-code branch,
  required/conditional header, negotiation variant, precondition + error
  family, and RM/AQL behaviour), each as its own small isolated case; a
  spec-defined behaviour with no case is a gap to close or an honest boundary
  to register, never a silent omission (`.claude/rules/testing.md` §CNF
  coverage). This is MACHINE-ENFORCED by the `surface-coverage` gate
  (`validate.rs`, issue #271): it enumerates the wire surface from the RELEASED
  sources only (the SM platform interfaces × their ITS-REST-docs branches —
  never the OAS) and fails on any SM operation, realized-binding
  outcome/format branch, or cross-cutting behaviour with neither a covering
  case nor a cited `artifacts/vocab/wire_surface.yaml` exception; `validate
  --specs` refreshes `docs/conformance/coverage-report.md`. Two halves of the
  domain are not authored but DERIVED, so nothing can hide by never being
  written down: a RELEASED ITS-REST operation the SM models no interface for
  is enumerated from the pinned `NON_SM_REST_OPERATIONS` table under a
  reserved `I_ITS_REST_*` pseudo-interface (a catalogue naming convention,
  never an SM claim — register AMB-161; an unpinned use of the prefix is a
  finding), and the Axis-3 domain is parsed from the `#`/`##` sections of the
  two released overview chapters — every section must be named by an authored
  `elements`/`branches` source or pinned in `AXIS3_SECTION_EXCLUSIONS` with
  its citation.
- **Verdicts are computed, never asserted** — pure functions of
  (statement, results, catalogue, capability matrix).
- **The measurement machinery is conformance-by-measurement** (`perf.rs`,
  the `perf_run/` modules — client/pack/corpus/schedule/execute/window —,
  `perf_assets.rs`, the `perf`/`perf-assets` subcommands): OPEN-LOOP
  offered load only (a deterministic seeded arrival schedule; latency from
  the PLANNED arrival instant so coordinated omission cannot hide stalls);
  every measurement embeds its base64 HDR V2 histograms + the ixit
  environment block; class verdicts (earned | not-earned) re-derive from
  the DECODED histograms in the verdict pipeline — the stored verdict and
  summary percentiles are tamper-checked, never trusted. **The workload is
  the hospital simulation**: the class cases name journey shares
  decomposed through `artifacts/vocab/journey_catalogue.yaml` (the closed
  `PerfOp` vocabulary; every stage its own planned arrival; dependent
  stages never block — an unlanded prerequisite records as an error; the
  `journey-envelope` validator gate reconciles every workload's expanded
  write share into the population-anchored 10:1..50:1 band). Journey
  payloads = the CKM template pack (`artifacts/corpus/templates/ckm/`,
  COMPOSITION-rooted only, provenance in its PROVENANCE.md; example
  skeletons regenerate via `scripts/generate-ckm-examples.sh` against a
  running SUT). The scale corpora + the standing ward seed strictly
  through the public API per `artifacts/corpus/recipes/scale_ladder.md`;
  published SVGs/summary tables render FROM committed results.json
  (`scripts/render-perf-assets.sh`, CI regenerate-and-diff guarded). The
  sustained window only extends (`--hours 1|2|4|6|8|12`; the case's
  normative hour is the floor; the diurnal arrival curve is valid only for
  the >= 8 h holds) — no shortened run exists, so nothing sub-normative
  can ever look like a measured record.
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
