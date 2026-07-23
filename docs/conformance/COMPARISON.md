# openEHR CDR conformance comparison (generated)

> **Measured, not asserted.** Every cell below is derived from the two
> committed `results.json`/`verdicts.json` sets produced by the CNF 2.0
> reference runner (`tools/cnf-runner`) executing the SAME committed
> catalogue against each system, each with its own committed party statement
> (`tools/cnf-runner/party/<sut>/`). Nothing here is hand-entered
> (`scripts/render-comparison.sh`; CI: `scripts/check-conformance-numbers.sh`).
>
> - A capability a party's statement does not claim reads **not claimed** and
>   never gates its verdicts; a ground unrealizable on a party's topology or
>   technology profile reads **N/A with a machine citation**, never fail.
> - This comparison makes **no certification claim on behalf of any other
>   vendor** -- each column is computed from that SUT's own run.
> - Where the comparison SUT out-performs ehrbase-rs, its cell reads pass
>   while ours reads fail -- stated plainly, not hidden.

## Systems under test

| | ehrbase-rs | upstream (Java) |
|---|---|---|
| Product | ehrbase-rs 3.7.0 | ehrbase-java 2.34.0 |
| Run date | 2026-07-23 | 2026-07-22 |
| Party statement | `tools/cnf-runner/party/ehrbase-rs/` | `tools/cnf-runner/party/ehrbase-java/` |
| Stack | root compose, built from the current sources | `docker/sut-ehrbase-java.yml` (official images) |

## Methodology

Both systems execute the **same committed CNF 2.0 catalogue** (390 case-by-format
executions) through the same reference runner (`tools/cnf-runner`), each on
fresh volumes with its own committed party set: the ixit names the reachable
instances (upstream declares no readonly principal), and the statement (the
ICS) declares the claimed capabilities and ambiguity-register options —
ISO/IEC 9646-style test selection excuses undeclared option branches as N/A
with a citation, never as silent skips. Verdicts are pure functions of
(statement, results, catalogue, capability matrix).

## Profile verdicts

| Profile | ehrbase-rs | upstream (Java) |
|---|---|---|
| CORE | pass | fail |
| STANDARD | pass | fail |
| OPTIONS | pass | not claimed |
| SEC-BASIC | pass | not claimed |

## Outcome totals

Runs compared: **ehrbase-rs** (run of 2026-07-23) vs **upstream EHRbase
2.34.0** (run of 2026-07-22) — the SAME catalogue through the same
runner, each with its own committed party statement.

| | executed | passed | failed | errored | skipped | N/A |
|---|---|---|---|---|---|---|
| **ehrbase-rs** | 390 | 323 | 0 | 0 | 0 | 67 |
| **upstream (Java)** | 390 | 158 | 126 | 38 | 0 | 68 |

An **errored** row is inconclusive (the wire answered outside the operation's
bound outcome map), never counted as a failure. An **N/A** row carries a
machine-readable citation (an undeclared option branch, an unrealizable wire
on the technology profile, or a ground the party's topology cannot
establish).

## Capability-by-capability

