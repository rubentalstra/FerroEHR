# Conformance Report (generated)

> Generated from a conformance run — never hand-asserted. Every claim is a
> pure function of the recorded outcomes; every coverage bound is printed.

## 1. System under test

| Field | Value |
|---|---|
| Product | ehrbase-rs ehrbase-rs 3.5.0 |
| SUT class | ours (ehrbase-rs) |
| Base URL | `http://localhost:8080/ehrbase/rest/openehr/v1` |
| Auth mode | basic |
| Edition policy | pinned (release-1.1.0) |
| Spec versions | RM 1.2.0 · ITS-REST Release-1.1.0 · AQL 1.1.0 · TERM 3.1.0 |
| Reference corpus | openEHR/specifications-CNF@33251d2a |
| Run started | 2026-07-21T08:31:42.09722Z |

**402 case×format executions · 384 passed · 0 failed · 0 errored · 0 skipped · 18 not applicable.**

## 2. Per-area matrix

| Area | Catalogue (active) | Passed | Failed | Errored | Skipped | N/A |
|---|--:|--:|--:|--:|--:|--:|
| EHR — EHR service | 13 | 13 | 0 | 0 | 0 | 0 |
| STA — EHR_STATUS | 10 | 10 | 0 | 0 | 0 | 0 |
| COM — COMPOSITION | 32 | 39 | 0 | 0 | 0 | 0 |
| CTB — CONTRIBUTION (change sets) | 31 | 31 | 0 | 0 | 0 | 0 |
| DIR — DIRECTORY (FOLDER) | 37 | 37 | 0 | 0 | 0 | 0 |
| TPL — Template / OPT provisioning | 17 | 17 | 0 | 0 | 0 | 0 |
| SQR — Stored-query provisioning | 7 | 7 | 0 | 0 | 0 | 0 |
| QRY — AQL execution | 25 | 25 | 0 | 0 | 0 | 0 |
| VAL — Content / archetype validation | 119 | 119 | 0 | 0 | 0 | 0 |
| DEM — Demographic service | 31 | 31 | 0 | 0 | 0 | 0 |
| ADM — Admin service | 14 | 6 | 0 | 0 | 0 | 8 |
| SEC — Security / authorization | 2 | 2 | 0 | 0 | 0 | 0 |
| SIG — Version signing | 5 | 6 | 0 | 0 | 0 | 0 |
| MSG — Messaging | 10 | 0 | 0 | 0 | 0 | 10 |
| TS — Terminology-server integration | 9 | 9 | 0 | 0 | 0 | 0 |
| SF — Simplified Formats (FLAT / STRUCTURED / Web Template) | 16 | 16 | 0 | 0 | 0 | 0 |
| ADL2 — ADL2 template provisioning | 12 | 12 | 0 | 0 | 0 | 0 |
| AQT — AQL terminology functions | 4 | 4 | 0 | 0 | 0 | 0 |

## 3. Capability matrix

Cases grouped by capability; the evidence classification folds a transport error into `failed` (an errored capability is never claimed as passed).

| Capability | Passed | Failed | Errored | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 1 | 0 | 0 | 0 | 0 | pass |
| Adl14OptProvisioning | 16 | 0 | 0 | 0 | 0 | pass |
| Adl2Provisioning | 12 | 0 | 0 | 0 | 0 | pass |
| EhrOperations | 12 | 0 | 0 | 0 | 0 | pass |
| EhrStatus | 10 | 0 | 0 | 0 | 0 | pass |
| CompositionOps | 35 | 0 | 0 | 0 | 0 | pass |
| ChangeSets | 31 | 0 | 0 | 0 | 0 | pass |
| Versioning | 7 | 0 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 119 | 0 | 0 | 0 | 0 | pass |
| DirectoryOps | 34 | 0 | 0 | 0 | 0 | pass |
| QueryProvisioning | 7 | 0 | 0 | 0 | 0 | pass |
| AqlBasic | 24 | 0 | 0 | 0 | 0 | pass |
| AqlAdvanced | 1 | 0 | 0 | 0 | 0 | pass |
| AqlTerminology | 4 | 0 | 0 | 0 | 0 | pass |
| PartyOperations | 25 | 0 | 0 | 0 | 0 | pass |
| PartyRelationshipOperations | 6 | 0 | 0 | 0 | 0 | pass |
| AdminActivityReport | 0 | 0 | 0 | 0 | 4 | not evidenced |
| AdminPhysicalDeletion | 6 | 0 | 0 | 0 | 1 | pass |
| AdminEhrDumpLoad | 0 | 0 | 0 | 0 | 1 | not evidenced |
| AdminEhrArchive | 0 | 0 | 0 | 0 | 1 | not evidenced |
| AdminDemographicArchive | 0 | 0 | 0 | 0 | 1 | not evidenced |
| MessagingEhrExtract | 0 | 0 | 0 | 0 | 7 | not evidenced |
| MessagingTds | 0 | 0 | 0 | 0 | 3 | not evidenced |
| Signing | 6 | 0 | 0 | 0 | 0 | pass |
| AnonymousEhrs | 1 | 0 | 0 | 0 | 0 | pass |
| Authentication | 2 | 0 | 0 | 0 | 0 | pass |
| Terminology | 9 | 0 | 0 | 0 | 0 | pass |
| SimplifiedFormats | 16 | 0 | 0 | 0 | 0 | pass |

## 4. Profile verdict (machine-computed)

CORE/STANDARD are all-of (every listed capability must be `pass`); OPTIONS is any-of (obtained if any optional capability passes) — `master03-profiles.adoc`. An unevidenced required capability fails the claim.

