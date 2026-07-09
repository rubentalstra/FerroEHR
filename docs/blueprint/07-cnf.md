# Blueprint 7 — CNF (Conformance)

Scope: the vendored openEHR Conformance component at `docs/specs/openehr/CNF/`
(Conformance Guide, Platform Conformance Test Schedule, Platform Profiles,
Conformance Certificate, the Robot suite + test-data corpus), pinned at
`openEHR/specifications-CNF` master commit
`33251d2abe5a75c042e11c9385d2e9a79aa15904` (`CNF/PROVENANCE.md`). Every CNF
document is `spec_status: DEVELOPMENT`; the amendment record shows the schedule
last touched **24 Mar 2022 (0.8.6)** with "CNF Release 1.0.0 (unreleased)"
(`docs/platform_test_schedule/master00-amendment_record.adoc`). The schedule is
explicitly EHRbase-derived: *"Rewrite main schedule based on EhrBase (Github
commit 674e8b2)"* (same file, issue 0.8.0, 23 Nov 2021). Per ADR-008 and the
project's ECC decision (`tools/conformance/src/lib.rs`), this corpus is the
**design-time oracle** for what to test; the runnable instrument is our own ECC
framework. This chapter also contains the requested audit of the runner
(`tools/conformance/src`) against the vendored CNF + ITS-REST — §4/§5 report
every version-string or schedule divergence found.

---

## Normative requirements (what a compliant CDR MUST do)

The CNF component defines conformance *assessment*, so its "MUSTs" are about
what a platform must demonstrably pass and how the demonstration is produced.

1. **Two test aspects must both be assessed: API conformance and Data
   Validation conformance.** *"Conformance of the implemented APIs to the
   published APIs, in a concrete API technology … Conformance of platform's
   validation of data against semantic models (archetypes etc)"* —
   `CNF/docs/platform_test_schedule/master03-overview.adoc` §Scope (same table
   in `CNF/docs/guide/master03-overview.adoc` §Product Scope).

2. **Test cases derive from the openEHR Platform Service Model (SM)
   operations**, each documented as
   `<SERVICE_COMPONENT>.<operation>-<test-specific id>` with
   Description / Pre-conditions / Post-conditions / Flow; *"A 'test' is
   therefore the execution of a particular test case with a particular data
   set."* — `master03-overview.adoc` §API Conformance Test Design. The
   technology chain is SM operation → REST binding → abstract test case →
   executable runner — `CNF/docs/guide/master04-framework.adoc` §From
   Specifications to Runnable Tests.

3. **The Conformance Statement must state the supported RM version(s).**
   *"The supported RM version(s) by the SUT should be stated in the Conformance
   Statement … The minimum required version is RM 1.0.2."* —
   `master03-overview.adoc` (NOTE). (Outdated floor; we pin RM 1.2.0 —
   `docs/VERSIONS.md`.)

