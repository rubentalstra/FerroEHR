# Conformance Report

SUT: ehrbase-rs 3.7.0 · schedule cnf-2.0-w2 · ITS its-rest
Runner: cnf-runner 3.7.0 · verification pack: passed

## Summary

| Status | Count |
| --- | --- |
| passed | 323 |
| failed | 0 |
| errored | 0 |
| skipped | 0 |
| not_applicable | 67 |
| total | 390 |

## By chapter

| Chapter | passed | failed | errored | skipped | not_applicable |
| --- | --- | --- | --- | --- | --- |
| CONT | 88 | 0 | 0 | 0 | 0 |
| I_ADMIN_ARCHIVE | 0 | 0 | 0 | 0 | 4 |
| I_ADMIN_DUMP_LOAD | 0 | 0 | 0 | 0 | 2 |
| I_ADMIN_SERVICE | 2 | 0 | 0 | 0 | 10 |
| I_DEFINITION_ADL14 | 9 | 0 | 0 | 0 | 8 |
| I_DEFINITION_ADL2 | 16 | 0 | 0 | 0 | 8 |
| I_DEFINITION_QUERY | 7 | 0 | 0 | 0 | 0 |
| I_DEMOGRAPHIC_SERVICE | 12 | 0 | 0 | 0 | 12 |
| I_EHR_COMPOSITION | 32 | 0 | 0 | 0 | 0 |
| I_EHR_CONTRIBUTION | 27 | 0 | 0 | 0 | 5 |
| I_EHR_DIRECTORY | 34 | 0 | 0 | 0 | 3 |
| I_EHR_EXTRACT_SERVICE | 0 | 0 | 0 | 0 | 10 |
| I_EHR_SERVICE | 12 | 0 | 0 | 0 | 0 |
| I_EHR_STATUS | 10 | 0 | 0 | 0 | 0 |
| I_QUERY_SERVICE | 15 | 0 | 0 | 0 | 0 |
| I_TDD_SERVICE | 0 | 0 | 0 | 0 | 4 |
| SEC | 5 | 0 | 0 | 0 | 0 |
| SF | 50 | 0 | 0 | 0 | 1 |
| SIG | 4 | 0 | 0 | 0 | 0 |

## Performance measurements

### PERF-hospital_sim-class_POC — class POC · not earned

Offered load sustained: 2.03/s over 3600 s (after 300 s warmup) · environment: consumer-laptop (8 cores, 16 GB, nvme, single-node docker compose (8-CPU/8GB Docker VM) on Apple M2)

| Operation | Requests | Errors | p50 (ms) | p90 (ms) | p99 (ms) |
| --- | --- | --- | --- | --- | --- |
| adhoc_query | 953 | 0 | 29.7 | 36.8 | 46.8 |
| composition_commit | 315 | 0 | 38.1 | 73.1 | 114.0 |
| composition_delete | 4 | 0 | 20.7 | 25.2 | 25.2 |
| composition_read | 1906 | 0 | 19.5 | 25.2 | 32.0 |
| composition_read_current | 1072 | 0 | 20.6 | 26.9 | 32.9 |
| composition_revision_history | 1029 | 0 | 13.9 | 18.3 | 21.4 |
| composition_update | 77 | 0 | 33.6 | 44.5 | 86.2 |
| contribution_commit | 52 | 0 | 55.3 | 91.4 | 123.6 |
| contribution_read | 110 | 0 | 17.5 | 23.3 | 26.8 |
| directory_create | 13 | 0 | 15.6 | 24.1 | 34.3 |
| directory_read | 967 | 0 | 14.2 | 19.3 | 26.7 |
| directory_update | 13 | 0 | 25.0 | 31.0 | 32.9 |
| ehr_create | 13 | 0 | 22.8 | 29.0 | 43.9 |
| ehr_read | 98 | 0 | 16.1 | 22.0 | 29.1 |
| ehr_status_read | 26 | 0 | 16.4 | 21.8 | 22.6 |
| ehr_status_update | 26 | 0 | 23.9 | 32.2 | 42.7 |
| stored_query_execute | 204 | 0 | 32.3 | 38.5 | 46.6 |
| tags_put | 34 | 0 | 21.9 | 30.0 | 31.5 |
| tags_read | 34 | 0 | 13.6 | 17.6 | 23.4 |
| template_get | 85 | 0 | 29.3 | 63.4 | 89.7 |
| template_list | 85 | 0 | 44.3 | 62.3 | 84.2 |
| ward_query | 204 | 0 | 310.5 | 5029.9 | 5775.4 |

Violations:

- ward_query LatencyP99 5775.359ms > max 1000ms

Percentiles re-derive from the embedded HDR V2 histograms; the class verdict is recomputed from them by the verdict pipeline, never trusted from this table.

## Honesty

Coverage: 323 of 390 selected cases driven.

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
