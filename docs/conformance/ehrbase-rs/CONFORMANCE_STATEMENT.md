# ehrbase-rs — Conformance Statement (generated)

> Generated from a conformance run (`results.json`) — never hand-asserted. Every
> claim below is a pure function of the machine profile verdicts.

## System under test

| Field | Value |
|---|---|
| Product | ehrbase-rs ehrbase-rs 3.1.1 |
| SUT class | ours (ehrbase-rs) |
| Base URL | `http://localhost:8080/ehrbase/rest/openehr/v1` |
| Auth mode | basic |
| Edition policy | pinned (development) |
| Run started | 2026-07-18T14:19:21.017017Z |
| Reference corpus | openEHR/specifications-CNF@33251d2a |

## Supported specification versions

| Specification | Version |
|---|---|
| Reference Model (RM) | 1.2.0 |
| ITS-REST contract | development@e8a093e |
| AQL (QUERY) | 1.1.0 |
| Terminology (TERM) | 3.1.0 |

> CNF requires the Conformance Statement to state the supported RM version(s); the minimum required is RM 1.0.2 (`master03-overview.adoc` §46). This SUT declares **RM 1.2.0**.


### Discovered edition profile

Every laddered assertion matched the newest edition form — no lower-rung findings.

## External data formats

Declared: XML, JSON (`master03-profiles.adoc` §Other Non-Functional). This run exercised: json, xml.


## Capability scope

| Capability | Required in | Result |
|---|---|---|
| Adl14ArchetypeProvisioning | CORE (required) | **FAIL** |
| Adl14OptProvisioning | CORE (required) | **FAIL** |
| Adl2Provisioning | OPTIONS (optional) | no cases |
| EhrOperations | CORE (required) | **FAIL** |
| EhrStatus | CORE (required) | **FAIL** |
| CompositionOps | CORE (required) | **FAIL** |
| ChangeSets | CORE (required) | **FAIL** |
| Versioning | CORE (required) | **FAIL** |
| ArchetypeValidation | CORE (required) | **FAIL** |
| DirectoryOps | STANDARD (required) | **FAIL** |
| QueryProvisioning | STANDARD (required) | **FAIL** |
| AqlBasic | STANDARD (required) | **FAIL** |
| AqlAdvanced | OPTIONS (optional) | **FAIL** |
| AqlTerminology | OPTIONS (optional) | no cases |
| PartyOperations | OPTIONS (optional) | **FAIL** |
| PartyRelationshipOperations | OPTIONS (optional) | **FAIL** |
| AdminActivityReport | OPTIONS (optional) | not evidenced |
| AdminPhysicalDeletion | OPTIONS (optional) | **FAIL** |
| AdminEhrDumpLoad | OPTIONS (optional) | not evidenced |
| AdminBulkEhrLoad | OPTIONS (optional) | no cases |
| AdminEhrArchive | OPTIONS (optional) | not evidenced |
| AdminDemographicArchive | OPTIONS (optional) | not evidenced |
| MessagingEhrExtract | OPTIONS (optional) | not evidenced |
| MessagingTds | OPTIONS (optional) | not evidenced |
| Signing | STANDARD (required) | **FAIL** |
| AnonymousEhrs | CORE (required) | **FAIL** |
| Authentication | reported (non-gating) | **FAIL** |
| Terminology | reported (non-gating) | **FAIL** |
| SimplifiedFormats | OPTIONS (optional) | **FAIL** |

## Profile claims (machine-computed)

| Profile | Aggregation | Result |
|---|---|---|
| Core | all listed capabilities | not claimable |
| Standard | all listed capabilities | not claimable |
| Options | any optional capability | not obtained |

### OPTIONS — obtained optional capabilities

_None obtained in this run._

## Adjudicated skips and not-applicable cases

### Skipped (adjudicated / structural), by reason

| Reason | Cases |
|---|--:|
| NativeApiOnly: I_ADMIN_ARCHIVE.archive_ehrs is exercised by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs is exercised by app/ehrbase/tests/service_dump_load.rs::export_then_load_into_fresh_db_round_trips_byte_equal — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_SERVICE.composition_version_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_SERVICE.contribution_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_SERVICE.list_contributions is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_ADMIN_SERVICE.versioned_composition_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding | 1 |
| NoRestBinding: I_ADMIN_ARCHIVE.archive_parties has no ITS-REST route and acts on the demographic extension; the archive path is proven natively by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged | 1 |
| NoRestBinding: I_ADMIN_SERVICE.physical_party_delete has no ITS-REST route and acts on the demographic extension; exercised natively by app/ehrbase/tests/service_admin.rs::physical_party_delete_cascades_relationships_and_spares_partner | 1 |
| SutConfig: server not in `pgp` mode (needs a configured OpenPGP key); a pgp-keyed compose profile is a follow-up — the digest cases prove the Signing capability | 1 |
| SutConfig: the 5xx fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:55326 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_server_error_is_5xx + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception. | 1 |
| SutConfig: the malformed fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:55326 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_malformed_is_not_json + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception. | 1 |
| SutConfig: the timeout fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:55326 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception. | 1 |
| all 11 C/loaded_db goldens are dialect-routed or require id-substitution/binds | 1 |
| master04 §delete_opt: SM I_DEFINITION_ADL14.delete_opt() has no ITS-REST ADL 1.4 binding — deletion lives in the ADMIN API only; a 405 here would be a schedule-vs-ITS-REST gap, not a server defect (register 01 G-5 / D2). The ADMIN template-deletion path is evidenced in the Admin area. | 4 |
| master05 §list_queries: SM I_DEFINITION_QUERY.list_queries() (bare collection) has no ITS-REST binding — Release-1.0.3 and development@e8a093e expose GET /definition/query/{qualified_query_name}, not a bare GET /definition/query. An edition exposing a bare-list resource would make this case live (register 02 G-2 edition probe). | 2 |
| master08 §list_contributions: the SM operation I_EHR_CONTRIBUTION.list_contributions() has no ITS-REST binding — /ehr/{ehr_id}/contribution is POST-only (no GET collection resource) in the tested development@e8a093e OAS and in Release-1.0.3; the list is a native-API concern, not wire-exercisable | 5 |

## Selection

| Field | Value |
|---|---|
| Profile filter | all |
| Id filter | — |
| Formats | json, xml |

