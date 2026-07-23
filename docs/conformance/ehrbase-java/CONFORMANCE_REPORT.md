# Conformance Report

SUT: ehrbase-java 2.34.0 · schedule cnf-2.0-w2 · ITS its-rest
Runner: cnf-runner 3.7.0 · verification pack: passed

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
