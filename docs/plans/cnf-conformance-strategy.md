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
  can be tested" (his model: DICOM PACS conformance statements). → §8.3: the
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
- Don't hard-code REST as the only access method conceptually. → §8.1's
  case-core/binding split makes that structural.
- Archetype-validation conformance points need precise definitions. → §11
  roadmap item 2 + the content decision-table schema (§8.1, example 3).

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
   separate per-ITS binding artifacts (§8.1). New protocols add binding files,
   never new suites.
3. **Harness independence by construction.** Catalogue + data sets + schemas +
   verdict rules are the contract; every runner is a downstream implementation
   verified against a shared reference pack (§8.7). No harness is normative.
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

## 8. The proposal — CNF 2.0 architecture

### 8.1 The machine-readable Conformance Schedule (the ATS)

One catalogue, versioned in specifications-CNF. **Each test case is a
protocol-neutral core file; wire specifics live in per-ITS binding overlay
files** — the Guide's framework elements (3) abstract test case and (4)
technology binding, kept separate on disk exactly as the "square" separates
them conceptually. (YAML shown for readability; the format decision belongs to
SEC — the normative artifact is the published JSON Schema, and a JSON encoding
is equivalent.)

**Example 1 — functional case core (protocol-neutral):**

```yaml
# schedule/platform/ehr/I_EHR_SERVICE.create_ehr-no_status.yaml
id: I_EHR_SERVICE.create_ehr-no_status     # global CNF id — 2022 scheme retained
kind: functional
component: EHR
sm_operation: I_EHR_SERVICE.create_ehr      # the SM anchor
test_purpose: >                              # ISO/IEC 9646 "test purpose"
  Creating an EHR without a supplied EHR_STATUS yields a new EHR whose
  platform-created default status is queryable and modifiable.
spec_refs:
  - "SM openehr_platform §I_EHR_SERVICE.create_ehr"
  - "RM ehr §EHR creation semantics"
applies: { rm: ">=1.0.2" }                   # spec-version applicability
profiles: [CORE]
preconditions: []
flow:
  - call: create_ehr                         # abstract SM call, no wire detail
    outcome: created                         # logical outcome, mapped by bindings
postconditions:
  - "EHR exists and is retrievable by the returned ehr_id"
  - "EHR_STATUS.is_queryable = true; EHR_STATUS.is_modifiable = true"
data_sets: []
```

**Its REST binding overlay (separate file; other ITSs add siblings):**

```yaml
# bindings/its-rest/I_EHR_SERVICE.create_ehr-no_status.yaml
case: I_EHR_SERVICE.create_ehr-no_status
its: its-rest
applies: { its_rest: ">=1.0.0" }
formats: [canonical-json, canonical-xml]
flow:
  - call: create_ehr
    request: { method: POST, path: "/ehr" }
    outcomes:
      created: { status: 201, headers: [ETag, Location] }
```

**Example 2 — an AQL case (the empty master11), deterministic, with an
explicit result-match vocabulary:**

```yaml
# schedule/platform/querying/I_QUERY_SERVICE.execute_adhoc-where_magnitude.yaml
id: I_QUERY_SERVICE.execute_adhoc-where_magnitude
kind: functional
component: QUERY
sm_operation: I_QUERY_SERVICE.execute_adhoc_query
test_purpose: >
  An ad-hoc AQL query with a WHERE predicate on DV_QUANTITY.magnitude returns
  exactly the compositions whose stored magnitude satisfies the predicate.
spec_refs:
  - "QUERY AQL 1.1 §WHERE"
  - "QUERY AQL 1.1 §ORDER BY"
  - "RM data_types §DV_QUANTITY.magnitude"
applies: { rm: ">=1.0.2", aql: ">=1.1" }
profiles: [STANDARD]                          # AqlBasic per the Profiles book
preconditions:
  - "data_set cnf.vitals.bp-10 committed to a fresh EHR"
flow:
  - call: execute_adhoc_query
    input:
      q: >
        SELECT c/uid/value AS uid FROM EHR e CONTAINS COMPOSITION c
        CONTAINS OBSERVATION o [openEHR-EHR-OBSERVATION.blood_pressure.v2]
        WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude >= $mag
        ORDER BY c/uid/value ASC
      query_parameters: { mag: 140 }
    outcome: ok
result_expectation:
  match: ordered            # vocabulary: ordered | set | count | contains
  rows: { from_data_set: "cnf.vitals.bp-10#magnitude>=140, sorted by uid" }
data_sets: [cnf.vitals.bp-10]
```

Cases without `ORDER BY` use `match: set`; aggregate cases use `match: count`.
The equivalence rules (canonical path forms, RM number typing, NULL handling,
row identity) are defined **once, normatively, at schema level** — not per
case — and are called out as the prerequisite SEC design decision in §11.1,
because they *are* the AQL conformance question.

**Example 3 — a content decision-table case (the master15–17 shape), showing
that content cases keep their nature under a sibling schema:**

