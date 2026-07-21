# Conformance Report (generated)

> Generated from a conformance run — never hand-asserted. Every claim is a
> pure function of the recorded outcomes; every coverage bound is printed.

## 1. System under test

| Field | Value |
|---|---|
| Product | ehrbase-java EHRbase 2.34.0 |
| SUT class | foreign (comparison data) |
| Base URL | `http://localhost:8091/ehrbase/rest/openehr/v1` |
| Auth mode | basic |
| Edition policy | auto (ladder: newest form first, step down) |
| Spec versions | RM 1.2.0 · ITS-REST Release-1.1.0 · AQL 1.1.0 · TERM 3.1.0 |
| Reference corpus | openEHR/specifications-CNF@33251d2a |
| Run started | 2026-07-21T04:52:00.770002Z |

**402 case×format executions · 146 passed · 196 failed · 0 errored · 5 skipped · 55 not applicable.**

## 2. Per-area matrix

| Area | Catalogue (active) | Passed | Failed | Errored | Skipped | N/A |
|---|--:|--:|--:|--:|--:|--:|
| EHR — EHR service | 13 | 12 | 1 | 0 | 0 | 0 |
| STA — EHR_STATUS | 10 | 6 | 4 | 0 | 0 | 0 |
| COM — COMPOSITION | 32 | 23 | 16 | 0 | 0 | 0 |
| CTB — CONTRIBUTION (change sets) | 31 | 23 | 8 | 0 | 0 | 0 |
| DIR — DIRECTORY (FOLDER) | 37 | 37 | 0 | 0 | 0 | 0 |
| TPL — Template / OPT provisioning | 17 | 12 | 5 | 0 | 0 | 0 |
| SQR — Stored-query provisioning | 7 | 7 | 0 | 0 | 0 | 0 |
| QRY — AQL execution | 25 | 14 | 11 | 0 | 0 | 0 |
| VAL — Content / archetype validation | 119 | 0 | 119 | 0 | 0 | 0 |
| DEM — Demographic service | 31 | 0 | 0 | 0 | 0 | 31 |
| ADM — Admin service | 14 | 1 | 4 | 0 | 1 | 8 |
| SEC — Security / authorization | 2 | 2 | 0 | 0 | 0 | 0 |
| SIG — Version signing | 5 | 0 | 0 | 0 | 0 | 6 |
| MSG — Messaging | 10 | 0 | 0 | 0 | 0 | 10 |
| TS — Terminology-server integration | 9 | 2 | 3 | 0 | 4 | 0 |
| SF — Simplified Formats (FLAT / STRUCTURED / Web Template) | 16 | 3 | 13 | 0 | 0 | 0 |
| ADL2 — ADL2 template provisioning | 12 | 1 | 11 | 0 | 0 | 0 |
| AQT — AQL terminology functions | 4 | 3 | 1 | 0 | 0 | 0 |

## 3. Capability matrix

Cases grouped by capability; the evidence classification folds a transport error into `failed` (an errored capability is never claimed as passed).

| Capability | Passed | Failed | Errored | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 1 | 0 | 0 | 0 | 0 | pass |
| Adl14OptProvisioning | 11 | 5 | 0 | 0 | 0 | **FAIL** |
| Adl2Provisioning | 1 | 11 | 0 | 0 | 0 | **FAIL** |
| EhrOperations | 11 | 1 | 0 | 0 | 0 | **FAIL** |
| EhrStatus | 6 | 4 | 0 | 0 | 0 | **FAIL** |
| CompositionOps | 19 | 16 | 0 | 0 | 0 | **FAIL** |
| ChangeSets | 23 | 8 | 0 | 0 | 0 | **FAIL** |
| Versioning | 7 | 0 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 0 | 119 | 0 | 0 | 0 | **FAIL** |
| DirectoryOps | 34 | 0 | 0 | 0 | 0 | pass |
| QueryProvisioning | 7 | 0 | 0 | 0 | 0 | pass |
| AqlBasic | 13 | 11 | 0 | 0 | 0 | **FAIL** |
| AqlAdvanced | 1 | 0 | 0 | 0 | 0 | pass |
| AqlTerminology | 3 | 1 | 0 | 0 | 0 | **FAIL** |
| PartyOperations | 0 | 0 | 0 | 0 | 25 | not evidenced |
| PartyRelationshipOperations | 0 | 0 | 0 | 0 | 6 | not evidenced |
| AdminActivityReport | 0 | 0 | 0 | 0 | 4 | not evidenced |
| AdminPhysicalDeletion | 1 | 4 | 0 | 1 | 1 | **FAIL** |
| AdminEhrDumpLoad | 0 | 0 | 0 | 0 | 1 | not evidenced |
| AdminEhrArchive | 0 | 0 | 0 | 0 | 1 | not evidenced |
| AdminDemographicArchive | 0 | 0 | 0 | 0 | 1 | not evidenced |
| MessagingEhrExtract | 0 | 0 | 0 | 0 | 7 | not evidenced |
| MessagingTds | 0 | 0 | 0 | 0 | 3 | not evidenced |
| Signing | 0 | 0 | 0 | 0 | 6 | not evidenced |
| AnonymousEhrs | 1 | 0 | 0 | 0 | 0 | pass |
| Authentication | 2 | 0 | 0 | 0 | 0 | pass |
| Terminology | 2 | 3 | 0 | 4 | 0 | **FAIL** |
| SimplifiedFormats | 3 | 13 | 0 | 0 | 0 | **FAIL** |

## 4. Profile verdict (machine-computed)

CORE/STANDARD are all-of (every listed capability must be `pass`); OPTIONS is any-of (obtained if any optional capability passes) — `master03-profiles.adoc`. An unevidenced required capability fails the claim.

### Core — not claimable

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 1 | 0 | 0 | 0 | pass |
| Adl14OptProvisioning | 11 | 5 | 0 | 0 | **FAIL** |
| EhrOperations | 11 | 1 | 0 | 0 | **FAIL** |
| EhrStatus | 6 | 4 | 0 | 0 | **FAIL** |
| CompositionOps | 19 | 16 | 0 | 0 | **FAIL** |
| ChangeSets | 23 | 8 | 0 | 0 | **FAIL** |
| Versioning | 7 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 0 | 119 | 0 | 0 | **FAIL** |
| AnonymousEhrs | 1 | 0 | 0 | 0 | pass |

### Standard — not claimable

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 1 | 0 | 0 | 0 | pass |
| Adl14OptProvisioning | 11 | 5 | 0 | 0 | **FAIL** |
| EhrOperations | 11 | 1 | 0 | 0 | **FAIL** |
| EhrStatus | 6 | 4 | 0 | 0 | **FAIL** |
| CompositionOps | 19 | 16 | 0 | 0 | **FAIL** |
| ChangeSets | 23 | 8 | 0 | 0 | **FAIL** |
| Versioning | 7 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 0 | 119 | 0 | 0 | **FAIL** |
| AnonymousEhrs | 1 | 0 | 0 | 0 | pass |
| QueryProvisioning | 7 | 0 | 0 | 0 | pass |
| DirectoryOps | 34 | 0 | 0 | 0 | pass |
| AqlBasic | 13 | 11 | 0 | 0 | **FAIL** |
| Signing | 0 | 0 | 0 | 6 | not evidenced |

### Options — OBTAINED

| Capability | Passed | Failed | Skipped | N/A | Evidence |
|---|--:|--:|--:|--:|---|
| Adl2Provisioning | 1 | 11 | 0 | 0 | **FAIL** |
| PartyOperations | 0 | 0 | 0 | 25 | not evidenced |
| PartyRelationshipOperations | 0 | 0 | 0 | 6 | not evidenced |
| AqlAdvanced | 1 | 0 | 0 | 0 | pass |
| AqlTerminology | 3 | 1 | 0 | 0 | **FAIL** |
| AdminActivityReport | 0 | 0 | 0 | 4 | not evidenced |
| AdminPhysicalDeletion | 1 | 4 | 1 | 1 | **FAIL** |
| AdminEhrDumpLoad | 0 | 0 | 0 | 1 | not evidenced |
| AdminBulkEhrLoad | 0 | 0 | 0 | 0 | no cases |
| AdminEhrArchive | 0 | 0 | 0 | 1 | not evidenced |
| AdminDemographicArchive | 0 | 0 | 0 | 1 | not evidenced |
| MessagingEhrExtract | 0 | 0 | 0 | 7 | not evidenced |
| MessagingTds | 0 | 0 | 0 | 3 | not evidenced |
| SimplifiedFormats | 3 | 13 | 0 | 0 | **FAIL** |

## 5. Failures

Each failure is a conformance finding — never an exclusion (standing rule 3).

- **ECC-STA-003** Set EHR_STATUS is_queryable — existing EHR (`sta/set-ehr-queryable-existing-ehr`, json): expected status in [200, 204], got 400
  _cite: CNF master06-func_tc_ehr §set_ehr_queryable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 200; RM 1.2.0 ehr §EHR_STATUS.is_queryable_
- **ECC-STA-005** Set EHR_STATUS is_modifiable — existing EHR (`sta/set-ehr-modifiable-existing-ehr`, json): expected status in [200, 204], got 400
  _cite: CNF master06-func_tc_ehr §set_ehr_modifiable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 200; RM 1.2.0 ehr §EHR_STATUS.is_modifiable_
- **ECC-STA-007** Clear EHR_STATUS is_queryable — existing EHR (`sta/clear-ehr-queryable-existing-ehr`, json): expected status in [200, 204], got 400
  _cite: CNF master06-func_tc_ehr §clear_ehr_queryable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 200; RM 1.2.0 ehr §EHR_STATUS.is_queryable_
- **ECC-STA-009** Clear EHR_STATUS is_modifiable — existing EHR (`sta/clear-ehr-modifiable-existing-ehr`, json): expected status in [200, 204], got 400
  _cite: CNF master06-func_tc_ehr §clear_ehr_modifiable; ITS-REST 1.1.0 EHR_STATUS API ehr_status_update.yaml 200; RM 1.2.0 ehr §EHR_STATUS.is_modifiable_
- **ECC-EHR-012** Create EHR — reject invalid EHR_STATUS data sets (`ehr/create-ehr-invalid-status`, json): invalid EHR_STATUS fixture "000_ehr_status_type_missing.json": expected 4xx (rejected), got 201
  _cite: CNF master06-func_tc_ehr §Test Data Sets class 2 (invalid EHR_STATUS shapes); ITS-REST 1.1.0 EHR API ehr_create.yaml 400/422; RM ehr master04 §EHR Status + common §PARTY_SELF_
