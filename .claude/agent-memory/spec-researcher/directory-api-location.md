---
name: directory-api-location
description: Where the DIRECTORY (EHR FOLDER) 5-operation API + FOLDER RM + SM I_EHR_DIRECTORY + CNF live, and the enumerated released-text gaps
metadata:
  type: reference
---

DIRECTORY / EHR FOLDER API spec locations (ITS-REST is OAS-split YAML; the EHR-API
docs prose is a STUB — see [[ehr-status-ops-location]]).

- Routes: `ITS-REST/specifications/ehr.openapi.yaml` L77-88 — `/ehr/{ehr_id}/directory`
  (post/put/delete/get) + `/ehr/{ehr_id}/directory/{version_uid}` (get). No `folders`
  route, no `versioned_directory` route, no FOLDER `tags` route (only composition +
  ehr_status have `/tags`).
- Operations: `operations/directory_{create,update,delete,get_at_time,get_by_version_id}.yaml`.
- Responses: `201_directory`, `200_directory_updated`, `204_version_updated` (SHARED with
  ehr_status/composition/demographic updates — it, not the 200, declares
  `Location_version` + the two item-tag headers), `204_deleted`, `204_deleted_at_time`,
  `200_FOLDER_retrieved`, `412_directory`, `404_unknown_ehr_id`, `400`,
  `404_directory_unknown_ehr_id_or_no_version_{at_time,uid}_or_no_path`.
- Params: `parameters/query/{path,version_at_time}.yaml`, `path/{ehr_id,version_uid}.yaml`,
  `header/{If-Match,Prefer,Accept_LOCATABLE,ContentType_LOCATABLE}.yaml`.
- Headers: `headers/{ETag_FOLDER,ETag,Location_directory,Location_version,Location_deprecated,ContentType_LOCATABLE}.yaml`.
- Schema: `schemas/ehr/Folder.yaml` (+ `UMFolder.yaml`); items ->
  `schemas/base_types/UObjectRefOfUidBasedId.yaml` -> `ObjectRef.yaml` (namespace/type/id
  ALL required). `schemas/ehr/Ehr.yaml` carries **NO** directory/folders/compositions/
  contributions properties at all.

**THE `path` QUERY PARAM: its ONLY released definition is `parameters/query/path.yaml`**
("slash-separated values of the name attribute of FOLDERs in the directory",
example `episodes/a/b/c`) + SM `has_path` Meaning (same words). The overview docs
(`docs/overview/*.md`) mention directory/folder only 3 times and NEVER the path param —
grep-verified. RM's own directory paths are a DIFFERENT syntax
(`/folders[hospital episodes]/items[1]`, `RM/docs/common/master05-directory_package.adoc §Paths`).

RM grounding: `RM/docs/common/master05-directory_package.adoc` (VERSIONED_FOLDER =
VERSIONED_OBJECT<FOLDER>) + `RM/docs/UML/classes/org.openehr.rm.common.{folder,versioned_folder}.adoc`
(FOLDER declares NO invariants) + `RM/docs/ehr/master04-ehr_package.adoc §Folders` (L102-131)
and L237 (`is_modifiable` covers Folders) + `master06-change_control_package.adoc §Contributions`
(change_type 249/250/251/523) + `§Logical Deletion`. See [[folder-directory-model-location]].

SM: `SM/docs/UML/classes/i_ehr_directory.adoc` — 9 ops (has_directory, has_path,
create/update/delete_directory, get_directory, get_directory_at_time,
has_directory_version, get_directory_at_version, get_versioned_directory);
`uv_folder.adoc` (UV_FOLDER = UPDATE_VERSION<FOLDER>) + `update_version.adoc` +
`openehr_platform/master03-common_package.adoc §Version Update Semantics`.

CNF (stalled guide): `CNF/docs/platform_test_schedule/master09-func_tc_ehr_directory.adoc`
(cases C.1–L.3). Robot: `CNF/tests/platform/robot/I_EHR_DIRECTORY/**` +
`_resources/keywords/directory_keywords.robot` (the validate-* keywords carry the concrete
status codes; L730 literally says the 409-already-exists case "is not (yet) in the SPEC").
Fixtures: `_resources/test_data_sets/directory/*.json`.

RELEASED-TEXT GAPS for directory (all confirmed first-hand):
create-on-existing-directory status; update/delete when EHR has no directory;
`path` leading-slash + root-name-included + escaping + items-addressable; the
`path`-miss branch on a 204-deleted version; `EHR.directory`/`folders` never on the wire;
committal + item-tag request headers absent from all 5 op files (docs text mandates them);
is_modifiable rejection status; body-`uid`-vs-If-Match mismatch; DELETE 204 declares no
headers at all.
