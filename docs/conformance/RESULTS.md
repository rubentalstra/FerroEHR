# openEHR CNF — Test Execution Report

- SUT: `http://127.0.0.1:63643/ehrbase/rest/openehr/v1`
- RM version: 1.2.0
- Auth mode: basic (self-host, RBAC off)
- Corpus: `openEHR/specifications-CNF` @ `33251d2a`
- Started: 2026-07-07T22:42:52.121563Z

**322 identified cases · 210 implemented · 91 passed · 36 failed.**

## Per-chapter matrix

| Chapter | Implemented | Passed | Failed | Excluded | Not-yet | Total |
|---|--:|--:|--:|--:|--:|--:|
| master04 | 3 | 1 | 2 | 12 | 12 | 15 |
| master05 | 3 | 3 | 0 | 4 | 4 | 7 |
| master06 | 21 | 21 | 1 | 0 | 0 | 21 |
| master07 | 28 | 26 | 2 | 3 | 3 | 31 |
| master08 | 22 | 18 | 4 | 9 | 9 | 31 |
| master09 | 11 | 11 | 0 | 26 | 26 | 37 |
| master10 | 0 | 0 | 0 | 24 | 0 | 24 |
| master11 | 4 | 3 | 9 | 1 | 0 | 5 |
| master12 | 0 | 0 | 0 | 18 | 0 | 18 |
| master13 | 0 | 0 | 0 | 14 | 0 | 14 |
| master15 | 12 | 0 | 0 | 0 | 0 | 12 |
| master16 | 26 | 0 | 10 | 0 | 0 | 26 |
| master17.1 | 5 | 0 | 1 | 0 | 0 | 5 |
| master17.2 | 5 | 1 | 1 | 1 | 0 | 6 |
| master17.3 | 47 | 3 | 3 | 0 | 0 | 47 |
| master17.4 | 13 | 0 | 2 | 0 | 0 | 13 |
| master17.5 | 0 | 0 | 0 | 0 | 0 | 0 |
| master17.6 | 4 | 0 | 0 | 0 | 0 | 4 |
| master17.7 | 6 | 0 | 1 | 0 | 0 | 6 |

## Failures

Each failure must become a finding (`F-AA-NN`) before/with the fix (§4.5).

