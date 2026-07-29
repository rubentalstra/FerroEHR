# Conformance Report

SUT: ehrbase-java 2.34.0 · schedule cnf-2.0-w2 · ITS its-rest
Runner: cnf-runner 3.11.0 · verification pack: passed

## Summary

| Status | Count |
| --- | --- |
| passed | 143 |
| failed | 144 |
| errored | 232 |
| skipped | 0 |
| not_applicable | 248 |
| total | 767 |

## By chapter

Grouping is the published per-chapter chart's taxonomy: chapters with their bands; `cited n/a` counts the not-executed outcomes that carry a citation (`not_applicable` + `skipped`).

| Chapter / band | passed | failed | errored | cited n/a |
| --- | --- | --- | --- | --- |
| **EHR** | 104 | 50 | 111 | 60 |
| — EHR resource | 18 | 5 | 1 | 2 |
| — EHR_STATUS | 12 | 24 | 9 | 2 |
| — COMPOSITION | 11 | 1 | 67 | 10 |
| — DIRECTORY | 48 | 12 | 3 | 4 |
| — CONTRIBUTION | 13 | 7 | 30 | 10 |
| — Item tags | 0 | 0 | 0 | 30 |
| — Revision history | 2 | 1 | 1 | 2 |
| **Definitions** | 21 | 23 | 20 | 24 |
| — ADL 1.4 templates | 3 | 6 | 8 | 11 |
| — ADL 2 artefacts | 2 | 7 | 11 | 9 |
| — Stored queries | 16 | 10 | 1 | 4 |
| **Query** | 11 | 3 | 21 | 1 |
| — Ad-hoc AQL | 8 | 2 | 15 | 0 |
| — Stored query execution | 3 | 1 | 6 | 1 |
| **Demographic** | 4 | 0 | 16 | 50 |
| — Parties | 4 | 0 | 15 | 29 |
| — Party relationships | 0 | 0 | 0 | 15 |
| — Versioned party | 0 | 0 | 1 | 6 |
| **Messaging** | 0 | 0 | 0 | 14 |
| — EHR Extract | 0 | 0 | 0 | 10 |
| — TDD | 0 | 0 | 0 | 4 |
| **Admin** | 0 | 0 | 0 | 25 |
| — Admin service | 0 | 0 | 0 | 19 |
| — Archive | 0 | 0 | 0 | 4 |
| — Dump & load | 0 | 0 | 0 | 2 |
| **System** | 0 | 0 | 1 | 0 |
| — Conformance manifest | 0 | 0 | 1 | 0 |
| **Content validation** | 0 | 67 | 56 | 0 |
| — Data types | 0 | 15 | 37 | 0 |
| — Interval data types | 0 | 11 | 19 | 0 |
| — Structure & cardinality | 0 | 41 | 0 | 0 |
| **Simplified formats** | 0 | 0 | 0 | 64 |
| — FLAT & STRUCTURED | 0 | 0 | 0 | 21 |
| — Web Template | 0 | 0 | 0 | 5 |
| — Path mapping | 0 | 0 | 0 | 30 |
| — Scope & legacy media | 0 | 0 | 0 | 8 |
| **Security & privacy** | 3 | 0 | 2 | 1 |
| — Authenticated access | 2 | 0 | 0 | 0 |
| — Authorization separation | 0 | 0 | 0 | 1 |
| — Audit accountability | 0 | 0 | 1 | 0 |
| — Anonymous EHRs | 0 | 0 | 1 | 0 |
| — EHR/demographic separation | 1 | 0 | 0 | 0 |
| **Signing** | 0 | 1 | 5 | 6 |
| — Version signing | 0 | 1 | 5 | 6 |
| **SMART App Launch** | 0 | 0 | 0 | 3 |
| — Discovery | 0 | 0 | 0 | 1 |
| — Resource scopes | 0 | 0 | 0 | 2 |
| **Performance** | 0 | 0 | 0 | 0 |

## By capability

Selected gating cases per claimed capability. `inconclusive` counts cases whose exchange did not conclude (transport fault, unmapped status, step resolution) — they block a `passed` evidence token and are triaged, never absorbed.

