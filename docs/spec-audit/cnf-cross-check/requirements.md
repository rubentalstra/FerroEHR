# A1 Spec Audit — Phase 1 (Extract) — Chapter `cnf-cross-check`

- **Date:** 2026-07-11
- **Method (special task):** Enumerate the normative CNF Platform Conformance
  Test Schedule cases + Robot fixtures, diff them against the ECC catalogue in
  `tools/conformance/src/suites/`, and select ≥20 normative cases the ECC
  catalogue does **not** cover, prioritising rejection duties and write-path
  integrity. Each selected uncovered case = one requirement. Risk was calibrated
  by a light static check of the server (`app/**`) done while extracting.
- **Spec files read (oracle, relative to `docs/specs/openehr/`):**
  - `CNF/docs/platform_test_schedule/master04..master17.7-*.adoc` (all
    functional + content test-case chapters)
  - `CNF/tests/platform/robot/**` (I_ADMIN_SERVICE, I_DEFINITION_ADL14/QUERY,
    I_EHR_COMPOSITION, I_EHR_CONTRIBUTION, I_EHR_DIRECTORY, I_EHR_SERVICE,
    I_EHR_STATUS, I_QUERY_SERVICE, SECURITY_TESTS fixtures)
- **ECC catalogue diffed against:** `tools/conformance/src/suites/{admin,
  composition,contribution,definition_adl14,definition_query,demographic,
  directory,ehr,message,query,security,signing,terminology}.rs` +
  `tools/conformance/src/suites/content/**` (case slugs extracted).

## Coverage summary

The functional chapters master04–09 (Definition-ADL14, Definition-Query, EHR,
Composition, Contribution, Directory) and the content chapters master15/16/17.x
are near-fully mirrored slug-for-slug by the ECC suites. The uncovered surface
concentrates in four areas, all of which the schedule/Robot fixtures make
normative but the ECC catalogue omits entirely:

1. **Versioned-object retrieval sub-resources** (`versioned_ehr_status`,
   `versioned_composition/revision_history`) — Robot `C.6 A–D` fixtures; the
   ECC `ehr`/`composition` suites have no versioned-EHR_STATUS case and no
   revision-history case.
2. **ADMIN physical-delete + cache** — Robot `I_ADMIN_SERVICE/001–006`; the ECC
   `admin` suite only covers `adm/ehr-delete*`. The server (`app/**`) implements
   **only** `admin_ehr_delete` + `admin_ehr_delete_all` (confirmed via
   `ehrbase-rest/src/access/authz/classify.rs`), so these are *unimplemented and
   untested* — genuine write-path gaps.
3. **Demographic PARTY_RELATIONSHIP + temporal party reads** — master10 Service
   Model operations; the ECC `demographic` suite has no relationship case and no
   `get_party_at_time` case.
4. **Composition timezone round-trip integrity** — Robot
   `COMPOSITION_WITH_DIFFERENT_TIME_ZONES/*`; no ECC case asserts that a
   committed extended date-time preserves its original UTC offset.

## Requirements (uncovered CNF cases)

