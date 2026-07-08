# The ehrbase-rs Conformance Catalogue (ECC) — design (v3.1: our own framework)

> **v3.1 ownership inversion (owner directive, 2026-07-08):** the primary test
> system is **ours** — our numbering, our taxonomy, our catalogue. The official
> openEHR CNF corpus is **reference material**: upstream is frozen/unmaintained
> (dormant since 2024-08, stub chapters never finished), so it cannot be the
> key system of a living test engine. Instead, every official unit (schedule
> case, truth-table row, robot case) must be **traced** by at least one ECC
> case or excluded with a reason — guards prove we dropped nothing the
> official corpus defines, while the catalogue itself is free to exceed it.

- **Status:** v3 accepted 2026-07-08 (owner directive: complete redesign) —
  **supersedes v2's registry-only model**; implementation in progress on
  `claude/cnf-hardening` (`docs/plans/s2-phase-05-cnf-engine-rewrite.md`).
  The v1/v2 crate (`crates/ehrbase-conformance`, PR #27) is the substrate being
  rewritten in place, not discarded: SUT modes, client, reports, and the
  transcribed suites carry forward under the new case model.
- **Oracle:** the vendored spec corpus at `docs/specs/openehr/` — CNF schedule
  + Robot suite (`specifications-CNF` @ `33251d2a`, verified byte-identical to
  upstream `master` HEAD 2026-07-08; upstream dormant since 2024-08),
  ITS-REST @ `Release-1.0.3` (`4aec22de`; still the latest published release —
  no 1.0.4/1.1 exists; the 290 unreleased master commits are next-release work
  we correctly do not target), QUERY/AQL 1.1, RM 1.2.0, TERM 3.1.0, ITS-JSON.
- **Related:** `docs/design/version-signing.md`, `docs/enterprise/access-control.md`,
  `docs/plans/s2-phase-04-cnf-hardening.md` (the findings backlog this engine
  feeds), `docs/spec-audit/SPEC_AUDIT.md`.

---

## 1. Goal

Make *"EHRbase-rs is conformant to the openEHR platform specifications"*
provable, reproducible, and public — at **data-set granularity**, not
case-heading granularity. The v2 framework enforced the 324 schedule case
headings; this redesign makes the engine's case base **exhaustively derived
from every executable artifact in the pinned official corpus**, each executable
unit individually identified, individually reported, and individually gated:

> A conformance claim is a pure function of a run over an enumerable,
> spec-derived case universe. Nothing hand-counted, nothing silently dropped.

## 2. Why v2 was not enough (the redesign's forcing facts)

Full inventories: 2026-07-08 recon (schedule, Robot, crate, freshness). The
load-bearing facts:

1. **The schedule's real size is ~1,600 tests, not 324.** The 324 headings
   carry normative data-set/truth tables: the content chapters (master15–17.7)
   alone define **1,371 accepted/rejected rows** (17.4 date_time: 604, 17.3
   quantity: 406, 15: 108, 16: 138, 17.1: 30, 17.2: 24, 17.6: 23, 17.7: 38);
   the functional chapters add ~70+ enumerated rows (master06's 16-row valid
   EHR_STATUS matrix + 5 invalid classes, master08's 12-row `one_commit`
   table + 15-row EHR_STATUS matrix + 3-row FOLDER table + A–D multi-version
   tables, master09's 14 `$path/$result` rows, master04's per-operation OPT
   data-set lists). v2's parser extracted only headings; a case silently
   dropping half its rows still counted as implemented.
2. **The Robot suite is the concrete oracle and was used only as fixtures.**
   464 declared robot cases across 10 service dirs (template-driven files fan
   out further), with exact status-code and body-path assertions — the
   schedule's own `Test runners` cells point at them, and our spec-adherence
   rule says the CNF test case wins where prose is abstract. None of the 464
   were gated; only their fixture files were consumed.
3. **The ITS-REST contract is enumerable and was not enumerated.** The
   vendored OAS (the same corpus `emit-rest` consumes) documents every
   operation and every response code. Nothing verified that each documented
   (operation, status) pair has conformance evidence.
4. **Five schedule chapters are stubs upstream** (05, 10, 11, 12, 13 — 68
   placeholder cases) and 17.5 is empty. Upstream is dormant: these holes are
   permanent, so the official corpus itself tells us to fill them from the
   adjacent official artifacts (Robot suites where they exist — QUERY has 109
   robot cases against a stub chapter — SM operation lists, ITS-REST, RM
   spec text), with explicit provenance.
5. **Assertions were JSON-only** (XML by string-scraping), the profile
   all-or-nothing rule was narrative not enforced, and constants (corpus SHA,
   creds, ignore-keys, heading counts) were scattered hardcodes.

## 3. The case universe: six gated sources

The catalogue (the ECC registry, §3.1) is the primary system; the sources
below are where its cases take their **reference material** from. Every
reference source has (a) an **extractor** that parses the vendored artifact
into an inventory of reference-unit ids, and (b) a **trace guard** (unit
test) asserting `extracted = traced-by-ECC ∪ excluded(reason)` —
build-breaking on re-vendor drift, silent-drop-proof by construction. The
catalogue is free to define cases with **no** official reference (S5/S6 and
anything the frozen corpus never covered); those carry their spec citation as
the grounding instead.

| # | Source | Artifact parsed | Executable units (target) |
|---|--------|-----------------|---------------------------|
| S1 | **Schedule** | `CNF/docs/platform_test_schedule/*.adoc` headings **and their normative tables** | 324 cases → **~1,650 variant-expanded** |
| S2 | **Robot** | `CNF/tests/platform/robot/**/*.robot` test-case declarations | **464** (minus structurally-excluded) |
| S3 | **ITS-REST matrix** | vendored ITS-REST OAS: operation × documented response code | **~350–400** pairs |
| S4 | **AQL corpus** | `_resources/test_data_sets/query/**` | 119 valid × {empty_db, loaded_db golden} + invalid ≈ **250** |
| S5 | **Spec-fill** | RM/SM spec text for upstream-stub areas (17.5 time_specification, DV_STATE/DV_PARAGRAPH, master10/12 real cases from SM ops) | engine-defined, curated |
| S6 | **Runner-defined** | Signing (`SIGN-*`), security (RBAC 401/403 sweeps) | small, curated |

Total case universe ≈ **2,600–2,900 executable units**, every one carrying a
spec citation (file + heading/row/operation) — versus v2's 324.

De-duplication is by **evidence link, not deletion**: an S2 robot case that
realizes an S1 schedule case (the schedule's `Test runners` cell names it)
records `realizes: SCHED:<id>`; the report can show both views. An S3 pair
already evidenced by an S1/S2 case records the covering case id instead of a
new implementation — S3's guard accepts *coverage by reference*, so the matrix
is a completeness check first and a case generator only for genuinely
unevidenced (operation, status) pairs.

### 3.1 Case identity: the ECC id (ours, stable, industry-style)

The catalogue's primary key is our own id, in the classic
`<prefix>-<area>-<number>` test-catalogue convention:

```
ECC-<AREA>-<NNN>          a test case      (ECC-EHR-003, ECC-QRY-118)
ECC-<AREA>-<NNN>.<VV>     a data-set variant of that case (ECC-VAL-042.07)
```

- **AREA** (the category — the "full list per category" view): `EHR` (EHR
  service), `STA` (EHR_STATUS), `COM` (COMPOSITION), `CTB` (CONTRIBUTION),
  `DIR` (directory/FOLDER), `TPL` (template/OPT provisioning), `SQR` (stored
  queries), `QRY` (AQL execution), `VAL` (content/archetype validation —
  the master15/16/17 ground plus our fills), `REST` (ITS-REST operation ×
  status matrix), `DEM` (demographic), `ADM` (admin), `SEC` (security/authz),
  `SIG` (version signing), `MSG` (messaging, when implemented).
- **Numbers are allocated once and never reused**; a retired case keeps its
  number with status `Retired`. Variants `.01`-`.99` are the case's data-set
  rows.
- **Trace links** are metadata, not identity: each ECC case lists the official
  references it realizes —
  `sched:I_EHR_SERVICE.create_ehr-main`, `sched-row:CONT-DV_COUNT-validate_range#r07`,
  `robot:I_EHR_COMPOSITION/create_composition-event/001`,
  `oas:EHR.createEhr@409`, `aql:B/102@loaded_db` — plus the spec citation
  (file + section) that grounds its assertions.
- **Requirements dimension:** `iso18308:<section>` refs link cases to the
  openEHR ISO 18308 Conformance Statement (vendored at
  `docs/specs/openehr/REQUIREMENTS/iso18308_conformance.pdf`, Rev 1.5.1) —
  the requirements-level view (ISO §4 privacy/security/audit/integrity, §5.7
  version control, §1 structure, …). The report can roll ECC results up by
  ISO 18308 section, giving a requirements-conformance overview on top of the
  API/content one.

Reference-side unit ids (the `sched:`/`robot:`/`oas:`/`aql:` forms) are
assigned deterministically by the extractors (table row order, document
order), and every traced schedule row pins a **content fingerprint**
(normalized row text hash) — a re-vendor that inserts or edits a row breaks
the trace guard loudly instead of silently shifting meaning.

### 3.2 S1 — the schedule, variant-expanded (the normative spine)

The extractor (`schedule.rs`, rewritten) parses per `.adoc`:

- case headings (`=+ Test Case <id>`, tolerant of the double-space and
  level-3/4 content variants) — as today;
- **normative tables**: within a case body, `|===`/`!===` tables whose header
  row ends in `expected` / contains `accepted|rejected` columns (content
  chapters), and the named functional matrices (master06 valid-EHR_STATUS,
  master08 `[[one_commit]]` / EHR_STATUS / `[[folder_commit]]`, master09
  `$path/$result`, master04 `Data set(s)` lists) — each data row becomes a
  variant id + fingerprint;
- `===== Data set ...` sub-blocks (17.3's DV_INTERVAL<DV_PROPORTION> style)
  as named variants.

The registry implements each variant as a first-class runnable (a const-table
entry driving a shared case body). Reporting is per-variant: `RESULTS.md` can
say `CONT-DV_DURATION-validate_range: 41/44 rows passed, r12 r17 r31 failed`,
and a failing row is a finding with the exact spec row cited.

Profile membership follows the **profiles matrix**
(`CNF/docs/profiles/master03-profiles.adoc`) now encoded as a
capability→profile table in code; the all-or-nothing rule is **machine
enforced**: the report computes a per-capability verdict (every required
case/variant passed) and a per-profile verdict (every required capability
passed), and the statement's claim line is generated from that verdict only.

### 3.3 S2 — the Robot suite, transcribed natively

The extractor parses every `*.robot` under the vendored suite for declared
test cases (`*** Test Cases ***` blocks), keyed
`ROBOT:<service>/<file-stem>/<case-slug>`, capturing the upstream `[Tags]`,
`Force Tags`, `TOP_TEST_SUITE`, and `[Documentation]` anchor (the
schedule-mapping evidence). Classification enums (all structural):

- `Transcribed` — implemented natively in Rust (no Python, ever), asserting
  the robot file's concrete expectations (status codes, body paths, golden
  comparisons) translated through our assert layer;
- `RealizedBySchedule(SCHED:<id>)` — the robot case is the runner for a
  schedule case our S1 implementation already executes with the same
  assertions (link recorded; guard verifies the link target exists);
- `Excluded(reason)` — `EhrbaseHarnessArtifact` (DB backdoors via
  `db_keywords`, `java -jar` lifecycle, EHRbase error-string asserts that are
  not spec), `UpstreamTodoStub` (the 9 `TODO-*` files), `UpstreamTagged`
  (`future`/`not-ready`/`obsolete`/`TODO` — upstream's own exclusion set),
  `SecurityEnvironment` (Keycloak-flow cases runnable only in the compose
  tier, skipped-with-reason elsewhere).

Priority transcription targets (highest evidence value): `I_QUERY_SERVICE`
(109 cases backing the stub master11), `I_EHR_SERVICE/create_ehr-main` (52,
the data-set fan-out), `get_versioned_*` C.6 families (status/composition),
`I_DEFINITION_ADL14` (42), `I_ADMIN_SERVICE` (29 — the real content behind
stub master12), directory `DS-01..10` path sets, the time-zone trio.

Golden `expected_results` marked suspect by the corpus's own README (EhrScape
mis-generated ids: A/109, B/103, C/100–103, D/306–311…) carry
`golden: Suspect` — compared, but a mismatch reports `FailedSuspectGolden`
(a distinct status feeding a finding that adjudicates against the AQL spec
text, never silently trusted either way).

### 3.4 S3 — the ITS-REST operation × status matrix

The extractor loads the vendored ITS-REST OAS bundles (the `-codegen` variant
`emit-rest` already consumes — same pinned files, zero new vendoring) and
emits every `(api_group, operationId, method, path, documented_status)` tuple.
The guard requires each tuple to name its evidence: an S1/S2 case id that
exercises exactly that operation+status, or a dedicated `REST:*` case, or a
structural exclusion (`Adl2Returns501`, `NotImplemented(Messaging)`, …).

This is the instrument that makes "conformant to ITS-REST 1.0.3" a checked
sentence: every documented response of every documented operation has named
evidence or a named reason. Dedicated `REST:*` cases are typically small
(drive the op into the documented state; assert status + response schema
against the OAS-declared schema + canonical codecs).

### 3.5 S4 — the AQL corpus as first-class gated cases

Extractor enumerates `aql_queries_valid/{A–D}/*.json` (119),
`aql_queries_invalid/**`, and both golden sets. Each valid query yields up to
two units (`@empty_db`, `@loaded_db`) depending on golden availability; the
loaded-db path uses the corpus's own `data_load` fixtures (11 EHRs + 21
compositions) loaded through the public API only. Golden comparison runs
through the documented normalizer (rule-named suppressions only:
envelope/generator fields, RM-1.0.x-era formatting, `SignatureDefaultOn`);
placeholder substitution (`__MODIFY_EHR_ID_n__`) as upstream. Invalid queries
assert a 4xx rejection with an AQL-spec-cited reason.

### 3.6 S5 — spec-fill for upstream stubs (declared, never blended)

Where the schedule is a stub but the capability is real and testable, the
engine defines cases grounded in the adjacent official spec text, with
`provenance: SpecFill` and the grounding citation:

- **master17.5** (time_specification) and **DV_STATE/DV_PARAGRAPH**: truth
  tables authored from `RM/docs/data_types/` normative text (the same
  accepted/rejected table shape the sibling chapters use);
- **master10/12**: real cases derived from the SM service interfaces
  (`I_DEMOGRAPHIC_SERVICE`, admin ops) + the ITS-REST demographic/admin API —
  the existing `DEMO-*`/`ADMIN-*` suites re-keyed under this source;
- **master11**: covered by S2 (the 109 QUERY robots) + S4 — no fill needed
  beyond the 5 stub headings' classification.

SpecFill results are reported in their own section and count toward OPTIONS
capabilities only — never toward a CORE/STANDARD claim the schedule alone
must support (exception: master11/AQL-basic, where the profiles doc requires
the capability and the official evidence *is* the robot+corpus material; the
statement says exactly that).

### 3.7 S6 — runner-defined capabilities

Unchanged from v2 in substance: the `SIGN-*` suite (upstream ships zero
Signing material; our five cases are the capability's evidence, declared as
such) plus RBAC/401/403 sweeps derived from the `SECURITY_TESTS` intent.
`provenance: RunnerDefined`.

## 4. Engine architecture (what changes in the crate)

Kept: crate boundary (`crates/ehrbase-conformance`, app-layer, nothing depends
on it), SUT modes (External / SelfHosted+testcontainers), `Transport`
abstraction, reqwest client with Basic/Bearer + admin slot, CLI shape
(`run`/`list`/`report`), `scripts/conformance.sh` contract, exit codes,
failure-is-a-finding discipline, `docs/conformance/` artifact set.

Rewritten / new:

```
src/
├── model.rs         # CaseId {source, case, variant}, CaseMeta (chapter,
│                    #   capability, profiles, formats, provenance, citation,
│                    #   realizes-link, golden-trust), VariantMeta {id, fingerprint}
├── extract/         # the six extractors (pure, over docs/specs/openehr/**)
│   ├── schedule.rs  #   headings + normative tables + data-set blocks
│   ├── robot.rs     #   *** Test Cases *** + tags + doc anchors
│   ├── oas.rs       #   ITS-REST operation × status tuples
│   └── aql.rs       #   query corpus + goldens (+ suspect list from README)
├── registry.rs      # per-source registries + classification + evidence links
├── coverage.rs      # (tests) the per-source guards: extracted = impl ∪ excluded,
│                    #   fingerprint pinning, evidence-link validity, profile matrix
├── flow.rs          # small step-DSL for multi-request cases: named steps,
│                    #   captured vars (ehr_id, version_uid), per-step assertion —
│                    #   so a failure reports "step 3 of 5: PUT … expected 412"
├── assert.rs        # + structural XML: FromXml → typed RM → canonical JSON →
│                    #   same compare modes (no string-scraping); OAS response-
│                    #   schema checks for REST:* cases
├── profile.rs       # the profiles-doc capability matrix, encoded; per-capability
│                    #   and per-profile verdict computation (all-or-nothing)
├── suites/          # implementations, organized by source then chapter
└── report.rs        # per-source + per-chapter + per-profile matrices;
                     #   per-variant failure rows; statement generated from the
                     #   machine profile verdict; badge
```

Config object (`RunConfig`) absorbs the v2 hardcodes: corpus pin read from
`PROVENANCE.md` at build/run time (not a string literal), credentials/RM
version/ignore-keys all injectable with the current values as defaults.

Data-set iteration is no longer an opaque loop: a case with variants is
registered as `(shared body, const variant table)`; the runner executes and
records **one outcome per variant id**.

## 5. Reports and the claim

`docs/conformance/` artifact set, regenerated per run:

- `results.json` — per-unit outcomes (source, id, variant, status, message,
  duration, evidence links), SUT identity, corpus pins (all of: CNF, ITS-REST,
  RM versions), run config.
- `RESULTS.md` — per-source × per-chapter matrix with variant-level tallies.
- `COVERAGE.md` — generated (replaces hand-maintained COVERAGE_GAPS.md):
  the classification of the entire universe (implemented / evidence-linked /
  excluded-by-reason), per source.
- `CONFORMANCE_STATEMENT.md` — certificate-template structure; the profile
  claim line comes from `profile.rs`'s machine verdict; deviations register
  enumerates every exclusion reason with counts and citations.
- `badge.json` — `passed/universe` + profile verdict.

Statuses: `Passed | Failed | FailedSuspectGolden | Errored | Skipped(reason)`.
Discipline unchanged: a failure becomes an `F-AA-NN` finding; exclusion
reasons are structural only; **never** exclude to green a run.

## 6. Fixtures

Unchanged policy (vendored corpus read-only; programmatic RM-1.2.0 adaptation
with PORT-NOTE provenance; FLAT→canonical through the production
`openehr_flat` path; authored constraint-OPTs for content chapters where the
corpus ships none — each authored OPT documented as runner artifact). New:
the authored-OPT catalogue becomes systematic — every content-chapter
constraint named in a truth table gets an authored OPT builder, eliminating
the v2 `Skipped(no constraining template)` bucket (~90 cases) except where the
SUT genuinely lacks the surface (then it's a finding, not a skip).

## 7. CI

- **PR tier**: self-hosted, S1 functional + S6, JSON — minutes, required.
- **Full tier** (containers.yml): compose stack, the entire universe, both
  formats, RBAC on — artifacts uploaded, `docs/conformance/` refreshed on
  `develop`.

## 8. Implementation plan (compiling, tested increments; branch: `claude/cnf-hardening`)

1. **Engine core**: `model.rs`, per-source `extract/` (schedule tables first),
   `registry.rs` + `coverage.rs` guards, `profile.rs`; migrate existing suites
   onto the new model with their current granularity (guards initially permit
   `VariantPending` per case — visible, counted, enforced-shrinking).
2. **S1 variant expansion**: content chapters (1,371 rows — table-driven,
   mechanical) then functional matrices (master06/08/09/04 tables).
3. **S2 Robot transcription**: QUERY (109) → create_ehr-main (52) →
   DEFINITION_ADL14 (42) → versioned-object C.6 families → directory DS
   sets → admin (29) → the rest; tags/exclusions recorded.
4. **S3 OAS matrix**: extractor + evidence linking; author the residual
   `REST:*` cases for unevidenced pairs.
5. **S4 AQL corpus gating** (upgrade the existing QUERY-FIXTURE suite) +
   suspect-golden handling.
6. **S5 spec-fill** (17.5, DV_STATE/DV_PARAGRAPH, re-keyed DEMO/ADMIN) and
   **S6** RBAC sweeps; profile verdict + new reports; CI tiers; first v3
   statement.

Each step: clippy-clean, `cargo nextest run -p ehrbase-conformance`
(+ `--features self-host` e2e), guards green, phase checkbox ticked.

## 9. Appendix — the official capability → profile matrix

Transcribed from `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc`
("Default Profiles"; ✔ = required by the profile). This is the table
`profile.rs` encodes and the all-or-nothing verdict is computed from — kept
here as the human-readable overview; the machine copy in code cites this
section and the source file.

**Profiles:** **CORE** = minimal functional platform (storage + retrieval of
EHR data), all capabilities all-or-nothing. **STANDARD** = CORE + AQL
querying + logging. **OPTIONS** = catch-all for everything else, reported
per-capability.

| Component | Capability | CORE | STANDARD | OPTIONS |
|---|---|:-:|:-:|:-:|
| Definitions | ADL 1.4 Archetype provisioning | ✔ | ✔ | |
| | ADL 1.4 OPT provisioning | ✔ | ✔ | |
| | ADL 2 Archetype provisioning | | | ✔ |
| | ADL 2 OPT provisioning | | | ✔ |
| | Query provisioning | | ✔ | |
| EHR Persistence | EHR Operations | ✔ | ✔ | |
| | EHR Status | ✔ | ✔ | |
| | Composition Operations | ✔ | ✔ | |
| | Directory Operations | | ✔ | |
| | Change sets | ✔ | ✔ | |
| | Versioning | ✔ | ✔ | |
| | Archetype Validation | ✔ | ✔ | |
| Demographic | Party / Party Relationship / Archetype validation | | | ✔ |
| Querying | AQL basic | | ✔ | |
| | AQL advanced; AQL & terminology | | | ✔ |
| Admin | Activity Report, Physical Deletion, EHR Dump/Load, Bulk EHR load, EHR Archive, Demographic Archive | | | ✔ |
| Messaging | EHR Extract, TDS | | | ✔ |
| REST APIs | DEFINITION API, EHR API | ✔ | ✔ | |
| | QUERY API | | ✔ | |
| | DEMOGRAPHIC / ADMIN / MESSAGE API | | | ✔ |
| Security & Privacy (non-functional) | Signing | | ✔ | |
| | Anonymous EHRs | ✔ | ✔ | |
| External Data Formats | XML, JSON | ✔ | ✔ | |

Chapter mapping: CORE/STANDARD ≈ master04 (ADL 1.4 half), 06, 07, 08
(change sets/versioning), 15–17.x (archetype validation); STANDARD adds
master05 + master09 (directory) + master11/AQL + Signing; master10/12/13 and
ADL2 are OPTIONS — consistent with exactly those chapters being upstream
stubs.

## 10. What this is not

Unchanged from v2: not a Robot/Python port (native Rust only); not the
EhrScape/FLAT test bed; not a benchmark. And explicitly: not a re-vendor —
the pinned corpus is upstream-current (verified 2026-07-08); the depth comes
from exhausting what is pinned.
