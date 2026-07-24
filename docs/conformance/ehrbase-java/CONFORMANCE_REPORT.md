# Conformance Report

SUT: ehrbase-java 2.34.0 · schedule cnf-2.0-w2 · ITS its-rest
Runner: cnf-runner 3.8.0 · verification pack: passed

## Summary

| Status | Count |
| --- | --- |
| passed | 159 |
| failed | 128 |
| errored | 38 |
| skipped | 0 |
| not_applicable | 68 |
| total | 393 |

## By chapter

| Chapter | passed | failed | errored | skipped | not_applicable |
| --- | --- | --- | --- | --- | --- |
| CONT | 37 | 47 | 4 | 0 | 0 |
| I_ADMIN_ARCHIVE | 0 | 0 | 0 | 0 | 4 |
| I_ADMIN_DUMP_LOAD | 0 | 0 | 0 | 0 | 2 |
| I_ADMIN_SERVICE | 1 | 1 | 0 | 0 | 10 |
| I_DEFINITION_ADL14 | 3 | 1 | 5 | 0 | 8 |
| I_DEFINITION_ADL2 | 1 | 3 | 12 | 0 | 8 |
| I_DEFINITION_QUERY | 8 | 2 | 0 | 0 | 0 |
| I_DEMOGRAPHIC_SERVICE | 4 | 0 | 8 | 0 | 12 |
| I_EHR_COMPOSITION | 19 | 9 | 4 | 0 | 0 |
| I_EHR_CONTRIBUTION | 22 | 4 | 1 | 0 | 5 |
| I_EHR_DIRECTORY | 30 | 4 | 0 | 0 | 3 |
| I_EHR_EXTRACT_SERVICE | 0 | 0 | 0 | 0 | 10 |
| I_EHR_SERVICE | 11 | 1 | 0 | 0 | 0 |
| I_EHR_STATUS | 2 | 8 | 0 | 0 | 0 |
| I_QUERY_SERVICE | 9 | 5 | 1 | 0 | 0 |
| I_TDD_SERVICE | 0 | 0 | 0 | 0 | 4 |
| SEC | 4 | 0 | 1 | 0 | 0 |
| SF | 5 | 42 | 2 | 0 | 2 |
| SIG | 3 | 1 | 0 | 0 | 0 |

## Performance measurements

### PERF-hospital_sim-class_POC — class POC · not earned

Offered load sustained: 2.03/s over 3600 s (after 300 s warmup) · environment: ci-runner (4 cores, 16 GB, ssd, single-node docker compose (docker/sut-ehrbase-java.yml, ehrbase/ehrbase:2.34.0 + ehrbase-v2-postgres:16.2; no readonly principal — EHRbase Basic auth carries one clinical user and one admin user))

| Operation | Requests | Errors | p50 (ms) | p90 (ms) | p99 (ms) |
| --- | --- | --- | --- | --- | --- |
| adhoc_query | 953 | 0 | 43.6 | 90.3 | 145.0 |
| composition_commit | 315 | 0 | 72.6 | 154.4 | 272.1 |
| composition_delete | 4 | 0 | 36.4 | 56.2 | 56.2 |
| composition_read | 1906 | 0 | 19.1 | 28.7 | 56.9 |
| composition_read_current | 1072 | 0 | 24.2 | 40.8 | 80.0 |
| composition_revision_history | 1029 | 0 | 22.4 | 35.8 | 56.8 |
| composition_update | 77 | 77 | 52.1 | 93.3 | 157.8 |
| contribution_commit | 52 | 0 | 90.0 | 227.7 | 300.0 |
| contribution_read | 110 | 0 | 23.2 | 37.1 | 52.7 |
| directory_create | 13 | 0 | 25.3 | 46.8 | 62.1 |
| directory_read | 967 | 0 | 17.7 | 27.0 | 55.8 |
| directory_update | 13 | 0 | 43.4 | 84.7 | 117.4 |
| ehr_create | 13 | 0 | 35.6 | 50.0 | 50.9 |
| ehr_read | 98 | 0 | 20.0 | 29.4 | 41.0 |
| ehr_status_read | 26 | 0 | 26.9 | 40.2 | 51.3 |
| ehr_status_update | 26 | 26 | 36.2 | 62.8 | 74.2 |
| stored_query_execute | 204 | 0 | 38.1 | 56.0 | 75.6 |
| tags_put | 34 | 34 | 29.5 | 47.7 | 52.0 |
| tags_read | 34 | 34 | 27.6 | 42.1 | 62.2 |
| template_get | 85 | 3 | 60.1 | 92.5 | 201.2 |
| template_list | 85 | 0 | 17.6 | 24.0 | 33.6 |
| ward_query | 204 | 0 | 7962.6 | 9338.9 | 10887.2 |