| id | requirement | citation | category | risk |
|----|-------------|----------|----------|------|
| cnf-cross-check-R1 | ADMIN physical-delete of a COMPOSITION (`DELETE /admin/ehr/{ehr_id}/composition/{versioned_object_uid}`) MUST physically remove every version row of that versioned object so post-delete table counts return to their pre-commit baseline (a hard delete, distinct from the logical delete of the normal endpoint). Not covered by ECC; server implements only `admin_ehr_delete`/`admin_ehr_delete_all`. | `CNF/tests/platform/robot/I_ADMIN_SERVICE/002-Composition.robot` (`001 ADMIN - Delete Composition`, `check composition admin delete table counts`) | rejection-duty | high |
| cnf-cross-check-R2 | ADMIN physical-delete of a CONTRIBUTION (`DELETE /admin/ehr/{ehr_id}/contribution/{contribution_uid}`) MUST physically remove the contribution and its owned version rows, restoring table counts. Not covered by ECC. | `CNF/tests/platform/robot/I_ADMIN_SERVICE/003-Contribution.robot` | rejection-duty | high |
| cnf-cross-check-R3 | ADMIN physical-delete of a DIRECTORY / FOLDER tree (`DELETE /admin/ehr/{ehr_id}/directory/{versioned_object_uid}`) MUST physically remove all folder version rows while leaving the EHR and its creating contribution intact (`contr_records` stays 1). Not covered by ECC. | `CNF/tests/platform/robot/I_ADMIN_SERVICE/004-Directory.robot` (`check directory admin delete table counts`, line 108) | rejection-duty | high |
| cnf-cross-check-R4 | ADMIN physical-delete / overwrite of a template/OPT (`DELETE /admin/template/{template_id}`, with `SYSTEM_ALLOWTEMPLATEOVERWRITE`) MUST physically remove the stored OPT so a subsequent upload of the same `template_id` does not conflict. Not covered by ECC. | `CNF/tests/platform/robot/I_ADMIN_SERVICE/005-Template.robot`; `CNF/tests/platform/robot/I_ADMIN_SERVICE/002-Composition.robot` (`(admin) delete OPT`) | rejection-duty | high |
| cnf-cross-check-R5 | ADMIN cache-clear (`DELETE /admin/*/cache` / template-cache invalidation) MUST evict cached template/WebTemplate state so subsequent reads reflect physically-deleted artefacts. Not covered by ECC. | `CNF/tests/platform/robot/I_ADMIN_SERVICE/006-Cache.robot` | behaviour | medium |
| cnf-cross-check-R6 | `I_ADMIN_SERVICE.physical_party_delete()` MUST hard-delete a demographic PARTY and its version rows (admin surface). Server admin op set is only EHR delete; unimplemented and untested. | `CNF/docs/platform_test_schedule/master12-func_tc_admin.adoc` §`I_ADMIN_SERVICE.physical_party_delete()` (line 109) | rejection-duty | high |
| cnf-cross-check-R7 | `get_versioned_ehr_status` (`GET /ehr/{ehr_id}/versioned_ehr_status`) MUST return `200` with a `VERSIONED_EHR_STATUS` whose `uid.value` equals the versioned-object id and whose `owner_id.id.value` equals the `ehr_id`. No ECC `sta/*` case exercises the versioned_ehr_status resource. | `CNF/tests/platform/robot/I_EHR_STATUS/get_versioned_ehr_status/C.6__A)_Get_Versioned_EHR_STATUS.robot` | behaviour | medium |
| cnf-cross-check-R8 | Revision history of the versioned EHR_STATUS (`GET /ehr/{ehr_id}/versioned_ehr_status/revision_history`) MUST return `200` with one `REVISION_HISTORY_ITEM` per committed EHR_STATUS version (1 entry for a new EHR, 2 after one `is_queryable`/`is_modifiable` update), each `version_id.value` matching the corresponding version uid. Not covered by ECC. | `CNF/tests/platform/robot/I_EHR_STATUS/get_versioned_ehr_status/C.6__B)_Get_Versioned_EHR_STATUS_Revision_History.robot` | behaviour | medium |
| cnf-cross-check-R9 | Versioned EHR_STATUS at time (`GET /ehr/{ehr_id}/versioned_ehr_status/version?version_at_time=...`) MUST return the `ORIGINAL_VERSION<EHR_STATUS>` current at the supplied instant. Not covered by ECC. | `CNF/tests/platform/robot/I_EHR_STATUS/get_versioned_ehr_status/C.6__C)_Get_Versioned_EHR_STATUS_By_Time.robot` | behaviour | medium |
| cnf-cross-check-R10 | Versioned EHR_STATUS by version id (`GET /ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}`) MUST return the addressed `ORIGINAL_VERSION<EHR_STATUS>`, and `404` for an unknown version uid. Not covered by ECC. | `CNF/tests/platform/robot/I_EHR_STATUS/get_versioned_ehr_status/C.6__D)_Get_Versioned_EHR_STATUS_By_Version.robot` | behaviour | medium |
| cnf-cross-check-R11 | Revision history of a versioned COMPOSITION (`GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history`) MUST return `200` with one entry per version and each `version_id.value` matching. ECC covers `get-versioned-composition` but not its `revision_history` sub-resource. | `CNF/tests/platform/robot/I_EHR_COMPOSITION/get_versioned_composition/C.6__B)_Get_Versioned_COMPOSITION_Revision_History.robot` | behaviour | medium |
| cnf-cross-check-R12 | The versioned-COMPOSITION revision history MUST be returned in commit order (oldest→newest), with `version_id` suffix `::…::1`, `::2` sequential. Not covered by ECC. | `CNF/tests/platform/robot/I_EHR_COMPOSITION/get_versioned_composition/C.6__B)_Get_Versioned_COMPOSITION_Revision_History.robot` (`3. Get Correct Ordered Revision History…`) | serialization | medium |
| cnf-cross-check-R13 | A COMPOSITION committed with a non-UTC (Berlin, +01:00/+02:00) extended date-time MUST round-trip that exact offset: `get versioned composition - version at time` returns the "original value" unchanged (no server-side normalization to UTC). Write-path/serialization integrity, not covered by ECC. | `CNF/tests/platform/robot/I_EHR_COMPOSITION/COMPOSITION_WITH_DIFFERENT_TIME_ZONES/COMPOSITION_JSON_Berlin_time_zone.robot` | serialization | high |
| cnf-cross-check-R14 | A COMPOSITION committed with a date-time that has **no** timezone offset MUST round-trip with the offset still absent (no synthetic offset injected). Not covered by ECC. | `CNF/tests/platform/robot/I_EHR_COMPOSITION/COMPOSITION_WITH_DIFFERENT_TIME_ZONES/COMPOSITION_JSON_no_time_zone.robot` | serialization | high |
| cnf-cross-check-R15 | A COMPOSITION committed with a UTC (`Z`) date-time MUST round-trip preserving the `Z` designator verbatim. Not covered by ECC. | `CNF/tests/platform/robot/I_EHR_COMPOSITION/COMPOSITION_WITH_DIFFERENT_TIME_ZONES/COMPOSITION_JSON_utc.robot` | serialization | high |
| cnf-cross-check-R16 | The same Berlin-timezone composition MUST also be retrievable and offset-preserving via AQL (`SELECT` of the datetime leaf returns the original offset). Not covered by ECC (query suite uses fixed corpora only). | `CNF/tests/platform/robot/I_EHR_COMPOSITION/COMPOSITION_WITH_DIFFERENT_TIME_ZONES/COMPOSITION_JSON_Berlin_time_zone.robot` (`… using AQL`) | behaviour | medium |
| cnf-cross-check-R17 | `I_DEMOGRAPHIC_SERVICE.create_party_relationship()` MUST create a versioned `PARTY_RELATIONSHIP` between two existing parties and reject (error) when a referenced party does not exist. ECC `demographic` suite has no relationship case. | `CNF/docs/platform_test_schedule/master10-func_tc_demographic.adoc` §`I_DEMOGRAPHIC_SERVICE.create_party_relationship()` (line 127) | rejection-duty | high |
| cnf-cross-check-R18 | `I_DEMOGRAPHIC_SERVICE.get_party_relationship()` MUST return the addressed `PARTY_RELATIONSHIP` (`200`) and `404` for an unknown relationship uid. Not covered by ECC. | `CNF/docs/platform_test_schedule/master10-func_tc_demographic.adoc` §`I_DEMOGRAPHIC_SERVICE.get_party_relationship()` (line 139) | behaviour | medium |
| cnf-cross-check-R19 | `I_DEMOGRAPHIC_SERVICE.update_party_relationship()` MUST create a new version of the relationship under optimistic-concurrency (`If-Match`), rejecting a stale/absent precondition. Write-path integrity; not covered by ECC. | `CNF/docs/platform_test_schedule/master10-func_tc_demographic.adoc` §`I_DEMOGRAPHIC_SERVICE.update_party_relationship()` (line 165) | rejection-duty | high |
| cnf-cross-check-R20 | `I_DEMOGRAPHIC_SERVICE.delete_party_relationship()` MUST logically delete the relationship and reject deletion of a non-existent one. Not covered by ECC. | `CNF/docs/platform_test_schedule/master10-func_tc_demographic.adoc` §`I_DEMOGRAPHIC_SERVICE.delete_party_relationship()` (line 178) | rejection-duty | high |
| cnf-cross-check-R21 | `I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_time()` / `…_at_version()` MUST return the relationship version current at the supplied instant / addressed by the version uid. Not covered by ECC. | `CNF/docs/platform_test_schedule/master10-func_tc_demographic.adoc` §`get_party_relationship_at_time()` (line 152), §`get_party_relationship_at_version()` (line 191) | behaviour | medium |
| cnf-cross-check-R22 | `I_DEMOGRAPHIC_SERVICE.get_party_at_time()` MUST return the PARTY version current at a supplied instant (temporal read distinct from the version-uid read the ECC `dem/person-get-by-version` case exercises). Not covered by ECC. | `CNF/docs/platform_test_schedule/master10-func_tc_demographic.adoc` §`I_DEMOGRAPHIC_SERVICE.get_party_at_time()` (line 75) | behaviour | medium |
| cnf-cross-check-R23 | `I_ADMIN_SERVICE.contribution_count()` / `composition_version_count()` / `versioned_composition_count()` reporting operations MUST return accurate counts for an EHR. Not covered by ECC and not in the server admin op set. | `CNF/docs/platform_test_schedule/master12-func_tc_admin.adoc` §`I_ADMIN_SERVICE.contribution_count()` (line 57), §`composition_version_count()` (line 83), §`versioned_composition_count()` (line 70) | behaviour | low |

