---
name: directory-api-location
description: Where the DIRECTORY (EHR FOLDER) API + FOLDER RM + CNF tests live, and the spec gaps for status codes
metadata:
  type: reference
---

DIRECTORY / EHR FOLDER API spec locations (ITS-REST is OAS-split YAML, NOT prose .adoc):

- Routes: `docs/specs/openehr/ITS-REST/specifications/ehr.openapi.yaml` lines 77-88 — `/ehr/{ehr_id}/directory` (post/put/delete/get) + `/ehr/{ehr_id}/directory/{version_uid}` (get).
- Operations: `.../specifications/operations/directory_{create,update,delete,get_at_time,get_by_version_id}.yaml`.
- Responses: `.../specifications/responses/` — `201_directory`, `200_directory_updated`, `204_version_updated`, `204_deleted`, `204_deleted_at_time`, `200_FOLDER_retrieved`, `412_directory`, `404_directory_*`, `404_unknown_ehr_id`, `400`.
- Params: `.../parameters/query/{version_at_time,path}.yaml`, `.../parameters/path/{ehr_id,version_uid}.yaml`, `.../parameters/header/{If-Match,Prefer,Accept_LOCATABLE,ContentType_LOCATABLE}.yaml`.
- Headers: `.../headers/{ETag_FOLDER,Location_directory,Location_deprecated,ContentType_LOCATABLE}.yaml`.
- Schema: `.../schemas/ehr/Folder.yaml` (FOLDER: items[OBJECT_REF] / folders[FOLDER] / details[ITEM_STRUCTURE], all 0..1; allOf Versionable->Locatable).
- General header/status semantics (Location deprecation, ETag W/, If-Match->412/400, Prefer, version_at_time, openehr-version/audit-details/item-tag): `.../specifications/docs/overview/Requests_and_responses.md`.
- Vendored codegen OAS (same content, bundled): `crates/openehr-its/vendor/rest-oas/ehr-*.openapi.yaml`.

RM grounding:
- FOLDER class: `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.folder.adoc` (no uniqueness invariant at class level; only LOCATABLE.name). Prose: `RM/docs/common/master05-directory_package.adoc` (VERSIONED_FOLDER = VERSIONED_OBJECT<FOLDER>; path = slash-separated name values + uniqueness modifier).
- EHR.directory: `RM/docs/UML/classes/org.openehr.rm.ehr.ehr.adoc` L42 (directory: OBJECT_REF 0..1), invariants Directory_valid (type=VERSIONED_FOLDER) L70, Directory_in_folders L76.

CNF tests: `docs/specs/openehr/CNF/docs/platform_test_schedule/master09-func_tc_ehr_directory.adoc` — SM I_EHR_DIRECTORY ops (has_directory, has_path, create/update/delete_directory, get_directory[_at_time/_at_version], has_directory_version, get_versioned_directory). Robot suites under CNF/tests/platform/robot/I_EHR_DIRECTORY/.

KNOWN SPEC GAPS (flag as "spec-silent / our own design"):
- No REST endpoint for get_versioned_directory (VERSIONED_OBJECT of directory) — CNF L.1-L.3 have no ITS-REST route.
- create on EHR-that-already-has-directory: CNF E.2 requires an error, REST create defines only 400/404 (no 409). Status undefined.
- update/delete when EHR exists but has NO directory: CNF H.2/I.1 require error, REST defines only 404_unknown_ehr_id/412/400 (404 is "unknown ehr_id" only). Status undefined.
- GET at_time with malformed version_at_time: operation omits 400 (only 200/204/404); overview general 400 would apply.
- Last-Modified: overview says SHOULD for VERSION resources, but directory responses list only ETag/Location/Content-Type.
