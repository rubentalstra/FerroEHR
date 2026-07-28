# `cnf-runner` — the CNF 2.0 reference runner (tooling, not part of the app)

The ground-up rebuild of the conformance instrument (#202, W1–W7): a
data-driven interpreter over the CNF 2.0 machine-readable schedule. The
schedule is the ONLY DSL — no cucumber, no Hurl, no second test-definition
language, ever.

## The canonical CLI (use EXACTLY these forms — never improvise flags)

| Command | Purpose |
|---|---|
| `bash scripts/conformance.sh` | THE pipeline (compose fresh → run → verdicts → badges). For `ehrbase-rs` it composes TWO deployments of the one built image — the standard SMART/digest stack plus the openPGP-posture stack (`-p ehrbase-rs-cnf-pgp`, host port 8081) — so both claimed signing modes land in the one record. Env: `CONF_SUT=ehrbase-rs\|ehrbase-java\|byo`, `CONF_PERF_CLASS=POC\|S\|L\|R` (adds the measured stage), `CONF_PERF_HOURS=1\|2\|4\|6\|8\|12`, `CONF_NO_COMPOSE=1`, `SKIP_BUILD=1` |
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
  emitted schema. **A binding file is named after the binding it declares** —
  `<sm_operation>[-<variant>].yaml` — machine-enforced by the
  `binding-filename` gate: selection is by the declared fields, so a
  disagreeing name misleads only the reader and the grep, silently.
- **A CLAIM without cases is unrepresentable** (issue #622). `validate`
  sweeps the committed party statements beside the artifact root
  (`<root>/../party/*/statement.json`) and relates them to the catalogue, so
  a hollow claim fails before any SUT is composed: `claim-completeness` (a
  claimed capability has ≥ 1 verdict-bearing case; a capability whose cases
  all resolve excused/deselected names its `evidence_exception` register
  entry, and a stale one is a finding), `capability-depth` (every matrix row
  keeps its `min_cases` floor — floors ratchet UP only, never down to match a
  shrunken battery), `workload-coverage` (a claimed capability the measured
  hospital simulation does not exercise carries a register-linked
  `workload_exclusion`, rendered on the certificate instead of a bare
  `NO — catalogue gap`; a stale exclusion is a finding too). The matrix row's
  `realization: released-wire | extension` marks what the cases drive and is
  a certificate column; an `extension` row may never be `required`.
- **A capability with no RELEASED wire is either served or unclaimed** (issue
  #623) — never parked as excused-while-claimed. A binding may carry an
  `extension:` block (family + reason + source + register ambiguity) beside its
  full request/outcomes form, declaring that it drives one of the SUT's own
  `served_extensions` routes; such a battery EXECUTES and gates the CAPABILITY
  verdict only. The `realization-scope` gate fences it: the family and the
  request-path shape must resolve in the served_extensions axis, the
  adjudication must resolve in the register, and the matrix `realization`
  marker must match what the cases actually drive (both directions). Where
  nothing is served, the party STATEMENT drops the claim — the matrix row
  stays, because the matrix is the Profiles book as data, not a claim list.
- **Every closed vocabulary is a Rust enum/newtype** (outcome kinds,
  selectors, header matchers, capture sources, the `${…}` reference grammar,
  dispositions, sentinels): illegal states unrepresentable. New vocabulary
  values enter only by schedule release, never ad hoc.
- **A deployment fact no released operation discloses is an IXIT
  declaration, never a runner guess**: the party's `ixit.json` declares it
  (e.g. `system_id`) and a case reads it as `${ixit:<field>}` — the field set
  is closed like every other grammar. A party that declares nothing makes the
  referencing cases not-applicable with that citation, so an undeclared fact
  costs coverage, never correctness. The same law extends to whole
  **deployment postures**: `signing` declares the version-signing posture, and
  `ixit.smart` declares the SMART resource-server posture PLUS the static test
  issuer the runner mints per-step scoped Bearer tokens against (a flow step's
  `scopes:` key — empty list included — marks it SMART-lane; the CDR never
  issues tokens, ITS-REST `docs/smart_app_launch/master06-authentication.adoc`
  §Supported Authentication Flows, so the runner takes the Authorization-Server
  role for that lane only). Undeclared => not-applicable with the citation,
  never a driven guess. The SMART resource-server posture IS the standard
  ehrbase-rs conformance posture (owner ruling 2026-07-28): the pipeline
  always overlays `docker/sut-smart.yml`, the ixit's principals mint scoped
  Bearer tokens (per-instance standing grants; a step-level `scopes:`
  overrides for the boundary cases), and the ONE committed record covers the
  whole claimed surface in one run — no focused lanes, no sidecar artifact
  directories.
- **A claimed capability is tested, and a posture the product claims BOTH
  branches of is tested in both — in the ONE record** (owner ruling
  2026-07-28). Version signing is the live case: RM common master06 §Digital
  Signature defines digest and openPGP as alternative depths of one mechanism
  and a deployment realizes exactly one, so the pipeline composes a SECOND
  deployment of the same built image in the pgp posture
  (`docker/sut-signing-pgp.yml` + `docker/sut-pgp-parallel.yml`, project
  `ehrbase-rs-cnf-pgp`, host port 8081) and the ixit declares it as the
  `sut_pgp` **instance** carrying its OWN `signing` block. `signing` is
  therefore per-instance-first with the top-level block as the party default,
  and the `-pgp` SIG-VERSION siblings address that instance with `on:`. One
  `run`, one `results.json`, no outcome merging, no environment knob (the
  retired `CONF_SIGNING_MODE`/`ixit.pgp.json` pair is gone). A case addressing
  an instance the party does not declare is not-applicable with the citation
  at SELECTION time (`run.rs`), never a drive-time transport error, and case
  preconditions provision on the deployment the flow addresses.
- **Cases speak SM + outcome kinds only** — nothing wire-level (no HTTP
  status, header, or media type) in a case core; wire lives in per-ITS
  operation bindings, each mapping cited to its source under the oracle
  order (owner rulings 2026-07-24 + 2026-07-28): the ITS-REST docs text
  first and on every conflict; the released OAS only for behaviour the docs
  text is silent on, cited AS the OAS.
- **A version floor is the SAME `applies` block at every level, and it goes
  where the SPEC puts the requirement.** One struct
  (`rm`/`base`/`am`/`aql`/`its_rest`/`term`, semver ranges), one predicate
  (`Applies::satisfied_by`, undeclared ⇒ out of scope), three legal homes:
  a CASE core when the whole behaviour is release-dated; an OPERATION binding
  (`OperationBinding.applies`, enforced at selection in `run.rs`) when the
  wire itself arrived in a later release; a HEADER expectation
  (`HeaderExpectation.applies`) when only one response RULE is dated and the
  operation is not — the live case, since the overview dates the `W/` ETag
  MUST and the read/DELETE `Location` restriction to Release 1.1.0. Never
  raise a header rule's floor to the case: that takes a party out of scope
  for behaviour it does implement. A header expectation also carries
  `optional: true` (authored bare as `present?`) when the released text makes
  PRESENCE a SHOULD while the FORM stays a MUST.
- **One request-construction path** (`exec/driver.rs::compose_headers`): a
  driven step and a case precondition build headers in the SAME function, so
  an operation can never go on the wire two ways by code path. Bindings
  declare the `Accept`/`Content-Type` they intend; provisioning passes
  `step: None` and that is the only difference a caller may express.
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