- **ECC-COM-003** Create composition — same OPT twice (`com/create-composition-same-opt-twice`, json): expected a negative (4xx) response, got 201
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-032** Composition existence check — existing composition (`com/has-composition`, json): payload comparison (Superset) failed: expected {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"links":[{"_type":"LINK","meaning":{"_type":"DV_TEXT","value":"meaning"},"type":{"_type":"DV_TEXT","value":"type"},"target":{"_type":"DV_EHR_URI","value":"ehr://target1"}},{"_type":"LINK","meaning":{"_type":"DV_TEXT","value":"meanin…, got {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"archetype_details":{"archetype_id":{"value":"openEHR-EHR-COMPOSITION.nesting.v1"},"template_id":{"value":"nested.en.v1"},"rm_version":"1.0.2"},"language":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"IS…
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-008** Get latest composition (`com/get-composition-latest`, json): expected status in [200, 204], got 400
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-008** Get latest composition (`com/get-composition-latest`, xml): expected status in [200, 204], got 400
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-013** Get composition at time (`com/get-composition-at-time`, json): payload comparison (Superset) failed: expected {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"links":[{"_type":"LINK","meaning":{"_type":"DV_TEXT","value":"meaning"},"type":{"_type":"DV_TEXT","value":"type"},"target":{"_type":"DV_EHR_URI","value":"ehr://target1"}},{"_type":"LINK","meaning":{"_type":"DV_TEXT","value":"meanin…, got {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"archetype_details":{"archetype_id":{"value":"openEHR-EHR-COMPOSITION.nesting.v1"},"template_id":{"value":"nested.en.v1"},"rm_version":"1.0.2"},"language":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"IS…
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-013** Get composition at time (`com/get-composition-at-time`, xml): payload comparison (Superset) failed: expected {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"archetype_node_id":"openEHR-EHR-COMPOSITION.nesting.v1","links":[{"_type":"LINK","meaning":{"_type":"DV_TEXT","value":"meaning"},"type":{"_type":"DV_TEXT","value":"type"},"target":{"_type":"DV_EHR_URI","value":"ehr://target1"}}],"a…, got {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"archetype_details":{"archetype_id":{"value":"openEHR-EHR-COMPOSITION.nesting.v1"},"template_id":{"value":"nested.en.v1"},"rm_version":"1.0.2"},"language":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"IS…
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-014** Get composition at time — no time arg (`com/get-composition-at-time-no-time-arg`, json): payload comparison (Superset) failed: expected {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"links":[{"_type":"LINK","meaning":{"_type":"DV_TEXT","value":"meaning"},"type":{"_type":"DV_TEXT","value":"type"},"target":{"_type":"DV_EHR_URI","value":"ehr://target1"}},{"_type":"LINK","meaning":{"_type":"DV_TEXT","value":"meanin…, got {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"archetype_details":{"archetype_id":{"value":"openEHR-EHR-COMPOSITION.nesting.v1"},"template_id":{"value":"nested.en.v1"},"rm_version":"1.0.2"},"language":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"IS…
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-014** Get composition at time — no time arg (`com/get-composition-at-time-no-time-arg`, xml): payload comparison (Superset) failed: expected {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"archetype_node_id":"openEHR-EHR-COMPOSITION.nesting.v1","links":[{"_type":"LINK","meaning":{"_type":"DV_TEXT","value":"meaning"},"type":{"_type":"DV_TEXT","value":"type"},"target":{"_type":"DV_EHR_URI","value":"ehr://target1"}}],"a…, got {"_type":"COMPOSITION","name":{"_type":"DV_TEXT","value":"Nesting"},"archetype_details":{"archetype_id":{"value":"openEHR-EHR-COMPOSITION.nesting.v1"},"template_id":{"value":"nested.en.v1"},"rm_version":"1.0.2"},"language":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"IS…
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-017** Get composition at multiple times (`com/get-composition-at-times`, json): expected status in [200, 204], got 400
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-018** Get composition version (`com/get-composition-version`, json): expected status in [200, 204], got 400
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-018** Get composition version (`com/get-composition-version`, xml): expected status in [200, 204], got 400
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-021** Get composition versions (`com/get-composition-versions`, json): expected status in [200, 204], got 400
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_{create,get,update,delete}.yaml + versioned_composition_get.yaml; RM 1.2.0 ehr §COMPOSITION, common §VERSIONED_OBJECT + change_control_
- **ECC-COM-025** Update composition — event (`com/update-composition-event`, json): expected status in [200, 204], got 400
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_update.yaml 200; RM common §Version tree; TERM SupportTerminology audit_change_type 249 creation / 251 modification_
- **ECC-COM-026** Update composition — persistent (`com/update-composition-persistent`, json): expected status in [200, 204], got 400
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_update.yaml 200; RM common §Version tree; TERM SupportTerminology audit_change_type 249 creation / 251 modification_
- **ECC-COM-029** Delete composition — event (`com/delete-composition-event`, json): deleted VERSION.lifecycle_state should be openehr::523|deleted| (master07 §delete_composition), got Some("532")
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_delete.yaml 204 + composition_get.yaml 204_because_deleted/404; master07 §delete_composition (logical delete: VERSION.lifecycle_state = openehr::523|deleted|)_
- **ECC-COM-030** Delete composition — persistent (`com/delete-composition-persistent`, json): deleted VERSION.lifecycle_state should be openehr::523|deleted| (master07 §delete_composition), got Some("532")
  _cite: ITS-REST 1.1.0 COMPOSITION API composition_delete.yaml 204 + composition_get.yaml 204_because_deleted/404; master07 §delete_composition (logical delete: VERSION.lifecycle_state = openehr::523|deleted|)_
- **ECC-CTB-001** Commit contribution — valid composition (`ctb/commit-contribution-valid-composition`, json): expected status 201, got 400 (body: {"error":"Bad Request","message":"Message at /version/contribution (/version/contribution):  Attribute contribution must not be set"})
  _cite: master08 §valid_composition; ITS-REST contribution_create 201_CONTRIBUTION; RM common master06 §Version_
- **ECC-CTB-014** Commit contribution — invalid EHR status (`ctb/commit-contribution-invalid-ehr-status`, json): D.4 invalid EHR_STATUS (is_queryable removed): status 201 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: {
  "_type" : "CONTRIBUTION",
  "uid" : {
    "_type" : "HIER_OBJECT_ID",
    "value" : "0ec15be2-c9b5-4b43-9677-cabf01ac80e4"
  },
  "versions" : [ {
    "_type" : "OBJECT_REF",
    "namespace" : "local",
    "type" : "EHR_STATUS",
    "id" : {
      "_type" : "OBJECT_VERSION_ID",
      "value" : "
  _cite: master08 §EHR_STATUS Reject 4 (invalid EHR_STATUS); ITS-REST contribution_create 400_CONTRIBUTION_
- **ECC-CTB-017** Commit contribution — fail modify non existing directory (`ctb/commit-contribution-fail-modify-non-existing-directory`, json): expected status in [400, 409], got 412
  _cite: master08 §fail_modify_non_existing_directory (modify a directory that does not exist)_
- **ECC-CTB-027** List contributions — empty (`ctb/list-contributions-empty`, json): expected status 200, got 405 (body: {"error":"Method Not Allowed","message":"Request method 'GET' is not supported"})
  _cite: master08 §list_contributions-empty (retrieve an empty list); SM I_EHR_CONTRIBUTION.list_contributions; no ITS-REST binding — ehrbase-rs extension (GET /ehr/{ehr_id}/contribution → {rows,total}); RM common master06 §Contributions (initial EHR_STATUS is committed within a CONTRIBUTION → empty-list realized via the beyond-end page)_
- **ECC-CTB-028** List contributions — non existing EHR (`ctb/list-contributions-non-existing-ehr`, json): expected status 404, got 405 (body: {"error":"Method Not Allowed","message":"Request method 'GET' is not supported"})
  _cite: master08 §list_contributions-non_existing_ehr (error: EHR with ehr_id doesn't exist); SM I_EHR_CONTRIBUTION.list_contributions; no ITS-REST binding — ehrbase-rs extension (GET /ehr/{ehr_id}/contribution → 404 unknown ehr_id)_
- **ECC-CTB-029** List contributions — post commit (`ctb/list-contributions-post-commit`, json): expected status 200, got 405 (body: {"error":"Method Not Allowed","message":"Request method 'GET' is not supported"})
  _cite: master08 §list_contributions-post_commit (list reflects a committed VERSION<COMPOSITION>); SM I_EHR_CONTRIBUTION.list_contributions; no ITS-REST binding — ehrbase-rs extension (GET /ehr/{ehr_id}/contribution)_
- **ECC-CTB-030** List contributions — EHR containing directory (`ctb/list-contributions-ehr-containing-directory`, json): expected status 200, got 405 (body: {"error":"Method Not Allowed","message":"Request method 'GET' is not supported"})
  _cite: master08 §list_contributions-ehr_containing_directory (list reflects a committed VERSION<FOLDER>); SM I_EHR_CONTRIBUTION.list_contributions; no ITS-REST binding — ehrbase-rs extension (GET /ehr/{ehr_id}/contribution)_
- **ECC-CTB-031** List contributions — EHR containing EHR status (`ctb/list-contributions-ehr-containing-ehr-status`, json): expected status 200, got 405 (body: {"error":"Method Not Allowed","message":"Request method 'GET' is not supported"})
  _cite: master08 §list_contributions-ehr_containing_ehr_status (list reflects a committed VERSION<EHR_STATUS>); SM I_EHR_CONTRIBUTION.list_contributions; no ITS-REST binding — ehrbase-rs extension (GET /ehr/{ehr_id}/contribution)_
- **ECC-TPL-012** Validate OPT — invalid OPT (`tpl/validate-opt-invalid-opt`, json): 9/18 invalid OPTs rejected; first: invalid OPT minimal_action.opt accepted with 201 (expected 4xx)
  _cite: CNF master04 §I_DEFINITION_ADL14; ITS-REST 1.1.0 DEFINITION ADL 1.4 API (upload/get/validate); AM 1.4 §OPERATIONAL_TEMPLATE_
- **ECC-TPL-002** Upload OPT — invalid OPT (`tpl/upload-opt-invalid-opt`, json): 17/18 invalid OPTs rejected; first: invalid OPT minimal_admin_invalid_4.opt accepted with 500 (expected 4xx)
  _cite: CNF master04 §I_DEFINITION_ADL14; ITS-REST 1.1.0 DEFINITION ADL 1.4 API (upload/get/validate); AM 1.4 §OPERATIONAL_TEMPLATE_
- **ECC-TPL-014** Delete OPT — delete existing (`tpl/delete-opt-delete-existing`, json): expected status 204, got 404
  _cite: master04 §delete_opt-delete_existing (delete an existing OPT → 204, an unreferenced uniquified template); SM I_DEFINITION_ADL14.delete_opt(); no ITS-REST ADL 1.4 DELETE binding — ehrbase-rs extension (DELETE /admin/template/{template_id})_
- **ECC-TPL-015** Delete OPT — delete latest version (`tpl/delete-opt-delete-latest-version`, json): expected status 204, got 404
  _cite: master04 §delete_opt-delete_latest_version; the admin route has no version-addressed template resource (§upload_opt-valid_opt_twice NOTE — OPT versioning non-standard), so this is whole-template delete (204) with the physical delete leaving no trace (re-delete 404); SM I_DEFINITION_ADL14.delete_opt(); no ITS-REST ADL 1.4 DELETE binding — ehrbase-rs extension (DELETE /admin/template/{template_id})_
- **ECC-TPL-016** Delete OPT — delete specific version (`tpl/delete-opt-delete-specific-version`, json): expected status 201, got 400 (body: {"error":"Bad Request","message":"Message at /version/contribution (/version/contribution):  Attribute contribution must not be set"})
  _cite: master04 §delete_opt-delete_specific_version; no version-addressed resource on the admin route, so a template underpinning a specific committed version is refused (409 with a reference count — physical delete never orphans committed data); SM I_DEFINITION_ADL14.delete_opt(); no ITS-REST ADL 1.4 DELETE binding — ehrbase-rs extension (DELETE /admin/template/{template_id})_
- **ECC-QRY-002** Execute ad-hoc AQL query — empty db (`qry/execute-ad-hoc-query-empty-db`, json): adhoc empty_db golden mismatch (suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"}], served=[{"path":"e/ehr_id/value","name":"#0"}]
  _cite: CNF master11 §I_QUERY_SERVICE (stub, xx flow); ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query (200_QUERY.yaml RESULT_SET); AQL 1.1_
- **ECC-QRY-003** Execute stored AQL query — empty db (`qry/execute-stored-query-empty-db`, json): stored empty_db golden mismatch (suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"}], served=[{"path":"e/ehr_id/value","name":"#0"}]
  _cite: CNF master11 §I_QUERY_SERVICE (stub, xx flow); ITS-REST 1.1.0 QUERY API §execute_stored_query + DEFINITION QUERY §store; AQL 1.1_
- **ECC-QRY-004** Execute ad-hoc AQL query — loaded db (`qry/execute-ad-hoc-query-loaded-db`, json): expected column path /uid/value, got Some("c/uid/value")
  _cite: CNF master11 §I_QUERY_SERVICE (stub, xx flow); ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query; AQL 1.1 CONTAINS (master03-syntax §containsExpr)_
- **ECC-QRY-006** AQL corpus — A empty db (`qry/corpus-a-empty-db`, json): 0/24 A/empty_db goldens matched (3 skipped); first divergence: A/100_get_ehrs.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"}], served=[{"path":"e/ehr_id/value","name":"#0"}]
  _cite: AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query 200_QUERY.yaml; reference: CNF query corpus expected_results_
- **ECC-QRY-007** AQL corpus — B empty db (`qry/corpus-b-empty-db`, json): 0/17 B/empty_db goldens matched (4 skipped); first divergence: B/100_get_compositions_from_all_ehrs.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/"}], served=[{"path":"c","name":"#0"}]
  _cite: AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query 200_QUERY.yaml; reference: CNF query corpus expected_results_
- **ECC-QRY-008** AQL corpus — C empty db (`qry/corpus-c-empty-db`, json): 0/10 C/empty_db goldens matched (1 skipped); first divergence: C/100_get_entries_empty_db.json (Full, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/"}], served=[{"path":"entry","name":"#0"}]
  _cite: AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query 200_QUERY.yaml; reference: CNF query corpus expected_results_
- **ECC-QRY-009** AQL corpus — D empty db (`qry/corpus-d-empty-db`, json): 0/16 D/empty_db goldens matched (10 skipped); first divergence: D/200_select_data_values_from_all_ehrs_contains_composition.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"},{"name":"#1","path":"/time_created/value"},{"name":"#2","path":"/system_id/value"}], served=[{"path":"e/ehr_id/value","name":"#0"},{"path":"e/time_created/value","name":"#1"},{"path":"e/system_id/value","name":"#2"}]
  _cite: AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query 200_QUERY.yaml; reference: CNF query corpus expected_results_
- **ECC-QRY-010** AQL corpus — A loaded db (`qry/corpus-a-loaded-db`, json): 0/20 A/loaded_db goldens matched (7 skipped); first divergence: A/100_get_ehrs.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"}], served=[{"path":"e/ehr_id/value","name":"#0"}]
  _cite: AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query 200_QUERY.yaml; reference: CNF query corpus expected_results_
- **ECC-QRY-011** AQL corpus — B loaded db (`qry/corpus-b-loaded-db`, json): 0/14 B/loaded_db goldens matched (10 skipped); first divergence: B/100_get_compositions_from_all_ehrs.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/"}], served=[{"path":"c","name":"#0"}]
  _cite: AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query 200_QUERY.yaml; reference: CNF query corpus expected_results_
- **ECC-QRY-012** AQL corpus — C loaded db (`qry/corpus-c-loaded-db`, json): 0/6 C/loaded_db goldens matched (1 dialect-routed, 4 adjudicated untestable); first divergence: C/300_get_entries_with_type_from_ehr_with_uid_contains_compositions_with_archetype_from_all_ehrs.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/"}], served=[{"path":"entry","name":"#0"}]
  _cite: AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query 200_QUERY.yaml; reference: CNF query corpus expected_results_
- **ECC-QRY-013** AQL corpus — D loaded db (`qry/corpus-d-loaded-db`, json): 0/7 D/loaded_db goldens matched (19 skipped); first divergence: D/200_select_data_values_from_all_ehrs_contains_composition.json (ColumnsOnly, suppressed via [meta_envelope_ignored,query_echo_ignored]): columns differ: golden=[{"name":"#0","path":"/ehr_id/value"},{"name":"#1","path":"/time_created/value"},{"name":"#2","path":"/system_id/value"}], served=[{"path":"e/ehr_id/value","name":"#0"},{"path":"e/time_created/value","name":"#1"},{"path":"e/system_id/value","name":"#2"}]
  _cite: AQL 1.1 + the vendored golden RESULT_SETs; ITS-REST 1.1.0 QUERY API §execute_ad_hoc_query 200_QUERY.yaml; reference: CNF query corpus expected_results_
- **ECC-VAL-001** Validate COMPOSITION — content card any context any (`val/comp-content-card-any-context-any`, json): 0 content item(s), context present → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-002** Validate COMPOSITION — content card 1plus context any (`val/comp-content-card-1plus-context-any`, json): 0 content item(s), context present → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-003** Validate COMPOSITION — content card 3plus context any (`val/comp-content-card-3plus-context-any`, json): 0 content item(s), context present → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-004** Validate COMPOSITION — content card OPT context any (`val/comp-content-card-opt-context-any`, json): 0 content item(s), context present → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-005** Validate COMPOSITION — content card mand context any (`val/comp-content-card-mand-context-any`, json): 0 content item(s), context present → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-006** Validate COMPOSITION — content card 3to5 context any (`val/comp-content-card-3to5-context-any`, json): 0 content item(s), context present → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-007** Validate COMPOSITION — content card any context mand (`val/comp-content-card-any-context-mand`, json): 0 content item(s), context present → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-008** Validate COMPOSITION — content card 1plus context mand (`val/comp-content-card-1plus-context-mand`, json): 0 content item(s), context present → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-009** Validate COMPOSITION — content card 3plus context mand (`val/comp-content-card-3plus-context-mand`, json): 0 content item(s), context present → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-010** Validate COMPOSITION — content card OPT context mand (`val/comp-content-card-opt-context-mand`, json): 0 content item(s), context present → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-011** Validate COMPOSITION — content card mand context mand (`val/comp-content-card-mand-context-mand`, json): 0 content item(s), context present → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-012** Validate COMPOSITION — content card 3to5 context mand (`val/comp-content-card-3to5-context-mand`, json): 0 content item(s), context present → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 composition §COMPOSITION.content/context (content List 0..1, context 0..1, Category_validity only); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-013** Validate OBSERVATION — state ex OPT protocol ex OPT (`val/obs-state-ex-opt-protocol-ex-opt`, json): data present, state/protocol absent → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 ehr §OBSERVATION (data 1..1; state/protocol 0..1); AM aom14 §C_ATTRIBUTE existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-014** Validate OBSERVATION — state ex OPT protocol ex mand (`val/obs-state-ex-opt-protocol-ex-mand`, json): data present, state/protocol absent → rejected (existence.lower): reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 ehr §OBSERVATION (data 1..1; state/protocol 0..1); AM aom14 §C_ATTRIBUTE existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-015** Validate OBSERVATION — state ex mand protocol ex OPT (`val/obs-state-ex-mand-protocol-ex-opt`, json): data present, state/protocol absent → rejected (existence.lower): reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 ehr §OBSERVATION (data 1..1; state/protocol 0..1); AM aom14 §C_ATTRIBUTE existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-016** Validate OBSERVATION — state ex mand protocol ex mand (`val/obs-state-ex-mand-protocol-ex-mand`, json): data present, state/protocol absent → rejected (existence.lower): reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 ehr §OBSERVATION (data 1..1; state/protocol 0..1); AM aom14 §C_ATTRIBUTE existence; ITS-REST 1.1.0 composition_create (201 / 422 validation)_
- **ECC-VAL-017** Validate HISTORY — events card any summary ex OPT (`val/hist-events-card-any-summary-ex-opt`, json): 1 event(s), summary absent → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-018** Validate HISTORY — events card 1plus summary ex OPT (`val/hist-events-card-1plus-summary-ex-opt`, json): 1 event(s), summary absent → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-019** Validate HISTORY — events card 3plus summary ex OPT (`val/hist-events-card-3plus-summary-ex-opt`, json): 1 event(s), summary absent → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-020** Validate HISTORY — events card OPT summary ex OPT (`val/hist-events-card-opt-summary-ex-opt`, json): 1 event(s), summary absent → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-021** Validate HISTORY — events card mand summary ex OPT (`val/hist-events-card-mand-summary-ex-opt`, json): 1 event(s), summary absent → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-022** Validate HISTORY — events card 3to5 summary ex OPT (`val/hist-events-card-3to5-summary-ex-opt`, json): 1 event(s), summary absent → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-023** Validate HISTORY — events card any summary ex mand (`val/hist-events-card-any-summary-ex-mand`, json): 1 event(s), summary absent → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-024** Validate HISTORY — events card 1plus summary ex mand (`val/hist-events-card-1plus-summary-ex-mand`, json): 1 event(s), summary absent → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-025** Validate HISTORY — events card 3plus summary ex mand (`val/hist-events-card-3plus-summary-ex-mand`, json): 1 event(s), summary absent → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-026** Validate HISTORY — events card OPT summary ex mand (`val/hist-events-card-opt-summary-ex-mand`, json): 1 event(s), summary absent → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-027** Validate HISTORY — events card mand summary ex mand (`val/hist-events-card-mand-summary-ex-mand`, json): 1 event(s), summary absent → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-028** Validate HISTORY — events card 3to5 summary ex mand (`val/hist-events-card-3to5-summary-ex-mand`, json): 1 event(s), summary absent → rejected: reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §HISTORY (events cardinality; summary existence; Events_valid: ≥1 event OR summary); AM aom14 §C_ATTRIBUTE cardinality/existence; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-029** Validate EVENT — state ex OPT (`val/event-state-ex-opt`, json): data present, state absent → accepted: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_structures §EVENT/POINT_EVENT/INTERVAL_EVENT (state 0..1; INTERVAL_EVENT.width/math_function mandatory); AM aom14 §type narrowing; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-030** Validate EVENT — state ex mand (`val/event-state-ex-mand`, json): data present, state absent → rejected (EVENT.state existence.lower): reject (ITS-REST composition_create validation): status 204 matches no supported edition form (tried release-1.1.0=422, release-1.0.3=400); body: 
  _cite: RM 1.2.0 data_structures §EVENT/POINT_EVENT/INTERVAL_EVENT (state 0..1; INTERVAL_EVENT.width/math_function mandatory); AM aom14 §type narrowing; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-031** Validate EVENT — type any (`val/event-type-any`, json): POINT_EVENT accepted in an open EVENT slot: expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_structures §EVENT/POINT_EVENT/INTERVAL_EVENT (state 0..1; INTERVAL_EVENT.width/math_function mandatory); AM aom14 §type narrowing; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-032** Validate EVENT — type point event (`val/event-type-point-event`, json): POINT_EVENT accepted (events narrowed to POINT_EVENT): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_structures §EVENT/POINT_EVENT/INTERVAL_EVENT (state 0..1; INTERVAL_EVENT.width/math_function mandatory); AM aom14 §type narrowing; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-033** Validate EVENT — type interval event (`val/event-type-interval-event`, json): INTERVAL_EVENT accepted (events narrowed to INTERVAL_EVENT): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_structures §EVENT/POINT_EVENT/INTERVAL_EVENT (state 0..1; INTERVAL_EVENT.width/math_function mandatory); AM aom14 §type narrowing; ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-034** Validate ITEM_STRUCTURE — type any (`val/item-str-type-any`, json): OPT upload returned 400 (expected 2xx or 409 already-present)
  _cite: RM 1.2.0 data_structures §ITEM_STRUCTURE (ITEM_TREE/LIST/TABLE/SINGLE); AM aom14 §type narrowing (Class not allowed); ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-035** Validate ITEM_STRUCTURE — type item tree (`val/item-str-type-item-tree`, json): OPT upload returned 400 (expected 2xx or 409 already-present)
  _cite: RM 1.2.0 data_structures §ITEM_STRUCTURE (ITEM_TREE/LIST/TABLE/SINGLE); AM aom14 §type narrowing (Class not allowed); ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-036** Validate ITEM_STRUCTURE — type item list (`val/item-str-type-item-list`, json): OPT upload returned 400 (expected 2xx or 409 already-present)
  _cite: RM 1.2.0 data_structures §ITEM_STRUCTURE (ITEM_TREE/LIST/TABLE/SINGLE); AM aom14 §type narrowing (Class not allowed); ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-037** Validate ITEM_STRUCTURE — type item table (`val/item-str-type-item-table`, json): OPT upload returned 400 (expected 2xx or 409 already-present)
  _cite: RM 1.2.0 data_structures §ITEM_STRUCTURE (ITEM_TREE/LIST/TABLE/SINGLE); AM aom14 §type narrowing (Class not allowed); ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-038** Validate ITEM_STRUCTURE — type item single (`val/item-str-type-item-single`, json): OPT upload returned 400 (expected 2xx or 409 already-present)
  _cite: RM 1.2.0 data_structures §ITEM_STRUCTURE (ITEM_TREE/LIST/TABLE/SINGLE); AM aom14 §type narrowing (Class not allowed); ITS-REST 1.1.0 composition_create (201 / 422)_
- **ECC-VAL-039** Validate DV_BOOLEAN — anything allowed (`val/dv-boolean-anything-allowed`, json): DV_BOOLEAN with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_BOOLEAN; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-040** Validate DV_BOOLEAN — only true allowed (`val/dv-boolean-only-true-allowed`, json): value true allowed (C_BOOLEAN true-only): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_BOOLEAN {true}; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-041** Validate DV_BOOLEAN — only false allowed (`val/dv-boolean-only-false-allowed`, json): value false allowed (C_BOOLEAN false-only): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_BOOLEAN; AM 1.4 C_BOOLEAN {false}; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-042** Validate DV_IDENTIFIER — all pattern (`val/dv-identifier-all-pattern`, json): id 54480987 matches [0-9]+ (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_IDENTIFIER; AM 1.4 C_STRING.pattern on id; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-043** Validate DV_IDENTIFIER — all list (`val/dv-identifier-all-list`, json): id 54480987 in list (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_IDENTIFIER; AM 1.4 C_STRING.list on id; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-044** Validate DV_TEXT — open (`val/dv-text-open`, json): DV_TEXT with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_TEXT; AM 1.4 C_STRING open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-045** Validate DV_TEXT — list (`val/dv-text-list`, json): DV_TEXT value in the C_STRING list (accepted): expected accepted (composition_create.yaml 201), got 500 ({"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: RM 1.2.0 data_types §DV_TEXT; AM 1.4 C_STRING.list; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-046** Validate DV_CODED_TEXT — open (`val/dv-coded-text-open`, json): DV_CODED_TEXT with defining_code (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_CODE_PHRASE open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-047** Validate DV_CODED_TEXT — local codes (`val/dv-coded-text-local-codes`, json): DV_CODED_TEXT local::at0023 in code_list (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_CODE_PHRASE local code_list; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-048** Validate DV_CODED_TEXT — ext term (`val/dv-coded-text-ext-term`, json): SNOMED-CT 73211009 in the external code_list (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_CODED_TEXT; AM 1.4 C_CODE_PHRASE external terminology; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-049** Validate DV_ORDINAL — open (`val/dv-ordinal-open`, json): DV_ORDINAL with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_ORDINAL; AM 1.4 C_DV_ORDINAL open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-050** Validate DV_ORDINAL — constraint (`val/dv-ordinal-constraint`, json): DV_ORDINAL symbol local::at0014 in list (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_ORDINAL; AM 1.4 C_DV_ORDINAL.list; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-051** Validate DV_SCALE — open (`val/dv-scale-open`, json): DV_SCALE with value+symbol (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_SCALE (RM ≥ 1.1.0, SPECRM-19); AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-052** Validate DV_SCALE — constraint (`val/dv-scale-constraint`, json): DV_SCALE value 1.0 in list {1.0} (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_SCALE (RM ≥ 1.1.0); AM 1.4 C_REAL.list on value (no C_DV_SCALE in AM 1.4, SPECPR-381); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-053** Validate DV_COUNT — open (`val/dv-count-open`, json): DV_COUNT with magnitude (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_INTEGER open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-054** Validate DV_COUNT — range (`val/dv-count-range`, json): DV_COUNT magnitude 3 in range [0,10] (accepted): expected accepted (composition_create.yaml 201), got 500 ({"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_INTEGER.range; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-055** Validate DV_COUNT — list (`val/dv-count-list`, json): DV_COUNT magnitude 3 in the C_INTEGER list {3} (accepted): expected accepted (composition_create.yaml 201), got 500 ({"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: RM 1.2.0 data_types §DV_COUNT; AM 1.4 C_INTEGER.list; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-056** Validate DV_QUANTITY — open (`val/dv-quantity-open`, json): DV_QUANTITY with magnitude (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_DV_QUANTITY open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-057** Validate DV_QUANTITY — property (`val/dv-quantity-property`, json): units mg matches property mass openehr::124 (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_DV_QUANTITY.property; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-058** Validate DV_QUANTITY — property units (`val/dv-quantity-property-units`, json): DV_QUANTITY units 'mg' in [mg,kg] (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_DV_QUANTITY units list; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-059** Validate DV_QUANTITY — property units mag (`val/dv-quantity-property-units-mag`, json): DV_QUANTITY 702.9 mm3 in magnitude range [0,inf) (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_QUANTITY; AM 1.4 C_DV_QUANTITY units list + magnitude range; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-060** Validate DV_PROPORTION — open (`val/dv-proportion-open`, json): DV_PROPORTION with numerator (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PROPORTION (kind invariants); AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-061** Validate DV_PROPORTION — ratio (`val/dv-proportion-ratio`, json): type 0 in list {0} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PROPORTION (ratio kind 0); AM 1.4 C_INTEGER.list on type; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-062** Validate DV_PROPORTION — unitary (`val/dv-proportion-unitary`, json): type 1 in list {1} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PROPORTION (unitary kind 1); AM 1.4 C_INTEGER.list on type; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-063** Validate DV_PROPORTION — percent (`val/dv-proportion-percent`, json): type 2 in list {2} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PROPORTION (percent kind 2); AM 1.4 C_INTEGER.list on type; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-064** Validate DV_PROPORTION — fraction (`val/dv-proportion-fraction`, json): type 3 in list {3} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PROPORTION (fraction kind 3); AM 1.4 C_INTEGER.list on type; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-065** Validate DV_PROPORTION — integer fraction (`val/dv-proportion-integer-fraction`, json): type 4 in list {4} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PROPORTION (integer_fraction kind 4); AM 1.4 C_INTEGER.list on type; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-066** Validate DV_PROPORTION — any fraction (`val/dv-proportion-any-fraction`, json): DV_PROPORTION type 3 (fraction) in list [3,4] (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_INTEGER.list {3,4} on type; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-067** Validate DV_PROPORTION — ratio range (`val/dv-proportion-ratio-range`, json): numerator 398.5 in range [0,1000] (accepted): expected accepted (composition_create.yaml 201), got 500 ({"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: RM 1.2.0 data_types §DV_PROPORTION; AM 1.4 C_REAL.range on numerator; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-068** Validate DV_INTERVAL<DV_COUNT> — open (`val/dv-interval-dv-count-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-069** Validate DV_INTERVAL<DV_COUNT> — lower upper (`val/dv-interval-dv-count-lower-upper`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-070** Validate DV_INTERVAL<DV_COUNT> — lower upper list (`val/dv-interval-dv-count-lower-upper-list`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_COUNT>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-071** Validate DV_INTERVAL<DV_QUANTITY> — open (`val/dv-interval-dv-quantity-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_QUANTITY>; BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-072** Validate DV_INTERVAL<DV_QUANTITY> — upper lower (`val/dv-interval-dv-quantity-upper-lower`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_QUANTITY>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-073** Validate DV_INTERVAL<DV_DATE_TIME> — open (`val/dv-interval-dv-date-time-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-074** Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint (`val/dv-interval-dv-date-time-lower-upper-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-075** Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range (`val/dv-interval-dv-date-time-lower-upper-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DATE_TIME>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-076** Validate DV_INTERVAL<DV_DATE> — open (`val/dv-interval-dv-date-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-077** Validate DV_INTERVAL<DV_DATE> — lower upper constraint (`val/dv-interval-dv-date-lower-upper-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-078** Validate DV_INTERVAL<DV_DATE> — lower upper range (`val/dv-interval-dv-date-lower-upper-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DATE>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-079** Validate DV_INTERVAL<DV_TIME> — open (`val/dv-interval-dv-time-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-080** Validate DV_INTERVAL<DV_TIME> — lower upper constraint (`val/dv-interval-dv-time-lower-upper-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-081** Validate DV_INTERVAL<DV_TIME> — lower upper range (`val/dv-interval-dv-time-lower-upper-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_TIME>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-082** Validate DV_INTERVAL<DV_DURATION> — open (`val/dv-interval-dv-duration-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-083** Validate DV_INTERVAL<DV_DURATION> — constraint (`val/dv-interval-dv-duration-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-084** Validate DV_INTERVAL<DV_DURATION> — range (`val/dv-interval-dv-duration-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_DURATION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-085** Validate DV_INTERVAL<DV_ORDINAL> — open (`val/dv-interval-dv-ordinal-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_ORDINAL>; BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-086** Validate DV_INTERVAL<DV_ORDINAL> — constraint (`val/dv-interval-dv-ordinal-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_ORDINAL>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-087** Validate DV_INTERVAL<DV_SCALE> — open (`val/dv-interval-dv-scale-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_SCALE> (RM ≥ 1.1.0); BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-088** Validate DV_INTERVAL<DV_SCALE> — constraint (`val/dv-interval-dv-scale-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_SCALE> (RM ≥ 1.1.0); BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-089** Validate DV_INTERVAL<DV_PROPORTION> — open (`val/dv-interval-dv-proportion-open`, json): valid DV_INTERVAL, bounded + included, lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval invariants; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-090** Validate DV_INTERVAL<DV_PROPORTION> — ratio (`val/dv-interval-dv-proportion-ratio`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-091** Validate DV_INTERVAL<DV_PROPORTION> — unitary (`val/dv-interval-dv-proportion-unitary`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-092** Validate DV_INTERVAL<DV_PROPORTION> — percentage (`val/dv-interval-dv-proportion-percentage`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-093** Validate DV_INTERVAL<DV_PROPORTION> — fraction (`val/dv-interval-dv-proportion-fraction`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-094** Validate DV_INTERVAL<DV_PROPORTION> — integer fraction (`val/dv-interval-dv-proportion-integer-fraction`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-095** Validate DV_INTERVAL<DV_PROPORTION> — ratio range (`val/dv-interval-dv-proportion-ratio-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_INTERVAL<DV_PROPORTION>; BASE foundation_types §Interval (lower ≤ upper); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-096** Validate DV_DURATION — open (`val/dv-duration-open`, json): DV_DURATION with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_DURATION; AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-097** Validate DV_DURATION — fields (`val/dv-duration-fields`, json): DV_DURATION base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_DURATION.pattern (allowed fields); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-098** Validate DV_DURATION — range (`val/dv-duration-range`, json): DV_DURATION base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_DURATION.range; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-099** Validate DV_DURATION — fields range (`val/dv-duration-fields-range`, json): DV_DURATION base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_DURATION; AM 1.4 C_DURATION.pattern + range; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-100** Validate DV_TIME — open (`val/dv-time-open`, json): DV_TIME with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_TIME; AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-101** Validate DV_TIME — constraint (`val/dv-time-constraint`, json): DV_TIME base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 500 ({"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: RM 1.2.0 data_types §DV_TIME; AM 1.4 C_TIME.pattern; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-102** Validate DV_TIME — range (`val/dv-time-range`, json): DV_TIME base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 500 ({"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: RM 1.2.0 data_types §DV_TIME; AM 1.4 C_TIME.range; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-103** Validate DV_DATE — open (`val/dv-date-open`, json): DV_DATE with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_DATE; AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-104** Validate DV_DATE — constraint (`val/dv-date-constraint`, json): DV_DATE base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 500 ({"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: RM 1.2.0 data_types §DV_DATE; AM 1.4 C_DATE.pattern; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-105** Validate DV_DATE — range (`val/dv-date-range`, json): DV_DATE base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 500 ({"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: RM 1.2.0 data_types §DV_DATE; AM 1.4 C_DATE.range; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-119** Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected) (`val/dv-date-day-disallowed-pattern`, json): expected status 422, got 204 (body: )
  _cite: AM 1.4 C_DATE (yyyy-??-XX: month optional, day disallowed; org.openehr.am.aom14.c_date.adoc); ITS-REST 1.1.0 composition_create (422 rejected)_
- **ECC-VAL-106** Validate DV_DATE_TIME — open (`val/dv-date-time-open`, json): DV_DATE_TIME with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-107** Validate DV_DATE_TIME — constraint (`val/dv-date-time-constraint`, json): DV_DATE_TIME full timestamp matches yyyy-mm-ddTHH:MM:SS (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 C_DATE_TIME.pattern (yyyy-mm-ddTHH:MM:SS); ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-108** Validate DV_DATE_TIME — range (`val/dv-date-time-range`, json): DV_DATE_TIME base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_DATE_TIME; AM 1.4 C_DATE_TIME.range; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-109** Validate DV_PARSABLE — open (`val/dv-parsable-open`, json): DV_PARSABLE with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PARSABLE; AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-110** Validate DV_PARSABLE — value formalism (`val/dv-parsable-value-formalism`, json): formalism ISO8601 in list (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_PARSABLE; AM 1.4 C_STRING.list on formalism; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-111** Validate DV_MULTIMEDIA — open (`val/dv-multimedia-open`, json): DV_MULTIMEDIA with media_type (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_MULTIMEDIA; AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-112** Validate DV_MULTIMEDIA — media type (`val/dv-multimedia-media-type`, json): media_type image/png in list (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_MULTIMEDIA; AM 1.4 C_CODE_PHRASE on media_type; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-113** Validate DV_URI — open (`val/dv-uri-open`, json): DV_URI with value (RM present): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_URI (RFC3986 validity); AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-114** Validate DV_URI — pattern (`val/dv-uri-pattern`, json): URI http://ok matches pattern (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_URI; AM 1.4 C_STRING.pattern; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-115** Validate DV_URI — list (`val/dv-uri-list`, json): URI http://ok in list (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_URI; AM 1.4 C_STRING.list; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-116** Validate DV_EHR_URI — open (`val/dv-ehr-uri-open`, json): DV_EHR_URI with value (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_EHR_URI (ehr: scheme); AM 1.4 open; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-117** Validate DV_EHR_URI — pattern (`val/dv-ehr-uri-pattern`, json): ehr://x matches pattern (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_EHR_URI; AM 1.4 C_STRING.pattern; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-VAL-118** Validate DV_EHR_URI — list (`val/dv-ehr-uri-list`, json): ehr://ok in list (accepted): expected accepted (composition_create.yaml 201), got 204 ()
  _cite: RM 1.2.0 data_types §DV_EHR_URI; AM 1.4 C_STRING.list; ITS-REST 1.1.0 composition_create (201/422)_
- **ECC-ADM-001** Admin EHR delete (`adm/ehr-delete`, json): physical_ehr_delete 204: status 404 matches no supported edition form (tried release-1.1.0=204); body: {"error":"Not Found","message":"No resource found at path: rest/admin/ehr/85f43461-bfbc-4773-898b-4b9014fef78a"}
  _cite: CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete.yaml 204; SM I_ADMIN_SERVICE.physical_ehr_delete_
- **ECC-ADM-003** Admin EHR delete idempotent (`adm/ehr-delete-idempotent`, json): physical_ehr_delete 204: status 404 matches no supported edition form (tried release-1.1.0=204); body: {"error":"Not Found","message":"No resource found at path: rest/admin/ehr/89b34d4c-3cf9-44cc-9917-8f2b794ed963"}
  _cite: CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete.yaml 204 then 404 (physical delete leaves no trace); SM I_ADMIN_SERVICE.physical_ehr_delete_
- **ECC-ADM-004** Admin EHR delete all (`adm/ehr-delete-all`, json): physical_ehr_delete bulk 204: status 404 matches no supported edition form (tried release-1.1.0=204); body: {"error":"Not Found","message":"No resource found at path: rest/admin/ehr/all"}
  _cite: CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete_all.yaml 204 (204_deleted_hard, bodyless); SM I_ADMIN_SERVICE.physical_ehr_delete_
- **ECC-ADM-005** Admin EHR delete all partial (`adm/ehr-delete-all-partial`, json): physical_ehr_delete partial bulk 204: status 404 matches no supported edition form (tried release-1.1.0=204); body: {"error":"Not Found","message":"No resource found at path: rest/admin/ehr/all"}
  _cite: CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete_all.yaml — instrument-encodes-server-behaviour: a bulk set including a missing id still 204s (OAS declares no per-id failure)_
- **ECC-TS-001** TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET (`ts/expand-bundle-accepted`, json): expected status 200, got 400 (body: {"error":"Bad Request","message":"Not implemented: Only primitive operands are supported"})
  _cite: QUERY master03 §Functions/Other functions/TERMINOLOGY (lines 748–767); AQL 1.1; ITS-REST 1.1.0 QUERY API execute_ad_hoc_query; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS)_
- **ECC-TS-002** TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes (`ts/expand-bundle-constrains`, json): expected status 200, got 400 (body: {"error":"Bad Request","message":"Not implemented: Only primitive operands are supported"})
  _cite: QUERY master03 §Functions/Other functions/TERMINOLOGY (lines 748–767); AQL 1.1; ITS-REST 1.1.0 QUERY API execute_ad_hoc_query; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS)_
- **ECC-TS-003** TERMINOLOGY expand (bundle) — explicit code merged with the expansion (`ts/expand-bundle-mixed-list`, json): expected status 200, got 400 (body: {"error":"Bad Request","message":"Not implemented: Only primitive operands are supported"})
  _cite: QUERY master03 §Functions/Other functions/TERMINOLOGY (lines 748–767); AQL 1.1; ITS-REST 1.1.0 QUERY API execute_ad_hoc_query; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS)_
- **ECC-SF-001** FLAT commit then read-back as FLAT, canonical JSON, and STRUCTURED (`sf/flat-commit-read-back`, json): expected status 201, got 415 (body: {"timestamp":"2026-07-21T04:52:00.585+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/d623b20e-a89c-48ed-9ed6-8584bf71ce93/composition"})
  _cite: ITS-REST simplified_formats master04 §Flat format + master05 §RM Mapping; Resources.md §Simplified Formats (application/openehr.wt.flat+json); Requests_and_responses.md §openehr-template-id_
- **ECC-SF-002** STRUCTURED commit then read-back as STRUCTURED, canonical JSON, and FLAT (`sf/structured-commit-read-back`, json): expected status 201, got 415 (body: {"timestamp":"2026-07-21T04:52:00.597+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/b654efac-18a5-4256-8320-8e0281cff619/composition"})
  _cite: ITS-REST simplified_formats master04 §Structured format + master05 §RM Mapping; Resources.md §Simplified Formats (application/openehr.wt.structured+json); Requests_and_responses.md §openehr-template-id_
- **ECC-SF-003** Accept q-values select the highest-weight simplified format; every non-204 carries Content-Type (`sf/negotiation-qvalue`, json): expected status 201, got 415 (body: {"timestamp":"2026-07-21T04:52:00.606+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/accedd29-2786-4324-8d6f-61abc5012985/composition"})
  _cite: Resources.md §Data representation (RFC 9110 §12.5.1 quality-value negotiation) + §Simplified Formats (Content-Type MUST be present unless 204)_
- **ECC-SF-004** Deprecated + legacy simplified media types are rejected on Accept (406) (`sf/reject-retired-media-type-accept`, json): expected status 201, got 415 (body: {"timestamp":"2026-07-21T04:52:00.615+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/ee3aae2f-0637-4233-a094-b1efee5709e6/composition"})
  _cite: Resources.md §Simplified Formats NOTE (deprecated .schema+json) + §Alternative data formats (legacy nc.flat/tds2) + the 406 MUST rule ("If the service cannot fulfill this aspect of the request, it MUST respond with 406 Not Acceptable")_
- **ECC-SF-005** Deprecated + legacy simplified media types are rejected on write Content-Type (415) (`sf/reject-retired-media-type-content-type`, json): application/openehr.wt.flat.schema+json on composition POST: expected status 415, got 500 (body: {"error":"Internal Server Error","message":"An internal error has occurred. Please contact your administrator."})
  _cite: Resources.md §Simplified Formats NOTE (deprecated .schema+json) + §Alternative data formats (legacy nc.flat/tds2) + the 415 MUST rule ("If the service cannot process the request payload as the simplified format is not supported, it MUST respond with 415 Unsupported Media Type")_
- **ECC-SF-006** FLAT commit without openehr-template-id (and no payload template id) → 422 (`sf/flat-missing-template-id`, json): expected status 422, got 415 (body: {"timestamp":"2026-07-21T04:52:00.633+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/bcc79edf-2c9f-4613-b032-b464413a4d71/composition"})
  _cite: Requests_and_responses.md §openehr-template-id ("MUST be used whenever committing COMPOSITION using a Simplified Format which does not support TEMPLATE_ID under archetype_details.template_id") + §HTTP status codes row 422 (well-formed but unprocessable)_
- **ECC-SF-007** FLAT commit with an unknown field identifier → 422 (`sf/flat-reject-unknown-field`, json): expected status 422, got 415 (body: {"timestamp":"2026-07-21T04:52:00.642+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/b0100a89-d5ef-4994-aeec-7eccde62bc60/composition"})
  _cite: ITS-REST simplified_formats master04 §Validation ("Field identifiers match WT metadata structure"); Requests_and_responses.md §HTTP status codes row 422_
- **ECC-SF-008** FLAT commit with |other combined with |code on one coded leaf → 422 (`sf/flat-reject-other-with-code`, json): expected status 422, got 415 (body: {"timestamp":"2026-07-21T04:52:00.651+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/128eb8fd-07d7-4162-a3c2-4ec62bb08a21/composition"})
  _cite: ITS-REST simplified_formats master04 §Open Value-Sets and the |other Suffix ("|other is mutually exclusive with |code, |value and |terminology on the same leaf path; servers MUST reject combinations"); Requests_and_responses.md §HTTP status codes row 422_
- **ECC-SF-010** GET a template example in each of the four Accept forms (json, xml, flat, structured) (`sf/template-example-accept-forms`, json): application/openehr.wt.flat+json on example GET: expected status 200, got 406 (body: {"error":"Not Acceptable","message":"No acceptable representation"})
  _cite: Resources.md §Data representation + §Simplified Formats (the LOCATABLE example is negotiable across canonical JSON/XML and the FLAT/STRUCTURED simplified forms); the Content-Type MUST match the negotiated format_
- **ECC-SF-012** CONTRIBUTION with a FLAT COMPOSITION inner payload: canonical envelope in, simplified read-back (`sf/contribution-flat-commit-read-back`, json): expected status 201, got 415 (body: {"timestamp":"2026-07-21T04:52:00.699+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/a174f4c6-c179-49cc-8772-942a9739fa47/contribution"})
  _cite: ITS-REST contribution_create.yaml + contribution_get.yaml §Simplified Formats (the CONTRIBUTION envelope stays canonical JSON; each versions[i].data COMPOSITION is simplified); simplified_formats master05 §scope (COMPOSITION + contained classes only)_
- **ECC-SF-014** DIRECTORY (FOLDER) has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 (`sf/non-templated-directory-reject`, json): expected status 406, got 404 (body: {"error":"Not Found","message":"EHR with id 5da4c003-1e6a-4266-9dfb-90bf5d7309af not found"})
  _cite: ITS-REST simplified_formats master05 §scope (FOLDER is not templated) + Resources.md §Simplified Formats 406/415 MUST rules_
- **ECC-SF-015** Demographic PARTY has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 (`sf/non-templated-demographic-reject`, json): expected status 406, got 404 (body: {"error":"Not Found","message":"No resource found at path: rest/openehr/v1/demographic/person/82d2d4e9-b8c0-465d-b837-a68c3364175e"})
  _cite: ITS-REST simplified_formats master05 §scope (demographic PARTY types are not templated) + Resources.md §Simplified Formats 406/415 MUST rules_
- **ECC-SF-016** FLAT ctx/time sets EVENT_CONTEXT.start_time; ctx/setting defaults to openehr::238 (`sf/ctx-observability`, json): expected status 201, got 415 (body: {"timestamp":"2026-07-21T04:52:00.720+00:00","status":415,"error":"Unsupported Media Type","path":"/ehrbase/rest/openehr/v1/ehr/abe699ad-bb77-4cc5-a1fa-477edd9dce58/composition"})
  _cite: ITS-REST simplified_formats master06 §time (ctx/time sets COMPOSITION.context.start_time) + §setting (ctx/setting defaults to openehr::238|other care| when not set)_
- **ECC-ADL2-001** Upload a valid ADL2 template → 201 with Location; Prefer selects minimal/representation/identifier bodies (`adl2/upload-201-prefer-triad`, json): expected status 201, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_upload.yaml (text/plain source) + 201_Template_adl2_upload.yaml (Location + Prefer: minimal empty / representation source / identifier TemplateIdentifier JSON)_
- **ECC-ADL2-002** Upload the same ADL2 HRID twice → the second is a 409 conflict (`adl2/upload-duplicate-conflict`, json): expected status 201, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_upload.yaml §409 (409_template_already_exists.yaml — a template with the same template_id already exists)_
- **ECC-ADL2-003** Upload an unparseable ADL2 source → 422 carrying syntax rule codes in validationErrors (`adl2/upload-unparseable-422`, json): expected status 422, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_upload.yaml (an invalid source is rejected; our wire renders the Error object with validationErrors — the OAS folds this under 400, the served surface documents 422)_
- **ECC-ADL2-004** Upload a semantically invalid ADL2 template (missing description) → 422 with the AOM2 rule code VARD (`adl2/upload-invalid-422-vcode`, json): expected status 422, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_upload.yaml; AOM2 master03-archetype_definitions §Validity Rules VARD (a description section is mandatory) — reported as a rule code in validationErrors_
- **ECC-ADL2-005** Upload a parent archetype, then a specialised child that validates against the stored parent → 201 (`adl2/upload-specialised-child-resolves-parent`, json): expected status 201, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_upload.yaml; AOM2 master05-specialisation §Specialisation (a specialised archetype is validated against its flat parent, resolved from the repository)_
- **ECC-ADL2-006** Get an ADL2 template as text/plain source, application/json OperationalTemplateV2, and 406 on xml-only (`adl2/get-representations`, json): expected status 201, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_get.yaml + 200_Template_adl2_retrieved.yaml (text/plain source | application/json OperationalTemplateV2) + Accept_Template_adl2.yaml (application/xml has no declared response body → 406)_
- **ECC-ADL2-008** Version get resolves an exact SEMVER and a major prefix (latest match) → 200; an unknown version → 404 (`adl2/version-get-exact-prefix-unknown`, json): expected status 201, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_version_get.yaml (deprecated but served) + 200_Template_adl2_retrieved.yaml + template_id_adl2.yaml (a partial template_id resolves to the latest matching major version)_
- **ECC-ADL2-009** Get a template example in each of the four Accept_LOCATABLE forms → 200; the JSON form is a COMPOSITION rooted at the template's archetype (`adl2/example-four-accept-forms`, json): expected status 201, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_example_get.yaml + 200_Template_example_retrieved.yaml (LOCATABLE oneOf) + Accept_LOCATABLE.yaml (json / xml / wt.flat+json / wt.structured+json)_
- **ECC-ADL2-010** Example honours the detail_level enum (required/medium/complete) and rejects a bad type/detail_level with 400 (`adl2/example-detail-levels-and-bad-enum`, json): expected status 201, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_example_get.yaml + example_type.yaml (input|output) + example_detail_level.yaml (required|medium|complete) + 400.yaml (out-of-enum → 400)_
- **ECC-ADL2-011** Example for an unknown template_id → 404; an Accept outside the four LOCATABLE forms → 406 (`adl2/example-unknown-404-wrong-accept-406`, json): expected status 404, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_example_get.yaml §404 (404_unknown_template_id.yaml) + §406 (406.yaml) + Accept_LOCATABLE.yaml_
- **ECC-ADL2-012** List ADL2 templates → TemplateMetadata carrying template_id, concept, archetype_id, created_timestamp (`adl2/list-template-metadata`, json): expected status 201, got 501 (body: )
  _cite: ITS-REST definition_template_adl2_list.yaml + 200_TemplateList_adl2.yaml + schemas/definition/TemplateMetadata.yaml (template_id / concept / archetype_id / created_timestamp)_
- **ECC-AQT-001** TERMINOLOGY('expand') as a matches operand filters committed compositions by the value set's codes (`aqt/expand-matches-over-committed-data`, json): expected status 200, got 400 (body: {"error":"Bad Request","message":"Not implemented: Only primitive operands are supported"})
  _cite: QUERY master03-syntax §Functions/Other functions/TERMINOLOGY (lines 699–767); AQL 1.1; ITS-REST 1.1.0 QUERY API execute_ad_hoc_query (200_QUERY / 400_QUERY); profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS)_

## 6. Skipped, by reason

| Reason | Cases |
|---|--:|
| SutConfig: FHIR terminology provider not exercisable — the SUT answered 400 to a `hl7.org/fhir/4.0` expand (a configured provider lacking the fixture value set, or a non-provider rejection). Not a fabricated pass. | 1 |
| SutConfig: the 5xx fault requires a fault-injecting terminology server wired to the SUT (the composed run points the SUT's [terminology.external] provider at the fixture via host.docker.internal); this run is not wired. Harness tx server: http://127.0.0.1:59779 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_server_error_is_5xx + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception. | 1 |
| SutConfig: the malformed fault requires a fault-injecting terminology server wired to the SUT (the composed run points the SUT's [terminology.external] provider at the fixture via host.docker.internal); this run is not wired. Harness tx server: http://127.0.0.1:59779 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_malformed_is_not_json + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception. | 1 |
| SutConfig: the timeout fault requires a fault-injecting terminology server wired to the SUT (the composed run points the SUT's [terminology.external] provider at the fixture via host.docker.internal); this run is not wired. Harness tx server: http://127.0.0.1:59779 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception. | 1 |
| destructive case runs only against disposable composed SUTs (an empty ehr_id selector deletes ALL EHRs); skipped for a foreign / bring-your-own endpoint | 1 |

## 7. Not applicable to this SUT (extensions / RM-version-sensitive)

Adjudicated in the committed fairness register (foreign SUTs only), not a conformance finding — excluded from pass/fail and capability math.

- **ECC-DEM-001** Demographic person create — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-021** Demographic create bad body — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-002** Demographic person get — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-007** Demographic person get absent — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-006** Demographic person get deleted — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-003** Demographic person get by version — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-025** Demographic person get at time — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-004** Demographic person update — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-008** Demographic person update bad if match — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-005** Demographic person delete — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-009** Demographic agent create — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-010** Demographic agent get — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-011** Demographic agent delete — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-012** Demographic group create — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-013** Demographic group get — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-014** Demographic group delete — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-015** Demographic organisation create — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-016** Demographic organisation get — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-017** Demographic organisation delete — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-018** Demographic role create — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-019** Demographic role get — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-020** Demographic role delete — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-022** Demographic versioned party get — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-023** Demographic versioned party revision history — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-024** Demographic person tags — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-026** Demographic relationship create — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-027** Demographic relationship get — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-028** Demographic relationship get at time — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-029** Demographic relationship update — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-030** Demographic relationship delete — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-DEM-031** Demographic relationship get by version — Upstream EHRbase exposes no demographic REST API; the DEM cases exercise ehrbase-rs's own demographic wire. _(cite: no openEHR spec governs this fairness call — upstream has no demographic REST API; the RM demographic wire is our own design/extension)_
- **ECC-ADM-007** Admin list contributions — NativeApiOnly: I_ADMIN_SERVICE.list_contributions is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §list_contributions (TBD stub); SM I_ADMIN_SERVICE.list_contributions — no ITS-REST admin route)_
- **ECC-ADM-008** Admin contribution count — NativeApiOnly: I_ADMIN_SERVICE.contribution_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §contribution_count (TBD stub); SM I_ADMIN_SERVICE.contribution_count — no ITS-REST admin route)_
- **ECC-ADM-009** Admin versioned composition count — NativeApiOnly: I_ADMIN_SERVICE.versioned_composition_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §versioned_composition_count (TBD stub); SM I_ADMIN_SERVICE.versioned_composition_count — no ITS-REST admin route)_
- **ECC-ADM-010** Admin composition version count — NativeApiOnly: I_ADMIN_SERVICE.composition_version_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §composition_version_count (TBD stub); SM I_ADMIN_SERVICE.composition_version_count — no ITS-REST admin route)_
- **ECC-ADM-011** Admin export EHRs (dump/load) — NativeApiOnly: I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs is exercised by app/ehrbase/tests/service_dump_load.rs::export_then_load_into_fresh_db_round_trips_byte_equal — no ITS-REST admin route reaches it _(cite: CNF master12 §export_ehrs (TBD stub); SM I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs — no ITS-REST admin route)_
- **ECC-ADM-012** Admin archive EHRs — NativeApiOnly: I_ADMIN_ARCHIVE.archive_ehrs is exercised by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged — no ITS-REST admin route reaches it _(cite: CNF master12 §archive_ehrs (TBD stub); SM I_ADMIN_ARCHIVE.archive_ehrs — no ITS-REST admin route)_
- **ECC-ADM-013** Admin physical party delete — NoRestBinding: I_ADMIN_SERVICE.physical_party_delete has no ITS-REST route and acts on the demographic extension; exercised natively by app/ehrbase/tests/service_admin.rs::physical_party_delete_cascades_relationships_and_spares_partner _(cite: CNF master12 §physical_party_delete (TBD stub); SM I_ADMIN_SERVICE.physical_party_delete acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route)_
- **ECC-ADM-014** Admin archive parties — NoRestBinding: I_ADMIN_ARCHIVE.archive_parties has no ITS-REST route and acts on the demographic extension; the archive path is proven natively by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged _(cite: CNF master12 §archive_parties (TBD stub); SM I_ADMIN_ARCHIVE.archive_parties acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route)_
- **ECC-MSG-001** EHR Extract — export whole EHR (export_ehrs) — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehrs; RM EHR Extract IM (X_VERSIONED_*); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub, listed twice — authoring duplicate))_
- **ECC-MSG-002** EHR Extract — spec-driven export (export_ehr_extracts) — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts (EXTRACT_ENTITY_MANIFEST + EXTRACT_VERSION_SPEC); CNF master13 §I_EHR_EXTRACT.export_ehr_extracts (TBD stub))_
- **ECC-MSG-003** EHR Extract — export of unknown EHR fails — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehrs (ehr_id_does_not_exist precondition); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub))_
- **ECC-MSG-004** EHR Extract — import whole-EHR clone reusing source id — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr; RM common master06 §Copying Case 1 (reuse source EHR identifier); CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-005** EHR Extract — import whole EHR into a caller-fixed id — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr (same patient in another EHR service); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-006** EHR Extract — import into a duplicate target id fails — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr (ehr_create_fail_duplicate_id); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-007** EHR Extract — import extract into an existing EHR — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr_extract; RM common master06 §Copying Case 2 (first receipt clones VERSIONED_OBJECT; re-import is a conflict); CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-008** TDD — import a TDD as a committed COMPOSITION — NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdd; TDD → COMPOSITION over OPT/WebTemplate (openehr_flat::tdd::from_tdd); CNF master13 §I_TDD.import_tdd (TBD stub))_
- **ECC-MSG-009** TDD — import rejects malformed / non-TDD / unknown EHR / unknown template — NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdd (typed envelope rejections); CNF master13 §I_TDD.import_tdd (TBD stub))_
- **ECC-MSG-010** TDD — batch import commits all, fail-fast on error — NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdds; CNF master13 §I_TDD.import_tdds (TBD stub))_
- **ECC-SIG-001** Version signing — digest present — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_
- **ECC-SIG-002** Version signing — digest recomputes — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_
- **ECC-SIG-003** Version signing — all kinds — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_
- **ECC-SIG-004** Version signing — client verbatim — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_
- **ECC-SIG-005** Version signing — pgp verifies — Version signing is an ehrbase-rs extension (VERSION.signature); it is not part of the ITS-REST contract upstream implements. _(cite: profiles master03 §Non-Functional Signing; no openEHR spec governs version signing — ehrbase-rs extension)_

## 8. Edition findings (the SUT's discovered edition profile)

A case satisfied its normative core at a rung below the newest edition — recorded, never a silent pass (`master03-overview.adoc` §API Conformance; the aggregated findings feed the Conformance Statement's supported-versions field).

| ECC id | Format | Satisfied rung | Observations |
|---|---|---|---|
| ECC-COM-001 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-001 | xml | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-002 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-002 | xml | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-032 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-008 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-008 | xml | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-013 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-013 | xml | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-014 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-014 | xml | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-017 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-018 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-018 | xml | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-019 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-020 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-021 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-022 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-022 | xml | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-025 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-026 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-027 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-028 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-029 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-030 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-COM-031 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-CTB-002 | json | release-1.0.3 | release-1.0.3: C.2 invalid COMPOSITION (composer removed): status 400 (release-1.0.3) |
| ECC-CTB-003 | json | release-1.0.3 | release-1.0.3: C.3 empty CONTRIBUTION (no VERSIONs): status 400 (release-1.0.3) |
| ECC-CTB-004 | json | release-1.0.3 | release-1.0.3: C.4 mixed valid+invalid commit rejected: status 400 (release-1.0.3) |
| ECC-CTB-005 | json | release-1.0.3 | release-1.0.3: C.10 COMPOSITION references a non-existent OPT: status 400 (release-1.0.3) |
| ECC-CTB-009 | json | release-1.0.3 | release-1.0.3: C.8 second commit invalid content: status 400 (release-1.0.3) |
| ECC-CTB-015 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-CTB-019 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-CTB-023 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-CTB-029 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-CTB-030 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-CTB-031 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-013 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-015 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-016 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-002 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-004 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-023 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-006 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-008 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-009 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-010 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-011 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-019 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-020 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-021 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-025 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-026 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-029 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-030 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-031 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-032 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-033 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-034 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-035 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form |
| ECC-DIR-036 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: H.2 update a non-existent directory: status 412 (release-1.0.3) |
| ECC-DIR-037 | json | release-1.0.3 | release-1.0.3: ETag emitted in the deprecated bare form; release-1.0.3: I.1 delete a non-existent directory: status 412 (release-1.0.3) |

## 9. Coverage bounds (driven vs schedule data-set rows)

Cases whose driven data-set count is below the governing schedule table's row count — a bound is logged, never silent. Widening the driven set is data, not a new case.

| ECC id | Format | Driven / schedule rows |
|---|---|--:|
| ECC-CTB-004 | json | 1/4 |
| ECC-DIR-016 | json | 5/12 |

## 10. ECC-original cases (no direct schedule backing)

Stub-derived / extension cases — labelled here and **never presented as schedule-conformant**. Their result stands, but the claim is against our own derivation, not an abstract schedule test case.

- **ECC-EHR-012** Create EHR — reject invalid EHR_STATUS data sets — data-set class 2 (master06 §Test Data Sets, invalid EHR_STATUS shapes); no single master06 test case enumerates class 2
- **ECC-EHR-013** Create anonymous (subject-less) EHR — extension: Anonymous EHRs non-functional capability (master03-profiles §Non-Functional); doubles as class 1.b default-EHR_STATUS coverage; no master06 functional test case
- **ECC-TPL-017** Example COMPOSITION round-trips (ADL 1.4 example → commit) — CNF master04/master15 define no example-generation/commit case; the ITS-REST example operation is non-normative. ECC-derived: asserts the operation's own committable-`required` contract end-to-end (upload OPT → GET example → commit 201).
- **ECC-SQR-001** Store stored query — valid — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-valid (master05:54, A.3.a)
- **ECC-SQR-007** Store stored query — invalid — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-invalid (master05:67, A.3.b)
- **ECC-SQR-006** Store stored query — bad formalism — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.valid_query-bad_formalism (master05:80, A.3.c)
- **ECC-SQR-008** Stored query existence check — existing — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.has_query-xxx (master05:37, placeholder id; slug descriptivised)
- **ECC-SQR-002** List stored queries — non empty — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY (named list resource, D2 rebind) + AQL 1.1 — I_DEFINITION_QUERY.list_queries-non_empty (master05:110)
- **ECC-SQR-004** List stored queries — empty — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.list_queries-empty (master05:97)
- **ECC-SQR-005** List stored queries — select items — schedule stub (master05 is TBD); derived from ITS-REST 1.1.0 DEFINITION QUERY + AQL 1.1 — I_DEFINITION_QUERY.list_queries-select_items (master05:123)
- **ECC-QRY-001** Query service smoke test — I_QUERY_SERVICE.smoke_test (master11:48, stub xx flow)
- **ECC-QRY-002** Execute ad-hoc AQL query — empty db — I_QUERY_SERVICE.execute_ad_hoc_query-empty_db (master11:83, A.1.z, stub xx flow)
- **ECC-QRY-003** Execute stored AQL query — empty db — I_QUERY_SERVICE.execute_stored_query-empty_db (master11:61, stub xx flow)
- **ECC-QRY-004** Execute ad-hoc AQL query — loaded db — I_QUERY_SERVICE.execute_ad_hoc_query-loaded_db (master11:96, A.1.a, stub xx flow)
- **ECC-QRY-025** AQL uid projection — c/uid/value returns the version id — schedule stub (master11 is TBD); the loaded-db case asserts only the projected column path — this case asserts the projected CELL equals the committed OBJECT_VERSION_ID (a null cell was a real, otherwise-invisible engine defect)
- **ECC-QRY-005** AQL corpus — invalid queries rejected — schedule stub (master11 is TBD — no invalid-query case); AQL 1.1 negative-rejection evidence
- **ECC-QRY-014** AQL advanced — ORDER BY + LIMIT/OFFSET — schedule stub (master11 is TBD); AQL-advanced ORDER BY + LIMIT/OFFSET, profiles §AQL advanced OPTIONS
- **ECC-QRY-006** AQL corpus — A empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-007** AQL corpus — B empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-008** AQL corpus — C empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-009** AQL corpus — D empty db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-010** AQL corpus — A loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-011** AQL corpus — B loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-012** AQL corpus — C loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-013** AQL corpus — D loaded db — schedule stub (master11 is TBD); golden RESULT_SET diffs derived from AQL 1.1 + the vendored corpus
- **ECC-QRY-015** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-016** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-017** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); TIMEWINDOW golden, spec-supersedes-corpus (adjudications/ecc-own.toml)
- **ECC-QRY-018** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-019** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-020** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-021** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-022** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-023** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-QRY-024** AQL corpus — dialect-adjudicated query rejected — schedule stub (master11 is TBD); LIMIT-before-ORDER-BY golden, corpus-dialect (adjudications/ecc-own.toml)
- **ECC-VAL-119** Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected) — no schedule case — ECC-authored negative guard for the corrected all_types fixture (§3, testdata/fixtures/REGISTER.md); the vendored all_types.composition.json carries a day-bearing DV_DATE at a leaf whose OPT C_DATE pattern disallows the day; a spec-correct validator must 422 it (archie is lenient)
- **ECC-DEM-001** Demographic person create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-021** Demographic create bad body — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-002** Demographic person get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-007** Demographic person get absent — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-006** Demographic person get deleted — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-003** Demographic person get by version — schedule stub (master10 §get_party_at_version TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party_at_version + RM common Versioning
- **ECC-DEM-025** Demographic person get at time — schedule stub (master10 §get_party_at_time TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party_at_time + RM common version_at_time
- **ECC-DEM-004** Demographic person update — schedule stub (master10 §update_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.update_party + RM Demographic IM
- **ECC-DEM-008** Demographic person update bad if match — schedule stub (master10 §update_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.update_party + RM Demographic IM
- **ECC-DEM-005** Demographic person delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-009** Demographic agent create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-010** Demographic agent get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-011** Demographic agent delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-012** Demographic group create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-013** Demographic group get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-014** Demographic group delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-015** Demographic organisation create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-016** Demographic organisation get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-017** Demographic organisation delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-018** Demographic role create — schedule stub (master10 §create_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.create_party + RM Demographic IM
- **ECC-DEM-019** Demographic role get — schedule stub (master10 §get_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.get_party + RM Demographic IM
- **ECC-DEM-020** Demographic role delete — schedule stub (master10 §delete_party TBD); derived from SM I_DEMOGRAPHIC_SERVICE.delete_party + RM Demographic IM
- **ECC-DEM-022** Demographic versioned party get — extension: VERSIONED_PARTY read (RM common Versioning); no master10 SM operation
- **ECC-DEM-023** Demographic versioned party revision history — extension: REVISION_HISTORY read (RM common Versioning); no master10 SM operation
- **ECC-DEM-024** Demographic person tags — extension: item tags — no openEHR spec governs item tags
- **ECC-DEM-026** Demographic relationship create — schedule stub (master10 §create_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP + RM PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-027** Demographic relationship get — schedule stub (master10 §get_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP + RM PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-028** Demographic relationship get at time — schedule stub (master10 §get_party_relationship_at_time TBD); derived from SM I_PARTY_RELATIONSHIP + RM common version_at_time — ehrbase-rs extension wire
- **ECC-DEM-029** Demographic relationship update — schedule stub (master10 §update_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-030** Demographic relationship delete — schedule stub (master10 §delete_party_relationship TBD); derived from SM I_PARTY_RELATIONSHIP — ehrbase-rs extension wire
- **ECC-DEM-031** Demographic relationship get by version — schedule stub (master10 §get_party_relationship_at_version TBD); derived from SM I_PARTY_RELATIONSHIP + RM common OBJECT_VERSION_ID — ehrbase-rs extension wire
- **ECC-ADM-001** Admin EHR delete — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-002** Admin EHR delete absent — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-003** Admin EHR delete idempotent — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-004** Admin EHR delete all — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-005** Admin EHR delete all partial — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-006** Admin EHR delete all (empty selector) — schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml
- **ECC-ADM-007** Admin list contributions — schedule stub (master12 §list_contributions TBD); derived from SM I_ADMIN_SERVICE.list_contributions — native-API-only
- **ECC-ADM-008** Admin contribution count — schedule stub (master12 §contribution_count TBD); derived from SM I_ADMIN_SERVICE.contribution_count — native-API-only
- **ECC-ADM-009** Admin versioned composition count — schedule stub (master12 §versioned_composition_count TBD); derived from SM I_ADMIN_SERVICE.versioned_composition_count — native-API-only
- **ECC-ADM-010** Admin composition version count — schedule stub (master12 §composition_version_count TBD); derived from SM I_ADMIN_SERVICE.composition_version_count — native-API-only
- **ECC-ADM-011** Admin export EHRs (dump/load) — schedule stub (master12 §export_ehrs TBD); derived from SM I_ADMIN_DUMP_LOAD.export_ehrs — native-API-only
- **ECC-ADM-012** Admin archive EHRs — schedule stub (master12 §archive_ehrs TBD); derived from SM I_ADMIN_ARCHIVE.archive_ehrs — native-API-only
- **ECC-ADM-013** Admin physical party delete — schedule stub (master12 §physical_party_delete TBD); derived from SM I_ADMIN_SERVICE.physical_party_delete — demographic-dependent, no ITS-REST binding
- **ECC-ADM-014** Admin archive parties — schedule stub (master12 §archive_parties TBD); derived from SM I_ADMIN_ARCHIVE.archive_parties — demographic-dependent, no ITS-REST binding
- **ECC-MSG-001** EHR Extract — export whole EHR (export_ehrs) — schedule stub (master13 §I_EHR_EXTRACT.export_ehr TBD, listed twice — authoring duplicate); derived from SM I_EHR_EXTRACT_SERVICE.export_ehrs
- **ECC-MSG-002** EHR Extract — spec-driven export (export_ehr_extracts) — schedule stub (master13 §I_EHR_EXTRACT.export_ehr_extracts TBD); derived from SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts
- **ECC-MSG-003** EHR Extract — export of unknown EHR fails — schedule stub (master13 §I_EHR_EXTRACT.export_ehr TBD); derived from SM I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR)
- **ECC-MSG-004** EHR Extract — import whole-EHR clone reusing source id — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying Case 1; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr
- **ECC-MSG-005** EHR Extract — import whole EHR into a caller-fixed id — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr
- **ECC-MSG-006** EHR Extract — import into a duplicate target id fails — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr (duplicate target)
- **ECC-MSG-007** EHR Extract — import extract into an existing EHR — schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying Case 2; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr_extract
- **ECC-MSG-008** TDD — import a TDD as a committed COMPOSITION — schedule stub (master13 §I_TDD.import_tdd TBD); derived from SM I_TDD_SERVICE.import_tdd
- **ECC-MSG-009** TDD — import rejects malformed / non-TDD / unknown EHR / unknown template — schedule stub (master13 §I_TDD.import_tdd TBD); derived from SM I_TDD_SERVICE.import_tdd (typed rejections)
- **ECC-MSG-010** TDD — batch import commits all, fail-fast on error — schedule stub (master13 §I_TDD.import_tdds TBD); derived from SM I_TDD_SERVICE.import_tdds
- **ECC-SEC-001** Unauthenticated request to a protected route is refused (401) — no CNF schedule chapter for authentication (out of band per SM master02); Robot SECURITY_TESTS/I_OAuth2_Keycloak §06 'API endpoints are secured' intent, reproduced over Basic auth
- **ECC-SEC-002** Regular credential on an ADMIN-only route is forbidden (403) — no CNF schedule chapter for authorization (out of band per SM master02); Robot SECURITY_TESTS/I_OAuth2_Keycloak role-distinction intent, reproduced over Basic auth
- **ECC-SIG-001** Version signing — digest present — extension: VERSION.signature is an ehrbase-rs feature (no openEHR spec governs the digest algorithm); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-002** Version signing — digest recomputes — extension: sha256: digest recompute is an ehrbase-rs feature (RFC 8785 canonical form); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-003** Version signing — all kinds — extension: version signing rides every versioned-object write (ehrbase-rs); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-004** Version signing — client verbatim — extension: client-supplied signatures win (ehrbase-rs); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-SIG-005** Version signing — pgp verifies — extension: pgp signing mode is an ehrbase-rs feature (RFC 4880); profiles master03 §Non-Functional Signing STANDARD; RM common master06 §Version signature slot
- **ECC-TS-001** TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-002** TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-003** TERMINOLOGY expand (bundle) — explicit code merged with the expansion — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-004** TERMINOLOGY expand — unknown value set rejected (400) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-005** TERMINOLOGY expand — unknown service_api rejected (400) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the 'openehr' bundle flavour is an ehrbase-rs AQL engine extension
- **ECC-TS-006** TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS) + QUERY master03 §TERMINOLOGY 748-767; the FHIR service_api path realizes the spec mechanism (generic, not an extension)
- **ECC-TS-007** TERMINOLOGY expand (FHIR) — terminology-server timeout is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-TS-008** TERMINOLOGY expand (FHIR) — terminology-server 5xx is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-TS-009** TERMINOLOGY expand (FHIR) — malformed terminology response is a server fault (500) — no CNF schedule chapter for terminology integration; profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS); a fault-injecting tx cannot be wired into an external SUT over the HTTP-only ECC — fault→500 proven off-wire (MSG precedent)
- **ECC-SF-001** FLAT commit then read-back as FLAT, canonical JSON, and STRUCTURED — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-002** STRUCTURED commit then read-back as STRUCTURED, canonical JSON, and FLAT — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-003** Accept q-values select the highest-weight simplified format; every non-204 carries Content-Type — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-004** Deprecated + legacy simplified media types are rejected on Accept (406) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-005** Deprecated + legacy simplified media types are rejected on write Content-Type (415) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-006** FLAT commit without openehr-template-id (and no payload template id) → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-007** FLAT commit with an unknown field identifier → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-008** FLAT commit with |other combined with |code on one coded leaf → 422 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-009** GET a template as a Web Template document (Accept application/openehr.wt+json) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-010** GET a template example in each of the four Accept forms (json, xml, flat, structured) — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-011** GET a template example with an unsupported Accept → 406 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-012** CONTRIBUTION with a FLAT COMPOSITION inner payload: canonical envelope in, simplified read-back — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-013** EHR_STATUS has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-014** DIRECTORY (FOLDER) has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-015** Demographic PARTY has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-SF-016** FLAT ctx/time sets EVENT_CONTEXT.start_time; ctx/setting defaults to openehr::238 — the CNF Platform Conformance Test Schedule defines no simplified-formats chapter; ECC-derived from the STABLE ITS-REST Simplified Formats specification (docs/specs/openehr/ITS-REST/docs/simplified_formats/ + specifications/docs/overview/Resources.md §Simplified Formats)
- **ECC-ADL2-001** Upload a valid ADL2 template → 201 with Location; Prefer selects minimal/representation/identifier bodies — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-002** Upload the same ADL2 HRID twice → the second is a 409 conflict — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-003** Upload an unparseable ADL2 source → 422 carrying syntax rule codes in validationErrors — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-004** Upload a semantically invalid ADL2 template (missing description) → 422 with the AOM2 rule code VARD — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-005** Upload a parent archetype, then a specialised child that validates against the stored parent → 201 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-006** Get an ADL2 template as text/plain source, application/json OperationalTemplateV2, and 406 on xml-only — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-007** Get an unknown ADL2 template_id → 404 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-008** Version get resolves an exact SEMVER and a major prefix (latest match) → 200; an unknown version → 404 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-009** Get a template example in each of the four Accept_LOCATABLE forms → 200; the JSON form is a COMPOSITION rooted at the template's archetype — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-010** Example honours the detail_level enum (required/medium/complete) and rejects a bad type/detail_level with 400 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-011** Example for an unknown template_id → 404; an Accept outside the four LOCATABLE forms → 406 — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-ADL2-012** List ADL2 templates → TemplateMetadata carrying template_id, concept, archetype_id, created_timestamp — the CNF Platform Conformance Test Schedule defines no ADL 2 test case (master04 I_DEFINITION_ADL2 has no upstream cases); ECC-derived from the DEVELOPMENT-status ITS-REST DEFINITION ADL2 operation YAMLs (docs/specs/openehr/ITS-REST/specifications/operations/definition_template_adl2_*.yaml). ADL 2 is OPTIONAL for openEHR conformance (docs/VERSIONS.md ITS-REST DEVELOPMENT row) — OPTIONS-tier, never CORE/STANDARD-gating.
- **ECC-AQT-001** TERMINOLOGY('expand') as a matches operand filters committed compositions by the value set's codes — the CNF Platform Conformance Test Schedule defines no terminology-function test case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS).
- **ECC-AQT-002** A non-expand TERMINOLOGY operation as a matches operand (lookup/map) → 400 — the CNF Platform Conformance Test Schedule defines no terminology-function test case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS).
- **ECC-AQT-003** TERMINOLOGY() in an unsupported position (a SELECT column) → 400 — the CNF Platform Conformance Test Schedule defines no terminology-function test case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS).
- **ECC-AQT-004** A Boolean TERMINOLOGY assertion with an unsupported operation (lookup) → 400 — the CNF Platform Conformance Test Schedule defines no terminology-function test case (master05/master11 name none); ECC-derived from QUERY master03-syntax §TERMINOLOGY + profiles master03 §Functional Querying 'AQL & terminology' (OPTIONS).

## 11. Detailed test report

| ECC id | Capability | Format | Data sets | Rung | Result |
|---|---|---|--:|---|---|
| ECC-EHR-001 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-002 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-003 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-004 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-005 | EhrOperations | json | 16/16 | — | PASS |
| ECC-EHR-006 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-007 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-008 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-009 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-010 | EhrOperations | json | 1/1 | — | PASS |
| ECC-EHR-011 | EhrOperations | json | 1/1 | — | PASS |
| ECC-STA-001 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-002 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-003 | EhrStatus | json | 0/0 | — | **FAIL** |
| ECC-STA-004 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-005 | EhrStatus | json | 0/0 | — | **FAIL** |
| ECC-STA-006 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-007 | EhrStatus | json | 0/0 | — | **FAIL** |
| ECC-STA-008 | EhrStatus | json | 1/1 | — | PASS |
| ECC-STA-009 | EhrStatus | json | 0/0 | — | **FAIL** |
| ECC-STA-010 | EhrStatus | json | 1/1 | — | PASS |
| ECC-EHR-012 | EhrOperations | json | 0/0 | — | **FAIL** |
| ECC-EHR-013 | AnonymousEhrs | json | 1/1 | — | PASS |
| ECC-COM-001 | CompositionOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-COM-001 | CompositionOps | xml | 1/1 | release-1.0.3 | PASS |
| ECC-COM-002 | CompositionOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-COM-002 | CompositionOps | xml | 1/1 | release-1.0.3 | PASS |
| ECC-COM-003 | CompositionOps | json | 0/0 | — | **FAIL** |
| ECC-COM-004 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-005 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-006 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-007 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-032 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-011 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-012 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-008 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-008 | CompositionOps | xml | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-009 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-010 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-013 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-013 | CompositionOps | xml | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-014 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-014 | CompositionOps | xml | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-015 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-016 | CompositionOps | json | 1/1 | — | PASS |
| ECC-COM-017 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-018 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-018 | CompositionOps | xml | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-019 | CompositionOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-COM-020 | CompositionOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-COM-021 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-022 | Versioning | json | 1/1 | release-1.0.3 | PASS |
| ECC-COM-022 | Versioning | xml | 1/1 | release-1.0.3 | PASS |
| ECC-COM-023 | Versioning | json | 1/1 | — | PASS |
| ECC-COM-024 | Versioning | json | 1/1 | — | PASS |
| ECC-COM-025 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-026 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-027 | CompositionOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-COM-028 | CompositionOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-COM-029 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-030 | CompositionOps | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-COM-031 | CompositionOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-001 | ChangeSets | json | 0/0 | — | **FAIL** |
| ECC-CTB-002 | ChangeSets | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-003 | ChangeSets | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-004 | ChangeSets | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-005 | ChangeSets | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-006 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-007 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-008 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-009 | ChangeSets | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-010 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-011 | ChangeSets | json | 15/15 | — | PASS |
| ECC-CTB-012 | ChangeSets | json | 15/15 | — | PASS |
| ECC-CTB-013 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-014 | ChangeSets | json | 0/0 | — | **FAIL** |
| ECC-CTB-015 | ChangeSets | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-016 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-017 | ChangeSets | json | 0/0 | — | **FAIL** |
| ECC-CTB-018 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-019 | ChangeSets | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-020 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-021 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-022 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-023 | ChangeSets | json | 1/1 | release-1.0.3 | PASS |
| ECC-CTB-024 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-025 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-026 | ChangeSets | json | 1/1 | — | PASS |
| ECC-CTB-027 | ChangeSets | json | 0/0 | — | **FAIL** |
| ECC-CTB-028 | ChangeSets | json | 0/0 | — | **FAIL** |
| ECC-CTB-029 | ChangeSets | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-CTB-030 | ChangeSets | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-CTB-031 | ChangeSets | json | 0/0 | release-1.0.3 | **FAIL** |
| ECC-DIR-012 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-013 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-014 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-015 | DirectoryOps | json | 2/2 | release-1.0.3 | PASS |
| ECC-DIR-016 | DirectoryOps | json | 5/5 | release-1.0.3 | PASS |
| ECC-DIR-017 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-018 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-001 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-002 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-003 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-022 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-004 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-023 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-005 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-006 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-007 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-008 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-009 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-010 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-011 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-019 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-020 | DirectoryOps | json | 2/2 | release-1.0.3 | PASS |
| ECC-DIR-021 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-024 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-025 | DirectoryOps | json | 3/3 | release-1.0.3 | PASS |
| ECC-DIR-026 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-027 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-028 | DirectoryOps | json | 1/1 | — | PASS |
| ECC-DIR-029 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-030 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-031 | DirectoryOps | json | 2/2 | release-1.0.3 | PASS |
| ECC-DIR-032 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-033 | Versioning | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-034 | Versioning | json | 2/2 | release-1.0.3 | PASS |
| ECC-DIR-035 | Versioning | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-036 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-DIR-037 | DirectoryOps | json | 1/1 | release-1.0.3 | PASS |
| ECC-TPL-011 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-012 | Adl14OptProvisioning | json | 0/0 | — | **FAIL** |
| ECC-TPL-001 | Adl14ArchetypeProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-002 | Adl14OptProvisioning | json | 0/0 | — | **FAIL** |
| ECC-TPL-004 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-005 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-006 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-009 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-007 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-008 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-010 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-003 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-017 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-TPL-014 | Adl14OptProvisioning | json | 0/0 | — | **FAIL** |
| ECC-TPL-015 | Adl14OptProvisioning | json | 0/0 | — | **FAIL** |
| ECC-TPL-016 | Adl14OptProvisioning | json | 0/0 | — | **FAIL** |
| ECC-TPL-013 | Adl14OptProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-001 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-007 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-006 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-008 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-002 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-004 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-SQR-005 | QueryProvisioning | json | 1/1 | — | PASS |
| ECC-QRY-001 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-002 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-003 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-004 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-025 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-005 | AqlBasic | json | 2/2 | — | PASS |
| ECC-QRY-014 | AqlAdvanced | json | 1/1 | — | PASS |
| ECC-QRY-006 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-007 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-008 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-009 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-010 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-011 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-012 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-013 | AqlBasic | json | 0/0 | — | **FAIL** |
| ECC-QRY-015 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-016 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-017 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-018 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-019 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-020 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-021 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-022 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-023 | AqlBasic | json | 1/1 | — | PASS |
| ECC-QRY-024 | AqlBasic | json | 1/1 | — | PASS |
| ECC-VAL-001 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-002 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-003 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-004 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-005 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-006 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-007 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-008 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-009 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-010 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-011 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-012 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-013 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-014 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-015 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-016 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-017 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-018 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-019 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-020 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-021 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-022 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-023 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-024 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-025 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-026 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-027 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-028 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-029 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-030 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-031 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-032 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-033 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-034 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-035 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-036 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-037 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-038 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-039 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-040 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-041 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-042 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-043 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-044 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-045 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-046 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-047 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-048 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-049 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-050 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-051 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-052 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-053 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-054 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-055 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-056 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-057 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-058 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-059 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-060 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-061 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-062 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-063 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-064 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-065 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-066 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-067 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-068 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-069 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-070 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-071 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-072 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-073 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-074 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-075 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-076 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-077 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-078 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-079 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-080 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-081 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-082 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-083 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-084 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-085 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-086 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-087 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-088 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-089 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-090 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-091 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-092 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-093 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-094 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-095 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-096 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-097 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-098 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-099 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-100 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-101 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-102 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-103 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-104 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-105 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-119 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-106 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-107 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-108 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-109 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-110 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-111 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-112 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-113 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-114 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-115 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-116 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-117 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-VAL-118 | ArchetypeValidation | json | 0/0 | — | **FAIL** |
| ECC-DEM-001 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-021 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-002 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-007 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-006 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-003 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-025 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-004 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-008 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-005 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-009 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-010 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-011 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-012 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-013 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-014 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-015 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-016 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-017 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-018 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-019 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-020 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-022 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-023 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-024 | PartyOperations | json | 0/0 | — | N/A |
| ECC-DEM-026 | PartyRelationshipOperations | json | 0/0 | — | N/A |
| ECC-DEM-027 | PartyRelationshipOperations | json | 0/0 | — | N/A |
| ECC-DEM-028 | PartyRelationshipOperations | json | 0/0 | — | N/A |
| ECC-DEM-029 | PartyRelationshipOperations | json | 0/0 | — | N/A |
| ECC-DEM-030 | PartyRelationshipOperations | json | 0/0 | — | N/A |
| ECC-DEM-031 | PartyRelationshipOperations | json | 0/0 | — | N/A |
| ECC-ADM-001 | AdminPhysicalDeletion | json | 0/0 | — | **FAIL** |
| ECC-ADM-002 | AdminPhysicalDeletion | json | 1/1 | — | PASS |
| ECC-ADM-003 | AdminPhysicalDeletion | json | 0/0 | — | **FAIL** |
| ECC-ADM-004 | AdminPhysicalDeletion | json | 0/0 | — | **FAIL** |
| ECC-ADM-005 | AdminPhysicalDeletion | json | 0/0 | — | **FAIL** |
| ECC-ADM-006 | AdminPhysicalDeletion | json | 0/0 | — | skipped |
| ECC-ADM-007 | AdminActivityReport | json | 0/0 | — | N/A |
| ECC-ADM-008 | AdminActivityReport | json | 0/0 | — | N/A |
| ECC-ADM-009 | AdminActivityReport | json | 0/0 | — | N/A |
| ECC-ADM-010 | AdminActivityReport | json | 0/0 | — | N/A |
| ECC-ADM-011 | AdminEhrDumpLoad | json | 0/0 | — | N/A |
| ECC-ADM-012 | AdminEhrArchive | json | 0/0 | — | N/A |
| ECC-ADM-013 | AdminPhysicalDeletion | json | 0/0 | — | N/A |
| ECC-ADM-014 | AdminDemographicArchive | json | 0/0 | — | N/A |
| ECC-MSG-001 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-002 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-003 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-004 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-005 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-006 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-007 | MessagingEhrExtract | json | 0/0 | — | N/A |
| ECC-MSG-008 | MessagingTds | json | 0/0 | — | N/A |
| ECC-MSG-009 | MessagingTds | json | 0/0 | — | N/A |
| ECC-MSG-010 | MessagingTds | json | 0/0 | — | N/A |
| ECC-SEC-001 | Authentication | json | 1/1 | — | PASS |
| ECC-SEC-002 | Authentication | json | 1/1 | — | PASS |
| ECC-SIG-001 | Signing | json | 0/0 | — | N/A |
| ECC-SIG-001 | Signing | xml | 0/0 | — | N/A |
| ECC-SIG-002 | Signing | json | 0/0 | — | N/A |
| ECC-SIG-003 | Signing | json | 0/0 | — | N/A |
| ECC-SIG-004 | Signing | json | 0/0 | — | N/A |
| ECC-SIG-005 | Signing | json | 0/0 | — | N/A |
| ECC-TS-001 | Terminology | json | 0/0 | — | **FAIL** |
| ECC-TS-002 | Terminology | json | 0/0 | — | **FAIL** |
| ECC-TS-003 | Terminology | json | 0/0 | — | **FAIL** |
| ECC-TS-004 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-005 | Terminology | json | 1/1 | — | PASS |
| ECC-TS-006 | Terminology | json | 0/0 | — | skipped |
| ECC-TS-007 | Terminology | json | 0/0 | — | skipped |
| ECC-TS-008 | Terminology | json | 0/0 | — | skipped |
| ECC-TS-009 | Terminology | json | 0/0 | — | skipped |
| ECC-SF-001 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-002 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-003 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-004 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-005 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-006 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-007 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-008 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-009 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-010 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-011 | SimplifiedFormats | json | 1/1 | — | PASS |
| ECC-SF-012 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-013 | SimplifiedFormats | json | 2/2 | — | PASS |
| ECC-SF-014 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-015 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-SF-016 | SimplifiedFormats | json | 0/0 | — | **FAIL** |
| ECC-ADL2-001 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-002 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-003 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-004 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-005 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-006 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-007 | Adl2Provisioning | json | 1/1 | — | PASS |
| ECC-ADL2-008 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-009 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-010 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-011 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-ADL2-012 | Adl2Provisioning | json | 0/0 | — | **FAIL** |
| ECC-AQT-001 | AqlTerminology | json | 0/0 | — | **FAIL** |
| ECC-AQT-002 | AqlTerminology | json | 2/2 | — | PASS |
| ECC-AQT-003 | AqlTerminology | json | 1/1 | — | PASS |
| ECC-AQT-004 | AqlTerminology | json | 1/1 | — | PASS |

## 12. Terminology server (TS area)

- Server: `http://127.0.0.1:59779`
- Mode: fixture

Recorded FHIR-tx exchange (4 request(s)):

| # | Method | Path | Query |
|--:|---|---|---|
| 1 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface |
| 2 | GET | `/ValueSet/$validate-code` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface&code=B |
| 3 | GET | `/CodeSystem/$lookup` | code=B |
| 4 | GET | `/CodeSystem/$subsumes` | codeA=L&codeB=O |
