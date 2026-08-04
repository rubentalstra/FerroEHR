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
> - Where the comparison SUT out-performs ferroehr, its cell reads pass
>   while ours reads fail -- stated plainly, not hidden.

## Systems under test

| | ferroehr | EHRbase |
|---|---|---|
| Product | ferroehr 3.17.2 | ehrbase-java 2.34.0 |
| Run date | 2026-08-03 | 2026-07-28 |
| Party statement | `tools/cnf-runner/party/ferroehr/` | `tools/cnf-runner/party/ehrbase-java/` |
| Stack | root compose, built from the current sources | `docker/sut-ehrbase-java.yml` (official images) |

## Methodology

Both systems execute the **same committed CNF 2.0 catalogue** (991 case-by-format
executions) through the same reference runner (`tools/cnf-runner`), each on
fresh volumes with its own committed party set: the ixit names the reachable
instances (EHRbase declares no readonly principal), and the statement (the
ICS) declares the claimed capabilities, spec versions, and ambiguity-register
options — ISO/IEC 9646-style test selection excuses undeclared option
branches, unclaimed capabilities, and release-dated behaviour outside the
declared versions as N/A with a citation, never as silent skips. Verdicts are
pure functions of (statement, results, catalogue, capability matrix).

**The declared-version delta matters and is stated, not hidden:** ferroehr
declares ITS-REST **1.1.0**
while EHRbase declares ITS-REST
**1.0.3** —
the catalogue realizes 1.1.0, so every Release-1.1.0-dated behaviour (the
Demographic API, ITEM_TAGs, Simplified Formats on the wire, the admin EHR
delete, the weak-`ETag`/`Location` header forms, …) is cited N/A for the
1.0.3 declaration rather than driven against a release EHRbase never
claimed. The verdict-bearing comparison below is therefore each party's
**in-scope subset**, never the raw record.

## Profile verdicts

| Profile | ferroehr | EHRbase |
|---|---|---|
| CORE | fail | fail |
| STANDARD | fail | fail |
| OPTIONS | pass | not claimed |
| SEC-BASIC | fail | not claimed |

## In-scope outcomes

Runs compared: **ferroehr** (run of 2026-08-03) vs **EHRbase
2.34.0** (run of 2026-07-28) — the SAME catalogue through the same
runner, each with its own committed party statement. Per the presentation
rule, the headline is each party's VERDICT SCOPE (the cases its own
declarations select), never the raw record: a raw count would book
release-dated and unclaimed surfaces against a party that never claimed
them.

| | verdict scope (selected) | driven | in-scope passed | in-scope failed | in-scope inconclusive |
|---|---|---|---|---|---|
| **ferroehr** | 991 | 954 | 565 | 387 | 2 |
| **EHRbase** | 499 | 459 | 136 | 132 | 191 |

An **inconclusive** row's wire answered outside the operation's bound outcome
map, or its required ground could not be established (e.g. a refused
provisioning exchange) — never counted as a failure of the behaviour under
test. Every not-run row in the full committed record
(`docs/conformance/<sut>/results.json`) carries a machine-readable
citation: an undeclared option branch, an unclaimed capability, a
release-dated behaviour outside the declared spec versions, or a ground the
party's topology cannot establish.

## Capability-by-capability

