# `cnf-runner` — the CNF 2.0 reference runner (tooling, not part of the app)

The ground-up rebuild of the conformance instrument (#202, W1–W7): a
data-driven interpreter over the CNF 2.0 machine-readable schedule. The
schedule is the ONLY DSL — no cucumber, no Hurl, no second test-definition
language, ever.

## The canonical CLI (use EXACTLY these forms — never improvise flags)

| Command | Purpose |
|---|---|
| `bash scripts/conformance.sh` | THE pipeline (compose fresh → run → verdicts → badges). For `ferroehr` it composes TWO deployments of the one built image — the standard SMART/digest stack plus the openPGP-posture stack (`-p ferroehr-cnf-pgp`, host port 8081) — so both claimed signing modes land in the one record. Env: `CONF_SUT=ferroehr\|ehrbase\|byo`, `CONF_PERF_CLASS=POC\|S\|L\|R` (adds the measured stage), `CONF_PERF_HOURS=1\|2\|4\|6\|8\|12`, `CONF_NO_COMPOSE=1`, `SKIP_BUILD=1` |
| `cargo run -p cnf-runner -- validate --root tools/cnf-runner/artifacts --specs docs/specs/openehr [--write-report]` | every machine gate over the artifact tree (zero findings = green). **Read-only by default**; `--write-report` additionally refreshes `docs/conformance/coverage-report.md` (a check verb never mutates the tree unasked) |
| `cargo run -p cnf-runner -- run --root tools/cnf-runner/artifacts --ixit <party>/ixit.json --out <dir> --sut-name N --sut-version V --statement <party>/statement.json [--filter SUBSTR]` | execute the functional catalogue against a live SUT |
| `cargo run -p cnf-runner -- verdicts --statement F --results F --root tools/cnf-runner/artifacts --out <dir>` | the pure verdict pipeline + report/statement/certificate |
| `cargo run -p cnf-runner -- perf --root tools/cnf-runner/artifacts --ixit F --results F --class POC\|S\|L\|R [--hours 1\|2\|4\|6\|8\|12]` | the measured class run (conformance-by-measurement; merges into results.json) |
| `cargo run -p cnf-runner -- stress --root tools/cnf-runner/artifacts --ixit F --out stress.json [--corpus-class POC] [--step-secs 120] [--bisections 3] [--max-rate 4096]` | the step-load stress ladder → maximum sustainable throughput (exploration only; NEVER touches results.json) |
| `cargo run -p cnf-runner -- aql-probe --root tools/cnf-runner/artifacts --ixit F --out aql-probe.json [--corpus-class POC] [--requests 20]` | the seeded-corpus AQL optimization probe: wire percentiles + pg_stat_statements attribution (exploration only; NEVER touches results.json) |
| `cargo run -p cnf-runner -- stress-compare --left F --left-label S --right F --right-label S --out F.svg` | the cross-SUT stress overlay FROM two committed stress.json reports (driven by scripts/render/comparison.sh) |
| `bash scripts/render/perf-assets.sh` (env `CONF_SUT`) | regenerate the published SVGs + summary FROM committed artifacts (CI diffs them) |
| `bash scripts/render/conformance-assets.sh` (env `CONF_SUT`) | regenerate the conformance visuals (capability heat grid + per-chapter outcome bars) FROM committed verdicts/results (CI diffs them) |
| `cargo run -p cnf-runner -- emit-schemas --out tools/cnf-runner/schemas` | regenerate the published JSON Schemas after a schema.rs change (drift-tested) |

Every instrument seeds a freshly composed, empty SUT and the stack is torn
down afterwards — there is no seed reuse and no skip-seed/sidecar
mechanism. Credentials for direct `run`/`perf`/`stress`/`aql-probe`
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
- **Corpus layout: catalogue fixtures vs breadth packs.** Per-case fixtures
  (`corpus/fixtures/**`, the curated `corpus/templates/ckm/*.opt`) carry a
  `MANIFEST.yaml` entry each — verdict + defect live there, never only in a
  filename. The **breadth packs** vendored from upstream libraries
  (`corpus/templates/ckm/full/` — every OPT the official CKM publishes;
  `corpus/archetypes/ckm/adl14/` — every CKM archetype, ADL 1.4;
  `corpus/archetypes/adl2/` — upstream's ADL 2 archetypes with their 1.4
  twins) instead record inventory + adjudications in the pack's
  `PROVENANCE.md` and are driven by directory-walking gates. Every pack is
  produced by a committed `scripts/vendor/*.sh` script, vendored verbatim, and
  must be 100% exercised with adjudicated skips only —
  `.claude/rules/vendored-corpora.md` (which also holds the CKM REST
  page/size pagination trap and the ADL 1.4-vs-2 sourcing law).
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
  marker must match what the cases actually drive (both directions). Such a
  case is ALSO party-scoped at selection (`run.rs`): a party claiming none of
  its capabilities gets not-applicable-with-citation, because a route openEHR
  does not specify is an offer only the party that CLAIMS it answers for.
  Where nothing is served, the party STATEMENT drops the claim — the matrix row
  stays, because the matrix is the Profiles book as data, not a claim list.
  Corollary on the FLOW side: a case whose subject is one extension operation
  must not drive a released one just to reach its precondition, or the
  realization it evidences stops being the one it is about. Provisioning
  therefore belongs in `requires` (`server`/`templates`/`ehr`/`directory`/
  `party`/`party_relationship`/`import`/`commit` — `requires.party` mints
  `${party_id}`,
  the VERSIONED_OBJECT uid the SM admin operations take, and
  `requires.party_relationship` mints `${party_relationship_id}` the same way,
  creating BOTH endpoint parties first and writing their container uids into the
  relationship's `source`/`target` PARTY_REFs, as RM demographic
  `master02-demographic_package.adoc` §Party Relationships requires:
  "OBJECT_REFs containing HIER_OBJECT_IDs to denote the Version container of a
  Party"; `requires.import` replays a received EHR-Extract so a RELEASED read
  has an `IMPORTED_VERSION` to serve — `{ extract, container }`, the container
  naming which `X_VERSIONED_*` content item the case is about — and mints
  `${imported_versioned_object_uid}` + `${imported_version_uid}`
  (+ `${imported_branch_version_uid}` when that container carries a branch)
  from the extract's OWN identities, which RM common master06 §Copying
  preserves, plus `${ehr_id}` when no `requires.ehr` makes it a whole-EHR clone
  (§Copying Case 1 vs Cases 2/3)), never as a flow step. Provisioning may itself
  drive an EXTENSION
  route where the release surfaces no wire for the precondition (the
  relationship create — register AMB-32; the extract import — register AMB-34),
  so such a requirement is usable only
  on a party that serves that family — enforced at SELECTION time in `run.rs`
  against the capabilities THAT family's cases gate, never the requiring case's
  own.
- **Every closed vocabulary is a Rust enum/newtype** (outcome kinds,
  selectors, header matchers, capture sources, the `${…}` reference grammar,
  dispositions, sentinels, and every token a bundled CONTRIBUTION member may
  spell — `change_type` is the openEHR `audit_change_type` group, `_type` the
  RM VERSION classes the commit wire is addressed with, `lifecycle_state` the
  `version_lifecycle_state` group): illegal states unrepresentable, and an
  unknown token is a `literal-grammar` finding at validate time plus a loud
  step error at drive time — never a silent fall-back to `creation` /
  `complete` / `ORIGINAL_VERSION`. A member that must carry a shape the
  vocabularies exclude (a deliberately out-of-group state) still authors the
  whole `ORIGINAL_VERSION` verbatim, which is what that seam is for. New
  vocabulary values enter only by schedule release, never ad hoc. The one OPEN grammar
  beside them is an `ignoring:`/`server_assigned` PATH, and it carries a
  `**` segment matching zero or more attribute steps (`**/uid`): recursive
  containment is an RM shape (`FOLDER.folders: List<FOLDER>`), so a
  depth-anchored ignore path would silently under-cover a deeper tree.
- **A deployment fact no released operation discloses is an IXIT
  declaration, never a runner guess**: the party's `ixit.json` declares it
  (`system_id`; `dump_location` — a writable path on the SUT's OWN file system
  for the admin dump/load pair, since which paths a containerized SUT can write
  is a property of its image and mounts) and a case reads it as
  `${ixit:<field>}` — the field set is closed like every other grammar. A party that declares nothing makes the
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
  FerroEHR conformance posture (owner ruling 2026-07-28): the pipeline
  always overlays `docker/sut-smart.yml`, the ixit's principals mint scoped
  Bearer tokens (per-instance standing grants; a step-level `scopes:`
  overrides for the boundary cases), and the ONE committed record covers the
  whole claimed surface in one run — no focused lanes, no sidecar artifact
  directories. TERMINOLOGY follows the identical law: `ixit.terminology`
  declares the terminology query servers a deployment is wired to
  (`servers: [{name, reachable, namespaces}]`) plus the unresolvable-value-set
  `posture` (`fail_open` | `fail_closed`), because released ITS-REST surfaces
  NO terminology resource at all (the nine `I_TERMINOLOGY_SERVICE` off_wire
  rows) while BASE `architecture_overview/master12-terminology.adoc`
  §"Binding Terminology Value-sets to Archetypes" puts the bound value set in
  a server outside the CDR. A case states what it needs in
  `requires.terminology` (`served` / `unreachable` namespaces, `posture`,
  `distinct_servers` for the N>=2 simultaneity proof) and `run.rs` selects on
  it at SELECTION time; the block is per-instance-first with the top-level one
  as the party default, exactly like `signing`, so the two claimed postures
  ride the two composed deployments and land in the ONE record. The
  terminology-server-DOWN branch is a DECLARED `reachable: false` server wired
  to an address nothing answers on for the whole run — never a mid-run stop or
  reconfiguration.
- **A claimed capability is tested, and a posture the product claims BOTH
  branches of is tested in both — in the ONE record** (owner ruling
  2026-07-28). Version signing is the live case: RM common master06 §Digital
  Signature defines digest and openPGP as alternative depths of one mechanism
  and a deployment realizes exactly one, so the pipeline composes a SECOND
  deployment of the same built image in the pgp posture
  (`docker/sut-signing-pgp.yml` + `docker/sut-pgp-parallel.yml`, project
  `ferroehr-cnf-pgp`, host port 8081) and the ixit declares it as the
  `sut_pgp` **instance** carrying its OWN `signing` block. `signing` is
  therefore per-instance-first with the top-level block as the party default,
  and the `-pgp` SIG-VERSION siblings address that instance with `on:`. One
  `run`, one `results.json`, no outcome merging, no environment knob. A case addressing
  an instance the party does not declare is not-applicable with the citation
  at SELECTION time (`run.rs`), never a drive-time transport error, and case
  preconditions provision on the deployment the flow addresses.
- **A selection rule the runner implements is never ALSO written as prose**
  (`guards:`). Capability scoping is the typed shape: a case gating only
  capabilities the ICS does not claim is not-applicable with its citation,
  decided once in `run.rs` from the case's own `capabilities:` list, and the
  `guard-scope` gate refuses a guard that restates it — a per-case
  restatement is free to drift from the implemented rule with nothing to
  catch it, and one scoped to a capability the case does not gate states a
  rule nothing implements (issue #2378). Prose guards stay legal for the
  conditions no rule expresses, and the boundary is published on the
  `guards` property of the case-core schema.
- **A party STATEMENT publishes only what THAT party declares.** The
  `served_extensions` axis of `vocab/wire_surface.yaml` carries the routes
  and configuration gate of each extension family; which families a party
  serves is the party's own `statement.served_extensions` declaration, and
  the `SDoC` renders exactly that (a party declaring none says so). A route
  family is one product's own design — publishing the catalogue's table in
  every statement made a false claim about other vendors (issue #2377). The
  derived party documents regenerate FROM the committed statement + results
  + catalogue (`scripts/render/conformance-docs.sh`, CI regenerate-and-diff
  guarded like the SVGs), so a catalogue change cannot leave a published
  document stale.
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
  SUT behaviour; EHRbase and other harnesses are prior art, not oracles.
  Spec silences go through the ambiguity register
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
  --specs --write-report` refreshes `docs/conformance/coverage-report.md` (the
  flag is required — plain `validate` never writes). Two halves of the
  domain are not authored but DERIVED, so nothing can hide by never being
  written down: a RELEASED ITS-REST operation the SM models no interface for
  is enumerated from the pinned `NON_SM_REST_OPERATIONS` table under a
  reserved `I_ITS_REST_*` pseudo-interface (a catalogue naming convention,
  never an SM claim — register AMB-161; an unpinned use of the prefix is a
  finding), and the Axis-3 domain is parsed from the `#`/`##` sections of the
  two released overview chapters — every section must be named by an authored
  `elements`/`branches` source or pinned in `AXIS3_SECTION_EXCLUSIONS` with
  its citation.
- **An authored input the driver never reads is a CATALOGUE DEFECT, not
  decoration** — the `step-arguments` gate (`validate.rs`, issue #1830).
  Every `with:` key a flow step authors must be consumed by the binding the
  driver would select for it (path param, declared query parameter, a
  header/query/body template reference, the payload role and its aliases, the
  bundled-CONTRIBUTION `versions`/`audit` pair, a `required` format header's
  `${ds:…}` source, or a `with_<p>` auto-variant selector). An unread key
  never reaches the SUT, so whatever the case asserts about it passes
  VACUOUSLY however the server behaves — the live instance was the SEC audit
  case's deliberately ancient client-supplied `audit.time_committed`, dropped
  for its whole life. The gate models `select_body`'s short-circuit ORDER, so
  it is sharp where the driver is deterministic and generous only where the
  choice is a runtime property (the single-payload scan).
- **Every CITATION inside a binding declaration is machine-resolved** — the
  `spec-ref` gate now reads `unrealized.source` / `extension.source` too
  (issue #1832). Those fields are DERIVATIONS (what the SM defines; what the
  released ITS surfaces, or does not; the spec-silence flag), authored as
  `;`-separated clauses each opening with its component token — because a
  fragment that opens with anything else is DROPPED unread by the citation
  splitter, which is how the ITS-REST half of a `A vs B` derivation used to
  escape the gate entirely. The sibling `reason` field stays free-text
  prose by design: `source` + `reason` are the citation/note split.
- **ITS-XML citations resolve against TWO roots** (issue #1833):
  `scripts/vendor/spec-docs.sh` vendors prose, so the docs tree's
  `ITS-XML/components/**` holds only upstream README stubs, while the
  released XSD bundles are vendored ONCE at `crates/openehr-its/schemas/xml/`
  as the canonical-XML codec's input. The gate learns the bundle as a second
  root — one vendored copy, two readers, never a duplicated bundle — and an
  XSD's `§` sections are its declared `name="…"` values, so
  `ITS-XML components/RM/Release-1.0.2/documents/Composition.xsd §composition`
  resolves and a phantom element is a finding.
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
  write share into the population-anchored 10:1..50:1 band).
  **The simulation exercises every CLAIMED capability** — the resting state,
  not a target: the only claimed capabilities outside it are the ones whose
  exercise is impossible inside a sustained hold, each carrying its own
  per-capability `workload_exclusion` (register AMB-170) of exactly one of
  two kinds — destructive mid-measurement (physical deletion, and the
  released two-operation ADMIN API that IS that erase pair), or no request
  to send at all (neither released wire nor served extension route). "Not
  reached yet" is not a legitimate reason, and `workload-coverage` fails a
  stale exclusion, so a landed journey cannot leave one behind.
  **A stage's operation fixes its ixit PRINCIPAL** (`PerfOp::principal`):
  ordinary stages ride the party's `sut` instance — under the SMART posture
  that is a scope-limited Bearer token minted ONCE from the instance's
  standing `default_scopes` grant and re-minted only near its declared
  expiry, never per arrival — while the boundary/platform stages address the
  ixit's `unauthenticated` / `readonly` / `smart.platform_instance`
  declarations. A journey whose principals the party does not declare is NOT
  SCHEDULED and the remaining shares renormalize (an undeclared deployment
  fact costs coverage, never correctness). The two DENY probes measure the
  refusal itself — 401/403 IS the arrival's success — so they load the
  authn/authz path without mutating the measured population.
  Journey payloads = the CURATED CKM template pack
  (`artifacts/corpus/templates/ckm/*.opt`, COMPOSITION-rooted only, slugs
  hand-pinned per cid because a CKM template's `resourceMainId` is a UUID —
  the slugs are a contract read by MANIFEST.yaml, the journey definitions and
  `scripts/generate-ckm-examples.sh`, so never rename or drop one; provenance
  in its PROVENANCE.md; example skeletons regenerate via
  `scripts/generate-ckm-examples.sh` against a running SUT) plus the
  AUXILIARY payloads the non-COMPOSITION stages carry
  (the Simplified-FLAT pair, the demographic fixtures) — committed corpus
  entries the functional catalogue already adjudicates, selected by
  `PerfOp::aux_payload`, manifest-checked by `journey-envelope` and
  preflighted by the seeder; the load instrument invents no payload. The
  scale corpora + the standing ward seed strictly
  through the public API per `artifacts/corpus/recipes/scale_ladder.md`;
  published SVGs/summary tables render FROM committed results.json
  (`scripts/render/perf-assets.sh`, CI regenerate-and-diff guarded). The
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
