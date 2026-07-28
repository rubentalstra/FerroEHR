# Conformance Report

SUT: ehrbase-rs 3.11.0 · schedule cnf-2.0-w2 · ITS its-rest
Runner: cnf-runner 3.11.0 · verification pack: passed

## Summary

| Status | Count |
| --- | --- |
| passed | 694 |
| failed | 0 |
| errored | 0 |
| skipped | 0 |
| not_applicable | 63 |
| total | 757 |

## By chapter

| Chapter | passed | failed | errored | skipped | not_applicable |
| --- | --- | --- | --- | --- | --- |
| CONT | 123 | 0 | 0 | 0 | 0 |
| I_ADMIN_ARCHIVE | 0 | 0 | 0 | 0 | 4 |
| I_ADMIN_DUMP_LOAD | 0 | 0 | 0 | 0 | 2 |
| I_ADMIN_SERVICE | 9 | 0 | 0 | 0 | 10 |
| I_DEFINITION_ADL14 | 22 | 0 | 0 | 0 | 6 |
| I_DEFINITION_ADL2 | 21 | 0 | 0 | 0 | 8 |
| I_DEFINITION_QUERY | 28 | 0 | 0 | 0 | 3 |
| I_DEMOGRAPHIC_SERVICE | 57 | 0 | 0 | 0 | 1 |
| I_EHR_COMPOSITION | 88 | 0 | 0 | 0 | 0 |
| I_EHR_CONTRIBUTION | 55 | 0 | 0 | 0 | 5 |
| I_EHR_DIRECTORY | 63 | 0 | 0 | 0 | 4 |
| I_EHR_EXTRACT_SERVICE | 0 | 0 | 0 | 0 | 10 |
| I_EHR_SERVICE | 24 | 0 | 0 | 0 | 1 |
| I_EHR_STATUS | 46 | 0 | 0 | 0 | 1 |
| I_ITS_REST_ITEM_TAGS | 30 | 0 | 0 | 0 | 0 |
| I_ITS_REST_REVISION_HISTORY | 6 | 0 | 0 | 0 | 0 |
| I_ITS_REST_SYSTEM | 1 | 0 | 0 | 0 | 0 |
| I_ITS_REST_VERSIONED_PARTY | 3 | 0 | 0 | 0 | 1 |
| I_QUERY_SERVICE | 36 | 0 | 0 | 0 | 0 |
| I_TDD_SERVICE | 0 | 0 | 0 | 0 | 4 |
| SEC | 6 | 0 | 0 | 0 | 0 |
| SF | 63 | 0 | 0 | 0 | 1 |
| SIG | 10 | 0 | 0 | 0 | 2 |
| SMART | 3 | 0 | 0 | 0 | 0 |

## By capability

Selected gating cases per claimed capability. `inconclusive` counts cases whose exchange did not conclude (transport fault, unmapped status, step resolution) — they block a `passed` evidence token and are triaged, never absorbed.