Evidence tokens from each party's computed verdicts: **passed** (every
gating case green), **failed** (at least one gating case red),
**inconclusive** (a gating case neither passed nor failed cleanly),
**not_evidenced** (claimed, but no gating case produced a verdict — there
is no excused state: a required capability without passing evidence fails
its tier, whichever party claims it), or **not claimed** (absent from that
party's ICS).

| Capability | ferroehr | EHRbase |
|---|---|---|
| ActivityReport | passed | not_evidenced |
| Adl14ArchetypeProvisioning | passed | not_evidenced |
| Adl14OptProvisioning | passed | failed |
| Adl2ArchetypeProvisioning | passed | not_evidenced |
| Adl2OptProvisioning | failed | not_evidenced |
| AdminApi | passed | not_evidenced |
| AnonymousEhrs | failed | not_evidenced |
| AqlAdvanced | passed | inconclusive |
| AqlBasic | failed | failed |
| AqlTerminology | passed | not_evidenced |
| ArchetypeValidation | failed | failed |
| AuditAccountability | passed | not_evidenced |
| AuthenticatedAccess | passed | passed |
| AuthorizationSeparation | passed | not_evidenced |
| BulkEhrLoad | failed | not_evidenced |
| ChangeSets | failed | failed |
| CompositionOps | failed | inconclusive |
| DefinitionApi | passed | failed |
| DemographicApi | failed | not_evidenced |
| DemographicArchetypeValidation | failed | not_evidenced |
| DemographicArchive | passed | not_evidenced |
| DirectoryOps | failed | failed |
| EhrApi | passed | failed |
| EhrArchive | passed | not_evidenced |
| EhrDemographicSeparation | passed | passed |
| EhrDumpLoad | passed | not_evidenced |
| EhrExtract | inconclusive | not_evidenced |
| EhrOperations | inconclusive | failed |
| EhrStatus | failed | failed |
| ItemTags | failed | not_evidenced |
| MessageApi | passed | not_evidenced |
| PartyOperations | failed | not_evidenced |
| PartyRelationshipOperations | failed | not_evidenced |
| PhysicalDeletion | passed | not_evidenced |
| QueryApi | failed | failed |
| QueryProvisioning | passed | failed |
| Signing | failed | not_evidenced |
| SimplifiedFormats | failed | not_evidenced |
| SmartAppLaunch | passed | not_evidenced |
| SystemApi | passed | not_evidenced |
| Tds | passed | not_evidenced |
| TemplateExamples | passed | not_evidenced |
| Versioning | failed | failed |

## Failures — both directions

### ferroehr failures (with the EHRbase outcome on the identical case)

| Case | Format | Failure | EHRbase outcome |
|---|---|---|---|
| CONT-COMP-content_card_1plus-context_any | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_1plus-context_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_3plus-context_any | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_3plus-context_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_3to5-context_any | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_3to5-context_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_any-context_any | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_any-context_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_mand-context_any | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_mand-context_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_opt-context_any | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMP-content_card_opt-context_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMPOSITION-content_cardinality_count6 | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-COMPOSITION-context_existence | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_BOOLEAN-anything_allowed | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_BOOLEAN-only_false_allowed | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_BOOLEAN-only_true_allowed | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_CODED_TEXT-validate_ext_term | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_CODED_TEXT-validate_local_codes | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_CODED_TEXT-validate_open | — | expected `validation_failed`, observed `bad_request` | failed |
| CONT-DV_COUNT-validate_list | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_COUNT-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_COUNT-validate_range | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_DATE-validate_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_DATE-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_DATE-validate_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_DATE_TIME-validate_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_DATE_TIME-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_DATE_TIME-validate_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_DURATION-validate_fields | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_DURATION-validate_fields_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_DURATION-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_DURATION-validate_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_EHR_URI-validate_list | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_EHR_URI-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_EHR_URI-validate_pattern | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_IDENTIFIER-validate_all_list | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_IDENTIFIER-validate_all_pattern | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_COUNT-validate_lower_upper | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_COUNT-validate_lower_upper_list | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_COUNT-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_DATE-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_DATE-validate_open_mixed_precision | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_DURATION-validate_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_DURATION-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_DURATION-validate_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_ORDINAL-validate_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_ORDINAL-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_fraction | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_integer_fraction | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_percentage | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_unitary | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_QUANTITY-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_QUANTITY-validate_upper_lower | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_SCALE-validate_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_SCALE-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_INTERVAL_DV_TIME-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_INTERVAL_DV_TIME-validate_open_mixed_precision | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_MULTIMEDIA-validate_media_type | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_MULTIMEDIA-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_ORDINAL-validate_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_ORDINAL-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_PARSABLE-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_PARSABLE-validate_value_formalism | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_PROPORTION-validate_any_fraction | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_PROPORTION-validate_fraction | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_PROPORTION-validate_integer_fraction | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_PROPORTION-validate_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_PROPORTION-validate_percent | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_PROPORTION-validate_ratio | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_PROPORTION-validate_ratio_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_PROPORTION-validate_unitary | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_QUANTITY-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_QUANTITY-validate_property | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_QUANTITY-validate_property_units | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_QUANTITY-validate_property_units_mag | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_SCALE-validate_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_SCALE-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_TEXT-validate_list | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_TEXT-validate_open | — | expected `validation_failed`, observed `bad_request` | failed |
| CONT-DV_TIME-validate_constraint | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_TIME-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_TIME-validate_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-DV_URI-validate_list | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-DV_URI-validate_open | — | expected `validation_failed`, observed `bad_request` | errored |
| CONT-DV_URI-validate_pattern | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| CONT-EVENT-state_ex_mand | — | expected `validation_failed`, observed `bad_request` | failed |
| CONT-EVENT-state_ex_opt | — | expected `validation_failed`, observed `bad_request` | failed |
| CONT-EVENT-type_any | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-EVENT-type_interval_event | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-EVENT-type_point_event | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_1plus-summary_ex_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_1plus-summary_ex_opt | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_3plus-summary_ex_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_3plus-summary_ex_opt | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_3to5-summary_ex_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_3to5-summary_ex_opt | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_any-summary_ex_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_any-summary_ex_opt | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_mand-summary_ex_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_mand-summary_ex_opt | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_opt-summary_ex_mand | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HIST-events_card_opt-summary_ex_opt | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-HISTORY-events_cardinality_count6 | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-ITEM_STR-type_any | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-ITEM_STR-type_item_list | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-ITEM_STR-type_item_single | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-ITEM_STR-type_item_table | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-ITEM_STR-type_item_tree | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| CONT-OBS-state_ex_mand-protocol_ex_mand | — | expected `validation_failed`, observed `bad_request` | failed |
| CONT-OBS-state_ex_mand-protocol_ex_opt | — | expected `validation_failed`, observed `bad_request` | failed |
| CONT-OBS-state_ex_opt-protocol_ex_mand | — | expected `validation_failed`, observed `bad_request` | failed |
| CONT-OBS-state_ex_opt-protocol_ex_opt | — | expected `validation_failed`, observed `bad_request` | failed |
| I_DEFINITION_ADL2.get_artefact-version_prefix | — | header ETag: pattern "W/\"<template_id>\"" names <template_id>, which is neither a capture | failed |
| I_DEFINITION_ADL2.upload_artefact-duplicate_conflict | — | header ETag: pattern "W/\"<template_id>\"" names <template_id>, which is neither a capture | failed |
| I_DEFINITION_ADL2.upload_artefact-valid_opt | — | header ETag: pattern "W/\"<template_id>\"" names <template_id>, which is neither a capture | failed |
| I_DEFINITION_ADL2.valid_artefact-valid | — | header ETag: pattern "W/\"<template_id>\"" names <template_id>, which is neither a capture | failed |
| I_DEMOGRAPHIC_SERVICE.create_party-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.create_party-archetyped_content_accepted | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.create_party-capabilities_present_empty | — | expected `bad_request`, observed `created` | not_applicable |
| I_DEMOGRAPHIC_SERVICE.create_party-client_supplied_uid | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.create_party-inline_relationships_verbatim | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.create_party-item_tag_wrapper_headers | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.create_party-kind_parity_agent | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.create_party-kind_parity_group | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.create_party-kind_parity_organisation | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.create_party-kind_parity_role | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.create_party-relationship_source_is_the_container | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.create_party-return_identifier | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.create_party-version_lifecycle | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.create_party_relationship-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.delete_party-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.delete_party-already_deleted | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.delete_party-stale_version_conflict | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.delete_party_relationship-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.delete_party_relationship-stale_version_conflict | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.get_party-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.get_party-wrong_kind_container | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.get_party-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.get_party_at_time-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.get_party_at_time-deleted_current | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.get_party_at_time-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.get_party_at_version-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.get_party_at_version-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_time-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_version-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.update_party-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.update_party-bbbb | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.update_party-case_variant_if_match | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.update_party-contacts_present_empty | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.update_party-invalid_body | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.update_party-missing_if_match | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.update_party-prefer_minimal | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.update_party-root_identity_mismatch | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.update_party-unknown_preceding_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.update_party-weak_etag_accepted | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.update_party-weak_etag_stale | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-aaaa | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-bbbb | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-invalid_body | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-missing_if_match | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-root_identity_mismatch | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_DEMOGRAPHIC_SERVICE.versioned_party_version_at_time | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_DEMOGRAPHIC_SERVICE.versioned_party_version_read | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_DEMOGRAPHIC_SERVICE.versioned_party_version_unknown | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_EHR_COMPOSITION.create_composition-audit_system_id_declared | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_EHR_COMPOSITION.create_composition-bulk_load_depth | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.create_composition-cluster_no_items | — | expected `validation_failed`, observed `bad_request` | — |
| I_EHR_COMPOSITION.create_composition-datetime_comma_fraction | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-datetime_lexical | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.create_composition-event | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.create_composition-feeder_audit_roundtrip | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-invalid_event | — | expected `validation_failed`, observed `bad_request` | errored |
| I_EHR_COMPOSITION.create_composition-invalid_persistent | — | expected `validation_failed`, observed `bad_request` | errored |
| I_EHR_COMPOSITION.create_composition-item_tag_wrapper_headers | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-largest_published_form | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-links_roundtrip | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-null_empty_absent | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.create_composition-participation_time_roundtrip | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-persistent | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.create_composition-prefer_absent | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::<n>\"" names <versioned_obj | errored |
| I_EHR_COMPOSITION.create_composition-prefer_minimal | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::<n>\"" names <versioned_obj | errored |
| I_EHR_COMPOSITION.create_composition-same_opt_twice | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.create_composition-subject_external_ref | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-terminology_binding_member | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-terminology_binding_unresolvable_fail_open | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.create_composition-version_bare_deprecated | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.create_composition-version_lifecycle | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.create_composition-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.delete_composition-already_deleted | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.delete_composition-audit_headers | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.delete_composition-contradictory_lifecycle_header | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.delete_composition-etag_names_new_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.delete_composition-event | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.delete_composition-not_latest_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.delete_composition-persistent | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_time | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_time-before_first_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_time-deleted_at_time | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_time-future | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_time-malformed | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_time-no_time_arg | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_time-simplified_forms | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_EHR_COMPOSITION.get_composition_at_time-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_times | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_version-deleted_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_at_version-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_latest | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_latest-deleted_head | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_latest-identifier_forms_agree | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_latest-simplified_forms | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_EHR_COMPOSITION.get_composition_latest-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_latest-xml_namespace_v1 | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.get_composition_latest-xml_namespace_v2 | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.get_composition_latest-xml_root_namespace | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.get_composition_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_composition_versions | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-at_time_before_first_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-at_time_future | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-at_time_malformed | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-at_time_omitted | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-container_shape | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-deleted_version_envelope | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-version_of_other_container | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-version_unknown | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.get_versioned_composition-version_xml_root | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.get_versioned_composition-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.has_composition | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.has_composition-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-audit_amendment | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-audit_bare_deprecated | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-audit_change_type_invalid | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-audit_change_type_mismatch | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-audit_system_id | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_EHR_COMPOSITION.update_composition-body_uid_mismatch | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-deleted_lifecycle_header | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.update_composition-event | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-feeder_audit_carried | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.update_composition-feeder_audit_removed | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.update_composition-flat | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_EHR_COMPOSITION.update_composition-missing_if_match | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-persistent | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-prefer_absent | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-prefer_minimal | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-return_identifier | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_EHR_COMPOSITION.update_composition-same_content | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_EHR_COMPOSITION.update_composition-stale_if_match | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-structured | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_EHR_COMPOSITION.update_composition-unknown_preceding_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-wrong_template | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_COMPOSITION.update_composition-xml | canonical-xml | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_EHR_CONTRIBUTION.commit_contribution-deleted_lifecycle_with_data | — | header ETag: value "W/\"019fcd6c-d514-7703-9491-b2c8d8413408::ferroehr.local::1\"" does no | — |
| I_EHR_DIRECTORY.create_directory-invalid_folder | — | expected `validation_failed`, observed `bad_request` | errored |
| I_EHR_DIRECTORY.create_directory-items_by_value | — | expected `validation_failed`, observed `bad_request` | — |
| I_EHR_DIRECTORY.create_directory-link_missing_meaning | — | expected `validation_failed`, observed `bad_request` | — |
| I_EHR_DIRECTORY.delete_directory-ehr_with_directory | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::<n>\"" names <versioned_obj | failed |
| I_EHR_DIRECTORY.get_directory-deleted_head | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::<n>\"" names <versioned_obj | failed |
| I_EHR_DIRECTORY.get_directory_at_time-deleted_at_time | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::<n>\"" names <versioned_obj | failed |
| I_EHR_DIRECTORY.get_directory_at_version-deleted_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::<n>\"" names <versioned_obj | failed |
| I_EHR_DIRECTORY.update_directory-invalid_folder | — | expected `validation_failed`, observed `precondition_missing` | failed |
| I_EHR_SERVICE.create_ehr-bulk_load_population | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | failed |
| I_EHR_STATUS.clear_ehr_modifiable-invalid_body | — | expected `bad_request`, observed `validation_failed` | errored |
| I_EHR_STATUS.clear_ehr_queryable-invalid_body | — | expected `bad_request`, observed `validation_failed` | errored |
| I_EHR_STATUS.set_ehr_modifiable-invalid_body | — | expected `bad_request`, observed `validation_failed` | errored |
| I_EHR_STATUS.set_ehr_queryable-invalid_body | — | expected `bad_request`, observed `validation_failed` | errored |
| I_ITS_REST_ITEM_TAGS.agent_tags_delete-key_scoped | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.agent_tags_delete-unknown_key | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.agent_tags_update-non_array_body | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_delete-key_scoped | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_delete-second_delete_not_found | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_delete-unknown_key | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_delete-version_container_disjoint | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_get-container_target_shape | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_get-cross_space_party_uid | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_get-supersession_does_not_move_tags | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_get-version_target_shape | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_get-xml_not_acceptable | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-duplicate_identity_last_wins | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-empty_target_path_absent | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-empty_value_invariant | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-key_target_path_identity | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-no_reversioning | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-non_array_body | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-prefer_identifier | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-version_container_disjoint | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-wrong_media | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.demographic_tags_get-space_wide_listing | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.ehr_status_tags_delete-wrong_kind_uid | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.ehr_status_tags_get-wrong_kind_uid | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.ehr_status_tags_update-wrong_kind_uid | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.ehr_tags_get-ehr_wide_listing | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.group_tags_delete-key_scoped | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.group_tags_delete-unknown_key | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.group_tags_update-non_array_body | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.organisation_tags_delete-key_scoped | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.organisation_tags_delete-wrong_kind_uid | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.organisation_tags_get-wrong_kind_uid | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.organisation_tags_update-non_array_body | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.organisation_tags_update-wrong_kind_uid | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_delete-key_scoped | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_delete-unknown_key | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_delete-version_container_disjoint | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_get-container_target_shape | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_get-cross_space_composition_uid | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_get-version_target_shape | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_update-empty_target_path_absent | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_update-key_target_path_identity | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_update-non_array_body | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.person_tags_update-version_container_disjoint | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.role_tags_delete-key_scoped | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.role_tags_delete-unknown_key | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_ITEM_TAGS.role_tags_update-non_array_body | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_REVISION_HISTORY.versioned_composition_revision_history-two_versions | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_ITS_REST_REVISION_HISTORY.versioned_party_revision_history-two_versions | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_VERSIONED_PARTY.versioned_party_get-container_shape | canonical-json | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_ITS_REST_VERSIONED_PARTY.versioned_party_get-xml_not_acceptable | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| I_QUERY_SERVICE.execute_ad_hoc_query-all_versions | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_QUERY_SERVICE.execute_ad_hoc_query-archetype_subsumption | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_QUERY_SERVICE.execute_ad_hoc_query-ehr_id_header_deprecated_get | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_QUERY_SERVICE.execute_ad_hoc_query-ehr_id_header_get | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_QUERY_SERVICE.execute_ad_hoc_query-ehr_id_header_post | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| I_QUERY_SERVICE.execute_ad_hoc_query-node_name_term_code | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_QUERY_SERVICE.execute_ad_hoc_query-version_metadata_predicate | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | — |
| I_QUERY_SERVICE.execute_stored_query-ehr_id_header | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| SEC-ANONYMOUS_EHRS-anonymous_lifecycle | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| SF-CONTRIB-flat_commit | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::<n>\"" names <versioned_obj | not_applicable |
| SF-CONTRIB-structured_commit | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::<n>\"" names <versioned_obj | not_applicable |
| SF-CTX-composer_name | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-CTX-composer_self | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-CTX-missing_mandatory | — | expected `validation_failed`, observed `bad_request` | not_applicable |
| SF-CTX-participations_forms | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-CTX-vocabulary_mapping | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-EXAMPLE-roundtrip | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-FIELDID-structure | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-FLAT-adl2_commit | — | header ETag: pattern "W/\"<template_id>\"" names <template_id>, which is neither a capture | not_applicable |
| SF-FLAT-adl2_reject_cardinality | — | header ETag: pattern "W/\"<template_id>\"" names <template_id>, which is neither a capture | not_applicable |
| SF-FLAT-commit_roundtrip_ctx_defaults | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-FLAT-reject_datatype_mismatch | — | expected `validation_failed`, observed `bad_request` | not_applicable |
| SF-FLAT-reject_other_with_code | — | expected `validation_failed`, observed `bad_request` | not_applicable |
| SF-FLAT-reject_terminology_binding | — | expected `validation_failed`, observed `bad_request` | not_applicable |
| SF-INDEX-multi_event_commit | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-INDEX-semantics | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-LEVELS-collapsed_wrappers | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-LEVELS-container_attribute_elision | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-LEVELS-lab_panel_example | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-attribute_suffix_table | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-context | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-dv_ordinal_proportion_count | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-dv_quantity | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-dv_text_coded | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-entries | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-events_audit | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-instruction_details | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-interval_event | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-interval_reference_range | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-multimedia_parsable | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-party | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-simple_values | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-structure | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-MAP-temporal | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-RAW-embedding | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-RAW-structured_embedding | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-RMATTR-normal_range_commit | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-RMATTR-underscore_mapping | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-SCOPE-demographic_no_simplified | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-STRUCT-arrays_single_cardinality | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-STRUCT-empty_object_omission | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-STRUCT-style_rules | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SF-STRUCTURED-commit_roundtrip | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SIG-VERSION-across_version_kinds | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| SIG-VERSION-distinct_per_version | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| SIG-VERSION-distinct_per_version-pgp | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |
| SIG-VERSION-signature_present | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| SIG-VERSION-verifiable | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | errored |
| SIG-VERSION-verifiable-pgp | — | header ETag: pattern "W/\"<versioned_object_uid>::<system_id>::1\"" names <versioned_objec | not_applicable |

### EHRbase failures by schedule chapter

| Chapter | failed cases |
|---|---|
| CONT | 67 |
| I_EHR_STATUS | 24 |
| I_EHR_DIRECTORY | 12 |
| I_DEFINITION_QUERY | 10 |
| I_DEFINITION_ADL2 | 7 |
| I_EHR_CONTRIBUTION | 7 |
| I_DEFINITION_ADL14 | 6 |
| I_EHR_SERVICE | 5 |
| I_QUERY_SERVICE | 3 |
| I_EHR_COMPOSITION | 1 |
| I_ITS_REST_REVISION_HISTORY | 1 |
| SIG | 1 |

<details><summary>Every EHRbase-failed case, with the ferroehr outcome on the identical case</summary>

| Case | Format | EHRbase failure | ferroehr outcome |
|---|---|---|---|
| CONT-COMP-content_card_1plus-context_any | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_1plus-context_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_3plus-context_any | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_3plus-context_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_3to5-context_any | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_3to5-context_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_any-context_any | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_any-context_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_mand-context_any | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_mand-context_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_opt-context_any | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMP-content_card_opt-context_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMPOSITION-content_cardinality_count6 | — | expected `created`, observed `validation_failed` | failed |
| CONT-COMPOSITION-context_existence | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_CODED_TEXT-validate_open | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_DATE-validate_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_DATE-validate_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_DATE_TIME-validate_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_DATE_TIME-validate_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_DURATION-validate_fields | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_DURATION-validate_fields_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_DURATION-validate_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_IDENTIFIER-validate_all_list | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_IDENTIFIER-validate_all_pattern | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_DATE_TIME-validate_lower_upper_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_DURATION-validate_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_DURATION-validate_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_ORDINAL-validate_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_PROPORTION-validate_ratio_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_SCALE-validate_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_INTERVAL_DV_TIME-validate_lower_upper_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_MULTIMEDIA-validate_media_type | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_PARSABLE-validate_value_formalism | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_TEXT-validate_open | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_TIME-validate_constraint | — | expected `created`, observed `validation_failed` | failed |
| CONT-DV_TIME-validate_range | — | expected `created`, observed `validation_failed` | failed |
| CONT-EVENT-state_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-EVENT-state_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| CONT-EVENT-type_any | — | expected `created`, observed `validation_failed` | failed |
| CONT-EVENT-type_interval_event | — | expected `created`, observed `validation_failed` | failed |
| CONT-EVENT-type_point_event | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_1plus-summary_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_1plus-summary_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_3plus-summary_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_3plus-summary_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_3to5-summary_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_3to5-summary_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_any-summary_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_any-summary_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_mand-summary_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_mand-summary_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_opt-summary_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-HIST-events_card_opt-summary_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| CONT-HISTORY-events_cardinality_count6 | — | expected `created`, observed `validation_failed` | failed |
| CONT-ITEM_STR-type_any | — | expected `created`, observed `validation_failed` | failed |
| CONT-ITEM_STR-type_item_list | — | expected `created`, observed `validation_failed` | failed |
| CONT-ITEM_STR-type_item_single | — | expected `created`, observed `validation_failed` | failed |
| CONT-ITEM_STR-type_item_table | — | expected `created`, observed `validation_failed` | failed |
| CONT-ITEM_STR-type_item_tree | — | expected `created`, observed `validation_failed` | failed |
| CONT-OBS-state_ex_mand-protocol_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-OBS-state_ex_mand-protocol_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| CONT-OBS-state_ex_opt-protocol_ex_mand | — | expected `created`, observed `validation_failed` | failed |
| CONT-OBS-state_ex_opt-protocol_ex_opt | — | expected `created`, observed `validation_failed` | failed |
| I_DEFINITION_ADL14.upload_opt-invalid_opt | — | expected `validation_failed`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL14.upload_opt-valid_opt | — | expected `created`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict | — | expected `created`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL14.upload_opt-valid_opt_twice_no_conflict | — | expected `created`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL14.validate_opt-invalid_opt | — | expected `validation_failed`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL14.validate_opt-valid_opt | — | expected `created`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL2.get_artefact-example_unknown | — | expected `not_found`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL2.get_artefact-version_prefix | — | expected `created`, observed `not_acceptable` | failed |
| I_DEFINITION_ADL2.upload_artefact-duplicate_conflict | — | expected `created`, observed `not_acceptable` | failed |
| I_DEFINITION_ADL2.upload_artefact-invalid_artefacts | — | expected `validation_failed`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL2.upload_artefact-valid_opt | — | expected `created`, observed `not_acceptable` | failed |
| I_DEFINITION_ADL2.valid_artefact-invalid | — | expected `validation_failed`, observed `not_acceptable` | passed |
| I_DEFINITION_ADL2.valid_artefact-valid | — | expected `created`, observed `not_acceptable` | failed |
| I_DEFINITION_QUERY.list_queries-prefix_all_versions | — | [0]/name: path resolves to nothing | passed |
| I_DEFINITION_QUERY.list_queries-version_get_xml_not_acceptable | — | expected `not_acceptable`, observed `ok` | passed |
| I_DEFINITION_QUERY.list_queries-xml_not_acceptable | — | expected `not_acceptable`, observed `ok` | passed |
| I_DEFINITION_QUERY.store_query-default_slot_with_higher_version | — | header Location: value "http://localhost:8091/ehrbase/rest/openehr/v1/definition/query/org | passed |
| I_DEFINITION_QUERY.store_query-dotted_name | — | expected `stored`, observed `bad_request` | passed |
| I_DEFINITION_QUERY.store_query-unqualified_name | — | expected `stored`, observed `bad_request` | passed |
| I_DEFINITION_QUERY.store_query-update_in_place | — | header Location: value "http://localhost:8091/ehrbase/rest/openehr/v1/definition/query/org | passed |
| I_DEFINITION_QUERY.store_query-version_duplicate_case_variant_name | — | expected `conflict`, observed `stored` | passed |
| I_DEFINITION_QUERY.store_query-version_prefix_rejected | — | expected `bad_request`, observed `stored` | passed |
| I_DEFINITION_QUERY.store_query-version_prerelease_rejected | — | expected `bad_request`, observed `stored` | passed |
| I_EHR_COMPOSITION.get_versioned_composition-malformed_uid | — | expected `bad_request`, observed `not_found` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-delete_directory | — | expected `created`, observed `not_found` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-deleted_member_with_data | — | expected `validation_failed`, observed `created` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-ehr_status_incomplete_lifecycle | — | expected `validation_failed`, observed `created` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type | — | expected `conflict`, observed `validation_failed` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type_deleted | — | expected `conflict`, observed `not_found` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-fail_modify_non_existing_directory | — | expected `validation_failed`, observed `precondition_failed` | passed |
| I_EHR_CONTRIBUTION.commit_contribution-non_exiting_opt | — | expected `template_not_found`, observed `validation_failed` | passed |
| I_EHR_DIRECTORY.create_directory-ehr_not_modifiable | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_DIRECTORY.delete_directory-ehr_with_directory | — | header Last-Modified: expected present, got none | failed |
| I_EHR_DIRECTORY.delete_directory-empty_ehr | — | expected `not_found`, observed `precondition_failed` | passed |
| I_EHR_DIRECTORY.delete_directory-etag_names_new_version | — | header Last-Modified: expected present, got none | passed |
| I_EHR_DIRECTORY.get_directory-deleted_head | — | header Last-Modified: expected present, got none | failed |
| I_EHR_DIRECTORY.get_directory-directory_with_structure | — | equivalent: retrieved content differs from committed (modulo the normative ignore-set); $/ | passed |
| I_EHR_DIRECTORY.get_directory_at_time-deleted_at_time | — | header Last-Modified: expected present, got none | failed |
| I_EHR_DIRECTORY.get_directory_at_version-deleted_version | — | header Last-Modified: expected present, got none | failed |
| I_EHR_DIRECTORY.update_directory-empty_ehr | — | expected `not_found`, observed `precondition_failed` | passed |
| I_EHR_DIRECTORY.update_directory-invalid_folder | — | expected `validation_failed`, observed `precondition_missing` | failed |
| I_EHR_DIRECTORY.update_directory-stale_if_match | — | header ETag: expected the latest version uid, got none | passed |
| I_EHR_DIRECTORY.update_directory-xml | canonical-xml | expected `updated`, observed `precondition_missing` | — |
| I_EHR_SERVICE.create_ehr-bulk_load_population | — | expected `created`, observed `validation_failed` | failed |
| I_EHR_SERVICE.create_ehr-committal_headers | — | commit_audit/description/value: path resolves to nothing | passed |
| I_EHR_SERVICE.create_ehr-invalid_status | — | expected `validation_failed`, observed `created` | errored |
| I_EHR_SERVICE.create_ehr-wrong_method | — | header Allow: expected a value matching ".*(GET.*POST\\|POST.*GET).*", got none | passed |
| I_EHR_SERVICE.get_ehr-malformed_ehr_id | — | expected `bad_request`, observed `not_found` | passed |
| I_EHR_STATUS.clear_ehr_modifiable-bad_ehr | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_STATUS.clear_ehr_modifiable-existing_ehr | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.clear_ehr_modifiable-stale_if_match | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.clear_ehr_queryable-bad_ehr | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_STATUS.clear_ehr_queryable-existing_ehr | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.clear_ehr_queryable-stale_if_match | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.get_ehr_status-at_time_future | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.get_ehr_status-at_time_omitted | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.get_ehr_status_at_version-addressed_version | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.get_versioned_ehr_status-at_time_future | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.get_versioned_ehr_status-at_time_omitted | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.get_versioned_ehr_status-contained_uid_form | — | header Last-Modified: expected present, got none | passed |
| I_EHR_STATUS.get_versioned_ehr_status-container_shape | — | owner_id/type: "ehr" != expected "EHR" | passed |
| I_EHR_STATUS.get_versioned_ehr_status-xml | canonical-xml | header Last-Modified: expected present, got none | passed |
| I_EHR_STATUS.set_ehr_modifiable-bad_ehr | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_modifiable-existing_ehr | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_modifiable-missing_if_match | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_modifiable-stale_if_match | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_modifiable-xml_body | canonical-xml | expected `updated`, observed `precondition_missing` | — |
| I_EHR_STATUS.set_ehr_queryable-bad_ehr | — | expected `not_found`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_queryable-existing_ehr | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_queryable-missing_if_match | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_queryable-stale_if_match | — | expected `updated`, observed `precondition_missing` | passed |
| I_EHR_STATUS.set_ehr_queryable-xml_body | canonical-xml | expected `updated`, observed `precondition_missing` | — |
| I_ITS_REST_REVISION_HISTORY.versioned_ehr_status_revision_history-two_versions | canonical-json | expected `updated`, observed `precondition_missing` | passed |
| I_QUERY_SERVICE.execute_ad_hoc_query-empty_db_bare_ehr | — | row count 100 != expected 1 | passed |
| I_QUERY_SERVICE.execute_ad_hoc_query-unknown_ehr_scope | — | expected `not_found`, observed `ok` | passed |
| I_QUERY_SERVICE.execute_stored_query-fetch_with_top | — | expected `stored`, observed `bad_request` | passed |
| SIG-VERSION-ehr_status_signature | — | signature: expected present, the ORIGINAL_VERSION envelope carries no signature | passed |

</details>

## EHRbase measured-window error classes (adjudicated)

The EHRbase measured record is honest as measured — errors are observations.
Every class below was reproduced against a freshly composed EHRbase and
adjudicated three-way against the RELEASED ITS-REST docs text before any
narrative: the driver payloads are spec-correct (the identical exchanges
succeed 0-error against ferroehr in the committed record), and each class
attributes to the EHRbase implementation. No expectation was bent either way.

| Operation | errors/requests (measured window) |
|---|---|
| `composition_update` | 77/77 |
| `ehr_status_update` | 26/26 |
| `tags_put` | 34/34 |
| `tags_read` | 34/34 |
| `template_get` | 3/85 |

- **`composition_update` + `ehr_status_update` (wire 400, deterministic)** —
  one shared root cause: EHRbase rejects the RFC-9110-quoted `If-Match`
  entity-tag form with `400 "UUID string too large"`. The released docs text
  itself mandates the quoted form (ITS-REST overview `Requests_and_responses.md`
  §"If-Match and accidental overwrites": `If-Match: "8849182c-…::openEHRSys.example.com::2"`),
  and EHRbase 400s even its own returned `ETag` echoed back verbatim; it
  succeeds only on the non-standard unquoted form. EHRbase non-conformance;
  the record stands.
- **`tags_put` + `tags_read` (wire 404, deterministic)** — the item-tag paths
  (`/ehr/{ehr_id}/tags`, `/ehr/{ehr_id}/composition/{uid}/tags`, …) are members
  of the STABLE EHR API in released ITS-REST 1.1.0 (RM grounding:
  `RM/docs/common/master07-tags.adoc`); EHRbase serves no such routes
  ("No resource found at path"). A released-STABLE surface absent from EHRbase is
  non-conformance, not a citable N/A; the record stands.
- **`template_get` (partial)** — the endpoint is fully functional when
  reproduced (200 for JSON and XML); the small error share sits in a load
  window whose `ward_query` p99 was ~10.9 s. Load-window transient, no defect
  on either side; honest error observation.