| Capability | Evidence | passed | failed | inconclusive | unevidenced |
| --- | --- | --- | --- | --- | --- |
| Adl14ArchetypeProvisioning | not evidenced | 0 | 0 | 0 | 6 |
| Adl14OptProvisioning | FAIL | 3 | 6 | 8 | 5 |
| QueryProvisioning | FAIL | 15 | 10 | 1 | 0 |
| EhrOperations | FAIL | 18 | 2 | 0 | 2 |
| EhrStatus | FAIL | 12 | 21 | 7 | 5 |
| CompositionOps | INCONCLUSIVE (errored rows — never green by absorption) | 5 | 0 | 20 | 3 |
| DirectoryOps | FAIL | 48 | 11 | 2 | 8 |
| ChangeSets | FAIL | 13 | 6 | 28 | 5 |
| Versioning | FAIL | 8 | 3 | 45 | 0 |
| ArchetypeValidation | FAIL | 0 | 67 | 54 | 0 |
| AqlBasic | FAIL | 6 | 2 | 11 | 0 |
| AqlAdvanced | INCONCLUSIVE (errored rows — never green by absorption) | 0 | 0 | 2 | 0 |
| PhysicalDeletion | not evidenced | 0 | 0 | 0 | 2 |
| DefinitionApi | FAIL | 0 | 1 | 0 | 0 |
| EhrApi | FAIL | 1 | 1 | 0 | 0 |
| QueryApi | FAIL | 3 | 2 | 14 | 0 |
| Signing | not evidenced | 0 | 0 | 0 | 2 |
| EhrDemographicSeparation | pass | 1 | 0 | 0 | 0 |
| AuthenticatedAccess | pass | 2 | 0 | 0 | 0 |
| AuthorizationSeparation | not evidenced | 0 | 0 | 0 | 1 |

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