| Capability | Evidence | passed | failed | inconclusive | unevidenced |
| --- | --- | --- | --- | --- | --- |
| Adl14ArchetypeProvisioning | excused (unrealized on this technology profile) | 0 | 0 | 0 | 1 |
| Adl14OptProvisioning | pass | 19 | 0 | 0 | 5 |
| Adl2ArchetypeProvisioning | excused (unrealized on this technology profile) | 0 | 0 | 0 | 6 |
| Adl2OptProvisioning | pass | 23 | 0 | 0 | 6 |
| TemplateExamples | pass | 3 | 0 | 0 | 0 |
| QueryProvisioning | pass | 27 | 0 | 0 | 0 |
| EhrOperations | pass | 22 | 0 | 0 | 1 |
| EhrStatus | pass | 45 | 0 | 0 | 1 |
| CompositionOps | pass | 27 | 0 | 0 | 0 |
| DirectoryOps | pass | 63 | 0 | 0 | 6 |
| ChangeSets | pass | 48 | 0 | 0 | 5 |
| Versioning | pass | 59 | 0 | 0 | 0 |
| ArchetypeValidation | pass | 121 | 0 | 0 | 0 |
| PartyOperations | pass | 50 | 0 | 0 | 2 |
| PartyRelationshipOperations | pass | 15 | 0 | 0 | 0 |
| AqlBasic | pass | 19 | 0 | 0 | 0 |
| AqlAdvanced | pass | 2 | 0 | 0 | 0 |
| AqlTerminology | pass | 1 | 0 | 0 | 0 |
| ActivityReport | excused (unrealized on this technology profile) | 0 | 0 | 0 | 8 |
| PhysicalDeletion | pass | 9 | 0 | 0 | 2 |
| EhrDumpLoad | excused (unrealized on this technology profile) | 0 | 0 | 0 | 2 |
| EhrArchive | excused (unrealized on this technology profile) | 0 | 0 | 0 | 2 |
| DemographicArchive | excused (unrealized on this technology profile) | 0 | 0 | 0 | 2 |
| EhrExtract | excused (unrealized on this technology profile) | 0 | 0 | 0 | 10 |
| Tds | excused (unrealized on this technology profile) | 0 | 0 | 0 | 4 |
| DefinitionApi | pass | 1 | 0 | 0 | 0 |
| EhrApi | pass | 2 | 0 | 0 | 0 |
| DemographicApi | pass | 26 | 0 | 0 | 1 |
| QueryApi | pass | 20 | 0 | 0 | 0 |
| AdminApi | pass | 8 | 0 | 0 | 0 |
| MessageApi | excused (unrealized on this technology profile) | 0 | 0 | 0 | 1 |
| SystemApi | pass | 1 | 0 | 0 | 0 |
| ItemTags | pass | 32 | 0 | 0 | 0 |
| Signing | pass | 10 | 0 | 0 | 2 |
| SimplifiedFormats | pass | 67 | 0 | 0 | 1 |
| SmartAppLaunch | pass | 3 | 0 | 0 | 0 |
| EhrDemographicSeparation | pass | 1 | 0 | 0 | 0 |
| AuthenticatedAccess | pass | 2 | 0 | 0 | 0 |
| AuthorizationSeparation | pass | 1 | 0 | 0 | 0 |
| AuditAccountability | pass | 1 | 0 | 0 | 0 |
| AnonymousEhrs | pass | 1 | 0 | 0 | 0 |

## Performance measurements

### PERF-hospital_sim-class_POC — class POC · EARNED

Offered load sustained: 2.03/s over 3600 s (after 300 s warmup) · environment: consumer-laptop (8 cores, 16 GB, nvme, single-node docker compose (8-CPU/8GB Docker VM) on Apple M2)

| Operation | Requests | Errors | p50 (ms) | p90 (ms) | p99 (ms) |
| --- | --- | --- | --- | --- | --- |
| adhoc_query | 953 | 0 | 23.1 | 29.3 | 41.3 |
| composition_commit | 315 | 0 | 38.0 | 74.6 | 101.4 |
| composition_delete | 4 | 0 | 21.0 | 28.3 | 28.3 |
| composition_read | 1906 | 0 | 18.7 | 24.8 | 31.8 |
| composition_read_current | 1072 | 0 | 18.3 | 24.1 | 31.6 |
| composition_revision_history | 1029 | 0 | 13.8 | 17.8 | 25.6 |
| composition_update | 77 | 0 | 32.8 | 46.7 | 56.0 |
| contribution_commit | 52 | 0 | 61.1 | 93.6 | 108.1 |
| contribution_read | 110 | 0 | 16.3 | 20.4 | 23.3 |
| directory_create | 13 | 0 | 14.7 | 17.7 | 20.4 |
| directory_read | 967 | 0 | 14.0 | 18.5 | 25.7 |
| directory_update | 13 | 0 | 21.3 | 25.2 | 26.8 |
| ehr_create | 13 | 0 | 18.6 | 27.8 | 29.9 |
| ehr_read | 98 | 0 | 13.7 | 19.2 | 22.0 |
| ehr_status_read | 26 | 0 | 17.4 | 20.7 | 22.8 |
| ehr_status_update | 26 | 0 | 25.7 | 34.1 | 39.5 |
| stored_query_execute | 204 | 0 | 29.3 | 35.6 | 44.7 |
| tags_put | 34 | 0 | 19.3 | 24.4 | 32.8 |
| tags_read | 34 | 0 | 14.9 | 18.5 | 20.6 |
| template_get | 85 | 0 | 50.3 | 76.8 | 95.9 |
| template_list | 85 | 0 | 43.8 | 62.2 | 80.4 |
| ward_query | 204 | 0 | 23.0 | 30.1 | 41.3 |

