# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Maintenance rules: every pull request that changes user-visible behaviour —
the REST surface, AQL, validation, storage/migrations, configuration, CLI,
container/Helm artifacts — adds an entry under **[Unreleased]** in the same
PR (a CI guard enforces this). Cutting a release renames [Unreleased] to the
version + date, adds fresh link references, and tags `vX.Y.Z`; the release
workflow refuses a tag that has no matching section here.

## [Unreleased]

### Changed

- **Simplified Formats folded into `openehr-its` (#268)**: the FLAT /
  STRUCTURED / Web-Template implementation moved from the standalone
  `openehr-flat` crate into `openehr-its` as the `openehr_its::flat` module,
  mirroring the openEHR ITS component decomposition (Simplified Formats is a
  STABLE ITS-REST 1.1.0 sub-specification, alongside canonical JSON, XML, and
  the REST contract this crate already houses). Pure packaging refactor — no
  change to the FLAT/STRUCTURED/Web-Template wire behaviour.

## [3.9.0] - 2026-07-24

### Added

- **Content structural conformance cases from the official schedule**: the
  master15 COMPOSITION content×context tables and the master16 ENTRY-family
  tables (OBSERVATION, HISTORY, EVENT, ITEM_STRUCTURE) are now encoded under
  their verbatim official ids, replacing the ad-hoc structural cases that
  had been authored on the false claim that those chapters were empty;
  derivable catalogue extensions beyond the official cells survive as
  flagged addition cases.

- **Dual POC measured records on the v3.8.0 build, both directions
  published**: ehrbase-rs earns class POC (normative hour at 2.03/s
  offered, worst p99 108 ms, 0 errors / 7,320 requests); upstream
  EHRbase 2.34.0 on the identical instrument, corpus, and resource floor
  does not (ward-dashboard AQL p99 10.9 s vs the 1 s ceiling, 2.4%
  errors). Comparison page and all measurement visuals derive from the
  committed runner artifacts.

### Changed

- **Version-signature read verification is now `strict` by default (#273)**:
  with signing enabled and `signing.verify_on_read` unset, the server now
  recomputes the signature of every version it served and returns a `500`
  integrity fault on a mismatch, instead of the previous silent-pass (`off`)
  default that signed every version and then never checked it. Set
  `signing.verify_on_read` explicitly to `warn` (log + meter, still serve) or
  `off` (never check) to opt out. **Client-supplied signatures** (an author's
  own signature, or one carried by an imported version) are tracked as such and
  are always stored verbatim and never re-verified, so strict-by-default never
  rejects a legitimately-stored foreign signature. Our-own-design integrity
  hardening — no openEHR spec governs server-side verify-on-read timing (RM
  common master06 §Digital Signature).

- **CNF catalogue audited case-by-case against the official spec text
  (#231)**: every case in every chapter re-verified across grounds,
  expectations, citations, fixtures, captures, and register linkage, with
  the findings applied directly to the catalogue and register (the durable
  record is the register + closed issues + git history).
  Highlights: spec-overreaching rejection rows removed (AQL TERMINOLOGY
  operation strictness; the mixed-precision interval rows now report-only
  under the SPECPR-380 openness); the SEC-BASIC proposal citations corrected;
  stale stub-era template ids fixed; the delete-latest-version OPT case
  realigned to the official version-less ground; the wrong-template update
  ground rebased onto a fixture that is valid against its own template; the
  physical-EHR-delete binding accepts the OAS-enumerated async 202; eight
  new ambiguity-register entries pin previously prose-only adjudications;
  and every phantom REQUIREMENTS.md pointer now carries its real anchor.

### Fixed

- **Conformance-runner commit provisioning fails loud**: a `requires.commit`
  key resolving to a plain composition fixture was silently skipped, leaving
  the case's committed-state precondition unestablished; a single object now
  commits as a one-item set and any other shape is a provisioning error.

- **The measured-window driver accepts the spec-legal `204 No Content`
  minimal-return form** on create-family writes (ITS-REST: with
  `Prefer: return=minimal` a service SHOULD use 204 when no body is
  returned) — previously every upstream journey commit was falsely
  counted an error; and the upstream comparison stack's database now
  gets the same `shm_size` floor as the ehrbase-rs stack (Docker's 64 MB
  default starved its PostgreSQL during maintenance settling).

## [3.8.0] - 2026-07-24

### Added

- **CNF catalogue: stored-query name-grammar cases** — three new
  `definition_query` cases pin the ITS-REST `Qualified_query_name` grammar:
  a plain unqualified name and a namespace-less dotted name (the dot is part
  of the query-name character set, not a namespace separator) both store and
  read back, and the reserved query-name `aql` is rejected case-insensitively.
- **`cnf-runner stress-compare`** — the cross-SUT stress overlay: both
  systems' latency-throughput curves on one canvas, rendered
  deterministically from the two committed `stress.json` reports (driven
  by `scripts/render-comparison.sh`); both directions on equal footing.
- **Measured runs record resource telemetry**: each measurement in
  `results.json` now carries an optional, schema-published `resources`
  block — per-container (server and database separately) CPU, resident
  memory, block-device and network I/O sampled every 10 s across the
  whole window (run-clock offsets, warmup/measured/drain phase stamps),
  plus the database volume's on-disk size at four anchors (empty → scale
  seed → ward seed → after the window) with the derived bytes per
  committed composition. Sampling is enabled by the new optional
  `containers` block in the ixit (compose container names); without it a
  run records no resources and the report says so — telemetry never
  influences a class verdict. Two new rendered assets (the resource
  time-series and the disk-growth chart) join the perf-assets family and
  the book's Performance chapter, drift-guarded in CI like every
  published number.
- **`cnf-runner aql-probe`** — the seeded-corpus AQL optimization probe:
  fires the measurement machinery's own AQL vocabulary against a freshly
  seeded server, records wire-latency percentiles per probe, and
  attributes the database-side cost per statement (`pg_stat_statements`
  through the ixit `containers` capability, degrading honestly without
  it). Report schema published (`aql-probe.schema.json`); exploration
  evidence only — never a conformance record.
- **Stress steps carry resource telemetry** — every load-ladder rung
  records the same per-container CPU/memory/I/O series as the measured
  class runs over its own warmup+hold window, so a breached rung shows
  where it saturated; the stress progress stream now logs each rung's
  verdict live (stable/BREACHED with the sustained rate, resource peaks,
  and named breaches) plus a ladder recap, and measured class runs log
  their verdict evidence at window end.
- A **diurnal day-curve** arrival option for the extended 8/12-hour
  measured holds (ITU-T E.500 busy-hour semantics: the class floor is the
  busy-hour rate).
- The conformance certificate gains a **Workload Coverage** section:
  claimed capabilities vs the set the measured hospital simulation
  actually exercised, with untouched claimed capabilities listed
  explicitly as journey-catalogue gaps.
- `scripts/generate-ckm-examples.sh` — regenerates the committed CKM
  example payload skeletons from a running SUT's example endpoint;
  `scripts/vendor-ckm-templates.sh` now vendors the runner's journey
  template pack.
- **Conformance visuals**: the capability-matrix heat grid (one cell per
  claimed capability, grouped by profile tier, evidence encoded as a
  CVD-safe color AND a glyph) and per-chapter outcome bars, rendered
  deterministically from the committed verdicts/results by the new
  `cnf-runner conformance-assets` subcommand
  (`scripts/render-conformance-assets.sh`, CI regenerate-and-diff
  guarded) and embedded on the book's conformance and comparison pages
  (both SUTs) and the landing page.

### Changed

- **`--skip-seed` and the sidecar corpus index are retired** (CLI flags on
  `perf`/`stress`, the `CONF_PERF_SKIP_SEED` pipeline variable): every
  measurement instrument now always seeds a freshly composed, empty
  server and the stack is torn down afterwards — seed reuse bred
  stale-state confusion.
- **Measurement instruments settle database maintenance
  deterministically** (`vacuumdb --analyze` through the DB container)
  after seeding and before every measured window and stress rung —
  a stale-statistics plan after the million-row seed cost a measured ~9×
  on the ward-worklist query; settling moves that debt outside every
  measured window, identically for every SUT.
- The CNF measured-performance workload is now a full **hospital
  simulation**: the class cases (`PERF-hospital_sim-*`, renamed from
  `PERF-mixed_load-*`) schedule clinical journeys — ADT
  admission/discharge, vitals rounds, the medication loop, medicines
  reconciliation, asynchronous laboratory/imaging order-to-result
  pipelines, specialist/registry reporting, public-health notifications,
  chart review, ward dashboards with a registered stored query, versioned
  corrections, contribution audit review, workflow tagging, logical
  deletion, and template polling — expanding into 22 measured operation
  kinds instead of 4, each with its own HDR-V2 record. The
  population-anchored envelope is unchanged and now validator-enforced
  (the expanded write share must reconcile to the derivation's 10:1..50:1
  read:write band); journey payloads commit against 15 COMPOSITION-rooted
  openEHR CKM templates vendored with provenance.

### Removed

- **The transitional benchmark lab** (`tools/benchmark`,
  `scripts/benchmark.sh`, `docker/benchmark/`, the manual benchmark
  workflow, and the committed `docs/benchmarks/**` artifacts): all
  measurement is native to the CNF runner — measured class runs, the
  stress ladder, the AQL probe, and the cross-SUT stress overlay — and the
  comparison page now derives its performance side from the committed
  `docs/conformance/<sut>/stress.json` reports (upstream shown as "not
  measured yet" until its report lands, never a one-sided claim).
- The completed ECC→CNF cutover comparison lane: the generated
  `docs/conformance/cnf-comparison.md`, the `cnf-runner compare-ecc`
  subcommand, the drift gate, and the preserved ECC catalogue/map (all in
  git history; the five deferred grounds are re-registered on the
  catalogue-deepening tracker). The `docs/conformance/CATALOG.md` pointer
  stub is gone with it, and the CNF 2.0 design record moved to
  `docs/conformance/cnf-design.md` as a permanent reference document.

### Fixed

- **Storing a query under the reserved name `aql` is now rejected** with
  400, case-insensitively and whether or not a namespace is supplied
  (ITS-REST `Qualified_query_name` §NOTE — the name would collide with the
  ad-hoc `/query/aql` route). A three-part `ns::aql::name` name keeps
  working: its middle segment is the formalism, not the query-name.
- **A coded value whose text is not the template-bound rubric is now
  rejected at commit** (422 naming the path, the committed value, and
  the bound rubric): RM `DV_CODED_TEXT` — "value must be the rubric from
  a controlled terminology" — enforced wherever the template itself is
  authoritative for the rubric (archetype-local at-codes and explicitly
  bound external term definitions, any bound language); `openehr`-
  terminology codes stay unchecked (the terminology ships official
  translations the template cannot enumerate), and a bound code with no
  rubric stays accepted. The once-accepted code-as-value instance is a
  pinned rejection.
- **Coded-text example values now carry the template-bound rubric**: the
  Web Template builder resolved display labels only for local at-codes,
  so an external code's rubric (OPT `term_definitions` keyed
  `TERMINOLOGY::code`, e.g. SNOMED-CT bindings) was lost and generated
  examples emitted the raw code as `DV_CODED_TEXT.value` — spec-invalid
  instance data (RM: "value must be the rubric from a controlled
  terminology"). The qualified key now resolves; the covid19 example
  regenerates with rubrics; every pack example commits clean on strict
  validators.
- **Child-assembled `DV_INTERVAL` values now carry the mandatory boundary
  flags**: an interval built from `lower`/`upper` sub-path children (the
  FLAT builder's container path — template examples included) previously
  omitted `lower_unbounded`/`upper_unbounded`/`lower_included`/
  `upper_included`, making every half-open interval spec-invalid (BASE
  `Interval`: the flags are mandatory and `Limits_consistent` is
  unevaluable against an absent bound); the flags now derive from bound
  presence, an explicit datum flag wins, and the committed CCTA example
  is regenerated. Strict validators (upstream EHRbase) rejected the old
  instances with 422.
- **Population AQL with `LIMIT` now streams instead of materializing the
  corpus**: a LIMIT-bearing, unordered, non-DISTINCT, non-aggregate
  population query lowers to a streaming FROM shape (the current-version
  spine with `LATERAL` node probes), so PostgreSQL stops at the LIMIT
  instead of building an archetype-anchor bitmap over every matching node
  first — measured on a million-composition corpus, the cross-EHR ward
  worklist drops from ~113 ms to ~2 ms per execution (~40× fewer buffer
  reads); ordered/aggregate/EHR-scoped queries keep the previous plan
  shape, and result semantics are unchanged. A version-field projection
  of `uid`/`contribution_id`/`lifecycle_state` no longer joins the audit
  table it never reads.
- **AQL cross-EHR queries with `LIMIT` no longer collapse under corpus
  scale**: predicates on multi-valued (anchored) paths now lower as
  existential semi-joins (`EXISTS` — the predicate holds when ANY matched
  node satisfies it; deterministic where the previous first-match pick was
  plan-dependent), the archetype anchor index leads with the RM type so
  the whole `CONTAINS`-class + archetype boundary is one index probe, and
  queries that never touch audit fields no longer join the audit table.
  The measured ward-dashboard profile (p99 5.8 s at class-POC scale) drops
  to milliseconds-per-request territory.
- The template **example generator no longer collapses `DV_INTERVAL`
  wrappers** onto a single constrained bound: interval-valued elements keep
  their interval identity (bounds as `/lower`/`/upper` sub-paths per the
  Simplified Formats mapping), fixing generated examples the platform's own
  validation rejected (the CKM CCTA report OPT); the CNF journey catalogue
  re-commits the CCTA imaging report.

## [3.7.0] - 2026-07-22

### Added

- The conformance pipeline assesses **upstream EHRbase (Java)** as a second
  system under test: `CONF_SUT=ehrbase-java scripts/conformance.sh` composes
  the official `ehrbase/ehrbase:2.34.0` + `ehrbase-v2-postgres` images on
  fresh volumes (`docker/sut-ehrbase-java.yml`, readiness probed externally
  — the official image carries no in-container health tooling) and runs the
  same committed catalogue with upstream's own committed party set
  (`tools/cnf-runner/party/ehrbase-java/`). The public comparison
  (`docs/conformance/COMPARISON.md` + the website comparison page) is fully
  generated from the two committed results/verdicts sets — profile verdicts,
  the 39-capability evidence matrix, and failure tables in both directions.
- The conformance runner performs ISO/IEC 9646-style ICS-driven test
  selection: `cnf-runner run --statement` excuses option-gated cases whose
  register branch the party statement does not declare as N/A with citation
  (previously they ran and recorded spurious failures the verdict pipeline
  then excused).
- Conformance badges carry measured amounts: per-tier badges read e.g.
  `PASS 10/10 capabilities`, the overall badge `CORE+STANDARD PASS ·
  323/323 cases` — derived from `verdicts.json` + the capability matrix,
  never hand-typed.


- Read-only role support in RBAC: a principal carrying the configured
  `authz.rbac.readonly_role` (default `READONLY`) is refused with `403` on
  every write operation — creating an EHR, committing a composition,
  uploading a template, and any update/delete — even when it also holds
  granting roles such as `ADMIN`. Reads and AQL queries stay permitted, so a
  `READONLY` account is an authenticated, view-only principal. The dev compose
  stack ships an `ehrbase-readonly` account (password `ehrbase`) for
  evaluation.
- CNF 2.0 reference runner, third increment — the executor and both verdict
  machineries: the data-driven flow interpreter under the five interpreter
  laws (per-row re-provisioning, step-mismatch row abort, errored-vs-failed
  classification, fixed temporal resolution, aggregates-after-last-row) with
  the live HTTP driver realized purely from the operation bindings, the
  reference resolver (corpus/recipes/rows/captures with normative sentinel
  semantics), the normative RESULT_SET equivalence comparator, content-case
  execution via the synthesized generate→commit→expect flow, the party
  artifacts (statement/results/ixit with schema validation and mandatory
  N/A citations), the pure verdict pipeline + deterministic
  report/statement/certificate renderers, the runner-verification pack
  (committed transcript + player: adjudicated verdicts reproduced, broken
  runners rejected), and the performance machinery (class cases with the
  published population-anchored floors, re-checkable HDR V2 measurement
  records, the earned/not-earned pure verdict). Nine published JSON-Schema
  families, drift-guarded. Live-SUT runs (the earned-class measurement and
  pack part 2) execute against a composed SUT via the new `run`/`verdicts`
  CLI once cutover lands.
- CNF 2.0 reference runner, second increment: the complete CNF 2.0 catalogue
  authored from the framework — 347 cases across every schedule chapter
  (EHR, EHR_STATUS, COMPOSITION, CONTRIBUTION, DIRECTORY, ADL 1.4 + ADL2
  definitions, stored queries, demographic, admin, messaging, AQL, content
  data-type and structural validation, simplified formats, Security
  SEC-BASIC + Signing) with 84 per-operation ITS-REST bindings (every
  status/header mapping cited to its OAS source; wire gaps are typed
  `unrealized` declarations, not silent absences), the ambiguity register
  grown to 38 adjudicated entries, and the ECC↔CNF comparison gate CLEAN:
  all 394 active rows of the old harness's catalogue adjudicated
  (350 covered, 5 deferred to the simplified-formats deepening, 18 dropped
  with justification, 9 out of scope, 12 ADL2 rows covered) in the committed
  map with the generated report at `docs/conformance/cnf-comparison.md`
  (drift-guarded). Old-harness retirement follows the owner's report review
  with the executor/emission workstreams so an acceptance instrument runs
  continuously.

- CNF 2.0 reference runner (`tools/cnf-runner`), first increment: the typed
  schedule-artifact model (case cores, per-ITS operation bindings, outcome +
  selector vocabularies, the capability→family→tier matrix, corpus manifest,
  ambiguity register — every closed vocabulary a Rust enum/newtype), a
  published JSON-Schema set for all seven artifact families (committed under
  `tools/cnf-runner/schemas/`, drift-guarded, vendorable by any runner), a
  full cross-artifact validator (id uniqueness, SM-operation and spec-ref
  resolution against the vendored specs, binding completeness, corpus
  integrity, reference/sentinel and decision-table grammars, capability-tier
  consistency), the `cnf-runner` CLI (`emit-schemas`, `validate`), and the
  eight pilot case encodings as the first schedule artifacts. The existing
  ECC (`tools/conformance`) is unchanged and remains the acceptance
  instrument until the comparison gate.
- Performance conformance, measured end to end: a `cnf-runner perf` run plays
  an open-loop offered-load schedule against a composed server at a
  population-anchored volumetric class (proof-of-concept, small, large,
  regional), records re-checkable HDR histograms into the conformance
  results, and earns — never declares — a class verdict recomputed by the
  verdict pipeline. `CONF_PERF_CLASS=<class> scripts/conformance.sh` runs it
  as a pipeline stage; the earned classes flow into the verdicts, report,
  certificate, and a performance badge. Published SVG assets (the class
  ladder and per-class latency charts) plus a generated summary are rendered
  from the committed measurement records by `scripts/render-perf-assets.sh`
  and guarded against drift in CI, and a new **Performance** chapter on the
  documentation website explains the class ladder, the floors' derivation
  from official activity statistics, how a coordinated-omission-free run
  works, and how to reproduce it.
- The sustained-window ladder: `cnf-runner perf --hours 1|2|4|6|8|12`
  (pipeline: `CONF_PERF_HOURS`) extends a class run's measured window beyond
  the normative hour — a longer hold of the same offered load is a stricter
  demonstration and persists like any measured run. There is deliberately no
  shortened run.
- A step-load **stress instrument**, distinct from conformance:
  `cnf-runner stress` climbs short intense load steps (geometric doubling,
  ~two-minute holds, bisection refinement) to the **maximum sustainable
  throughput** inside a latency budget, over the same seeded corpus and
  workload mix as the class runs. The report (`stress.json`,
  schema-published, environment-bound, per-step re-checkable histograms)
  earns no class and never touches the conformance results; the class floors
  appear as context only. A latency-throughput curve SVG renders from the
  committed report through the same drift-guarded asset pipeline, and the
  documentation's Performance chapter tells the two-instrument story.

### Changed

- The conformance acceptance instrument is now the CNF 2.0 reference runner
  (`tools/cnf-runner`) end to end: `scripts/conformance.sh` composes the SUT
  on fresh volumes, executes the committed machine-readable catalogue,
  computes verdicts through the pure pipeline, and writes
  results/verdicts/report/statement/certificate + badges per SUT. The ECC
  harness (`tools/conformance`) is retired — its final inventory is
  preserved at `tools/cnf-runner/comparison/ecc-catalog.tsv` and the
  reviewed cutover record is `docs/conformance/cnf-comparison.md`; the
  previous ehrbase-java comparison artifacts are frozen as historical data.
  Committed per-SUT party sets (ixit + statement) live under
  `tools/cnf-runner/party/`.
- Verdict semantics: a REQUIRED capability whose every selected case is
  excluded by a schedule-registered ambiguity (an unrealized wire on the
  technology profile, e.g. ADL 1.4 archetype provisioning under ITS-REST
  1.1.0 — AMB-41) is now recorded as an explicit `unrealized` scope
  exclusion on the certificate instead of silently failing the tier; the
  API-presence capabilities (EHR/DEFINITION/QUERY API) are evidenced by
  chapter exemplar cases.
- The benchmark harness converged onto the conformance runner's corpus,
  recipes, and ixit topology, so both instruments seed identical clinical
  documents through the public write path. The performance numbers in the
  README and on the website are no longer hand-typed: they derive from
  committed run artifacts (the benchmark comparison charts and the CNF
  measurement records), and the site stale-numbers guard now also rejects a
  hand-typed rate, latency, or footprint in the sources.


- OPT-1.4 → ADL2 conversion fidelity: `DV_ORDINAL`/`DV_QUANTITY` constraints
  now convert to real AOM2 attribute tuples (`[value, symbol]`,
  `[units, magnitude(, precision)]`) instead of loose unconstrained nodes;
  slot include/exclude assertions are carried (both retained 1.4 slots and
  the filled-slot `include` naming the embedded archetype); OPT
  `default_value`s are carried and serialized as the ADL2 `_default`
  pseudo-attribute; temporal constraints keep both the ISO8601 pattern and
  the range plus assumed values; `referenceSetUri` becomes an ac-code term
  binding; `CONSTRAINT_REF` resolves against the merged 1.4
  `constraint_definitions`/`constraint_bindings`; and everything a
  decomposed root cannot express (out-of-scope bindings, tuple assumed
  values, `DV_STATE` machines, unconvertible assertions) is reported in the
  converted archetype's `RESOURCE_DESCRIPTION.conversion_details`. The
  whole vendored OPT corpus now converts, validates and re-parses as the
  standing test gate.

### Fixed

- OPT 1.4→2 decomposition now emits phase-1-clean ADL2 sources for every
  template in the corpus: a `-`-specialised embedded root (whose
  differential lineage a flattened OPT cannot resolve) is emitted as an
  unspecialised depth-0 archetype with every dotted code renumbered into
  the flat code space, and 1.4 node codes legitimately reused across
  sibling subtrees re-mint archetype-wide-unique ADL2 ids — terminology
  definitions and bindings follow in both cases, and every remap is
  recorded in the converted archetype's `conversion_details` provenance.

- The ATNA Audit Record Repository no longer loses records under a sustained
  write load: the audit drain now takes queued events in batches and
  persists each batch in one multi-row `INSERT` (the previous per-event
  round trips saturated far below write-path rates, filling the bounded
  queue and fail-open dropping the tail). Drop warnings are rate-limited to
  one per interval carrying the count since the previous warning instead of
  one log line per dropped record (the exact count stays on the
  `atna_audit_dropped_total` metric), and the default
  `audit.queue_capacity` rises from `1024` to `8192` for burst headroom.

- Composition validation closes eight archetype-constraint enforcement gaps
  the CNF content chapter exposed: `C_STRING` list/pattern constraints on
  `DV_IDENTIFIER.issuer`/`assigner`/`type` (only `id` was checked);
  `DV_MULTIMEDIA.size` against `C_INTEGER` list and range constraints
  (previously unvalidated); `C_ATTRIBUTE` existence `1..1` on
  `OBSERVATION.state`/`protocol`, `HISTORY.summary`, and `EVENT.state` now
  rejects the absent attribute; `DV_SCALE` value/symbol value-set
  constraints (generic `C_REAL` list + `C_CODE_PHRASE` code list — AOM 1.4
  has no `C_DV_SCALE`) are enforced, including on `DV_INTERVAL` bounds;
  `timezone_validity` on `C_TIME`/`C_DATE_TIME` (mandatory and prohibited)
  is honoured; half-open (one-side-unbounded) temporal range constraints
  reject out-of-range values; a `DV_PROPORTION` of kind fraction or
  integer-fraction with a non-zero `precision` is rejected
  (`Fraction_validity`); and a partial `DV_TIME` such as `10` is no longer
  over-rejected against `HH:??:??`/`HH:XX:XX` patterns (optional and
  not-allowed fields both admit an absent field).
- A `DV_TIME`/`DV_DATE_TIME` literal carrying a fraction on the hours or
  minutes component (e.g. `10.5`, `10:05.5`) is now rejected: openEHR
  supports fractional seconds only (BASE time types §ISO 8601 semantics not
  included).
- A `DV_URI` whose value has no URI scheme (e.g. `xyz`, `www.example.org`)
  is now rejected on commit per the CNF content schedule's RFC-3986 rule;
  plain-text URI content after the scheme remains accepted per the RM's
  plain-text allowance.
- A COMPOSITION create (`201`) or update (`200`) whose response is negotiated
  as a Simplified Format (`Accept: application/openehr.wt.flat+json` or
  `…wt.structured+json`) now returns the `ETag` and `Location` headers, matching
  the canonical (`application/json`/`application/xml`) response. Previously a
  FLAT/STRUCTURED commit body omitted both version-id headers, so clients could
  not read the new version uid or resource URL from a simplified-format commit.
- Composition validation now rejects a `DV_DURATION` whose value carries a
  decimal fraction on any component other than seconds (e.g. `P1Y3M4DT2.5H` or
  `PT2H14.5M`). openEHR permits a fraction only on the seconds component
  (BASE time types: "in openEHR, only fractional seconds are supported"), so
  such a value now fails its RM `Value_valid` invariant with `422` instead of
  being accepted.
- Composition validation now enforces a `DV_QUANTITY` constraint that fixes a
  measurement `property` (with no enumerated unit list): the committed `units`
  must be a unit of that physical property (per the openEHR measurement
  property↔unit table). A quantity constrained to `length` committed with a
  mass unit such as `mg` is now rejected with `422` instead of being accepted.
- Composition validation now rejects a coded value whose terminology is
  foreign to a `C_CODE_PHRASE` constraint that explicitly binds the
  archetype-`local` terminology with a closed code list. Committing a
  `DV_CODED_TEXT` whose `defining_code` uses, e.g., SNOMED-CT against a
  `local`-scoped closed list now yields `422` instead of being accepted.
- The AQL `ehr_id` execution scope now also binds bare `FROM EHR e` sources:
  a scoped query without a CONTAINS chain previously ran over the whole
  population instead of the single EHR context the `ehr_id` parameter selects
  (ITS-REST query `Request.md` §Common Headers and Query Parameters).
- A CONTRIBUTION delete member targeting the EHR_STATUS is now refused with
  `409 Conflict`: `EHR.ehr_status` is mandatory (RM ehr, EHR class, 1..1), so
  deleting the only status would leave the EHR violating its own invariant.
- FLAT/STRUCTURED commits: spec-listed direct RM-attribute paths that an
  operational template leaves unconstrained are no longer rejected as unknown
  paths. `ACTION/ism_transition` (`current_state`/`transition`/`careflow_step`
  + `_reason:i`) and `ACTION/time`, plus `INSTRUCTION/narrative`,
  `OBSERVATION/history_origin`, `ACTIVITY/timing` + `action_archetype_id`, and
  `INTERVAL_EVENT/width` + `math_function`, are now built from their datum
  parts per the ITS-REST Simplified-Formats `master05-rm_mapping.adoc` per-type
  tables, and emitted symmetrically on the reverse (RM → FLAT) direction so
  round-trips stay lossless. Previously a client-supplied `ism_transition` was
  rejected with "unknown simplified path" and the ACTION state fell back to the
  synthesized `initial` default.
- AQL paging: the REST `fetch`/`offset` parameters now page over the result
  set the AQL `LIMIT`/`OFFSET` clauses define instead of being rejected with
  `400` when combined. Per ITS-REST query `Request.md`, only pairing `fetch`
  with the deprecated AQL `TOP` modifier is prohibited — that rejection
  remains. Negative `fetch`/`offset` values are now rejected explicitly.


- Spec version identity is now derived from the `openehr-*` crate versions
  instead of hand-typed literals, fixing the stale values those literals had
  drifted to: the startup banner advertised `ITS-REST 1.0.3` (now `1.1.0`),
  and the AQL `RESULT_SET` `meta._schema_version` was still emitted as
  `1.0.3` (now `1.1.0`, the implemented ITS-REST release). Every `openehr-*`
  spec crate exposes a `SPEC_VERSION` constant (= its crate version; the AM
  crate also exposes per-generation `am14`/`am24` constants from the BMM
  schemas), and the shared provenance constants behind the banner,
  `/status`, `OPTIONS /` (System Options), and `/management/info` read
  those, so a future pin bump propagates everywhere at compile time. The
  served `restapi_specs_version`/`openehr_rest_api_version` identity is now
  the plain version string `1.1.0` (matching the System API OAS example)
  instead of the tag-styled `Release-1.1.0`.
- SM call-status fidelity: service-layer "does not exist" failures now carry
  their granular `CALL_STATUS_TYPE` (`ehr_id_does_not_exist`,
  `composition_does_not_exist`, `template_does_not_exist`,
  `object_version_does_not_exist`, …) end-to-end instead of resurfacing as
  the generic `versioned_object_does_not_exist` after crossing the service
  boundary. HTTP status codes are unchanged (every does-not-exist status was
  and remains `404`); some `404` body messages are now the precise
  construction-site text.

## [3.5.0] - 2026-07-21

### Changed

- Conformance: zero skipped outcomes. The former 35 skips are eliminated —
  11 cases now execute against the documented ehrbase-rs extension surfaces
  (contribution listing, admin template deletion, bare stored-query
  listing), 6 more execute via new composed-stack wiring (an OpenPGP-signing
  sibling instance and a hermetic FHIR terminology fixture with fault
  injection) and loaded-database AQL golden support, and 18 native-API-only
  service operations are now first-class not-applicable verdicts carrying
  their SM citation and native-test evidence.

### Added

- ADL 2 archetype validation now enforces VETDF (external term-binding
  validity): a term bound to an external terminology (SNOMED CT, LOINC, …)
  that the configured terminology service reports as absent is rejected
  `422` with the `VETDF` rule code. Bindings the service cannot verify (no
  external provider configured, an unknown terminology, or a transport
  fault) are not raised, per the spec's "subject to tool accessibility"
  carve-out; archetype-internal (`local`/`openehr`) bindings are unaffected
  (covered by VTTBK/VTCBK key validity).
- ISO 8601 temporal ordering on the openEHR BASE time types
  (`Iso8601_date`/`_time`/`_date_time`/`_duration`): comparison with honest
  incomparability (partial-date range semantics, UTC normalization for
  zoned values, duration ordering via the spec's own `to_seconds`
  reduction with the `Time_definitions` average constants). ADL 2
  archetype validation now enforces assumed-value interval containment for
  temporal constraint types (previously undecidable and skipped); an
  incomparable pair never raises a violation.

## [3.4.0] - 2026-07-20

### Changed

- The implemented openEHR REST API is **ITS-REST Release-1.1.0** (published
  upstream 19-Jul-2026). The server was already built against the
  pre-release text of this release — the regenerated REST contract is
  byte-identical at the release tag — so wire behaviour is unchanged; the
  advertised API identity moves from 1.0.3/development to 1.1.0 everywhere
  (documentation, OpenAPI metadata, conformance artifacts), and the
  `openehr-its` spec crate is now versioned 1.1.0. Conformance reports
  state the tested edition as `release-1.1.0` (formerly `development`;
  the old label remains accepted as a CLI/config alias).

## [3.3.0] - 2026-07-20

### Added
- **ADL2 templates are now compiled and validated by the full ADL2 engine.**
  `POST /definition/template/adl2` runs the complete `openehr-adl` pipeline —
  parse, then the AOM2 validity catalogue (phase 1 basic integrity, reference-
  model conformance, and specialisation conformance against an already-loaded
  parent) — in place of the former source-subset probe. An invalid artefact is
  a **422** whose `Error.validationErrors` list the offending rule-code
  mnemonics (S-codes for an unparseable source, V-codes for a validation-phase
  failure). `GET /definition/template/adl2/{template_id}` now serves the
  `application/json` `OperationalTemplateV2` projection alongside the
  `text/plain` source, and resolves a partial `template_id` to the latest
  matching version; the previously `501` `…/{template_id}/{version}` (versioned
  get, marked deprecated in the spec) is implemented, and template list rows now
  carry `concept` and `archetype_id`. `GET …/{template_id}/example` now generates
  an example COMPOSITION from the compiled operational template (an ADL2 →
  Web Template front end feeding the shared example generator), served across the
  four `Accept_LOCATABLE` representations (canonical JSON/XML, `openehr.wt.flat`,
  `openehr.wt.structured`) with `type` (`input`/`output`) + `detail_level`
  (`required`/`medium`/`complete`) query parameters, and `400`/`404`/`406` exactly
  as the ADL 1.4 example endpoint. An `Accept` naming only `application/xml` on
  the plain template GET is a `406` (the operation declares no XML response body).
- **ADL 1.4 archetypes are now validated by the ADL 1.4 engine, and can be
  migrated to ADL 2.** An ADL 1.4 source archetype (the `I_DEFINITION_ADL14`
  archetype surface) is now parsed and validated **as ADL 1.4** by the
  `openehr-adl` engine — the subset of the phase-1 catalogue that corresponds to
  the ADL 1.4 / AOM 1.4 standalone validity rules (VARID, VARDT, VARCN, VATID,
  VDSEV/VDSIV, …), replacing the former structural probe. An invalid source is a
  **422** naming the offending rule-code mnemonic. A new service capability
  migrates a stored ADL 1.4 archetype to ADL 2 source (`adl14_convert_to_adl2`);
  no openEHR spec governs 1.4 → 2 conversion (our own design/extension) and the
  ITS-REST contract declares no conversion operation, so it is a library
  capability with no REST endpoint. The ADL 1.4 operational-template (OPT) REST
  surface (`/definition/template/adl1.4`) is unchanged.
- **RM terminology-backed invariant validation.** Composition (and any RM
  value) validation now enforces the openEHR terminology-service and code-set
  RM class invariants at the wire-boundary dispatcher, unified into a single
  hook (`openehr-its`) that every validation consumer inherits. The 30 wired
  invariants (each audited clean against the whole corpus before enforcement):
  `COMPOSITION` category/language/territory, `EVENT_CONTEXT` setting,
  `ELEMENT` null-flavour, `ISM_TRANSITION` current-state/transition,
  `PARTICIPATION` + `EXTRACT_PARTICIPATION` function/mode, `INTERVAL_EVENT`
  math-function, `TERM_MAPPING` purpose, `AUDIT_DETAILS` change-type,
  `ATTESTATION` reason, `PARTY_RELATED` relationship, `VERSION`
  lifecycle-state, `ENTRY`/`DV_TEXT` language + encoding, `DV_MULTIMEDIA`
  media-type/charset/language/compression/integrity algorithms, `DV_PARSABLE`
  charset/language, `DV_ORDERED` normal-status, and the `AUTHORED_RESOURCE` /
  `RESOURCE_DESCRIPTION_ITEM` / `TRANSLATION_DETAILS` original-language. An
  out-of-vocabulary openEHR code is a `422` naming the violated RM invariant;
  HTTP status codes are unchanged.

- Admin console: the Directory tab is now a complete directory experience —
  a structured folder-tree editor (add/rename/remove sub-folders, attach and
  remove composition item references with a picker), version history with
  read-only views and one-click restore, a `version_at_time` time-travel
  control, a sub-folder `path` query, and directory deletion with
  confirmation — on top of the existing create-from-template flow (raw JSON
  editing stays available as an advanced mode).

### Changed
- **RM validation invariant messages now carry the spec's (BMM) invariant
  names.** Three class-invariant violation messages were reconciled from their
  inherited archie spellings to the openEHR BMM invariant names, so a `422`
  validation payload reporting one of them changes text: `Accuracy_valid` →
  `Accuracy_validity` (DV_AMOUNT and its descendants — DV_QUANTITY, DV_COUNT,
  DV_DURATION, DV_PROPORTION), `Is_archetypeRoot` → `Is_archetype_root` (the
  ENTRY subtypes — OBSERVATION, EVALUATION, INSTRUCTION, ACTION, ADMIN_ENTRY),
  and `Location_validity` → `location_valid` (EVENT_CONTEXT). The check logic
  and HTTP status codes are unchanged; only the invariant name inside the
  `Invariant <name> failed on type <TYPE>` message differs.

- **Canonical-JSON codec cutover.** The openEHR spec types are now
  (de)serialized to/from canonical JSON entirely by a native emitted
  `ToJson`/`FromJson` codec in `openehr-its` — the spec types (`openehr-base`,
  `openehr-rm`, `openehr-am`, `openehr-term`, `openehr-lang`) no longer carry a
  serde derive, and the `openehr-derive` proc-macro crate is removed. The wire
  bytes are unchanged (proven by the R0 determinism manifest + the byte-hazard
  gates); the only externally visible difference is the **error-message shape on
  a malformed JSON request body** — the codec's parser reports `expected … at
  line N column M` / `missing field … on …` diagnostics instead of the previous
  serde phrasing (the HTTP status codes are unchanged: still `400`/`422`). A
  present-but-`null` array field is now rejected as a type error (was silently
  treated as an empty array), matching the strict tolerance contract.

- The served OpenAPI document now describes the COMPLETE wire for every
  operation (162 declarations across all API groups): every path/query
  parameter, request headers (`Prefer` incl. `return=identifier`, required
  `If-Match` forms, the committal headers), every reachable status code
  with its exact trigger, and the load-bearing response headers (weak
  `ETag`, `Location`, `Last-Modified`) — audited operation-by-operation
  against the vendored ITS-REST specification (both the operation
  definitions and the normative overview rules). A structural completeness
  test now gates the document.
- A disabled Admin API now answers `405 Method Not Allowed` (the status the
  ITS-REST specification declares for a disabled admin operation) instead
  of `404`.
- COMPOSITION and EHR_STATUS tag updates now honour the `Prefer` header as
  the specification defines: the default (`return=minimal`) returns
  `204 No Content`; `return=representation` returns `200` with the stored
  tag list. Previously the stored list was always returned with `200`.
- Demographic responses now carry `Last-Modified` (from the version's
  commit time) alongside the weak `ETag`; PARTY_RELATIONSHIP create/update
  honour `Prefer: return=identifier`.

### Fixed
- **Template example generation now produces fully-valid compositions.**
  `GET /definition/template/adl1.4/{template_id}/example` populated only a
  skeleton for many templates (issue #94) and could emit out-of-range or
  wrongly-typed values. The generator now synthesizes spec-valid values for
  every constrained field — quantities inside their magnitude ranges (with
  dimensionless empty units preserved), proportions satisfying their kind's
  invariants inside the archetype's numerator/denominator ranges, durations
  inside their declared range, coded text from closed value lists, URIs and
  parsables honouring their pattern constraints, and the archetype-constrained
  container/event types (`ITEM_LIST`/`ITEM_SINGLE`/`INTERVAL_EVENT`) instead
  of abstract defaults — and every generated example at the committable detail
  levels (`required`, `medium`) passes the server's own full composition
  validation. Generation is byte-deterministic.
- **Archetype-conformance validation no longer demands `archetype_node_id` on
  reference-model types that cannot carry one.** `EVENT_CONTEXT` (and any
  other non-`LOCATABLE` type) inherits `PATHABLE`, which the RM gives no
  `archetype_node_id`; a template archetyping `/context[at…]` therefore could
  never be satisfied by canonical data and such compositions were wrongly
  rejected on commit. Non-`LOCATABLE` nodes now match structurally by their
  attribute position (per the RM inheritance graph); `LOCATABLE` nodes keep
  strict node-id matching.

- Admin console: text typed into the EHR finder and create-EHR fields before
  the app finished loading is no longer silently wiped (the inputs are now
  hydration-safe, like the login form); success toasts no longer intercept
  clicks on buttons beneath them in the e2e battery.
- `GET /ehr/{ehr_id}/directory/{version_uid}` now honours the `path` query
  parameter (slash-separated FOLDER names selecting a sub-folder subtree),
  as the ITS-REST `directory_get_by_version_id` operation specifies; an
  unresolved path returns 404. Previously the parameter was accepted but
  ignored and the full tree was always returned.
- The served OpenAPI now documents the full DIRECTORY wire contract
  (`version_at_time`/`path` parameters, `Prefer` including
  `return=identifier`, `If-Match`, and the complete status ladders
  including 204/400/409/412).

## [3.2.0] - 2026-07-18

### Added
- **`GET {base}/admin/config` — the redacted effective configuration** (an
  ehrbase-rs extension; the openEHR admin API defines only EHR deletes).
  Returns the merged effective configuration (file + `EHRBASE_*` env +
  `--set` overrides) as a JSON tree with every secret-bearing value redacted
  structurally by its secret type — passwords, password hashes, HMAC/signing
  secrets, and S3 secret keys render as `***`, and connection URLs (database,
  AMQP) mask their embedded credentials while keeping host and path; non-secret
  identifiers (usernames, roles, OIDC issuer) stay visible. Shares the admin
  gate and authorization of the admin deletes (`EHRBASE__ADMIN__ENABLED=true`,
  `ADMIN` role); disabled admin API answers `404`.
- **`ehrbase-admin-ui` — the admin console**, a new standalone web
  application (its own binary and OCI image,
  `ghcr.io/rubentalstra/ehrbase-rs-admin-ui`) that manages any
  ITS-REST-1.0.3 CDR strictly over its REST API. Pure Rust end to end
  (Leptos SSR + WASM, zero hand-written JavaScript). Feature set:
  dual Basic + OIDC login (credentials held server-side in the BFF),
  a dashboard (count tiles, query-group tiles, a commit-activity trend
  chart), a Template Manager (list/filter/upload OPTs with the CDR's
  validation diagnostics verbatim; per-template path-catalog tree, raw-OPT
  view, and format-switchable generated example), an EHR browser (finder,
  status/directory/compositions/contributions, and a composition viewer
  with canonical JSON/XML + FLAT/STRUCTURED toggle, version history, and
  audit details), a **point-and-click Query Builder** that assembles the
  real AQL AST (typed per-datatype criteria from the template's
  constrained value sets, nested AND/OR/NOT groups, projection columns,
  live AQL preview) and runs it via the Query API, a raw AQL editor with
  BFF-side grammar validation and parameter bindings, stored-query
  management with console-local query groups, and a system panel (CDR
  status, SMART discovery, the served OpenAPI rendered natively).
  Configured by one `ehrbase-admin-ui.toml` (+ `EHRBASE_ADMIN__*` env);
  ships in the quickstart compose as the `ehrbase-admin-ui` service on
  port 3000. The sign-in page is served fully rendered and works with
  JavaScript disabled (the login form posts and redirects natively), and
  offers exactly the methods that can work: the console's configured login
  modes intersected with the authentication schemes the CDR advertises in
  its `WWW-Authenticate` challenge. The console received a full design
  system (semantic design tokens with lockstep light/dark theming, a teal
  brand shared by the widget kit, iconified navigation, breadcrumbed page
  headers, named table headers, empty states, and toast feedback on every
  mutation) and the complete working feature set: query result **export**
  (CSV/JSON, a plain form download that works without WebAssembly),
  **EHR creation** (empty or subject-bound) and **find-by-subject-id**,
  **composition commit** (canonical JSON/XML/FLAT with verbatim CDR
  validation diagnostics) and **edit-as-new-version** (`If-Match`
  concurrency), stored-query **open-in-editor**, shareable URL-driven tab
  state on the detail screens, a template identity card (version,
  languages, UID, archetype id), an **EHRs (cohort)** query shape
  (`SELECT DISTINCT` over the criteria tree), a **Table | Chart** toggle
  on numeric result columns, a version **timeline strip** with a
  `version_at_time` picker on the composition viewer, and a
  **contributions table** on the EHR detail screen. The Directory tab can
  now **create and edit the EHR folder directory** (spec-standard
  POST/PUT with `If-Match`), starting from console-local **folder
  templates** (two built-ins included); the System panel gained
  **repository usage** (per-template composition counts) and a read-only
  **runtime configuration** view backed by the CDR's new redacted
  `GET /admin/config` endpoint (secrets redacted structurally by their
  types — never by key matching). The E2E harness gained an image mode
  (`UI_E2E_IMAGE=1`) that runs the identical journey battery against the
  composed OCI image — including a genuinely end-to-end OIDC journey: the
  quickstart Keycloak now pins one canonical issuer and the dev CDR config
  trusts it via standard OIDC discovery, so a bearer-authenticated console
  session queries the CDR for real. Verified by a Rust-native browser E2E
  journey suite (merge-gating in CI, screenshots published as artifacts),
  including journeys over seeded clinical data and a JavaScript-disabled
  login journey.
- **`GET /ehr/{ehr_id}/contribution` — a paged contribution list** (an
  ehrbase-rs extension; the openEHR REST API defines only the by-uid read).
  Returns the EHR's contributions newest-first as
  `{ "rows": [ { uid, time_committed, committer, change_type } ], "total" }`,
  paginated with `offset` (default 0) and `fetch` (default 20, capped at
  100); **404** for an unknown EHR. Authenticated like the other EHR reads.
- **`DELETE /admin/template/{template_id}` and
  `DELETE /admin/query/{qualified_query_name}/{version}`** — admin deletes for
  operational templates and stored-query versions (ehrbase-rs extensions; the
  openEHR admin API defines only EHR deletes). Same admin gate and
  authorization as the EHR deletes: **204** on success, **404** for an unknown
  id. The template delete additionally returns **409** when a committed
  version still references the template, so a physical delete never orphans
  clinical data.

- **ATNA audit — richer DICOM records**: every audit record now carries the
  concrete operation as a DICOM `EventTypeCode` (login/logout as DCM
  110122/110123; REST operations as their ITS-REST operation id under the
  `openEHR-ITS-REST` code system), and Bearer-authenticated requests record
  the token's `jti` as the minimal token identity (token contents are never
  logged).
- **ATNA audit — FHIR R4 `AuditEvent` rendering (IHE BALP)**: every audit
  record also renders as a FHIR R4 `AuditEvent` conforming to the IHE Basic
  Audit Log Patterns (Patient\*/plain Create/Read/Update/Delete/Query
  profiles, `OAUTHaccessTokenUse.Minimal` token agent, profile claims only
  when genuinely satisfied) — the modern half of the dual ATNA format.
- **ATNA audit — local Audit Record Repository, on by default**: audit
  records are persisted in a new PostgreSQL `audit` schema (append-only;
  strictly outside the EHR content; per-sink delivery stamps; configurable
  `retention_days` with an hourly reaper). Every deployment now gets a
  queryable audit trail out of the box with nothing leaving the node.
- **ATNA audit — RESTful ATNA forwarding (ITI-20 ATX:FHIR Feed)**: opt-in
  `[audit.fhir_feed]` sink POSTs each FHIR `AuditEvent` to an external Audit
  Record Repository; with the local store on, delivery is outbox-driven — an
  ARR outage loses nothing and pending records ship on recovery.
- **ATNA audit — per-sink metrics** (`atna_audit_sent_total{sink=…}`,
  `…send_failed_total{sink=…}`, `atna_audit_rejected_total`,
  `atna_audit_reaped_total`).
- **ITI-81 Retrieve ATNA Audit Event** (`GET /fhir/r4/AuditEvent`): the
  official RESTful-ATNA retrieval — a FHIR search over the local Audit
  Record Repository returning a `searchset` Bundle of the stored `AuditEvent`
  documents. Filters: `date` (`ge`/`le`), `patient`, `agent`, `entity`,
  `outcome`, `action`, plus `_count`/`_offset` paging. Admin-only under
  RBAC; `404` when the local store is disabled.
- **Native TLS + mutual-TLS client authentication** (`[server.tls]`): the
  main listener can terminate TLS itself (TLS 1.2+ floor per IETF BCP 195)
  and demand a verified client certificate
  (`client_auth = "off" | "optional" | "required"`) against an explicit CA —
  the IHE ATNA ITI-19 node-authentication posture. The management listener
  stays plain HTTP.
- A dedicated **Audit trail (IHE ATNA)** book chapter covering the dual
  formats, the sinks, the ITI-81 retrieval, fail-mode semantics, and mTLS.
- **Admin console — the Audit log screen** (`/audit`): browse the CDR's
  ATNA security audit trail through the standard ITI-81 retrieval, with
  URL-driven filters (event-time window, patient, principal, outcome,
  action), pagination, and a per-row view of the full stored FHIR
  `AuditEvent`. Admin-only under RBAC; a disabled local audit store and a
  no-matches filter each render their own first-class state.

### Changed
- The ITS-REST template list (`GET /definition/template/adl1.4`) now reports
  the optional `version` field of each `TemplateMetadata`, derived from the
  template id's version axis (the spec documents the value as "taken from
  `template_id`"); it is omitted when the id carries no version.
- **Audit configuration redesigned: `[atna]` is now `[audit]`**, on by
  default with only the local store active, and sink-structured:
  `[audit.store]` (local repository), `[audit.syslog]` (classic
  DICOM-over-syslog feed; keys `host`/`port`/`transport`/`tls_ca_file`/
  `tls_identity_cert_file`/`tls_identity_key_file` replace the old
  `repository_host`/`repository_port`/`tls_*_path`), `[audit.fhir_feed]`
  (RESTful ATNA). `resolve_subject` now defaults to `true`. A configuration
  still using `[atna]` fails at boot with did-you-mean guidance (strict
  loader; no silent aliasing).
- **Fail-closed auditing got stronger**: with `fail_mode = "closed"` and the
  local store enabled, a store that stops accepting writes makes every
  subsequent auditable operation answer `503 Service Unavailable` until a
  write succeeds again — no un-audited PHI access.

### Fixed
- **ATNA audit — IHE/DICOM conformance corrections** (IHE ITI TF-2 ITI-20 /
  DICOM PS3.15 §A.5.1): the syslog `MSGID` is now the mandated
  `IHE+RFC-3881` (was `IHE+DICOM`); AQL query execution uses the dedicated
  DICOM EventID 110112 "Query" (was 110110); EHR-Extract communication uses
  the direction-coded EventIDs 110106 "Export" / 110107 "Import";
  authentication events (genuine logins and rejected 401/403 attempts) use
  EventID 110114 "User Authentication" with `EventTypeCode` 110122 "Login"
  (were generic Application Activity); and 1xx/3xx responses (e.g. `304 Not
  Modified`) are now recorded as success instead of minor failure.
- **Admin console — icon-only chrome and small polish**: every emoji and
  typographic glyph in the UI is replaced by a proper SVG icon (folder tree,
  status capability badges, remove buttons, disclosure carets, upload
  trigger, pagination arrows); the Audit log screen highlights its own
  navigation entry; and the documentation screenshots now cover every EHR
  detail tab — including the directory tab both before (create from a folder
  template) and after the directory exists — plus the audit raw-record view.

## [3.1.1] - 2026-07-17

### Fixed
- The release pipeline attaches the per-architecture server binary tarballs
  again: since the crate consolidation the binary is produced by the
  `ehrbase-server` package (the executable is still named `ehrbase`), but
  the release asset build still compiled the `ehrbase` platform library and
  failed — v3.1.0 published without binary assets. Container images were
  not affected. Use v3.1.1 for downloadable binaries.

## [3.1.0] - 2026-07-17

### Added
- External terminology providers cache their FHIR operation results
  (`$validate-code`/`$expand`/`$subsumes`/`$lookup`) for a configurable TTL
  (`[terminology.external.providers.<name>] cache_ttl_secs`, default 300 s,
  `0` disables; `cache_capacity`, default 10000) — a validation burst over
  the same codes costs one remote round trip per window instead of one per
  code.
- A new `atna_audit_serialize_failed_total` metric counts ATNA audit records
  dropped because the message failed to serialize, so audit loss is always
  metered.

### Changed
- The FLAT and STRUCTURED (Simplified Formats) layer was rewritten against
  the official openEHR ITS-REST Simplified Formats specification: exact
  node-id generation, per-type attribute suffixes, the full `ctx/`
  vocabulary with its documented defaults, `|raw` embedding, and the
  `|other` open-value-set rules (invalid combinations are now rejected with
  `422` instead of being silently ignored). Unknown field identifiers in a
  simplified payload are now rejected rather than dropped.
- Format selection is done exclusively via the `Accept` and `Content-Type`
  headers on every endpoint that supports the simplified media types
  (`application/openehr.wt.flat+json`, `…wt.structured+json`, and
  `application/openehr.wt+json` for template rendering), with proper
  RFC 9110 q-value negotiation, `406`/`415` answers naming the supported
  formats, and simplified support on CONTRIBUTION payloads
  (`versions[].data`) with the envelope staying canonical.
- Committing a composition in a simplified format now requires the
  `openehr-template-id` request header (`422` without it, previously `400`);
  the undocumented `template_id` query parameter is no longer read.
- Content negotiation is strict everywhere: an `Accept` header that none of
  an endpoint's supported formats can satisfy is answered with `406`
  (previously some JSON-only endpoints leniently returned JSON), and the
  server's own generated OpenAPI now advertises the simplified media types
  on the composition, contribution, and template endpoints.
- Release builds now abort on integer arithmetic overflow instead of
  silently wrapping (`overflow-checks` enabled in the release profile) — a
  corrupted-value class of fault becomes a crash-and-restart instead of
  wrong clinical data.


- The application is consolidated to two library crates plus a thin binary
  (`ehrbase` — the platform, `ehrbase-rest` — the ITS-REST adapter,
  `ehrbase-server` — the binary): the `ehrbase-sm` trait catalog is gone,
  the REST adapter calls the concrete platform service directly, and the
  full configuration tree (`[server]`, `[auth]`, `[authz]`, `[smart]`,
  `[management]`, `[tenancy]`, `[admin]`) is defined in the platform crate.
  The served wire, the `ehrbase.toml` schema, and the container entrypoint
  (`ehrbase`) are unchanged.
- Bundle-backed terminology lookups and template/query validity checks are
  now synchronous in-process calls (no behaviour change on the wire).
- Every versioned write now commits through the single folded
  audit+contribution+version statement even with digest signing enabled
  (the commit instant is read up front with the placement, so the signature
  is computed before any insert); version-tree placement is one read instead
  of three, and contribution commits batch their target pre-reads. Fewer
  round trips per write, identical wire behaviour and stored semantics.
- The OpenAPI documents (the composed `openapi.json` and the twelve Swagger
  spec-selector family documents) and the SMART `.well-known/smart-configuration`
  discovery document are now built once at server startup instead of being
  regenerated on every request. No change to the document content.

### Removed
- The `ehrbase-quirks` cargo feature and its vendor-specific behaviours
  (alternate duplicate-id spelling, the non-standard `|unit_system` /
  `|unit_display_name` quantity suffixes) — the specification-defined
  behaviour is now the only behaviour.

### Fixed
- A tenant-resolution failure (tenant registry unreachable) now fails the
  request with `503` instead of silently serving it under the default
  tenant; unknown tenant keys keep the documented unscoped behaviour and
  are negative-cached.
- Audits for authenticated writes that carry no committal headers are now
  attributed to the authenticated user (Basic username / token subject, with
  the mechanism recorded as the identifier type) instead of the generic
  system identity.
- Multi-tenant deployments now actually run on the tenant-scoped connection
  pool: with `tenancy.enabled = true` every database connection carries the
  request's tenant for the row-level-security policies. Previously the
  binary always built the plain pool, so all requests fell through to the
  default tenant regardless of configuration.
- Multi-tenancy: a connection freshly opened by the pool while serving a
  request (pool growth under load) could miss the tenant stamp and run as
  the reserved default tenant — reads returning nothing and writes landing
  outside the caller's tenant. The tenant-scoped pool now stamps
  `ehrbase.tenant_id` both when a connection is opened and on every
  checkout, so every connection carries the caller's tenant. Deployments
  with `tenancy.enabled = true` should upgrade.
- The demographic APIs (party and relationship writes) now honour the
  `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` committal headers exactly
  as the EHR APIs do — a caller-supplied committer, description, and
  system id are merged into the stored version's audit.
- Direct COMPOSITION create/update/delete now honour the ITS-REST committal
  headers (`openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*`): a
  caller-supplied committer, audit description, change type, lifecycle
  state, signature, and attestations are merged into the stored version
  exactly as on the CONTRIBUTION path (previously the direct paths discarded
  them and always committed server defaults).
- The template store no longer double-reads the OPT XML when generating an
  example for a cold template, and template upload is a single atomic
  statement (the duplicate-check race window is gone).
- The event-outbox publisher declares its AMQP topology only on connect or
  subscription change (previously every poll cycle re-declared each queue),
  and the FHIR outbound emitter parks a persistently failing row after a
  bounded retry budget instead of blocking the stream forever.
- A FLAT/STRUCTURED composition body that parses as JSON but does not conform
  to its target template now returns `422 Unprocessable Entity` instead of
  `500 Internal Server Error` — such an input is client data, not a server
  fault. Output conversion of stored compositions remains a `500` on failure.
- Panicking request handlers and audit fail-closed (`503`) responses now
  carry the standard openEHR `{ error, message }` JSON error body (the audit
  `503` also carries `Retry-After`), instead of a plain-text body.
- A malformed `If-Match` header on a state-changing request is now rejected
  with `400 Bad Request` instead of being silently ignored — an unparseable
  precondition previously ran as if no `If-Match` was sent, opening a
  lost-update window. `If-Match: *` and valid version ids are unaffected.
- Database constraint and serialization/deadlock failures now surface as
  `409 Conflict`, and connection-pool exhaustion under load as `503 Service
  Unavailable` with `Retry-After`, instead of collapsing every database error
  to `500 Internal Server Error`.
- Stored-query and template metadata list/read endpoints no longer silently
  blank a field when a database column fails to decode; a decode failure now
  surfaces as `500` with a real error instead of an empty value.

## [3.0.3] - 2026-07-16

### Changed
- The served OpenAPI documents now categorize operations the way the
  official ITS-REST reference documents do: standard-group operations are
  tagged by resource (EHR, EHR_STATUS, COMPOSITION, DIRECTORY, CONTRIBUTION,
  ITEM_TAG; PERSON, AGENT, GROUP, ORGANISATION, ROLE, VERSIONED_PARTY;
  ADL 1.4, ADL 2, Query) instead of one flat tag per API group, and the
  Swagger UI spec selector offers one document per API family — the five
  standardised openEHR groups and the seven server-extension families —
  plus the complete composed surface, all filtered from the server's own
  generated document.

### Fixed
- Duplicate-template-id fixture resolution in the validation corpus test is
  now deterministic (sorted path order) instead of OS-dependent `read_dir`
  order, fixing a Linux-only CI failure.

## [3.0.2] - 2026-07-15

### Changed
- The benchmark instrument measures both comparison stacks under a fairer,
  more deterministic protocol: the databases get a 1 GB `/dev/shm` floor
  (Docker's 64 MB default starved PostgreSQL's parallel workers mid-run),
  maintenance debt is settled with `VACUUM ANALYZE` after seeding and
  between ladder rungs (autovacuum no longer lands inside measured
  windows), the ladder drains in-flight backlog between rungs, and the
  measured cold start no longer includes building the ehrbase-rs container
  image. Ladder output prints latencies in magnitude-appropriate units
  (µs/ms/s), and the generated comparison page reports clinical events per
  minute beside request rates.
- **Configuration is now one `ehrbase.toml`.** The whole server is configured
  by a single TOML file (sections `[server]`, `[db]`, `[log]`, `[telemetry]`,
  `[auth]`, `[authz]`, `[admin]`, `[tenancy]`, `[smart]`, `[management]`,
  `[signing]`, `[query]`, `[events]`, `[fhir]`, `[terminology]`,
  `[multimedia]`, `[atna]`, `[subject_proxy]`), discovered from `--config`,
  `EHRBASE_CONFIG`, `./ehrbase.toml`, or `/etc/ehrbase/ehrbase.toml`. Every
  `EHRBASE_*` environment variable is now a mechanical per-key override:
  `EHRBASE` + the TOML path, upper-cased, with `__` between every segment
  including after the prefix
  (e.g. `EHRBASE__DB__MAX_CONNECTIONS`, `EHRBASE__AUTH__OIDC__ISSUER`). This
  replaces the previous ~14 independent per-subsystem loaders and their
  several env-name grammars. **Old spellings are not aliased** (greenfield —
  nothing is deployed to migrate): a pre-redesign variable fails at boot with
  the exact uniform replacement suggested (e.g. `EHRBASE_DB_MAX_CONNECTIONS`
  → "did you mean `EHRBASE__DB__MAX_CONNECTIONS`?"). `DATABASE_URL` and
  `RUST_LOG` remain permanent conventional aliases. New `ehrbase config
  default` prints an annotated template and `ehrbase config check` validates a
  config (and prints the effective, secret-redacted result) without a
  database. The compose stack, Helm chart, and docs all move to the new file +
  spellings; the PostgreSQL-init container variables `EHRBASE_DB_USER` /
  `_PASSWORD` / `_NAME` were renamed `PG_INIT_USER` / `_PASSWORD` / `_DB` so
  they no longer collide with the server's reserved `EHRBASE_` namespace.

### Removed
- The nine per-subsystem `EHRBASE_*_CONFIG` file pointers
  (`EHRBASE_REST_CONFIG`, `EHRBASE_AUTHZ_CONFIG`, `EHRBASE_ATNA_CONFIG`,
  `EHRBASE_SIGNING_CONFIG`, `EHRBASE_EVENTS_CONFIG`,
  `EHRBASE_FHIR_OUTBOUND_CONFIG`, `EHRBASE_MULTIMEDIA_CONFIG`,
  `EHRBASE_VALIDATION_CONFIG`, `EHRBASE_MANAGEMENT_CONFIG`,
  `EHRBASE_SUBJECT_PROXY_CONFIG`): merge each file's contents into the single
  `ehrbase.toml` under its `[section]`.
- `EHRBASE_REST_AUTH__ADMIN_SCOPE`: subsumed by `authz.rbac.admin_role`.

### Fixed
- Unknown or misspelled configuration is now rejected at boot with a
  did-you-mean suggestion (and the `file:line` for a file key) — previously a
  typo'd TOML key or `EHRBASE_*` variable was silently ignored, so a
  not-applied security setting could pass unnoticed.
- The documented `EHRBASE__SUBJECT_PROXY__SYSTEMS__<name>__BASE_URL` env form
  now actually binds — the old loader stripped the prefix such that this
  spelling was dead, so subject-proxy systems could only be set via a file.
- Unparseable `[query]` values (`query.plan_cache_capacity`, `query.timeout_ms`)
  now error at boot instead of silently falling back to defaults.
- The Swagger UI works again and now documents the **complete server
  surface** from one natively generated OpenAPI document. `…/rest/swagger-ui`
  previously entered an infinite redirect loop (the UI's trailing-slash
  redirect fought the server's path normalization) and its OpenAPI document
  was an empty stub. The UI now loads directly (documentation URL corrected to
  `/ehrbase/rest/swagger-ui`), and its spec selector has a single entry,
  `ehrbase-rest`, generated by the server itself (`utoipa-axum`, one
  `#[utoipa::path]` handler per operation, so route and documentation cannot
  drift): every ITS-REST API group (EHR, COMPOSITION, CONTRIBUTION, DIRECTORY,
  DEMOGRAPHIC, DEFINITION, QUERY, ADMIN) plus the server's own extensions
  (terminology, PARTY_RELATIONSHIP, event-subscription, multi-tenancy, FHIR
  connector) and its operational endpoints (status/health, management, SMART
  discovery, the OpenAPI endpoints). No vendored OpenAPI is served. The
  document also declares the server's **configured** authentication scheme so
  the "Authorize" dialog and per-endpoint padlocks match the running server:
  HTTP Bearer (JWT) when OIDC is configured, otherwise HTTP Basic, and none
  when authentication is disabled — never both at once.

## [3.0.1] - 2026-07-14

### Added
- The server now prints an ASCII-art startup banner to stdout before the
  structured startup logs: the `EHRbase-rs` wordmark, the running version, the
  maintainer credit (Ruben Talstra), the project URL, and the load-bearing
  spec/platform pins (openEHR RM 1.2.0 · ITS-REST 1.0.3 · AQL 1.1 ·
  PostgreSQL 18). The banner is suppressed under JSON logging
  (`EHRBASE_LOG_FORMAT=json`) so machine log consumers see only structured
  lines.
- AQL queries are now planned once and cached: a repeated ad-hoc or stored
  query text reuses its lowered plan instead of re-parsing and re-analysing on
  every execution, while per-request parameter values, `fetch`/`offset`
  paging, and EHR scope still bind independently. Queries that resolve
  terminology (`matches TERMINOLOGY(…)`) are never cached, so their expansion
  is always current. New configuration knob
  `EHRBASE_QUERY__PLAN_CACHE_CAPACITY` (default `256`; `0` disables the cache)
  bounds how many distinct plans are held, and a new `aql_plan_cache_events_total`
  metric (`event` = `hit`/`miss`) reports cache activity.


- Storage migration `0008`: a promoted `context_start timestamptz` column on
  COMPOSITION root node rows (backfilled from stored data, partially
  indexed), plus the fail-safe `ext.openehr_timestamp` conversion function.
  The AQL engine reads the indexed column for
  `ORDER BY`/`WHERE` on `c/context/start_time/value` — the measured
  patient-dashboard hot path — instead of re-extracting JSONB per candidate
  row; results are unchanged, including NULL placement and the verbatim
  projected value.
- Overload backpressure: the REST server now caps the number of API requests
  it handles concurrently and sheds the excess immediately with
  `503 Service Unavailable` + `Retry-After: 1` instead of queueing every
  request until it runs out of memory. Under sustained offered load beyond
  database capacity the server now degrades with clean errors rather than
  being killed. The cap is configurable via `EHRBASE_REST_MAX_IN_FLIGHT`
  (concurrent requests, not per second; default 256, raise for
  high-throughput deployments; `0` disables shedding). The `/status`, health,
  and discovery
  endpoints are never limited, so operators can always probe an overloaded
  server. (No openEHR spec governs overload behaviour; the `503` follows
  RFC 9110 §15.6.4.)
- Conformance framework (`tools/conformance`) redesigned and rewritten from
  the openEHR CNF component up (W-10). It now assesses **any** openEHR CDR:
  point it at a deployed server (`scripts/conformance.sh` with
  `CONF_SUT=byo CONF_BASE_URL=…`, or the CLI's `--sut byo --base-url …`) and
  receive the full spec-cited artefact set — `results.json`, a conformance
  report, a Conformance Statement, a Conformance **Certificate** (a
  machine-computed framework assessment, explicitly not an official openEHR
  certification), and badges, written per SUT. Upstream EHRbase (Java) is a
  built-in target (`CONF_SUT=ehrbase-java`) with a committed fairness
  register; a cross-SUT comparison matrix can be rendered from two or more
  runs (`conformance compare`). Assertions carry a **spec-edition ladder**:
  the runner tries the newest edition form first (weak `W/"…"` ETags,
  RM 1.2.0 wire) and steps down to Release-1.0.3-era forms, reporting the
  satisfied edition level per case instead of failing a CDR on edition
  deltas; ehrbase-rs CI runs stay pinned to the development edition so the
  ladder can never mask a regression.

- AQL: `OR`-combined `CONTAINS` expressions now execute (previously rejected
  as unsupported), including nested `AND`/`OR`/`NOT` containment trees, and
  `NOT CONTAINS` accepts compound operands.
- ATNA auditing: EHR-Extract export and import operations now emit audit
  events (object class `Extract`) when auditing is enabled.
- Multiple folder hierarchies per EHR (`EHR.folders`): beyond the
  `/directory` hierarchy, additional root `FOLDER`s can be committed through
  the CONTRIBUTION endpoint, each versioned independently. The EHR resource
  now carries the `folders` reference list (creation order) and `directory`
  (always its first member); EHR extract import and admin dump/load carry
  the hierarchies too. The `/directory` endpoints behave exactly as before.
- `ehr:` URI support: `DV_EHR_URI` values are parsed against the full
  openEHR `ehr:` grammar (EHR / top-level structure by uid or exact version
  id / interior item paths, absolute and relative forms), and the server can
  resolve local `ehr:` references internally (e.g. LINK targets). openEHR
  path processing now also supports `//` path patterns and 1-based
  positional predicates in stored-structure navigation (AQL is unchanged —
  its grammar defines neither).
- `EHR_ACCESS` access-control is now enforced. The spec-mandated,
  change-controlled `EHR_ACCESS` object of an EHR (RM ehr §EHR_ACCESS Class)
  is the foundational access-decision layer, evaluated after authentication
  and before dispatch on every EHR-scoped route; the enterprise RBAC/ABAC
  layers compose on top of it. Its `settings` use the
  `ehrbase.access_control.v1` scheme (`docs/design/ehr-access-scheme.md`):
  a `default_access` (`open`/`restricted`) with a `user:`/`role:` access
  list gating the EHR, per-Composition privacy-level ceilings on Composition
  reads, and a gate-keeper that guards changes to the settings themselves
  (`403 Forbidden` on a denial). Every existing EHR keeps working — the
  default (no settings) is open.
- Client-supplied CONTRIBUTION `uid`s are honoured on commit when unused
  (`409 Conflict` when already in use; previously silently ignored).
- `Prefer: resolve_refs` is honoured on contribution reads: the
  CONTRIBUTION's `versions` are returned as full `ORIGINAL_VERSION`
  objects instead of `OBJECT_REF`s (ITS-REST representation negotiation).
- AQL single-row functions now execute: `LENGTH`, `SUBSTRING`, `POSITION`,
  the string `CONTAINS`, `CONCAT`/`CONCAT_WS`, `ABS`/`MOD`/`CEIL`/`FLOOR`/
  `ROUND`, and `CURRENT_DATE`/`CURRENT_TIME`/`CURRENT_DATE_TIME`/`NOW`/
  `CURRENT_TIMEZONE` (QUERY master03 §Functions).
- AQL `TERMINOLOGY()` Boolean value expressions
  (`TERMINOLOGY('validate'|'subsumes', …) = true`) and terminology-URI
  `matches` operands (`matches { terminology://… }`) are now evaluated
  through the terminology service (previously typed rejects).
- AQL archetype predicates now honour archetype-specialisation subsumption:
  a query naming a parent archetype (e.g.
  `[openEHR-EHR-OBSERVATION.laboratory.v1]`) also matches data created with
  any specialisation child (e.g. `…laboratory-glucose.v1`), scoped to the
  same RM entity and major version (BASE architecture_overview master10
  §Design-time Relationships; AM master07 §Querying). Non-HRID predicates
  (at/id-codes) keep exact case-folded matching.
- **Version-tree branching and merge provenance** (RM common master06
  §Version tree / §Distributed versioning / §Version Merging). Branch
  version ids (`trunk.branch.version`) are now first-class on every
  surface: modifying a version that was imported from another system forks
  a branch with the local `creating_system_id` (the spec's mandated rule
  for local modifications of copied versions) while the imported trunk
  version stays the container current; branch tips are continued,
  superseded, read, exported, and re-imported like any version; the
  container current / `LATEST_VERSION` (including in AQL) is the latest
  *trunk* version. `ORIGINAL_VERSION.preceding_version_uid` is now stored
  at commit (previously synthesized) and `other_input_version_uids` (merge
  provenance) is accepted on the CONTRIBUTION wire, preserved on import,
  and served on read. The `vo_version` storage carries the version tree in
  explicit columns with per-lineage temporal non-overlap constraints and
  the spec's global version-identity uniqueness tuple.

### Changed
- Basic-auth verification no longer re-runs the Argon2 password hash on
  every request: verified credentials are cached (as a SHA-256 digest,
  never plaintext) for `EHRBASE_REST_AUTH__VERIFIED_CACHE_TTL_SECONDS`
  (default 60 s; `0` disables), and cache misses hash on a background
  thread. At load this removes roughly a full CPU core of per-request
  hashing.
- Composition create/update responses are built from the commit result
  instead of re-reading the just-written document from the database — one
  connection acquisition and two queries fewer per write; when version
  signing is disabled the server also no longer rebuilds the full document
  it would only have signed. Response bodies and headers are unchanged.
- Storage: the version table's two GiST exclusion constraints and two
  speculative JSONB indexes on the node table (a GIN over every fragment and
  a magnitude expression index — no query the engine generates could use
  either) were removed; version-validity non-overlap is unchanged and held
  by construction (one open row per lineage via unique indexes, atomic
  close-then-insert writes, and an overlap audit on archive load). This
  removes the dominant per-commit index-maintenance and lock-contention
  costs on the write path.
- Connection-pool defaults changed: `EHRBASE_DB_MAX_CONNECTIONS` 10 → 20,
  `EHRBASE_DB_MIN_CONNECTIONS` 0 → 2, and the per-checkout liveness ping is
  disabled (a broken connection is detected by its first statement).
  `TCP_NODELAY` is now set on accepted sockets, removing Nagle-induced
  latency on small responses.
- Composition commits make fewer database round trips: the audit and
  contribution rows are written in one statement, and the create-path EHR
  existence + modifiability gates are one read instead of two. Error
  behaviour is unchanged (a missing EHR is still `404` before a
  non-modifiable `409`).
- The transactional event outbox is no longer written on every commit when no
  eventing consumer is configured. The per-commit `event_outbox` row (and its
  envelope serialization) is now written only when the AMQP publisher
  (`EHRBASE_EVENTS_ENABLED`) or the FHIR outbound emitter
  (`EHRBASE_FHIR_OUTBOUND_ENABLED`) is enabled. Consequence: the outbox
  records commits made while a consumer is enabled (at-least-once, even with
  zero bound subscribers — the gate is the boot-time config, not the current
  subscriber set); commits made while every consumer was off are not
  back-filled if eventing is later enabled.
- IHE ATNA login ("Application Activity") records now mark genuine
  authentication events rather than every authenticated request. A login
  record is emitted only when the request actually verified credentials (a
  Basic verified-credential cache miss); a cache hit continues an established
  session and a Bearer request authenticated out of band at the OIDC provider,
  so neither mints a per-request login record. Rejections (401/403) are still
  always audited, and login records remain off by default
  (`EHRBASE_ATNA_SUPPRESS_LOGIN_EVENTS`, default `true`).
- Per-EHR `EHR_ACCESS` access-settings are cached as default-open at EHR
  creation, so the access gate's first check on a freshly created EHR no
  longer costs a database lookup (a hospital-day workload creates EHRs
  constantly). Importing an `EHR_ACCESS` version into an existing EHR now
  evicts that cache entry, so the access decision reflects the imported
  policy immediately.
- Composition validation is substantially faster with identical outcomes:
  the RM-invariant pass validates each node directly against the
  spec-generated Reference Model instead of deserializing every node into
  its typed struct (falling back to the typed path for anything it cannot
  vouch for), the archetype-constraint walk reuses constraint paths parsed
  once per cached WebTemplate instead of re-parsing them on every node
  visit, and validation error messages are byte-for-byte unchanged
  (equivalence is pinned by tests across the full corpus). Measured
  end-to-end: a fully populated International Patient Summary validates in
  well under half its previous time.


- Version lifecycle states are now enforced as a state machine (RM common
  §Version Lifecycle): a commit whose `lifecycle_state` is not a legal
  transition from the preceding version's state (for example
  `incomplete` → `inactive` without completing first) is rejected `422`.
- Template identifiers now compare case-insensitively (case-preserving):
  lookups accept any casing and uploading a case-variant duplicate is a
  `409` conflict, backed by a unique index (new migration).
- AQL `MIN`/`MAX` aggregate over non-numeric leaves (text, dates, times)
  now compares type-appropriately instead of forcing a numeric cast, and
  mixed-type leaf comparison dispatches numerically for numbers.
- Contribution commits now verify the target EHR exists (`404` otherwise)
  and honour the `EHR_STATUS.is_modifiable = false` write guard and
  versioned-composition invariants on every path, including
  CONTRIBUTION-wrapped commits. Re-creating an existing directory (a folder
  hierarchy with the same root archetype and name) via a CONTRIBUTION is a
  `409` conflict; a hierarchy with a distinct root remains a new
  `EHR.folders` member.
- EHR-index errors now carry the precise SM error names
  (`ehr_id_does_not_exist`, `subject_id_does_not_exist`) instead of a
  generic not-found.
- Contribution retrieval now lists versions affected by `attestation`-only
  items alongside committed versions for demographic contributions,
  matching the EHR-scoped behaviour.
- SMART App Launch resource-server support (openEHR SMART App Launch
  framework, development edition), config-gated and off by default
  (`EHRBASE_REST_SMART__*`): the `/.well-known/smart-configuration`
  discovery document, the full resource-scope grammar
  (`compartment/resource.permission` with `*`/`**`/`ns::*` patterns), and
  scope + launch-context (`ehrId`→patient) enforcement composed after
  RBAC/ABAC.
- Subject Proxy Service completed (SM `I_SUBJECT_PROXY_SERVICE`): variables
  are now tracked over time (a persisted sample history per variable),
  `currency` freshness is evaluated (fresh samples are served without
  re-querying; data-set registration tightens currency), data-set local
  aliases resolve on reads, `using_app_ids` lifecycle drops empty data
  sets, and frames execute with primary→fallback semantics. New FHIR frame
  executor (config-gated named systems, `EHRBASE_SUBJECT_PROXY__*`) lets
  variables be populated from FHIR R4 servers; manual variables gain a
  notification input channel.
- System API `OPTIONS /` conformance manifest rebuilt: reports the live
  mounted endpoint groups, a single provenance source (the tested
  development-edition ITS-REST identity), and configurable identity fields
  (`EHRBASE_REST_SYSTEM__*`); also mounted at the API base path.
- Item tags via headers (`openehr-item-tag`/`openehr-version-item-tag`):
  accepted on EHR-group and demographic writes and echoed on responses.
- Query API: multi-EHR scoping (`ehr_ids` set), an honest
  `ehr_id_does_not_exist` (404) for a well-formed absent EHR id, a weak
  `ETag` on `RESULT_SET` responses, parameter-substituted
  `meta._executed_aql`, and an optional query execution timeout
  (`EHRBASE_QUERY__TIMEOUT_MS`) mapped to `408`.
- Definition API: template list filtering (`template_id` glob, `concept`,
  `version`) and pagination are honoured; stored-query `query_type` is
  read with an honest unsupported-formalism rejection; ADL1.4 uploads
  return the JSON `TemplateIdentifier` under `Prefer: return=identifier`.
- FLAT/STRUCTURED (Simplified Formats, now STABLE): the `_`-prefixed
  optional RM attribute family (`_uid`, `_link`, `_feeder_audit`,
  `_null_flavour`, `_mapping`, `_normal_range`, participations, work-flow
  ids, …) round-trips in both directions; `|raw` canonical-JSON embedding
  on write; complete quantity/date-time/multimedia leaf attribute tables;
  `|other` open-value-set rules enforced.
- Development-edition ITS-REST protocol adopted (the server's tested
  contract identity, now reported consistently as such): `ETag` response
  headers carry the weak `W/"…"` indicator (bare quoted values are still
  accepted on `If-Match`); committal metadata uses the lowercase
  `openehr-version` / `openehr-audit-details` value-form headers (the
  deprecated `openEHR-VERSION.*` dotted spellings remain accepted) and a
  client-supplied `system_id` is merged into the commit audit; `Location`
  is emitted only on resource creation (no longer on reads/deletes);
  `Preference-Applied` echoes the honoured `Prefer`; `405`/`501` render
  the openEHR error body.
- Demographic DELETE follows the published Demographic API: the preceding
  version id rides in the path; a stale id yields `409` (with the latest
  version `ETag`), an already-deleted party `400`.
- Admin `DELETE /admin/ehr/all` follows the published Admin API: `204`
  with no body, and an absent `ehr_id` parameter now means delete ALL
  EHRs.
- FLAT duplicate node-name suffixes default to the specification form
  (`name_1`); the Better-compatible form (`name2`) is available behind the
  `ehrbase-quirks` feature.
- The `ehrbase-rest` and `ehrbase-sm` crates were restructured
  specification-first (one folder per ITS-REST spec / SM chapter, all
  spec-silent surfaces quarantined under `extensions/`) — no route
  changes beyond those listed here.
- `PUT …/composition/{uid_based_id}` rejects a body whose
  `COMPOSITION.uid` does not identify the versioned object addressed by
  the path (`400`).
- AQL semantic analysis is stricter per QUERY master03: duplicate FROM
  variable names reject, variable references are case-insensitive,
  `LIMIT 0`/negative `OFFSET` reject, `SUM`/`AVG` over non-numeric paths
  reject, scalar-function arity is validated, and `LIKE` `\*`/`\?`
  escapes now match the literal characters.
- OPT 1.4 template upload enforces the AOM 1.4 constraint-model invariants
  (attribute existence bounds, single-attribute occurrences, archetype-id
  well-formedness and root-type match, slot identifier validity,
  internal-reference target paths, constraint-reference definedness,
  boolean satisfiability, assumed-value validity, temporal and duration
  constraint-pattern validity, duplicate code-list codes) — invalid
  templates are rejected with `400` carrying the AOM rule code.
- ADL2 artefact upload (`I_DEFINITION_ADL2`) now validates sources against
  the registration-decidable AOM2 catalogue (mandatory sections, header
  versions, root type/node-id rules, specialisation depth, terminology
  language consistency, code definedness, value-set validity, term-binding
  keys) instead of a header-only probe — invalid sources are rejected with
  `422` carrying the AOM2 rule code.
- **Stricter spec-mandated validation** on the commit path: a client
  `AUDIT_DETAILS` with an empty `system_id`, a committer
  `PARTY_IDENTIFIED`/`PARTY_RELATED` with no identity, an empty committer
  name, or a `PARTY_RELATED.relationship` outside the openEHR
  `subject_relationship` group is now rejected with 422 (previously
  accepted, or surfaced as a 500 DB error); a non-root RM node carrying
  `archetype_details` violates `LOCATABLE.Archetyped_valid` and is
  rejected; EHR-Extract `versions[]` members with a `_type` other than
  `ORIGINAL_VERSION` are rejected on import.
- AQL `VERSION` `uid` values are now built from each version's stored
  `creating_system_id` and version-tree id, not the server's live
  `system_id` configuration.
- The `ehrbase-rs-postgres` image now pre-creates the layered group roles
  (`ehrbase_migrator`, `ehrbase_app`, `ehrbase_reader`), so Compose/dev
  deployments get the same least-privilege grant topology as hardened
  deployments instead of `roles absent` startup notices. Existing data
  volumes keep working; recreate the volume (or create the roles once by
  hand) to pick the grants up.
- Public documentation website at <https://rubentalstra.github.io/ehrbase-rs/>:
  a product landing page, a versioned user guide (frozen per release, `dev`
  tracking `develop`), and an offline OpenAPI endpoint reference covering all
  seven openEHR API groups. Built from `website/` and deployed by CI, with
  link-check and OpenAPI-drift gates.

### Fixed
- The composition validator no longer falsely rejects templates that use the
  same archetype more than once under one container, differentiated by name:
  each instance is now routed to the sibling constraint whose name it
  satisfies, instead of being checked against the first same-archetype
  sibling's overlay. Cross-contaminated content (a child from one overlay
  placed in the other-named instance) is still rejected.
- Template example generation (`GET …/example`) at `detail_level=medium` and
  `complete` no longer produces an empty composition for templates whose
  content is entirely optional: `medium` now returns a fully-populated
  single-instance committable example (honouring temporal patterns,
  C_DURATION field patterns, media-type code lists, and container
  cardinality bounds), and `complete` additionally demonstrates a second
  occurrence of repeating nodes. `required` (the default) is unchanged.
- AQL `SELECT c/uid/value` (and `c/uid`) on a COMPOSITION — or any
  versioned-object root — now returns the server-assigned
  `OBJECT_VERSION_ID`, version-correct under `LATEST_VERSION` and
  `ALL_VERSIONS`. It previously returned `null` because the uid was
  injected only on REST reads, never into stored data. (QUERY master03
  lists `COMPOSITION.uid.value` as a normative identified path.)
- Composition commits against an already-seen template no longer re-read the
  stored OPT from the database on every commit — the built WebTemplate cache
  is now consulted first (measured: 10,206 redundant reads in a 120 s load
  window, the #2 database statement by total time). Deleting a template now
  also evicts it from that cache, so a commit racing a delete gets the
  correct `422` ("template not known") instead of a foreign-key `500`.


- Template example generation (`GET /definition/template/adl1.4/{id}/example`)
  now honours the template's structural constraints: a missing mandatory
  ENTRY structure (e.g. `ACTION.description`) is synthesized with the
  template's constrained node (its RM type, `archetype_node_id`, and name)
  instead of a blind `at0001` placeholder, so generated examples validate
  and commit against the same template. Surfaced by the official openEHR
  CKM **International Patient Summary** template; probed by the new
  conformance case ECC-TPL-017 (example → commit round-trip).
- Template list endpoints no longer ignore filter and pagination
  parameters.
- The conformance manifest and `/rest/status` no longer misreport the
  implemented ITS-REST edition as `1.0.3`.
- Contribution commits: a creation version against an already-existing
  object, and a modification/deletion/attestation whose
  `preceding_version_uid` names an object the server does not hold, now
  return `400` (the contract's modification-type-mismatch scope) instead of
  `422`/`404` — on `POST /ehr/{ehr_id}/contribution`, `404` is reserved for
  an unknown `ehr_id`.
- Versioned-object reads (`GET …/versioned_composition`,
  `…/versioned_ehr_status`, versioned directory) now emit the concrete RM
  class (`VERSIONED_COMPOSITION` / `VERSIONED_EHR_STATUS` /
  `VERSIONED_FOLDER`) in `_type`, not the abstract `VERSIONED_OBJECT`.
- Demographic API: `If-Match` preconditions now verify the full
  `OBJECT_VERSION_ID` (previously only the version-tree number, which
  accepted phantom versions); relationship delete now honours the same
  `If-Match` preconditions as party delete; demographic `ETag`s are emitted
  in the weak form (`W/"…"`).

## [3.0.0] - 2026-07-11

First public release of **EHRbase-rs** — a pure-Rust openEHR Clinical Data
Repository. Version numbering starts at 3.0.0: this project began as a fork
of EHRbase (Java, 2.x line) and is released as its next-generation successor;
inherited upstream tags/releases were removed from the fork. Published as a
**pre-release**: the platform is feature-complete and conformance-verified,
but has not yet run in production.

### Added
#### openEHR platform
- openEHR REST API (ITS-REST 1.0.3): EHR, EHR_STATUS, COMPOSITION,
  DIRECTORY/FOLDER, CONTRIBUTION, QUERY, DEFINITION (ADL 1.4 + ADL2), admin
  and management surfaces, with canonical JSON **and** XML content
  negotiation. The wire contract is generated from the official openEHR
  OpenAPI/BMM/XSD models with a CI drift gate.
- AQL 1.1 query engine: typed path analysis over a spec-generated Reference
  Model compiled to PostgreSQL SQL; `LATEST_VERSION` **and** `ALL_VERSIONS`;
  terminology-backed `TERMINOLOGY()` expansion; stored parameterised queries.
- Full change-control semantics: contribution-atomic commits, indelible
  temporal version history (PostgreSQL 18 `WITHOUT OVERLAPS`), logical
  delete, attestations, per-version digital signatures (RFC 8785),
  point-in-time reads.
- Templates and validation: OPT 1.4 ingestion with artefact validity
  checking (AOM2 codes), WebTemplate / FLAT / STRUCTURED simplified formats,
  deep archetype-constraint validation on every commit.
- EHR Extract and messaging (SM I_EHR_EXTRACT/I_MESSAGE/I_TDD): whole-EHR
  export/import preserving distributed version identity, EHR cloning, TDD
  import.
- Demographics: versioned party store (PERSON, ORGANISATION, GROUP, AGENT,
  ROLE) with relationships.
- Terminology: the bundled openEHR terminology plus pluggable external FHIR
  terminology servers (validate / expand / subsume).
- Conformance instrument: the ECC runner executes the full catalogue (341
  cases, JSON + XML) against the composed server and computes profile
  verdicts — **CORE: PASS · STANDARD: PASS · OPTIONS: OBTAINED**, generating
  the Conformance Statement + Certificate.

#### Integration
- Change events: transactional outbox publishing every contribution commit
  to AMQP/RabbitMQ — at-least-once, per-EHR ordered, PHI-free envelopes,
  server-side filterable subscriptions (off by default).
- FHIR R4 connectors: mapping-driven inbound ingestion (validated
  compositions with FEEDER_AUDIT provenance), a read façade over AQL, and
  event-driven outbound resource emission (off by default).
- S3 multimedia externalization: threshold-based content-addressed offload
  of DV_MULTIMEDIA to any S3-compatible store with sha-256 integrity
  verification; SeaweedFS supported out of the box (off by default).

#### Security & operations
- Authentication: HTTP Basic (argon2) and OAuth2/OIDC bearer (Keycloak,
  Active Directory, any standards-compliant IdP).
- Authorization: RBAC plus ABAC via the embedded Cedar policy engine or a
  remote PDP.
- Multi-tenancy: each tenant an isolated logical openEHR system with its own
  `system_id`, enforced by PostgreSQL row-level security (off by default —
  single-tenant mode is unchanged).
- IHE ATNA system log: DICOM audit messages over (TLS) syslog with
  build-time operation coverage.
- Observability: structured logs, OpenTelemetry traces, Prometheus metrics,
  health probes; identified data never enters telemetry.
- Layered database roles (migrator / writer / reader) with a hardened
  PostgreSQL baseline.

#### Deployment
- Docker Compose stack (server + PostgreSQL 18) with an optional Grafana
  LGTM observability overlay.
- Distroless, non-root, shell-less multi-arch container images (amd64 +
  arm64) on GHCR.
- Helm chart with security-hardened defaults (non-root, read-only rootfs,
  seccomp, default-deny NetworkPolicy) and golden-render validation.


[unreleased]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.9.0...HEAD
[3.9.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.8.0...v3.9.0
[3.8.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.7.0...v3.8.0
[3.7.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.6.0...v3.7.0
[3.6.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.5.0...v3.6.0
[3.5.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.4.0...v3.5.0
[3.4.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.3.0...v3.4.0
[3.3.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.2.0...v3.3.0
[3.2.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.1.1...v3.2.0
[3.1.1]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.1.0...v3.1.1
[3.1.0]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.3...v3.1.0
[3.0.3]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.2...v3.0.3
[3.0.2]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.1...v3.0.2
[3.0.1]: https://github.com/rubentalstra/ehrbase-rs/compare/v3.0.0...v3.0.1
[3.0.0]: https://github.com/rubentalstra/ehrbase-rs/releases/tag/v3.0.0
