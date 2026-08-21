---
name: locatable-uid-and-owner-id-landmarks
description: Where the top-level LOCATABLE.uid population/form rule and the VERSIONED_OBJECT.owner_id concrete shapes live — the FIVE RM class-table NOTEs, the master03 truncating example, and the three ITS-REST Versioned*.yaml owner_id examples
metadata:
  type: reference
---

# `LOCATABLE.uid` on top-level objects + `VERSIONED_OBJECT.owner_id` — where the sentences are

## The uid-population NOTE lives in FIVE RM class tables, not one
grep `"strongly recommended that the inherited"` under `RM/docs/UML/classes/`
returns exactly five files — and it is the decisive answer to any
"is the served uid assigned anywhere / only for COMPOSITION?" question:
- `org.openehr.rm.composition.composition.adoc` L11-12
- `org.openehr.rm.ehr.ehr_status.adoc` L10-11
- `org.openehr.rm.ehr.ehr_access.adoc` L11-12
- `org.openehr.rm.common.folder.adoc` L11-12 (scoped to *top-level* FOLDER)
- `org.openehr.rm.demographic.party.adoc` L10-11
Present identically in the RELEASED RM 1.1.0 BMM `documentation` and in the
1.2.0 pin (checked via `tools/openehr-codegen/vendor/bmm/components/RM/json/`).
So a claim that "no released text states this outside COMPOSITION" is FALSE.

## The internal split on the FORM (object_id vs all three parts)
All five class NOTEs use the same two-sentence shape: the RULE says
"copied from the `object_id()` of the `uid` field of the enclosing VERSION",
but the WORKED EXAMPLE says the full `87284370-…::uk.nhs.ehr1::2` "would be
copied" — no truncation step. Only
`RM/docs/common/master03-archetyped_package.adoc` §Unique Node Identification
truncates in its example ("the Guid `87284370-…` would be copied"). Same section
also carries "The `_uid_` attribute will usually be empty in most EHR data".
ITS-REST side: `ITS-REST/specifications/docs/overview/Resources.md`
§Identifier types NOTE (scoped to COMPOSITION, "strongly recommended"/"should")
copies the FULL three-part value; the same section's addressing bullet derives
`versioned_object_uid` + version from `COMPOSITION.uid.value`.
PARTY is the only type with an invariant: `…demographic.party.adoc`
§Invariants `Uid_mandatory: uid /= Void` (also RM demographic
`master02-demographic_package.adoc` §Party Identification types the id
`OBJECT_VERSION_ID`).

## `VERSIONED_OBJECT.owner_id` — THREE concrete examples, not one
No prose anywhere (`RM/docs/`, `SM/docs/`, ITS-REST docs text) assigns
`namespace`/`type` values; only OAS examples do:
- `ITS-REST/specifications/schemas/ehr/VersionedComposition.yaml` → `local` / `EHR`
- `ITS-REST/specifications/schemas/ehr/VersionedEhrStatus.yaml` → `local` / `EHR`
- `ITS-REST/specifications/schemas/demographic/VersionedParty.yaml` → `local` / `SYSTEM`
  (`SYSTEM` is NOT an RM class, contra `BASE/docs/UML/classes/
  org.openehr.base.base_types.object_ref.adoc` `type` = "Name of the class …
  from the relevant reference model" — released-example defect)
The five `schemas/demographic/ItemTagOf*.yaml` examples use the same
`local`/`SYSTEM` pair but for the different `ITEM_TAG.owner_id` attribute.
Reverse edge IS an invariant: `…rm.ehr.ehr.adoc` §Invariants
`Ehr_status_valid` / `Ehr_access_valid` / `Compositions_valid` / `Directory_valid`.
Prose on owner_id: `RM/docs/common/master06-change_control_package.adoc` L27/L35/L205.

Related: [[version-lifecycle-and-identification-location]],
[[versioned-object-read-ops-location]], [[its-rest-wire-contract-location]].