Percentiles re-derive from the embedded HDR V2 histograms; the class verdict is recomputed from them by the verdict pipeline, never trusted from this table.

## Honesty

Coverage: 694 of 757 selected cases driven.

Not-executed verdicts (each cited):

| Case | Citation |
| --- | --- |
| I_ADMIN_ARCHIVE.archive_ehrs-archive_selected | I_ADMIN_ARCHIVE.archive_ehrs: AMB-33 |
| I_ADMIN_ARCHIVE.archive_ehrs-archive_unknown | I_ADMIN_ARCHIVE.archive_ehrs: AMB-33 |
| I_ADMIN_ARCHIVE.archive_parties-archive_selected | I_ADMIN_ARCHIVE.archive_parties: AMB-33 |
| I_ADMIN_ARCHIVE.archive_parties-archive_unknown | I_ADMIN_ARCHIVE.archive_parties: AMB-33 |
| I_ADMIN_DUMP_LOAD.export_ehrs-export_all | I_ADMIN_DUMP_LOAD.export_ehrs: AMB-33 |
| I_ADMIN_DUMP_LOAD.export_ehrs-export_formats | I_ADMIN_DUMP_LOAD.export_ehrs: AMB-33 |
| I_ADMIN_SERVICE.composition_version_count-all | I_ADMIN_SERVICE.composition_version_count: AMB-33 |
| I_ADMIN_SERVICE.composition_version_count-time_range | I_ADMIN_SERVICE.composition_version_count: AMB-33 |
| I_ADMIN_SERVICE.contribution_count-all | I_ADMIN_SERVICE.contribution_count: AMB-33 |
| I_ADMIN_SERVICE.contribution_count-time_range | I_ADMIN_SERVICE.contribution_count: AMB-33 |
| I_ADMIN_SERVICE.list_contributions-all | I_ADMIN_SERVICE.list_contributions: AMB-33 |
| I_ADMIN_SERVICE.list_contributions-time_range | I_ADMIN_SERVICE.list_contributions: AMB-33 |
| I_ADMIN_SERVICE.physical_party_delete-delete_existing | I_ADMIN_SERVICE.physical_party_delete: AMB-33 |
| I_ADMIN_SERVICE.physical_party_delete-delete_non_existing | I_ADMIN_SERVICE.physical_party_delete: AMB-33 |
| I_ADMIN_SERVICE.versioned_composition_count-all | I_ADMIN_SERVICE.versioned_composition_count: AMB-33 |
| I_ADMIN_SERVICE.versioned_composition_count-time_range | I_ADMIN_SERVICE.versioned_composition_count: AMB-33 |
| I_DEFINITION_ADL14.delete_opt-delete_existing | I_DEFINITION_ADL14.delete_opt: AMB-17 |
| I_DEFINITION_ADL14.delete_opt-delete_latest_version | I_DEFINITION_ADL14.delete_opt: AMB-17 |
| I_DEFINITION_ADL14.delete_opt-delete_non_existing | I_DEFINITION_ADL14.delete_opt: AMB-17 |
| I_DEFINITION_ADL14.delete_opt-delete_specific_version | I_DEFINITION_ADL14.delete_opt: AMB-17 |
| I_DEFINITION_ADL14.get_opt-retrieve_latest_version | option adl14-partial-id-latest: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEFINITION_ADL14.list_archetypes-unrealized | I_DEFINITION_ADL14.list_archetypes: AMB-41 |
| I_DEFINITION_ADL2.archetypes_count-unrealized | I_DEFINITION_ADL2.archetypes_count: AMB-37 |
| I_DEFINITION_ADL2.artefacts_count-unrealized | I_DEFINITION_ADL2.artefacts_count: AMB-37 |
| I_DEFINITION_ADL2.delete_artefact-existing | I_DEFINITION_ADL2.delete_artefact: AMB-37 |
| I_DEFINITION_ADL2.delete_artefact-non_existing | I_DEFINITION_ADL2.delete_artefact: AMB-37 |
| I_DEFINITION_ADL2.list_archetypes-unrealized | I_DEFINITION_ADL2.list_archetypes: AMB-37 |
| I_DEFINITION_ADL2.list_artefacts-unrealized | I_DEFINITION_ADL2.list_artefacts: AMB-37 |
| I_DEFINITION_ADL2.opts_count-unrealized | I_DEFINITION_ADL2.opts_count: AMB-37 |
| I_DEFINITION_ADL2.templates_count-unrealized | I_DEFINITION_ADL2.templates_count: AMB-37 |
| I_DEFINITION_QUERY.delete_query-delete_existing | I_DEFINITION_QUERY.delete_query: AMB-127 |
| I_DEFINITION_QUERY.list_matching_queries-id_pattern | I_DEFINITION_QUERY.list_matching_queries: AMB-121 |
| I_DEFINITION_QUERY.queries_count-count | I_DEFINITION_QUERY.queries_count: AMB-127 |
| I_DEMOGRAPHIC_SERVICE.get_party-xml_not_acceptable | option party-xml-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_EHR_CONTRIBUTION.list_contributions-ehr_containing_directory | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_CONTRIBUTION.list_contributions-ehr_containing_ehr_status | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_CONTRIBUTION.list_contributions-empty | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_CONTRIBUTION.list_contributions-non_existing_ehr | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_CONTRIBUTION.list_contributions-post_commit | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_DIRECTORY.get_directory-xml_not_acceptable | option directory-xml-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_EHR_DIRECTORY.get_versioned_directory-bad_ehr | I_EHR_DIRECTORY.get_versioned_directory: AMB-24 |
| I_EHR_DIRECTORY.get_versioned_directory-directory_with_two_versions | I_EHR_DIRECTORY.get_versioned_directory: AMB-24 |
| I_EHR_DIRECTORY.get_versioned_directory-empty_ehr | I_EHR_DIRECTORY.get_versioned_directory: AMB-24 |
| I_EHR_EXTRACT_SERVICE.export_ehr_extracts-by_spec | I_EHR_EXTRACT_SERVICE.export_ehr_extracts: AMB-34 |
| I_EHR_EXTRACT_SERVICE.export_ehr_extracts-empty_result | I_EHR_EXTRACT_SERVICE.export_ehr_extracts: AMB-34 |
| I_EHR_EXTRACT_SERVICE.export_ehrs-export_existing | I_EHR_EXTRACT_SERVICE.export_ehrs: AMB-34 |
| I_EHR_EXTRACT_SERVICE.export_ehrs-export_unknown | I_EHR_EXTRACT_SERVICE.export_ehrs: AMB-34 |
| I_EHR_EXTRACT_SERVICE.import_ehr-duplicate | I_EHR_EXTRACT_SERVICE.import_ehr: AMB-34 |
| I_EHR_EXTRACT_SERVICE.import_ehr-new | I_EHR_EXTRACT_SERVICE.import_ehr: AMB-34 |
| I_EHR_EXTRACT_SERVICE.import_ehr-with_id | I_EHR_EXTRACT_SERVICE.import_ehr: AMB-34 |
| I_EHR_EXTRACT_SERVICE.import_ehr_extract-into_existing | I_EHR_EXTRACT_SERVICE.import_ehr_extract: AMB-34 |
| I_EHR_EXTRACT_SERVICE.import_ehr_extract-invalid | I_EHR_EXTRACT_SERVICE.import_ehr_extract: AMB-34 |
| I_EHR_EXTRACT_SERVICE.import_ehr_extract-unknown_ehr | I_EHR_EXTRACT_SERVICE.import_ehr_extract: AMB-34 |
| I_EHR_SERVICE.get_ehr-xml_not_acceptable | option ehr-xml-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_EHR_STATUS.get_ehr_status-xml_not_acceptable | option ehr-status-xml-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_ITS_REST_VERSIONED_PARTY.versioned_party_get-xml | option versioned-party-xml-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_TDD_SERVICE.import_tdd-invalid | I_TDD_SERVICE.import_tdd: AMB-34 |
| I_TDD_SERVICE.import_tdd-valid | I_TDD_SERVICE.import_tdd: AMB-34 |
| I_TDD_SERVICE.import_tdds-bulk_invalid | I_TDD_SERVICE.import_tdds: AMB-34 |
| I_TDD_SERVICE.import_tdds-bulk_valid | I_TDD_SERVICE.import_tdds: AMB-34 |
| SF-DEPRECATED-media_supported | option sf-deprecated-types-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| SIG-VERSION-directory_signature_present | I_EHR_DIRECTORY.get_versioned_directory: AMB-24 |
| SIG-VERSION-directory_verifiable | I_EHR_DIRECTORY.get_versioned_directory: AMB-24 |
