# ehrbase-java — Conformance Statement (generated)

> Generated from a conformance run (`results.json`) — never hand-asserted. Every
> claim below is a pure function of the machine profile verdicts.

## System under test

| Field | Value |
|---|---|
| Product | ehrbase-java EHRbase upstream |
| SUT class | foreign (comparison data) |
| Base URL | `http://localhost:8091/ehrbase/rest/openehr/v1` |
| Auth mode | basic |
| Edition policy | auto (ladder: newest form first, step down) |
| Run started | 2026-07-13T15:05:42.186992Z |
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

The SUT satisfied the normative core of some assertions only at a rung below the newest edition:

| Satisfied rung | Cases |
|---|--:|
| release-1.0.3 | 59 |

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
| Adl2Provisioning | OPTIONS (optional) | no cases |
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
| AqlTerminology | OPTIONS (optional) | no cases |
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

- **ECC-DEM-001** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-021** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-002** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-007** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-006** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-003** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-025** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-004** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-008** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-005** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-009** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-010** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-011** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-012** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-013** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-014** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-015** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-016** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-017** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-018** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-019** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-020** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-022** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-023** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-024** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-026** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-027** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-028** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-029** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-030** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-DEM-031** — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: docs/plans/x1-comparison.md §2a/§2c (upstream has no demographic API); docs/architecture.md (RM demographic — own wire design))_
- **ECC-SIG-001** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: docs/plans/x1-comparison.md §2a (Sig — version signing is ours); docs/design/version-signing.md)_
- **ECC-SIG-002** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: docs/plans/x1-comparison.md §2a (Sig — version signing is ours); docs/design/version-signing.md)_
- **ECC-SIG-003** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: docs/plans/x1-comparison.md §2a (Sig — version signing is ours); docs/design/version-signing.md)_
- **ECC-SIG-004** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: docs/plans/x1-comparison.md §2a (Sig — version signing is ours); docs/design/version-signing.md)_
- **ECC-SIG-005** — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: docs/plans/x1-comparison.md §2a (Sig — version signing is ours); docs/design/version-signing.md)_

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
| SutConfig: FHIR terminology provider not exercisable — the SUT answered 400 to a `hl7.org/fhir/4.0` expand (a configured provider lacking the fixture value set, or a non-provider rejection). Not a fabricated pass. | 1 |
| SutConfig: the 5xx fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:64711 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_server_error_is_5xx + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception. | 1 |
| SutConfig: the malformed fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:64711 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_malformed_is_not_json + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception. | 1 |
| SutConfig: the timeout fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:64711 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception. | 1 |
| all 11 C/loaded_db goldens are dialect-routed or require id-substitution/binds | 1 |
| destructive case runs only against disposable composed SUTs (an empty ehr_id selector deletes ALL EHRs); skipped for a foreign / bring-your-own endpoint | 1 |
| master04 §delete_opt: SM I_DEFINITION_ADL14.delete_opt() has no ITS-REST ADL 1.4 binding — deletion lives in the ADMIN API only; a 405 here would be a schedule-vs-ITS-REST gap, not a server defect (register 01 G-5 / D2). The ADMIN template-deletion path is evidenced in the Admin area. | 4 |
| master05 §list_queries: SM I_DEFINITION_QUERY.list_queries() (bare collection) has no ITS-REST binding — Release-1.0.3 and development@e8a093e expose GET /definition/query/{qualified_query_name}, not a bare GET /definition/query. An edition exposing a bare-list resource would make this case live (register 02 G-2 edition probe). | 2 |
| master08 §list_contributions: the SM operation I_EHR_CONTRIBUTION.list_contributions() has no ITS-REST binding — /ehr/{ehr_id}/contribution is POST-only (no GET collection resource) in the tested development@e8a093e OAS and in Release-1.0.3; the list is a native-API concern, not wire-exercisable | 5 |

## Selection

| Field | Value |
|---|---|
| Profile filter | all |
| Id filter | — |
| Formats | json, xml |

