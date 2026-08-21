<!-- PERMANENT (owner rulings 2026-07-22 + 2026-07-23): this document is
the CNF 2.0 design record — the normative internal reference the runner
implements, in particular the §8.14 performance machinery (the
population-anchored volumetric class model: POC 2 · S 15 · L 150 ·
R 1,500 peak arrivals/s floors, the p99 <= 1 s SLO, the workload
derivation from OECD/Eurostat/NHS statistics, and the hospital-simulation
journey decomposition). It lived as docs/plans/cnf-conformance-strategy.md
under the delete-on-implementation lifecycle, was deleted at the
2026-07-22 cutover, restored the same day by owner instruction, and moved
here 2026-07-23 as a living reference document. Never delete it and never
move it back under docs/plans/. Spec-facing citations in CODE still point
at docs/specs/openehr/ per the citation rules — this file is the design
record, not a code-citable oracle. -->

# openEHR conformance & certification — the CNF 2.0 design

*The design record of the CNF 2.0 framework this repository implements:
the machine-readable conformance schedule, the pure verdict pipeline, the
profiles and certificate, and the measured performance-class model. Every
claim was verified 2026-07-21 against the sources in the Appendix (source
register), through repeated independent audit rounds (openEHR spec
conformance, ISO, legal/regulatory, internal consistency,
implementability). Built by the CNF 2.0 reference runner
(`tools/cnf-runner`); the measured artifacts live beside this document
under `docs/conformance/`.*

---

## 1. Summary

The official CNF component defines the right concepts — Conformance Guide,
Platform Conformance Test Schedule, Profiles, Certificate — and is frozen:
last content amendment March 2022, Release 1.0.0 (planned December 2018)
never cut, the entire assessment layer `TBD`, zero AQL test cases, and one
vendor-specific Robot suite that no longer runs as the only executable
artifact (§3). Procurement names openEHR with nothing verifiable to require —
Catalonia's ~€8.5M CDR tender had to use latency SLAs as the conformance
proxy — and in Europe the EHDS regulation is making self-assessed, CE-marked,
automatically-tested conformity the norm for EHR systems (implementing acts
due March 2027; EHR-system obligations staged 2029/2031 by data category), a
frame openEHR is currently not in (§6.5).

## 2. The three pillars

CNF 2.0 keeps the 2021–2022 community design (§4) and fixes the operating
model that killed it:

1. **Govern and resource it so it cannot stall again** (§12): a chartered
   maintainer group under openEHR International (voted decisions, no
   single-vendor majority), openEHR International owning repo/registry/
   trademark, recurring program funding, ≥2 competing vendors co-authoring
   the schema before ratification.
2. **A machine-readable Test Schedule as the single normative source**
   (§8): one versioned catalogue — in ISO terms the Abstract Test Suite —
   from which the spec prose is generated and against which any harness
   (Robot, Rust, Spock, Postman) proves itself; CI replaces the bottleneck
   maintainer. The same machine-readable philosophy openEHR already applies
   via BMM and OpenAPI.
3. **Certification defined with international vocabulary** (§6, §9), with a
   **multi-dimensional certificate**: functional profiles plus measured
   performance-class ratings (§8.14) and the Security & Privacy rating
   (SEC-BASIC, §8.15), Enterprise following (§11.11): a
   conformity-assessment scheme per ISO/IEC 17000 — ISO/IEC 17050
   supplier's declarations first, witnessed peer verification next,
   delegated ISO/IEC 17025-lab + 17065-certifier assessment (the only rung
   ISO/IEC 17067 governs) last — the architecture IHE and ONC already run
   and the shape EHDS Art 40 mandates.

Nothing conceptual is claimed as new: the four-artifact vocabulary, the
SM-anchors/ITS-executes split, tech profiles, and the global ID scheme are
the 2021–2022 community's work. The deltas are five: one-file-per-case data
with generated prose; CI enforcement of the derivation chain; computable
Statement/results schemas with mechanically computed verdicts; the
governance/resourcing charter; and the ISO/EHDS grounding. FerroEHR's ECC
(394 cases, both wire formats, machine-computed verdicts on the CNF profiles
model) is the working draft and one reference implementation — explicitly
not "the standard": the standard is community-owned, vendor-neutral, and
multi-harness by construction.

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
- `scripts/openehr_platform/*.txt` in the upstream CNF repository — 34 abstract
  pseudo-code scripts (© 2017), a third representation not wired to anything.
  Deliberately NOT vendored: nothing here reads them, and each one names
  CC-BY-SA 3.0 while linking the NoDerivatives licence, a contradiction
  reported upstream rather than carried.
- Robot coverage is asymmetric to the schedule: robots exist for stub chapters
  (Query, Admin) and are missing for others (Demographic, Messaging).

### 3.4 Vital signs