```yaml
# schedule/content/data_types/CONT-DV_ORDINAL-validate_constraint.yaml
id: CONT-DV_ORDINAL-validate_constraint
kind: content
rm_class: DV_ORDINAL
test_purpose: >
  A committed DV_ORDINAL is accepted iff its value/symbol pair matches one of
  the tuples constrained by the template.
spec_refs:
  - "RM data_types §DV_ORDINAL"
  - "AM aom §C_DV_ORDINAL constraint semantics"
applies: { rm: ">=1.0.2" }
profiles: [CORE]                              # ArchetypeValidation capability
constraint_context:
  template: cnf.tpl.ordinal_constraints       # governed-corpus key
  path: "/content[...]/value"
decision_table:                               # the master17.3 table, as data
  columns: [value, symbol_code, expected, constraints_violated]
  rows:
    - [1, "at0005", accepted, []]
    - [2, "at0006", accepted, []]
    - [3, "at0007", rejected, ["tuple not in constraint"]]
    - [1, "at0006", rejected, ["value/symbol mismatch"]]
data_sets: [cnf.tpl.ordinal_constraints]
```

What the machine-readable ATS fixes: the three-representations drift (prose,
pseudo-code, Robot) collapses into one source; stub chapters become an
*enumerable, assignable* backlog (a missing case is a missing file); coverage
of any harness is computable (`cases implemented / cases in schedule`, per
profile); and the published spec pages regenerate from the catalogue so the
document can never disagree with the tests again.

**Prose generation, scoped honestly**: the rendered chapters will be
*semantically equivalent* to today's hand-authored pages, verified by a
one-time human-reviewed diff — not byte-identical, because the fleshed
chapters carry hand-tuned narrative that a renderer should not be forced to
reproduce. The renderer is a real, costed deliverable of the pilot (§14), not
a freebie.

### 8.2 The derivation square, machine-checked

The Guide's specs → SM call → binding → runnable test chain
(`guide/master04-framework.adoc` §From Specifications to Runnable Tests)
becomes CI on specifications-CNF: every case core must carry non-empty
`spec_refs`, a resolvable `sm_operation` (checked against the SM component
list), and — for content cases — a decision table; every binding overlay must
reference an existing case and map every logical outcome; IDs are unique and
never reused (retired cases keep their ID with `status: retired`). Schema
validation + link checking run on every PR. This repo's ECC runs exactly this
guard today (`tools/conformance/tests/coverage.rs`) and it is the single most
effective discipline in the framework.

### 8.3 The computable Conformance Statement (ICS) + results schema

Three small JSON Schemas, published as normative parts of CNF 2.0:

- **`statement.json`** — the **ICS**: which components/capabilities/profiles
  the product claims, at which spec versions, under which tech profiles.
  Content requirements follow **ISO/IEC 17050-1** (unique product **and
  version/build** identification, the complete requirements list with selected
  options, methods used, results evaluation, responsible signatory, date) —
  the artifact SPECCNF-1 asked for in 2017, DICOM-statement-shaped,
  machine-comparable. **Product-version binding rule**: a statement pins the
  exact product version; a new product version requires either a new statement
  or an explicit signed "conformance-relevant surface unchanged" attestation
  referencing the prior results — so procurers always know what a green row
  actually covers.
