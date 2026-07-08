# openEHR Conformance Statement (generated)

> Generated from a conformance run — never hand-asserted. This is a
> scoped, honest statement; the deviations register below lists every
> excluded capability with its structural reason.

## 1. SUT identity

| Field | Value |
|---|---|
| Base URL | `http://127.0.0.1:54408/ehrbase/rest/openehr/v1` |
| RM version | 1.2.0 |
| Auth mode | basic (self-host, RBAC off) |
| Corpus | `openEHR/specifications-CNF` @ `33251d2a` |

## 2. Scope of test

| Field | Value |
|---|---|
| Profiles requested | all |
| Data formats | json |
| Identified cases | 322 |
| Implemented | 210 |
| Passed | 136 |
| Failed | 87 |

## 3. Detailed test report

| Case | Capability | Format | Data sets | Result |
|---|---|---|--:|---|
| `I_EHR_SERVICE.has_ehr-existing_ehr_id` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.has_ehr-existing_subject_id` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.has_ehr-non_existing_ehr_id` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.has_ehr-non_existing_subject_id` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.create_ehr-main` | EhrOperations | json | 16/16 | PASS |
| `I_EHR_SERVICE.create_ehr-same_ehr_twice` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.create_ehr-two_ehrs_same_patient` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.get_ehr-existing_ehr_by_ehr_id` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.get_ehr-existing_ehr_by_subject_id` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.get_ehr-get_ehr_by_invalid_ehr_id` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_SERVICE.get_ehr-get_ehr_by_invalid_subject_id` | EhrOperations | json | 1/1 | PASS |
| `I_EHR_STATUS.get_ehr_status-get_by_ehr_id` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.get_ehr_status-bad_ehr` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.set_ehr_queryable-existing_ehr` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.set_ehr_queryable-bad_ehr` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.set_ehr_modifiable-existing_ehr` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.set_ehr_modifiable-bad_ehr` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.clear_ehr_queryable-existing_ehr` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.clear_ehr_queryable-bad_ehr` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.clear_ehr_modifiable-existing_ehr` | EhrStatus | json | 1/1 | PASS |
| `I_EHR_STATUS.clear_ehr_modifiable-bad_ehr` | EhrStatus | json | 1/1 | PASS |
| `FIXTURE-I_EHR_SERVICE.create_ehr-invalid_status` | EhrOperations | json | 0/0 | **FAIL** |
| `I_EHR_COMPOSITION.create_composition-event` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.create_composition-persistent` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.create_composition-same_opt_twice` | CompositionOps | json | 0/0 | **FAIL** |
| `I_EHR_COMPOSITION.create_composition-invalid_event` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.create_composition-invalid_persistent` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.create_composition-event_bad_opt` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.create_composition-event_bad_ehr` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_latest` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_latest-bad_composition` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_latest-bad_ehr` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_at_time-no_time_arg` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_at_time-bad_composition` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_at_time-bad_ehr` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_at_times` | CompositionOps | json | 3/3 | PASS |
| `I_EHR_COMPOSITION.get_composition_version` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_version-bad_version` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_version-bad_ehr` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_composition_versions` | CompositionOps | json | 2/2 | PASS |
| `I_EHR_COMPOSITION.get_versioned_composition` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_versioned_composition-non_existent` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.get_versioned_composition-bad_ehr` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.update_composition-event` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.update_composition-persistent` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.update_composition-non_existent` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.update_composition-wrong_template` | CompositionOps | json | 0/0 | **FAIL** |
| `I_EHR_COMPOSITION.delete_composition-event` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.delete_composition-persistent` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_COMPOSITION.delete_composition-non_existent` | CompositionOps | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-valid_composition` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-invalid_composition` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-empty` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-valid_invalid_compositions` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-non_exiting_opt` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-event_composition` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-persistent_composition` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-delete` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_invalid` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_creation` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-minimal_ehr_status` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-full_ehr_status` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type` | ChangeSets | json | 0/0 | **FAIL** |
| `I_EHR_CONTRIBUTION.commit_contribution-invalid_ehr_status` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-valid_directory` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-fail_create_existing_directory` | ChangeSets | json | 0/0 | **FAIL** |
| `I_EHR_CONTRIBUTION.commit_contribution-fail_modify_non_existing_directory` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.commit_contribution-update_existing_directory` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.get_contribution-existing` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.get_contribution-empty_ehr` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.get_contribution-bad_ehr` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_CONTRIBUTION.get_contribution-bad_contribution` | ChangeSets | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.create_directory-empty_ehr` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.create_directory-ehr_with_directory` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.create_directory-bad_ehr` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.get_directory-ehr_root_directory` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.get_directory-bad_ehr` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.get_directory_at_time-bad_ehr` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.update_directory-ehr_with_directory` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.update_directory-bad_ehr` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.delete_directory-ehr_with_directory` | DirectoryOps | json | 1/1 | PASS |
| `I_EHR_DIRECTORY.delete_directory-bad_ehr` | DirectoryOps | json | 1/1 | PASS |
| `I_DEFINITION_ADL14.upload_opt-valid_opt` | Adl14OptProvisioning | json | 0/0 | **FAIL** |
| `I_DEFINITION_ADL14.upload_opt-invalid_opt` | Adl14OptProvisioning | json | 0/0 | **FAIL** |
| `I_DEFINITION_ADL14.get_opts-retrieve_all_no_opts` | Adl14OptProvisioning | json | 1/1 | PASS |
| `I_DEFINITION_QUERY.valid_query-valid` | QueryProvisioning | json | 1/1 | PASS |
| `I_DEFINITION_QUERY.list_queries-non_empty` | QueryProvisioning | json | 1/1 | PASS |
| `I_DEFINITION_QUERY.has_query-xxx` | QueryProvisioning | json | 1/1 | PASS |
| `I_QUERY_SERVICE.smoke_test` | AqlBasic | json | 1/1 | PASS |
| `I_QUERY_SERVICE.execute_ad_hoc_query-empty_db` | AqlBasic | json | 0/0 | **FAIL** |
| `I_QUERY_SERVICE.execute_stored_query-empty_db` | AqlBasic | json | 0/0 | **FAIL** |
| `I_QUERY_SERVICE.execute_ad_hoc_query-loaded_db` | AqlBasic | json | 1/1 | PASS |
| `QUERY-FIXTURE-invalid` | AqlBasic | json | 2/2 | PASS |
| `QUERY-FIXTURE-A-empty_db` | AqlBasic | json | 0/0 | **FAIL** |
| `QUERY-FIXTURE-B-empty_db` | AqlBasic | json | 0/0 | **FAIL** |
| `QUERY-FIXTURE-C-empty_db` | AqlBasic | json | 0/0 | **FAIL** |
| `QUERY-FIXTURE-D-empty_db` | AqlBasic | json | 0/0 | **FAIL** |
| `QUERY-FIXTURE-A-loaded_db` | AqlBasic | json | 0/0 | **FAIL** |
| `QUERY-FIXTURE-B-loaded_db` | AqlBasic | json | 0/0 | **FAIL** |
| `QUERY-FIXTURE-C-loaded_db` | AqlBasic | json | 0/0 | skipped |
| `QUERY-FIXTURE-D-loaded_db` | AqlBasic | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_any-context_any` | ArchetypeValidation | json | 6/6 | PASS |
| `CONT-COMP-content_card_1plus-context_any` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_3plus-context_any` | ArchetypeValidation | json | 6/6 | PASS |
| `CONT-COMP-content_card_opt-context_any` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_mand-context_any` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_3to5-context_any` | ArchetypeValidation | json | 6/6 | PASS |
| `CONT-COMP-content_card_any-context_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_1plus-context_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_3plus-context_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_opt-context_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_mand-context_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-COMP-content_card_3to5-context_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-OBS-state_ex_opt-protocol_ex_opt` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-OBS-state_ex_opt-protocol_ex_mand` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-OBS-state_ex_mand-protocol_ex_opt` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-OBS-state_ex_mand-protocol_ex_mand` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-HIST-events_card_any-summary_ex_opt` | ArchetypeValidation | json | 3/3 | PASS |
| `CONT-HIST-events_card_1plus-summary_ex_opt` | ArchetypeValidation | json | 3/3 | PASS |
| `CONT-HIST-events_card_3plus-summary_ex_opt` | ArchetypeValidation | json | 3/3 | PASS |
| `CONT-HIST-events_card_opt-summary_ex_opt` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-HIST-events_card_mand-summary_ex_opt` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-HIST-events_card_3to5-summary_ex_opt` | ArchetypeValidation | json | 3/3 | PASS |
| `CONT-HIST-events_card_any-summary_ex_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-HIST-events_card_1plus-summary_ex_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-HIST-events_card_3plus-summary_ex_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-HIST-events_card_opt-summary_ex_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-HIST-events_card_mand-summary_ex_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-HIST-events_card_3to5-summary_ex_mand` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-EVENT-state_ex_opt` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-EVENT-state_ex_mand` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-EVENT-type_any` | ArchetypeValidation | json | 1/1 | PASS |
| `CONT-EVENT-type_point_event` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-EVENT-type_interval_event` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-ITEM_STR-type_any` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-ITEM_STR-type_item_tree` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-ITEM_STR-type_item_list` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-ITEM_STR-type_item_table` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-ITEM_STR-type_item_single` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_BOOLEAN-anything_allowed` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_BOOLEAN-only_true_allowed` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_BOOLEAN-only_false_allowed` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_IDENTIFIER-validate_all_pattern` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_IDENTIFIER-validate_all_list` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_TEXT-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_TEXT-validate_list` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_CODED_TEXT-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_CODED_TEXT-validate_local_codes` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_CODED_TEXT-validate_ext_term` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_ORDINAL-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_ORDINAL-validate_constraint` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_SCALE-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_SCALE-validate_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_COUNT-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_COUNT-validate_range` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_COUNT-validate_list` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_QUANTITY-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_QUANTITY-validate_property` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_QUANTITY-validate_property_units` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_QUANTITY-validate_property_units_mag` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_PROPORTION-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_PROPORTION-validate_ratio` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_PROPORTION-validate_unitary` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_PROPORTION-validate_percent` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_PROPORTION-validate_fraction` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_PROPORTION-validate_integer_fraction` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_PROPORTION-validate_any_fraction` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_PROPORTION-validate_ratio_range` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_INTERVAL_DV_COUNT-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_COUNT-validate_lower_upper` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_COUNT-validate_lower_upper_list` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_QUANTITY-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_QUANTITY-validate_upper_lower` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DATE_TIME-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DATE-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_TIME-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DURATION-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DURATION-validate_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_DURATION-validate_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_ORDINAL-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_ORDINAL-validate_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_SCALE-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_SCALE-validate_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_PROPORTION-validate_open` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_PROPORTION-validate_unitary` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_PROPORTION-validate_percentage` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_PROPORTION-validate_fraction` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_PROPORTION-validate_integer_fraction` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_DURATION-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_DURATION-validate_fields` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_DURATION-validate_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_DURATION-validate_fields_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_TIME-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_TIME-validate_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_TIME-validate_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_DATE-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_DATE-validate_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_DATE-validate_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_DATE_TIME-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_DATE_TIME-validate_constraint` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_DATE_TIME-validate_range` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_PARSABLE-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_PARSABLE-validate_value_formalism` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_MULTIMEDIA-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_MULTIMEDIA-validate_media_type` | ArchetypeValidation | json | 0/0 | **FAIL** |
| `CONT-DV_URI-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_URI-validate_pattern` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_URI-validate_list` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_EHR_URI-validate_open` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_EHR_URI-validate_pattern` | ArchetypeValidation | json | 2/2 | PASS |
| `CONT-DV_EHR_URI-validate_list` | ArchetypeValidation | json | 2/2 | PASS |
| `SIGN-digest-present` | Signing | json | 1/1 | PASS |
| `SIGN-digest-recomputes` | Signing | json | 1/1 | PASS |
| `SIGN-all-kinds` | Signing | json | 4/4 | PASS |
| `SIGN-client-verbatim` | Signing | json | 1/1 | PASS |
| `SIGN-pgp-verifies` | Signing | json | 0/0 | skipped |

## 4. Deviations register

Excluded capabilities/cases, by structural reason (never "currently failing"):

| Reason | Cases |
|---|--:|
| not_yet_transcribed | 54 |
| upstream_duplicate | 1 |
| upstream_placeholder | 57 |