Violations:

- ward_query LatencyP99 10887.167ms > max 1000ms
- error_rate 0.023770491803278688 > max 0

Percentiles re-derive from the embedded HDR V2 histograms; the class verdict is recomputed from them by the verdict pipeline, never trusted from this table.

## Honesty

Coverage: 237 of 255 selected cases driven.

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
| I_DEFINITION_ADL14.get_opt-retrieve_latest_version | option adl14-duplicate-versioned: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEFINITION_ADL14.get_opt-retrieve_specific_version | option adl14-duplicate-versioned: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEFINITION_ADL14.list_archetypes-unrealized | I_DEFINITION_ADL14.list_archetypes: AMB-41 |
| I_DEFINITION_ADL14.upload_opt-valid_opt_twice_no_conflict | option adl14-duplicate-versioned: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEFINITION_ADL2.archetypes_count-unrealized | I_DEFINITION_ADL2.archetypes_count: AMB-37 |
| I_DEFINITION_ADL2.artefacts_count-unrealized | I_DEFINITION_ADL2.artefacts_count: AMB-37 |
| I_DEFINITION_ADL2.delete_artefact-existing | I_DEFINITION_ADL2.delete_artefact: AMB-37 |
| I_DEFINITION_ADL2.delete_artefact-non_existing | I_DEFINITION_ADL2.delete_artefact: AMB-37 |
| I_DEFINITION_ADL2.list_archetypes-unrealized | I_DEFINITION_ADL2.list_archetypes: AMB-37 |
| I_DEFINITION_ADL2.list_artefacts-unrealized | I_DEFINITION_ADL2.list_artefacts: AMB-37 |
| I_DEFINITION_ADL2.opts_count-unrealized | I_DEFINITION_ADL2.opts_count: AMB-37 |
| I_DEFINITION_ADL2.templates_count-unrealized | I_DEFINITION_ADL2.templates_count: AMB-37 |
| I_DEMOGRAPHIC_SERVICE.create_party_relationship-aaaa | I_DEMOGRAPHIC_SERVICE.create_party_relationship: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.create_party_relationship-bbbb | I_DEMOGRAPHIC_SERVICE.create_party_relationship: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.delete_party_relationship-aaaa | I_PARTY_RELATIONSHIP.delete_party_relationship: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.delete_party_relationship-bbbb | I_PARTY_RELATIONSHIP.delete_party_relationship: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship-aaaa | I_PARTY_RELATIONSHIP.get_party_relationship: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship-bbbb | I_PARTY_RELATIONSHIP.get_party_relationship: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_time-aaaa | I_PARTY_RELATIONSHIP.get_party_relationship_at_time: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_time-bbbb | I_PARTY_RELATIONSHIP.get_party_relationship_at_time: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_version-aaaa | I_PARTY_RELATIONSHIP.get_party_relationship_at_version: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_version-bbbb | I_PARTY_RELATIONSHIP.get_party_relationship_at_version: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-aaaa | I_PARTY_RELATIONSHIP.update_party_relationship: AMB-32 |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-bbbb | I_PARTY_RELATIONSHIP.update_party_relationship: AMB-32 |
| I_EHR_CONTRIBUTION.list_contributions-ehr_containing_directory | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_CONTRIBUTION.list_contributions-ehr_containing_ehr_status | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_CONTRIBUTION.list_contributions-empty | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_CONTRIBUTION.list_contributions-non_existing_ehr | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
| I_EHR_CONTRIBUTION.list_contributions-post_commit | I_EHR_CONTRIBUTION.list_contributions: AMB-22 |
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
| I_TDD_SERVICE.import_tdd-invalid | I_TDD_SERVICE.import_tdd: AMB-34 |
| I_TDD_SERVICE.import_tdd-valid | I_TDD_SERVICE.import_tdd: AMB-34 |
| I_TDD_SERVICE.import_tdds-bulk_invalid | I_TDD_SERVICE.import_tdds: AMB-34 |
| I_TDD_SERVICE.import_tdds-bulk_valid | I_TDD_SERVICE.import_tdds: AMB-34 |
| SF-DEPRECATED-media_supported | option sf-deprecated-types-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| SF-DEPRECATED-media_unsupported | option sf-deprecated-types-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
