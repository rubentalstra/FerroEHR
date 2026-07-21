# openEHR conformance & certification strategy — the CNF 2.0 upstream proposal

*Tracker: [#197](https://github.com/rubentalstra/ehrbase-rs/issues/197). Plan-file
lifecycle applies: this document is deleted in the PR that closes #197 (i.e. when
the proposal has been delivered upstream and the accepted workstreams have their
own issues). Evidence verified 2026-07-21 against the vendored CNF snapshot
(`docs/specs/openehr/CNF/`, commit `33251d2a`), the live
[openEHR/specifications-CNF](https://github.com/openEHR/specifications-CNF)
repository, the SPECCNF Jira project, and the openEHR Discourse archive.*

---

## 1. Executive summary

openEHR has no working conformance and certification story. The CNF component
defines the right concepts — a Conformance Guide, a Platform Conformance Test
Schedule, Profiles, a Certificate — but has been frozen since March 2022, has
never cut its Release 1.0.0 (planned for December 2018), leaves the entire
assessment layer `TBD`, has zero official test cases for AQL, and its only
executable artifact is one vendor's Robot suite that no longer runs out of the
box. Meanwhile procurement of openEHR CDRs is accelerating and every tender
that asks "is this product openEHR-conformant?" gets an unverifiable answer.

This document proposes **CNF 2.0** to the openEHR SEC: keep the vocabulary and
structure the community already agreed on in 2021–2022, and fix the three
things that killed the first attempt —

1. **Make the Test Schedule machine-readable and make it the single normative
   source.** Today the schedule exists as three unsynchronized representations
   (AsciiDoc prose tables, 2017 pseudo-code scripts, EHRbase Robot files).
   CNF 2.0 publishes one versioned, machine-readable catalogue — IDs, SM
   operation anchors, spec citations, REST bindings, data sets, expected
   outcomes, profile membership, spec-version applicability — from which the
   human-readable spec pages are *generated* and against which **any** harness
   (Robot, Rust, Spock, Postman collections) can prove itself. This is the same
   philosophy openEHR already uses everywhere else: the RM is published as BMM,
   the REST API as OpenAPI. Conformance is the one component still written only
   as prose.
2. **Define the certification governance that has been `TBD` since 2017**, as a
   concrete maturity ladder copied from programs that demonstrably work
   (OpenID self-certification, IHE Connectathons, ONC/Inferno): published
   statement registry → attested self-certification → community verification
   events → accredited third-party assessment.
3. **Resource it so it cannot stall on one person again**: a maintainer group,
   CI on the spec repo, and vendor-funded gap-fill with AQL first.

ehrbase-rs contributes its ECC framework — a working 394-case,
both-wire-formats, machine-computed-verdict conformance instrument built on the
CNF profiles model — as a working draft and one reference implementation.
Explicitly **not** as "the standard": the standard must be community-owned,
vendor-neutral, and multi-harness by construction. Our role is to donate
methodology, case designs, and engineering effort, and to be the first SUT
assessed by whatever the community ratifies.

---

## 2. The problem

- **Procurement.** Tenders increasingly name openEHR. Without an official test
  schedule + statement format, "openEHR-conformant" is a marketing claim; the
  2021 board decision to fund conformance work cited exactly this
  ([Discourse 1851](https://discourse.openehr.org/t/conformance-roadmap-2021/1851)).
- **Vendor fairness.** Every vendor today self-declares against a different
  private checklist. The only shared harness encodes one vendor's behaviour
  (EHRbase's Robot suite), which biases "conformance" toward one
  implementation's quirks — the exact failure the CNF Guide warns against.
- **Spec quality feedback.** A real conformance suite is the best defect
  detector the specifications themselves can have. The CNF content chapters
  (data types, entry structure) already proved this in 2022; the stub chapters
  (AQL!) mean the flagship capability has no executable definition of correct
  behaviour.
- **Community credibility.** HL7 ships Inferno test kits with regulatory teeth;
  IHE runs Connectathons; OpenID runs self-certification at scale. openEHR's
  conformance page still says `TBD` where "how do I get certified?" should be.

## 3. Evidence base — state of the official CNF component (2026-07-21)

### 3.1 The four books

Published at
[specifications.openehr.org/releases/CNF/development](https://specifications.openehr.org/releases/CNF/development);
sources in [openEHR/specifications-CNF](https://github.com/openEHR/specifications-CNF);
vendored snapshot `docs/specs/openehr/CNF/` @ `33251d2a`.

| Book | Status | Last substantive amendment | State |
|---|---|---|---|
| Conformance Guide | DEVELOPMENT | 0.6.0, 08-Jan-2022 (`guide/master00-amendment_record.adoc`) | Methodology sound (SUT model, the specs→runnable-tests "square", API-vs-content test split). **Assessment layer all `TBD`**: `guide/master05-assessment.adoc` §Tooling, §Test Execution Report, §Conformance Statement, §Conformance Certification. "Platform Clients" scope: bare `TBD`. |
| Platform Conformance Test Schedule | DEVELOPMENT | 0.8.6, 24-Mar-2022 (`platform_test_schedule/master00-amendment_record.adoc`) | See chapter map below. Minimum RM pinned 1.0.2 (`master03-overview.adoc`), behind RM 1.1.0/1.2.0. |
| Platform Profiles | DEVELOPMENT | 2022 | CORE / STANDARD / OPTIONS capability matrix (`profiles/master03-profiles.adoc`) — usable as-is; this repo's ECC implements it verbatim. |
| Conformance Certificate | DEVELOPMENT | 2021 | A **fictional worked example** ("BestEHR 2.4", "ACME EHR systems LLC", dated 2017; `certificate/master03-certificate.adoc`). No issuance procedure, assessor accreditation, validity period, or revocation anywhere. |

### 3.2 Test Schedule chapter map

From `docs/specs/openehr/CNF/docs/platform_test_schedule/` (`aaaa`/`bbbb`/`xx`/`TBD`
placeholders = stub):

| Chapter | Area | State |
|---|---|---|
| master04 | Definitions ADL 1.4 / ADL 2 | Fleshed for ADL 1.4; ~5 ADL2 mentions only |
| master05 | Definitions: stored queries | **Stub** (all `xx`) |
| master06–09 | EHR / COMPOSITION / CONTRIBUTION / DIRECTORY | **Fully fleshed** — the good core (~120 cases with Description/Pre/Post/Flow + data-set matrices) |
| master10 | Demographic | **Pure stub** (26 TBD markers) |
| master11 | Querying (AQL) | **Stub** — the flagship openEHR capability has zero official test cases |
| master12 | Admin | **Pure stub** |
| master13 | Messaging (EHR Extract / TDD) | **Pure stub**; duplicated `export_ehr()` heading |
| master14 | — | **Missing** (numbering jumps 13→15) |
| master15–16 | Content: COMPOSITION / ENTRY structures | Fleshed (decision tables) |
| master17.1–17.7 | Content: data types | Fleshed except 17.5 (time_specification, stub); 17.3 (quantities, 47 cases) is the exemplar |

Two ID families exist and are worth keeping: functional
`<SERVICE_COMPONENT>.<operation>-<case>` (e.g. `I_EHR_SERVICE.create_ehr-main`)
anchored to SM interface operations, and content `CONT-<TYPE>-<scenario>`
decision tables. The global ID scheme was announced as spanning "REST API,
content, everything"
([Discourse 2358](https://discourse.openehr.org/t/conformance-schedule-progress-data-types/2358)).

### 3.3 The executable layer

- `CNF/tests/platform/robot/` — 223 `.robot` files, imported wholesale from the
  EHRbase project: every header reads "This file is part of Project EHRbase"
  (© 2019 Vitasystems/HMS); `tests/Taskfile.yml` hard-codes
  `ehrbase/ehrbase:13.3` + `ehrbase/ehrbase-postgres:13.4` and Spring auth
  flags. It is an EHRbase dev harness, not a neutral instrument.
- [specifications-CNF PR #5](https://github.com/openEHR/specifications-CNF/pull/5)
  "Make the tests runnable" — open since **June 2023**, unmerged. The official
  suite does not run against an arbitrary SUT out of the box.
- `CNF/scripts/openehr_platform/*.txt` — 34 abstract pseudo-code scripts
  (© 2017), a third representation not wired to anything.
- Robot coverage is asymmetric to the schedule: robots exist for stub chapters
  (Query, Admin) and are missing for others (Demographic, Messaging).

### 3.4 Vital signs

- Jira [SPECCNF](https://openehr.atlassian.net/jira/software/c/projects/SPECCNF/list):
  **two** visible issues. [SPECCNF-1](https://openehr.atlassian.net/browse/SPECCNF-1)
  "Create openEHR Conformance Definition specification" — Open since 2017.
  [SPECCNF-6](https://openehr.atlassian.net/browse/SPECCNF-6) "Create
  Conformance Guide" — In Progress since **October 2021**, assignee Pablo
  Pazos, zero comments since.
- Jira Release-1.0.0: release date 2018-12-28, **never released**.
- specifications-CNF git: last content work 2022 (Pablo's schedule updates);
  2024 = rendering/link fixes; May 2026 = Antora docs-toolchain migration only.
- Open repo issues #1/#2 date from 2017.

## 4. How it got here — history and stall post-mortem

### 4.1 Timeline

| When | What |
|---|---|
| 2014–2017 | Early conformance framework sketches; the [openEHR Conformance wiki page](https://openehr.atlassian.net/wiki/spaces/spec/pages/73367558/openEHR+Conformance) (2017) proposes functional levels 1/2/3+O, enterprise D/M/X, volumetric POC/S/L/R ratings. |
| Aug 2017 | [Alkmaar SEC meeting notes](https://openehr.atlassian.net/wiki/spaces/spec/pages/94181296/Conformance+Notes+-+SEC+meeting+Alkmaar+2017): certificate expiry question, vendor/tender profiles, randomizable test data, testing beyond REST. Pablo Pazos posts a full review of the draft spec as [SPECCNF-1 comment 22500](https://openehr.atlassian.net/browse/SPECCNF-1?focusedCommentId=22500) — see §4.4. |
| Dec 2018 | Release 1.0.0 target passes; nothing cut. |
| 2019–2021 | EHRbase builds its Robot suite (Vitasystems/HMS, HiGHmed funding); becomes the de-facto answer to "how do I test?" ([Discourse 1335](https://discourse.openehr.org/t/conformance-testing/1335)). |
| 2021 | Board formally funds a conformance project ([Discourse 1851](https://discourse.openehr.org/t/conformance-roadmap-2021/1851)). The deep design debate happens ([Discourse 1616](https://discourse.openehr.org/t/openehr-conformance-conformance-levels-conformance-scopes/1616)). |
| Jan–Mar 2022 | High-water mark: framework block diagram ([2239](https://discourse.openehr.org/t/conformance-framework-description/2239)); data-type content tests + global ID scheme land in the official schedule ([2358](https://discourse.openehr.org/t/conformance-schedule-progress-data-types/2358)); CaboLabs publishes its broader framework guide ([2285](https://discourse.openehr.org/t/openehr-conformance-verification-design-document/2285)); Robot suite copied into specifications-CNF. Schedule 0.8.6 (24-Mar-2022) is the **last amendment ever**. |
| Sep 2022 | Stall on record: implementers ask about de-EHRbase-ifying the Robot suite; Pablo: "I don't have a timeline since I'm doing it in my free time. Help is welcome." ([2373](https://discourse.openehr.org/t/conformance-testing-implementation-alternatives/2373)). |
| 2023–2026 | PR #5 unmerged; rendering fixes; Antora migration. No content. |

### 4.2 What the 2021–2022 design era settled (keep all of it)

- **The four-artifact vocabulary**: Conformance **Schedule** (everything
  testable) / **Profile** (a viable product type's capability set) /
  **Statement** (what a product claims + which tests pass) / **Certificate**
  (statement + report + attestation).
- **SM names the capabilities, an ITS executes the tests**: test *definitions*
  anchor to Service Model operations (`I_EHR_SERVICE.create_ehr`); test
  *execution* binds to a concrete ITS (REST + a serialization). This resolved
  the Pablo-vs-Thomas anchoring debate and matches how this repo's service
  layer is organized.
- **Technology profiles**: serialization formats (and potentially other stack
  dimensions) parameterize the *implementation* of the suite, so functional
  results stay comparable across vendors with different stacks.
- **A global test-case ID scheme** spanning API + content tests.
- **The four-stage certification maturity ladder**: standardized vendor
  statements → procurer-run testing → vendor self-certification → trusted
  third-party certification.
- **Profiles CORE / STANDARD / OPTIONS** with the capability matrix.

### 4.3 Why it stalled (the post-mortem the proposal must answer)

1. **Single-person, spare-time ownership.** The ambitious half lived with one
   person unpaid; when HiGHmed's project need ended, momentum ended.
2. **Funding tied to one project, not to the program.** Board support produced
   a roadmap, not sustained resourcing for maintenance/certification phases.
3. **Two-track scope split with no owner for the union.** Official CNF stayed
   deliberately narrow (CDR/REST); the broader CaboLabs framework stayed one
   company's document; the certification/governance half belonged to neither.
4. **Single-harness lock-in.** The abstract-spec + any-technology model was
   *chosen* in principle but never realized: the only implementation stayed
   EHRbase-specific, and its generalization had no owner (PR #5's fate).

### 4.4 SPECCNF-1 comment 22500 (Pablo Pazos, Aug 2017) — still the best requirements list

Nine years old and almost fully unaddressed; CNF 2.0 should answer it point by
point:

- The spec should include **guidance + format for writing Conformance
  Statements** — "the first step before any testing, it actually defines what
  can be tested" (his model: DICOM PACS conformance statements).
- The Certificate section raises unanswered governance questions verbatim:
  *"what is this? how is it created? who can create it? who can grant it? who
  verifies it?"* — the exact `TBD`s still in `guide/master05`.
- Scope discipline: functional vs non-functional conformance points stated up
  front; don't redefine ISO 9126/25010 quality terms; certify **platforms**
  explicitly (don't silently exclude non-platform products); no assumptions
  that imply a web UI ("portal", "data viewer"); avoid manual testing.
- Don't hard-code REST as the only access method conceptually (while it is the
  pragmatic first binding).
- Archetype-validation conformance points need precise definitions (valid/
  invalid content × ADL vs OPT × data-vs-definition).

## 5. Prior art — how other standards run conformance

| Program | Model | What to copy |
|---|---|---|
| **DICOM conformance statements** ([DICOM PS3.2](https://www.dicomstandard.org/current)) | Every product publishes a standardized conformance statement; procurement compares statements; no central certification. | The **statement as the legally load-bearing artifact**, with a normative template. This was Pablo's original 2017 ask. CNF 2.0 upgrade: make it computable, not prose. |
| **OpenID Foundation certification** ([openid.net/certification](https://openid.net/certification/)) | **Self-certification**: vendor runs the official open-source suite, submits results + a signed legal attestation, pays a small fee, gets listed on the public certified page. Runs at scale since 2015 with new programs still launching in 2026. | The **cheapest credible rung**: official suite + published results + attestation + public registry. No assessor bureaucracy needed to start. |
| **HL7 FHIR / ONC Inferno** ([inferno.healthit.gov](https://inferno.healthit.gov/), [framework docs](https://inferno-framework.github.io/docs/)) | Open-source test kits per implementation guide; the (g)(10) kit is an approved test method inside a regulatory certification program. | **Test kits as first-class open-source products** with maintained releases; machine-readable expectations derived from published artifacts (CapabilityStatements/IGs); regulator-grade reporting from the same tool developers use daily. |
| **IHE Connectathons + Conformity Assessment Scheme** ([ihe.net](https://www.ihe.net/testing/)) | Annual supervised peer-testing events (results published) plus an ISO/IEC 17025-based accredited-lab scheme for formal conformity assessment. | The **community verification event** rung (a conformance-thon at EHRCON fits openEHR's culture) and the eventual accredited-assessor shape. |

Composite lesson: nobody starts with third-party certification. Working
programs start with an **official, runnable, vendor-neutral suite + a public
registry of published results**, add legal attestation (OpenID), then events
(IHE), then accreditation. That is exactly the 2021 ladder — the ladder was
right; the bottom rung was never built.

## 6. Design principles for CNF 2.0

1. **Machine-readable normative source, generated prose.** Like BMM → RM
   crates and OpenAPI → REST contract, the Schedule's normative form is data;
   the published spec pages are rendered from it. A test case that isn't in
   the catalogue doesn't exist; a catalogue entry without spec citations
   doesn't build.
2. **SM anchors semantics; ITS bindings execute.** Every case carries its SM
   operation (or content constraint) + spec citations, and one or more
   concrete bindings (REST first). New protocols add bindings, not new suites.
3. **Harness independence by construction.** The catalogue + data sets +
   results schema are the contract; Robot, Rust, Spock, Postman are all just
   runners. A runner is *itself* verified by replaying the catalogue against
   reference SUTs.
4. **Verdicts are computed, never asserted.** Profile verdicts (CORE /
   STANDARD / OPTIONS) derive mechanically from a machine-readable results
   file. A certificate row a human typed is a defect.
5. **Honesty is structural.** Coverage bounds printed, N/A adjudications
   cited, corpus defects adjudicated in a register rather than silently
   edited, both directions published when comparing products.
6. **Vendor neutrality is testable.** No vendor image names, endpoints,
   auth flows, or behavioural quirks in normative artifacts; fixtures carry
   spec citations, not `EhrBase ref:` markers. CI enforces it.
7. **Versioned like every other component.** The Schedule pins the spec
   versions each case applies to (RM/REST/AQL applicability ranges); a
   certificate names the schedule release + tech profile it was earned
   against. Within-major supersets follow openEHR's release strategy.
8. **Scope discipline.** Platform (CDR) profile first — CORE/STANDARD as
   defined. System classes beyond the CDR (clients, tools, brokers, analytics)
   get profiles later; performance/volumetrics stay out of functional
   conformance (per the Guide) but the statement schema reserves the slot.

## 7. The proposal — CNF 2.0 architecture

### 7.1 The machine-readable Conformance Schedule (the lead idea)

One catalogue, versioned in specifications-CNF, one file per test case (YAML
here for readability; the format decision belongs to SEC — JSON/YAML with a
published JSON Schema):

```yaml
# schedule/platform/ehr/I_EHR_SERVICE.create_ehr-no_status.yaml
id: I_EHR_SERVICE.create_ehr-no_status     # global CNF id — 2022 scheme retained
kind: functional                            # functional | content
component: EHR
sm_operation: I_EHR_SERVICE.create_ehr      # the SM anchor
spec_refs:
  - "SM openehr_platform §I_EHR_SERVICE.create_ehr"
  - "RM ehr §4.2 EHR creation semantics"
applies:                                    # spec-version applicability ladder
  rm: ">=1.0.2"
  its_rest: ">=1.0.0"
profiles: [CORE]
description: >
  Create a new EHR without a supplied EHR_STATUS; the platform must create a
  default status with is_queryable=true and is_modifiable=true.
preconditions: []
flow:
  - call: create_ehr
    bindings:
      rest: { method: POST, path: "/ehr" }
    expect:
      rest: { status: 201, headers: [ETag, Location] }
postconditions:
  - "EHR exists and is retrievable by the returned ehr_id"
  - "EHR_STATUS.is_queryable = true; EHR_STATUS.is_modifiable = true"
data_sets: []                               # keys into the governed corpus
formats: [canonical-json, canonical-xml]    # tech-profile dimension exercised
```

An AQL case — the area with zero official coverage today:

```yaml
# schedule/platform/querying/I_QUERY_SERVICE.execute_adhoc-where_magnitude.yaml
id: I_QUERY_SERVICE.execute_adhoc-where_magnitude
kind: functional
component: QUERY
sm_operation: I_QUERY_SERVICE.execute_adhoc_query
spec_refs:
  - "QUERY AQL 1.1 §WHERE"
  - "RM data_types §DV_QUANTITY.magnitude"
applies: { rm: ">=1.0.2", its_rest: ">=1.0.0", aql: ">=1.1" }
profiles: [STANDARD]                         # AqlBasic per the Profiles book
description: >
  Ad-hoc AQL with a WHERE predicate on DV_QUANTITY.magnitude returns exactly
  the compositions whose stored magnitude satisfies the predicate.
preconditions:
  - "data_set cnf.vitals.bp-10 committed to a fresh EHR"
flow:
  - call: execute_adhoc_query
    bindings:
      rest: { method: POST, path: "/query/aql" }
    input:
      q: >
        SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c
        CONTAINS OBSERVATION o [openEHR-EHR-OBSERVATION.blood_pressure.v2]
        WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= $mag
      query_parameters: { mag: 140 }
    expect:
      rest: { status: 200 }
      result_set:
        rows: { match: exact, from_data_set: "cnf.vitals.bp-10#magnitude>=140" }
data_sets: [cnf.vitals.bp-10]
formats: [canonical-json]
```

Content cases keep their decision-table nature — the table becomes rows in the
data file instead of an AsciiDoc grid, and the existing `CONT-*` IDs carry over
unchanged.

What this single change fixes: the three-representations drift (prose,
pseudo-code, Robot) collapses into one source; stub chapters become an
*enumerable, assignable* backlog (a missing case is a missing file); coverage
of any harness is computable (`cases implemented / cases in schedule`, per
profile); and the published spec pages regenerate from the catalogue so the
document can never disagree with the tests again.

### 7.2 The derivation square, machine-checked

The Guide's specs → SM call → binding → runnable test chain
(`guide/master04-framework.adoc` §From Specifications to Runnable Tests)
becomes CI on specifications-CNF: every catalogue entry must carry non-empty
`spec_refs`, a resolvable `sm_operation` (checked against the SM component
list), at least one binding, and — for content cases — a decision table.
Schema validation + link checking + ID-uniqueness (IDs are never reused;
retired cases keep their ID with `status: retired`) run on every PR. This
repo's ECC runs exactly this guard today (`tools/conformance/tests/coverage.rs`)
and it is the single most effective discipline in the framework.

### 7.3 Computable Conformance Statement + results schema

Two small JSON schemas, published as normative parts of CNF 2.0:

- **`results.json`** — one run of one harness against one SUT: SUT identity
  (product, version, deployment), harness identity + version, schedule release,
  tech profile (formats exercised), and per-case outcomes
  (`passed | failed | errored | skipped | not-applicable`, with a mandatory
  citation on every N/A and skip). Errored (transport/SUT fault) is never a
  conformance finding.
- **`statement.json`** — the vendor-facing artifact SPECCNF-1 asked for in
  2017: which components/capabilities/profiles the product claims, at which
  spec versions, under which tech profiles, backed by which `results.json`
  (hash-linked). Renderable to the human-readable Conformance Statement
  document; comparable mechanically across vendors — Pablo's "computable
  statement, published on a vendor-neutral site" idea from
  [Discourse 1851](https://discourse.openehr.org/t/conformance-roadmap-2021/1851),
  made concrete.

Profile verdicts (CORE/STANDARD pass = *all* required capabilities evidenced
and passing; OPTIONS = any) are a pure function of `results.json` + the
catalogue — the rule set in the Profiles book, executable.

### 7.4 Profiles and system classes

Keep the Profiles book's CORE / STANDARD / OPTIONS matrix as the **Platform
(CDR) profile family** — it is coherent and implemented in practice. Add, as
later work, profile families per system class (the 2021 insight that a
universal CORE is wrong): platform client, tool, demographic service,
terminology service. Procurers compose tender profiles from the matrix, which
is what the Profiles overview already intends.

### 7.5 Technology profiles and the spec-version ladder

- **Tech profile** = the serialization/protocol matrix a run exercised
  (canonical JSON, canonical XML; REST binding; later others). A statement
  says which tech profiles were run; CORE/STANDARD verdicts are per tech
  profile, ending the 2021 "all tests in all formats vs one format" stalemate
  by reporting both honestly instead of choosing.
- **Spec-version applicability** on every case (`applies:` ranges) lets one
  schedule serve SUTs pinned to different release lines — a certificate names
  the schedule release + the SUT's declared spec versions. ECC's "edition
  ladder" (assertion cores split from edition-specific wire forms, satisfied
  rung recorded as a finding) is the donated working model.

### 7.6 Data-set governance

- A **governed corpus** with manifest-keyed fixtures: every data set has an ID,
  provenance, spec citations, and validity adjudication (valid / deliberately
  invalid with the violated constraint named).
- **Generated data sets** for combinatorial areas (the content decision tables,
  AQL result-set fixtures) — generation code is part of the suite, seeded and
  deterministic, answering Alkmaar 2017's "randomisable test data sets" with
  reproducibility.
- **An adjudication register instead of silent edits**: when a fixture or
  golden is found defective, the defect is recorded with a citation and the
  affected cases skip-with-citation or run against the spec-derived
  expectation. Never edit history quietly.
- Migrate the usable EHRbase fixture trove (compositions, contributions,
  folders, invalid sets, OPTs) into the governed corpus, stripping vendor
  markers and re-adjudicating each item against the spec.

### 7.7 Harness independence, concretely

- CNF 2.0 normative artifacts: catalogue + JSON Schemas (case, results,
  statement) + corpus + verdict rules. **No harness is normative.**
- Any harness claims compliance by (a) implementing catalogue selection by ID,
  (b) emitting valid `results.json`, (c) passing a **runner-verification
  pack**: a reference SUT recording (or fixture server) with known
  pass/fail/N-A outcomes the runner must reproduce.
- Expected day-one runners: the de-EHRbase-ified Robot suite (rescuing PR #5's
  intent), ECC (Rust, this repo), and whatever vendors already run privately —
  which is the point: vendors keep their tooling and gain comparability.

### 7.8 CI on specifications-CNF

Schema validation, ID uniqueness/no-reuse, spec-ref link checks, corpus
manifest integrity, prose regeneration — so the repo can accept community PRs
safely, which is the mechanism that lets gap-fill scale beyond one maintainer.

## 8. Certification governance — the ladder made concrete

Answering SPECCNF-1's 2017 questions (*who creates / grants / verifies?*) with
the rungs prior art proved:

| Rung | Name | Mechanism | Who grants | Prior art |
|---|---|---|---|---|
| 0 | **Published statement** | Vendor publishes `statement.json` + `results.json` from any compliant runner; openEHR hosts a public registry page rendering them (claims are visible + mechanically comparable; no endorsement implied). | Nobody — publication only | DICOM statements + OpenID's public listing |
| 1 | **Self-certification** | Rung 0 + a signed legal attestation of result accuracy (+ optional modest fee funding the program). Listed as "self-certified" with schedule release, tech profile, date. | openEHR International (administrative check only) | OpenID Foundation |
| 2 | **Community-verified** | Results reproduced at a supervised conformance-thon (EHRCON slot) or by a named community witness re-running the suite against a vendor-provided deployment. | Event organizers / named witnesses | IHE Connectathon |
| 3 | **Certified** | Accredited assessor runs the suite, signs the certificate. Assessor accreditation criteria published by openEHR International (ISO/IEC 17025-shaped, scaled down). | Accredited assessors, program owned by openEHR International | IHE CAS, ONC |

Cross-cutting rules:

- **Validity**: a certificate/statement names the CNF schedule release + spec
  versions + tech profile. It never expires by clock alone; it is **superseded**
  when a newer schedule release changes the cases it rests on, and the registry
  shows currency (answering Alkmaar's expiry question without inventing a
  revocation bureaucracy).
- **Badges** derive from registry state (rung + profile + schedule release),
  machine-served, never self-hosted claims.
- **Access**: schedule, schemas, corpus, and runners are public and free
  (Inferno/OpenID lesson: adoption dies behind paywalls). Rungs 1–3 may carry
  fees; the 2021 members-only idea should apply to *services* (attestation
  processing, events, assessor program), never to the artifacts.
- **Legal force**: below rung 3, veracity remains a commercial-contract matter
  between vendor and procurer — stated plainly, as the 2021 roadmap did.

## 9. Gap-fill roadmap (content plan for the schedule itself)

Ordered by procurement value; each item is a bounded, assignable chapter task
once §7.1 makes cases enumerable files:

1. **Querying / AQL (master11 + master05)** — the flagship gap. Seed material
   exists: this repo's 25 QRY + 8 SQR + 4 AQT case designs (each carrying AQL
   1.1 citations), and EHRbase's AQL conformance corpus
   ([ehrbase/conformance-testing-documentation](https://github.com/ehrbase/conformance-testing-documentation),
   SELECT/WHERE/ORDER BY/LIMIT/FROM/parameter suites). Design decision for
   SEC: result-set equivalence rules (row order, path forms, number typing).
2. **Content chapters refresh** — raise the RM floor statement (1.0.2 → an
   applicability ladder), fill 17.5 or formally adjudicate it out, fix the
   master14 numbering gap and the master13 duplicate heading.
3. **Demographic (master10)** — schedule cases exist in no form today; ECC's 31
   DEM cases + the ITS-REST Demographic API (DEVELOPMENT lifecycle) are the
   seed; profile placement stays OPTIONS.
4. **Admin (master12) + Messaging (master13)** — decide what is *wire-testable*
   (platform API) vs inherently off-wire (dump/load, archives); off-wire
   capabilities move to statement-declared, not schedule-tested — the honest
   boundary ECC had to invent N/A adjudications for.
5. **Security & privacy conformance points** — currently only Signing +
   Anonymous EHRs in the Profiles book while the Certificate book advertises
   BASIC-SEC/BASIC-PRIV ratings with no defining cases. Minimum viable set:
   authenticated-access enforcement, audit-event emission on writes
   (IHE ATNA-shaped), signing. Explicitly scoped small; not a security
   evaluation scheme.
6. **ADL2 cases (master04)** — OPTIONS-profile depth for the `am24` generation.

## 10. What ehrbase-rs contributes — and what must stay community-owned

Offered (donated under the spec repo's license, relicensing ours to match):

- **Methodology + schemas as the working draft**: the catalogue entry model
  (ID, citation, SM anchor, binding, formats, profile, applicability), the
  results/statement schema shapes, the computed-verdict rules implementing the
  Profiles book, the coverage-guard CI design, the adjudication-register and
  fairness-register patterns, the edition ladder. All running code today
  (`tools/conformance/`), not paper.
- **~380 case designs** with spec citations as raw material for the stub
  chapters — QRY/SQR/AQT for master11/05, DEM for master10, VAL's 119 content
  cases cross-checked against master15–17, plus the honest off-wire (N/A)
  treatment for Admin/Messaging.
- **Engineering effort**: drafting the JSON Schemas, the specifications-CNF CI,
  the schedule-to-prose renderer, and the AQL chapter — as PRs under SEC
  review.
- **A second harness + first registry entries**: ECC adopts CNF 2.0 IDs as
  primary the day they exist upstream (they are our `ScheduleTrace` today) and
  ehrbase-rs volunteers as a rung-0/rung-1 guinea pig, alongside upstream
  EHRbase which we already assess.

Explicitly **not** claimed, and neutrality deltas we accept:

- ECC's private numbering (`ECC-<AREA>-<NNN>`) is ours, not a proposal; the
  global `<SERVICE_COMPONENT>.<operation>-<case>` / `CONT-*` scheme wins.
- Our latest-versions-only pins, self-assessment defaults, and
  internal-test-backed N/A pointers are self-assessment ergonomics; the
  community scheme must support older release lines and third-party-verifiable
  evidence only.
- The standard needs ≥2 independent runners and a maintainer group with no
  vendor majority — the single-owner failure mode is the one thing this
  proposal must not reproduce. If the community prefers rescuing the Robot
  suite as the reference runner, we support that; our value is the
  machine-readable spine, not the runner.

## 11. Engagement plan

1. **Discourse first** (Conformance category) — the strategy condensed to a
   discussion post (Appendix A), tagging the 2021–22 participants; goal:
   temperature check + volunteers, 2–3 weeks.
2. **Jira + repo** — comment on SPECCNF-1/6 linking the thread (Appendix B);
   new specifications-CNF issue proposing the machine-readable schedule format
   with the two example cases (Appendix C).
3. **SEC agenda item** — ask for a slot; deliverable: adopt-the-format decision
   + a CNF maintainer-group call + blessing the AQL chapter as the pilot.
4. **Pilot PR series** (after SEC nod): JSON Schemas + CI, master06 (the
   fleshed exemplar) converted to catalogue form with prose regenerated
   byte-comparable, then master11/AQL as the first *new* content.
5. **Registry MVP**: a static page on openehr.org rendering submitted
   statements — rung 0 exists the moment two products publish.
6. **EHRCON conformance-thon proposal** once ≥2 runners + ≥2 SUTs exist.

Success measures: SEC adopts the machine-readable schedule; ≥2 independent
runners pass the runner-verification pack; AQL chapter released; ≥3 products
on the public registry; Release 1.0.0 of CNF finally cut.

## 12. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Perceived vendor capture ("ehrbase-rs wants its framework blessed") | Lead with the community's own 2021–22 design (we implement it, we didn't invent it); donate under spec-repo license; propose a no-vendor-majority maintainer group; keep our numbering out of the proposal. |
| Repeat of the single-owner stall | The maintainer group + CI + enumerable-file backlog make contributions mergeable without a bottleneck person; gap-fill chapters are bounded tasks; we commit named engineering effort to the pilot series. |
| SEC bandwidth / spec-process latency | Rung 0 (registry of published statements) and the schedule format need no new normative prose to start delivering value; pilot PRs convert existing content before adding new. |
| Format bikeshed (YAML vs JSON vs tables) | Bring a JSON Schema + two worked examples + a prose renderer to the first discussion; decide on evidence, not taste. |
| Robot-suite loyalists read this as suite replacement | It isn't: the catalogue makes the Robot suite *one compliant runner*; rescuing PR #5 is inside the proposal. |
| CaboLabs framework overlap | Invite Pablo as co-author from the Discourse post onward; his 2017 review and 2022 framework are cited as requirements sources, and the statement schema is his idea made computable. |

---

## Appendix A — Discourse post draft (Conformance category)

> **Title: Reviving CNF: a concrete proposal for a machine-readable conformance schedule + a certification ladder**
>
> The CNF component defined the right things in 2021–22 — the Schedule /
> Profile / Statement / Certificate vocabulary, SM-anchored test cases, tech
> profiles, CORE/STANDARD/OPTIONS — and then stalled: the last schedule
> amendment is 0.8.6 (March 2022), Release 1.0.0 was never cut, the assessment
> chapter is still `TBD`, the Querying chapter has zero test cases, and the
> only executable suite is EHRbase-specific and currently doesn't run
> (specifications-CNF PR #5 has been open since 2023).
>
> We (ehrbase-rs) have been running a CNF-shaped conformance instrument in
> production against two CDRs for a while — profiles verdicts computed from the
> Profiles book, both canonical formats, every case citing the spec — and we'd
> like to bring the useful parts upstream rather than let another parallel
> framework grow. **We are not proposing our tool as the standard.** We are
> proposing three things for discussion:
>
> 1. **Make the Test Schedule machine-readable and normative** — one versioned
>    catalogue (ID, SM operation, spec citations, REST binding, data sets,
>    expected outcomes, profile, spec-version applicability) from which the
>    spec's prose pages are generated and against which *any* runner — the
>    Robot suite, ours, Spock, Postman — can prove itself. Same philosophy as
>    BMM for the RM and OpenAPI for REST. This turns the stub chapters (AQL
>    first!) into an enumerable backlog anyone can PR against.
> 2. **Define the bottom rungs of the 2021 certification ladder now**: a
>    computable Conformance Statement schema (the thing SPECCNF-1 asked for in
>    2017) + a public registry page of published statements/results, then
>    OpenID-style attested self-certification. Third-party certification stays
>    the end goal, not the entry ticket.
> 3. **A CNF maintainer group + CI on specifications-CNF** so this can't stall
>    on one person's spare time again.
>
> We're offering: the JSON Schemas and CI as pilot PRs, conversion of an
> existing fleshed chapter (master06) as proof the format loses nothing, a
> drafted AQL chapter seeded from our 30+ AQL case designs and EHRbase's AQL
> corpus, ~380 cited case designs as raw material for the other stubs, and our
> runner as one of the (at least) two independent implementations the scheme
> should require.
>
> Full strategy document with the evidence base (chapter-by-chapter state,
> the 2017 SPECCNF-1 review point-by-point, prior art from DICOM / OpenID /
> Inferno / IHE): [link]. @pablo @thomas.beale @birger.haarbrandt @sebastian.iancu —
> you built the 2021–22 foundation; does this direction match where you wanted
> it to go, and what would you change before this goes to a SEC agenda?

## Appendix B — SPECCNF-1 / SPECCNF-6 Jira comment draft

> We've written up a concrete proposal to finish what this ticket started,
> including answers to the questions in [the 2017 review comment]: a normative
> template + JSON schema for Conformance Statements (your "first step before
> any testing"), explicit certificate governance (who creates / grants /
> verifies, as a four-rung ladder starting with published statements and
> OpenID-style self-certification), platform-scope discipline, and no manual
> testing. Discussion thread with the full document: [Discourse link]. Happy to
> bring it to a SEC call if there's interest.

## Appendix C — specifications-CNF GitHub issue draft

> **Title: Proposal: machine-readable Platform Conformance Test Schedule (single normative source, generated prose, runner-independent)**
>
> Today the schedule exists as AsciiDoc prose, 2017 pseudo-code under
> `scripts/`, and the Robot suite under `tests/` — three representations that
> drifted apart, none machine-checkable, and PR #5 (making the tests runnable)
> has been open since 2023. Proposal: adopt a catalogue format (one
> YAML/JSON file per test case: global ID, SM operation, spec refs, bindings,
> expected outcomes, data-set keys, profile membership, spec-version
> applicability), generate the spec pages from it, validate it in CI (schema,
> ID uniqueness/no-reuse, spec-ref resolution), and treat every runner —
> including the Robot suite — as a downstream implementation verified against
> a shared reference pack. Two worked examples attached
> (`I_EHR_SERVICE.create_ehr-no_status` from the fleshed master06, and an AQL
> WHERE case for the empty master11). We volunteer the JSON Schemas, the CI
> workflow, the master06 conversion, and a drafted master11/AQL chapter as the
> pilot PR series. Discussion: [Discourse link].

## Appendix D — source register

- Vendored CNF snapshot: `docs/specs/openehr/CNF/` @ `33251d2a`
  (`PROVENANCE.md`); key files cited inline above.
- Published component: <https://specifications.openehr.org/releases/CNF/development>
  (Guide / Platform Test Schedule / Profiles / Certificate, all DEVELOPMENT).
- Repo: <https://github.com/openEHR/specifications-CNF> — master last content
  2022; development = Antora migration (May 2026); PR #5 open since 2023-06-11;
  issues #1/#2 from 2017.
- Jira: [SPECCNF-1](https://openehr.atlassian.net/browse/SPECCNF-1) (+ the
  [Pablo Pazos review, comment 22500](https://openehr.atlassian.net/browse/SPECCNF-1?focusedCommentId=22500)),
  [SPECCNF-6](https://openehr.atlassian.net/browse/SPECCNF-6); Release-1.0.0
  unreleased (target 2018-12-28).
- Wiki: [openEHR Conformance (2017)](https://openehr.atlassian.net/wiki/spaces/spec/pages/73367558/openEHR+Conformance),
  [Alkmaar SEC notes (2017)](https://openehr.atlassian.net/wiki/spaces/spec/pages/94181296/Conformance+Notes+-+SEC+meeting+Alkmaar+2017).
- Discourse: threads
  [1335](https://discourse.openehr.org/t/conformance-testing/1335),
  [1616](https://discourse.openehr.org/t/openehr-conformance-conformance-levels-conformance-scopes/1616),
  [1851](https://discourse.openehr.org/t/conformance-roadmap-2021/1851),
  [2239](https://discourse.openehr.org/t/conformance-framework-description/2239),
  [2285](https://discourse.openehr.org/t/openehr-conformance-verification-design-document/2285),
  [2358](https://discourse.openehr.org/t/conformance-schedule-progress-data-types/2358),
  [2373](https://discourse.openehr.org/t/conformance-testing-implementation-alternatives/2373).
- Ecosystem: [ehrbase/conformance-testing-documentation](https://github.com/ehrbase/conformance-testing-documentation)
  (AQL suites + fixtures, last push 2025-01-30);
  [CaboLabs openEHR Conformance Framework](https://www.cabolabs.com/blog/article/openehr_conformance_framework-61ef4f513f7c5.html).
- Prior art: [OpenID certification](https://openid.net/certification/);
  [Inferno](https://inferno-framework.github.io/docs/) +
  [inferno.healthit.gov](https://inferno.healthit.gov/);
  [IHE testing programs](https://www.ihe.net/testing/);
  [DICOM standard](https://www.dicomstandard.org/current) (PS3.2 conformance).
- Our instrument: `tools/conformance/` (ECC), latest committed baseline
  `docs/conformance/ehrbase-rs/CONFORMANCE_REPORT.md` (402 case×format
  executions · 384 passed · 0 failed · 18 N/A; CORE PASS / STANDARD PASS /
  OPTIONS OBTAINED).