### Core — PASS

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 1 | 0 | 0 | 0 | pass |
| Adl14OptProvisioning | 16 | 0 | 0 | 0 | pass |
| EhrOperations | 12 | 0 | 0 | 0 | pass |
| EhrStatus | 10 | 0 | 0 | 0 | pass |
| CompositionOps | 35 | 0 | 0 | 0 | pass |
| ChangeSets | 31 | 0 | 0 | 0 | pass |
| Versioning | 7 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 119 | 0 | 0 | 0 | pass |
| AnonymousEhrs | 1 | 0 | 0 | 0 | pass |

### Standard — PASS

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 1 | 0 | 0 | 0 | pass |
| Adl14OptProvisioning | 16 | 0 | 0 | 0 | pass |
| EhrOperations | 12 | 0 | 0 | 0 | pass |
| EhrStatus | 10 | 0 | 0 | 0 | pass |
| CompositionOps | 35 | 0 | 0 | 0 | pass |
| ChangeSets | 31 | 0 | 0 | 0 | pass |
| Versioning | 7 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 119 | 0 | 0 | 0 | pass |
| AnonymousEhrs | 1 | 0 | 0 | 0 | pass |
| QueryProvisioning | 7 | 0 | 0 | 0 | pass |
| DirectoryOps | 34 | 0 | 0 | 0 | pass |
| AqlBasic | 24 | 0 | 0 | 0 | pass |
| Signing | 6 | 0 | 0 | 0 | pass |

### Options — OBTAINED

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl2Provisioning | 12 | 0 | 0 | 0 | pass |
| PartyOperations | 25 | 0 | 0 | 0 | pass |
| PartyRelationshipOperations | 6 | 0 | 0 | 0 | pass |
| AqlAdvanced | 1 | 0 | 0 | 0 | pass |
| AqlTerminology | 4 | 0 | 0 | 0 | pass |
| AdminActivityReport | 0 | 0 | 0 | 4 | not evidenced |
| AdminPhysicalDeletion | 6 | 0 | 0 | 1 | pass |
| AdminEhrDumpLoad | 0 | 0 | 0 | 1 | not evidenced |
| AdminBulkEhrLoad | 0 | 0 | 0 | 0 | no cases |
| AdminEhrArchive | 0 | 0 | 0 | 1 | not evidenced |
| AdminDemographicArchive | 0 | 0 | 0 | 1 | not evidenced |
| MessagingEhrExtract | 0 | 0 | 0 | 7 | not evidenced |
| MessagingTds | 0 | 0 | 0 | 3 | not evidenced |
| SimplifiedFormats | 16 | 0 | 0 | 0 | pass |

## 5. Failures

_No failures in this run._

## 6. Skipped, by reason

_No skips in this run._

## 7. Not applicable to this SUT (extensions / RM-version-sensitive)

Adjudicated in the committed fairness register (foreign SUTs only), not a conformance finding — excluded from pass/fail and capability math.

- **ECC-ADM-007** Admin list contributions — NativeApiOnly: I_ADMIN_SERVICE.list_contributions is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §list_contributions (TBD stub); SM I_ADMIN_SERVICE.list_contributions — no ITS-REST admin route)_
- **ECC-ADM-008** Admin contribution count — NativeApiOnly: I_ADMIN_SERVICE.contribution_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §contribution_count (TBD stub); SM I_ADMIN_SERVICE.contribution_count — no ITS-REST admin route)_
- **ECC-ADM-009** Admin versioned composition count — NativeApiOnly: I_ADMIN_SERVICE.versioned_composition_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §versioned_composition_count (TBD stub); SM I_ADMIN_SERVICE.versioned_composition_count — no ITS-REST admin route)_
- **ECC-ADM-010** Admin composition version count — NativeApiOnly: I_ADMIN_SERVICE.composition_version_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §composition_version_count (TBD stub); SM I_ADMIN_SERVICE.composition_version_count — no ITS-REST admin route)_
- **ECC-ADM-011** Admin export EHRs (dump/load) — NativeApiOnly: I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs is exercised by app/ehrbase/tests/service_dump_load.rs::export_then_load_into_fresh_db_round_trips_byte_equal — no ITS-REST admin route reaches it _(cite: CNF master12 §export_ehrs (TBD stub); SM I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs — no ITS-REST admin route)_
- **ECC-ADM-012** Admin archive EHRs — NativeApiOnly: I_ADMIN_ARCHIVE.archive_ehrs is exercised by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged — no ITS-REST admin route reaches it _(cite: CNF master12 §archive_ehrs (TBD stub); SM I_ADMIN_ARCHIVE.archive_ehrs — no ITS-REST admin route)_
- **ECC-ADM-013** Admin physical party delete — NoRestBinding: I_ADMIN_SERVICE.physical_party_delete has no ITS-REST route and acts on the demographic extension; exercised natively by app/ehrbase/tests/service_admin.rs::physical_party_delete_cascades_relationships_and_spares_partner _(cite: CNF master12 §physical_party_delete (TBD stub); SM I_ADMIN_SERVICE.physical_party_delete acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route)_
- **ECC-ADM-014** Admin archive parties — NoRestBinding: I_ADMIN_ARCHIVE.archive_parties has no ITS-REST route and acts on the demographic extension; the archive path is proven natively by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged _(cite: CNF master12 §archive_parties (TBD stub); SM I_ADMIN_ARCHIVE.archive_parties acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route)_
- **ECC-MSG-001** EHR Extract — export whole EHR (export_ehrs) — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehrs; RM EHR Extract IM (X_VERSIONED_*); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub, listed twice — authoring duplicate))_
- **ECC-MSG-002** EHR Extract — spec-driven export (export_ehr_extracts) — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts (EXTRACT_ENTITY_MANIFEST + EXTRACT_VERSION_SPEC); CNF master13 §I_EHR_EXTRACT.export_ehr_extracts (TBD stub))_
- **ECC-MSG-003** EHR Extract — export of unknown EHR fails — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehrs (ehr_id_does_not_exist precondition); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub))_
- **ECC-MSG-004** EHR Extract — import whole-EHR clone reusing source id — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr; RM common master06 §Copying Case 1 (reuse source EHR identifier); CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-005** EHR Extract — import whole EHR into a caller-fixed id — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr (same patient in another EHR service); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-006** EHR Extract — import into a duplicate target id fails — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr (ehr_create_fail_duplicate_id); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-007** EHR Extract — import extract into an existing EHR — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr_extract; RM common master06 §Copying Case 2 (first receipt clones VERSIONED_OBJECT; re-import is a conflict); CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-008** TDD — import a TDD as a committed COMPOSITION — NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdd; TDD → COMPOSITION over OPT/WebTemplate (openehr_flat::tdd::from_tdd); CNF master13 §I_TDD.import_tdd (TBD stub))_
- **ECC-MSG-009** TDD — import rejects malformed / non-TDD / unknown EHR / unknown template — NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdd (typed envelope rejections); CNF master13 §I_TDD.import_tdd (TBD stub))_
- **ECC-MSG-010** TDD — batch import commits all, fail-fast on error — NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdds; CNF master13 §I_TDD.import_tdds (TBD stub))_