4. **Functional test suites, per SM component** (the API-conformance corpus,
   chapter = suite, counts = `==== Test Case` headings in the vendored files):
   - DEFINITION/ADL 1.4 — 15 cases (`master04-func_tc_definition_adl.adoc`:
     upload/get/list/validate/**delete** OPT);
   - DEFINITION/QUERY — 7 cases (`master05-func_tc_definition_query.adoc`:
     store, get, **list_queries**);
   - EHR — 21 cases (`master06-func_tc_ehr.adoc`: has/create/get EHR,
     EHR_STATUS get/set/clear flags, incl. the valid/invalid EHR_STATUS
     data-set tables);
   - EHR/COMPOSITION — 31 cases (`master07-func_tc_ehr_composition.adoc`);
   - EHR/CONTRIBUTION — 31 cases (`master08-func_tc_ehr_contribution.adoc`,
     incl. `I_EHR_CONTRIBUTION.list_contributions()` §at line 595);
   - EHR/DIRECTORY — 37 cases (`master09-func_tc_ehr_directory.adoc`, incl.
     `I_EHR_DIRECTORY.get_versioned_directory()` §at line 670);
   - DEMOGRAPHIC — 24 cases (`master10-func_tc_demographic.adoc`);
   - QUERYING — 5 cases (`master11-func_tc_querying.adoc` — a stub, see §5);
   - ADMIN — 18 cases (`master12-func_tc_admin.adoc`);
   - MESSAGING — 14 cases (`master13-func_tc_messaging.adoc` — EHR Extract /
     TDS, heavily TBD).

5. **Data-validation (content) test suites** — commit variable data sets,
   assert accept/reject against the archetype/OPT constraint:
   COMPOSITION structure — 12 cases (`master15-content_tc_composition.adoc`,
   the `CONT-COMP-content_card_*-context_*` matrix); ENTRY — 26 cases
   (`master16-content_tc_entry.adoc`); data types — Basic 5 / Text 6 /
   Quantity 47 / Date-time 13 / Time-spec 0 (empty) / Encapsulated 4 / URI 6
   (`master17.1`–`17.7`). Implementation note: the constraining archetypes
   *"should be generated"* per variant (`master15` §Implementation notes).

6. **Profiles gate the claim.** *"CORE: a minimal functional openEHR platform
   implementation that enables the storage and retrieval of openEHR EHR data;
   STANDARD: … adds AQL querying and logging to the CORE; OPTIONS: components
   that are considered optional … In order to obtain CORE or STANDARD
   conformance, **all** mentioned capabilities must be met in testing. …
   OPTIONS is obtained if **any** optional capability is passed in testing."*
   — `CNF/docs/profiles/master03-profiles.adoc` (preamble).

7. **The CORE capability set** (functional table, same file): ADL 1.4
   Archetype provisioning, ADL 1.4 OPT provisioning, EHR Operations,
   EHR Status, Composition Operations, Change sets, Versioning, Archetype
   Validation, plus the DEFINITION and EHR REST APIs. **STANDARD adds** Query
   provisioning, Directory Operations, AQL basic, and the QUERY API.
   **OPTIONS** holds ADL 2 provisioning, Demographic persistence (Party /
   Party-Relationship / archetype validation), AQL advanced, AQL & terminology,
   the six Admin capabilities (Activity Report, Physical Deletion, EHR
   Dump/Load, Bulk EHR load, EHR Archive, Demographic Archive), Messaging
   (EHR Extract, TDS), and the DEMOGRAPHIC / ADMIN / MESSAGE APIs.

8. **Non-functional capabilities:** *Signing* is STANDARD; *Anonymous EHRs*
   is CORE + STANDARD; the external data formats are **XML, JSON** —
   `master03-profiles.adoc` §Non-Functional / §Other Non-Functional. (So every
   wire-sensitive case must be demonstrable in both canonical formats.)

9. **Result artefacts:** a Test Execution Report, a Conformance Statement, and
   (via an assessing authority) a Conformance Certificate —
   `guide/master04-framework.adoc` §Specifications. The certificate's shape —
   per-capability, per-conformance-point, per-test-case, per-technology
   (REST/protobuf columns) detail plus a Profile Report — is templated in
   `CNF/docs/certificate/master03-certificate.adoc`.

10. **Test environment:** *"An operational test environment requires at a
    minimum a test application with the appropriate protocol client(s) in
    order to exercise the SUT"* — `guide/master05-assessment.adoc`
    §Test Environment; the SUT is exercised through its public API only
    (`guide/diagrams/conformance_sut_rest.xml`).

11. **Tendering/procurement usage:** profiles are composable — adopt CORE or
    STANDARD and add options (`profiles/master02-overview.adoc`); conformance
    claims must be traceable to a test run, not asserted
    (`guide/master03-overview.adoc` §Goals).

---

## Current implementation state (verified, not assumed)

The instrument is `tools/conformance` (the **ECC** framework, ADR-008's
acceptance instrument; design `docs/design/conformance-framework.md`): our own
case universe with stable `ECC-<AREA>-<NNN>` ids allocated in
`tools/conformance/inventory/ecc-catalog.tsv` (314 lines, 310 allocations, 0
retired/planned), executed by `scripts/conformance.sh` /
`tools/conformance/src/bin/conformance.rs` against a self-hosted or external
SUT. Latest committed run (2026-07-09, `docs/conformance/results.json` +
`CONFORMANCE_REPORT.md`): **318 case×format executions · 211 passed · 106
failed · 1 skipped**. `docs/GAP_REGISTER.md` §2.1 marks ArchetypeValidation
(81 failures) as "the one big gap"; note the register also records ECC as
*"suspended during the ADR-011 rebuild, re-converges at P19"*.

Per requirement:

- **R1 (both aspects)** — **DONE (as ECC).** API cases (EHR/STA/COM/CTB/DIR/
  TPL/SQR/QRY/ADM/DEM areas) + data-validation cases (VAL area, 118 cases,
  `suites/content/*` incl. programmatic OPT tightening in
  `content/author.rs` exactly per master15's "should be generated" note).
- **R2 (SM-derived, documented cases)** — **DONE (own form).** `CaseMeta`
  (`src/model/case.rs`) carries id/title/area/capability/profiles/formats/
  citation/compare; suites are organized by schedule chapter
  (`src/suites/mod.rs` comments map modules → master04…17). **Caveat:** the
  catalogue doc (`model/catalog.rs`) promises official CNF ids as "trace
  references carried in metadata", but `CaseMeta` has no field for the
  schedule's `I_*.op-case` id — traceability is by module comment + citation
  prose only.
- **R3 (versions in the statement)** — **PARTIAL.** `SpecVersions::latest()`
  (`src/model/version.rs:30-38`) records RM 1.2.0 / ITS-REST 1.0.3 / AQL 1.1.0
  / TERM 3.1.0 into `results.json` and the report header — but see §4 D1: the
  ITS-REST identity is not actually 1.0.3.
- **R4 (functional suites)** — **PARTIAL.** Implemented per area (catalogue
  counts): EHR 12 + STA 10 (schedule's 21 EHR cases split), COM 31, CTB 31,
  DIR 37, TPL 16, SQR 7, QRY 13 (schedule master11 is a stub; ours adds the
  AQL corpus cases), ADM 6 of 18 (upstream chapter is 21×TBD-riddled), DEM 24.
  **MISSING:** MSG (0 cases; `Area::Msg` exists in `catalog.rs`, no suite —
  SM-5 is design-only, `docs/design/sm-platform/10-message-integration.md`);
  SEC (0 cases; `Area::Sec` exists; the vendored Robot corpus has
  `SECURITY_TESTS/I_OAuth2_Keycloak` with no ECC counterpart).
- **R5 (content validation)** — **PARTIAL.** 118 VAL cases registered; run
  state 37 pass / 81 fail — the failures are server-side validation depth
  (occurrence/cardinality/value constraints; `docs/GAP_REGISTER.md` §2.1,
  F-open-3/9/31/40/30 in `docs/conformance/COVERAGE_GAPS.md` §1), not missing
  cases. master17.5 (time-spec) has zero upstream cases to transcribe.
- **R6/R7 (profiles + matrix)** — **PARTIAL.** `required_capabilities()`
  (`src/model/profile.rs:17-48`) matches master03-profiles for CORE/STANDARD
  (incl. AnonymousEhrs in CORE, Signing in STANDARD); verdicts are
  machine-computed all-or-nothing (`verdict()`, report §Standard/§Options).
  Two divergences: **(a)** OPTIONS is computed as all-of
  `[AdminApi, DemographicApi]` (`profile.rs:46` + all-or-nothing `verdict`),
  while the spec grants OPTIONS *"if any optional capability is passed"* —
  and the runner's OPTIONS set omits ADL 2, AQL advanced/terminology, the
  admin sub-capabilities, and Messaging entirely; **(b)** three CORE
  capabilities have **zero tagged cases** — the run report shows
  `Adl14ArchetypeProvisioning 0/0/0/0 fail`, `Versioning 0/0/0/0 fail`,
  `AnonymousEhrs 0/0/0/0 fail` — versioned-read cases exist but are tagged
  `CompositionOps` etc., so **CORE/STANDARD are structurally unclaimable even
  with a perfect server** (unevidenced-capability rule, `profile.rs:101` +
  the `unevidenced_required_capability_fails_the_profile` test).
- **R8 (formats, signing, anonymous EHRs)** — **PARTIAL.** JSON+XML both run
  where claimed (`Format` in `case.rs`; the run has 318 executions over 310
  cases). Signing: 5 SIG cases (`suites/signing.rs`; 1 skipped pending a
  pgp-keyed SUT). Anonymous EHRs: exercised implicitly by `POST /ehr` with no
  body (`suites/support.rs::create_ehr`) but no case carries the capability
  tag (see R6b). XML coverage has one server gap: ECC-COM-022 xml → 406
  ("canonical XML … once typed payloads land").
- **R9 (SM→REST concretization)** — **PARTIAL.** Done throughout the suites,
  but four SM operations were concretized to URLs that exist in **no** vendored
  ITS-REST vintage — see §4 D2 (12 structural failures currently booked
  against the server).
- **R10 (result artefacts)** — **PARTIAL.** Generated per run:
  `CONFORMANCE_REPORT.md` (SUT identity, per-area matrix, failures, profile
  verdicts, deviations-with-reasons), `CATALOG.md`, `results.json`, four
  badges (`src/reporting/*`). **MISSING:** a Conformance **Statement** and
  **Certificate** document per the `certificate/master03-certificate.adoc`
  template (per-conformance-point table, profile report) — the report §4
  verdict tables are the raw material.
- **R11 (test environment)** — **DONE.** Self-host harness (`engine/sut.rs`,
  `tests/self_host.rs`, testcontainers PG18) + external SUT
  (`--base-url`, `bin/conformance.rs:49`); Basic/none auth modes; the SUT is
  driven exclusively through the REST surface.

---

## The runner-vs-vendored-spec audit (the suspected "2024-era mismatch")

Every place `tools/conformance/src` references an outdated spec version or
diverges from the vendored CNF/ITS-REST, with adjudication:

**D1 — The "ITS-REST 1.0.3" label does not match the generated contract
(systemic).** `SpecVersions::latest()` (`model/version.rs:34`) and ~200
citation strings across `suites/*` say `ITS-REST 1.0.3`; `docs/VERSIONS.md`
pins 1.0.3; the vendored ITS-REST *spec text* is Release-1.0.3
(`docs/specs/openehr/ITS-REST/PROVENANCE.md`, commit `4aec22d`). **But** the
OAS bundles that `emit-rest` generates the server contract from are pinned to
ITS-REST **master** commit `e8a093e` fetched 2026-07-04
(`crates/openehr-its/vendor/rest-oas/PROVENANCE.md`), whose `info.version` is
`development`/`latest`. Diff vs the Release-1.0.3 OAS: ehr bundle **+5
ITEM_TAG paths** (`/ehr/{ehr_id}/tags`, composition/ehr_status `…/tags[/key]`),
definition bundle **+2 example paths**
(`/definition/template/adl1.4/{template_id}/example`, adl2 equivalent), and
708–3241 changed lines per bundle (ehr 2361, definition 3241, query 708; admin
identical). Consequence: the SUT implements the *development* contract while
the runner and report **claim** 1.0.3, and nothing machine-checks the release
(upstream stamps the 1.0.3 bundles `version: latest` too). Any status-code or
schema detail that changed between 1.0.3 and development is silently asserted
against whichever text the case author read. Fix: pick one identity (recommend:
re-vendor `rest-oas/` from the Release-1.0.3 tree, or re-label everything
`development@e8a093e`) and derive `SpecVersions.its_rest` from the vendored
provenance instead of a hardcoded literal.

**D2 — Four SM operations concretized at URLs with no ITS-REST binding, each
citing "ITS-REST 1.0.3" and each booked as a server failure (12 of the 106):**
   1. `GET /ehr/{ehr_id}/contribution` (`suites/contribution.rs:804,817,836,
      860,875`; cases `ctb/list-contributions-*` = ECC-CTB-027…031, citation
      "ITS-REST 1.0.3 CONTRIBUTION API §commit_contribution/get_contribution").
      ITS-REST 1.0.3 (and development) define **POST only** on that path
      (verified in `ehr-codegen.openapi.yaml`). The schedule *does* require
      `I_EHR_CONTRIBUTION.list_contributions()` (master08:595) — an SM
      operation ITS-REST never bound. Currently: 405 × 5.
   2. `GET /ehr/{ehr_id}/versioned_directory` (`suites/directory.rs:814`;
      `dir/get-versioned-directory-*`, ECC-DIR-032…034). No such path in any
      vendored ITS-REST vintage; the vendored Robot suite itself realizes
      `get_versioned_directory` as *"get DIRECTORY at version"*
      (`I_EHR_DIRECTORY.get_versioned_directory-empty_ehr.robot:50`), i.e.
      `GET /directory/{version_uid}`. Currently: 404 on the two-versions case.
   3. `GET /definition/query` (`suites/definition_query.rs:172,184`;
      `sqr/list-queries-*`, ECC-SQR-004/005). ITS-REST's list resource is
      `GET /definition/query/{qualified_query_name}` (verbs `[get, put]`,
      `definition-codegen.openapi.yaml`); a bare `/definition/query` does not
      exist. Currently: 404 × 2.
   4. `DELETE /definition/template/adl1.4/{template_id}`
      (`suites/definition_adl14.rs:317` — the comment concedes *"Delete is not
      a standard ITS-REST ADL1.4 verb"* yet the cases ECC-TPL-014/015 **fail**
      on 405 instead of skipping; schedule source: master04:319
      `I_DEFINITION_ADL14.delete_opt()`; ITS-REST puts template deletion in
      the ADMIN API only). Currently: 405 × 2.
   Root cause is the same for all four: **the 2021/22 schedule is SM-based and
   wider than ITS-REST 1.0.3** — the runner must either bind these to the real
   resource (2), mark them OPTIONS/extension cases asserting 405-when-absent,
   or skip-with-reason ("SM op without ITS-REST binding") instead of failing.

**D3 — The AQL golden corpus is 2019-era EHRbase dialect, driven as "valid"
against the AQL-1.1-pinned parser (5 of the 106 failures).** The vendored
`test_data_sets/query/aql_queries_valid/` queries place `LIMIT` **before**
`ORDER BY` (e.g. A/106: `… LIMIT 5 ORDER BY e/time_created ASC`), which is
invalid under the pinned grammar — `selectQuery : selectClause fromClause
whereClause? orderByClause? limitClause?`
(`crates/openehr-query/vendor/grammar/AqlParser.g4:22-24`) — so the parser's
rejection ("found 'Order'") is **spec-correct** and the golden is defective;
ECC-QRY-009/011/013 mis-book it as a server failure. The other corpus
rejection — `e/ehr_status` on `EHR` (A/106_get_ehrs variant, D/312) — is the
opposite adjudication: AQL 1.1 itself uses `e/ehr_status/…` paths
(`docs/specs/openehr/QUERY/docs/AQL/master03-syntax.adoc:373,443`), so
"attribute `ehr_status` is not defined on EHR (RM model)" is a **real server
gap** (the AQL EHR object dereferences `ehr_status`; the RM-model lookup must
special-case it). The runner needs a per-golden adjudication list: dialect
defects → skip-with-reason, spec-valid queries → keep failing.

**D4 — OPTIONS profile semantics** (also under R6): spec = *any* optional
capability grants OPTIONS, per-capability; runner = all-of a 2-capability set
(`profile.rs:46`). Also the runner's `Capability` enum has no members for
ADL 2 provisioning, AQL advanced, AQL & terminology, the six Admin
sub-capabilities, or Messaging — the OPTIONS surface of
`master03-profiles.adoc` is mostly unmodeled.

**D5 — Capability tagging vs the CORE matrix** (R6b): `Versioning`,
`AnonymousEhrs`, `Adl14ArchetypeProvisioning` have zero tagged cases
(run report §4), making CORE unclaimable by construction. Note
`Adl14ArchetypeProvisioning` may be intentionally empty (we provision OPTs,
not source archetypes — TPL covers `Adl14OptProvisioning`), but then the CORE
matrix in `profile.rs:20` must document how that capability is evidenced, or
the claim can never be produced.

**D6 — Provenance is consistent where it matters:** the runner's `CorpusPin`
(`reporting/results.rs:88-92`, `openEHR/specifications-CNF@33251d2a`) matches
`CNF/PROVENANCE.md` exactly; the fixture RM-adaptation layer
(`testdata/fixtures.rs` §RM-version adaptation) and the golden normalizer
(`suites/query_golden.rs`, named-rule suppression incl. `RmTypeIgnored`,
`SignatureDefaultOn`, `NumberFormatInsensitive`) correctly and *documentedly*
bridge the RM-1.0.x-era corpus to RM 1.2.0. One cosmetic nit: the
`query_golden.rs:366` test stuffs `meta._schema_version = "1.0.3"` — the
ITS-REST RESULT_SET example value is `"1.0.0"`
(`docs/specs/openehr/ITS-REST/specifications/schemas/query/ResultSetMetadata.yaml:32`);
harmless (meta is ignored) but worth aligning wherever the server emits it.

So: the owner's suspicion is confirmed in substance but not in vintage — the
fights are not "2024-era" runner literals; they are (a) a **2026 development
OAS labeled 1.0.3** (D1), (b) the **2021/22 SM-based schedule exceeding
ITS-REST 1.0.3** (D2), and (c) the **2019-era EHRbase AQL dialect in the
goldens** (D3). ~17 of the 106 failures are mis-booked runner/spec-gap issues,
not server defects.

---

## Remaining work (ordered, concrete)

1. **Resolve the ITS-REST identity (D1).** Decide 1.0.3 vs development; align
   `crates/openehr-its/vendor/rest-oas/` and `SpecVersions`/citations/
   `docs/VERSIONS.md` to the same commit; derive the version string from
   provenance, not a literal. Add a CI check that the two vendored ITS-REST
   trees (docs vs rest-oas) are the same ref.
2. **Re-adjudicate the D2 cluster (12 failures).** Rebind
   `get_versioned_directory` to `GET /directory/{version_uid}` semantics (the
   Robot realization); convert `list_contributions`, `list_queries` (bare),
   and `delete_opt` to skip-with-reason or extension-cases asserting the
   405/404-when-unbound contract; update the citations to say "SM op, no
   ITS-REST 1.0.3 binding".
3. **Golden-corpus adjudication list (D3, 5 failures).** Mark the
   LIMIT-before-ORDER-BY goldens as corpus-dialect defects (skip-with-reason);
   keep `e/ehr_status` failing and fix the AQL engine's EHR object model
   (AQL 1.1 master03-syntax §identified paths) — tracked with F-open-20 in
   `COVERAGE_GAPS.md`.
4. **Make CORE claimable (D5).** Tag the versioned-read cases (`com/get-
   versioned-composition`, `dir/get-versioned-directory-*`, revision-history
   cases) `Capability::Versioning`; add/tag an `AnonymousEhrs` case (the
   no-body `POST /ehr` path already exists); decide + document
   `Adl14ArchetypeProvisioning` evidencing.
5. **The ArchetypeValidation push (81 failures)** — the single
   highest-leverage server work item (`GAP_REGISTER.md` §2.1): occurrence/
   cardinality/value-constraint enforcement depth in the P15 validator;
   reconcile with the open spec-audit findings (82 open,
   `docs/spec-audit/SPEC_AUDIT.md`).
6. **Model the full OPTIONS surface (D4):** extend `Capability` with ADL 2
   provisioning, AQL advanced, AQL & terminology, the Admin sub-capabilities,
   Messaging; switch the OPTIONS verdict to per-capability "any passes"
   reporting per `master03-profiles.adoc`.
7. **Emit the Conformance Statement + Certificate artefacts (R10)** from
   `results.json`, following `certificate/master03-certificate.adoc`'s table
   shapes (capability × conformance point × test case × technology; profile
   report).
8. **MSG + SEC areas (R4).** MSG cases land with SM-5 (EHR Extract/TDS —
   design done, `sm-platform/10`); SEC cases (auth 401/403 surface, mirroring
   `SECURITY_TESTS/I_OAuth2_Keycloak`) can precede it cheaply.
9. **Trace-reference field (R2 caveat):** add an optional `schedule_ref` to
   `CaseMeta` carrying the official `I_*.op-case` id where one exists, so the
   catalogue's "trace references carried in metadata" claim becomes true.
10. **Fix the remaining honest server findings** surfaced by the run and
    already registered: canonical-XML for versioned reads (ECC-COM-022),
    stored-query formalism/AQL validation on PUT (ECC-SQR-006/007),
    demographic delete semantics (ECC-DEM-005…020), contribution edge cases.

---

## Spec defects/TBDs encountered (verbatim, cited)

- **The whole component is unreleased development text.** `manifest.json`:
  `"status": "DEVELOPMENT"`; amendment record: *"CNF Release 1.0.0
  (unreleased)"*, latest issue 0.8.6 dated 24 Mar 2022
  (`platform_test_schedule/master00-amendment_record.adoc`).
- **The schedule is EHRbase-derived by its own admission:** *"Rewrite main
  schedule based on EhrBase (Github commit 674e8b2)"* (ibid., issue 0.8.0);
  many cases carry `// EhrBase ref:` comments (e.g. master11:56).
- **master11 (QUERYING) is a stub:** Test Environment and Test Data Sets are
  single-cell tables containing *"TBD"*; every flow row is *"xx"*
  (*"|Description | xx |Pre-conditions | xx |Post-conditions| xx |Flow | xx"*);
  and there is a literal placeholder heading *"==== Test Case bbbb — TBD"*
  (`master11-func_tc_querying.adoc:31-100`).
- **Heavily-TBD chapters:** master10 DEMOGRAPHIC (26 `TBD`s), master12 ADMIN
  (21), master13 MESSAGING (17); master17.5 (time specification) is a 12-line
  file with 2 `TBD`s and zero test cases.
- **Guide assessment chapter is empty where it matters:** *"== Tooling —
  TBD"*, *"=== Test Execution Report — TBD"*, *"=== Conformance Statement —
  TBD"*, *"=== Conformance Certification — TBD"*
  (`guide/master05-assessment.adoc`).
- **Stale RM floor:** *"The minimum required version is RM 1.0.2."*
  (`platform_test_schedule/master03-overview.adoc` NOTE) — predates RM 1.1/1.2.
- **Schedule–ITS-REST binding gaps (the D2 root):** the schedule normatively
  defines `I_EHR_CONTRIBUTION.list_contributions()` (master08:595),
  `I_EHR_DIRECTORY.get_versioned_directory()` (master09:670),
  `I_DEFINITION_ADL14.delete_opt()` (master04:319), and
  `I_DEFINITION_QUERY.list_queries()` (master05:93) — none of which has a
  resource in ITS-REST Release-1.0.3 (`GET` absent on
  `/ehr/{ehr_id}/contribution`; no `/versioned_directory`; no bare
  `/definition/query`; no ADL 1.4 `DELETE`).
- **The AQL corpus contradicts the AQL 1.1 grammar:** vendored "valid" queries
  place `LIMIT` before `ORDER BY` (e.g. `aql_queries_valid/A/106…json`:
  *"select … from EHR e CONTAINS COMPOSITION c [openEHR-EHR-COMPOSITION.minimal.v1]
  LIMIT 5 ORDER BY e/time_created ASC"*) while the grammar requires
  `orderByClause? limitClause?` (`AqlParser.g4:23`).
- **Upstream doesn't stamp its own releases:** the Release-1.0.3 OAS bundles
  carry `info.version: latest` (`docs/specs/openehr/ITS-REST/computable/OAS/
  ehr-codegen.openapi.yaml:4`), so a release cannot be machine-verified from
  the artifact alone — the reason D1 could happen silently.
- **Certificate doc is a mock:** `certificate/master03-certificate.adoc` is a
  filled-in fictional example ("BestEHR release 2.4", "ACME EHR systems LLC")
  serving as the only template for the statement/certificate shape.