- **`results.json`** — the conformance test report (9646's PCTR analogue):
  SUT identity, harness identity + its runner-verification status (§8.7),
  schedule release, tech profile, per-case verdicts
  (`passed | failed | errored | skipped | not-applicable`) with a mandatory
  citation on every N/A and skip. Errored (transport/SUT fault) is never a
  conformance finding. Statements hash-link the results files they rest on;
  supporting-evidence retention follows ISO/IEC 17050-2.
- **`ixit.json`** — the deployment parameters a runner needs against a live
  SUT (base URL, auth mode, admin mount, template-id policy…), standardized so
  any runner can drive any SUT from the same file. (ECC's `SutDescriptor` is
  the donated draft.)

**ICS-driven selection** (9646's core mechanism): the statement's claimed
capabilities select which schedule cases apply; a **static conformance
review** — itself mechanical — checks the claim set is legal (e.g. STANDARD
claimed ⇒ every CORE capability claimed). Profile verdicts (CORE/STANDARD
pass = *all* required capabilities evidenced and passing; OPTIONS = any) are a
pure function of `statement.json` + `results.json` + the catalogue — the
Profiles book's rule set, executable.

### 8.4 Profiles and system classes

Keep the Profiles book's CORE / STANDARD / OPTIONS matrix as the **Platform
(CDR) profile family** — it is coherent and implemented in practice. Add, as
later work, profile families per system class (the 2021 insight that a
universal CORE is wrong): platform client, tool, demographic service,
terminology service. Procurers compose tender profiles from the matrix, which
is what the Profiles overview already intends (§10 gives them the reference
template).

### 8.5 Technology profiles and the spec-version ladder

- **Tech profile** = the serialization/protocol matrix a run exercised
  (canonical JSON, canonical XML; REST binding; later others). A statement
  says which tech profiles were run; CORE/STANDARD verdicts are per tech
  profile, ending the 2021 "all tests in all formats vs one format" stalemate
  by reporting both honestly instead of choosing.
- **Spec-version applicability** on every case and binding (`applies:` ranges)
  lets one schedule serve SUTs pinned to different release lines — a statement
  names the schedule release + the SUT's declared spec versions. ECC's
  "edition ladder" (assertion cores split from edition-specific wire forms,
  satisfied rung recorded as a finding) is the donated working model.

### 8.6 Data-set governance

- A **governed corpus** with manifest-keyed fixtures: every data set has an
  ID, provenance, spec citations, and validity adjudication (valid /
  deliberately invalid with the violated constraint named).
- **Generated data sets** for combinatorial areas (content decision tables,
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

### 8.7 Harness independence, concretely

- CNF 2.0 normative artifacts: catalogue + binding overlays + JSON Schemas
  (case, binding, statement, results, ixit) + corpus + verdict rules. **No
  harness is normative.**
- A runner claims compliance through a **two-part verification pack**,
  because the two failure modes are different:
  1. **Verdict conformance** — replay a *fixed transcript* (a canned
     request/response corpus with adjudicated expected verdicts, including
     deliberate fail/N-A/skip outcomes) and reproduce the verdicts + emit
     valid `results.json`. A fixture server is sufficient here.
  2. **Live-SUT conformance** — drive at least **two independent live SUTs**
     (different vendors) from an `ixit.json` and produce results consistent
     with those SUTs' published baselines. Two SUTs, not one recording,
     because a single reference recording would silently re-privilege one
     implementation's wire quirks — the bias this whole proposal exists to
     remove. Transcript expectations are adjudicated **against spec text**,
     never against whichever SUT emitted them, via the adjudication register.
- Expected day-one runners: the de-EHRbase-ified Robot suite (rescuing
  PR #5's intent), ECC (Rust, this repo), and whatever vendors already run
  privately — which is the point: vendors keep their tooling and gain
  comparability.

### 8.8 CI on specifications-CNF

Schema validation, ID uniqueness/no-reuse, spec-ref link checks, binding
completeness, corpus manifest integrity, prose regeneration — so the repo can
accept community PRs safely, which is the mechanism that lets gap-fill scale
beyond one maintainer.

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
| 0 | **Published statement** | First-party attestation, registered | Vendor publishes `statement.json` + `results.json`. **Listing preconditions**: the results come from a runner that has passed the §8.7 verification pack, and the statement passes static conformance review. Registry rows display runner identity + verification status and are visually labelled **self-published**. | Nobody — registration only |
| 1 | **Self-certified** | First-party attestation with signed SDoC (ISO/IEC 17050-1/-2) | Rung 0 + a signed legal attestation of result accuracy by an authorized officer (+ modest fee funding the program). The §6.4 responsibility sentence appears on the certificate. | openEHR International (administrative + static review only) |
| 2 | **Community-verified** | Second-party attestation | Results reproduced at a supervised conformance-thon (EHRCON slot) or by a named community witness re-running the suite from the vendor's `ixit.json` against a vendor-provided deployment. Witness identity on the registry row. | Event organizers / named witnesses |
| 3 | **Certified** | Third-party attestation → certification | An **ISO/IEC 17025**-accredited lab runs the suite; an **ISO/IEC 17065**-accredited certification body reviews and certifies, with surveillance obligations. Both roles **delegated to independent accredited bodies** (the IHE/ONC model) — openEHR International remains scheme owner only, because a spec author certifying its own ecosystem fails 17065 impartiality. **This rung is not offered until surveillance is funded**; advertising it earlier would be dishonest. | Accredited certification bodies |

Cross-cutting rules:

- **Validity & supersession**: a statement/certificate names the CNF schedule
  release + spec versions + tech profile + exact product version. It never
  expires by clock alone; it is **superseded** when a newer schedule release
  changes the cases it rests on or when the product version moves without a
  new statement/attestation (§8.3), and the registry shows currency —
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
- **Version discipline** (§8.3) tells the authority exactly what a listing
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
   *new* content — equivalence rules first.
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
independent runners pass the §8.7 verification pack; the AQL chapter released
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
| Prose-generation cost underestimated | "Byte-comparable" dropped; semantic equivalence with human-reviewed diff; renderer costed as a pilot deliverable (§8.1, §14.4). |

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
> the ICS that selects applicable cases. Three worked examples attached
> (`I_EHR_SERVICE.create_ehr-no_status` from the fleshed master06 with its
> REST binding overlay, an AQL WHERE case with explicit result-match
> vocabulary for the empty master11, and `CONT-DV_ORDINAL-validate_constraint`
> as a content decision-table case). We volunteer the JSON Schemas, the CI
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