Jira SPECCNF: two visible issues (SPECCNF-1 open since 2017; SPECCNF-6 "in
progress" since October 2021, zero comments); Release-1.0.0 dated 2018-12-28,
never released. Repo: last content work 2022; 2024 = link fixes; May 2026 =
Antora toolchain migration only; issues #1/#2 date from 2017.

## 4. History distilled — what carries forward, and why it stalled

The 2021–2022 community design era (Discourse threads 1616/1851/2239/2285/
2358/2373, board-funded 2021 — Appendix) settled the foundations this
framework keeps **wholesale**:

- the four-artifact vocabulary: Conformance **Schedule / Profile /
  Statement / Certificate**;
- **SM names the capabilities, an ITS executes the tests**;
- **technology profiles** parameterizing serialization/protocol;
- the **global test-case ID scheme** spanning API + content tests;
- the four-stage **certification maturity ladder**;
- profiles **CORE / STANDARD / OPTIONS** with the capability matrix.

It then stalled, for four operating-model causes this framework must answer
(§12 answers 1–3; §8's machine-readable schedule + CI answers 1 and 4):

1. single-person, spare-time ownership;
2. funding tied to one project, not the program;
3. a two-track scope split (narrow official CNF vs one company's broader
   framework) with no owner for the union;
4. single-harness lock-in — the abstract-spec/any-technology model was
   chosen but never realized; the only implementation stayed
   EHRbase-specific and its generalization (specifications-CNF PR #5,
   open since 2023) had no owner.

The 2017 conformance wiki page (T. Beale — Appendix) contributed five ideas
the 2021–22 era never carried forward, recovered into this design: the
maximal-coverage end-to-end template test and scenario/lifecycle suites
(§11.2–3), the Enterprise dimension — data portability, EHR
merge/split/move, cross-enterprise sync (§11.11) — the performance/
volumetric classes made testable (§8.14), and the Security & Privacy BASIC
rung (§8.15). Its functional levels 1/2/3+O were superseded by the Profiles
book's CORE/STANDARD/OPTIONS.

The 2017 spec review ([SPECCNF-1 comment 22500](https://openehr.atlassian.net/browse/SPECCNF-1?focusedCommentId=22500))
remains the oldest open requirements list; its asks are answered in the
design: computable Conformance Statements as the first artifact (§8.10),
certificate governance — who creates/grants/verifies (§9), scope discipline
via ISO/IEC 25010 with no manual testing (§6.3, §7),
no conceptual REST hard-coding (the §8.3/§8.4 case-core/binding split), and
precise archetype-validation conformance points (§8.9 pilot 5, §11).

## 5. Prior art — how other standards run conformance

| Program | Model | What to copy |
|---|---|---|
| **DICOM conformance statements** ([DICOM PS3.2](https://www.dicomstandard.org/current)) | Every product publishes a standardized conformance statement; procurement compares statements; no central certification. | The **statement as the legally load-bearing artifact**, with a normative template. CNF 2.0 upgrade: make it computable. |
| **OpenID Foundation certification** ([openid.net/certification](https://openid.net/certification/)) | **Self-certification**: vendor runs the official open-source suite, submits results + a signed legal attestation, pays a small fee, gets listed on the public certified page. Runs at scale since 2015. | The **cheapest credible rung**: official suite + published results + attestation + public registry. |
| **HL7 FHIR / ONC Inferno** ([inferno.healthit.gov](https://inferno.healthit.gov/), [framework docs](https://inferno-framework.github.io/docs/)) | Open-source test kits per implementation guide; the (g)(10) kit is an approved test method inside a regulatory certification program. Structure: policy (ASTP/ONC, 45 CFR 170) → open-source test method (Inferno) → **ISO/IEC 17025** labs (ONC-ATLs, NVLAP-accredited) → **ISO/IEC 17065** certifiers (ONC-ACBs) → accreditor (ANSI/ANAB), plus surveillance + the public CHPL product list. | **Test kits as maintained open-source products**; machine-readable expectations; and the five-layer separation: the standards body never tests or certifies its own conformity — it owns criteria and approves test methods. |
| **IHE Connectathons + Conformity Assessment Scheme** ([ihe.net/testing](https://www.ihe.net/testing/)) | Annual supervised peer-testing events (results published) plus a formal scheme **explicitly built on ISO/IEC 17025 + 17067**, with certification bodies under ISO/IEC 17065 evaluating accredited-lab results. | The **community verification event** rung (a conformance-thon at EHRCON fits openEHR's culture) and the canonical lab/certifier split for the eventual top rung. |
| **EHDS Article 40** ([Regulation (EU) 2025/327](https://eur-lex.europa.eu/eli/reg/2025/327/oj/eng)) | The Commission develops **open-source digital testing software**, operated as EU and national testing environments, for the harmonised EHR components; manufacturers must use these environments pre-market and file the results; positive results = presumption of conformity. Conformity is **manufacturer self-assessment** + EU declaration + CE marking + public registration — no notified bodies. | Regulatory confirmation of the whole shape: automated open-source suite + self-assessment + declaration + public registry is now *the law's own architecture* for EHR conformity in Europe. |
| **openEHR's own ISO 18308 Conformance Statement** ([PDF](https://specifications.openehr.org/releases/1.0.2/requirements/iso18308_conformance.pdf)) | A requirement-by-requirement statement of openEHR's conformance to ISO 18308, exceptions indexed. | **In-family precedent**: openEHR has already authored a requirement-indexed conformance statement; the computable Statement is its machine-readable evolution. |

Composite lesson: nobody starts with third-party certification — every
working program starts with an official runnable suite + a public registry,
then adds attestation, events, accreditation. The 2021 ladder was right; the
bottom rung was never built.

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
| Community verification rung | **Witnessed peer verification** — ISO defines no "second-party attestation"; genuinely second-party only when the witness is a purchaser/user | ISO/IEC 17000 (party definitions) |
| Accredited assessment rung | **Third-party attestation → certification** by an **ISO/IEC 17065** body using an **ISO/IEC 17025** lab | ISO/IEC 17065; 17025 |
| The program itself | A **conformity-assessment scheme** (ISO/IEC 17000 §3), openEHR International as **scheme owner**; only the third-party rung is an ISO/IEC 17067 product-certification scheme (Type 1a initially; **Type 5** — type testing + process assessment + surveillance — if ongoing certification ships) | ISO/IEC 17000; ISO/IEC 17067 (third-party rung only) |
| "Conformance" scope | **Functional suitability** (completeness + correctness, incl. the Security & Privacy behaviours §8.15) **plus performance efficiency** (§8.14) — nothing else | ISO/IEC 25010 (+25023 measures); software-product evaluation per ISO/IEC 25051 |

### 6.2 ISO/IEC 9646 — the 35-year-old blueprint for exactly this design

ISO/IEC 9646 standardized, in 1991, exactly this architecture: a supplier
fills in a published **ICS proforma**; the ICS **selects** which cases from
the **Abstract Test Suite** apply; the supplier provides the **IXIT**
(instance parameters to run the tests); runners realize the ATS as Executable
Test Suites; outcomes are pass/fail/inconclusive verdicts in a standardized
report. ETSI, Bluetooth SIG, and USB-IF still run on this vocabulary — the
machine-readable schedule is settled practice to adopt, not an invention to
evaluate.

### 6.3 Scope discipline via ISO/IEC 25010 (answering the 2017 review)

Conformance under CNF 2.0 attests exactly two ISO/IEC 25010 characteristics
— the two verdict machineries of §8:

- **Functional suitability** (completeness + correctness against the openEHR
  specifications) — the functional + content schedules (§8).
- **Performance efficiency** — the performance & volumetrics schedule
  (§8.14): measured pass/fail class ratings (POC/S/L/R) under normative
  workloads on declared environments. NOTE: the current Conformance Guide
  excludes performance ("Non-functional conformance (performance, etc) is
  not addressed by this guide" — `guide/master03-overview.adoc`); CNF 2.0
  deliberately extends the scope
  here, siding with the 2017 schedule's multi-dimensional certificate — an
  explicit SEC decision item. Measures follow ISO/IEC 25023.

The Security & Privacy family (§8.15) sits **inside** functional
suitability: SEC-BASIC attests the functional *correctness of security
behaviours* (access rejected, audit written, demographic content separated)
through the assertion machinery — and so does **signature verification**:
where the Signing capability is claimed, the presence, verifiability, and
chain integrity of version signatures are wire-testable behaviours (§8.15).
What stays out of scope is ISO/IEC 25010's **security quality characteristic
itself** — attack resistance, penetration strength, cryptographic *strength*
(algorithm security, key management assurance) — which belongs to security
evaluation schemes, not conformance testing; likewise reliability and
maintainability,
referenced by their ISO names rather than redefined (the 2017 review's
point). ISO/IEC 25051 (conformity evaluation of ready-to-use software
products) and ISO/IEC/IEEE 29119-3 (test documentation shapes) are the
supporting citations for the evaluation procedure and report formats.

### 6.4 Legal weight of self-declaration (the phrasing to adopt)

Under ISO/IEC 17050-1 the supplier's declaration is made on the supplier's
sole responsibility; the standard states, verbatim: *"References to
assessments by first, second or third parties are not to be interpreted as
reducing the responsibility of the supplier in any way."* CNF 2.0's lower
rungs should carry exactly this framing in the Guide:

> *A published Conformance Statement is a first-party attestation
> (ISO/IEC 17000) in the form of a supplier's declaration of conformity
> (ISO/IEC 17050-1). openEHR International registers and publishes it but does
> not verify or endorse it; responsibility for its accuracy rests entirely
> with the declaring supplier.*

That sentence gives self-declaration recognized legal weight without implying
openEHR liability or endorsement — and it is the same legal shape as the EHDS
EU declaration of conformity.

### 6.5 The EHDS clock — honest positioning

Facts, from the verbatim OJ text of
[Regulation (EU) 2025/327](https://eur-lex.europa.eu/eli/reg/2025/327/oj/eng)
(retrieved via the Publications Office machine channel,
`publications.europa.eu/resource/celex/32025R0327` — Art 105 quoted):

- **Entry into force 25 March 2025** — Art 105: *"This Regulation shall
  enter into force on the twentieth day following that of its publication
  in the Official Journal"* (published 5 March 2025).
- **General application** — Art 105: *"This Regulation shall apply from
  26 March 2027."*
- **Staged EHR-system waves** — Art 105 third paragraph: Articles 3–15,
  23(2)–(6), 25–27 and 47–49 apply *"from 26 March 2029 to priority
  categories … points (a), (b) and (c) [patient summaries, ePrescriptions,
  eDispensations], **and to EHR systems intended by the manufacturer to
  process such categories of data**"*, and from 26 March 2031 for points
  (d)–(f) (imaging, lab results, discharge reports) — i.e. the
  harmonised-component obligations (Arts 25–27) reach category-(a)–(c) EHR
  systems already in **2029**.
- **Chapter III as a whole** — Art 105: *"Chapter III shall apply to EHR
  systems put into service in the Union referred to in Article 26(2) from
  26 March 2031."* Chapter IV (secondary use) applies from 26 March 2029.

Every in-scope EHR system must
embed two **harmonised software components** (European interoperability
component; European logging component; Art 25, Annex II), pass an
**open-source digital testing environment** (Art 40 — Commission-developed
open-source software, operated as EU and national environments), and ship
with a manufacturer **self-assessed EU declaration of conformity** —
Art 39(1) verbatim: *"The EU declaration of conformity … shall state that
the manufacturer of an EHR system has demonstrated that the essential
requirements laid down in Annex II have been fulfilled"* — **CE marking**
(Art 41) and public registration (Art 49). The testing obligation is
Art 40 ("European digital testing environment") verbatim: *"The Commission
shall develop a European digital testing environment … The Commission shall
make the software supporting the European digital testing environment
available as open-source"* (40(1)); *"Before placing EHR systems on the
market, manufacturers **shall use** the digital testing environments … The
elements in relation to which the results of the assessment are positive
shall be presumed to be in conformity with this Regulation"* (40(3)). Common
specifications + the EEHRxF exchange format arrive as implementing acts
adopted by **26 March 2027** (Arts 36, 15), applying on the
priority-category clock, pre-drafted by the Xt-EHR joint action —
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
  running suite — before the March 2027 implementing acts and the 2029/2031
  application waves define "conformity" habits without openEHR in the room.

## 7. Design principles for CNF 2.0

1. **Machine-readable normative source, generated prose.** Like BMM → RM and
   OpenAPI → REST, the Schedule's normative form is data; the published spec
   pages are rendered from it. A test case that isn't in the catalogue doesn't
   exist; a catalogue entry without spec citations doesn't build — and the
   citation must be to a **RELEASED** spec component (RM / BASE / AM / QUERY /
   TERM / ITS-XML / SM / ITS-REST docs text). The frozen CNF schedule, the OAS, and
   the Robot suites are STALLED structural guides (which behaviours to cover),
   never the correctness oracle — this framework earns its authority from the
   released components, which is precisely why it is the first *enforceable* CNF.
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
5. **Honesty is structural.** Coverage is a mandate, not a pass rate — every
   wire behaviour the spec defines is its own case, and the bounds + honest
   boundaries are printed, never silent (§8.5, §11). N/A adjudications are
   cited; spec silences/defects are adjudicated in the ambiguity register
   rather than silently edited, and **every carried divergence is reported back
   upstream** (`upstream_issue` → §13), never absorbed. Both directions are
   published when comparing products; registry rows are labelled by attestation
   level so self-declaration is never mistaken for certification.
6. **Vendor neutrality is testable.** No vendor image names, endpoints, auth
   flows, or behavioural quirks in normative artifacts; fixtures carry spec
   citations, not `EhrBase ref:` markers; reference expectations are
   adjudicated against spec text, never against whichever SUT emitted them.
   (Red-run triage from the SUT side applies the same law: the vendored spec is
   the sole oracle and the application, runner, and catalogue are all suspects,
   none presumed correct — `.claude/rules/cnf-triage.md`.) The wire oracle is
   the ITS-REST **docs text**, never the (stalled) OAS — the OAS is `emit-rest`
   codegen input only, and where it and the docs text disagree, the text wins
   (owner ruling 2026-07-24).
   CI enforces what it can; the maintainer charter (§12) enforces the rest.
7. **Versioned like every other component.** Cases pin spec-version
   applicability ranges; a statement names the schedule release + tech profile
   it was earned against; within-major supersets follow openEHR's release
   strategy.
8. **Scope discipline.** Platform (CDR) profile first. Conformance =
   ISO/IEC 25010 functional suitability (including the Security & Privacy
   behaviours, §8.15) **plus performance efficiency** — the two verdict
   machineries (§6.3, §8.14); reliability, maintainability, and security
   *strength* (vs behaviour) stay out, declared in the Statement where
   relevant.
9. **Adopt international vocabulary** (§6.1) — ATS/ICS/IXIT/attestation
   levels/scheme-owner — instead of coining terms.

## 8. The CNF 2.0 normative artifact set — the production design

This section is the full design, not a sketch. It is derived from three
extractions performed against the vendored specs on 2026-07-21: (a) the
fleshed chapters of the official Test Schedule
(`platform_test_schedule/master03/04/06/07/17.3`, with master08/09 mined in
the v4 validation pass — the real case format, the
16-row create-EHR matrix, the per-row iteration law, the versioning cases,
the DV_QUANTITY decision tables); (b) the ITS-REST 1.1.0 wire contract, which
in Release-1.1.0 is a *decomposed OpenAPI* (`specifications/operations/*.yaml`
+ `responses/*.yaml` + `parameters/header/*.yaml`) — there are no per-API
prose status tables, so the binding layer below is driven from the OAS
fragments; and (c) the STABLE Simplified Formats specification
(`ITS-REST/docs/simplified_formats/master02–06`). Every rule below carries
its source.

**The architecture in one view.** Two **verdict machineries** over one
artifact discipline: **conformance-by-assertion** (functional + content
cases: typed assertions roll up case → capability → profile) and
**conformance-by-measurement** (performance cases: measured metrics against
class thresholds). Capabilities group into **families** — Platform
(CORE/STANDARD/OPTIONS), Enterprise (D/M/X — a proposed extension, §11.11), Security (SEC-BASIC, §8.15) — all
assessed by the assertion machinery; the certificate is the matrix
*machinery × family*: functional profile ratings per tech profile, plus an
earned performance class per environment. Below the machineries: one
schedule (case cores, three kinds), one binding layer (per SM operation per
ITS), one governed corpus (fixtures, recipes, views, scale classes,
workloads), one vocabulary layer (outcomes, the machine-readable
capability matrix, selectors), one party-artifact layer
(statement/results/ixit — ixit models **named SUT instances + an
environment**, so single-instance platform cases, dual-instance Enterprise
cases, and environment-bound performance runs all drive from the same file),
and one verdict layer (both machineries as pure functions). Content cases
are not a third machinery: a content case is a template-parameterized
functional execution (generate row instance → commit → expect verdict) —
one executor serves both.

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
12. **One commit may carry many versions, judged atomically** — a master08
    CONTRIBUTION bundles multiple VERSIONs (possibly of mixed RM types), each
    with its own change_type/lifecycle metadata, and the whole commit
    succeeds or fails as a transaction → bundled payloads + list captures +
    `for_each` assertions (§8.3, §8.6, pilot 8). DIRECTORY adds provisioned
    folder trees, at-time selection between captured commit instants, and
    scalar service returns → `requires.directory`, temporal references, and
    the `returns` assertion.

### 8.2 The artifact set

Seven normative, versioned-together artifact families in specifications-CNF
(each with a published schema or normative specification; where a JSON Schema exists it is the norm):

| # | Artifact | Path (proposed) | Content |
|---|---|---|---|
| 1 | **Case cores** | `schedule/<component>/<CASE_ID>.yaml` | Protocol-neutral test cases, all three kinds (§8.3, §8.14) — the Abstract Test Suite |
| 2 | **Operation bindings** | `bindings/<its>/<SM_OPERATION>.yaml` | Per-ITS wire realization of each SM operation's outcomes/captures (§8.4) |
| 3 | **Vocabularies & matrices** | `vocab/{outcomes,selectors}.yaml` + `vocab/capability_matrix.yaml` | The closed outcome taxonomy (§8.5), body/header selectors + ignore-sets (§8.4, §8.6), and the **machine-readable capability→family→tier matrix** — the Profiles book's capability×tier tables as data, the input the verdict machinery computes from; the Profiles book regenerates from the artifact set as a whole (capability tables from this matrix, the verdict-combination rules from family 7, the External Data Format attribute from the §8.7 format axis), with the same semantic-equivalence honesty as the schedule prose |
| 4 | **Governed corpus + manifest** | `corpus/**` + `corpus/MANIFEST.yaml` | Fixtures, templates, generated-set recipes, named views, **scale-class corpora** (shared by Enterprise + performance), **workload definitions**, adjudicated verdicts (§8.8) |
| 5 | **Ambiguity register** | `registers/ambiguities.yaml` | Known spec silences/divergences with normative handling (§8.5) |
| 6 | **Party artifacts** | `schemas/{statement,results,ixit}.schema.json` | The ICS/SDoC, test-report (incl. measurements), and SUT-topology contracts (§8.10) |
| 7 | **Verdict rules** | `schemas/verdicts.md` (normative prose) + reference impl | Both machineries as pure functions: assertion rollup + measured-class computation (§8.11, §8.14) |

The published spec pages (the human-readable schedule) are **generated** from
1–5; the derivation-square CI (§8.13) keeps every artifact internally linked.

**Encoding selection (pre-answering the bikeshed).** The normative artifact
is the data model (the published JSON Schema); file syntax is a serialization
choice, and each candidate gets exactly the job it is best at:

- **JSON** is the canonical interchange encoding — `statement.json`,
  `results.json`, `ixit.json` are hash-linked machine artifacts, and JSON
  parsers + JSON Schema validation exist natively in every runner ecosystem
  (Java, Python/Robot, JS, Groovy, Rust).
- **YAML** is the permitted authoring surface for case/binding files
  (comments, readable matrices); it parses to the same tree and is validated
  against the same schema in CI. If YAML's implicit-typing footguns worry
  the SEC, the fallback is JSON, not TOML.
- **TOML** was considered and rejected for case files on three hard grounds:
  it has **no null** (the official DV_QUANTITY table has `null` cells as
  first-class values), arrays-of-arrays/deep nesting (matrices, flows) are
  painful past two levels, and parser reach outside the Rust/Python config
  world is thin. TOML remains ideal for flat config-shaped *registers* and
  is used exactly there in the reference implementation.
- **TSV** serves two roles only: **generated indexes** (the catalogue
  listing — line-diff-friendly, never hand-edited) and the optional
  `rows_from:` bulk-row tables for large *generated* matrices (§8.3), which
  keeps a spreadsheet-authoring door open for content-chapter contributors
  without making runners implement a TSV join: inline typed rows remain the
  default, because TSV cells are untyped and cannot distinguish
  null/empty/absent.

### 8.3 The case core — full field definitions

One file per case. Normative fields (∎ = required):

| Field | Type | Semantics |
|---|---|---|
| `id` ∎ | string | Global CNF id. Families: `<SERVICE_COMPONENT>.<operation>-<variant>` (functional) and `CONT-<TYPE>-<variant>` (content) — both kept unchanged from the 2022 scheme; new chapters register their family with the maintainer group (this proposal registers `SF-<FORM>-<variant>` for the Simplified-Formats chapter). Ids are never reused; retired cases keep the id with `status: retired`. |
| `kind` ∎ | `functional \| content \| performance` | Selects which optional blocks are meaningful (performance cases: §8.14). |
| `status` | `active \| retired \| draft` | Default `active`. |
| `component` ∎ | enum | EHR, EHR_COMPOSITION, EHR_CONTRIBUTION, EHR_DIRECTORY, DEFINITION_ADL14, DEFINITION_ADL2, DEFINITION_QUERY, QUERY, DEMOGRAPHIC, ADMIN, MESSAGING, CONTENT, SIMPLIFIED_FORMATS, PERFORMANCE |
| `sm_operation` | string | Functional cases: the SM anchor (`I_EHR_SERVICE.create_ehr`). CI resolves it against the SM component list. |
| `rm_class` | string | Content cases: the RM/AM class under test (`DV_QUANTITY`). |
| `test_purpose` ∎ | string | The ISO/IEC 9646 test purpose — one narrow conformance requirement, prose. |
| `description` ∎ | string | The schedule's Description row. |
| `spec_refs` ∎ | string[] | Citations (component + document + section). CI link-checks them. |
| `applies` | map | Spec-version applicability ranges (`rm: ">=1.0.2"`, `aql: ">=1.1"` …) — range grammar = Cargo/semver requirement syntax. |
| `guards` | string[] | Non-version run conditions, each spec-cited (e.g. "modeling tool supports C_DV_QUANTITY list constraints — master17.3 NOTE"). A failed guard ⇒ `not-applicable`, citation mandatory. |
| `capabilities` ∎ (assertion-machinery cases) | string[] | The **verdict-bearing** capability names (§8.2 family 3 matrix) — keep MINIMAL: a case failure marks every listed capability `Failed` (§8.11 step 4). Performance cases carry `class` instead (their selection key, §8.11 step 2c) and omit this field. |
| `exercises` | string[] | Informative coverage tags: capabilities the case touches without bearing their verdict. |
| `profiles` | string[] | The profile **tier(s)** (CORE/STANDARD/OPTIONS) the capabilities belong to — derivable from the Profiles matrix, carried for readability; CI checks tier-vs-capability consistency. |
| `option` | string | For sibling cases realizing an ambiguity-register implementation choice (e.g. AMB-4): the option tag the ICS `options` declaration selects (§8.11 step 2b). |
| `formats` | string[] | Optional case-level format axis for cases **parameterized over** format: the case runs once per declared format ∩ the run's tech profile. Distinct from per-step `format:` (below) for cases whose formats are **intrinsic fixed roles** (round-trips). |
| `requires` | block | Typed prerequisites (below). |
| `parameters` | block | The data-set dimension (below). |
| `flow` ∎ (functional) | Step[] | Ordered steps (below). |
| `decision_table` ∎ (content) | block | Columns + rows (below). |
| `postconditions` | Assertion[] | Typed assertions (§8.6). Default evaluation is per parameter row; assertions marked `aggregate: true` (e.g. `unique`) evaluate once after all rows. |
| `verified_by` | string[] | Ids of cases that verify this case's deeper postconditions through separate reads (the master06 create→get pattern). CI checks the links resolve. |
| `ambiguities` | string[] | Ids into the ambiguity register that this case is subject to. |
| `data_sets` | string[] | Corpus manifest keys used (in addition to `parameters`). |

**`requires` block** — the schedule's precondition vocabulary, typed. Every
provisioned object mints a **named handle** usable as a variable in the flow:

```yaml
requires:
  server: empty            # empty | any        ("no EHRs, no commits, no OPTs")
  templates: []            # corpus keys provisioned before the flow
  ehr: none                # none | { commits: none | any }  — when present, mints ${ehr_id}
  directory: none          # none | <corpus key>  — a FOLDER tree provisioned in the EHR (master09)
  commit: []               # corpus set keys pre-committed into the EHR by the runner
                           #   (bulk setup is precondition state, never an un-anchored flow call)
```

`server: empty` is realized by runners through isolation (fresh SUT or
tenant), never by destructive cleanup of a shared system — a runner-layer
note, not a case concern. In multi-instance cases (§11.11), `requires` is
stated per instance (`instances: { source: {...}, target: { server: empty } }`).

**`parameters` block** — the data-set dimension. One mechanism serves the
functional matrices (master06) and the fixture sets (master04):

```yaml
parameters:
  iteration: reset_per_row   # reset_per_row (the master04 law) | single_pass
                             #   single_pass: rows execute against one shared server state —
                             #   required when an aggregate postcondition spans rows
  matrix:                    # inline value matrix (master06-style)
    columns: [ehr_status, is_queryable, is_modifiable, subject, other_details, ehr_id]
    rows: [ ... ]            # each row binds ${row.<column>}
    # rows_from: <path.tsv>  # optional bulk-row external table for large GENERATED matrices
    #                        #   (produced by a corpus recipe, never hand-edited)
  fixture_set:               # external-fixture iteration (master04-style)
    - { data_set: <corpus key>, expected: <outcome kind>, defect: "<why>", spec_ref: "<citation>" }
    # each entry binds ${fixture.data_set}, ${fixture.expected}, ${fixture.defect};
    # the current fixture's payload is referenced as ${ds:fixture}
```

Reserved matrix cell sentinels (normative, so a runner never confuses them
with literals): `absent` (omit the field entirely), `provided` (synthesize a
valid value via the case's recipe), `null` (JSON null). Reserved columns:
`expected` (per-row outcome override) and `violates` (content: the
violated-constraint list, §8.8 categories). Rows without `expected` inherit
the flow's expectations.

**Row-to-input synthesis**: where a step input is built *from* a row (not a
verbatim fixture), the case names a **recipe** declared in the corpus
manifest (§8.8) — `with: { ehr_status: ${recipe:ehr_status(row)} }`. The
recipe is committed, seeded, deterministic code; sentinels above govern
field presence.

**`flow` steps**:

```yaml
flow:
  - step: 1
    call: create_ehr                     # SM operation (short form resolves against sm_operation's interface)
    on: sut                              # OPTIONAL instance selector (default `sut`); Enterprise
                                         #   dual-instance cases address ixit-declared instances
                                         #   (e.g. `on: source` / `on: target` for dump/load, sync)
    format: wt-flat                      # OPTIONAL per-step format role (intrinsic-format cases only)
    with: { ehr_status: ${recipe:ehr_status(row)} }
    expect: created                      # outcome kind (§8.5); per-row override via the `expected` column
    capture: { ehr_id: created.ehr_id }  # logical captures; bindings map them to wire locations
    assert: []                           # optional post-step typed assertions (§8.6)
```

Variable reference grammar (closed): `${row.<column>}`, `${fixture.<field>}`,
`${<capture>}`, `${ds:<corpus key>}`, `${ds:<corpus key>#<view>}` (a named
projection declared in the manifest, §8.8), `${recipe:<name>(row)}`. Binding
path parameters (`{ehr_id}`, `{versioned_object_uid}`) resolve from the
case's variables — captures and `requires` handles. **There is no `${stepN}`
form**: a later step that needs an earlier response captures it explicitly
(`capture: { readback: ok.body }`).

Capture sources (closed): `<outcome>.<logical field>` as mapped by the
binding (e.g. `created.ehr_id`, `created.version_uid`), `<outcome>.body`
(the full response representation), `<outcome>.commit_time` (the committed
audit time — the anchor for temporal at-time cases). **List captures**: an
operation returning multiple values captures a list —
`capture: { version_uids: created.version_uids[] }` — asserted per-element
with `for_each` (§8.6).

**Bundled payloads (version sets)** — the master08 CONTRIBUTION construct: a
single call whose payload carries multiple members, each with its own
metadata, and ONE aggregate outcome (the commit is transactional):

```yaml
    with:
      versions:
        - { data: ${ds:<key>}, change_type: creation }
        - { data: ${ds:<key>}, change_type: modification, preceding_version_uid: ${v1} }
    expect: created            # or validation_failed — the AGGREGATE verdict; atomicity
    capture: { version_uids: created.version_uids[] }
```

**Temporal references** — for at-time/at-version selection (master07
`get_composition_at_times`, master09 `get_directory_at_time`): commit times
are captured (`t1: created.commit_time`) and at-time inputs use the closed
expressions `${time:before(<t>)}`, `${time:between(<t1>,<t2>)}`,
`${time:after(<t>)}` — resolved by the runner against the captured instants.

Rules: captures are case-scoped names; a step may reference any earlier
step's captures; `expect` names exactly one outcome kind — a case that needs
"either A or B" is two sibling cases carrying `option:` tags tied to an
ambiguity-register entry (§8.5, §8.11 step 2b). Substeps (the schedule's
`1.1`, `3.2`) are encoded as separate steps with a `variant:` tag when they
iterate different sources (see pilot 2).

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
  not_found:          { status: 404 }                               # unknown ehr_id (404_unknown_ehr_id.yaml)
  validation_failed:  { status: 422, body: error_loose }            # 422.yaml; AMB-1 error body
  template_not_found: { status: 422, body: error_loose }            # same wire code; kind distinguished by fixture
  missing_template_id:{ status: 422 }                               # simplified commit without openehr-template-id
  unsupported_media:  { status: 415 }     # layered from the overview negotiation rules — not in the operation's enumerated set (AMB-39)
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
  version_not_found:    { status: 404 }       # unknown preceding version — same 404 response family
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
  ok:            { status: 200, headers: { ETag: present? }, body: result_set_body }  # 200_Query.yaml
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
  `Content-Type` ⇒ 415. The deprecated/legacy simplified media types follow
  §8.7: correct 406/415 **where unsupported**, never mandatory rejection.
- **error_loose** body selector: see AMB-1 (§8.5) — assert at most that a
  `message` string is present, and only under `Prefer: return=representation`.

**Binding file — field contract** (∎ = required): `sm_operation` ∎,
`its` ∎, `applies`, `request` ∎ (`method` ∎, `path` ∎ with `{param}`
placeholders resolved from case variables, `body`, `headers`),
`formats`, `format_headers`, `outcomes` ∎ (map: outcome kind → wire
expectation `{status ∎, headers, body}`), `captures` (map: logical name →
source), `server_assigned` (the operation's ignore-set membership, below).

**Capture-source grammar** (closed): `header <Name>` ·
`header <Name> last-segment` · `body "<path>"` · `capture <name>` (derive
from another capture) — with optional modifiers `strip: weak-quotes`,
`transform: root-uid`, `fallback: <source>`. Nothing else.

**Header-matcher vocabulary** (closed): `present` · `present?` (optional —
assert only if the schedule row says so) · `absent` · `negotiated` ·
`latest-version-uid` · `pattern:<regex>` · a literal string.

**Body/header selector vocabulary** (closed, CI-checked like the outcome
kinds): `prefer_conditional` (full resource | `{uid}` | empty, per `Prefer`),
`error_loose` (AMB-1), `result_set_body` (the RESULT_SET schema — named
distinctly from the §8.6 `result_set` assertion), `negotiated` (equals the
negotiated media type), `present`, `absent`, and `pattern:<regex>` for header
values.

**Ignore-set membership is owned here**: each binding enumerates its
operation's `server_assigned` paths (e.g. create/update_composition:
`uid`, `context/_uid`, audit times, `system_id`-bearing ids); the
`ctx_defaults` set is enumerated once in the simplified-formats format
overlay (`context/start_time`, `context/setting`, composer defaults per
master06). The §8.6 `equivalent` assertion resolves named ignore-sets from
these lists — never from runner judgment.

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

(`ok_empty` and `stored` are forward-provisioned for the COMPOSITION
at-time-deleted and stored-query chapters; the closed-enum CI error bites
only *used* kinds.) Cases speak ONLY these kinds. Bindings map each kind to wire per operation
(the same kind may map to different codes on different operations — e.g.
`validation_failed` is 422 on composition ops but 400 on EHR create, per the
OAS). A kind a binding cannot map is a CI error.

**The ambiguity register** (`registers/ambiguities.yaml`) — every entry is a
real divergence or silence **CONFIRMED first-hand against the vendored spec
before it may exist** (the register is a suspect like any artifact we wrote; a
claimed ambiguity the spec actually DEFINES is a catalogue defect — the entry
is removed and the case made gating, never excused), with the normative
handling a runner must apply AND the outbound openEHR report it was raised as. Each entry carries
`ambiguity`, `source` (the first-hand spec citation), `handling`, a
machine-readable `disposition`, and an **`upstream_issue`** — the GitHub
issue number of the `upstream-report`-labeled tracker issue that pushes the
fix back upstream (owner ruling 2026-08-01; the former markdown ledger is
deleted — the issue IS the report). The
register is a living artifact; **the file is authoritative** — it has grown
well beyond the seed extraction. Entry categories (the authoritative, current
set — with citations, dispositions, and `upstream_issue`s — is the file itself,
so no per-id table is reproduced here to rot):

- **Released-spec silence** — a released component leaves a behaviour undefined
  (persistent-COMPOSITION uniqueness per EHR; reduced-precision temporal
  comparability; physical VERSIONED_OBJECT deletion). The case is `report_only`
  (reported, never gating) and the silence is reported upstream.
- **Released-spec contradiction / schedule defect** — a CNF-schedule row or data
  set contradicts a released component (e.g. a HISTORY row versus the RM
  `Events_valid` invariant; a range row versus the BASE `Interval` invariant
  set). The catalogue encodes the released-derivable reading (`editorial`) and
  proposes the upstream fix.
- **SM↔ITS realization gap** — the SM (an oracle) defines an operation the
  released ITS-REST does not yet realize (OPT delete, contribution-collection
  GET, versioned-directory, demographic relationships, admin/message ops). The
  case verdicts N/A-with-citation and an SM/ITS alignment report is filed.
- **Realization / naming note** (`fixed_handling`) — the released ITS-REST
  realizes an SM operation under a different shape or name (a whole-resource PUT
  for field-setters; a list GET for an existence probe; an SM name the schedule
  spells differently). Internal; no upstream report.

Every `report_only` and `editorial` entry carries an `upstream_issue` (§13);
the authoritative set is `registers/ambiguities.yaml` and each outbound
report is the GitHub issue it points at (label `upstream-report`).

Each entry carries a machine-readable **`disposition`** the pipeline
branches on (closed enum): `loose_assert` (assert only what the spec pins) ·
`fixed_handling` (handling encoded directly in bindings/cases) ·
`option_select` (sibling cases + ICS options) · `report_only` (verdicts
reported, never gating — reserved for genuinely open-upstream behaviour) ·
`statement_declared` · `editorial` (the schedule/spec text is itself defective;
the catalogue encodes the spec-derivable reading with a citation).

**Transparency is enforced, not optional.** The register never *absorbs* a
divergence — it documents it and reports it back. Every `report_only` and
`editorial` entry MUST carry an `upstream_issue` (enforced by the schema and by
`AmbiguityEntry::check_invariants`), so a gating suspension or a corrected spec
defect always has an outbound openEHR report attached (§13). This is
deliberate: a behaviour the spec leaves unassertable is more valuable
shown-and-reported than hidden — it is precisely where the spec needs fixing.
`report_only` is not a way to make red rows disappear; it is a cited,
upstream-linked suspension that reverts to gating when the upstream item
resolves.

The register is normative: a runner that "resolves" an ambiguity privately is
non-conformant to the schedule. It is **NOT an exclusion list** — every case
still runs; the register only governs how a spec-silent expectation is derived
and whether a genuinely-open-upstream behaviour gates the certificate. (Deleting
it in favour of "just let it fail" would be the opposite of honest: where the
spec assigns no value, an invented expectation fails every conformant server
and reports nothing useful; and an SM operation with no ITS-REST wire cannot be
"failed" at all. The register turns each such gap into a cited, actionable
upstream report instead.)

### 8.6 The assertion vocabulary

Typed assertions usable in `flow[].assert` and `postconditions` (all
evaluated per data-set row):

| Assertion | Fields | Semantics |
|---|---|---|
| `instance_of` | `rm_type`, `format?` | Body parses as the named RM type and validates against the ITS schema for the active format (canonical JSON ⇒ ITS-JSON; XML ⇒ XSD). |
| `field` | `path`, `equals \| exists \| absent \| matches` | RM-path-addressed field check; values may reference `${row.*}`/captures — e.g. `path: ehr_status/is_queryable, equals: ${row.is_queryable}`. |
| `equivalent` | `to: committed \| ${ds:…} \| ${capture}`, `ignoring:` named ignore-sets (`server_assigned`, `ctx_defaults`) and/or explicit `[paths]` | The master07 "content check": retrieved content equals committed content, modulo the declared server-assigned set (`uid`, `system_id`, audit times, …) — the ignore set is normative per operation, not runner-chosen. |
| `version` | `of: ${<version-uid capture>}` (the target version), `for_each: ${<list capture>}` (per-element over a list capture), `change_type \| lifecycle_state \| count \| uid_pattern` | RM versioning facts: `of: ${v2_uid}, change_type: MODIFY`, `lifecycle_state: "openehr::523\|deleted\|"` (AMB-11), `count: 2`, `uid_pattern: "<root>::<system>::2"`. `count` needs no `of:`. |
| `result_set` | `match: ordered \| set \| count \| contains`, `rows`, `columns?` | AQL results, compared under the normative equivalence rules below. |
| `unique` | `over: ${capture}`, `aggregate: true` | Values captured across rows are pairwise distinct (create_ehr-main's ehr_id uniqueness sub-constraint). Aggregate: evaluated once after all rows; requires `iteration: single_pass`. |
| `returns` | `equals \| matches` | Scalar service returns (master09 `has_path`/`has_directory` booleans) — asserted directly, no RM body. |
| `message_exemplar` | `text` | Informative only — the schedule's ``"EHR with <ehr_id> does not exist"`` prose; never a pass/fail criterion (AMB-1). |
| `state` | `text`, `verified_by?` | A prose postcondition whose machine verification lives in a linked case (the master06 create→get pattern). CI requires either a `verified_by` resolution or an in-case verification step. |

**RESULT_SET equivalence rules** (normative; each rule is either **[spec]**
— stated by the vendored specs, cited — or **[legislated]** — a proposed
default the specs are silent on, for SEC ratification with U5):

1. **Comparison scope** — equivalence is over `rows` only; every `meta`
   field is excluded (**[spec]**: all `ResultSetMetadata` fields optional and
   "implementation dependent … useful for debugging" —
   `ITS-REST schemas/query/ResultSetMetadata.yaml`, `docs/query/Response.md`);
   `columns` are compared only when the case asserts them, since the array
   itself is optional (**[spec]**: `ResultSet.yaml` `required: [rows]`).
   Column identity: the `AS` alias, else `#<0-based index>` (**[spec]**:
   `ResultSetColumn.yaml`).
2. **Order** — `match: ordered` is legal only when the query carries an
   ORDER BY that totally orders the expected rows (**[spec]**: absent
   ORDER BY, "default ordering in results is undefined" — QUERY
   `master03-syntax.adoc` §ORDER BY; LIMIT determinism requires unique
   ordering — §LIMIT). Otherwise `match: set`, which despite its name is
   **bag (multiset)** equality — duplicate rows are significant, because AQL
   is bag-semantics unless `DISTINCT` is present (**[spec]**: §DISTINCT).
   `match: count` compares row count only; `match: contains` requires every
   expected row to appear (bag-wise) with extra rows permitted. ORDER BY
   semantics: ASC default, left→right lexicographic tie-break (**[spec]**:
   §ORDER BY).
3. **Cell equality** — an RM-object cell (carries `_type`) compares by
   canonical-JSON structural equality (**[spec]**: cells may be full RM
   objects — QUERY `master04-result_structure.adoc`,
   `ResultSetRow.yaml`); a scalar numeric cell compares by **numeric
   value**, not lexeme — `140` = `140.0` (**[legislated]**: no spec rule on
   projected-scalar number typing); a void cell is encoded as JSON `null`
   and equals only `null` (**[legislated]**: AQL names the value NULL, the
   wire encoding is unpinned).
4. **NULL ordering** — under ORDER BY, null cells sort **last** ascending,
   **first** descending (**[legislated]**: QUERY is silent on NULL sort
   position).
5. **Under-specified orderings are avoided, not legislated** — RM
   `DV_ORDERED` comparison is itself incompletely specified for partial
   dates/timezones/durations, so schedule cases MUST NOT order or
   discriminate on values whose comparison the RM leaves open
   (**[legislated]** avoidance rule).
6. **Counts are always determined** — cases pass `fetch`/`LIMIT` explicitly
   (AMB-6: the default fetch is implementation-defined) and never truncate
   without a total ordering (**[spec]**: §LIMIT + §ORDER BY).

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
`application/openehr.wt+json`. The deprecated aliases
(`…wt.flat.schema+json`, `…wt.structured.schema+json`) and legacy types
(`application/openehr.nc.flat+json`, `application/openehr.tds2+xml`) are
listed in the ITS-REST overview (`Resources.md`) as deprecated/MAY-supported:
a server MAY still accept them, so cases assert only **correct negotiation
behaviour** — a type the server does not support yields 406 (Accept) / 415
(Content-Type) — never mandatory rejection, which would both exceed the spec
and contradict AMB-39.

Two distinct format models (both defined in §8.3): a case **parameterized
over** format declares a case-level `formats:` axis and runs once per
declared format ∩ the run's tech profile; a case whose formats are
**intrinsic fixed roles** (round-trips like pilot 6) pins `format:` per step
and is selected only when its required formats ⊆ the tech profile —
otherwise `not-applicable` with the tech profile as citation. Verdicts are
per tech profile either way. The ✘ cells are themselves conformance cases
(the 406/415 negatives).

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
    deprecated + legacy media types → correct 406/415 where unsupported).
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
  provenance: "openEHR CNF Robot corpus @33251d2a; vendor markers stripped; re-adjudicated <date>"
  views: {}          # named projections referenced as ${ds:<key>#<view>}:
                     #   each view = { select: <path expression over the set>,
                     #   where: <predicate>, order_by: <path> } — declarative,
                     #   evaluated over the corpus data, runner-independent
  recipes: {}        # named row-to-instance synthesis functions referenced as
                     #   ${recipe:<name>(row)}: name-resolved against the
                     #   runner's registered recipe set (committed, seeded,
                     #   deterministic `row → RM-fragment` functions); the
                     #   manifest entry records name + content digest so any
                     #   runner can verify it executes the same recipe version
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

These are the *official* schedule cases (plus two new-chapter candidates),
encoded losslessly — the proof artifacts the upstream proposal ships with.

**Pilot 1 — `I_EHR_SERVICE.create_ehr-main`** (master06 — both VALID
data-set classes: class 1.a *omitted* EHR_STATUS with server defaults, and
the official 16-row *provided*-status matrix; the schedule's own table
caption mislabels the provided-status table "1.a" against its own class
list — registered as AMB-12):

```yaml
id: I_EHR_SERVICE.create_ehr-main
kind: functional
component: EHR
sm_operation: I_EHR_SERVICE.create_ehr
capabilities: [EhrOperations]
profiles: [CORE]
test_purpose: >
  Creating an EHR succeeds for every valid EHR_STATUS variant, and for an
  omitted EHR_STATUS the server creates the defaults (is_queryable=true,
  is_modifiable=true, subject=PARTY_SELF).
description: "Create new EHR"
spec_refs:
  - "SM openehr_platform §I_EHR_SERVICE.create_ehr"
  - "CNF platform_test_schedule master06 §create_ehr data sets"
applies: { rm: ">=1.0.2" }
requires: { server: empty }
parameters:
  iteration: single_pass     # all EHRs coexist — the cross-row uniqueness
                             # postcondition is only meaningful on shared state
  matrix:
    columns: [ehr_status, is_queryable, is_modifiable, subject, other_details, ehr_id]
    rows:
      # class 1.a — EHR_STATUS omitted (server defaults); with and without client ehr_id
      - [absent, -,     -,     -,        -,        absent]
      - [absent, -,     -,     -,        -,        provided]
      # class 1.b — the official 16-row provided-status matrix, verbatim
      - [provided, true,  true,  provided, absent,   absent]
      - [provided, true,  false, provided, absent,   absent]
      - [provided, false, true,  provided, absent,   absent]
      - [provided, false, false, provided, absent,   absent]
      - [provided, true,  true,  provided, provided, absent]
      - [provided, true,  false, provided, provided, absent]
      - [provided, false, true,  provided, provided, absent]
      - [provided, false, false, provided, provided, absent]
      - [provided, true,  true,  provided, absent,   provided]
      - [provided, true,  false, provided, absent,   provided]
      - [provided, false, true,  provided, absent,   provided]
      - [provided, false, false, provided, absent,   provided]
      - [provided, true,  true,  provided, provided, provided]
      - [provided, true,  false, provided, provided, provided]
      - [provided, false, true,  provided, provided, provided]
      - [provided, false, false, provided, provided, provided]
flow:
  - step: 1
    call: create_ehr
    with: { ehr_status: ${recipe:ehr_status(row)}, ehr_id: ${row.ehr_id} }
    expect: created
    capture: { new_ehr_id: created.ehr_id }
postconditions:
  - { assert: unique, over: ${new_ehr_id}, aggregate: true }   # "ehr_id … should be unique"
  - { assert: state, text: "EHR exists and is consistent with the data set used
      (class 1.a rows: server defaults applied)",
      verified_by: I_EHR_STATUS.get_ehr_status-get_by_ehr_id }
verified_by: [I_EHR_STATUS.get_ehr_status-get_by_ehr_id]
ambiguities: [AMB-12]
```

**Pilot 2 — `I_EHR_SERVICE.create_ehr-same_ehr_twice`** (master06 — the
state-carrying failure case; the two ehr_id sources the schedule
distinguishes — "read from the response" vs "read from the test data sets" —
are the two matrix rows; the exactly-one-EHR postcondition is verified
in-case):

```yaml
id: I_EHR_SERVICE.create_ehr-same_ehr_twice
kind: functional
component: EHR
sm_operation: I_EHR_SERVICE.create_ehr
capabilities: [EhrOperations]
profiles: [CORE]
test_purpose: "ehr_id values are unique: re-creating an existing EHR is rejected."
description: "Attempt to create same EHR twice"
spec_refs:
  - "SM openehr_platform §I_EHR_SERVICE.create_ehr"
  - "CNF platform_test_schedule master06 §create_ehr-same_ehr_twice"
applies: { rm: ">=1.0.2" }
requires: { server: empty }
parameters: { iteration: reset_per_row,
              matrix: { columns: [ehr_id], rows: [[absent], [provided]] } }
flow:
  - step: 1
    call: create_ehr
    with: { ehr_id: ${row.ehr_id} }
    expect: created
    capture: { first_ehr_id: created.ehr_id }   # server-assigned OR data-set value — both rows covered
  - step: 2
    call: create_ehr
    with: { ehr_id: ${first_ehr_id} }           # "should be read from the response" / "from the test data sets"
    expect: already_exists
  - step: 3                                     # in-case verification of the postcondition
    call: get_ehr
    with: { ehr_id: ${first_ehr_id} }
    expect: ok
    assert:
      - { assert: instance_of, rm_type: EHR }
postconditions:
  - { assert: state, text: "Exactly one EHR exists — the one created in step 1
      (verified by step 3 retrieving it unchanged)" }
```

**Pilot 3 — `I_DEFINITION_ADL14.upload_opt-invalid_opt`** (master04 — the
fixture-set iteration with per-fixture defects; postcondition = unchanged
server):

```yaml
id: I_DEFINITION_ADL14.upload_opt-invalid_opt
kind: functional
component: DEFINITION_ADL14
sm_operation: I_DEFINITION_ADL14.upload_opt
capabilities: [Adl14OptProvisioning]
exercises: [ArchetypeValidation]
profiles: [CORE]
test_purpose: "Invalid OPTs are rejected and leave the server state unchanged."
description: "upload invalid OPTs"
spec_refs:
  - "SM openehr_platform §I_DEFINITION_ADL14.upload_opt"
  - "CNF platform_test_schedule master04 §upload_opt data sets"
applies: { rm: ">=1.0.2" }
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
    with: { opt: ${ds:fixture} }
    expect: ${fixture.expected}
postconditions:
  - { assert: state, text: "No OPTs are loaded on the system",
      verified_by: I_DEFINITION_ADL14.get_opts-retrieve_all_no_opts }
verified_by: [I_DEFINITION_ADL14.get_opts-retrieve_all_no_opts]
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
capabilities: [Versioning]
exercises: [CompositionOps, ChangeSets]
profiles: [CORE]
test_purpose: >
  Updating an existing event COMPOSITION with the correct
  preceding_version_uid creates a second VERSION with change_type MODIFY.
description: "Update an existing event COMPOSITION"
spec_refs:
  - "SM openehr_platform §I_EHR_COMPOSITION.update_composition"
  - "CNF platform_test_schedule master07 §update_composition-event"
  - "RM common §change_control (VERSION.commit_audit.change_type)"
applies: { rm: ">=1.0.2" }
requires:
  server: any
  templates: [cnf.opt.minimal_event]
  ehr: { commits: none }                 # mints ${ehr_id}
data_sets: [cnf.composition.minimal_event.v1, cnf.composition.minimal_event.v2]
flow:
  - step: 1
    call: create_composition
    with: { ehr_id: ${ehr_id}, composition: ${ds:cnf.composition.minimal_event.v1} }
    expect: created
    capture: { preceding_version_uid: created.version_uid,
               versioned_object_uid: created.versioned_object_uid }
  - step: 2
    call: update_composition
    with: { ehr_id: ${ehr_id},
            composition: ${ds:cnf.composition.minimal_event.v2},
            versioned_object_uid: ${versioned_object_uid},
            preceding_version_uid: ${preceding_version_uid} }   # ITS-REST: If-Match (AMB-3)
    expect: updated
    capture: { v2_uid: updated.version_uid }
    assert:
      - { assert: version, of: ${v2_uid}, uid_pattern: "${versioned_object_uid}::<system>::2" }
postconditions:
  - { assert: version, count: 2 }
  - { assert: version, of: ${preceding_version_uid}, change_type: CREATE }
  - { assert: version, of: ${v2_uid},                change_type: MODIFY }
  # NOTE: a strengthening addition — master07 places the "content check" in the
  # get_composition cases, not in update_composition; kept here as extra rigor.
  - { assert: equivalent, to: committed, ignoring: server_assigned }
ambiguities: [AMB-3]
```

(The negative siblings: the official `update_composition-non_existent` —
step 2 `with: preceding_version_uid: random`, `expect: version_not_found` —
and the REST-specific stale-latest variant, `expect: precondition_failed`
→ 412 with the latest ETag. Both outcome kinds are mapped by the
update_composition binding, §8.4.)

**Pilot 5 — `CONT-DV_QUANTITY-validate_property_units_mag`** (master17.3,
the richest official decision table, verbatim rows — structured constraint
literals; this table carries one violation per row, and the `violates` list
form also covers the multi-violation rows used elsewhere in master17):

```yaml
id: CONT-DV_QUANTITY-validate_property_units_mag
kind: content
component: CONTENT
rm_class: DV_QUANTITY
capabilities: [ArchetypeValidation]
profiles: [CORE]
test_purpose: >
  A committed DV_QUANTITY is accepted iff it satisfies the C_DV_QUANTITY
  property + units-list + per-unit magnitude-range constraints.
description: "DV_QUANTITY against C_DV_QUANTITY with property, units and magnitude range"
spec_refs:
  - "CNF platform_test_schedule master17.3 §CONT-DV_QUANTITY-validate_property_units_mag"
  - "AM aom14 §C_DV_QUANTITY"
  - "RM data_types §DV_QUANTITY"
applies: { rm: ">=1.0.2" }
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
categories 1+7 — every rule cited to the STABLE Simplified Formats spec; an
*intrinsic-format* case: the formats are fixed roles per step, and the case
is selected only when its required formats ⊆ the run's tech profile, §8.7):

```yaml
id: SF-FLAT-commit_roundtrip_ctx_defaults
kind: functional
component: SIMPLIFIED_FORMATS
sm_operation: I_EHR_COMPOSITION.create_composition
capabilities: [SimplifiedFormats]
profiles: [OPTIONS]           # SHOULD-level per ITS-REST — never gates CORE/STANDARD
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
requires:
  server: any
  templates: [cnf.opt.vitals]
  ehr: { commits: none }                 # mints ${ehr_id}
data_sets: [cnf.flat.vitals.minimal_ctx]
flow:
  - step: 1
    call: create_composition
    format: wt-flat                      # intrinsic role; binding adds openehr-template-id
    with: { ehr_id: ${ehr_id}, composition: ${ds:cnf.flat.vitals.minimal_ctx} }
    expect: created
    capture: { version_uid: created.version_uid }
  - step: 2
    call: get_composition
    format: canonical-json
    with: { ehr_id: ${ehr_id}, version_uid: ${version_uid} }
    expect: ok
    assert:
      - { assert: instance_of, rm_type: COMPOSITION }
      - { assert: field, path: "context/setting", equals: "openehr::238|other care|" }   # master06 default
      - { assert: field, path: "context/start_time", exists: true }                      # ctx/time → now()
      - { assert: field, path: "content[0]/data/events[0]/data/items[0]/value/magnitude",
          equals: ${ds:cnf.flat.vitals.minimal_ctx#temperature_magnitude} }              # named view (§8.8)
  - step: 3
    call: get_composition
    format: wt-flat
    with: { ehr_id: ${ehr_id}, version_uid: ${version_uid} }
    expect: ok
    capture: { flat_readback: ok.body }
    assert:
      - { assert: equivalent, to: committed, ignoring: [ctx_defaults, server_assigned] }
  - step: 4
    call: get_composition
    format: wt-structured
    with: { ehr_id: ${ehr_id}, version_uid: ${version_uid} }
    expect: ok
    assert:
      - { assert: equivalent, to: ${flat_readback}, ignoring: [] }   # FLAT↔STRUCTURED value-equality (master04)
```

**Pilot 7 — `I_QUERY_SERVICE.execute_adhoc-where_magnitude`** (new-chapter
candidate for the empty master11 — deterministic, RESULT_SET-shape-aware;
bulk data load is precondition state via `requires.commit`, not a flow call):

```yaml
id: I_QUERY_SERVICE.execute_adhoc-where_magnitude
kind: functional
component: QUERY
sm_operation: I_QUERY_SERVICE.execute_adhoc_query
capabilities: [AqlBasic]
profiles: [STANDARD]
test_purpose: >
  An ad-hoc AQL query with a WHERE predicate on DV_QUANTITY.magnitude
  returns exactly the matching compositions, as a spec-shaped RESULT_SET.
description: "Ad-hoc AQL, WHERE on magnitude, ordered result"
spec_refs:
  - "QUERY AQL 1.1 §WHERE, §ORDER BY"
  - "ITS-REST query §Response (RESULT_SET: rows required; columns, meta optional)"
applies: { rm: ">=1.0.2", aql: ">=1.1" }
requires:
  server: any
  templates: [cnf.opt.blood_pressure]
  ehr: { commits: none }                 # mints ${ehr_id}
  commit: [cnf.set.bp-10]                # generated: 10 BP compositions, magnitudes 100..190 (recipe in corpus)
flow:
  - step: 1
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
          rows: { from: "${ds:cnf.set.bp-10#magnitude_ge_140_by_uid}" },   # named view (§8.8)
          columns: [{ name: uid }] }
```

**Pilot 8 — `I_EHR_CONTRIBUTION.commit_contribution-valid_invalid_compositions`**
(master08 — the construct v3 could not express: one CONTRIBUTION carrying
multiple VERSIONs, judged as a single atomic transaction; master08's note:
"the whole commit should behave like a transaction and fail"):

```yaml
id: I_EHR_CONTRIBUTION.commit_contribution-valid_invalid_compositions
kind: functional
component: EHR_CONTRIBUTION
sm_operation: I_EHR_CONTRIBUTION.commit_contribution
capabilities: [ChangeSets]
profiles: [CORE]
test_purpose: >
  A CONTRIBUTION containing one valid and one invalid COMPOSITION is
  rejected atomically — no VERSION of either is created.
description: "One commit, multiple versions, one invalid — transactional rejection"
spec_refs:
  - "SM openehr_platform §I_EHR_CONTRIBUTION.commit_contribution"
  - "CNF platform_test_schedule master08 §commit_contribution-valid_invalid_compositions (+ transaction note)"
applies: { rm: ">=1.0.2" }
requires:
  server: any
  templates: [cnf.opt.minimal_event]
  ehr: { commits: none }                 # mints ${ehr_id}
flow:
  - step: 1
    call: commit_contribution
    with:
      ehr_id: ${ehr_id}
      versions:                          # the bundled-payload construct (§8.3)
        - { data: ${ds:cnf.composition.minimal_event.v1},               change_type: creation }
        - { data: ${ds:cnf.composition.minimal_event.invalid_structure}, change_type: creation }
    expect: validation_failed            # ONE aggregate outcome — the commit is a transaction
postconditions:
  - { assert: version, count: 0 }        # atomicity: nothing was committed
```

(The positive sibling, `commit_contribution-valid_compositions`, commits two
valid versions in one CONTRIBUTION, `expect: created`, captures
`version_uids: created.version_uids[]`, and asserts
`{ assert: version, for_each: ${version_uids}, change_type: CREATE }` +
`{ assert: version, count: 2 }` — exercising the list-capture and
per-element assertion machinery. Mixed-RM-type sets — COMPOSITION +
EHR_STATUS + FOLDER in one CONTRIBUTION — use the same `versions[]`
construct with per-member `data`.)

### 8.10 The ICS (statement), results, and IXIT schemas

Field-level contracts (JSON Schemas published with the schedule):

**`statement.json` — the ICS + SDoC** — one artifact deliberately carrying
two distinct standard roles: the **ISO/IEC 9646 ICS** (the capability
proforma that drives test selection) *and* the **ISO/IEC 17050-1**
supplier's-declaration content that makes it a legal SDoC (distinct
artifacts in the source standards, combined here as one computable file):

| Field | Semantics |
|---|---|
| `product` ∎ | name, **exact version/build**, vendor, unique product identifier |
| `schedule_release` ∎ | the CNF schedule release the claims are made against |
| `spec_versions` ∎ | declared RM/AQL/ITS-REST/TERM versions (drives `applies` filtering) |
| `claims` ∎ | claimed capabilities + profiles, validated against the machine-readable capability matrix (§8.2 family 3) |
| `tech_profiles` ∎ | which format/protocol matrices are claimed (e.g. `[its-rest: [canonical-json, canonical-xml, wt-flat]]`) |
| `options` | declared behaviour for register-listed implementation choices (e.g. AMB-4: conflict vs version-param) |
| `performance` | the claimed **volumetric class per declared environment** (`POC`/`S`/`L`/`R`, §8.14) — a verdict input for the performance dimension: the claim selects the performance cases to run, and the earned class is computed from measured `results.json` thresholds exactly like functional verdicts |
| `non_functional` | remaining declaration-only slots (security/privacy *posture* beyond the §8.15 SEC-BASIC behaviours: encryption configuration, pseudonymisation-on-export) — never verdict inputs |
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
| `measurements` | performance runs: per-case metric values + per-class `earned \| not-earned` verdicts, with the mandatory environment block (§8.14) |
| `ambiguity_dispositions` | which register options the run exercised |

`errored` (transport/SUT fault) is never a conformance finding. Mapping to
the ISO/IEC 9646 verdicts: `passed`→pass, `failed`→fail,
`errored`→**inconclusive**; `not-applicable` and `skipped` are **not** 9646
verdicts — they record ICS-driven selection and guard exclusions, each with
a mandatory citation. Coverage is computable: cases driven / cases selected
by the ICS, per profile.

**`ixit.json`** (9646 IXIT): the SUT **topology** — one or more **named
instances** (each: base URL, auth mode + credentials reference, admin mount,
template-id policy, system-id expectations, per-endpoint overrides) plus the
**environment block** (hardware class, cores, memory, storage class,
deployment topology — mandatory for performance runs, §8.14).
Single-instance platform cases use the default instance `sut`; Enterprise
dual-instance cases (§11.11) address `source`/`target` via the flow `on:`
selector (§8.3); performance verdicts bind to the environment. One file
drives any runner against any SUT topology. (ECC's `SutDescriptor` is the
donated draft.)

### 8.11 ICS-driven selection and verdict computation

Mechanical pipeline, normative — a pure function of (statement, results,
catalogue, **capability matrix**):

1. **Static conformance review** of the statement: claim-set legality
   against the capability matrix (STANDARD ⇒ all CORE capabilities claimed),
   spec-version consistency, option declarations present for every register
   entry the claims touch, an environment block present when a performance
   class is claimed.
2. **Selection**: cases whose `capabilities` ∩ claimed capabilities ≠ ∅,
   filtered by `applies` × declared spec versions and by `guards`.
   **2b — option deselection**: a case carrying an `option:` tag is selected
   only when the ICS `options` declaration matches it; the sibling
   realizing the undeclared behaviour is recorded `not-applicable` with the
   ICS declaration as citation (AMB-167).
   **2c — performance selection**: the claimed class per environment selects
   that class's performance cases; unclaimed classes are not run (a product
   claims S, it is measured for S — running R unasked is a runner choice,
   reported but not demanded).
3. **Execution**: per case × tech-profile format × parameter row, under the
   interpreter laws: **(a)** `reset_per_row` re-establishes the whole
   `requires` block around every row — for `server: empty` that means a
   fresh tenant/scope per row (cases where that cost is disproportionate use
   `single_pass`, which exists for exactly this reason); **(b)** a step whose
   observed outcome differs from `expect` fails the row and **aborts its
   remaining steps and row postconditions**; **(c)** transport/connection
   faults, timeouts, and responses no binding outcome maps → `errored`
   (inconclusive) — a *mapped but unexpected* outcome → `failed`;
   **(d)** `${time:before(t)}` = t − 1 ms, `${time:after(t)}` = t + 1 ms,
   `${time:between(t1,t2)}` = the midpoint — fixed rules so two runners
   query identical instants; **(e)** aggregate assertions collect their
   `over:` capture across all rows and evaluate once after the last row.
4. **Verdicts — computed per tech profile**: case passes iff every selected
   row passes in that tech profile. A failed case marks **every**
   verdict-bearing capability it lists `Failed` (which is why `capabilities`
   stays minimal and coverage moves to `exercises`, §8.3). Capability
   evidence: `Passed` (≥1 case ran, none failed) / `Failed` /
   `NotEvidenced` / `NoCases` (a printed coverage bound). Profile verdicts
   per the capability matrix: CORE/STANDARD = all required capabilities
   `Passed`; OPTIONS = any. `report_only`-disposition cases (AMB-5, AMB-29)
   report but never gate.
5. **Measured verdicts** (the second machinery): per claimed class, every
   §8.14 threshold holds in one measured run ⇒ class `earned`, else
   `not-earned`; bound to the ixit environment.
6. Everything above is a pure function of (statement, results, catalogue,
   capability matrix) — a reference implementation ships with the schemas;
   any two conformant implementations MUST compute identical verdicts.

### 8.12 Runner verification — the two-part pack

A runner claims schedule compliance through:

1. **Verdict conformance** — replay a fixed transcript and reproduce the
   adjudicated verdicts + emit schema-valid `results.json`. The transcript
   is itself a specified artifact (`transcript.schema.json`): an ordered
   sequence per case × format × row of `{ step, request: {method, path,
   headers, body_digest}, response: {status, headers, body},
   expected_verdict, adjudication_ref }` — replayed by sequence (the fixture
   server answers the Nth matching request with the Nth response; matching =
   method + path + negotiated media type), including deliberate
   fail/N-A/skip/guard outcomes and the AMB-1 error-body variants. A fixture
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
links resolve; `option:` tags resolve to register entries;
capability-vs-tier consistency against the Profiles matrix; reference and
sentinel grammar checks (`${…}` forms, `absent`/`provided`/`null`);
decision-table literals parse against the published grammar;
prose regeneration succeeds. This is the mechanism that lets the repo accept
community PRs without a bottleneck maintainer — and it is ECC's
coverage-guard discipline (`tools/conformance/tests/coverage.rs`),
generalized.

The **`surface-coverage` gate** (issue #271) closes the loop on breadth: it
enumerates the spec-defined wire surface from the RELEASED sources only — the
SM platform interfaces (`docs/specs/openehr/SM/`) × their ITS-REST-docs-text
wire branches, never the OAS — and fails on any behaviour (an SM operation,
a realized binding's outcome/format branch, or a cross-cutting header/
negotiation/error-family element) with neither a covering case nor an
adjudicated `vocab/wire_surface.yaml` exception. Silence is not coverage;
coverage only ratchets up. `cnf-runner validate --specs …` refreshes the
deterministic per-interface/per-binding coverage report at
`docs/conformance/coverage-report.md`.

The **claim-completeness gates** (issue #622) close the same loop on the CLAIM
side, so a certification claim can never be hollow. `validate` sweeps the
committed party statements beside the artifact root
(`<root>/../party/*/statement.json`) and relates them to the catalogue:

- **`claim-completeness`** — a capability a statement claims must have at
  least one verdict-bearing catalogue case (an active case naming it whose
  gating is not suspended by a `report_only` register entry). Declaring a
  capability IS the obligation to run the framework against it, so a hollow
  claim fails before any SUT is composed. A capability whose cases ALL resolve
  excused (an unrealized wire) or deselected (an undeclared option branch)
  must additionally name the register entry that adjudicated that in its
  matrix row's `evidence_exception`; the reverse also bites — an
  `evidence_exception` on a capability that can carry executed evidence is
  stale and must go, so an excuse can never outlive the wire it excused.
  ISO/IEC 9646 test selection legitimizes "not applicable" only for a
  capability a party does NOT claim.
- **`capability-depth`** — one token case never certifies a capability. Each
  matrix row records `min_cases`, the verdict-bearing case count its battery
  must keep; falling below it names the capability and the shortfall. Floors
  ratchet UP only: raising one to the current depth is always safe, so the
  committed floors are derived from the catalogue and re-derived by a test.
- **`workload-coverage`** — a claimed capability the measured
  hospital-simulation workload does not exercise must carry a register-linked
  `workload_exclusion` on its matrix row, which the certificate's Workload
  Coverage table renders in place of the bare `NO — catalogue gap` cell; an
  exclusion on a capability the simulation has since started exercising is
  stale and must go. No released openEHR component governs a measured
  workload at all (CNF guide `master03-overview.adoc` §Product Scope assesses
  API conformance and data-validation conformance only), so the whole
  instrument — and every exclusion from it — is our own design/extension,
  declared once in the register and enumerated per capability in the matrix.

The matrix row also carries **`realization: released-wire | extension`**,
rendered as its own certificate column: an `extension` row is verified over
routes no openEHR specification governs, so it may never be `required` and no
openEHR profile tier ever rests on it.

**Extension realizations (issue #623).** Where a capability the CNF Profiles
book names has NO released wire, the honest question is not "can we excuse
it?" but "does the product serve it at all?", and the answer decides the row:

- **The product serves it on a declared route** → the operation's binding
  carries an `extension:` block (`family` + `reason` + `source` + the register
  `ambiguity`) alongside its full request/outcomes form, and its battery is
  EXECUTED. `extension` and `unrealized` are mutually exclusive by
  construction.
- **Nothing serves it** → the STATEMENT stops claiming the capability. A claim
  with no testable surface is the dishonest row; ISO/IEC 9646 test selection
  legitimizes "not applicable" precisely for a capability a party does NOT
  claim. The matrix row stays (the matrix is the Profiles book as data, not a
  claim list) and keeps its `evidence_exception` as a statement of catalogue
  fact.

The fence around an extension realization is the **`realization-scope`** gate,
not a convention: the binding's `family` and request-path SHAPE must resolve in
the `served_extensions` axis (so a binding can only drive a route the SUT
declares outwardly), its adjudication must resolve in the register, a
capability whose verdict-bearing cases ALL drive extension bindings must carry
`realization: extension`, and the mirror bites too — an `extension` marker on a
capability whose cases drive released operations is stale and understates the
conformance the product earned. Extension bindings are also excluded from the
released-path set the Axis-4 claim check compares against, since their paths
are by construction the declared extension routes.

Extension realizations are also **party-scoped at SELECTION** (`run.rs`,
beside the option-branch and version-floor arms): a case driving an extension
binding is not-applicable — with the family + register id as its citation —
for a party whose statement claims none of the case's capabilities. A route
openEHR does not specify is an offer only the party that CLAIMS it answers
for; driving it at another vendor's system under test would publish failures
for routes that vendor never offered to serve, and the published comparison
has to be honest in both directions.

### 8.14 The performance & volumetrics schedule

Performance conformance is its own dimension with its own machine-readable
schedule — same artifact discipline, different verdict machinery. A
performance case (`kind: performance`) defines:

```yaml
# schedule/performance/PERF-hospital_sim-class_S.yaml (the journey shape,
# 2026-07-22 — the original flat mix `{ composition_read: 61%, adhoc_query:
# 30%, composition_commit: 8%, ehr_create: 1% }` is preserved here as the
# derivation's historical first realization; the journey catalogue
# decomposes the same envelope)
id: PERF-hospital_sim-class_S
kind: performance
component: PERFORMANCE
description: "Class-S sustained hospital-simulation workload"
test_purpose: >
  Under the class-S normative offered load, decomposed into clinical
  journeys over the whole claimed platform surface, the platform sustains
  the class-S latency and throughput thresholds on every operation.
spec_refs: ["CNF 2.0 performance schedule §classes (this proposal; 2017 schedule lineage)"]
class: S                        # POC | S | L | R — the selection key (§8.11 step 2c);
                                # performance cases carry no `capabilities` (§8.3)
corpus: cnf.scale.100k          # synthesized corpus recipe (§11.11 scale classes)
workload:                       # OPEN-LOOP offered load:
  arrival_rate: 15/s            #   the class-S floor (table below), in aggregate
                                #   OPERATION arrivals — a seeded schedule, never
  warmup: PT5M                  #   closed-loop users, so coordinated omission
  duration: PT1H                #   cannot hide stalls
  journeys:                     # shares of JOURNEY instances (the catalogue,
    chart_review: 56%           # vocab/journey_catalogue.yaml, expands each into
    ward_dashboard: 12%         # its ordered, time-offset operation stages);
    vitals_round: 6%            # the validator recomputes the expansion — the
    # … (the committed cases    # expanded write share must sit inside the
    # carry 18 journeys)        # derived 10:1..50:1 read-heavy band (below)
thresholds:                     # ALL must hold in the single measured run
  - { metric: latency_p99, max: 1s }   # operation-less = EVERY measured operation
  - { metric: error_rate, max: 0 }
  - { metric: offered_load_sustained, min: 15/s }   # class-S floor (table below)
# environment: bound to the mandatory ixit.json environment block (§8.10)
```

**Provisional class parameters** (proposed defaults pending SEC
ratification; derivation shown so the numbers are arguable, not arbitrary):

| Class | Population served | Corpus | Offered-load floor (peak API arrivals, sustained) | p99 API budget | Error rate |
|---|---|---|---|---|---|
| POC | demo | 10k EHRs | 2/s (demonstration floor, not population-derived) | ≤ 1 s | 0 |
| S | 100k | 100k EHRs | **15/s** (derived band 13–75/s) | ≤ 1 s | 0 |
| L | 1M | 1M EHRs | **150/s** (band 130–750/s) | ≤ 1 s | 0 |
| R | 10M | 10M EHRs | **1,500/s** (band 1,300–7,500/s) | ≤ 1 s | 0 |

Derivation — the **population-anchored utilization model** (**[legislated]**
composite of cited statistics, each input graded; SEC ratifies or amends;
replaces the 2017 concurrent-users guess, following the TPC-C/SAP-SD
precedent of anchoring load to a countable base unit with a per-unit rate):

1. **Clinical documents per capita per year ≈ 46**, from published activity
   rates × documents per event: doctor consultations 6/capita (OECD Health
   at a Glance 2023; Eurostat range 4.4–10.1) × 1 doc; inpatient discharges
   0.128/capita (OECD/Eurostat) × ~10 docs/stay; ED visits 0.3/capita
   (OECD WP 83; 40-country median) × ~4 docs; laboratory results 15/capita
   (RCPath — flagged low-confidence); imaging 0.82/capita (NHS Diagnostic
   Imaging Dataset 2023/24) × 1; prescription items 21.8/capita (NHS BSA
   PCA 2024/25, accredited statistic) × 1. Sanity: between Denmark/Estonia
   major-document exchange rates (~8–10/capita/yr) and Finland Kanta's
   all-inclusive ~130/capita/yr (>2M documents stored/day).
2. **Average writes/s** = population × 46 ÷ year-seconds → 0.15/s per 100k.
3. **Peak factor ×8** (band 6–10): weekday concentration ×1.4 · busy-hour
   ≈17% of daily traffic ×~4 (the Cisco/ITU-T E.500 busy-hour convention) ·
   intra-hour burstiness ×1.2–1.5; healthcare corroboration: ED arrival
   rates vary >3× over 24 h, ~88% of arrivals in the 16 non-overnight
   hours.
4. **Read multiplier 10:1** (the 90/10 read-heavy OLTP convention,
   OLTP-Bench/YCSB) as the floor's mix — with audit-log evidence (~597 EHR
   interactions per encounter, PMC10148376) bounding a read-heavy upper
   band at ~50:1, which gives each class its band ceiling.
5. **Result: ≈13 peak API arrivals/s per 100k population** (band 13–75);
   class floors rounded to 15/150/1,500 per class population. Envelope
   checks against real systems: NHS Spine peaks at 3,500 coarse
   messages/s for ~60M people; the Catalonia 13M-patient openEHR CDR
   (117M compositions) published the lesson that **per-EHR data volume
   dominates query cost** — honoured by the corpus ladder's per-patient
   volume assumption (~100 composition versions/EHR, §11.11).

The latency budget is the **standard per-operation SLO of p99 ≤ 1 s,
uniform across classes** — the same SLO the committed knee-ladder
methodology already defines sustainability by (`docs/benchmarks/*/KNEE.md`:
"SLO p99 ≤ 1 s, error ≤ 0.1%"). The 2017 page's user counts and screen
latencies are recorded as lineage only. Procurers who need tighter tails or
the read-heavy band ceiling tighten per tender via the §10 template
parameters. Corpus sizes are the 2017 D-row scale ladder. Feasibility is
evidenced by the committed measurement artifacts (`docs/benchmarks/`,
regenerated per release, never hand-typed): FerroEHR sustains a
631.5 req/s knee at p99 204.7 ms and upstream EHRbase 475.0 req/s
(`ferroehr/KNEE.md`, `ehrbase/KNEE.md`) on 8-core consumer hardware —
the class-L floor (150/s) is comfortably attainable — a consumer laptop
already sustains 4× it — and the class-R floor (1,500/s) is a
server/scaled-deployment target bracketed by NHS Spine's published peak,
consistent with its "Region" intent.

Performance case fields (∎ beyond the §8.3 commons — `id`, `kind`,
`component`, `description`, `test_purpose`, `spec_refs`): `class` ∎,
`corpus` ∎, `workload` ∎ (`arrival_rate` ∎, `warmup` ∎, `duration` ∎,
`mix` ∎), `thresholds` ∎. The **knee-finding ladder is not a case** — it is
the exploratory procedure (donated methodology) an implementer uses to pick
which class to claim; the class case is the single fixed-offered-load
sustained run that earns it. The measurement record in `results.json`
carries, per case × operation: request count, error count, p50/p90/p99
latencies, and the full HDR histogram (encoded) — so thresholds are
re-checkable from the artifact. Performance `spec_refs` cite the proposal
lineage and are exempt from vendored-spec link-checking (the admitted scope
extension, §6.3).

Rules:

- **Classes are earned, not declared**: a class rating requires every
  threshold of that class's case(s) to hold in a single measured run;
  results land in `results.json` as measurements + a per-class
  `earned | not-earned` verdict. The 2017 ladder supplies the shape (POC ~5
  users; S ~100 users/100k EHRs; L ~1000 users/1M; R ~10k users/10M); the
  concrete threshold numbers carry the provisional defaults above (the 2017
  page's "XX" rates made concrete, derivation shown) — SEC ratifies or
  amends them with the chapter (§11.4).
- **Environment-bound**: performance is meaningless without the deployment
  described — the `ixit.json` environment block (hardware class, cores,
  memory, storage class, topology) is mandatory for performance runs, and
  every earned class is reported *with* its environment. This answers the
  reason the current Guide excluded performance, without excluding it.
- **Statement + certificate**: the statement claims a target class per
  environment; the certificate reports the earned class alongside the
  functional profile — the 2017 multi-dimensional certificate
  (Functional | Performance | Security per §8.15, with Enterprise following §11.11).
- **Reference methodology**: seeded workload generators + the knee-finding
  and sustained-run procedure of a published benchmark harness (this repo's
  `tools/benchmark` is the donated working draft); any runner reproducing
  the workload definition and emitting the measurement schema qualifies —
  harness independence holds here too.

**The journey decomposition (2026-07-22, the hospital simulation — #240).**
The measured workload evolved from the flat four-operation mix into
journey-structured arrivals: the performance case's `workload` block names
shares of *clinical journeys* (a committed catalogue,
`vocab/journey_catalogue.yaml` — ADT admission/discharge, vitals rounds,
the eMAR loop, medicines reconciliation, asynchronous laboratory/imaging
order-to-result pipelines, specialist/registry reporting, public-health
notification, chart review, ward dashboards with a registered stored
query, versioned corrections, contribution audit review, ITEM_TAG
workflow tagging, logical deletion, and the integration-engine template
poll), each journey an ordered, time-offset operation sequence over a
closed 22-operation vocabulary with fixed ITS-REST wire realizations,
payloads carried by COMPOSITION-rooted openEHR CKM templates vendored
with provenance. The population anchor is UNCHANGED and machine-enforced:
`arrival_rate` stays aggregate operation arrivals (this section's floors'
unit); every journey's `derivation` cites the same activity-statistics
register as the floors (documents per discharge, prescription items per
capita, laboratory reports per capita, the audit-log ~597
interactions/encounter read evidence); and the runner's `journey-envelope`
validator gate recomputes the catalogue expansion of every workload,
requiring the expanded write share inside the 10:1..50:1 read-heavy band
of step 4 — the decomposition is arguable, never arbitrary. Every stage is
its own planned arrival on the same open-loop schedule (dependent stages
never block; an unlanded prerequisite records as an error), and the
extended 8/12-hour holds may follow a diurnal day curve realizing step 3's
ITU-T E.500 busy-hour convention (the floor is then the busy-hour rate).
The certificate reports workload coverage: exercised capabilities vs the
ICS claims, with untouched claimed capabilities listed as catalogue gaps.

### 8.15 The Security & Privacy schedule

Security & Privacy is the fourth certificate rating of the 2017 schedule
(Functional | Enterprise | Performance | Security),
realized as an **assertion-machinery capability family** (§8's architecture:
these are testable functional behaviours — this is explicitly NOT a security
evaluation scheme in the Common Criteria sense, §11.9). The family's first
level is **SEC-BASIC** (the 2017 schedule's "BASIC" rung); higher levels are
future SEC work. SEC-BASIC's conformance points, each an ordinary §8.3 case
family:

| Point | Conformance behaviour | Anchor |
|---|---|---|
| **EHR/demographic separation** | No demographic identifying content is reachable through EHR-side endpoints: `EHR_STATUS.subject` carries only opaque external refs / PARTY_SELF; EHR queries cannot join demographic records without demographic-service authorization. | 2017 schedule Security BASIC (lineage); RM ehr §EHR_STATUS.subject |
| **Authenticated access enforced** | Every platform route rejects unauthenticated requests (`401`) — a negative sweep over the route table. | ITS-REST overview §HTTP status codes |
| **Authorization separation** | Admin-family operations reject non-administrative principals (`403`); read-only principals cannot commit. | ITS-REST overview §HTTP status codes |
| **Audit accountability** | Every change-controlled commit carries `commit_audit` with committer identity and **server-set** `time_committed` (client value ignored — the §8.4 commit-metadata rule as a security assertion); audit-event emission on writes where an audit log is supported (IHE ATNA-shaped). | RM common §change_control; ITS-REST overview §openehr-audit-details |
| **Anonymous EHRs** | An EHR is creatable and fully operable with no demographic identity attached. | Profiles book §Non-Functional (existing CORE capability) |
| **Version-signature integrity** *(applies when the Signing capability is claimed — Profiles: STANDARD, not SEC-BASIC-required)* | Committed VERSIONs carry a verifiable `signature`: produced over the canonical form of the version data, verifiable against the declared key material, with the version lineage (`preceding_version_uid` chain) intact — the digital signing chain. Verification behaviour is conformance; algorithm strength and key management are not (§6.3). | RM common §change_control (`ORIGINAL_VERSION.signature`); Profiles book §Non-Functional (Signing) |

Signing thereby gets its concrete conformance point (this repo's SIG case
area + PGP signing sidecar are the donated seed). The
**statement-declared posture** (never wire verdicts): transport/at-rest
encryption configuration and id-pseudonymisation-on-export — the 2017
D-row's "encryption? id pseudonymisation?" aspects — declared in the
statement's `non_functional` security slots and revisited when the
Enterprise-D chapter (§11.11) makes export regression testable.

**Certificate rating**: `Security: SEC-BASIC` is earned when every SEC-BASIC
capability case passes (assertion machinery, per tech profile) — computed
like every other cell, never declared.

**Capability-matrix entries** (§8.2 family 3 — entry shape, stated here once
for the whole matrix): `capability → { family: Platform | Enterprise |
Security, tier: <family-scoped>, required: bool }`, where tiers are scoped
per family — Platform: CORE/STANDARD/OPTIONS; Security: SEC-BASIC (…);
Enterprise: D/M/X. The SEC-BASIC points above enter as
`family: Security, tier: SEC-BASIC, required: true` — except
version-signature integrity, which enters under the existing Signing
capability (`family: Platform, tier: STANDARD`) per its own row.

## 9. Certification governance — the ladder as a conformity-assessment scheme

**Scheme owner: openEHR International** (the CIC that operationally runs the
specification program — the body the Conformance Guide already names as the
Platform Specifier). The program is a **conformity-assessment scheme** in the
ISO/IEC 17000 sense. Its self-declaration rungs are governed by
**ISO/IEC 17050** — not by 17067, which is by definition third-party product
certification; only the top rung is an ISO/IEC 17067 scheme: **Type 1a**
initially (type testing of a specific product version, no surveillance),
maturing to **Type 5** (type testing + process/QMS assessment + ongoing
surveillance of both) if surveillance is funded. Rungs are labelled by
attestation level so no rung can masquerade as a higher one:

| Rung | Name | ISO frame | Mechanism | Who grants |
|---|---|---|---|---|
| 0 | **Published statement** | First-party attestation, registered | Vendor publishes `statement.json` + `results.json`. **Listing preconditions**: the results come from a runner that has passed the §8.12 verification pack, and the statement passes static conformance review. Registry rows display runner identity + verification status and are visually labelled **self-published**. | Nobody — registration only |
| 1 | **Self-declared (signed SDoC)** | First-party attestation with signed SDoC (ISO/IEC 17050-1/-2) — "self-certification" in industry usage (OpenID); ISO reserves "certification" for third-party attestation, which is rung 3 alone | Rung 0 + a signed legal attestation of result accuracy by an authorized officer (+ modest fee funding the program). The §6.4 responsibility sentence appears on the certificate. | openEHR International (administrative + static review only) |
| 2 | **Community-verified** | Witnessed peer verification (genuinely second-party only when the witness is a procurer/user of the product) | Results reproduced at a supervised conformance-thon (EHRCON slot) or by a named community witness re-running the suite from the vendor's `ixit.json` against a vendor-provided deployment. Witness identity on the registry row. | Event organizers / named witnesses |
| 3 | **Certified** | Third-party attestation → certification | An **ISO/IEC 17025**-accredited lab runs the suite; an **ISO/IEC 17065**-accredited certification body reviews and certifies, with surveillance obligations. Both roles **delegated to independent accredited bodies** (the IHE/ONC model) — openEHR International remains scheme owner only, because a spec author certifying its own ecosystem fails 17065 impartiality. **This rung is not offered until surveillance is funded**; advertising it earlier would be dishonest. | Accredited certification bodies |

Cross-cutting rules:

- **Certificate ratings are the machinery × family matrix** (the 2017
  multi-dimensional certificate, realized cleanly): assertion-machinery
  ratings per capability family — Platform (CORE/STANDARD/OPTIONS), and
  Enterprise (D/M/X, §11.11) as its chapter lands and Security (SEC-BASIC,
  §8.15) — each per tech profile, plus the measurement-machinery rating
  (earned performance class per environment, §8.14). Every cell is computed from `results.json` +
  the capability matrix, never hand-asserted.
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
  **The badge is a licensed ordinary trademark, not a registered
  certification mark**: an EU certification mark legally asserts that the
  proprietor *certifies* (EUTMR (EU) 2017/1001 Art 83) — incompatible with
  self-declared rungs — and its owner may not carry on business involving
  the certified goods (Art 83(2)). The OpenID model applies instead:
  revocable, royalty-free trademark licence with prescribed per-rung usage
  statements, goodwill to openEHR International, mandatory removal on
  supersession or withdrawal; wording implying certification is licensed at
  rung 3 only. A rung-0/1 badge signifies a self-declaration *registered by*
  openEHR International, never certification *by* it.
- **Registry Terms of Use** (binding every submitter; drafting precedent:
  the OpenID Certification Terms & Conditions): the registry publishes
  **"as is," without warranty**; openEHR International has **no obligation
  to validate** any claim and may reject or remove entries; the submitter
  **represents and warrants** accuracy and must promptly update or withdraw
  on material change; the submitter **indemnifies** openEHR International
  and liability is capped; entries are removed or labelled
  **Withdrawn / Superseded / Disputed** (the takedown mechanic the dispute
  path feeds); the badge licence terminates with the listing. Privacy: the
  17050-1 signatory name/role is personal data — openEHR International acts
  as controller under a registry privacy notice (legitimate
  interest/contract; retention tied to statement currency).
- **Access**: schedule, schemas, corpus, and runners are public and free
  (Inferno/OpenID lesson: adoption dies behind paywalls). Rungs 1–3 may carry
  fees; the 2021 members-only idea applies to *services* (attestation
  processing, events, assessor program), never to the artifacts.

## 10. The procurement pack — usable within 12 months

The deliverable a tendering authority can use the moment rung 0 exists:

- **A normative RFP requirement template** (new short section of the Guide,
  answering the framework's "RFI/RFP guides: future" TODO):

  > *The offered product must demonstrate openEHR conformance to [CNF
  > schedule release ≥ R, profile ≥ STANDARD, technology profile including
  > canonical JSON], evidenced by a published openEHR Conformance Statement
  > at registry rung ≥ 1 for the product version offered, **or by equivalent
  > means of proof** (including a manufacturer's technical dossier or an
  > equivalent conformance report) demonstrating conformity to the same test
  > cases. The awarding authority will accept any evidence that objectively
  > establishes equivalent conformance, and reserves the right to require a
  > witnessed re-run (rung 2) of the published or submitted results prior to
  > acceptance.*

  Tender authors fill four parameters (release, profile, tech profile, rung).
  **The equivalence clause is not optional**: Directive 2014/24/EU Arts 42–44
  oblige contracting authorities to accept equivalent labels and equivalent
  means of proof (and a technical dossier where the operator demonstrably
  could not obtain the label in time) — a template naming one scheme's
  certificate exclusively is challengeable as discriminatory. The scheme's
  openness (public artifacts, open governance) satisfies the Art 43(1)
  label conditions; the equivalence duty applies regardless. This replaces
  the Catalonia-style behavioural-SLA workaround with a lawful,
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
once §8.3 makes cases enumerable files:

1. **Querying / AQL (master11 + master05)** — the flagship gap. The
   result-set equivalence rules are **written, normative, and cited in §8.6**
   (spec-grounded rules marked [spec]; the four points the specs are silent
   on carried as [legislated] proposed defaults) — U5's gate is SEC
   ratification of the [legislated] points, not de-novo design. Seed
   material: this repo's 25 QRY + 8 SQR + 4 AQT case
   designs (each carrying AQL 1.1 citations) and EHRbase's AQL conformance
   corpus ([ehrbase/conformance-testing-documentation](https://github.com/ehrbase/conformance-testing-documentation),
   SELECT/WHERE/ORDER BY/LIMIT/FROM/parameter suites).
2. **The maximal-coverage template round-trip** (the 2017 "template
   injection test"): one template exercising ALL RM types (every DV_* incl.
   generic derivations like DV_INTERVAL<DV_QUANTITY>) and all compositional
   hierarchy shapes, driven end-to-end — inject OPT → commit instance →
   export canonical JSON+XML → regression-compare. Pairs with master04's
   "maximal valid OPT" data set; one case family, enormous coverage per case.
3. **Scenario/lifecycle suites** (the 2017 "EHR API lifecycle test"):
   realistic multi-contribution journeys — admission (admin COMPOSITION) →
   persistent medication list → event vital signs → update both → retrieve
   all versions in both formats — encoded as ordinary §8.3 flows; these
   catch cross-operation state defects the per-operation cases cannot.
4. **The performance & volumetrics chapter** (§8.14): normative workload
   definitions + the provisional class parameters now tabled in §8.14
   (derivation shown; SEC ratifies or amends), the synthesized scale corpora
   shared with §11.11, and the measurement schema. Ships after the functional pilot
   proves the artifact discipline; the schedule extension of the Guide's
   scope is flagged for SEC in §6.3.
5. **Content chapters refresh** — raise the RM floor statement (1.0.2 → an
   applicability ladder), fill 17.5 or formally adjudicate it out, fix the
   master14 numbering gap and the master13 duplicate heading.
6. **Demographic (master10)** — schedule cases exist in no form today; ECC's
   31 DEM cases + the ITS-REST Demographic API (DEVELOPMENT lifecycle) are the
   seed; profile placement stays OPTIONS.
7. **Admin (master12) + Messaging (master13)** — decide what is
   *wire-testable* (platform API) vs inherently off-wire (dump/load,
   archives); off-wire capabilities move to statement-declared, not
   schedule-tested — the honest boundary.
8. **N/A re-adjudication of donated material (hard gate)** — every donated
   case whose evidence or N/A justification points at FerroEHR internal
   tests is re-adjudicated to spec-text-only evidence **before** entering the
   normative catalogue. No exceptions; this is a scoped workstream, not an
   assumption.
9. **Security & privacy conformance points** — the Certificate book
   advertises BASIC-SEC/BASIC-PRIV with no defining cases; only Signing +
   Anonymous EHRs exist in the Profiles book. **The SEC-BASIC level is now
   defined in §8.15** (EHR/demographic separation, authenticated access,
   authorization separation, audit accountability, anonymous EHRs) — this
   roadmap item authors its cases. Explicitly scoped small; not a security
   evaluation scheme.
10. **ADL2 cases (master04)** — OPTIONS-profile depth for the `am24`
   generation.
11. **The Enterprise capability family** (the 2017 schedule's D/M/X
   dimension, absent from every later draft): **D — data portability**
   (full-EHR dump/load in canonical form **between independent instances** —
   single-instance export/archive already exists as Admin capabilities
   (master12 `I_ADMIN_DUMP_LOAD`/`I_ADMIN_ARCHIVE`; Profiles "EHR
   Dump/Load"); the cross-instance portability regression is the new part —
   verified by lossless regression over a random query set, on synthesized
   corpora at declared scales — 1k/10k/100k/1M/10M EHRs, ~100 composition
   versions each, the recipes joining the §8.8 governed corpus);
   **M — EHR management** (merge/split/move of EHRs across instances);
   **X — cross-enterprise synchronisation** (asynchronous update merging —
   specifications-CNF issue #1 is the 2017 seed). Architecturally supported
   already: ixit declares named instances and flow steps carry `on:`
   selectors (§8.3, §8.10), so dual-instance cases are ordinary cases. Its
   own capability family in the matrix + an SM grounding decision; dump/load
   overlaps §11.7's off-wire boundary.
12. **The openEHR→EEHRxF seam (EHDS alignment, later)** — cases verifying that
   priority-category content in a conformant CDR renders faithfully to the
   EEHRxF FHIR models, once the March 2027 implementing acts fix them. Flag:
   this extends conformance scope beyond the platform API; it needs its own
   profile family and SEC decision.

## 12. Governance & resourcing — the section that answers the post-mortem

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
  cut like spec releases (versioned, changelogged); CI (§8.13) makes
  community PRs safe to accept, which is what actually de-bottlenecks a
  volunteer group.
- **IP**: donated cases/schemas/corpus items enter under the spec repo's
  licence with contributor licence hygiene (no retained vendor copyright in
  normative artifacts, no patent encumbrance).
- **Funding**: a recurring program line (registry hosting, CI, maintainer
  coordination, event slots) funded from openEHR International's program
  budget + rung-1 attestation fees — explicitly *not* from one vendor's
  project budget, because §4 stall-cause 2 is how that ends. Gap-fill chapters can be
  vendor-sponsored (bounded, reviewable tasks), but the *program* must not
  be — and a sponsoring vendor is never the sole adjudicator of its own
  sponsored cases: sponsored work is scoped to case authorship reviewed
  against spec text by non-sponsor maintainers.
- **Commitments in hand**: FerroEHR commits the pilot engineering (§14).
  The upstream ask explicitly requests matching co-commitments — a second
  vendor's engineering time and 2–3 maintainer volunteers — before the SEC
  agenda item, so the SEC decides on a resourced plan, not a hope.
- **Impartiality by structure**: openEHR International is scheme owner and
  registrar only. It never tests, never certifies (rung 3 is delegated to
  accredited bodies; rung 1 is administrative). A spec author grading its own
  ecosystem is the 17065 impartiality failure the IHE/ONC split exists to
  avoid.

## 13. Upstream path

1. **Discourse** (Conformance category): the proposal condensed for
   discussion, collecting the §12 co-commitments before any SEC agenda item.
2. **SPECCNF-1/6 comment + a specifications-CNF issue** carrying the §8
   artifact set and the eight encoded pilots.
3. **SEC agenda item**: adopt-the-format decision, the maintainer-group
   charter, the AQL chapter blessed as the pilot.
4. **Execution**: the §14.1 PR series; the registry the moment two products
   publish (FerroEHR volunteers; upstream EHRbase, already assessed by ECC,
   is the natural second); an EHRCON26 conformance slot; EHDS liaison per
   §6.5 (track the Art 36/15 implementing acts; revisit the EEHRxF-seam
   profile when they land in 2027).

**Outbound spec-defect reports (the ambiguity register → openEHR).** Building
and running the catalogue against the real spec surfaces where the spec itself
is silent, self-contradictory, or misaligned across SM / ITS / CNF. Every such
finding is a register entry with an `upstream_issue` (§8.5), each written as
a concrete openEHR report on its `upstream-report`-labeled GitHub issue (a
plain summary, what the released spec says, what this implementation does,
the resolution sought) for a maintainer to file, recording the returned
channel key on the issue. This is a first-class deliverable of the
framework, not a side effect: an
instrument that exercises the whole spec is exactly the instrument that finds
the spec's own defects, and reporting them back is how the standard improves.

Success measures: SEC adopts the schedule + charter; ≥2 independent runners
pass the §8.12 verification pack; the AQL chapter ships with normative
equivalence rules; ≥3 products on the public registry; CNF Release 1.0.0
finally cut — before the March 2027 EHDS implementing acts.

## 14. Production implementation plan

Two tracks, both production-grade from day one — no throwaway prototype. The
in-repo track does not wait for upstream adoption: the reference runner
implements the §8 artifact set as its production format immediately, which is
simultaneously the proof the upstream proposal ships with.

### 14.1 Upstream: the specifications-CNF PR series

Sequenced, each PR independently reviewable and CI-green, each with an
acceptance gate:

| PR | Content | Acceptance gate |
|---|---|---|
| U1 | The five schedule-artifact schema families (§8.2 #1–5: case cores, bindings, vocabularies incl. the capability matrix, corpus manifest, ambiguity register — a living artifact, each carried divergence linked via `upstream_issue` to its outbound report) + the §8.13 CI workflow | Schemas validate the §8.9 pilot files; CI runs on the repo |
| U2 | master06 (EHR) converted: all 21 cases as case cores + the its-rest bindings for the EHR operations + corpus manifest over the existing EHR fixtures | Generated prose semantically equivalent to the current chapter (human-reviewed diff); zero information loss against the AsciiDoc tables |
| U3 | master07/08/09 (COMPOSITION/CONTRIBUTION/DIRECTORY) conversion + bindings | Same gate; the versioning cases (§8.9 pilot 4 shape) round-trip |
| U4 | Content chapters (master15–17) conversion — decision tables as data + the literal grammar + generation recipes | Every existing table row preserved verbatim; grammar parses 100% of existing literals |
| U5 | **master11/AQL — the first new chapter**: the §8.6 equivalence rules as normative schema text + ~37 cases seeded from ECC QRY/SQR/AQT (25 QRY + 8 SQR + 4 AQT) + the EHRbase AQL corpus | SEC ratifies the §8.6 [legislated] defaults FIRST; every case spec-cited to AQL 1.1 |
| U6 | **Simplified-Formats chapter** (new): the §8.7 fifteen categories, ~60 cases driven from the master04/05/06 spec-example blocks | Every case cites its simplified_formats section; OPTIONS-profile placement |
| U7 | statement/results/ixit schemas + verdict rules + the reference verdict implementation + the runner verification pack (transcripts + adjudications) | Two independent runners (ECC + the rescued Robot suite or another vendor's) compute identical verdicts on the pack |
| U8 | The registry (production, on openehr.org): statement rendering, attestation-level labels, badges, dispute log | First two products listed (FerroEHR + upstream EHRbase baselines) |

The performance & volumetrics chapter (§8.14 + §11.4), Demographic
(master10), and Admin/Messaging (master12/13) follow as U9+ per the §11
roadmap once the pattern is proven on U2–U6.

### 14.2 This codebase: the reference runner, built from scratch

Owner ruling: the conformance + benchmark tooling is **rebuilt from the
ground up** as one runner implementing the §8 architecture natively — not an
incremental adaptation of today's ECC, and **not a 1:1 transcription of its
catalogue**: the new catalogue is authored from the CNF 2.0 framework itself
(the official schedule cases per the §8.9 pilots, the new chapters, the
framework's own selection/format/option machinery). The current ECC and its
committed baseline (402 case×format executions · 384 passed · 18 N/A) remain
running untouched as the **comparison reference**: differences are expected
and are enumerated + justified in a comparison report (the W2 gate); the old
harness retires on a reviewed report, and the new runner establishes the
new baseline. Tracked as dedicated issues (opened when this design is
owner-approved), sequenced:

| WS | Workstream | Content | Done-gate |
|---|---|---|---|
| W1 | **Artifact schemas in Rust** | `tools/conformance`: typed model + validator for case cores, bindings, vocabularies (outcomes + the capability matrix), corpus manifest, ambiguity register; JSON-Schema emission so the same schemas ship upstream in U1. The §8.13 checks become `cargo nextest` guards alongside the existing coverage guard. Scope: assertion-machinery artifacts; the performance case-core schema lands with W7. | Validator rejects every seeded-defect artifact fixture; schemas byte-identical to the U1 set |
| W2 | **Catalogue authoring + comparison** | The CNF 2.0 catalogue authored per the framework: official schedule cases first (the §8.9 pilot encodings generalized across master04–09/15–17), then the ECC-original designs that fill genuine gaps — each re-adjudicated to spec-text-only evidence (§11.8) before entry, keeping an `ecc-` namespace pending upstream adoption. Official CNF ids primary; `inventory/ecc-catalog.tsv` retires with the old harness. | **Comparison gate** (not reproduction): committed ECC↔CNF coverage map + comparison report — every difference from the old baseline enumerated and justified against the framework, verdict regressions on equivalent coverage explained, official-schedule coverage ≥ the old harness; cutover, old-harness retirement, and the new baseline follow a reviewed report |
| W3 | **Data-driven executor** | The engine executes functional case cores directly from the artifact files (flow interpreter: requires-setup, parameter iteration with reset_per_row, captures, outcome mapping via bindings, typed assertions). Hand-written Rust remains only for generation recipes and genuinely non-mechanizable glue — each such exception is registered. Content decision tables execute from the data. The AQL `result_set` assertions execute under the §8.6 equivalence rules (U5 ratifies the [legislated] defaults). | ≥90% of cases run through the interpreter; every exception listed in the report; the W2 comparison report stays green |
| W4 | **Statement / results / ixit emission** | `results.json` is emitted in the §8.10 schema (per-row outcomes, ambiguity dispositions, runner verification status); `statement.json` (ICS) + `ixit.json` (formalizing `SutDescriptor`) emitted per SUT; the Certificate/Statement/Comparison artifacts render from them; verdict computation moves to the shared pure function. | All `docs/conformance/**` artifacts regenerate from the new schemas; the honesty blocks survive; badges derive from the new results |
| W5 | **Simplified-formats deepening** | The §8.7 blueprint's gap categories 2–9 (node-id algorithm, level removal, the 43 suffix tables, `_`-attributes, `\|raw`, full ctx vocabulary, counters, STRUCTURED style) + deepened 1/10 — ~40 new SF cases, all spec-example-driven, all OPTIONS-profile. | Every master04/05/06 spec-example JSON block exercised; ECC baseline ratchets upward only |
| W6 | **Runner verification pack** | Author the U7 transcripts + adjudications; ECC self-verifies against them in CI; publish the pack so the Robot suite (and any vendor runner) can prove itself. | ECC passes both pack parts; the pack rejects a deliberately-broken runner build |
| W7 | **Performance schedule implementation** | `tools/benchmark`'s workload generation, knee-finding ladder, and sustained-run procedure re-expressed as §8.14 performance cases + the measurement schema; class verdicts computed into results.json; environment block formalized in ixit.json. | An earned-class run against both SUTs committed; verdicts reproduce the published benchmark artifacts |

Sequencing: W1 → W2 → {W3, W4} → {W5, W6, W7}. Standing gates apply throughout:
`cargo clippy --workspace --all-targets --all-features`, full nextest, the
ECC zero-drift rule (the baseline only ratchets upward), and the
changelog/docs-website rules for any user-visible surface.

### 14.3 Runner technology — why the reference runner is Rust

No harness is normative (§8.7 discipline: any technology that passes the
§8.12 verification pack is a compliant runner — the Robot suite, Spock,
Postman, anything). The **reference** runner, however, is a deliberate
technology choice, and it is Rust:

- **There is no Robot-equivalent to inherit.** The Rust ecosystem has strong
  building blocks — [goose](https://github.com/tag1consulting/goose)
  (Locust-class load-testing framework),
  [cucumber-rs](https://github.com/cucumber-rs/cucumber) (native BDD
  runner), [Hurl](https://hurl.dev) (declarative plain-text HTTP testing) —
  but no keyword-driven acceptance framework like Robot. CNF 2.0 makes one
  unnecessary: the machine-readable schedule **is** the keyword layer, so
  the runner is a data-driven interpreter over the artifact files (W3) —
  precisely the thing worth building once, well.
- **Type safety enforces the framework's own laws.** The closed
  vocabularies this design lives by — outcome kinds, body/header selectors,
  reference grammar, capability matrix — map to Rust enums and newtypes:
  an outcome kind outside the taxonomy or a malformed `${…}` reference is a
  *compile-time* error in the reference implementation, not a runtime
  surprise. The verdict machineries are pure functions over typed data —
  the property §8.11 demands ("any two conformant implementations MUST
  compute identical verdicts") is easiest to guarantee in a language where
  the types make illegal states unrepresentable.
- **One language serves both machineries.** The measurement machinery
  (§8.14) needs a workload generator that is never itself the bottleneck at
  class-R concurrency; Rust's async runtime handles goose-class load
  generation, and this repo's `tools/benchmark` (knee ladder + sustained
  runs, already published against two CDRs) is the donated draft. Assertion
  interpreter + load generator + verdict computation share one toolchain.
- **A single static binary ends the environment rot that killed the last
  suite.** The Robot suite's practical death was environmental
  (specifications-CNF PR #5, unmerged since 2023: Python dependencies,
  hard-coded vendor images). A vendor or procurer runs the reference runner
  as one downloaded binary against an `ixit.json` — no interpreter, no
  package manager, no environment to rot.
- **Memory safety + fearless concurrency** matter for hour-long sustained
  performance runs and parallel case execution against live clinical
  systems.

**The selected stack** (the ground-up rebuild's dependency decisions —
following this workspace's discipline: pinned workspace crates, never
hand-roll what a vetted crate provides, verify versions live at adoption):

| Concern | Selection | Rationale |
|---|---|---|
| Async runtime / HTTP | `tokio` + `reqwest` (rustls) | The runner is an interpreter + HTTP driver; both machineries share the client; already pinned + vetted in this workspace |
| Artifact model | `serde` + hand-written typed model (enums/newtypes for every closed vocabulary) | The §8 laws become compile-time properties; no framework needed or wanted |
| Canonical interchange | `serde_json` | statement/results/ixit are JSON; hash-linked artifacts |
| Schema validation | `jsonschema` | Validates the artifact families + emitted party artifacts against the published schemas |
| YAML authoring front-end | **`serde-saphyr`** (1.0.0-rc.1, 2026-07-18) | The only actively-maintained pure-Rust (`unsafe`-forbidden) serde YAML 1.2 front-end with **no RUSTSEC advisories** — `serde_yaml` is archived and both libyaml-fork successors inherit RUSTSEC-2023-0075; deserializes to `serde_json::Value` (the exact shape `jsonschema` validates) and ships a `Budget` cap on nesting/alias expansion, closing billion-laughs on untrusted test files. Pin the rc or its stable line. |
| Workload engine (measurement machinery) | **own tokio-native engine** — the knee-ladder + sustained-run code of `tools/benchmark`, carried into the rebuild | Already built, published against two CDRs, and methodology-specific; [goose](https://github.com/tag1consulting/goose) evaluated and named as fallback — its Locust user-behaviour model does not fit the ladder |
| Latency statistics | **`hdrhistogram`** (7.6.0, 2026-07-18) | The canonical Rust HdrHistogram port, no RUSTSEC advisories; the `serialization` feature emits the standard V2/compressed encoding so §8.14 thresholds are re-checkable from the results artifact, and `record_correct` provides the coordinated-omission correction the open-loop model requires. |
| CLI / errors / telemetry | `clap` / `thiserror` / `tracing` | Workspace standards |
| Test-definition DSLs | **[cucumber-rs](https://github.com/cucumber-rs/cucumber) and [Hurl](https://hurl.dev): evaluated and declined** | Each would introduce a second test-definition language beside the schedule — the exact three-representations drift CNF 2.0 exists to kill; the schedule is the only DSL |

ECC (`tools/conformance` + `tools/benchmark`) is the prior art and the
comparison reference for the rebuild (§14.2); W1–W7 deliver the new runner as
the first production implementation of the artifact set. The Robot suite
remains a first-class *compliant* runner via the verification pack —
rescuing it is inside the proposal (§8.7, §13); it is simply no longer the
thing the framework's credibility depends on.

**The comparison gate, operationally** (the W2 done-gate): (a) a committed
ECC↔CNF **coverage map** relating old cases to the ground the new catalogue
covers; (b) a **comparison report** enumerating every difference from the
old 402-execution baseline — added/dropped/reshaped cases, execution-set
changes under the §8.7 format model, N/A rationale changes under the new
guard/option machinery — each justified against the framework, reviewed
line-by-line; (c) any verdict **regression on genuinely-equivalent
coverage** is explained (a real finding vs a deliberate case change — never
silently absorbed); (d) official-schedule coverage is ≥ the old harness's.
The framework changes what is tested and how it is counted, so the numbers
WILL differ — honesty lives in the enumeration, not in reproduction.
Cutover, old-harness retirement, and the establishment of the new baseline
(which then ratchets) follow a reviewed report.

What this buys strategically: when U1 reaches the SEC, the schemas arrive
with a production runner already storing, validating, executing, and
reporting through them against two real CDRs — the difference between
proposing a format and demonstrating one.

---

## Appendix — source register

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
  36–41, 49, 105; Annexes II–IV); verbatim text retrieved via the
  Publications Office machine channel
  <http://publications.europa.eu/resource/celex/32025R0327> (CELEX
  32025R0327; Arts 105/40/39 quoted in §6.5).
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

**Performance-model sources (§8.14 derivation):**
- OECD Health at a Glance 2023, consultations —
  <https://www.oecd.org/en/publications/2023/11/health-at-a-glance-2023_e04f8239/full-report/consultations-with-doctors_159193ce.html>;
  Eurostat healthcare activities (consultations; hospital discharges) —
  <https://ec.europa.eu/eurostat/statistics-explained/index.php?title=Healthcare_activities_statistics_-_consultations>,
  <https://ec.europa.eu/eurostat/statistics-explained/index.php?title=Hospital_discharges_and_length_of_stay_statistics>.
- NHS Diagnostic Imaging Dataset 2023/24 (47.2M exams) —
  <https://www.england.nhs.uk/statistics/wp-content/uploads/sites/2/2024/11/DID-Annual-Statistical-Release-2023-24.pdf>;
  NHS BSA Prescription Cost Analysis 2024/25 (1.26B items) —
  <https://www.nhsbsa.nhs.uk/statistical-collections/prescription-cost-analysis-england/prescription-cost-analysis-england-202425>;
  RCPath pathology volumes (flagged estimate) —
  <https://www.rcpath.org/discover-pathology/news/fact-sheets/pathology-facts-and-figures-.html>;
  OECD Emergency Care WP 83 —
  <https://www.oecd.org/content/dam/oecd/en/publications/reports/2015/08/emergency-care-services_g17a26ec/5jrts344crns-en.pdf>.
- EHR interaction intensity (~597 audit events/encounter) —
  <https://pmc.ncbi.nlm.nih.gov/articles/PMC10148376/>; OLTP read/write
  conventions — <https://www.cs.cmu.edu/~pavlo/papers/oltpbench-vldb.pdf>.
- Busy-hour conventions — Cisco VoIP traffic analysis
  <https://www.cisco.com/c/en/us/td/docs/ios/solutions_docs/voip_solutions/TA_ISD.html>;
  ITU-T E.500 <https://www.itu.int/rec/T-REC-E.500>; ED diurnal
  distribution <https://pmc.ncbi.nlm.nih.gov/articles/PMC6656946/>.
- Class-anchoring precedent — TPC-C v5.11 (warehouse-anchored tpmC ceiling)
  <https://www.tpc.org/tpc_documents_current_versions/pdf/tpc-c_v5.11.0.pdf>;
  SAP SAPS <https://www.sap.com/about/benchmark/measuring.html>.
- Real-system envelope — NHS Spine (peak 3,500 msg/s, ~60M)
  <https://digital.nhs.uk/services/spine>; Finland Kanta (>2M docs/day)
  <https://www.kanta.fi/en/statistics>; Catalonia 13M-patient openEHR CDR
  whitepaper (117M compositions; per-EHR volume dominates query cost)
  <https://hip.vitagroup.ag/wp-content/uploads/2026/04/Whitepaper_EN_Towards_large_scale_openEHR_Clinical_Data_Repositories.pdf>.

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
- Rust crates (verified live 2026-07-21): serde-saphyr
  <https://crates.io/crates/serde-saphyr> (1.0.0-rc.1; RUSTSEC: none) vs the
  archived serde_yaml and the libyaml forks carrying RUSTSEC-2023-0075
  <https://rustsec.org/advisories/RUSTSEC-2023-0075.html>; hdrhistogram
  <https://crates.io/crates/hdrhistogram> (7.6.0;
  <https://github.com/HdrHistogram/HdrHistogram_rust>).
- AQL/RESULT_SET equivalence grounding: QUERY
  `docs/AQL/master03-syntax.adoc` (§SELECT/§DISTINCT/§TOP/§ORDER BY/§LIMIT),
  `master04-result_structure.adoc`; ITS-REST
  `schemas/query/{ResultSet,ResultSetColumn,ResultSetRow,ResultSetMetadata}.yaml`,
  `docs/query/{Request,Response}.md` (all vendored).
- Our instrument: `tools/conformance/` (ECC), latest committed baseline
  `docs/conformance/ferroehr/CONFORMANCE_REPORT.md` (402 case×format
  executions · 384 passed · 0 failed · 18 N/A; CORE PASS / STANDARD PASS /
  OPTIONS OBTAINED; 394 active catalogue cases).
