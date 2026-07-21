# ehrbase-java — Conformance Statement (generated)

> Generated from a conformance run (`results.json`) — never hand-asserted. Every
> claim below is a pure function of the machine profile verdicts.

## System under test

| Field | Value |
|---|---|
| Product | ehrbase-java EHRbase 2.34.0 |
| SUT class | foreign (comparison data) |
| Base URL | `http://localhost:8091/ehrbase/rest/openehr/v1` |
| Auth mode | basic |
| Edition policy | auto (ladder: newest form first, step down) |
| Run started | 2026-07-21T04:52:00.770002Z |
| Reference corpus | openEHR/specifications-CNF@33251d2a |

## Supported specification versions

| Specification | Version |
|---|---|
| Reference Model (RM) | 1.2.0 |
| ITS-REST contract | Release-1.1.0 |
| AQL (QUERY) | 1.1.0 |
| Terminology (TERM) | 3.1.0 |

> CNF requires the Conformance Statement to state the supported RM version(s); the minimum required is RM 1.0.2 (`master03-overview.adoc` §46). This SUT declares **RM 1.2.0**.


### Discovered edition profile

The SUT satisfied the normative core of some assertions only at a rung below the newest edition:

| Satisfied rung | Cases |
|---|--:|
| release-1.0.3 | 62 |

Observations:

- release-1.0.3: C.10 COMPOSITION references a non-existent OPT: status 400 (release-1.0.3)
- release-1.0.3: C.2 invalid COMPOSITION (composer removed): status 400 (release-1.0.3)
- release-1.0.3: C.3 empty CONTRIBUTION (no VERSIONs): status 400 (release-1.0.3)
- release-1.0.3: C.4 mixed valid+invalid commit rejected: status 400 (release-1.0.3)
- release-1.0.3: C.8 second commit invalid content: status 400 (release-1.0.3)
- release-1.0.3: ETag emitted in the deprecated bare form
- release-1.0.3: H.2 update a non-existent directory: status 412 (release-1.0.3)
- release-1.0.3: I.1 delete a non-existent directory: status 412 (release-1.0.3)

## External data formats

Declared: XML, JSON (`master03-profiles.adoc` §Other Non-Functional). This run exercised: json, xml.


## Capability scope

| Capability | Required in | Result |
|---|---|---|
| Adl14ArchetypeProvisioning | CORE (required) | pass |
| Adl14OptProvisioning | CORE (required) | **FAIL** |
| Adl2Provisioning | OPTIONS (optional) | **FAIL** |
| EhrOperations | CORE (required) | **FAIL** |
| EhrStatus | CORE (required) | **FAIL** |
| CompositionOps | CORE (required) | **FAIL** |
| ChangeSets | CORE (required) | **FAIL** |
| Versioning | CORE (required) | pass |
| ArchetypeValidation | CORE (required) | **FAIL** |
| DirectoryOps | STANDARD (required) | pass |
| QueryProvisioning | STANDARD (required) | pass |
| AqlBasic | STANDARD (required) | **FAIL** |
| AqlAdvanced | OPTIONS (optional) | pass |
| AqlTerminology | OPTIONS (optional) | **FAIL** |
| PartyOperations | OPTIONS (optional) | not evidenced |
| PartyRelationshipOperations | OPTIONS (optional) | not evidenced |
| AdminActivityReport | OPTIONS (optional) | not evidenced |
| AdminPhysicalDeletion | OPTIONS (optional) | **FAIL** |
| AdminEhrDumpLoad | OPTIONS (optional) | not evidenced |
| AdminBulkEhrLoad | OPTIONS (optional) | no cases |
| AdminEhrArchive | OPTIONS (optional) | not evidenced |
| AdminDemographicArchive | OPTIONS (optional) | not evidenced |
| MessagingEhrExtract | OPTIONS (optional) | not evidenced |
| MessagingTds | OPTIONS (optional) | not evidenced |
| Signing | STANDARD (required) | not evidenced |
| AnonymousEhrs | CORE (required) | pass |
| Authentication | reported (non-gating) | pass |
| Terminology | reported (non-gating) | **FAIL** |
| SimplifiedFormats | OPTIONS (optional) | **FAIL** |

## Profile claims (machine-computed)

| Profile | Aggregation | Result |
|---|---|---|
| Core | all listed capabilities | not claimable |
| Standard | all listed capabilities | not claimable |
| Options | any optional capability | OBTAINED |

### OPTIONS — obtained optional capabilities

- AqlAdvanced

## Adjudicated skips and not-applicable cases

### Not applicable (fairness register)

