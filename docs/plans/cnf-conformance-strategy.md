# openEHR conformance & certification strategy — the CNF 2.0 upstream proposal (v2)

*Tracker: [#197](https://github.com/rubentalstra/ehrbase-rs/issues/197). Plan-file
lifecycle applies: this document is deleted in the PR that closes #197 (i.e. when
the proposal has been delivered upstream and the accepted workstreams have their
own issues). Evidence verified 2026-07-21 against the vendored CNF snapshot
(`docs/specs/openehr/CNF/`, commit `33251d2a`), the live
[openEHR/specifications-CNF](https://github.com/openEHR/specifications-CNF)
repository, the SPECCNF Jira project, the openEHR Discourse archive, the
OJ text of Regulation (EU) 2025/327, and the ISO/CASCO, IHE, and ONC primary
sources listed in Appendix D.*

*v2 (2026-07-21): grounded in the ISO conformity-assessment corpus
(ISO/IEC 17000-series, 17050, 17067, 9646) and the EHDS regulatory clock;
pitch rebalanced to lead with governance + resourcing; worked examples fixed
(case-core/binding split, deterministic AQL with an explicit match vocabulary,
a content decision-table example added); procurement pack and anti-gaming
registry rules added; effort claims right-sized. All thirteen findings of the
v1 adversarial review are addressed.*

*v3 (2026-07-21): §8 replaced by the full production design of the CNF 2.0
normative artifact set — derived from verbatim extractions of the fleshed
schedule chapters (master03/04/06/07/17.3), the decomposed ITS-REST 1.1.0
OpenAPI, and the STABLE Simplified Formats spec: case-core field contract,
per-SM-operation bindings with real status/header mappings, the outcome-kind
taxonomy, the ambiguity register (AMB-1…7), the typed assertion vocabulary,
corpus-manifest governance, seven fully-encoded official pilot cases, and
field-level ICS/results/IXIT schemas. New §16: the production implementation
plan (upstream PR series U1–U8 + this codebase's ECC adoption W1–W6).*

---

## 1. Executive summary

openEHR has no working conformance and certification story. The CNF component
defines the right concepts — a Conformance Guide, a Platform Conformance Test
Schedule, Profiles, a Certificate — but has been frozen since March 2022, has
never cut its Release 1.0.0 (planned for December 2018), leaves the entire
assessment layer `TBD`, has zero official test cases for AQL, and its only
executable artifact is one vendor's Robot suite that no longer runs out of the
box. Meanwhile two clocks are running: **procurement** of openEHR CDRs keeps
accelerating with nothing verifiable to require, and in Europe the **EHDS
regulation** (in force March 2025) is building a mandatory, self-assessed,
CE-marked conformity culture for EHR systems around implementing acts due
**March 2027** — a frame in which openEHR currently does not figure at all.

This document proposes **CNF 2.0** to the openEHR SEC. The 2021–2022 community
design was right — the Schedule / Profile / Statement / Certificate vocabulary,
SM-anchored test cases, technology profiles, the global test-case ID scheme,
the certification maturity ladder. What failed was not the design. It was the
operating model: one person's spare time, funding tied to one project, no owner
for the whole, one vendor-specific harness. CNF 2.0 therefore leads with the
operating model and uses engineering to make it cheap to run:

1. **Govern and resource it so it cannot stall again.** A CNF maintainer group
   chartered under openEHR International (voted decisions, no single-vendor
   majority, no unilateral veto), the normative repo, trademark and badge owned
   by openEHR International, a recurring program funding line rather than
   project money, and ≥2 competing vendors co-authoring the schema **before**
   any format decision is ratified. We commit named engineering effort to the
   pilot series; the ask to other vendors is matching co-commitment.
2. **Make the Test Schedule machine-readable — the mechanism that makes rule 1
   affordable.** One versioned catalogue (IDs, SM operation anchors, spec
   citations, bindings, data sets, expected outcomes, profile membership,
   spec-version applicability) from which the human-readable spec pages are
   *generated* and against which **any** harness — Robot, Rust, Spock, Postman
   — can prove itself. In ISO terms (§6): the schedule is the **Abstract Test
   Suite**; the runners are Executable Test Suites; CI enforcement replaces a
   bottleneck maintainer. This is the same machine-readable philosophy openEHR
   already applies everywhere else (BMM for the RM, OpenAPI for REST) —
   conformance is the one component still written only as prose.
3. **Define certification with international vocabulary, on the EHDS clock.**
   The ladder that has been `TBD` since 2017 becomes an **ISO/IEC 17067
   conformance scheme owned by openEHR International**, with rungs labelled by
   ISO/IEC 17000 attestation level: a published-statement registry and attested
   **supplier's declaration of conformity** (ISO/IEC 17050) first, community
   verification events next, delegated ISO/IEC 17025-lab + 17065-certifier
   assessment last — the exact architecture IHE and the US ONC program already
   run, and the exact self-assessment + open-source-testing-environment shape
   the EHDS regulation mandates for EHR systems in Europe.

ehrbase-rs contributes its ECC framework — a working 394-case,
both-wire-formats, machine-computed-verdict conformance instrument built on the
CNF profiles model — as a working draft and one reference implementation.
Explicitly **not** as "the standard": the standard must be community-owned,
vendor-neutral, and multi-harness by construction. Our role is to donate
methodology, case designs, and engineering effort under the community's
licence, and to be the first SUT assessed by whatever the community ratifies.

### What is new here versus the 2021–2022 design

Nothing in the *conceptual* model is claimed as new: the four-artifact
vocabulary, the SM-anchors/ITS-executes split, technology profiles, and the
global test-case ID scheme are the 2021–2022 community's work
([Discourse 1616](https://discourse.openehr.org/t/openehr-conformance-conformance-levels-conformance-scopes/1616),
[1851](https://discourse.openehr.org/t/conformance-roadmap-2021/1851),
[2358](https://discourse.openehr.org/t/conformance-schedule-progress-data-types/2358)),
and this proposal completes that direction rather than replacing it. The
deltas are exactly five: **(a)** the schedule as one-file-per-case *data* with
generated prose, **(b)** CI enforcement of the derivation chain on the spec
repo, **(c)** computable Statement/results schemas with mechanically computed
verdicts, **(d)** the governance/resourcing charter, and **(e)** the ISO/EHDS
grounding that makes the program legible to procurement and regulators.

---

## 2. The problem — and why now

- **Procurement.** Tenders increasingly name openEHR, and there is nothing
  verifiable to require. The documented proof is Catalonia: the CatSalut CDR
  platform tender (closed Dec 2022, awarded ~€8.5M to UTE IBM–Viewnext,
  [Discourse 3910](https://discourse.openehr.org/t/region-of-catalonia-award-of-the-tender-for-the-service-of-cdr-platform/3910))
  had to define "openEHR conformance" via behavioural latency SLAs (query
  ≤40 ms p95, write ≤60 ms p95) because no certificate, statement format, or
  official test result existed to reference. Sweden's Karolinska framework
  ("Tender Area 1: openEHR-based Software"), Malta's national EHR, Slovenia's
  national CRPD, and Wales' National Data Resource all name openEHR the same
  unverifiable way. The 2021 board decision to fund conformance work cited
  precisely this ([Discourse 1851](https://discourse.openehr.org/t/conformance-roadmap-2021/1851)).
- **The EHDS clock (Europe).** Regulation (EU) 2025/327 entered into force
  26 March 2025 and creates a mandatory conformity regime for EHR systems:
  manufacturer **self-assessment**, an **EU declaration of conformity**
  (Art 39), **CE marking** (Art 41), a public registration database (Art 49),
  and a Commission-provided **open-source digital testing environment**
  (Art 40) whose positive results yield a presumption of conformity. Common
  specifications and the EEHRxF exchange-format implementing acts are due
  **26 March 2027**; enforcement waves hit 2029/2031. openEHR is **not** in
  that frame today — the EEHRxF deliverables are HL7 FHIR logical models and
  the Xt-EHR conformity-assessment scheme (D8.2, May 2026) is IHE/FHIR-based —
  so the realistic positioning is §6.5: openEHR as the conformant
  persistence layer *behind* the EHDS interoperability component, with a
  conformance program that speaks the same self-assessment + open-source
  testing-environment language. Either openEHR has a credible, ISO-legible
  conformance program before the 2027 acts crystallize the ecosystem's habits,
  or "conformance" in European health IT becomes a FHIR-only concept by
  default.
- **Vendor fairness.** Every vendor today self-declares against a different
  private checklist. The only shared harness encodes one vendor's behaviour
  (EHRbase's Robot suite), which biases "conformance" toward one
  implementation's quirks — the exact failure the CNF Guide warns against.
- **Spec quality feedback.** A real conformance suite is the best defect
  detector the specifications themselves can have. The CNF content chapters
  proved this in 2022; the stub chapters (AQL!) mean the flagship capability
  has no executable definition of correct behaviour.
- **Community credibility.** HL7 ships Inferno test kits with regulatory
  teeth; IHE runs Connectathons and an ISO-based conformity-assessment scheme;
  OpenID runs self-certification at scale; the EHDS makes automated
  conformity testing a legal instrument. openEHR's conformance page still
  says `TBD` where "how do I get certified?" should be — and the HL7–openEHR
  convergence track (Dublin joint statement, May 2026) plus EHRCON26's heavy
  EHDS programme make this the political window to fix it.

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
| Conformance Certificate | DEVELOPMENT | 2021 | A **fictional worked example** ("BestEHR 2.4", "ACME EHR systems LLC", dated 2017; `certificate/master03-certificate.adoc`). No issuance procedure, assessor accreditation, validity period, or revocation anywhere. The book advertises BASIC-SEC/BASIC-PRIV ratings for which no defining test cases exist. |

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

Two ID families exist and are kept unchanged by this proposal: functional
`<SERVICE_COMPONENT>.<operation>-<case>` (e.g. `I_EHR_SERVICE.create_ehr-main`)
anchored to SM interface operations, and content `CONT-<TYPE>-<scenario>`
decision tables — the global ID scheme announced in 2022 as spanning "REST
API, content, everything"
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
  Conformance Guide" — In Progress since **October 2021**, zero comments since.
- Jira Release-1.0.0: release date 2018-12-28, **never released**.
- specifications-CNF git: last content work 2022 (the schedule updates);
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
| Sep 2022 | Stall on record: implementers ask about de-EHRbase-ifying the Robot suite; the answer is "I don't have a timeline since I'm doing it in my free time. Help is welcome." ([2373](https://discourse.openehr.org/t/conformance-testing-implementation-alternatives/2373)). |
| 2023–2026 | PR #5 unmerged; rendering fixes; Antora migration. No content. Meanwhile: EHDS adopted (Mar 2025), HL7–openEHR joint statements (Amsterdam Jun 2025, Dublin May 2026), EHRCON26 programmes a heavy EHDS track and one conformance-testing session — and no product-certification launch. |

### 4.2 What the 2021–2022 design era settled (keep all of it)

- **The four-artifact vocabulary**: Conformance **Schedule** (everything
  testable) / **Profile** (a viable product type's capability set) /
  **Statement** (what a product claims + which tests pass) / **Certificate**
  (statement + report + attestation).
- **SM names the capabilities, an ITS executes the tests**: test *definitions*
  anchor to Service Model operations (`I_EHR_SERVICE.create_ehr`); test
  *execution* binds to a concrete ITS (REST + a serialization).
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

These four causes are what §1's ordering answers: the governance/resourcing
charter (§13) addresses 1–3 directly; the machine-readable schedule + CI (§8)
is the mechanism that makes 1 and 4 structurally cheap rather than heroic.

### 4.4 SPECCNF-1 comment 22500 (Pablo Pazos, Aug 2017) — still the best requirements list

Nine years old and almost fully unaddressed; CNF 2.0 answers it point by point:

- The spec should include **guidance + format for writing Conformance
  Statements** — "the first step before any testing, it actually defines what
  can be tested" (his model: DICOM PACS conformance statements). → §8.10: the
  Statement is a normative, computable schema; in ISO terms an ICS proforma
  with ISO/IEC 17050-1 content.
- The Certificate section raises unanswered governance questions verbatim:
  *"what is this? how is it created? who can create it? who can grant it? who
  verifies it?"* → §9 answers each with ISO/IEC 17000 attestation levels and
  named roles.
- Scope discipline: functional vs non-functional conformance points stated up
  front; don't redefine ISO 9126/25010 quality terms; certify **platforms**
  explicitly; no assumptions that imply a web UI; avoid manual testing.
  → §6.3 scopes conformance as ISO/IEC 25010 *functional suitability* and
  §7.8 keeps non-functional out.
- Don't hard-code REST as the only access method conceptually. → the §8.3/§8.4
  case-core/binding split makes that structural.
- Archetype-validation conformance points need precise definitions. → §11
  roadmap item 2 + the content decision-table schema (§8.9, pilot 5).

## 5. Prior art — how other standards run conformance

| Program | Model | What to copy |
|---|---|---|
| **DICOM conformance statements** ([DICOM PS3.2](https://www.dicomstandard.org/current)) | Every product publishes a standardized conformance statement; procurement compares statements; no central certification. | The **statement as the legally load-bearing artifact**, with a normative template. CNF 2.0 upgrade: make it computable. |
| **OpenID Foundation certification** ([openid.net/certification](https://openid.net/certification/)) | **Self-certification**: vendor runs the official open-source suite, submits results + a signed legal attestation, pays a small fee, gets listed on the public certified page. Runs at scale since 2015. | The **cheapest credible rung**: official suite + published results + attestation + public registry. |
| **HL7 FHIR / ONC Inferno** ([inferno.healthit.gov](https://inferno.healthit.gov/), [framework docs](https://inferno-framework.github.io/docs/)) | Open-source test kits per implementation guide; the (g)(10) kit is an approved test method inside a regulatory certification program. Structure: policy (ASTP/ONC, 45 CFR 170) → open-source test method (Inferno) → **ISO/IEC 17025** labs (ONC-ATLs, NVLAP-accredited) → **ISO/IEC 17065** certifiers (ONC-ACBs) → accreditor (ANSI/ANAB), plus surveillance + the public CHPL product list. | **Test kits as maintained open-source products**; machine-readable expectations; and the five-layer separation: the standards body never tests or certifies its own conformity — it owns criteria and approves test methods. |
| **IHE Connectathons + Conformity Assessment Scheme** ([ihe.net/testing](https://www.ihe.net/testing/)) | Annual supervised peer-testing events (results published) plus a formal scheme **explicitly built on ISO/IEC 17025 + 17067**, with certification bodies under ISO/IEC 17065 evaluating accredited-lab results. | The **community verification event** rung (a conformance-thon at EHRCON fits openEHR's culture) and the canonical lab/certifier split for the eventual top rung. |
| **EHDS Article 40** ([Regulation (EU) 2025/327](https://eur-lex.europa.eu/eli/reg/2025/327/oj/eng)) | The Commission must provide an **open-source digital testing environment** for the harmonised EHR components; manufacturers must use it pre-market and file the results; positive results = presumption of conformity. Conformity is **manufacturer self-assessment** + EU declaration + CE marking + public registration — no notified bodies. | Regulatory confirmation of the whole shape: automated open-source suite + self-assessment + declaration + public registry is now *the law's own architecture* for EHR conformity in Europe. |
| **openEHR's own ISO 18308 Conformance Statement** ([PDF](https://specifications.openehr.org/releases/1.0.2/requirements/iso18308_conformance.pdf)) | A requirement-by-requirement statement of openEHR's conformance to ISO 18308, exceptions indexed. | **In-family precedent**: openEHR has already authored a requirement-indexed conformance statement; the computable Statement is its machine-readable evolution. |

Composite lesson: nobody starts with third-party certification. Working
programs start with an **official, runnable, vendor-neutral suite + a public
registry of published results**, add legal attestation (OpenID, EHDS DoC),
then events (IHE), then accreditation (ONC). That is exactly the 2021 ladder —
the ladder was right; the bottom rung was never built.

## 6. The international frame — ISO vocabulary, and the regulatory clock

CNF 2.0 should adopt the international conformity-assessment vocabulary
instead of inventing terms. Everything this proposal describes has a settled
ISO name, which buys procurement- and regulator-legibility at zero design
cost, and openEHR gains the right to say its program is *structured per
ISO/IEC 17067* rather than home-grown.

### 6.1 The conformity-assessment mapping (CASCO toolbox)

| CNF 2.0 concept | International term to adopt | Standard to cite |
|---|---|---|
| The machine-readable Test Schedule | **Abstract Test Suite (ATS)**; per-case **test purposes** | ISO/IEC 9646-1/-2 (ITU-T X.290/X.291) |
| A concrete runner (Robot, ECC, Spock…) | **Executable Test Suite (ETS)** realized by a **Means of Testing** | ISO/IEC 9646-4/-5 |
| The computable Conformance Statement | **Implementation Conformance Statement (ICS)** from a normative **proforma**; legally a **first-party attestation / supplier's declaration of conformity** | ISO/IEC 9646-7; ISO/IEC 17050-1; ISO/IEC 17000 |
| Evidence linked to a statement | **Supporting documentation** (traceability, availability, retention) | ISO/IEC 17050-2 |
| Deployment parameters to run against a live SUT (base URL, auth, template-id policy…) | **IXIT** (Implementation eXtra Information for Testing) | ISO/IEC 9646-1 (ITU-T X.292) |
| "The Statement selects which schedule cases apply" | **ICS-driven test selection**; checking the Statement's internal legality = **static conformance review** | ISO/IEC 9646-1/-7 |
| The product / the deployed system | **IUT** / **SUT** | ISO/IEC 9646-1 |
| A run's outcome | **verdicts** (pass / fail / inconclusive) + the **conformance test report** | ISO/IEC 9646-1; report shape per ISO/IEC/IEEE 29119-3 |
| Registry / self-certification rungs | **First-party attestation** (SDoC) | ISO/IEC 17000; 17050-1/-2 |
| Community verification rung | **Second-party attestation** | ISO/IEC 17000 |
| Accredited assessment rung | **Third-party attestation → certification** by an **ISO/IEC 17065** body using an **ISO/IEC 17025** lab | ISO/IEC 17065; 17025 |
| The program itself | A **conformance/certification scheme**, openEHR International as **scheme owner** | ISO/IEC 17067 (Type 1a for version-scoped self-declared conformance; Type 6 framing if ongoing accredited certification ever ships) |
| "Conformance" scope | **Functional suitability** (completeness + correctness) — nothing else | ISO/IEC 25010; software-product evaluation per ISO/IEC 25051 |

### 6.2 ISO/IEC 9646 — the 35-year-old blueprint for exactly this design

The OSI Conformance Testing Methodology and Framework standardized, in 1991,
the precise architecture CNF 2.0 proposes: a supplier fills in a published
**ICS proforma** declaring which capabilities are implemented; the ICS
**selects** which cases from the **Abstract Test Suite** apply; suppliers also
provide the **IXIT** (instance parameters needed to actually run the tests);
runners realize the ATS as Executable Test Suites; outcomes are recorded as
pass/fail/inconclusive verdicts in a standardized test report. ETSI, Bluetooth
SIG, and USB-IF still run their programs on ICS/IXIT vocabulary today. Citing
this lineage matters strategically: the machine-readable-schedule pitch is not
an invention to evaluate but settled international practice to adopt — the
2022 global-ID work was already converging on it.

### 6.3 Scope discipline via ISO/IEC 25010 (answering the 2017 review)

Conformance under CNF 2.0 attests **functional suitability** — functional
completeness and correctness against the openEHR specifications — and nothing
else. Performance efficiency, reliability, security, maintainability are
distinct ISO/IEC 25010 characteristics: out of conformance scope (per the CNF
Guide), referenced by their ISO names rather than redefined (the 2017 review's
point), with the Statement schema reserving fields for products to declare
them separately. ISO/IEC 25051 (conformity evaluation of ready-to-use software
products) and ISO/IEC/IEEE 29119-3 (test documentation shapes) are the
supporting citations for the evaluation procedure and report formats.

### 6.4 Legal weight of self-declaration (the phrasing to adopt)

Under ISO/IEC 17050-1 the supplier's declaration is made on the supplier's
sole responsibility; references to any third-party results "are not to be
interpreted as reducing the responsibility of the supplier". CNF 2.0's lower
rungs should carry exactly this framing, verbatim in the Guide:

> *A published Conformance Statement is a first-party attestation
> (ISO/IEC 17000) in the form of a supplier's declaration of conformity
> (ISO/IEC 17050-1). openEHR International registers and publishes it but does
> not verify or endorse it; responsibility for its accuracy rests entirely
> with the declaring supplier.*

That sentence gives self-declaration recognized legal weight without implying
openEHR liability or endorsement — and it is the same legal shape as the EHDS
EU declaration of conformity.

### 6.5 The EHDS clock — honest positioning

Facts (OJ text, [Regulation (EU) 2025/327](https://eur-lex.europa.eu/eli/reg/2025/327/oj/eng)):
in force 26 March 2025; general application 26 March 2027; EHR-system
obligations phase in 2029 (patient summaries, ePrescriptions/eDispensations)
and 2031 (imaging, labs, discharge reports). Every in-scope EHR system must
embed two **harmonised software components** (European interoperability
component; European logging component; Art 25, Annex II), pass the
Commission's **open-source digital testing environment** (Art 40), and ship
with a manufacturer **self-assessed EU declaration of conformity** (Art 39),
**CE marking** (Art 41) and public registration (Art 49). Common
specifications + the EEHRxF exchange format arrive as implementing acts by
**26 March 2027** (Arts 36, 15), pre-drafted by the Xt-EHR joint action —
whose deliverables are **HL7 FHIR logical models** and whose
conformity-assessment scheme (D8.2, May 2026) is **IHE/FHIR-based**. The
regulation itself names no standard at all.

Honest implications:

- **openEHR is not, and should not claim to be, the EHDS conformity route.**
  The exchange layer is FHIR/IHE territory. Overclaiming would be factually
  wrong and reputationally expensive.
- **The credible position**: an openEHR CDR is the system-of-record *behind*
  the EHDS interoperability component. CNF 2.0 certifies the CDR's openEHR
  conformance — and its roadmap includes verifying that a conformant CDR can
  drive the EEHRxF export faithfully (the openEHR→FHIR seam; EHRCON26 already
  programmes "Conformance Testing openEHR with FHIR TestScript"). An openEHR
  conformance result becomes *complementary evidence alongside* an EHDS DoC,
  never a competitor to it.
- **Architectural alignment is free**: EHDS Art 40 mandates precisely the
  shape CNF 2.0 proposes (automated, open-source, self-run testing +
  self-assessed declaration + public registry). Building CNF 2.0 in that shape
  makes openEHR conformance culturally and procedurally compatible with what
  every European vendor will be doing anyway from 2027.
- **Timing**: the program needs to exist — visibly, with a registry and a
  running suite — before the March 2027 implementing acts and the 2027–2029
  procurement wave define "conformity" habits without openEHR in the room.

## 7. Design principles for CNF 2.0

1. **Machine-readable normative source, generated prose.** Like BMM → RM and
   OpenAPI → REST, the Schedule's normative form is data; the published spec
   pages are rendered from it. A test case that isn't in the catalogue doesn't
   exist; a catalogue entry without spec citations doesn't build.
2. **SM anchors semantics; ITS bindings execute — structurally.** Every case
   is a protocol-neutral core (SM operation / content constraint, spec
   citations, pre/postconditions, logical outcomes); wire specifics live in
   separate per-ITS binding artifacts (§8.4). New protocols add binding files,
   never new suites.
3. **Harness independence by construction.** Catalogue + data sets + schemas +
   verdict rules are the contract; every runner is a downstream implementation
   verified against a shared reference pack (§8.12). No harness is normative.
4. **Verdicts are computed, never asserted.** Profile verdicts derive
   mechanically from the results file. A certificate row a human typed is a
   defect.
5. **Honesty is structural.** Coverage bounds printed, N/A adjudications
   cited, corpus defects adjudicated in a register rather than silently
   edited, both directions published when comparing products, registry rows
   labelled by attestation level so self-declaration is never mistaken for
   certification.
6. **Vendor neutrality is testable.** No vendor image names, endpoints, auth
   flows, or behavioural quirks in normative artifacts; fixtures carry spec
   citations, not `EhrBase ref:` markers; reference expectations are
   adjudicated against spec text, never against whichever SUT emitted them.
   CI enforces what it can; the maintainer charter (§13) enforces the rest.
7. **Versioned like every other component.** Cases pin spec-version
   applicability ranges; a statement names the schedule release + tech profile
   it was earned against; within-major supersets follow openEHR's release
   strategy.
8. **Scope discipline.** Platform (CDR) profile first. Conformance =
   ISO/IEC 25010 functional suitability only (§6.3); other system classes get
   profile families later; performance/volumetrics stay out (per the Guide)
   with reserved declaration slots in the Statement.
9. **Adopt international vocabulary** (§6.1) — ATS/ICS/IXIT/attestation
   levels/scheme-owner — instead of coining terms.

## 8. The CNF 2.0 normative artifact set — the production design

This section is the full design, not a sketch. It is derived from three
extractions performed against the vendored specs on 2026-07-21: (a) the
fleshed chapters of the official Test Schedule
(`platform_test_schedule/master03/04/06/07/17.3` — the real case format, the
16-row create-EHR matrix, the per-row iteration law, the versioning cases,
the DV_QUANTITY decision tables); (b) the ITS-REST 1.1.0 wire contract, which
in Release-1.1.0 is a *decomposed OpenAPI* (`specifications/operations/*.yaml`
+ `responses/*.yaml` + `parameters/header/*.yaml`) — there are no per-API
prose status tables, so the binding layer below is driven from the OAS
fragments; and (c) the STABLE Simplified Formats specification
(`ITS-REST/docs/simplified_formats/master02–06`). Every rule below carries
its source.

**Design law**: everything the official schedule's fleshed chapters express
today MUST be representable losslessly in the case model, and nothing
wire-level may appear in a case core. The schedule itself never states an
HTTP status code (verified across master04/06/07 — error expectations are
prose exemplars like ``"EHR with <ehr_id> does not exist"``); the codes live
only in the runners today, which is precisely the layer the operation
bindings make normative.

### 8.1 The testable surface — what the case model must carry

Requirements extracted from the real material (each drove a schema feature
below):

1. **A "test" = one case × one data set** (`master03-overview.adoc`: "A
   'test' is therefore the execution of a particular test case with a
   particular data set") → parameter matrices are first-class (§8.3
   `parameters`).
2. **Pre/postconditions are re-established around every data-set row**
   (`master04` §iteration semantics: "the pre-conditions and post-conditions
   apply to the run for X") → `iteration: reset_per_row` (§8.3).
3. **State flows between steps and between cases**: the server-assigned
   `ehr_id` "should be read from the response" and replayed
   (`master06` create_ehr-same_ehr_twice); `preceding_version_uid` "should be
   the version uid from the COMPOSITION created in step 1" (`master07`
   update_composition-event); a create row's expected `is_queryable` values
   are verified in a *different* case
   (`master06` get_ehr_status-get_by_ehr_id) → captures, variable references,
   and `verified_by` links (§8.3).
4. **Error expectations are kinds, not codes**: the schedule distinguishes
   duplicate-EHR vs non-existent-EHR vs non-existent-OPT vs
   data-validation-failure vs template-mismatch for the *same* operations,
   always as prose → the outcome-kind taxonomy (§8.5), mapped to wire only in
   bindings (§8.4).
5. **Prerequisites are typed server state**: "The server should be empty (no
   EHRs, no commits, no OPTs)", "An EHR with known ehr_id should exist",
   "The EHR should have no commits", "The OPT … should exist on the server"
   (master06/07) → the `requires` block (§8.3).
6. **Fixtures carry adjudicated verdicts**: the Robot corpus encodes the
   defect in the filename (`007_ehr_status_is_modifiable_missing.json`,
   `…__invalid_wrong_structure.json`) and uses runtime placeholders
   (`__AUTO-GENRATED-BY-TEST__`) → the corpus manifest (§8.8).
7. **Versioning is asserted in RM terms**: `VERSION.commit_audit.change_type`
   CREATE/MODIFY, `lifecycle_state = openehr::523|deleted|`, version counts,
   at-time/at-version selection (master07) → the `version` assertion family
   (§8.6); ETag/If-Match are the REST realization only (§8.4).
8. **Content decision tables carry structured constraint literals** —
   ranges `5.0..10.0`, lists `[cm 5.0..10.0, m]`, term codes `openehr::122
   (length)` / `local::at0005`, and violation categories that name RM/schema
   rules, **named RM invariants** (`limits_consistent (invariant)`), ISO 8601
   rules, and constraint clauses (`C_DV_QUANTITY.list: …`) (master17.3) →
   the literal grammar + violation categories (§8.8).
9. **Applicability guards exist per case** (DV_SCALE only at RM ≥ 1.1.0;
   list constraints tool-dependent — master17.3 NOTEs) → `applies` +
   `guards` (§8.3).
10. **The same logical case runs across multiple representations**
    (XML/JSON/FLAT/STRUCTURED/TDD "content check" language in master07) →
    format axes (§8.7).
11. **The wire layer needs specific primitives** (from the OAS extraction):
    exact-status assertion; header presence + value patterns (weak ETag
    `W/"…::…::N"`, `Location` on 201 only, `Content-Type` unless 204,
    `Preference-Applied`); `Prefer`-conditional body selection
    (full | `{uid}` | empty); ETag capture → `If-Match` replay; the
    media-type matrix incl. 406/415 negatives; and a deliberately **loose
    error-body assertion** (§8.5 ambiguity register).

### 8.2 The artifact set

Seven normative, versioned-together artifact families in specifications-CNF
(all with published JSON Schemas; YAML/JSON encodings equivalent — the schema
is the norm):

| # | Artifact | Path (proposed) | Content |
|---|---|---|---|
| 1 | **Case cores** | `schedule/<component>/<CASE_ID>.yaml` | Protocol-neutral test cases (§8.3) — the Abstract Test Suite |
| 2 | **Operation bindings** | `bindings/<its>/<SM_OPERATION>.yaml` | Per-ITS wire realization of each SM operation's outcomes/captures (§8.4) |
| 3 | **Outcome-kind vocabulary** | `vocab/outcomes.yaml` | The closed taxonomy cases and bindings share (§8.5) |
| 4 | **Governed corpus + manifest** | `corpus/**` + `corpus/MANIFEST.yaml` | Fixtures, templates, generated-set recipes, adjudicated verdicts (§8.8) |
| 5 | **Ambiguity register** | `registers/ambiguities.yaml` | Known spec silences/divergences with normative handling (§8.5) |
| 6 | **ICS / results / IXIT schemas** | `schemas/{statement,results,ixit}.schema.json` | The conformance-statement, test-report, and SUT-parameter contracts (§8.10) |
| 7 | **Verdict rules** | `schemas/verdicts.md` (normative prose) + reference impl | Pure-function profile computation (§8.11) |

The published spec pages (the human-readable schedule) are **generated** from
1–5; the derivation-square CI (§8.13) keeps every artifact internally linked.

### 8.3 The case core — full field definitions

One file per case. Normative fields (∎ = required):

| Field | Type | Semantics |
|---|---|---|
| `id` ∎ | string | Global CNF id, existing families kept: `<SERVICE_COMPONENT>.<operation>-<variant>` (functional) / `CONT-<TYPE>-<variant>` (content). Never reused; retired cases keep the id with `status: retired`. |
| `kind` ∎ | `functional \| content` | Selects which optional blocks are meaningful. |
| `status` | `active \| retired \| draft` | Default `active`. |
| `component` ∎ | enum | EHR, EHR_COMPOSITION, EHR_CONTRIBUTION, EHR_DIRECTORY, DEFINITION_ADL14, DEFINITION_ADL2, DEFINITION_QUERY, QUERY, DEMOGRAPHIC, ADMIN, MESSAGING, CONTENT, SIMPLIFIED_FORMATS, … |
| `sm_operation` | string | Functional cases: the SM anchor (`I_EHR_SERVICE.create_ehr`). CI resolves it against the SM component list. |
| `rm_class` | string | Content cases: the RM/AM class under test (`DV_QUANTITY`). |
| `test_purpose` ∎ | string | The ISO/IEC 9646 test purpose — one narrow conformance requirement, prose. |
| `description` ∎ | string | The schedule's Description row. |
| `spec_refs` ∎ | string[] | Citations (component + document + section). CI link-checks them. |
| `applies` | map | Spec-version applicability ranges (`rm: ">=1.0.2"`, `aql: ">=1.1"` …). |
| `guards` | string[] | Non-version run conditions, each spec-cited (e.g. "modeling tool supports C_DV_QUANTITY list constraints — master17.3 NOTE"). A failed guard ⇒ `not-applicable`, citation mandatory. |
| `profiles` ∎ | string[] | Profile-book capability membership (drives ICS selection, §8.11). |
| `requires` | block | Typed prerequisites (below). |
| `parameters` | block | The data-set dimension (below). |
| `flow` ∎ (functional) | Step[] | Ordered steps (below). |
| `decision_table` ∎ (content) | block | Columns + rows (below). |
| `postconditions` | Assertion[] | Typed assertions (§8.6) evaluated after the flow, per row. |
| `verified_by` | string[] | Ids of cases that verify this case's deeper postconditions through separate reads (the master06 create→get pattern). CI checks the links resolve. |
| `ambiguities` | string[] | Ids into the ambiguity register that this case is subject to. |
| `data_sets` | string[] | Corpus manifest keys used (in addition to `parameters`). |

**`requires` block** — the schedule's precondition vocabulary, typed:

```yaml
requires:
  server: empty            # empty | any        ("no EHRs, no commits, no OPTs")
  templates: []            # corpus keys that must be provisioned before the flow
  ehr: none                # none | { commits: none | any }   (an EHR with known ehr_id)
  compositions: []         # corpus keys pre-committed (for query/read suites)
```

`server: empty` is realized by runners through isolation (fresh SUT or
tenant), never by destructive cleanup of a shared system — a runner-layer
note, not a case concern.

**`parameters` block** — the data-set dimension. One mechanism serves the
functional matrices (master06) and the fixture sets (master04):

```yaml
parameters:
  iteration: reset_per_row   # reset_per_row (default, the master04 law) | single_pass
  matrix:                    # inline value matrix (master06-style)
    columns: [is_queryable, is_modifiable, subject, other_details, ehr_id]
    rows: [ ... ]            # each row binds ${row.<column>}
  fixture_set:               # external-fixture iteration (master04-style)
    - { data_set: <corpus key>, expected: <outcome kind>, defect: "<why>", spec_ref: "<citation>" }
```

Reserved matrix columns: `expected` (per-row outcome override) and
`violates` (content: the violated-constraint list, §8.8 categories). Rows
without `expected` inherit the flow's expectations.

**`flow` steps**:

```yaml
flow:
  - step: 1
    call: create_ehr                     # SM operation (short form resolves against sm_operation's interface)
    with: { ehr_status: ${row.ehr_status} }   # inputs; ${row.*}, ${<capture>}, ${ds:<corpus key>} references
    expect: created                      # outcome kind (§8.5); per-row override via the `expected` column
    capture: { ehr_id: created.ehr_id }  # logical captures; bindings map them to wire locations
    assert: []                           # optional post-step typed assertions (§8.6)
```

Rules: captures are case-scoped names; a step may reference any earlier
step's captures; `expect` names exactly one outcome kind — a case that needs
"either A or B" is two cases (the schedule never disjuncts outcomes; where
the *spec* allows alternatives, that is an ambiguity-register entry, not a
loose expectation). Substeps (the schedule's `1.1`, `3.2`) are encoded as
separate steps with a `variant:` tag when they iterate different sources
(see pilot case 2).

**Content `decision_table`** (master15–17 shape, §8.8 literal grammar):

```yaml
constraint_context:
  template: <corpus key>      # the OPT carrying the constraint under test
  path: "<path to the constrained node>"
decision_table:
  columns: [<input attrs...>, <constraint attrs...>, expected, violates]
  rows: [ ... ]
```

Each row is one committed instance (generated from the row's input attrs
into the context template) + `expected: accepted | rejected` +
`violates: [...]` naming the violated rules per the §8.8 categories.

### 8.4 Operation bindings — the wire layer, per SM operation

**One binding file per SM operation per ITS** — not per case. Every case that
touches `I_EHR_COMPOSITION.update_composition` reuses the same binding;
per-case overrides exist but are a review smell. A binding maps: request
construction, each outcome kind → wire expectation, and each logical capture
→ wire source. The binding is where `Prefer`, `If-Match`, ETags, media types,
and status codes live — and each mapping cites its OAS source.

Real bindings for ITS-REST 1.1.0 (from `specifications/operations/*.yaml` +
`responses/*.yaml`):

```yaml
# bindings/its-rest/I_EHR_SERVICE.create_ehr.yaml
sm_operation: I_EHR_SERVICE.create_ehr
its: its-rest
applies: { its_rest: ">=1.0.0" }
request:
  method: POST
  path: /ehr
  body: ehr_status?                       # optional EHR_STATUS (ehr_create.yaml)
  headers:
    Prefer: "return=representation"       # default is return=minimal (Prefer.yaml); we ask for the body
formats: [canonical-json, canonical-xml]  # EHR resource is canonical-only (Accept_canonical)
outcomes:
  created:            { status: 201, headers: { ETag: present, Location: present },
                        body: prefer_conditional }   # oneOf [Ehr | {uid} | empty] per Prefer (201_EHR.yaml)
  already_exists:     { status: 409 }     # subject-id/namespace conflict when EHR_STATUS supplied (409_EHR.yaml)
  validation_failed:  { status: 400 }     # NOTE ambiguity AMB-2: no 422 enumerated on EHR create
captures:
  ehr_id:      { from: body "ehr_id.value", fallback: header Location last-segment }
  version_uid: { from: header ETag, strip: weak-quotes }
```

```yaml
# bindings/its-rest/I_EHR_COMPOSITION.create_composition.yaml
sm_operation: I_EHR_COMPOSITION.create_composition
its: its-rest
request:
  method: POST
  path: /ehr/{ehr_id}/composition
  body: composition
  headers: { Prefer: "return=representation" }
formats: [canonical-json, canonical-xml, wt-flat, wt-structured]   # Accept_LOCATABLE / ContentType_LOCATABLE
format_headers:
  wt-flat:       { Content-Type: application/openehr.wt.flat+json,       openehr-template-id: required }
  wt-structured: { Content-Type: application/openehr.wt.structured+json, openehr-template-id: required }
outcomes:
  created:            { status: 201,
                        headers: { ETag: 'pattern:W/"<versioned_object_uid>::<system_id>::1"',
                                   Location: present, Content-Type: negotiated },
                        body: prefer_conditional }                  # 201_COMPOSITION.yaml
  ehr_not_found:      { status: 404 }                               # 404_unknown_ehr_id.yaml
  validation_failed:  { status: 422, body: error_loose }            # 422.yaml; AMB-1 error body
  template_not_found: { status: 422, body: error_loose }            # same wire code; kind distinguished by fixture
  missing_template_id:{ status: 422 }                               # simplified commit without openehr-template-id
  unsupported_media:  { status: 415 }                               # Resources.md negotiation rules
captures:
  version_uid: { from: header ETag, strip: weak-quotes }            # OBJECT_VERSION_ID …::…::1
  versioned_object_uid: { from: capture version_uid, transform: root-uid }
```

```yaml
# bindings/its-rest/I_EHR_COMPOSITION.update_composition.yaml
sm_operation: I_EHR_COMPOSITION.update_composition
its: its-rest
request:
  method: PUT
  path: /ehr/{ehr_id}/composition/{versioned_object_uid}
  body: composition
  headers:
    If-Match: '"${preceding_version_uid}"'   # REQUIRED (If-Match.yaml); realizes SM preceding_version_uid
    Prefer: "return=representation"
formats: [canonical-json, canonical-xml, wt-flat, wt-structured]
outcomes:
  updated:              { status: 200,        # 204 when Prefer minimal (200_COMPOSITION_updated / 204_version_updated)
                          headers: { ETag: 'pattern:W/"…::…::<n+1>"' }, body: prefer_conditional }
  precondition_failed:  { status: 412, headers: { ETag: latest-version-uid } }  # 412_COMPOSITION.yaml, MUST
  precondition_missing: { status: 400 }       # If-Match absent → SHOULD 400 (Requests_and_responses.md §If-Match)
  not_found:            { status: 404 }       # unknown ehr_id or uid (404_unknown_ehr_id_or_uid_based_id.yaml)
  validation_failed:    { status: 422, body: error_loose }
  template_mismatch:    { status: 422, body: error_loose }          # wrong-template update (master07)
captures:
  version_uid: { from: header ETag, strip: weak-quotes }
```

```yaml
# bindings/its-rest/I_EHR_COMPOSITION.delete_composition.yaml
sm_operation: I_EHR_COMPOSITION.delete_composition
its: its-rest
request: { method: DELETE, path: /ehr/{ehr_id}/composition/{preceding_version_uid} }
outcomes:
  deleted:         { status: 204 }            # 204_version_deleted.yaml — delete is 204, never 200
  already_deleted: { status: 400 }            # 400_already_deleted.yaml
  not_found:       { status: 404 }
  conflict:        { status: 409 }            # 409_COMPOSITION_with_uid_based_id.yaml
```

```yaml
# bindings/its-rest/I_DEFINITION_ADL14.upload_opt.yaml
sm_operation: I_DEFINITION_ADL14.upload_opt
its: its-rest
request:
  method: POST
  path: /definition/template/adl1.4
  body: opt_xml
  headers: { Content-Type: application/xml }  # the ONLY accepted upload type (operation enum)
outcomes:
  created:            { status: 201 }
  already_exists:     { status: 409 }         # duplicate template_id (409_template_already_exists.yaml) — AMB-4
  validation_failed:  { status: 400, body: error_loose }
captures:
  template_id: { from: body-or-location }     # implementation latitude; AMB register
```

```yaml
# bindings/its-rest/I_QUERY_SERVICE.execute_adhoc_query.yaml
sm_operation: I_QUERY_SERVICE.execute_adhoc_query
its: its-rest
request:
  method: POST                                # spec-recommended over GET for parameterized queries
  path: /query/aql
  body: { q: ${q}, query_parameters: ${query_parameters}, offset: ${offset?}, fetch: ${fetch?} }
  headers: { Content-Type: application/json, Accept: application/json }
outcomes:
  ok:            { status: 200, headers: { ETag: present? }, body: result_set }  # 200_Query.yaml
  invalid_query: { status: 400 }              # 400_Query.yaml
  timeout:       { status: 408 }              # 408_Query.yaml
```

Binding-level normative rules (all cited from
`docs/overview/Requests_and_responses.md` + `Resources.md`):

- **ETag discipline**: value = version identifier, format-independent ⇒
  weak — `W/"…"` MUST in 1.1.0; the bare pre-1.1.0 form MAY be tolerated on
  read (a per-edition toggle, §8.7). Source attributes:
  `VERSIONED_OBJECT.uid` / `VERSION.uid` / `EHR.ehr_id`.
- **Prefer discipline**: default `return=minimal`; `return=identifier` ⇒
  `{ "uid": … }` body, never 204; `Preference-Applied` MAY be echoed (assert
  only when the schedule says so).
- **`Location`** appears on 201 only; its use on GET/DELETE responses is
  deprecated — bindings assert absence where the spec deprecates.
- **`Content-Type`** MUST be present on every non-204 response and equal the
  negotiated type.
- **Commit metadata**: servers MUST accept `openehr-version` +
  `openehr-audit-details` on change-controlled commits;
  `AUDIT_DETAILS.time_committed` is always server-set (client value ignored)
  — a testable assertion.
- **Negotiation negatives**: unfulfillable `Accept` ⇒ 406; unsupported
  `Content-Type` ⇒ 415 — including the deprecated/legacy simplified media
  types, which are asserted-to-reject (§8.7).
- **error_loose** body selector: see AMB-1 (§8.5) — assert at most that a
  `message` string is present, and only under `Prefer: return=representation`.

### 8.5 The outcome-kind taxonomy and the ambiguity register

**Outcome kinds** (`vocab/outcomes.yaml`, closed enum, extensible only by
schedule release):

| Kind | Class | Meaning (schedule language) |
|---|---|---|
| `created` | success | New resource exists ("positive response associated to the successful creation") |
| `ok` | success | Read/query succeeded with content |
| `ok_empty` | success | Fulfilled with no content (e.g. composition logically deleted at requested time) |
| `updated` | success | New version of existing resource created |
| `deleted` | success | Logical delete performed (a new version, `lifecycle_state = openehr::523\|deleted\|`) |
| `stored` | success | Definition stored (stored query PUT — wire 200, not 201) |
| `already_exists` | error | Duplicate identity ("an EHR with the provided ehr_id … should be unique"; duplicate template_id) |
| `not_found` | error | Target does not exist ("EHR with <ehr_id> does not exist") |
| `version_not_found` | error | preceding_version_uid does not exist |
| `precondition_failed` | error | Version precondition evaluated false (stale preceding_version_uid) |
| `precondition_missing` | error | Required version precondition absent |
| `validation_failed` | error | Semantically invalid content ("information about the errors in the provided COMPOSITION") |
| `template_not_found` | error | Referenced OPT not on server ("information about the non-existent OPT") |
| `template_mismatch` | error | Content commits against a different template_id than the versioned object |
| `missing_template_id` | error | Simplified-format commit without template identification |
| `already_deleted` | error | Delete of an already-deleted version |
| `conflict` | error | Other uniqueness/state conflict |
| `not_acceptable` | error | No representation satisfies `Accept` |
| `unsupported_media` | error | Payload media type unsupported |
| `invalid_query` | error | Malformed/unprocessable AQL |
| `timeout` | error | Server aborted at max execution time |

Cases speak ONLY these kinds. Bindings map each kind to wire per operation
(the same kind may map to different codes on different operations — e.g.
`validation_failed` is 422 on composition ops but 400 on EHR create, per the
OAS). A kind a binding cannot map is a CI error.

**The ambiguity register** (`registers/ambiguities.yaml`) — every entry is a
real, verified divergence or silence, with the normative handling a runner
must apply. Seeded from this extraction:

| Id | Ambiguity (source) | Normative handling |
|---|---|---|
| AMB-1 | **Error body shape diverges inside ITS-REST 1.1.0**: prose says `{message, code, errors[DV_CODED_TEXT]}` under `Prefer: return=representation` (`Requests_and_responses.md` §Error handling); the OAS `Error.yaml` says `{message, validationErrors[string]}` and is wired only into 400. Most 4xx bodies are undefined. | `error_loose`: assert only `message` present (when Prefer representation); never assert either full shape. SEC decision item: pick one shape in ITS-REST 1.2.0. |
| AMB-2 | **EHR create enumerates no 422** — EHR_STATUS validation failure has no assigned code (`ehr_create.yaml` responses = 201/400/409). | Bind `validation_failed` → 400 on EHR create; flag for upstream clarification. |
| AMB-3 | **SM does not say where `preceding_version_uid` lives for update** (`master07` preamble, verbatim spec-ambiguity note). | Case speaks `preceding_version_uid` abstractly; ITS-REST binding realizes it as `If-Match`. SPECPR candidate. |
| AMB-4 | **ADL 1.4 templates have no formal versioning** — duplicate `template_id` handling is implementation-defined: conflict vs version-parameter (master04 NOTE). | Two cases exist (`…-valid_opt_twice_conflict` / `…_no_conflict`); ICS declares which behaviour the product implements; exactly one MUST pass. |
| AMB-5 | **Persistent-COMPOSITION uniqueness per EHR is under SEC debate** (master07 NOTE). | Affected cases carry the flag; verdicts on them are reported but excluded from profile computation until resolved. |
| AMB-6 | **`fetch` default is implementation-defined**; `fetch` cannot combine with AQL `TOP` (`query/Request.md`). | Cases always pass `fetch` explicitly; the TOP+fetch rejection is its own case. |
| AMB-7 | **Additional non-conflicting status codes are permitted** (`Requests_and_responses.md` §HTTP status codes). | Bindings assert the expected code exactly for the expected outcome; they never enumerate-reject other codes for other situations. |

The register is normative: a runner that "resolves" an ambiguity privately is
non-conformant to the schedule.

### 8.6 The assertion vocabulary

Typed assertions usable in `flow[].assert` and `postconditions` (all
evaluated per data-set row):

| Assertion | Fields | Semantics |
|---|---|---|
| `instance_of` | `rm_type`, `format?` | Body parses as the named RM type and validates against the ITS schema for the active format (canonical JSON ⇒ ITS-JSON; XML ⇒ XSD). |
| `field` | `path`, `equals \| exists \| absent \| matches` | RM-path-addressed field check; values may reference `${row.*}`/captures — e.g. `path: ehr_status/is_queryable, equals: ${row.is_queryable}`. |
| `equivalent` | `to: committed \| ${ds:…} \| ${capture}`, `ignoring: server_assigned \| [paths]` | The master07 "content check": retrieved content equals committed content, modulo the declared server-assigned set (`uid`, `system_id`, audit times, …) — the ignore set is normative per operation, not runner-chosen. |
| `version` | `change_type \| lifecycle_state \| count \| uid_pattern` | RM versioning facts: `change_type: MODIFY`, `lifecycle_state: "openehr::523\|deleted\|"`, `count: 2`, `uid_pattern: "<root>::<system>::2"`. |
| `result_set` | `match: ordered \| set \| count \| contains`, `rows`, `columns?` | AQL results. `rows` required by the RESULT_SET schema; `columns`/`meta` optional (assert only when the case says so). Equivalence rules (path forms, RM number typing, NULL cells) are schema-level normative text — the §11.1 SEC prerequisite. |
| `unique` | `over: ${capture}` | Values captured across rows are pairwise distinct (create_ehr-main's ehr_id uniqueness sub-constraint). |
| `message_exemplar` | `text` | Informative only — the schedule's ``"EHR with <ehr_id> does not exist"`` prose; never a pass/fail criterion (AMB-1). |
| `state` | `text`, `verified_by?` | A prose postcondition whose machine verification lives in a linked case (the master06 create→get pattern). CI requires either a `verified_by` resolution or an in-case verification step. |

### 8.7 Format axes — canonical and simplified, first-class

**The media-type matrix** (from `Accept_*`/`ContentType_*` parameter files —
which formats are legal where):

| Endpoint family | canonical-json | canonical-xml | wt-flat | wt-structured | wt (template) |
|---|---|---|---|---|---|
| EHR / EHR_STATUS / DIRECTORY / CONTRIBUTION envelope | ✔ | ✔ | ✘ (415/406) | ✘ (415/406) | ✘ |
| COMPOSITION (create/update/get) | ✔ | ✔ | ✔ | ✔ | ✘ |
| Template get | — | ✔ (OPT XML) | ✘ | ✘ | ✔ `application/openehr.wt+json` |
| Template example | ✔ | ✔ | ✔ | ✔ | ✘ (406) |
| Query | ✔ (RESULT_SET) | — | ✘ | ✘ | ✘ |

Media types (normative): `application/json`, `application/xml`,
`application/openehr.wt.flat+json`, `application/openehr.wt.structured+json`,
`application/openehr.wt+json`. Deprecated (assert-reject):
`…wt.flat.schema+json`, `…wt.structured.schema+json`; legacy (assert-reject):
`application/openehr.nc.flat+json`, `application/openehr.tds2+xml`.

A case declares `formats:` sensitivity; the tech profile (§8.10 statement)
selects which the run exercises; verdicts are per tech profile. The ✘ cells
are themselves conformance cases (the 406/415 negatives).

**The Simplified-Formats chapter blueprint.** The current schedule has NO
simplified-formats chapter — every existing test anywhere is
implementation-original. CNF 2.0 adds one, derived case-by-case from the
STABLE spec (`ITS-REST/docs/simplified_formats/`), in fifteen categories:

1. Round-trip fidelity canonical↔FLAT↔STRUCTURED (commit each form, read all
   three, leaf equality + `_type` on canonical read; FLAT↔STRUCTURED
   value-equality per the master04 conversion algorithms).
2. Node-ID generation (the master04 7-step algorithm: normalisation,
   lowercase, digit-prefix `a`, sibling-uniqueness `_1` — the worked examples
   table becomes a decision table).
3. Level removal (container-attribute elision list; always-collapsed
   ITEM_STRUCTURE/HISTORY; the conditional EVENT collapse both ways).
4. Per-RM-type suffix mapping (the 43 master05 tables — DV_QUANTITY
   `|magnitude`/`|unit` through DV_INTERVAL; each spec-example JSON block is
   a vector).
5. `_`-prefixed RM attributes (`_uid`, `_link:i`, `_feeder_audit`,
   `_normal_range`, `_participation:i`, `_mapping:i`).
6. `|raw` canonical embedding (must carry `_type`; decomposes correctly).
7. `ctx/` semantics (mandatory language/territory; `ctx/time` → `now()`
   default; `ctx/setting` → `openehr::238`; `composer_self` vs
   `composer_name`; participations compact + expanded forms; the master06
   default-mapping table).
8. Instance-index/counter semantics (`:N` zero-based; multi-event,
   multi-observation; STRUCTURED arrays even at 1..1).
9. STRUCTURED style rules (nested objects, `|`-props, `ctx` object,
   empty-object omission).
10. Reject rules (unknown field → `validation_failed`; `|other`+`|code`
    mutually exclusive; `|other` on closed list; missing
    `openehr-template-id`; missing mandatory ctx; datatype/cardinality/
    binding violations).
11. Negotiation strictness (q-values; Content-Type presence/match;
    deprecated + legacy media types → 406/415 both directions).
12. Web-Template retrieval shape (`templateId` + `tree` + node-id rules +
    aqlPath present; the Better-dialect extras are NOT normative).
13. Template example generation (four `Accept_LOCATABLE` forms; `wt+json` on
    the example endpoint → 406).
14. CONTRIBUTION with simplified inner data (canonical envelope, simplified
    `versions[i].data`).
15. Scope negatives (EHR_STATUS/DIRECTORY/demographic have no simplified
    mapping — 406/415).

Simplified Formats is a **SHOULD** in ITS-REST ⇒ the whole chapter sits in
the OPTIONS profile (capability `SimplifiedFormats`) and never gates
CORE/STANDARD.

### 8.8 Data-set governance — the corpus manifest

Every fixture and generated set is a manifest entry:

```yaml
# corpus/MANIFEST.yaml (one entry)
cnf.ehr_status.is_modifiable_missing:
  source: fixtures/ehr/invalid/007_ehr_status_is_modifiable_missing.json
  format: canonical-json
  rm_versions: [">=1.0.2"]
  validity:
    verdict: invalid
    defect: "RM/Schema: is_modifiable is mandatory"
    spec_ref: "RM ehr §EHR_STATUS"
  placeholders: { subject_id: runtime-random }    # the __AUTO-GENERATED__ convention, formalized
  provenance: "openEHR CNF Robot corpus @33251d2a; vendor markers stripped; re-adjudicated 2026-.."
```

Rules (each answering an observed defect in the current corpus):

- **Verdict + defect live in the manifest, never only in a filename.**
- **Adjudication register, not silent edits**: a fixture found wrong gets a
  register entry (defect, citation, disposition: skip-with-citation or
  spec-derived expectation); history is never rewritten.
- **Generated sets are recipes**: content decision-table rows and AQL
  result fixtures are generated from the row values + a context template by
  committed, seeded, deterministic code — the Alkmaar "randomisable data
  sets" answered reproducibly. The recipe is part of the corpus.
- **Per-RM-version variants** are additive overlays (the RM-1.0.x → 1.2.0
  `_type` discriminator injection pattern), declared in the manifest.
- **The decision-table literal grammar is normative** (small PEG published
  with the schemas): ranges `a..b`, lists `[x, y]`, unit-scoped ranges
  `[cm 5.0..10.0, m]`, terminology codes `openehr::122 (length)` /
  `local::at0005`, ordinal tuples `1|[local::at0005]`, quantity literals
  `100 mg`. Violation categories: `rm_schema` (mandatory/typing),
  `rm_invariant(<name>)` (e.g. `limits_consistent`), `iso8601(<rule>)`,
  `constraint(<clause>)` (e.g. `C_DV_QUANTITY.list`), each row may list
  several.

### 8.9 The encoded pilot — official cases, fully encoded

These are the *official* schedule cases (and two new-chapter candidates),
encoded losslessly. They are the proof artifacts Appendix C attaches.

**Pilot 1 — `I_EHR_SERVICE.create_ehr-main`** (master06, verbatim content —
the 16-row matrix, the uniqueness sub-constraint, the cross-case
verification):

```yaml
id: I_EHR_SERVICE.create_ehr-main
kind: functional
component: EHR
sm_operation: I_EHR_SERVICE.create_ehr
test_purpose: >
  Creating an EHR with each valid EHR_STATUS variant succeeds; server
  defaults apply when the status is omitted (is_queryable=true,
  is_modifiable=true, subject=PARTY_SELF).
description: "Create new EHR"
spec_refs:
  - "SM openehr_platform §I_EHR_SERVICE.create_ehr"
  - "CNF platform_test_schedule master06 §create_ehr data sets"
applies: { rm: ">=1.0.2" }
profiles: [CORE]
requires: { server: empty }
parameters:
  iteration: reset_per_row
  matrix:
    columns: [is_queryable, is_modifiable, subject, other_details, ehr_id]
    rows:   # the official 16-row VALID data-set matrix, verbatim
      - [true,  true,  provided, absent,   absent]
      - [true,  false, provided, absent,   absent]
      - [false, true,  provided, absent,   absent]
      - [false, false, provided, absent,   absent]
      - [true,  true,  provided, provided, absent]
      - [true,  false, provided, provided, absent]
      - [false, true,  provided, provided, absent]
      - [false, false, provided, provided, absent]
      - [true,  true,  provided, absent,   provided]
      - [true,  false, provided, absent,   provided]
      - [false, true,  provided, absent,   provided]
      - [false, false, provided, absent,   provided]
      - [true,  true,  provided, provided, provided]
      - [true,  false, provided, provided, provided]
      - [false, true,  provided, provided, provided]
      - [false, false, provided, provided, provided]
flow:
  - step: 1
    call: create_ehr
    with: { ehr_status: "generate(row)", ehr_id: ${row.ehr_id} }
    expect: created
    capture: { ehr_id: created.ehr_id }
postconditions:
  - { assert: unique, over: ${ehr_id} }        # "ehr_id … should be unique for each invocation"
  - { assert: state, text: "EHR exists and is consistent with the data set used",
      verified_by: I_EHR_STATUS.get_ehr_status-get_by_ehr_id }
verified_by: [I_EHR_STATUS.get_ehr_status-get_by_ehr_id]
```

**Pilot 2 — `I_EHR_SERVICE.create_ehr-same_ehr_twice`** (master06 — the
state-carrying failure case; note the two ehr_id sources the schedule
distinguishes: "read from the response" vs "read from the test data sets"):

```yaml
id: I_EHR_SERVICE.create_ehr-same_ehr_twice
kind: functional
component: EHR
sm_operation: I_EHR_SERVICE.create_ehr
test_purpose: "ehr_id values are unique: re-creating an existing EHR is rejected."
description: "Attempt to create same EHR twice"
spec_refs:
  - "SM openehr_platform §I_EHR_SERVICE.create_ehr"
  - "CNF platform_test_schedule master06 §create_ehr-same_ehr_twice"
applies: { rm: ">=1.0.2" }
profiles: [CORE]
requires: { server: empty }
parameters: { iteration: reset_per_row,
              matrix: { columns: [ehr_id], rows: [[absent], [provided]] } }
flow:
  - step: 1
    call: create_ehr
    with: { ehr_id: ${row.ehr_id} }
    expect: created
    capture: { first_ehr_id: created.ehr_id }   # server-assigned OR data-set value — both variants covered
  - step: 2
    call: create_ehr
    with: { ehr_id: ${first_ehr_id} }           # "should be read from the response" / "from the test data sets"
    expect: already_exists
postconditions:
  - { assert: state, text: "Exactly one EHR exists — the one created in step 1" }
```

**Pilot 3 — `I_DEFINITION_ADL14.upload_opt-invalid_opt`** (master04 — the
fixture-set iteration with per-fixture defects; postcondition = unchanged
server):

```yaml
id: I_DEFINITION_ADL14.upload_opt-invalid_opt
kind: functional
component: DEFINITION_ADL14
sm_operation: I_DEFINITION_ADL14.upload_opt
test_purpose: "Invalid OPTs are rejected and leave the server state unchanged."
description: "upload invalid OPTs"
spec_refs:
  - "SM openehr_platform §I_DEFINITION_ADL14.upload_opt"
  - "CNF platform_test_schedule master04 §upload_opt data sets"
applies: { rm: ">=1.0.2" }
profiles: [CORE]              # OPT provisioning is CORE per the Profiles book
requires: { server: empty }
parameters:
  iteration: reset_per_row
  fixture_set:                 # the official invalid-OPT data-set rows, one per defect
    - { data_set: cnf.opt.invalid.empty_file,          expected: validation_failed, defect: "empty file" }
    - { data_set: cnf.opt.invalid.empty_template_id,   expected: validation_failed, defect: "empty template_id" }
    - { data_set: cnf.opt.invalid.removed_mandatory,   expected: validation_failed, defect: "removed mandatory elements" }
    - { data_set: cnf.opt.invalid.multiple_elements,   expected: validation_failed, defect: "multiple elements where upper bound is 1" }
flow:
  - step: 1
    call: upload_opt
    with: { opt: ${ds:row} }
    expect: ${row.expected}
postconditions:
  - { assert: state, text: "No OPTs are loaded on the system",
      verified_by: I_DEFINITION_ADL14.get_opts-empty_server }
ambiguities: [AMB-4]
```

**Pilot 4 — `I_EHR_COMPOSITION.update_composition-event`** (master07 — the
versioning case: prerequisites, capture → preceding_version_uid replay,
RM-level version assertions; the REST binding realizes `preceding_version_uid`
as `If-Match` per AMB-3):

```yaml
id: I_EHR_COMPOSITION.update_composition-event
kind: functional
component: EHR_COMPOSITION
sm_operation: I_EHR_COMPOSITION.update_composition
test_purpose: >
  Updating an existing event COMPOSITION with the correct
  preceding_version_uid creates a second VERSION with change_type MODIFY.
description: "Update an existing event COMPOSITION"
spec_refs:
  - "SM openehr_platform §I_EHR_COMPOSITION.update_composition"
  - "CNF platform_test_schedule master07 §update_composition-event"
  - "RM common §change_control (VERSION.commit_audit.change_type)"
applies: { rm: ">=1.0.2" }
profiles: [CORE]              # ChangeSets/Versioning capabilities
requires:
  server: any
  templates: [cnf.opt.minimal_event]
  ehr: { commits: none }
data_sets: [cnf.composition.minimal_event.v1, cnf.composition.minimal_event.v2]
flow:
  - step: 1
    call: create_composition
    with: { composition: ${ds:cnf.composition.minimal_event.v1} }
    expect: created
    capture: { preceding_version_uid: created.version_uid,
               versioned_object_uid: created.versioned_object_uid }
  - step: 2
    call: update_composition
    with: { composition: ${ds:cnf.composition.minimal_event.v2},
            versioned_object_uid: ${versioned_object_uid},
            preceding_version_uid: ${preceding_version_uid} }   # ITS-REST: If-Match (AMB-3)
    expect: updated
    capture: { v2_uid: updated.version_uid }
    assert:
      - { assert: version, uid_pattern: "${versioned_object_uid}::<system>::2" }
postconditions:
  - { assert: version, count: 2 }
  - { assert: version, of: ${preceding_version_uid}, change_type: CREATE }
  - { assert: version, of: ${v2_uid},                change_type: MODIFY }
  - { assert: equivalent, to: committed, ignoring: server_assigned }   # the master07 "content check"
ambiguities: [AMB-3]
```

(The stale-precondition negative is the official sibling
`update_composition-non_existent`: same shape, step 2 `with:
preceding_version_uid: random`, `expect: version_not_found`; its REST binding
distinguishes the stale-latest case, `expect: precondition_failed` → 412 with
latest ETag.)

**Pilot 5 — `CONT-DV_QUANTITY-validate_property_units_mag`** (master17.3,
the richest official decision table, verbatim rows — structured constraint
literals and multi-category violations):

```yaml
id: CONT-DV_QUANTITY-validate_property_units_mag
kind: content
component: CONTENT
rm_class: DV_QUANTITY
test_purpose: >
  A committed DV_QUANTITY is accepted iff it satisfies the C_DV_QUANTITY
  property + units-list + per-unit magnitude-range constraints.
description: "DV_QUANTITY against C_DV_QUANTITY with property, units and magnitude range"
spec_refs:
  - "CNF platform_test_schedule master17.3 §CONT-DV_QUANTITY-validate_property_units_mag"
  - "AM aom14 §C_DV_QUANTITY"
  - "RM data_types §DV_QUANTITY"
applies: { rm: ">=1.0.2" }
profiles: [CORE]              # ArchetypeValidation
constraint_context:
  template: cnf.tpl.quantity_property_units_mag    # C_DV_QUANTITY: property=openehr::122, list=[cm 5.0..10.0, m]
  path: "/content[...]/value"
decision_table:
  columns: [magnitude, units, expected, violates]
  rows:
    - [null, null, rejected, ["rm_schema: magnitude and units are mandatory"]]
    - [null, "cm", rejected, ["rm_schema: magnitude is mandatory"]]
    - [1.0,  null, rejected, ["rm_schema: units is mandatory"]]
    - [0.0,  "mg", rejected, ["constraint(C_DV_QUANTITY.property): mg is not a length unit"]]
    - [0.0,  "cm", rejected, ["constraint(C_DV_QUANTITY.list): magnitude not in range for unit"]]
    - [0.0,  "km", rejected, ["constraint(C_DV_QUANTITY.list): km is not allowed"]]
    - [1.0,  "cm", rejected, ["constraint(C_DV_QUANTITY.list): magnitude not in range for unit"]]
    - [5.7,  "cm", accepted, []]
    - [10.0, "cm", accepted, []]
```

(Execution semantics: each row generates a composition from the context
template with the row's DV_QUANTITY, commits it via
`I_EHR_COMPOSITION.create_composition`, and expects
`created`/`validation_failed` per the verdict — the generation recipe lives
in the corpus, §8.8.)

**Pilot 6 — `SF-FLAT-commit_roundtrip_ctx_defaults`** (new-chapter candidate,
category 1+7 — every rule cited to the STABLE Simplified Formats spec):

```yaml
id: SF-FLAT-commit_roundtrip_ctx_defaults
kind: functional
component: SIMPLIFIED_FORMATS
sm_operation: I_EHR_COMPOSITION.create_composition
test_purpose: >
  A FLAT composition committed with minimal ctx round-trips to canonical
  JSON and STRUCTURED with equal clinical leaves, and the ctx defaults
  (time→start_time now(), setting→openehr::238) are applied.
description: "FLAT commit, three-format read-back, ctx defaulting"
spec_refs:
  - "ITS-REST simplified_formats master02 §MIME Types"
  - "ITS-REST simplified_formats master04 §Field Identifiers, §Validation"
  - "ITS-REST simplified_formats master06 §ctx defaults"
  - "ITS-REST overview Requests_and_responses §openehr-template-id"
applies: { rm: ">=1.0.2", its_rest: ">=1.1.0" }
profiles: [OPTIONS]           # SimplifiedFormats capability — SHOULD-level, never gates CORE/STANDARD
requires: { server: any, templates: [cnf.opt.vitals], ehr: { commits: none } }
data_sets: [cnf.flat.vitals.minimal_ctx]     # FLAT map: ctx/language, ctx/territory, ctx/composer_name,
                                             # vitals/body_temperature:0/any_event:0/temperature|magnitude, |unit
flow:
  - step: 1
    call: create_composition
    with: { composition: ${ds:cnf.flat.vitals.minimal_ctx}, format: wt-flat }
    expect: created                          # binding adds openehr-template-id (required for simplified commits)
    capture: { version_uid: created.version_uid }
  - step: 2
    call: get_composition
    with: { version_uid: ${version_uid}, format: canonical-json }
    expect: ok
    assert:
      - { assert: instance_of, rm_type: COMPOSITION }
      - { assert: field, path: "context/setting", equals: "openehr::238|other care|" }   # master06 default
      - { assert: field, path: "context/start_time", exists: true }                      # ctx/time → now()
      - { assert: field, path: "content[0]/data/events[0]/data/items[0]/value/magnitude",
          equals: ${ds:cnf.flat.vitals.minimal_ctx#temperature.magnitude} }
  - step: 3
    call: get_composition
    with: { version_uid: ${version_uid}, format: wt-flat }
    expect: ok
    assert:
      - { assert: equivalent, to: committed, ignoring: [ctx-defaults, server_assigned] }
  - step: 4
    call: get_composition
    with: { version_uid: ${version_uid}, format: wt-structured }
    expect: ok
    assert:
      - { assert: equivalent, to: ${step3}, ignoring: [] }   # FLAT↔STRUCTURED value-equality (master04 algorithms)
```

**Pilot 7 — `I_QUERY_SERVICE.execute_adhoc-where_magnitude`** (new-chapter
candidate for the empty master11 — deterministic, RESULT_SET-shape-aware):

```yaml
id: I_QUERY_SERVICE.execute_adhoc-where_magnitude
kind: functional
component: QUERY
sm_operation: I_QUERY_SERVICE.execute_adhoc_query
test_purpose: >
  An ad-hoc AQL query with a WHERE predicate on DV_QUANTITY.magnitude
  returns exactly the matching compositions, as a spec-shaped RESULT_SET.
description: "Ad-hoc AQL, WHERE on magnitude, ordered result"
spec_refs:
  - "QUERY AQL 1.1 §WHERE, §ORDER BY"
  - "ITS-REST query §Response (RESULT_SET: rows required; columns, meta optional)"
applies: { rm: ">=1.0.2", aql: ">=1.1" }
profiles: [STANDARD]          # AqlBasic
requires: { server: any, templates: [cnf.opt.blood_pressure], ehr: { commits: none } }
data_sets: [cnf.set.bp-10]    # generated: 10 BP compositions, magnitudes 100..190 step 10 (recipe in corpus)
flow:
  - step: 1
    call: commit_data_set
    with: { set: ${ds:cnf.set.bp-10} }
    expect: created
  - step: 2
    call: execute_adhoc_query
    with:
      q: >
        SELECT c/uid/value AS uid FROM EHR e CONTAINS COMPOSITION c
        CONTAINS OBSERVATION o [openEHR-EHR-OBSERVATION.blood_pressure.v2]
        WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= $mag
        ORDER BY c/uid/value ASC
      query_parameters: { mag: 140 }
      fetch: 100               # AMB-6: fetch always explicit
    expect: ok
    assert:
      - { assert: result_set, match: ordered,
          rows: { from_data_set: "cnf.set.bp-10#magnitude>=140, sorted by uid" },
          columns: [{ name: uid }] }
```

### 8.10 The ICS (statement), results, and IXIT schemas

Field-level contracts (JSON Schemas published with the schedule):

**`statement.json` — the ICS** (ISO/IEC 17050-1 content, §6.1):

| Field | Semantics |
|---|---|
| `product` ∎ | name, **exact version/build**, vendor, unique product identifier |
| `schedule_release` ∎ | the CNF schedule release the claims are made against |
| `spec_versions` ∎ | declared RM/AQL/ITS-REST/TERM versions (drives `applies` filtering) |
| `claims` ∎ | claimed capabilities per the Profiles matrix + claimed profiles |
| `tech_profiles` ∎ | which format/protocol matrices are claimed (e.g. `[its-rest: [canonical-json, canonical-xml, wt-flat]]`) |
| `options` | declared behaviour for register-listed implementation choices (e.g. AMB-4: conflict vs version-param) |
| `non_functional` | reserved declaration slots (performance/security postures) — never verdict inputs (§6.3) |
| `evidence` ∎ | hash links to the `results.json` files backing the claims |
| `attestation` | rung ≥ 1: signatory name/role/date + the §6.4 responsibility sentence |

Version-binding rule: a statement pins the exact product version; a new
version needs a new statement or a signed "conformance-relevant surface
unchanged" attestation referencing the prior evidence.

**`results.json` — the conformance test report** (9646 PCTR analogue):

| Field | Semantics |
|---|---|
| `sut` ∎ | product identity + deployment description |
| `runner` ∎ | harness name, version, **verification-pack status** (§8.12) |
| `schedule_release` ∎, `tech_profile` ∎ | what was run, under which format matrix |
| `ixit_digest` ∎ | hash of the ixit.json used (reproducibility) |
| `outcomes[]` ∎ | per case × format × row: `passed \| failed \| errored \| skipped \| not-applicable`, with **rows_driven/rows_total**, the failing step + assertion on failure, and a mandatory citation on every N/A, skip, and guard exclusion |
| `ambiguity_dispositions` | which register options the run exercised |

`errored` (transport/SUT fault) is never a conformance finding. Coverage is
computable: cases driven / cases selected by the ICS, per profile.

**`ixit.json`** (9646 IXIT): base URL, auth mode + credentials reference,
admin mount, template-id policy, system-id expectations, per-endpoint
overrides — everything a runner needs to drive a deployed SUT, standardized
so any runner drives any SUT from the same file. (ECC's `SutDescriptor` is
the donated draft.)

### 8.11 ICS-driven selection and verdict computation

Mechanical pipeline, normative:

1. **Static conformance review** of the statement: claim-set legality
   (STANDARD ⇒ all CORE capabilities claimed), spec-version consistency,
   option declarations present for every register entry the claims touch.
2. **Selection**: cases whose `profiles` ∩ claimed capabilities ≠ ∅, filtered
   by `applies` × declared spec versions and by `guards`.
3. **Execution**: per case × tech-profile format × parameter row, with
   `reset_per_row` honoured.
4. **Verdicts**: case passes iff every selected row passes. Capability
   evidence: `Passed` (≥1 case ran, none failed) / `Failed` /
   `NotEvidenced` / `NoCases` (a printed coverage bound). Profile verdicts:
   CORE/STANDARD = all required capabilities `Passed`; OPTIONS = any.
   AMB-5-flagged cases report but do not gate.
5. Everything above is a pure function of (statement, results, catalogue) —
   a reference implementation ships with the schemas; any two conformant
   implementations MUST compute identical verdicts.

### 8.12 Runner verification — the two-part pack

A runner claims schedule compliance through:

1. **Verdict conformance** — replay a fixed transcript (canned
   request/response corpus with adjudicated expected verdicts, including
   deliberate fail/N-A/skip/guard outcomes and AMB-1 error-body variants) and
   reproduce the verdicts + emit schema-valid `results.json`. A fixture
   server suffices.
2. **Live-SUT conformance** — drive ≥ 2 independent live SUTs (different
   vendors) from their `ixit.json` and produce results consistent with those
   SUTs' published baselines. Two SUTs, not one recording, so no single
   implementation's wire quirks become the de-facto reference. Transcript
   expectations are adjudicated against spec text via the register, never
   against whichever SUT emitted them.

### 8.13 CI on specifications-CNF — the machine gates

Every PR: schema validation of all seven artifact families; id uniqueness +
no-reuse; `sm_operation` resolution against the SM; `spec_refs` link check;
binding completeness (every outcome kind a case uses is mapped by every
declared ITS binding of its operation; every capture a case uses has a wire
source); `verified_by` resolution; corpus-manifest integrity (every
referenced key exists; every fixture has a verdict + provenance); ambiguity
links resolve; decision-table literals parse against the published grammar;
prose regeneration succeeds. This is the mechanism that lets the repo accept
community PRs without a bottleneck maintainer — and it is ECC's
coverage-guard discipline (`tools/conformance/tests/coverage.rs`),
generalized.

## 9. Certification governance — the ladder as an ISO/IEC 17067 scheme

**Scheme owner: openEHR International** (the CIC that operationally runs the
specification program — the body the Conformance Guide already names as the
Platform Specifier). The scheme is 17067 **Type 1a** at the self-declaration
rungs (a specific product version is type-tested and declared); a future
accredited rung is framed toward **Type 6** (ongoing process assurance across
releases) only if surveillance is funded. Rungs are labelled by ISO/IEC 17000
attestation level so no rung can masquerade as a higher one:

| Rung | Name | ISO frame | Mechanism | Who grants |
|---|---|---|---|---|
| 0 | **Published statement** | First-party attestation, registered | Vendor publishes `statement.json` + `results.json`. **Listing preconditions**: the results come from a runner that has passed the §8.12 verification pack, and the statement passes static conformance review. Registry rows display runner identity + verification status and are visually labelled **self-published**. | Nobody — registration only |
| 1 | **Self-certified** | First-party attestation with signed SDoC (ISO/IEC 17050-1/-2) | Rung 0 + a signed legal attestation of result accuracy by an authorized officer (+ modest fee funding the program). The §6.4 responsibility sentence appears on the certificate. | openEHR International (administrative + static review only) |
| 2 | **Community-verified** | Second-party attestation | Results reproduced at a supervised conformance-thon (EHRCON slot) or by a named community witness re-running the suite from the vendor's `ixit.json` against a vendor-provided deployment. Witness identity on the registry row. | Event organizers / named witnesses |
| 3 | **Certified** | Third-party attestation → certification | An **ISO/IEC 17025**-accredited lab runs the suite; an **ISO/IEC 17065**-accredited certification body reviews and certifies, with surveillance obligations. Both roles **delegated to independent accredited bodies** (the IHE/ONC model) — openEHR International remains scheme owner only, because a spec author certifying its own ecosystem fails 17065 impartiality. **This rung is not offered until surveillance is funded**; advertising it earlier would be dishonest. | Accredited certification bodies |

Cross-cutting rules:

- **Validity & supersession**: a statement/certificate names the CNF schedule
  release + spec versions + tech profile + exact product version. It never
  expires by clock alone; it is **superseded** when a newer schedule release
  changes the cases it rests on or when the product version moves without a
  new statement/attestation (§8.10), and the registry shows currency —
  answering Alkmaar's expiry question without inventing a revocation
  bureaucracy.
- **Disputes**: when a procurer or competitor contests a published result, the
  named path is a rung-2 witnessed re-run (same schedule release, same
  `ixit.json` shape); the registry records the dispute and its outcome. Below
  rung 3, legal veracity remains a commercial-contract matter between vendor
  and procurer (ISO/IEC 17050 framing, §6.4) — stated plainly, as the 2021
  roadmap did.
- **Badges** derive from registry state (rung + profile + schedule release +
  tech profile), machine-served by the registry, never self-hosted claims.
  The badge/trademark is owned by openEHR International with published
  grant/withdraw criteria tied to registry state and an appeals path (§13).
- **Access**: schedule, schemas, corpus, and runners are public and free
  (Inferno/OpenID lesson: adoption dies behind paywalls). Rungs 1–3 may carry
  fees; the 2021 members-only idea applies to *services* (attestation
  processing, events, assessor program), never to the artifacts.

## 10. The procurement pack — usable within 12 months

The deliverable a tendering authority can use the moment rung 0 exists:

- **A normative RFP requirement template** (new short section of the Guide,
  answering the framework's "RFI/RFP guides: future" TODO):

  > *The offered product must hold a published openEHR Conformance Statement
  > (CNF schedule release ≥ R, profile ≥ STANDARD, technology profile
  > including canonical JSON) at registry rung ≥ 1, for the product version
  > offered. The awarding authority reserves the right to require a witnessed
  > re-run (rung 2) of the published results prior to acceptance.*

  Tender authors fill four parameters (release, profile, tech profile, rung).
  This replaces the Catalonia-style behavioural-SLA workaround with a
  referenceable requirement.
- **Comparability**: the registry renders statements side-by-side per profile
  and tech profile (mechanically comparable because the statements are
  computable — the 2021 "vendor-neutral comparison site" idea, scoped to
  what's honest).
- **The dispute path** (§9) gives an authority a defined action when two green
  statements conflict with lived experience — the answer v1 lacked.
- **Version discipline** (§8.10) tells the authority exactly what a listing
  covers when the vendor ships the next release.

## 11. Gap-fill roadmap (content plan for the schedule itself)

Ordered by procurement value; each item is a bounded, assignable chapter task
once §8.1 makes cases enumerable files:

1. **Querying / AQL (master11 + master05)** — the flagship gap. **Prerequisite
   design decision for SEC, resolved before cases ship**: the result-set
   equivalence rules (the `match:` vocabulary — ordered/set/count/contains —
   plus canonical path forms, RM number typing, NULL semantics), normative at
   schema level. Seed material: this repo's 25 QRY + 8 SQR + 4 AQT case
   designs (each carrying AQL 1.1 citations) and EHRbase's AQL conformance
   corpus ([ehrbase/conformance-testing-documentation](https://github.com/ehrbase/conformance-testing-documentation),
   SELECT/WHERE/ORDER BY/LIMIT/FROM/parameter suites).
2. **Content chapters refresh** — raise the RM floor statement (1.0.2 → an
   applicability ladder), fill 17.5 or formally adjudicate it out, fix the
   master14 numbering gap and the master13 duplicate heading.
3. **Demographic (master10)** — schedule cases exist in no form today; ECC's
   31 DEM cases + the ITS-REST Demographic API (DEVELOPMENT lifecycle) are the
   seed; profile placement stays OPTIONS.
4. **Admin (master12) + Messaging (master13)** — decide what is
   *wire-testable* (platform API) vs inherently off-wire (dump/load,
   archives); off-wire capabilities move to statement-declared, not
   schedule-tested — the honest boundary.
5. **N/A re-adjudication of donated material (hard gate)** — every donated
   case whose evidence or N/A justification points at ehrbase-rs internal
   tests is re-adjudicated to spec-text-only evidence **before** entering the
   normative catalogue. No exceptions; this is a scoped workstream, not an
   assumption.
6. **Security & privacy conformance points** — currently only Signing +
   Anonymous EHRs in the Profiles book while the Certificate book advertises
   BASIC-SEC/BASIC-PRIV with no defining cases. Minimum viable set:
   authenticated-access enforcement, audit-event emission on writes
   (IHE ATNA-shaped), signing. Explicitly scoped small; not a security
   evaluation scheme.
7. **ADL2 cases (master04)** — OPTIONS-profile depth for the `am24`
   generation.
8. **The openEHR→EEHRxF seam (EHDS alignment, later)** — cases verifying that
   priority-category content in a conformant CDR renders faithfully to the
   EEHRxF FHIR models, once the March 2027 implementing acts fix them. Flag:
   this extends conformance scope beyond the platform API; it needs its own
   profile family and SEC decision.

## 12. What ehrbase-rs contributes — and what must stay community-owned

Offered (donated under the spec repo's licence, with an explicit statement
that no ehrbase-rs copyright or patent claim is retained in normative
artifacts):

- **Methodology + schemas as the working draft**: the catalogue entry model,
  the statement/results/ixit schema shapes, the computed-verdict rules
  implementing the Profiles book, the coverage-guard CI design, the
  adjudication-register and fairness-register patterns, the edition ladder.
  All running code today (`tools/conformance/`), not paper.
- **394 active case designs** with spec citations as raw material for the stub
  chapters — QRY/SQR/AQT for master11/05, DEM for master10, VAL's 119 content
  cases cross-checked against master15–17 — **subject to the §11.5
  re-adjudication gate**, plus the honest off-wire treatment for
  Admin/Messaging.
- **Engineering effort**: drafting the JSON Schemas, the specifications-CNF
  CI, the schedule-to-prose renderer (a costed deliverable, §8.1), and the
  AQL chapter — as PRs under SEC review.
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
  evidence only (hence §11.5).
- **The schema is co-authored or it is nothing**: ≥2 competing vendors are
  invited as schema co-authors *before* the format decision is ratified, so
  the data model is not shaped by one implementation's convenience. If the
  community prefers rescuing the Robot suite as the reference runner, we
  support that; our value is the machine-readable spine, not the runner.

## 13. Governance & resourcing — the section that answers the post-mortem

The 2021–22 effort had board sponsorship and still stalled; this section is
the difference between "nice idea, same risk" and "resourced program".

- **Ownership**: the normative repo (specifications-CNF), the registry, the
  "openEHR Conformant" wordmark/badge, and the scheme rules are owned by
  **openEHR International** (the CIC already operating the specification
  program). No vendor owns any normative artifact.
- **The CNF maintainer group**: chartered under the SEC; 5–7 seats with **no
  single-vendor majority**; schema and scheme-rule changes by **recorded
  vote** (simple majority, SEC escalation path), never by PR-volume or any
  party's unilateral veto — including ours and including openEHR
  International's own staff. Charter published in the repo before the first
  normative merge.
- **Change control**: RFC process for schema/scheme changes; schedule releases
  cut like spec releases (versioned, changelogged); CI (§8.8) makes
  community PRs safe to accept, which is what actually de-bottlenecks a
  volunteer group.
- **IP**: donated cases/schemas/corpus items enter under the spec repo's
  licence with contributor licence hygiene (no retained vendor copyright in
  normative artifacts, no patent encumbrance).
- **Funding**: a recurring program line (registry hosting, CI, maintainer
  coordination, event slots) funded from openEHR International's program
  budget + rung-1 attestation fees — explicitly *not* from one vendor's
  project budget, because §4.3.2 is how that ends. Gap-fill chapters can be
  vendor-sponsored (bounded, reviewable tasks), but the *program* must not be.
- **Commitments in hand**: ehrbase-rs commits the pilot engineering (§12).
  The Discourse ask (Appendix A) explicitly requests matching co-commitments —
  a second vendor's engineering time and 2–3 maintainer volunteers — before
  the SEC agenda item, so the SEC decides on a resourced plan, not a hope.
- **Impartiality by structure**: openEHR International is scheme owner and
  registrar only. It never tests, never certifies (rung 3 is delegated to
  accredited bodies; rung 1 is administrative). A spec author grading its own
  ecosystem is the 17065 impartiality failure the IHE/ONC split exists to
  avoid.

## 14. Engagement plan

1. **Discourse first** (Conformance category) — the strategy condensed to a
   discussion post (Appendix A), tagging the 2021–22 participants; goal:
   temperature check + the §13 co-commitments, 2–3 weeks.
2. **Jira + repo** — comment on SPECCNF-1/6 linking the thread (Appendix B);
   new specifications-CNF issue proposing the machine-readable schedule format
   with the three example cases (Appendix C).
3. **SEC agenda item** — deliverables: adopt-the-format decision, the
   maintainer-group charter, and blessing the AQL chapter as the pilot.
4. **Pilot PR series** (after SEC nod): JSON Schemas + CI, master06 (the
   fleshed exemplar) converted to catalogue form with semantically-equivalent
   regenerated prose (human-reviewed diff), then master11/AQL as the first
   *new* content — equivalence rules first. The fully sequenced series with
   acceptance gates is §16.1; the in-repo production track is §16.2.
5. **Registry MVP**: a static page on openehr.org rendering submitted
   statements with attestation-level labels — rung 0 exists the moment two
   products publish (ehrbase-rs volunteers; upstream EHRbase is already
   assessed by ECC and is the natural second).
6. **EHRCON26 (Amsterdam, Sep 2026)**: a conformance slot alongside the
   existing EHDS track and the "Conformance Testing openEHR with FHIR
   TestScript" session — the natural venue to socialize the proposal and
   recruit maintainers; a conformance-thon proposal for the following cycle
   once ≥2 runners + ≥2 SUTs exist.
7. **EHDS liaison**: track the Art 36/15 implementing acts and the Xt-EHR
   D8.2 scheme; position the openEHR registry/statement artifacts as
   *complementary evidence* alongside the EHDS DoC (§6.5); revisit the
   §11.8 EEHRxF-seam profile when the acts land (2027).

Success measures: SEC adopts the machine-readable schedule + charter; ≥2
independent runners pass the §8.12 verification pack; the AQL chapter released
with normative equivalence rules; ≥3 products on the public registry; CNF
Release 1.0.0 finally cut — before the March 2027 EHDS implementing acts.

## 15. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Perceived vendor capture ("ehrbase-rs wants its framework blessed") | Lead with the community's own 2021–22 design (§1 credits it explicitly); donate under spec-repo licence with no retained IP; charter with voted decisions and no single-vendor majority (§13); ≥2 competing vendors co-author the schema before ratification; our numbering stays out; the registry and trademark belong to openEHR International. |
| Repeat of the single-owner stall | §13: recurring program funding (not project money), charter + CI so contributions merge without a bottleneck person, gap-fill as bounded sponsorable tasks, and co-commitments collected *before* the SEC decision. |
| SEC bandwidth / spec-process latency | Rung 0 (registry) and the schedule format need no new normative prose to start delivering value; pilot PRs convert existing content before adding new; the EHDS clock (§6.5) is the forcing function for prioritization. |
| Format bikeshed (YAML vs JSON vs tables) | The normative artifact is the JSON Schema; bring it + three worked examples + the renderer to the first discussion; decide on evidence. |
| Robot-suite loyalists read this as suite replacement | It isn't: the catalogue makes the Robot suite *one compliant runner*; rescuing PR #5 is inside the proposal; the verification pack gives it a first-class path. |
| CaboLabs framework overlap | Invite the 2017 reviewer as co-author from the Discourse post onward; the 2017 review and 2022 framework are cited as requirements sources; the statement schema is that idea made computable with ISO/IEC 17050 shape. |
| Registry misread as endorsement / gamed by permissive runners | §9 rung-0 gates: runner-verification precondition, runner identity + status on every row, attestation-level labels, dispute path; the §6.4 responsibility sentence on every rung-0/1 artifact. |
| Overclaiming EHDS relevance | §6.5 states plainly that the EEHRxF/conformity route is FHIR/IHE-based and openEHR is the persistence layer behind it; the EEHRxF seam is a later, SEC-gated profile family (§11.8). |
| Prose-generation cost underestimated | "Byte-comparable" dropped; semantic equivalence with human-reviewed diff; renderer scoped as a gated deliverable (§16.1 U2). |

## 16. Production implementation plan

Two tracks, both production-grade from day one — no throwaway prototype. The
in-repo track does not wait for upstream adoption: ECC implements the §8
artifact set as its own production format immediately, which is
simultaneously the proof the upstream proposal ships with.

### 16.1 Upstream: the specifications-CNF PR series

Sequenced, each PR independently reviewable and CI-green, each with an
acceptance gate:

| PR | Content | Acceptance gate |
|---|---|---|
| U1 | The seven artifact schemas (§8.2) + the outcome vocabulary + the ambiguity register (seeded with AMB-1…7) + the §8.13 CI workflow | Schemas validate the §8.9 pilot files; CI runs on the repo |
| U2 | master06 (EHR) converted: all 21 cases as case cores + the its-rest bindings for the EHR operations + corpus manifest over the existing EHR fixtures | Generated prose semantically equivalent to the current chapter (human-reviewed diff); zero information loss against the AsciiDoc tables |
| U3 | master07/08/09 (COMPOSITION/CONTRIBUTION/DIRECTORY) conversion + bindings | Same gate; the versioning cases (§8.9 pilot 4 shape) round-trip |
| U4 | Content chapters (master15–17) conversion — decision tables as data + the literal grammar + generation recipes | Every existing table row preserved verbatim; grammar parses 100% of existing literals |
| U5 | **master11/AQL — the first new chapter**: result-set equivalence rules (normative schema text) + ~30 cases seeded from ECC QRY/SQR/AQT + the EHRbase AQL corpus | SEC sign-off on the equivalence rules FIRST; every case spec-cited to AQL 1.1 |
| U6 | **Simplified-Formats chapter** (new): the §8.7 fifteen categories, ~60 cases driven from the master04/05/06 spec-example blocks | Every case cites its simplified_formats section; OPTIONS-profile placement |
| U7 | statement/results/ixit schemas + verdict rules + the reference verdict implementation + the runner verification pack (transcripts + adjudications) | Two independent runners (ECC + the rescued Robot suite or another vendor's) compute identical verdicts on the pack |
| U8 | The registry (production, on openehr.org): statement rendering, attestation-level labels, badges, dispute log | First two products listed (ehrbase-rs + upstream EHRbase baselines) |

Demographic (master10) and Admin/Messaging (master12/13) follow as U9+ per
the §11 roadmap once the pattern is proven on U2–U6.

### 16.2 This codebase: ECC becomes the first production implementation

ECC adopts the §8 artifact set as its own storage format — not a shadow
export. Tracked as dedicated issues (opened when this design is
owner-approved), sequenced:

| WS | Workstream | Content | Done-gate |
|---|---|---|---|
| W1 | **Artifact schemas in Rust** | `tools/conformance`: typed model + validator for case cores, bindings, outcome vocab, corpus manifest, ambiguity register; JSON-Schema emission so the same schemas ship upstream in U1. The §8.13 checks become `cargo nextest` guards alongside the existing coverage guard. | Validator rejects every seeded-defect artifact fixture; schemas byte-identical to the U1 set |
| W2 | **Catalogue conversion** | The 394 ECC cases re-expressed as §8.3 case cores + §8.4 operation bindings. Where an official schedule case exists, the CNF id becomes primary (ECC numbers retire to trace metadata — inverting today's `ScheduleTrace`); ECC-original cases keep an `ecc-` namespace pending upstream adoption. `inventory/ecc-catalog.tsv` becomes a generated view. | Zero-drift: the converted catalogue reproduces the current 402-execution baseline exactly (384 passed · 18 N/A) |
| W3 | **Data-driven executor** | The engine executes functional case cores directly from the artifact files (flow interpreter: requires-setup, parameter iteration with reset_per_row, captures, outcome mapping via bindings, typed assertions). Hand-written Rust remains only for generation recipes and genuinely non-mechanizable glue — each such exception is registered. Content decision tables execute from the data (they already nearly do). | ≥90% of cases run through the interpreter; every exception listed in the report; ECC baseline unchanged |
| W4 | **Statement / results / ixit emission** | `results.json` migrates to the §8.10 schema (per-row outcomes, ambiguity dispositions, runner verification status); `statement.json` (ICS) + `ixit.json` (formalizing `SutDescriptor`) emitted per SUT; the Certificate/Statement/Comparison artifacts render from them; verdict computation moves to the shared pure function. | All `docs/conformance/**` artifacts regenerate from the new schemas; the honesty blocks survive; badges derive from the new results |
| W5 | **Simplified-formats deepening** | The §8.7 blueprint's gap categories 2–9 (node-id algorithm, level removal, the 43 suffix tables, `_`-attributes, `\|raw`, full ctx vocabulary, counters, STRUCTURED style) + deepened 1/10 — ~40 new SF cases, all spec-example-driven, all OPTIONS-profile. | Every master04/05/06 spec-example JSON block exercised; ECC baseline ratchets upward only |
| W6 | **Runner verification pack** | Author the U7 transcripts + adjudications; ECC self-verifies against them in CI; publish the pack so the Robot suite (and any vendor runner) can prove itself. | ECC passes both pack parts; the pack rejects a deliberately-broken runner build |

Sequencing: W1 → W2 → {W3, W4} → {W5, W6}. Standing gates apply throughout:
`cargo clippy --workspace --all-targets --all-features`, full nextest, the
ECC zero-drift rule (the baseline only ratchets upward), and the
changelog/docs-website rules for any user-visible surface.

What this buys strategically: when U1 reaches the SEC, the schemas arrive
with a production runner already storing, validating, executing, and
reporting through them against two real CDRs — the difference between
proposing a format and demonstrating one.

---

## Appendix A — Discourse post draft (Conformance category)

> **Title: Reviving CNF: a resourced proposal — machine-readable conformance schedule, ISO-grounded certification ladder, and the EHDS clock**
>
> The CNF component defined the right things in 2021–22 — the Schedule /
> Profile / Statement / Certificate vocabulary, SM-anchored test cases, tech
> profiles, CORE/STANDARD/OPTIONS, and a certification maturity ladder — and
> then stalled: the last schedule amendment is 0.8.6 (March 2022), Release
> 1.0.0 was never cut, the assessment chapter is still `TBD`, the Querying
> chapter has zero test cases, and the only executable suite is
> EHRbase-specific and currently doesn't run (specifications-CNF PR #5 has
> been open since 2023). The stall wasn't the design — it was the operating
> model: spare-time ownership, project-tied funding, one harness.
>
> Meanwhile the outside world moved: the EHDS regulation (in force March
> 2025) is making self-assessed, CE-marked, automatically-tested conformity
> the norm for EHR systems in Europe, with implementing acts due March 2027 —
> and openEHR is not in that frame. And procurement keeps naming openEHR with
> nothing verifiable to require (Catalonia's CDR tender had to use latency
> SLAs as a proxy for conformance).
>
> We (ehrbase-rs) have been running a CNF-shaped conformance instrument in
> production against two CDRs — profile verdicts computed from the Profiles
> book, both canonical formats, every case citing the spec — and we'd like to
> bring the useful parts upstream rather than let another parallel framework
> grow. **We are not proposing our tool as the standard.** We are proposing,
> for discussion:
>
> 1. **Govern and resource CNF so it cannot stall again**: a CNF maintainer
>    group chartered under openEHR International (voted decisions, no
>    single-vendor majority), the repo/registry/badge owned by openEHR
>    International, recurring program funding rather than project money — and
>    we're asking here, before any SEC agenda item, for matching
>    co-commitments: a second vendor's engineering time and 2–3 maintainer
>    volunteers.
> 2. **Make the Test Schedule machine-readable and normative** — one versioned
>    catalogue (protocol-neutral case cores + per-ITS binding overlays, spec
>    citations, data sets, profiles, spec-version applicability) from which
>    the spec's prose pages are generated and against which *any* runner —
>    the Robot suite, ours, Spock, Postman — can prove itself. In ISO terms:
>    the schedule is the Abstract Test Suite (ISO/IEC 9646), completing the
>    global-ID direction the 2022 work already set. This turns the stub
>    chapters (AQL first!) into an enumerable backlog anyone can PR against.
> 3. **Define the bottom rungs of the 2021 certification ladder with
>    international vocabulary**: a computable Conformance Statement schema
>    (an ICS with ISO/IEC 17050-1 content — the thing SPECCNF-1 asked for in
>    2017), a public registry with attestation-level labels and anti-gaming
>    rules, then OpenID-style attested self-certification. The whole thing
>    framed as an ISO/IEC 17067 scheme owned by openEHR International;
>    accredited third-party certification (17025 lab + 17065 certifier, the
>    IHE/ONC split) stays the end goal — offered only when surveillance can
>    be funded, and never operated by openEHR itself.
>
> We're offering: the JSON Schemas and CI as pilot PRs, conversion of an
> existing fleshed chapter (master06) as proof the format loses nothing
> (semantic-equivalence, human-reviewed), a drafted AQL chapter seeded from
> our 30+ AQL case designs and EHRbase's AQL corpus — with the result-set
> equivalence rules resolved first — 394 cited case designs as raw material
> (re-adjudicated to spec-text-only evidence before anything enters the
> catalogue), and our runner as one of the ≥2 independent implementations the
> scheme requires.
>
> Full strategy document with the evidence base (chapter-by-chapter state,
> the 2017 SPECCNF-1 review point-by-point, ISO/CASCO mapping, EHDS analysis,
> prior art from DICOM / OpenID / Inferno / IHE): [link].
> @pablo @thomas.beale @birger.haarbrandt @sebastian.iancu — you built the
> 2021–22 foundation; does this direction match where you wanted it to go,
> and what would you change before this goes to a SEC agenda?

## Appendix B — SPECCNF-1 / SPECCNF-6 Jira comment draft

> We've written up a concrete, resourced proposal to finish what this ticket
> started, including answers to the questions in [the 2017 review comment]:
> a normative template + JSON schema for Conformance Statements (your "first
> step before any testing" — shaped as an ISO/IEC 9646 ICS with ISO/IEC
> 17050-1 supplier's-declaration content), explicit certificate governance
> (who creates / grants / verifies, as an ISO/IEC 17067 scheme owned by
> openEHR International with attestation-level rungs), platform-scope
> discipline via ISO/IEC 25010 functional suitability, and no manual testing.
> Discussion thread with the full document: [Discourse link]. Happy to bring
> it to a SEC call if there's interest.

## Appendix C — specifications-CNF GitHub issue draft

> **Title: Proposal: machine-readable Platform Conformance Test Schedule (single normative source, generated prose, runner-independent)**
>
> Today the schedule exists as AsciiDoc prose, 2017 pseudo-code under
> `scripts/`, and the Robot suite under `tests/` — three representations that
> drifted apart, none machine-checkable, and PR #5 (making the tests runnable)
> has been open since 2023. Proposal: adopt a catalogue format — one data file
> per test case holding the protocol-neutral core (global ID, SM operation,
> test purpose, spec refs, pre/postconditions, logical outcomes, data-set
> keys, profile membership, spec-version applicability) plus separate per-ITS
> binding overlays mapping logical outcomes to wire specifics — generate the
> spec pages from it (semantic equivalence, human-reviewed), validate it in CI
> (schema, ID uniqueness/no-reuse, spec-ref resolution, binding completeness),
> and treat every runner — including the Robot suite — as a downstream
> implementation verified against a shared two-part verification pack
> (verdict conformance on a fixed transcript + live-SUT conformance against
> ≥2 independent SUTs). In ISO/IEC 9646 terms: the catalogue is the Abstract
> Test Suite; runners are Executable Test Suites; the Conformance Statement is
> the ICS that selects applicable cases. Attached: the full artifact-set design (case-core field contract,
> per-SM-operation bindings with the real ITS-REST status/header mappings, the
> outcome-kind taxonomy, an ambiguity register seeded with seven verified spec
> silences, a typed assertion vocabulary, corpus-manifest governance) and
> seven fully-encoded pilot cases — five of them existing official schedule
> cases encoded losslessly (create_ehr-main with its 16-row matrix,
> create_ehr-same_ehr_twice, upload_opt-invalid_opt,
> update_composition-event, CONT-DV_QUANTITY-validate_property_units_mag)
> plus an AQL case for the empty master11 and a Simplified-Formats case for
> the missing chapter. We volunteer the JSON Schemas, the CI
> workflow, the master06 conversion, and a drafted master11/AQL chapter as the
> pilot PR series. Discussion: [Discourse link].

## Appendix D — source register

**openEHR:**
- Vendored CNF snapshot: `docs/specs/openehr/CNF/` @ `33251d2a`
  (`PROVENANCE.md`); key files cited inline above.
- Published component: <https://specifications.openehr.org/releases/CNF/development>.
- Repo: <https://github.com/openEHR/specifications-CNF> — master last content
  2022; development = Antora migration (May 2026); PR #5 open since
  2023-06-11; issues #1/#2 from 2017.
- Jira: [SPECCNF-1](https://openehr.atlassian.net/browse/SPECCNF-1) (+ the
  [review, comment 22500](https://openehr.atlassian.net/browse/SPECCNF-1?focusedCommentId=22500)),
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
- Governance: <https://openehr.org/governance/> (openEHR Foundation + openEHR
  International CIC); HL7–openEHR joint statements (Amsterdam Jun 2025;
  [Dublin "Converge & Collaborate" May 2026](https://discourse.openehr.org/t/converge-collaborate-2026-joint-statement-from-hl7-international-and-openehr-international-press-release/16843));
  [EHRCON26 programme](https://openehr.org/ehrcon26/programme/).
- openEHR's own [ISO 18308 Conformance Statement](https://specifications.openehr.org/releases/1.0.2/requirements/iso18308_conformance.pdf).

**ISO / conformity assessment:**
- ISO/IEC 17000:2020 (vocabulary; attestation levels) —
  <https://www.iso.org/obp/ui/#iso:std:iso-iec:17000:ed-1:en>; CASCO overview
  <https://casco.iso.org/attestations-of-conformity.html>.
- ISO/IEC 17025:2017 (testing labs) — <https://www.iso.org/standard/66912.html>.
- ISO/IEC 17065:2012 (certification bodies) —
  <https://www.iso.org/obp/ui/#iso:std:iso-iec:17065:ed-1:v1:en>.
- ISO/IEC 17067:2013 (scheme types) — <https://www.iso.org/standard/55087.html>.
- ISO/IEC 17050-1:2004 / -2:2004 (supplier's declaration of conformity) —
  <https://www.iso.org/standard/29373.html>,
  <https://www.iso.org/standard/35516.html>.
- ISO/IEC 9646 (conformance testing methodology; PICS/ICS, IXIT, ATS/ETS,
  verdicts) — <https://www.iso.org/standard/17473.html> (part 1),
  overview <https://homes.cs.aau.dk/~kgl/TOV03/iso9646.pdf>.
- ISO/IEC 25010:2023 (quality model; functional suitability) —
  <https://www.iso.org/obp/ui/#iso:std:iso-iec:25010:ed-1:v1:en>;
  ISO/IEC 25051:2014 — <https://www.iso.org/standard/61579.html>;
  ISO/IEC/IEEE 29119-3:2021 — <https://www.iso.org/standard/79429.html>.
- ISO 18308:2011 — <https://www.iso.org/standard/52823.html>.

**Regulatory / programs:**
- Regulation (EU) 2025/327 (EHDS) — OJ text:
  <https://eur-lex.europa.eu/eli/reg/2025/327/oj/eng> (Arts 14–15, 25, 30,
  36–41, 49, 105; Annexes II–IV).
- Xt-EHR joint action — <https://www.xt-ehr.eu/> ;
  D8.2 EHR Conformity Assessment Scheme (May 2026)
  <https://www.xt-ehr.eu/wp-content/uploads/2026/05/Xt-EHR-D8.2.pdf> ;
  EEHRxF FHIR models <https://www.xt-ehr.eu/fhir/models/index.html>.
- ONC/ASTP Health IT Certification Program — 45 CFR Part 170 Subpart E
  <https://www.ecfr.gov/current/title-45/subtitle-A/subchapter-D/part-170/subpart-E>;
  program structure <https://www.healthit.gov/faq/a1-how-onc-health-it-certification-program-structured>;
  Inferno <https://inferno.healthit.gov/> +
  <https://inferno-framework.github.io/docs/>.
- IHE testing programs — <https://www.ihe.net/testing/>; IHE International
  Conformity Assessment Scheme Part 1 (ISO/IEC 17025 + 17067 basis)
  <https://www.ihe.net/wp-content/uploads/2018/08/IHE_International_Conformity_Assessment_Scheme_Part_1_Rev1-0_2014-06-25.pdf>.
- OpenID certification — <https://openid.net/certification/>.
- DICOM (PS3.2 conformance) — <https://www.dicomstandard.org/current>.

**Procurement evidence:**
- Catalonia CDR award —
  <https://discourse.openehr.org/t/region-of-catalonia-award-of-the-tender-for-the-service-of-cdr-platform/3910>.
- Karolinska/Stockholm framework —
  <https://discourse.openehr.org/t/karolinska-stockholm-procurement-of-digital-health-platform-cdr-tools-services-consultants/4457>.
- Malta NEHR — <https://www.openehr.org/news_events/industry_news/272>.
- Wales DHCW National Data Resource —
  <https://dhcw.nhs.wales/our-programmes/national-data-resource1/>.
- openEHR procurement index —
  <https://openehr.atlassian.net/wiki/spaces/resources/pages/416514052/>.

**Ecosystem:**
- [ehrbase/conformance-testing-documentation](https://github.com/ehrbase/conformance-testing-documentation)
  (AQL suites + fixtures, last push 2025-01-30);
  [CaboLabs openEHR Conformance Framework](https://www.cabolabs.com/blog/article/openehr_conformance_framework-61ef4f513f7c5.html).
- Our instrument: `tools/conformance/` (ECC), latest committed baseline
  `docs/conformance/ehrbase-rs/CONFORMANCE_REPORT.md` (402 case×format
  executions · 384 passed · 0 failed · 18 N/A; CORE PASS / STANDARD PASS /
  OPTIONS OBTAINED; 394 active catalogue cases).