## Static-verification notes (informing risk)

- **R1–R6 (ADMIN physical delete + party delete):** the server admin surface is
  exactly `admin_ehr_delete` + `admin_ehr_delete_all` — asserted by
  `app/ehrbase-rest/src/access/authz/classify.rs`
  (`admin_routes_are_the_only_admin_class`) and `audit_table.rs`. No route for
  composition/contribution/directory/template/cache/party physical delete exists,
  so these CNF duties are **unimplemented and invisible to ECC** → high.
- **R7–R12 (versioned EHR_STATUS / composition revision history):** service
  methods exist (`app/ehrbase-sm/src/services/ehr_status.rs`
  `get_versioned_ehr_status`, `ehr_status_revision_history`, version-at-time,
  version-by-uid) → likely implemented but **unverified by any ECC case** →
  medium.
- **R13–R16 (timezone round-trip):** canonical serialization is the storage
  format (ADR-008 "verbatim canonical fragment"), so offset preservation is
  plausible, but no ECC case asserts it end-to-end (commit→versioned read /
  AQL) → high for the write-path round-trip, medium for the AQL variant.
- **R17–R22 (demographic relationships / temporal reads):** `PARTY_RELATIONSHIP`
  extension routes are wired (`app/ehrbase-rest/src/dispatch/demographic.rs`,
  `party_relationship_*`) and a `PartyRelationshipService` trait exists, but no
  ECC case exercises them → medium/high per rejection-duty weighting.