Coverage: 459 of 499 selected cases driven.

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
| I_ADMIN_SERVICE.physical_ehr_delete-delete_all_cascade | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_all_malformed_id | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_all_subset | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_all_subset_repeated | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_all_unfiltered | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_existing | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_existing_cascade | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_existing_recreate | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_ehr_delete-delete_non_existing | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ADMIN_SERVICE.physical_party_delete-delete_existing | I_ADMIN_SERVICE.physical_party_delete: AMB-33 |
| I_ADMIN_SERVICE.physical_party_delete-delete_non_existing | I_ADMIN_SERVICE.physical_party_delete: AMB-33 |
| I_ADMIN_SERVICE.versioned_composition_count-all | I_ADMIN_SERVICE.versioned_composition_count: AMB-33 |
| I_ADMIN_SERVICE.versioned_composition_count-time_range | I_ADMIN_SERVICE.versioned_composition_count: AMB-33 |
| I_DEFINITION_ADL14.delete_opt-delete_existing | I_DEFINITION_ADL14.delete_opt: AMB-17 |
| I_DEFINITION_ADL14.delete_opt-delete_latest_version | I_DEFINITION_ADL14.delete_opt: AMB-17 |
| I_DEFINITION_ADL14.delete_opt-delete_non_existing | I_DEFINITION_ADL14.delete_opt: AMB-17 |
| I_DEFINITION_ADL14.delete_opt-delete_specific_version | I_DEFINITION_ADL14.delete_opt: AMB-17 |
| I_DEFINITION_ADL14.get_opt-example_detail_levels | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEFINITION_ADL14.get_opt-example_invalid_detail_level | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEFINITION_ADL14.get_opt-example_type_output | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEFINITION_ADL14.get_opt-flat_not_acceptable | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEFINITION_ADL14.get_opt-retrieve_latest_version | option adl14-partial-id-latest: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEFINITION_ADL14.list_archetypes-unrealized | I_DEFINITION_ADL14.list_archetypes: AMB-41 |
| I_DEFINITION_ADL14.upload_opt-prefer_identifier | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEFINITION_ADL2.archetypes_count-unrealized | I_DEFINITION_ADL2.archetypes_count: AMB-37 |
| I_DEFINITION_ADL2.artefacts_count-unrealized | I_DEFINITION_ADL2.artefacts_count: AMB-37 |
| I_DEFINITION_ADL2.delete_artefact-existing | I_DEFINITION_ADL2.delete_artefact: AMB-37 |
| I_DEFINITION_ADL2.delete_artefact-non_existing | I_DEFINITION_ADL2.delete_artefact: AMB-37 |
| I_DEFINITION_ADL2.list_archetypes-unrealized | I_DEFINITION_ADL2.list_archetypes: AMB-37 |
| I_DEFINITION_ADL2.list_artefacts-unrealized | I_DEFINITION_ADL2.list_artefacts: AMB-37 |
| I_DEFINITION_ADL2.opts_count-unrealized | I_DEFINITION_ADL2.opts_count: AMB-37 |
| I_DEFINITION_ADL2.templates_count-unrealized | I_DEFINITION_ADL2.templates_count: AMB-37 |
| I_DEFINITION_ADL2.upload_artefact-prefer_identifier | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEFINITION_QUERY.delete_query-delete_existing | I_DEFINITION_QUERY.delete_query: AMB-127 |
| I_DEFINITION_QUERY.list_matching_queries-id_pattern | I_DEFINITION_QUERY.list_matching_queries: AMB-121 |
| I_DEFINITION_QUERY.queries_count-count | I_DEFINITION_QUERY.queries_count: AMB-127 |
| I_DEFINITION_QUERY.store_query-reserved_name | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-archetype_node_id_mismatch | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-archetype_root_missing | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-archetyped_content_accepted | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-capabilities_present_empty | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-client_supplied_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-contacts_present_empty | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-identity_details_missing | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-identity_details_wrong_type | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-item_tag_wrapper_headers | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-prefer_minimal | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-return_identifier | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-roles_present_empty | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-simplified_content_type | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-wrong_subtype_body | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party-xml | option party-xml-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party_relationship-aaaa | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.create_party_relationship-bbbb | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.delete_party-already_deleted | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.delete_party-stale_version_conflict | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.delete_party_relationship-aaaa | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.delete_party_relationship-bbbb | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.delete_party_relationship-stale_version_conflict | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party-wrong_kind_container | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party-xml | option party-xml-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party-xml_not_acceptable | option party-xml-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_at_time-deleted_current | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_at_time-xml | option party-xml-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_at_version-xml | option party-xml-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship-aaaa | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship-bbbb | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_time-aaaa | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_time-bbbb | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_version-aaaa | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_version-bbbb | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party-invalid_body | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party-missing_if_match | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party-prefer_minimal | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party-unknown_container | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party-unknown_preceding_version | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party-xml | option party-xml-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-aaaa | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-bbbb | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-invalid_body | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.update_party_relationship-missing_if_match | extension realization (party-relationship; AMB-32): the ICS claims none of this case's capabilities, and no openEHR specification governs the route — ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.versioned_party_version_at_time | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_DEMOGRAPHIC_SERVICE.versioned_party_version_unknown | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.create_composition-audit_system_id_declared | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.create_composition-item_tag_wrapper_headers | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.create_composition-return_identifier | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.get_composition_at_time-simplified_forms | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.get_composition_latest-simplified_forms | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.get_composition_latest-xml_namespace_v2 | option xml-namespace-negotiated: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.update_composition-audit_system_id | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.update_composition-flat | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.update_composition-return_identifier | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_COMPOSITION.update_composition-structured | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_CONTRIBUTION.commit_contribution-demographic | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_CONTRIBUTION.commit_contribution-demographic_client_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_CONTRIBUTION.commit_contribution-demographic_invalid_change_types | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_CONTRIBUTION.commit_contribution-return_identifier | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_CONTRIBUTION.get_contribution-demographic_unknown | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
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
| I_EHR_SERVICE.create_ehr-return_identifier | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_EHR_SERVICE.get_ehr-xml_not_acceptable | option ehr-xml-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_EHR_STATUS.get_ehr_status-xml_not_acceptable | option ehr-status-xml-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_EHR_STATUS.set_ehr_queryable-return_identifier | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_delete-key_scoped | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_delete-unknown_key | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_delete-version_container_disjoint | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_get-container_target_shape | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_get-cross_space_party_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_get-version_target_shape | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-key_target_path_identity | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-non_array_body | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-unknown_container | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.composition_tags_update-version_container_disjoint | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.demographic_tags_get-space_wide_listing | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.ehr_status_tags_delete-wrong_kind_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.ehr_status_tags_get-wrong_kind_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.ehr_status_tags_update-key_target_path_identity | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.ehr_status_tags_update-wrong_kind_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.ehr_tags_get-ehr_wide_listing | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.ehr_tags_get-unknown_ehr | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.organisation_tags_delete-wrong_kind_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.organisation_tags_get-wrong_kind_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.organisation_tags_update-wrong_kind_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_delete-key_scoped | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_delete-unknown_key | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_delete-version_container_disjoint | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_get-container_target_shape | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_get-cross_space_composition_uid | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_get-version_target_shape | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_update-key_target_path_identity | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_update-non_array_body | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_update-unknown_container | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_ITEM_TAGS.person_tags_update-version_container_disjoint | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_REVISION_HISTORY.versioned_party_revision_history-two_versions | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_REVISION_HISTORY.versioned_party_revision_history-unknown_container | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_VERSIONED_PARTY.versioned_party_get-container_shape | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_VERSIONED_PARTY.versioned_party_get-unknown_container | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_ITS_REST_VERSIONED_PARTY.versioned_party_get-xml | option versioned-party-xml-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_ITS_REST_VERSIONED_PARTY.versioned_party_get-xml_not_acceptable | option versioned-party-xml-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| I_QUERY_SERVICE.execute_stored_query-reserved_name | case version floor unmet (rm >=1.0.2, aql >=1.1, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| I_TDD_SERVICE.import_tdd-invalid | I_TDD_SERVICE.import_tdd: AMB-34 |
| I_TDD_SERVICE.import_tdd-valid | I_TDD_SERVICE.import_tdd: AMB-34 |
| I_TDD_SERVICE.import_tdds-bulk_invalid | I_TDD_SERVICE.import_tdds: AMB-34 |
| I_TDD_SERVICE.import_tdds-bulk_valid | I_TDD_SERVICE.import_tdds: AMB-34 |
| SEC-AUTHORIZATION_SEPARATION-readonly_write_denied | the ixit declares no instance readonly — the case's flow addresses it with `on:` and this party runs no such deployment/principal; ISO/IEC 9646 test selection |
| SF-CONTRIB-flat_commit | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-CONTRIB-flat_read | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-CONTRIB-structured_commit | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-CONTRIB-structured_read | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-CTX-composer_name | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-CTX-composer_self | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-CTX-missing_mandatory | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-CTX-participations_forms | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-CTX-vocabulary_mapping | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-DEPRECATED-accept_unsupported | option sf-deprecated-types-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| SF-DEPRECATED-media_supported | option sf-deprecated-types-supported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| SF-DEPRECATED-media_unsupported | option sf-deprecated-types-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| SF-EXAMPLE-accept_forms | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-EXAMPLE-roundtrip | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-EXAMPLE-unsupported_accept | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FIELDID-structure | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-adl2_commit | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-adl2_reject_cardinality | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-commit_roundtrip_ctx_defaults | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-missing_template_id | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-reject_cardinality | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-reject_datatype_mismatch | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-reject_other_closed_list | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-reject_other_with_code | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-reject_terminology_binding | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-FLAT-reject_unknown_field | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-INDEX-multi_event_commit | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-INDEX-semantics | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-LEGACY-nc_flat_media_unsupported | option legacy-alt-formats-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| SF-LEGACY-tds2_accept_unsupported | option legacy-alt-formats-unsupported: the ICS does not declare this register branch (statement.options) — ISO/IEC 9646 test selection |
| SF-LEVELS-collapsed_wrappers | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-LEVELS-container_attribute_elision | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-LEVELS-lab_panel_example | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-attribute_suffix_table | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-context | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-dv_ordinal_proportion_count | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-dv_quantity | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-dv_text_coded | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-entries | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-events_audit | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-instruction_details | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-interval_event | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-interval_reference_range | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-multimedia_parsable | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-party | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-simple_values | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-structure | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-MAP-temporal | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-NODEID-generation_rules | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-NODEID-web_template_ids | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-RAW-embedding | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-RAW-missing_type | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-RAW-structured_embedding | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-RMATTR-normal_range_commit | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-RMATTR-underscore_mapping | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-SCOPE-demographic_no_simplified | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-SCOPE-directory_no_simplified | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-SCOPE-ehr_status_no_simplified | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-STRUCT-arrays_single_cardinality | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-STRUCT-empty_object_omission | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-STRUCT-style_rules | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-STRUCTURED-commit_roundtrip | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-WT-web_template_get | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SF-WT-web_template_json_accept | case version floor unmet (rm >=1.0.2, its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SIG-VERSION-client_supplied_verbatim-pgp | the ixit declares no instance sut_pgp — the case's flow addresses it with `on:` and this party runs no such deployment/principal; ISO/IEC 9646 test selection |
| SIG-VERSION-directory_signature_present | I_EHR_DIRECTORY.get_versioned_directory: AMB-24 |
| SIG-VERSION-directory_verifiable | I_EHR_DIRECTORY.get_versioned_directory: AMB-24 |
| SIG-VERSION-distinct_per_version-pgp | the ixit declares no instance sut_pgp — the case's flow addresses it with `on:` and this party runs no such deployment/principal; ISO/IEC 9646 test selection |
| SIG-VERSION-ehr_status_signature-pgp | the ixit declares no instance sut_pgp — the case's flow addresses it with `on:` and this party runs no such deployment/principal; ISO/IEC 9646 test selection |
| SIG-VERSION-verifiable-pgp | the ixit declares no instance sut_pgp — the case's flow addresses it with `on:` and this party runs no such deployment/principal; ISO/IEC 9646 test selection |
| SMART-DISCOVERY-document_shape | case version floor unmet (its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SMART-RESOURCE_SCOPES-granted_scope_permits | case version floor unmet (its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
| SMART-RESOURCE_SCOPES-scope_deny_403 | case version floor unmet (its_rest >=1.1.0) — the party's declared spec versions do not satisfy the case's applies ranges; ISO/IEC 9646 test selection |