- `FIXTURE-I_EHR_SERVICE.create_ehr-invalid_status` (json, master06-func_tc_ehr.adoc §Test Data Sets (INVALID class 2)): 2/11 invalid EHR_STATUS data sets were rejected (the rest were accepted)
- `I_EHR_COMPOSITION.create_composition-same_opt_twice` (json, I_EHR_COMPOSITION.create_composition-same_opt_twice): expected a negative (4xx) response, got 201
- `I_EHR_COMPOSITION.update_composition-wrong_template` (json, I_EHR_COMPOSITION.update_composition-wrong_template): expected a negative (4xx) response, got 200
- `I_EHR_CONTRIBUTION.commit_contribution-invalid_composition` (json, I_EHR_CONTRIBUTION.commit_contribution-invalid_composition): expected a negative (4xx) response, got 201
- `I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_invalid` (json, I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_invalid): expected a negative (4xx) response, got 201
- `I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type` (json, I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type): expected a negative (4xx) response, got 201
- `I_EHR_CONTRIBUTION.commit_contribution-fail_create_existing_directory` (json, I_EHR_CONTRIBUTION.commit_contribution-fail_create_existing_directory): expected a negative (4xx) response, got 201
- `I_DEFINITION_ADL14.upload_opt-valid_opt` (json, I_DEFINITION_ADL14.upload_opt-valid_opt): valid OPT minimal_evaluation.opt rejected with 409
- `I_DEFINITION_ADL14.upload_opt-invalid_opt` (json, I_DEFINITION_ADL14.upload_opt-invalid_opt): 12/18 invalid OPTs were rejected (the rest were accepted)
- `I_QUERY_SERVICE.execute_ad_hoc_query-empty_db` (json, I_QUERY_SERVICE.execute_ad_hoc_query-empty_db): adhoc empty_db golden mismatch (suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"}], served=[{"name":"#0"}]
- `I_QUERY_SERVICE.execute_stored_query-empty_db` (json, I_QUERY_SERVICE.execute_stored_query-empty_db): stored empty_db golden mismatch (suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"}], served=[{"name":"#0"}]
- `QUERY-FIXTURE-A-empty_db` (json, query/expected_results/empty_db/A §columns/full): 0/27 A/empty_db goldens matched (0 skipped); first divergence: A/100_get_ehrs.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"}], served=[{"name":"#0"}]
- `QUERY-FIXTURE-B-empty_db` (json, query/expected_results/empty_db/B §columns/full): 17/18 B/empty_db goldens matched (3 skipped); first divergence: B/103_get_compositions_within_timewindow.json: valid query rejected with status 400 (body: {"error":"Bad Request","message":"bad request: invalid AQL: found 'Identifier(\"TIMEWINDOW\")' at 5..6"})
- `QUERY-FIXTURE-C-empty_db` (json, query/expected_results/empty_db/C §columns/full): 10/11 C/empty_db goldens matched (0 skipped); first divergence: C/103_get_entries_empty_db.json: valid query rejected with status 400 (body: {"error":"Bad Request","message":"bad request: invalid AQL: found 'Identifier(\"TIMEWINDOW\")' at 18..19"})
- `QUERY-FIXTURE-D-empty_db` (json, query/expected_results/empty_db/D §columns/full): 7/18 D/empty_db goldens matched (8 skipped); first divergence: D/200_select_data_values_from_all_ehrs_contains_composition.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"},{"name":"#1","path":"/time_created/value"},{"name":"#2","path":"/system_id/value"}], served=[{"name":"#0"},{"name":"#1"},{"name":"#2"}]
- `QUERY-FIXTURE-A-loaded_db` (json, query/expected_results/loaded_db/A §columns): 0/23 A/loaded_db goldens matched (4 skipped); first divergence: A/100_get_ehrs.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"}], served=[{"name":"#0"}]
- `QUERY-FIXTURE-B-loaded_db` (json, query/expected_results/loaded_db/B §columns): 14/18 B/loaded_db goldens matched (6 skipped); first divergence: B/103_get_compositions_within_timewindow.json: valid query rejected with status 400 (body: {"error":"Bad Request","message":"bad request: invalid AQL: found 'Identifier(\"TIMEWINDOW\")' at 5..6"})
- `QUERY-FIXTURE-D-loaded_db` (json, query/expected_results/loaded_db/D §columns): 0/9 D/loaded_db goldens matched (17 skipped); first divergence: D/200_select_data_values_from_all_ehrs_contains_composition.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"},{"name":"#1","path":"/time_created/value"},{"name":"#2","path":"/system_id/value"}], served=[{"name":"#0"},{"name":"#1"},{"name":"#2"}]
- `CONT-OBS-state_ex_opt-protocol_ex_opt` (json, CONT-OBS-state_ex_opt-protocol_ex_opt): OBSERVATION without data (RM/schema OBSERVATION.data existence.lower): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-OBS-state_ex_opt-protocol_ex_mand` (json, CONT-OBS-state_ex_opt-protocol_ex_mand): OBSERVATION without data (RM/schema OBSERVATION.data existence.lower): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-OBS-state_ex_mand-protocol_ex_opt` (json, CONT-OBS-state_ex_mand-protocol_ex_opt): OBSERVATION without data (RM/schema OBSERVATION.data existence.lower): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-OBS-state_ex_mand-protocol_ex_mand` (json, CONT-OBS-state_ex_mand-protocol_ex_mand): OBSERVATION without data (RM/schema OBSERVATION.data existence.lower): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-EVENT-state_ex_opt` (json, CONT-EVENT-state_ex_opt): POINT_EVENT without data (RM/schema POINT_EVENT.data existence.lower): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-EVENT-state_ex_mand` (json, CONT-EVENT-state_ex_mand): POINT_EVENT without data (RM/schema POINT_EVENT.data existence.lower): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-ITEM_STR-type_item_tree` (json, CONT-ITEM_STR-type_item_tree): ITEM_STRUCTURE ITEM_TREE slot filled with ITEM_LIST (Class not allowed): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-ITEM_STR-type_item_list` (json, CONT-ITEM_STR-type_item_list): ITEM_STRUCTURE ITEM_LIST slot filled with ITEM_TREE (Class not allowed): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-ITEM_STR-type_item_table` (json, CONT-ITEM_STR-type_item_table): ITEM_STRUCTURE ITEM_TABLE slot filled with ITEM_TREE (Class not allowed): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-ITEM_STR-type_item_single` (json, CONT-ITEM_STR-type_item_single): ITEM_STRUCTURE ITEM_SINGLE slot filled with ITEM_TREE (Class not allowed): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-DV_BOOLEAN-anything_allowed` (json, CONT-DV_BOOLEAN-anything_allowed): DV_BOOLEAN without value (RM/Schema mandatory): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-DV_TEXT-validate_open` (json, CONT-DV_TEXT-validate_open): DV_TEXT without value (RM/Schema mandatory): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-DV_ORDINAL-validate_open` (json, CONT-DV_ORDINAL-validate_open): DV_ORDINAL without value (RM/Schema mandatory): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-DV_COUNT-validate_open` (json, CONT-DV_COUNT-validate_open): DV_COUNT without magnitude (RM/Schema mandatory): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-DV_PROPORTION-validate_any_fraction` (json, CONT-DV_PROPORTION-validate_any_fraction): DV_PROPORTION type 0 (ratio) not in list [3,4] (C_INTEGER.list): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-DV_DATE_TIME-validate_open` (json, CONT-DV_DATE_TIME-validate_open): DV_DATE_TIME without value (RM/Schema mandatory): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-DV_DATE_TIME-validate_constraint` (json, CONT-DV_DATE_TIME-validate_constraint): DV_DATE_TIME '2021' missing mandatory month/day/time (C_DATE_TIME validity): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- `CONT-DV_EHR_URI-validate_open` (json, CONT-DV_EHR_URI-validate_open): DV_EHR_URI without value (RM/Schema mandatory): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
