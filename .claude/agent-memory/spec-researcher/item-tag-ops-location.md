---
name: item-tag-ops-location
description: Where the ITEM_TAG spec text lives — RM common master07-tags + item_tag class, the 7 EHR-side + 16 demographic-side ITS-REST tag ops/schemas/params, the header wrappers; and the total SM/CNF/ITS-XML silence
metadata:
  type: reference
---

# ITEM_TAG — where the spec text lives

## RM (the class authority)
- `RM/docs/common/master07-tags.adoc` — the `common.tags` package chapter
  (Overview + Semantics + include of the class). Package is a SIBLING of
  `change_control` (master06), NOT inside it.
- `RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc` — the ONLY class
  table: key/value/target/target_path/owner_id + `Inv_key_valid`,
  `Inv_value_valid`.
- Added by SPECRM-87 (`RM/docs/common/master00-amendment_record.adoc`,
  issue row 2.1.5, 17 Nov 2022). Also in
  `RM/computable/BMM/openehr_rm_1.2.0.bmm.json` (~L1982).
- `is_justified` (used in Inv_key_valid) is **defined nowhere** — the BASE
  String class (`BASE/docs/UML/classes/org.openehr.base.foundation_types.string.adoc`)
  lists only is_empty/is_integer/as_integer/append/less_than/contains.
  The intent lives only in the attribute Meaning column.

## ITS-REST (the wire)
- Routes in `ITS-REST/specifications/ehr.openapi.yaml` L95–113 (5 EHR-side
  paths / 7 ops); tag group `ITEM_TAG` L148.
- Ops: `operations/{ehr,composition,ehr_status}_tags_{get,update,delete}.yaml`
  (+ 15 demographic ones: person/role/group/organisation/agent).
- Schemas: `schemas/common/ItemTag.yaml` (base, `additionalProperties: false`),
  `schemas/common/UpdateItemTag.yaml` (the PUT item — key/value/target_path
  ONLY), `schemas/ehr/ItemTagOf{Composition,EhrStatus}.yaml`
  (allOf ItemTag + example only).
- Responses: `responses/200_{COMPOSITION,EHR_STATUS}_ItemTagList_{retrieved,
  updated}.yaml`, `204_updated.yaml`, `204_deleted.yaml`,
  `404_unknown_ehr_id[_or_uid_based_id[_or_key]].yaml`.
- Params: `parameters/path/key.yaml`, `parameters/query/tag_{key,value,
  target_path}.yaml`, `parameters/path/uid_based_id.yaml`.
- Header wrappers: prose in
  `ITS-REST/specifications/docs/overview/Requests_and_responses.md`
  §"openehr-item-tag and openehr-version-item-tag" (L98–126);
  request param files `parameters/header/openehr-*item-tag.yaml`,
  response header files `headers/openehr-*item-tag.yaml`.
- There is NO tag prose in `docs/ehr/Description.md` (it is a stub — see
  [[ehr-status-ops-location]]).

## The DEMOGRAPHIC tag half (group 13, `demographic.openapi.yaml` L93–135)
- **16** path-method pairs (NOT 13): `GET /demographic/tags` (1) + per party
  subtype `person|agent|group|organisation|role` × {GET,PUT `/…/{uid_based_id}/
  tags`, DELETE `/…/tags/{key}`} = 15.
- Ops `{person,agent,group,organisation,role}_tags_{get,update,delete}.yaml`
  are **byte-identical mod type-name across all 5 subtypes** (verified by
  sed-normalized diff); so are the 10 `200_<T>_ItemTagList_{retrieved,updated}
  .yaml` responses and the 5 `schemas/demographic/ItemTagOf<T>.yaml`.
- Structural mirror of the EHR twin **minus `ehr_id`**: `person_tags_get` ≡
  `composition_tags_get`, `_update` ≡ `composition_tags_update`, `_delete` ≡
  `composition_tags_delete`, `demographic_tags_get` ≡ `ehr_tags_get`.
  404 files swap to `404_unknown_uid_based_id[_or_key].yaml`.
- Deltas worth remembering: `demographic_tags_get` has **no scope param at
  all** (server-wide list) yet its description still says "within given EHR",
  and it declares the PERSON-specific list schema; `ItemTagOf<Party>` examples
  use `owner_id.type: SYSTEM` (EHR side uses `EHR`) and `target_path: ""`;
  `person_tags_update` says "VERSIONED_OBJECT.uid.value" where get/delete say
  "VERSIONED_PARTY.uid.value"; all 5 `<t>_update.yaml` declare ONLY
  `openehr-version-item-tag` (missing `openehr-item-tag`) though their prose
  names both — the composition/ehr_status updates declare both.
- **RM has NO demographic-side tag anchor**: `EHR.tags` (`RM/docs/UML/classes/
  org.openehr.rm.ehr.ehr.adoc` L53-55, prose-only "Tag `_target_` values can
  only be within the same EHR", NO `Tags_valid` invariant) is the only
  containment; grep of `RM/docs/demographic/` + all `org.openehr.rm.
  demographic.*.adoc` = zero tag hits. RM `ITEM_TAG.target` is a plain
  `UID_BASED_ID` (OAS wraps it in `UObjectRefOfUidBasedId` — RM wins),
  target types unrestricted, `owner_id` "such as EHR" (open list).

## Total silences (verified by grep across the whole vendored tree)
- **SM**: zero ITEM_TAG / tag-operation anchor anywhere in `SM/docs/`
  (only unrelated "tagged String values" / "language tag" hits). No
  I_* interface owns these routes.
- **CNF**: zero coverage — no schedule row, no Robot suite (the one grep hit
  is an unrelated archetype element named `tags` in a FLAT fixture).
- **ITS-XML**: no ITEM_TAG type at all, although the tag ops declare
  `Accept: application/xml`.
- The `ehr.openapi.yaml` ITEM_TAG tag description links RM
  **/development/** (every other class links /latest/) — ITEM_TAG is in no
  released RM.

Related: [[its-rest-wire-contract-location]],
[[composition-crud-ops-location]], [[ehr-status-ops-location]],
[[directory-api-location]].