- **ECC-DEM-001** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-021** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-002** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-007** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-006** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-003** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-025** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-004** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-008** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-005** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-009** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-010** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-011** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-012** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-013** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-014** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-015** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-016** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-017** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-018** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-019** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-020** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-022** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-023** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-024** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-026** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-027** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-028** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-029** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-030** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-031** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-ADM-007** — NativeApiOnly: I_ADMIN_SERVICE.list_contributions is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §list_contributions (TBD stub); SM I_ADMIN_SERVICE.list_contributions — no ITS-REST admin route)_
- **ECC-ADM-008** — NativeApiOnly: I_ADMIN_SERVICE.contribution_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §contribution_count (TBD stub); SM I_ADMIN_SERVICE.contribution_count — no ITS-REST admin route)_
- **ECC-ADM-009** — NativeApiOnly: I_ADMIN_SERVICE.versioned_composition_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §versioned_composition_count (TBD stub); SM I_ADMIN_SERVICE.versioned_composition_count — no ITS-REST admin route)_
- **ECC-ADM-010** — NativeApiOnly: I_ADMIN_SERVICE.composition_version_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §composition_version_count (TBD stub); SM I_ADMIN_SERVICE.composition_version_count — no ITS-REST admin route)_
- **ECC-ADM-011** — NativeApiOnly: I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs is exercised by app/ehrbase/tests/service_dump_load.rs::export_then_load_into_fresh_db_round_trips_byte_equal — no ITS-REST admin route reaches it _(cite: CNF master12 §export_ehrs (TBD stub); SM I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs — no ITS-REST admin route)_
- **ECC-ADM-012** — NativeApiOnly: I_ADMIN_ARCHIVE.archive_ehrs is exercised by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged — no ITS-REST admin route reaches it _(cite: CNF master12 §archive_ehrs (TBD stub); SM I_ADMIN_ARCHIVE.archive_ehrs — no ITS-REST admin route)_
- **ECC-ADM-013** — NoRestBinding: I_ADMIN_SERVICE.physical_party_delete has no ITS-REST route and acts on the demographic extension; exercised natively by app/ehrbase/tests/service_admin.rs::physical_party_delete_cascades_relationships_and_spares_partner _(cite: CNF master12 §physical_party_delete (TBD stub); SM I_ADMIN_SERVICE.physical_party_delete acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route)_
- **ECC-ADM-014** — NoRestBinding: I_ADMIN_ARCHIVE.archive_parties has no ITS-REST route and acts on the demographic extension; the archive path is proven natively by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged _(cite: CNF master12 §archive_parties (TBD stub); SM I_ADMIN_ARCHIVE.archive_parties acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route)_
- **ECC-MSG-001** — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehrs; RM EHR Extract IM (X_VERSIONED_*); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub, listed twice — authoring duplicate))_
- **ECC-MSG-002** — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts (EXTRACT_ENTITY_MANIFEST + EXTRACT_VERSION_SPEC); CNF master13 §I_EHR_EXTRACT.export_ehr_extracts (TBD stub))_
- **ECC-MSG-003** — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehrs (ehr_id_does_not_exist precondition); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub))_
- **ECC-MSG-004** — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr; RM common master06 §Copying Case 1 (reuse source EHR identifier); CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-005** — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr (same patient in another EHR service); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-006** — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr (ehr_create_fail_duplicate_id); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-007** — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr_extract; RM common master06 §Copying Case 2 (first receipt clones VERSIONED_OBJECT; re-import is a conflict); CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-008** — NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdd; TDD → COMPOSITION over OPT/WebTemplate (openehr_flat::tdd::from_tdd); CNF master13 §I_TDD.import_tdd (TBD stub))_
- **ECC-MSG-009** — NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdd (typed envelope rejections); CNF master13 §I_TDD.import_tdd (TBD stub))_
- **ECC-MSG-010** — NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdds; CNF master13 §I_TDD.import_tdds (TBD stub))_
- **ECC-SIG-001** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_
- **ECC-SIG-002** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_
- **ECC-SIG-003** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_
- **ECC-SIG-004** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_
- **ECC-SIG-005** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_

### Skipped (adjudicated / structural), by reason

| Reason | Cases |
|---|--:|
| SutConfig: FHIR terminology provider not exercisable — the SUT answered 400 to a `hl7.org/fhir/4.0` expand (a configured provider lacking the fixture value set, or a non-provider rejection). Not a fabricated pass. | 1 |
| SutConfig: the 5xx fault requires a fault-injecting terminology server wired to the SUT (the composed run points the SUT's [terminology.external] provider at the fixture via host.docker.internal); this run is not wired. Harness tx server: http://127.0.0.1:59779 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_server_error_is_5xx + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception. | 1 |
| SutConfig: the malformed fault requires a fault-injecting terminology server wired to the SUT (the composed run points the SUT's [terminology.external] provider at the fixture via host.docker.internal); this run is not wired. Harness tx server: http://127.0.0.1:59779 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_malformed_is_not_json + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception. | 1 |
| SutConfig: the timeout fault requires a fault-injecting terminology server wired to the SUT (the composed run points the SUT's [terminology.external] provider at the fixture via host.docker.internal); this run is not wired. Harness tx server: http://127.0.0.1:59779 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception. | 1 |
| destructive case runs only against disposable composed SUTs (an empty ehr_id selector deletes ALL EHRs); skipped for a foreign / bring-your-own endpoint | 1 |

## Selection

| Field | Value |
|---|---|
| Profile filter | all |
| Id filter | — |
| Formats | json, xml |