Evidence tokens from each party's computed verdicts: **passed** (every
gating case green), **failed** (at least one gating case red), **unrealized**
(every case excused by a register citation — e.g. AMB-41: ADL 1.4 archetype
provisioning has no ITS-REST wire), **not_evidenced** (claimed, no gating
case ran), **no_cases**, or **not claimed** (absent from that party's ICS).

| Capability | ehrbase-rs | upstream (Java) |
|---|---|---|
| ActivityReport | unrealized | not_evidenced |
| Adl14ArchetypeProvisioning | unrealized | unrealized |
| Adl14OptProvisioning | passed | failed |
| Adl2ArchetypeProvisioning | unrealized | not_evidenced |
| Adl2OptProvisioning | passed | not_evidenced |
| AdminApi | passed | failed |
| AnonymousEhrs | passed | not_evidenced |
| AqlAdvanced | passed | not_evidenced |
| AqlBasic | passed | failed |
| AqlTerminology | passed | not_evidenced |
| ArchetypeValidation | passed | failed |
| AuditAccountability | passed | not_evidenced |
| AuthenticatedAccess | passed | passed |
| AuthorizationSeparation | passed | not_evidenced |
| BulkEhrLoad | no_cases | no_cases |
| ChangeSets | passed | failed |
| CompositionOps | passed | failed |
| DefinitionApi | passed | not_evidenced |
| DemographicApi | passed | not_evidenced |
| DemographicArchetypeValidation | no_cases | no_cases |
| DemographicArchive | unrealized | not_evidenced |
| DirectoryOps | passed | failed |
| EhrApi | passed | passed |
| EhrArchive | unrealized | not_evidenced |
| EhrDemographicSeparation | passed | passed |
| EhrDumpLoad | unrealized | not_evidenced |
| EhrExtract | unrealized | not_evidenced |
| EhrOperations | passed | failed |
| EhrStatus | passed | failed |
| MessageApi | unrealized | not_evidenced |
| PartyOperations | passed | not_evidenced |
| PartyRelationshipOperations | unrealized | not_evidenced |
| PhysicalDeletion | passed | failed |
| QueryApi | passed | passed |
| QueryProvisioning | passed | passed |
| Signing | passed | not_evidenced |
| SimplifiedFormats | passed | not_evidenced |
| Tds | unrealized | not_evidenced |
| Versioning | passed | failed |

## Failures — both directions

### ehrbase-rs failures (with the upstream outcome on the identical case)

| Case | Format | Failure | upstream outcome |
|---|---|---|---|
| — | — | *none — zero failing cases* | — |

### Upstream failures by schedule chapter

| Chapter | failed cases |
|---|---|
| CONT | 47 |
| SF | 42 |
| I_EHR_COMPOSITION | 9 |
| I_EHR_STATUS | 8 |
| I_QUERY_SERVICE | 5 |
| I_EHR_CONTRIBUTION | 4 |
| I_EHR_DIRECTORY | 4 |
| I_DEFINITION_ADL2 | 3 |
| I_ADMIN_SERVICE | 1 |
| I_DEFINITION_ADL14 | 1 |
| I_EHR_SERVICE | 1 |
| SIG | 1 |

<details><summary>Every upstream-failed case, with the ehrbase-rs outcome on the identical case</summary>

| Case | Format | Upstream failure | ehrbase-rs outcome |
|---|---|---|---|
| CONT-COMPOSITION-content_cardinality | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_CODED_TEXT-validate_local_codes | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_DATE-validate_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_DATE-validate_range | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_DATE_TIME-validate_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_DATE_TIME-validate_range | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_DURATION-validate_fields | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_DURATION-validate_fields_range | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_DURATION-validate_open | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_IDENTIFIER-validate_all_list | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_IDENTIFIER-validate_all_pattern | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_range | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_DATE-validate_open | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_range | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_open | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_INTERVAL_DV_DURATION-validate_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_DURATION-validate_open | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_INTERVAL_DV_ORDINAL-validate_constraint | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_INTERVAL_DV_ORDINAL-validate_open | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_fraction | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_integer_fraction | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_SCALE-validate_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_SCALE-validate_open | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_range | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_INTERVAL_DV_TIME-validate_open | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_MULTIMEDIA-validate_media_type | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_ORDINAL-validate_constraint | — | expected `created`, observed `validation_failed` | passed |
| CONT-DV_PARSABLE-validate_value_formalism | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_PROPORTION-validate_any_fraction | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_PROPORTION-validate_fraction | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_PROPORTION-validate_integer_fraction | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_PROPORTION-validate_open | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_PROPORTION-validate_ratio | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_QUANTITY-validate_property | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_SCALE-validate_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_TIME-validate_constraint | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_TIME-validate_range | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_URI-validate_list | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_URI-validate_open | — | expected `validation_failed`, observed `created` | passed |
| CONT-DV_URI-validate_pattern | — | expected `validation_failed`, observed `created` | passed |
| CONT-EVENT-type_narrowing | — | expected `validation_failed`, observed `created` | passed |
| CONT-HISTORY-events_cardinality | — | expected `created`, observed `validation_failed` | passed |
| CONT-ITEM_STRUCTURE-type_narrowing | — | expected `created`, observed `validation_failed` | passed |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_existing | — | expected `ok_empty`, observed `not_found` | passed |
| I_DEFINITION_ADL14.get_opt-retrieve_single | — | equivalent: retrieved content differs from committed (modulo the normative ignore-set); go | passed |
| I_DEFINITION_ADL2.get_artefact-example | canonical-json | expected `ok`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL2.get_artefact-example_unknown | — | expected `not_found`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL2.get_artefact-retrieve | — | expected `ok`, observed `not_found` | passed |
| I_EHR_COMPOSITION.get_composition_at_time | — | equivalent: retrieved content differs from committed (modulo the normative ignore-set); go | passed |
| I_EHR_COMPOSITION.get_composition_at_time-no_time_arg | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_COMPOSITION.get_composition_at_times | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_COMPOSITION.get_composition_latest | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_COMPOSITION.get_composition_version | — | equivalent: retrieved content differs from committed (modulo the normative ignore-set); go | passed |
| I_EHR_COMPOSITION.get_composition_versions | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_COMPOSITION.update_composition-event | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_COMPOSITION.update_composition-non_existent | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_COMPOSITION.update_composition-wrong_template | — | expected `template_mismatch`, observed `precondition_missing` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type | — | expected `conflict`, observed `validation_failed` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type_deleted | — | expected `conflict`, observed `not_found` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-persistent_composition | — | expected `created`, observed `validation_failed` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_creation | — | expected `created`, observed `validation_failed` | passed |
| I_EHR_DIRECTORY.delete_directory-ehr_with_directory | — | expected `ok_empty`, observed `not_found` | passed |
| I_EHR_DIRECTORY.delete_directory-empty_ehr | — | expected `not_found`, observed `precondition_failed` | passed |
| I_EHR_DIRECTORY.get_directory-directory_with_structure | — | equivalent: retrieved content differs from committed (modulo the normative ignore-set); go | passed |
| I_EHR_DIRECTORY.update_directory-empty_ehr | — | expected `not_found`, observed `precondition_failed` | passed |
| I_EHR_SERVICE.create_ehr-invalid_status | — | expected `validation_failed`, observed `created` | passed |
| I_EHR_STATUS.clear_ehr_modifiable-bad_ehr | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_STATUS.clear_ehr_modifiable-existing_ehr | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.clear_ehr_queryable-bad_ehr | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_STATUS.clear_ehr_queryable-existing_ehr | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_modifiable-bad_ehr | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_modifiable-existing_ehr | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_queryable-bad_ehr | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_queryable-existing_ehr | — | expected `updated`, observed `precondition_missing` | passed |
| I_QUERY_SERVICE.execute_ad_hoc_query-empty_db_bare_ehr | — | row count 100 != expected 1 | passed |
| I_QUERY_SERVICE.execute_ad_hoc_query-empty_db_shapes | — | row count 100 != expected 0 | passed |
| I_QUERY_SERVICE.execute_ad_hoc_query-terminology_expand_matches | — | expected `ok`, observed `invalid_query` | passed |
| I_QUERY_SERVICE.execute_ad_hoc_query-where_magnitude | — | row count 36 != expected 6 | passed |
| I_QUERY_SERVICE.execute_stored_query-empty_db | — | row count 100 != expected 0 | passed |
| SF-CTX-composer_name | — | expected `created`, observed `unsupported_media` | passed |
| SF-CTX-composer_self | — | expected `created`, observed `unsupported_media` | passed |
| SF-CTX-missing_mandatory | — | expected `validation_failed`, observed `unsupported_media` | passed |
| SF-CTX-participations_forms | — | expected `created`, observed `unsupported_media` | passed |
| SF-CTX-vocabulary_mapping | — | expected `created`, observed `unsupported_media` | passed |
| SF-EXAMPLE-accept_forms | — | expected `ok`, observed `not_acceptable` | passed |
| SF-FIELDID-structure | — | expected `created`, observed `unsupported_media` | passed |
| SF-FLAT-commit_roundtrip_ctx_defaults | — | expected `created`, observed `unsupported_media` | passed |
| SF-FLAT-missing_template_id | — | expected `missing_template_id`, observed `unsupported_media` | passed |
| SF-FLAT-reject_cardinality | — | expected `validation_failed`, observed `unsupported_media` | passed |
| SF-FLAT-reject_datatype_mismatch | — | expected `validation_failed`, observed `unsupported_media` | passed |
| SF-FLAT-reject_other_closed_list | — | expected `validation_failed`, observed `unsupported_media` | passed |
| SF-FLAT-reject_other_with_code | — | expected `validation_failed`, observed `unsupported_media` | passed |
| SF-FLAT-reject_terminology_binding | — | expected `validation_failed`, observed `unsupported_media` | passed |
| SF-FLAT-reject_unknown_field | — | expected `validation_failed`, observed `unsupported_media` | passed |
| SF-INDEX-multi_event_commit | — | expected `created`, observed `unsupported_media` | passed |
| SF-INDEX-semantics | — | expected `created`, observed `unsupported_media` | passed |
| SF-LEVELS-collapsed_wrappers | — | expected `created`, observed `unsupported_media` | passed |
| SF-LEVELS-container_attribute_elision | — | expected `created`, observed `unsupported_media` | passed |
| SF-LEVELS-lab_panel_example | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-attribute_suffix_table | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-context | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-dv_ordinal_proportion_count | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-dv_quantity | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-dv_text_coded | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-entries | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-events_audit | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-interval_reference_range | — | expected `created`, observed `validation_failed` | passed |
| SF-MAP-multimedia_parsable | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-party | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-simple_values | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-structure | — | expected `created`, observed `unsupported_media` | passed |
| SF-MAP-temporal | — | expected `created`, observed `unsupported_media` | passed |
| SF-RAW-embedding | — | expected `created`, observed `unsupported_media` | passed |
| SF-RAW-missing_type | — | expected `validation_failed`, observed `unsupported_media` | passed |
| SF-RAW-structured_embedding | — | expected `created`, observed `unsupported_media` | passed |
| SF-RMATTR-normal_range_commit | — | expected `created`, observed `unsupported_media` | passed |
| SF-RMATTR-underscore_mapping | — | expected `created`, observed `unsupported_media` | passed |
| SF-STRUCT-arrays_single_cardinality | — | expected `created`, observed `unsupported_media` | passed |
| SF-STRUCT-empty_object_omission | — | expected `created`, observed `validation_failed` | passed |
| SF-STRUCT-style_rules | — | expected `created`, observed `validation_failed` | passed |
| SF-STRUCTURED-commit_roundtrip | — | expected `created`, observed `unsupported_media` | passed |
| SIG-VERSION-across_version_kinds | — | expected `updated`, observed `precondition_missing` | passed |

</details>