## 8. Edition findings (the SUT's discovered edition profile)

A case satisfied its normative core at a rung below the newest edition — recorded, never a silent pass (`master03-overview.adoc` §API Conformance; the aggregated findings feed the Conformance Statement's supported-versions field).

_None — every laddered assertion matched the newest edition form._

## 9. Coverage bounds (driven vs schedule data-set rows)

Cases whose driven data-set count is below the governing schedule table's row count — a bound is logged, never silent. Widening the driven set is data, not a new case.

| ECC id | Format | Driven / schedule rows |
|---|---|--:|
| ECC-CTB-004 | json | 1/4 |
| ECC-DIR-016 | json | 5/12 |
| ECC-VAL-001 | json | 6/9 |
| ECC-VAL-002 | json | 6/9 |
| ECC-VAL-003 | json | 6/9 |
| ECC-VAL-004 | json | 6/9 |
| ECC-VAL-005 | json | 6/9 |
| ECC-VAL-006 | json | 6/9 |
| ECC-VAL-007 | json | 6/9 |
| ECC-VAL-008 | json | 6/9 |
| ECC-VAL-009 | json | 6/9 |
| ECC-VAL-010 | json | 6/9 |
| ECC-VAL-011 | json | 6/9 |
| ECC-VAL-012 | json | 6/9 |
| ECC-VAL-013 | json | 3/8 |
| ECC-VAL-014 | json | 3/8 |
| ECC-VAL-015 | json | 3/8 |
| ECC-VAL-016 | json | 3/8 |
| ECC-VAL-017 | json | 3/6 |
| ECC-VAL-018 | json | 3/6 |
| ECC-VAL-019 | json | 3/6 |
| ECC-VAL-020 | json | 3/6 |
| ECC-VAL-021 | json | 3/6 |
| ECC-VAL-022 | json | 3/6 |
| ECC-VAL-023 | json | 3/6 |
| ECC-VAL-024 | json | 3/6 |
| ECC-VAL-025 | json | 3/6 |
| ECC-VAL-026 | json | 3/6 |
| ECC-VAL-027 | json | 3/6 |
| ECC-VAL-028 | json | 3/6 |
| ECC-VAL-029 | json | 3/4 |
| ECC-VAL-030 | json | 3/4 |
| ECC-VAL-034 | json | 2/4 |
| ECC-VAL-035 | json | 2/4 |
| ECC-VAL-036 | json | 2/4 |
| ECC-VAL-037 | json | 2/4 |
| ECC-VAL-038 | json | 2/4 |
| ECC-VAL-042 | json | 2/12 |
| ECC-VAL-043 | json | 2/12 |
| ECC-VAL-044 | json | 2/3 |
| ECC-VAL-045 | json | 2/3 |
| ECC-VAL-046 | json | 2/5 |
| ECC-VAL-047 | json | 2/5 |
| ECC-VAL-048 | json | 2/5 |
| ECC-VAL-049 | json | 2/5 |
| ECC-VAL-050 | json | 2/3 |
| ECC-VAL-051 | json | 2/5 |
| ECC-VAL-052 | json | 2/3 |
| ECC-VAL-053 | json | 2/5 |
| ECC-VAL-054 | json | 2/5 |
| ECC-VAL-055 | json | 2/5 |
| ECC-VAL-056 | json | 2/7 |
| ECC-VAL-057 | json | 2/8 |
| ECC-VAL-058 | json | 2/9 |
| ECC-VAL-059 | json | 3/9 |
| ECC-VAL-060 | json | 2/19 |
| ECC-VAL-061 | json | 2/5 |
| ECC-VAL-062 | json | 2/5 |
| ECC-VAL-063 | json | 2/5 |
| ECC-VAL-064 | json | 2/5 |
| ECC-VAL-065 | json | 2/5 |
| ECC-VAL-066 | json | 2/5 |
| ECC-VAL-067 | json | 2/4 |
| ECC-VAL-068 | json | 4/12 |
| ECC-VAL-069 | json | 2/7 |
| ECC-VAL-070 | json | 2/7 |
| ECC-VAL-071 | json | 4/10 |
| ECC-VAL-072 | json | 2/7 |
| ECC-VAL-073 | json | 4/27 |
| ECC-VAL-074 | json | 2/68 |
| ECC-VAL-075 | json | 2/24 |
| ECC-VAL-076 | json | 4/8 |
| ECC-VAL-077 | json | 2/29 |
| ECC-VAL-078 | json | 2/4 |
| ECC-VAL-079 | json | 4/8 |
| ECC-VAL-080 | json | 2/5 |
| ECC-VAL-081 | json | 2/9 |
| ECC-VAL-082 | json | 4/9 |
| ECC-VAL-083 | json | 2/35 |
| ECC-VAL-084 | json | 2/10 |
| ECC-VAL-085 | json | 4/6 |
| ECC-VAL-086 | json | 2/7 |
| ECC-VAL-087 | json | 4/6 |
| ECC-VAL-088 | json | 2/7 |
| ECC-VAL-089 | json | 4/18 |
| ECC-VAL-090 | json | 2/12 |
| ECC-VAL-091 | json | 2/12 |
| ECC-VAL-092 | json | 2/12 |
| ECC-VAL-093 | json | 2/12 |
| ECC-VAL-094 | json | 2/12 |
| ECC-VAL-095 | json | 2/18 |
| ECC-VAL-096 | json | 2/14 |
| ECC-VAL-097 | json | 2/18 |
| ECC-VAL-098 | json | 2/21 |
| ECC-VAL-099 | json | 2/9 |
| ECC-VAL-100 | json | 2/23 |
| ECC-VAL-101 | json | 2/70 |
| ECC-VAL-102 | json | 2/200 |
| ECC-VAL-103 | json | 2/10 |
| ECC-VAL-104 | json | 2/15 |
| ECC-VAL-105 | json | 2/9 |
| ECC-VAL-106 | json | 2/29 |
| ECC-VAL-107 | json | 2/176 |
| ECC-VAL-108 | json | 2/37 |
| ECC-VAL-109 | json | 2/4 |
| ECC-VAL-110 | json | 2/7 |
| ECC-VAL-111 | json | 2/4 |
| ECC-VAL-112 | json | 2/8 |
| ECC-VAL-113 | json | 2/13 |
| ECC-VAL-116 | json | 2/17 |
| ECC-VAL-117 | json | 2/3 |
| ECC-VAL-118 | json | 2/3 |

## 10. ECC-original cases (no direct schedule backing)

Stub-derived / extension cases — labelled here and **never presented as schedule-conformant**. Their result stands, but the claim is against our own derivation, not an abstract schedule test case.

- **ECC-EHR-012** Create EHR — reject invalid EHR_STATUS data sets — data-set class 2 (master06 §Test Data Sets, invalid EHR_STATUS shapes); no single master06 test case enumerates class 2
- **ECC-EHR-013** Create anonymous (subject-less) EHR — extension: Anonymous EHRs non-functional capability (master03-profiles §Non-Functional); doubles as class 1.b default-EHR_STATUS coverage; no master06 functional test case
- **ECC-TPL-017** Example COMPOSITION round-trips (ADL 1.4 example → commit) — CNF master04/master15 define no example-generation/commit case; the ITS-REST example operation is non-normative. ECC-derived: asserts the operation's own committable-`required` contract end-to-end (upload OPT → GET example → commit 201).
- **ECC-SQR-001** Store stored query — valid — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-valid (master05:54, A.3.a)
- **ECC-SQR-007** Store stored query — invalid — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-invalid (master05:67, A.3.b)
- **ECC-SQR-006** Store stored query — bad formalism — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-bad_formalism (master05:80, A.3.c)
- **ECC-SQR-008** Stored query existence check — existing — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.has_query-xxx (master05:37, placeholder id; slug descriptivised)
- **ECC-SQR-002** List stored queries — non empty — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY (named list resource, D2 rebind) + AQL 1.1 — I_DEFINITION_QUERY.list_queries-non_empty (master05:110)
- **ECC-SQR-004** List stored queries — empty — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.list_queries-empty (master05:97)
- **ECC-SQR-005** List stored queries — select items — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.list_queries-select_items (master05:123)
- **ECC-QRY-001** Query service smoke test — I_QUERY_SERVICE.smoke_test (master11:48, stub xx flow)
- **ECC-QRY-002** Execute ad-hoc AQL query — empty db — I_QUERY_SERVICE.execute_ad_hoc_query-empty_db (master11:83, A.1.z, stub xx flow)
- **ECC-QRY-003** Execute stored AQL query — empty db — I_QUERY_SERVICE.execute_stored_query-empty_db (master11:61, stub xx flow)
- **ECC-QRY-004** Execute ad-hoc AQL query — loaded db — I_QUERY_SERVICE.execute_ad_hoc_query-loaded_db (master11:96, A.1.a, stub xx flow)
- **ECC-QRY-025** AQL uid projection — c/uid/value returns the version id — schedule stub (master11 is TBD); the loaded-db case asserts only the projected column path — this case asserts the projected CELL equals the committed OBJECT_VERSION_ID (a null cell was a real, otherwise-invisible engine defect)
- **ECC-QRY-005** AQL corpus — invalid queries rejected — schedule stub (master11 is TBD — no invalid-query case); AQL 1.1 negative-rejection evidence
- **ECC-QRY-014** AQL advanced — ORDER BY + LIMIT/OFFSET — schedule stub (master11 is TBD); AQL-advanced ORDER BY + LIMIT/OFFSET, profiles §AQL advanced OPTIONS
- **ECC-QRY-006** AQL corpus — A empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-007** AQL corpus — B empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-008** AQL corpus — C empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-009** AQL corpus — D empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-010** AQL corpus — A loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-011** AQL corpus — B loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-012** AQL corpus — C loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-013** AQL corpus — D loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-015** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-016** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-017** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-018** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-019** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-020** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-021** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-022** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-023** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-024** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-VAL-119** Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected) — no schedule case — ECC-authored negative guard for the corrected all_types fixture (§3, testdata/fixtures/REGISTER.md); the vendored all_types.composition.json carries a day-bearing DV_DATE at a leaf whose OPT C_DATE pattern disallows the day; a spec-correct validator must 422 it (archie is lenient)
- **ECC-DEM-001** Demographic person create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-021** Demographic create bad body — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-002** Demographic person get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-007** Demographic person get absent — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-006** Demographic person get deleted — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-003** Demographic person get by version — schedule stub (master10 §get_party_at_version TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party_at_version + RM common Versioning
- **ECC-DEM-025** Demographic person get at time — schedule stub (master10 §get_party_at_time TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party_at_time + RM common version_at_time
- **ECC-DEM-004** Demographic person update — schedule stub (master10 §update_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.update_party + RM Demographic IM
- **ECC-DEM-008** Demographic person update bad if match — schedule stub (master10 §update_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.update_party + RM Demographic IM
- **ECC-DEM-005** Demographic person delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-009** Demographic agent create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-010** Demographic agent get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-011** Demographic agent delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-012** Demographic group create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-013** Demographic group get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-014** Demographic group delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-015** Demographic organisation create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-016** Demographic organisation get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-017** Demographic organisation delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-018** Demographic role create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-019** Demographic role get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-020** Demographic role delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-022** Demographic versioned party get — extension: VERSIONED_PARTY read (RM common Versioning); no master10 SM operation
- **ECC-DEM-023** Demographic versioned party revision history — extension: REVISION_HISTORY read (RM common Versioning); no master10 SM operation
- **ECC-DEM-024** Demographic person tags — extension: item tags — no openEHR spec governs item tags
- **ECC-DEM-026** Demographic relationship create — schedule stub (master10 §create_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP + RM PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-027** Demographic relationship get — schedule stub (master10 §get_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP + RM PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-028** Demographic relationship get at time — schedule stub (master10 §get_party_relationship_at_time TBD); derived from SM I_PARTY_RELATIONSHIP + RM common version_at_time — ehrbase-rs extension wire
- **ECC-DEM-029** Demographic relationship update — schedule stub (master10 §update_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-030** Demographic relationship delete — schedule stub (master10 §delete_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-031** Demographic relationship get by version — schedule stub (master10 §get_party_relationship_at_version TBD); derived from SM I_PARTY_RELATIONSHIP + RM common OBJECT_VERSION_ID — ehrbase-rs extension wire
- **ECC-ADM-001** Admin EHR delete — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-002** Admin EHR delete absent — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-003** Admin EHR delete idempotent — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-004** Admin EHR delete all — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-005** Admin EHR delete all partial — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-006** Admin EHR delete all (empty selector) — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-007** Admin list contributions — schedule stub (master12 §list_contributions TBD); derived from SM I_ADMIN_SERVICE.list_contributions — native-API-only
- **ECC-ADM-008** Admin contribution count — schedule stub (master12 §contribution_count TBD); derived from SM I_ADMIN_SERVICE.contribution_count — native-API-only
- **ECC-ADM-009** Admin versioned composition count — schedule stub (master12 §versioned_composition_count TBD); derived from SM I_ADMIN_SERVICE.versioned_composition_count — native-API-only
- **ECC-ADM-010** Admin composition version count — schedule stub (master12 §composition_version_count TBD); derived from SM I_ADMIN_SERVICE.composition_version_count — native-API-only
- **ECC-ADM-011** Admin export EHRs (dump/load) — schedule stub (master12 §export_ehrs TBD); derived from SM I_ADMIN_DUMP_LOAD.export_ehrs — native-API-only
- **ECC-ADM-012** Admin archive EHRs — schedule stub (master12 §archive_ehrs TBD); derived from SM I_ADMIN_ARCHIVE.archive_ehrs — native-API-only
- **ECC-ADM-013** Admin physical party delete — schedule stub (master12 §physical_party_delete TBD); derived from SM I_ADMIN_SERVICE.physical_party_delete — demographic-dependent, no ITS-REST binding
- **ECC-ADM-014** Admin archive parties — schedule stub (master12 §archive_parties TBD); derived from SM I_ADMIN_ARCHIVE.archive_parties — demographic-dependent, no ITS-REST binding
- **ECC-MSG-001** EHR Extract — export whole EHR (export_ehrs) — schedule stub (master13 §I_EHR_EXTRACT.export_ehr TBD, listed twice — authoring duplicate); derived from SM I_EHR_EXTRACT_SERVICE.export_ehrs
- **ECC-MSG-002** EHR Extract — spec-driven export (export_ehr_extracts) — schedule stub (master13 §I_EHR_EXTRACT.export_ehr_extracts TBD); derived from SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts
- **ECC-MSG-003** EHR Extract — export of unknown EHR fails — schedule stub (master13 §I_EHR_EXTRACT.export_ehr TBD); derived from SM I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR)
- **ECC-MSG-004** EHR Extract — import whole-EHR clone reusing source id — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying Case 1; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr
- **ECC-MSG-005** EHR Extract — import whole EHR into a caller-fixed id — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr
- **ECC-MSG-006** EHR Extract — import into a duplicate target id fails — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr (duplicate target)
- **ECC-MSG-007** EHR Extract — import extract into an existing EHR — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying Case 2; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr_extract
- **ECC-MSG-008** TDD — import a TDD as a committed COMPOSITION — schedule stub (master13 §I_TDD.import_tdd TBD); derived from SM I_TDD_SERVICE.import_tdd
- **ECC-MSG-009** TDD — import rejects malformed / non-TDD / unknown EHR / unknown template — schedule stub (master13 §I_TDD.import_tdd TBD); derived from SM I_TDD_SERVICE.import_tdd (typed rejections)
- **ECC-MSG-010** TDD — batch import commits all, fail-fast on error — schedule stub (master13 §I_TDD.import_tdds TBD); derived from SM I_TDD_SERVICE.import_tdds
- **ECC-SEC-001** Unauthenticated request to a protected route is refused (401) — no CNF schedule chapter for authentication (out of band per SM master02); Robot SECURITY_TESTS/I_OAuth2_Keycloak §06 'API endpoints are secured' intent, reproduced over Basic auth
- **ECC-SEC-002** Regular credential on an ADMIN-only route is forbidden (403) — no CNF schedule chapter for authorization (out of band per SM master02); Robot SECURITY_TESTS/I_OAuth2_Keycloak role-distinction intent, reproduced over Basic auth
- **ECC-SIG-001** Version signing — digest present — extension: VERSION.signature is an ehrbase-rs feature (no openEHR spec governs the digest algorithm); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-002** Version signing — digest recomputes — extension: sha256: digest recompute is an ehrbase-rs feature (RFC 8785 canonical form); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-003** Version signing — all kinds — extension: version signing rides every versioned-object write (ehrbase-rs); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-004** Version signing — client verbatim — extension: client-supplied signatures win (ehrbase-rs); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-005** Version signing — pgp verifies — extension: pgp signing mode is an ehrbase-rs feature (RFC 4880); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-TS-001** TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-002** TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-003** TERMINOLOGY expand (bundle) — explicit code merged with the expansion — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-004** TERMINOLOGY expand — unknown value set rejected (400) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-005** TERMINOLOGY expand — unknown service_api rejected (400) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-006** TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the FHIR service_api path realizes the spec mechanism (generic, not an extension)
- **ECC-TS-007** TERMINOLOGY expand (FHIR) — terminology-server timeout is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-TS-008** TERMINOLOGY expand (FHIR) — terminology-server 5xx is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-TS-009** TERMINOLOGY expand (FHIR) — malformed terminology response is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-SF-001** FLAT commit then read-back as FLAT, canonical JSON, and STRUCTURED — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-002** STRUCTURED commit then read-back as STRUCTURED, canonical JSON, and FLAT — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-003** Accept q-values select the highest-weight simplified format; every non-204 carries Content-Type — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-004** Deprecated + legacy simplified media types are rejected on Accept (406) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-005** Deprecated + legacy simplified media types are rejected on write Content-Type (415) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-006** FLAT commit without openehr-template-id (and no payload template id) → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-007** FLAT commit with an unknown field identifier → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-008** FLAT commit with |other combined with |code on one coded leaf → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-009** GET a template as a Web Template document (Accept application/openehr.wt+json) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-010** GET a template example in each of the four Accept forms (json, xml, flat, structured) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-011** GET a template example with an unsupported Accept → 406 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-012** CONTRIBUTION with a FLAT COMPOSITION inner payload: canonical envelope in, simplified read-back — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-013** EHR_STATUS has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-014** DIRECTORY (FOLDER) has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-015** Demographic PARTY has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-016** FLAT ctx/time sets EVENT_CONTEXT.start_time; ctx/setting defaults to openehr::238 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-ADL2-001** Upload a valid ADL2 template → 201 with Location; Prefer selects minimal/representation/identifier bodies — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-002** Upload the same ADL2 HRID twice → the second is a 409 conflict — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-003** Upload an unparseable ADL2 source → 422 carrying syntax rule codes in validationErrors — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-004** Upload a semantically invalid ADL2 template (missing description) → 422 with the AOM2 rule code VARD — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-005** Upload a parent archetype, then a specialised child that validates against the stored parent → 201 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-006** Get an ADL2 template as text/plain source, application/json OperationalTemplateV2, and 406 on xml-only — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-007** Get an unknown ADL2 template_id → 404 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-008** Version get resolves an exact SEMVER and a major prefix (latest match) → 200; an unknown version → 404 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-009** Get a template example in each of the four Accept_LOCATABLE forms → 200; the JSON form is a COMPOSITION rooted at the template's archetype — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-010** Example honours the detail_level enum (required/medium/complete) and rejects a bad type/detail_level with 400 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-011** Example for an unknown template_id → 404; an Accept outside the four LOCATABLE forms → 406 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-012** List ADL2 templates → TemplateMetadata carrying template_id, concept, archetype_id, created_timestamp — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-AQT-001** TERMINOLOGY('expand') as a matches operand filters committed compositions by the value set's codes — the CNF Platform Conformance Test Schedule defines no terminology-function test case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS).
- **ECC-AQT-002** A non-expand TERMINOLOGY operation as a matches operand (lookup/map) → 400 — the CNF Platform Conformance Test Schedule defines no terminology-function test case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS).
- **ECC-AQT-003** TERMINOLOGY() in an unsupported position (a SELECT column) → 400 — the CNF Platform Conformance Test Schedule defines no terminology-function test case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS).
- **ECC-AQT-004** A Boolean TERMINOLOGY assertion with an unsupported operation (lookup) → 400 — the CNF Platform Conformance Test Schedule defines no terminology-function test case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS).

## 11. Detailed test report

| ECC id | Capability | Format | Data sets | Rung | Result |
|---|---|---|--:|---|---|
| ECC-EHR-001 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-002 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-003 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-004 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-005 | EhrOperations | json | 16/16 | — | PASS |
| ECC-EHR-006 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-007 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-008 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-009 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-010 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-011 | EhrOperations | json | 1/1 | — | PASS |
| ECC-STA-001 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-002 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-003 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-004 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-005 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-006 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-007 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-008 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-009 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-010 | EhrStatus | json | 1/1 | — | PASS |
| ECC-EHR-012 | EhrOperations | json | 11/11 | — | PASS |
| ECC-EHR-013 | AnonymousEhrs | json | 1/1 | — | PASS |
| ECC-COM-001 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-001 | CompositionOps | xml | 1/1 | — | PASS |
| ECC-COM-002 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-002 | CompositionOps | xml | 1/1 | — | PASS |
| ECC-COM-003 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-004 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-005 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-006 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-007 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-032 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-011 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-012 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-008 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-008 | CompositionOps | xml | 1/1 | — | PASS |
| ECC-COM-009 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-010 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-013 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-013 | CompositionOps | xml | 1/1 | — | PASS |
| ECC-COM-014 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-014 | CompositionOps | xml | 1/1 | — | PASS |
| ECC-COM-015 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-016 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-017 | CompositionOps | json | 3/3 | — | PASS |
| ECC-COM-018 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-018 | CompositionOps | xml | 1/1 | — | PASS |
| ECC-COM-019 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-020 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-021 | CompositionOps | json | 2/2 | — | PASS |
| ECC-COM-022 | Versioning | json | 1/1 | — | PASS |
| ECC-COM-022 | Versioning | xml | 1/1 | — | PASS |
| ECC-COM-023 | Versioning | json | 1/1 | — | PASS |
| ECC-COM-024 | Versioning | json | 1/1 | — | PASS |
| ECC-COM-025 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-026 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-027 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-028 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-029 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-030 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-031 | CompositionOps | json | 1/1 | — | PASS |
| ECC-CTB-001 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-002 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-003 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-004 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-005 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-006 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-007 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-008 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-009 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-010 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-011 | ChangeSets | json | 15/15 | — | PASS |
| ECC-CTB-012 | ChangeSets | json | 15/15 | — | PASS |
| ECC-CTB-013 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-014 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-015 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-016 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-017 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-018 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-019 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-020 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-021 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-022 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-023 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-024 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-025 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-026 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-027 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-028 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-029 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-030 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-031 | ChangeSets | json | 1/1 | — | PASS |
| ECC-DIR-012 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-013 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-014 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-015 | DirectoryOps | json | 2/2 | — | PASS |
| ECC-DIR-016 | DirectoryOps | json | 5/5 | — | PASS |
| ECC-DIR-017 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-018 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-001 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-002 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-003 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-022 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-004 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-023 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-005 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-006 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-007 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-008 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-009 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-010 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-011 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-019 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-020 | DirectoryOps | json | 2/2 | — | PASS |
| ECC-DIR-021 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-024 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-025 | DirectoryOps | json | 3/3 | — | PASS |
| ECC-DIR-026 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-027 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-028 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-029 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-030 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-031 | DirectoryOps | json | 2/2 | — | PASS |
| ECC-DIR-032 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-033 | Versioning | json | 1/1 | — | PASS |
| ECC-DIR-034 | Versioning | json | 2/2 | — | PASS |
| ECC-DIR-035 | Versioning | json | 1/1 | — | PASS |
| ECC-DIR-036 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-037 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-TPL-011 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-012 | Adl14OptProvisioning | json | 18/18 | — | PASS |
| ECC-TPL-001 | Adl14ArchetypeProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-002 | Adl14OptProvisioning | json | 18/18 | — | PASS |
| ECC-TPL-004 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-005 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-006 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-009 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-007 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-008 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-010 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-003 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-017 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-014 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-015 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-016 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-013 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-001 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-007 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-006 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-008 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-002 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-004 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-005 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-QRY-001 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-002 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-003 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-004 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-025 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-005 | AqlBasic | json | 2/2 | — | PASS |
| ECC-QRY-014 | AqlAdvanced | json | 1/1 | — | PASS |
| ECC-QRY-006 | AqlBasic | json | 24/24 | — | PASS |
| ECC-QRY-007 | AqlBasic | json | 17/17 | — | PASS |
| ECC-QRY-008 | AqlBasic | json | 10/10 | — | PASS |
| ECC-QRY-009 | AqlBasic | json | 16/16 | — | PASS |
| ECC-QRY-010 | AqlBasic | json | 20/20 | — | PASS |
| ECC-QRY-011 | AqlBasic | json | 14/14 | — | PASS |
| ECC-QRY-012 | AqlBasic | json | 6/6 | — | PASS |
| ECC-QRY-013 | AqlBasic | json | 7/7 | — | PASS |
| ECC-QRY-015 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-016 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-017 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-018 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-019 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-020 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-021 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-022 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-023 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-024 | AqlBasic | json | 1/1 | — | PASS |
| ECC-VAL-001 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-002 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-003 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-004 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-005 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-006 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-007 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-008 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-009 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-010 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-011 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-012 | ArchetypeValidation | json | 6/6 | — | PASS |
| ECC-VAL-013 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-014 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-015 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-016 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-017 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-018 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-019 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-020 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-021 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-022 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-023 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-024 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-025 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-026 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-027 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-028 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-029 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-030 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-031 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-032 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-033 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-034 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-035 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-036 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-037 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-038 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-039 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-040 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-041 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-042 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-043 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-044 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-045 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-046 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-047 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-048 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-049 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-050 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-051 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-052 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-053 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-054 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-055 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-056 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-057 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-058 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-059 | ArchetypeValidation | json | 3/3 | — | PASS |
| ECC-VAL-060 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-061 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-062 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-063 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-064 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-065 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-066 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-067 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-068 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-069 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-070 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-071 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-072 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-073 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-074 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-075 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-076 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-077 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-078 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-079 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-080 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-081 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-082 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-083 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-084 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-085 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-086 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-087 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-088 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-089 | ArchetypeValidation | json | 4/4 | — | PASS |
| ECC-VAL-090 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-091 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-092 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-093 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-094 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-095 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-096 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-097 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-098 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-099 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-100 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-101 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-102 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-103 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-104 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-105 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-119 | ArchetypeValidation | json | 1/1 | — | PASS |
| ECC-VAL-106 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-107 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-108 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-109 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-110 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-111 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-112 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-113 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-114 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-115 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-116 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-117 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-VAL-118 | ArchetypeValidation | json | 2/2 | — | PASS |
| ECC-DEM-001 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-021 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-002 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-007 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-006 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-003 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-025 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-004 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-008 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-005 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-009 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-010 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-011 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-012 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-013 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-014 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-015 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-016 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-017 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-018 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-019 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-020 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-022 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-023 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-024 | PartyOperations | json | 1/1 | — | PASS |
| ECC-DEM-026 | PartyRelationshipOperations | json | 1/1 | — | PASS |
| ECC-DEM-027 | PartyRelationshipOperations | json | 1/1 | — | PASS |
| ECC-DEM-028 | PartyRelationshipOperations | json | 1/1 | — | PASS |
| ECC-DEM-029 | PartyRelationshipOperations | json | 1/1 | — | PASS |
| ECC-DEM-030 | PartyRelationshipOperations | json | 1/1 | — | PASS |
| ECC-DEM-031 | PartyRelationshipOperations | json | 1/1 | — | PASS |
| ECC-ADM-001 | AdminPhysicalDeletion | json | 1/1 | — | PASS |
| ECC-ADM-002 | AdminPhysicalDeletion | json | 1/1 | — | PASS |
| ECC-ADM-003 | AdminPhysicalDeletion | json | 1/1 | — | PASS |
| ECC-ADM-004 | AdminPhysicalDeletion | json | 1/1 | — | PASS |
| ECC-ADM-005 | AdminPhysicalDeletion | json | 1/1 | — | PASS |
| ECC-ADM-006 | AdminPhysicalDeletion | json | 1/1 | — | PASS |
| ECC-ADM-007 | AdminActivityReport | json | 0/0 | — | N/A |
| ECC-ADM-008 | AdminActivityReport | json | 0/0 | — | N/A |
| ECC-ADM-009 | AdminActivityReport | json | 0/0 | — | N/A |
| ECC-ADM-010 | AdminActivityReport | json | 0/0 | — | N/A |
| ECC-ADM-011 | AdminEhrDumpLoad | json | 0/0 | — | N/A |
| ECC-ADM-012 | AdminEhrArchive | json | 0/0 | — | N/A |
| ECC-ADM-013 | AdminPhysicalDeletion | json | 0/0 | — | N/A |
| ECC-ADM-014 | AdminDemographicArchive | json | 0/0 | — | N/A |
| ECC-MSG-001 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-002 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-003 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-004 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-005 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-006 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-007 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-008 | MessagingTds | json | 0/0 | — | N/A |
| ECC-MSG-009 | MessagingTds | json | 0/0 | — | N/A |
| ECC-MSG-010 | MessagingTds | json | 0/0 | — | N/A |
| ECC-SEC-001 | Authentication | json | 1/1 | — | PASS |
| ECC-SEC-002 | Authentication | json | 1/1 | — | PASS |
| ECC-SIG-001 | Signing | json | 1/1 | — | PASS |
| ECC-SIG-001 | Signing | xml | 1/1 | — | PASS |
| ECC-SIG-002 | Signing | json | 1/1 | — | PASS |
| ECC-SIG-003 | Signing | json | 4/4 | — | PASS |
| ECC-SIG-004 | Signing | json | 1/1 | — | PASS |
| ECC-SIG-005 | Signing | json | 1/1 | — | PASS |
| ECC-TS-001 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-002 | Terminology | json | 2/2 | — | PASS |
| ECC-TS-003 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-004 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-005 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-006 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-007 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-008 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-009 | Terminology | json | 1/1 | — | PASS |
| ECC-SF-001 | SimplifiedFormats | json | 3/3 | — | PASS |
| ECC-SF-002 | SimplifiedFormats | json | 3/3 | — | PASS |
| ECC-SF-003 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-004 | SimplifiedFormats | json | 8/8 | — | PASS |
| ECC-SF-005 | SimplifiedFormats | json | 8/8 | — | PASS |
| ECC-SF-006 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-007 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-008 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-009 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-010 | SimplifiedFormats | json | 4/4 | — | PASS |
| ECC-SF-011 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-012 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-013 | SimplifiedFormats | json | 2/2 | — | PASS |
| ECC-SF-014 | SimplifiedFormats | json | 2/2 | — | PASS |
| ECC-SF-015 | SimplifiedFormats | json | 2/2 | — | PASS |
| ECC-SF-016 | SimplifiedFormats | json | 2/2 | — | PASS |
| ECC-ADL2-001 | Adl2Provisioning | json | 3/3 | — | PASS |
| ECC-ADL2-002 | Adl2Provisioning | json | 1/1 | — | PASS |
| ECC-ADL2-003 | Adl2Provisioning | json | 1/1 | — | PASS |
| ECC-ADL2-004 | Adl2Provisioning | json | 1/1 | — | PASS |
| ECC-ADL2-005 | Adl2Provisioning | json | 2/2 | — | PASS |
| ECC-ADL2-006 | Adl2Provisioning | json | 3/3 | — | PASS |
| ECC-ADL2-007 | Adl2Provisioning | json | 1/1 | — | PASS |
| ECC-ADL2-008 | Adl2Provisioning | json | 3/3 | — | PASS |
| ECC-ADL2-009 | Adl2Provisioning | json | 4/4 | — | PASS |
| ECC-ADL2-010 | Adl2Provisioning | json | 5/5 | — | PASS |
| ECC-ADL2-011 | Adl2Provisioning | json | 2/2 | — | PASS |
| ECC-ADL2-012 | Adl2Provisioning | json | 1/1 | — | PASS |
| ECC-AQT-001 | AqlTerminology | json | 1/1 | — | PASS |
| ECC-AQT-002 | AqlTerminology | json | 2/2 | — | PASS |
| ECC-AQT-003 | AqlTerminology | json | 1/1 | — | PASS |
| ECC-AQT-004 | AqlTerminology | json | 1/1 | — | PASS |

## 12. Terminology server (TS area)

- Server: `http://host.docker.internal:8099`
- Mode: fixture

Recorded FHIR-tx exchange (8 request(s)):

| # | Method | Path | Query |
|--:|---|---|---|
| 1 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface |
| 2 | GET | `/ValueSet/$validate-code` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface&code=B |
| 3 | GET | `/CodeSystem/$lookup` | code=B |
| 4 | GET | `/CodeSystem/$subsumes` | codeA=L&codeB=O |
| 5 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface |
| 6 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fehrbase.invalid%2Ffault%2Ftimeout |
| 7 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fehrbase.invalid%2Ffault%2Fserver-error |
| 8 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fehrbase.invalid%2Ffault%2Fmalformed |
