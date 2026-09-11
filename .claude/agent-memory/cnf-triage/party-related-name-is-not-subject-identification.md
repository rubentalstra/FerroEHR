---
name: party-related-name-is-not-subject-identification
description: APP-bin overreach class — a named PARTY_RELATED is only the subject when relationship codes openehr::0 `self`; SM content_valid == RM validity is the oracle for "may we refuse this at all"
metadata:
  type: project
---

A `PARTY_RELATED` carrying `name` is RM-VALID unless its relationship codes
`self`. The 2026-09-11 red run (917/1104, 178 failed + 9 errored) was ONE app
defect: `privacy::check_party` refused every named `PARTY_RELATED`, and 42 of
the catalogue's composition fixtures carry `"name":"Alexandra Alamo"` with
relationship `openehr::10` (mother).

**Why:** the released chain obliges acceptance, and it is the general oracle
for any "may the app refuse this content?" question:

- SM `docs/UML/classes/i_ehr_composition.adoc` §create_composition —
  `Pre_content_valid: valid_content(a_comp)`, errors exactly
  {ehr_id_does_not_exist, composition_already_exists, definition_unknown,
  content_invalid}, `Post_has_composition`.
- SM `docs/UML/classes/i_validity_checker.adoc` §content_valid — "Return
  `True` if the content structure is a valid instance of the relevant RM
  classes." So content-invalidity IS RM-invalidity, nothing else.
- RM `docs/UML/classes/org.openehr.rm.common.party_related.adoc` declares ONE
  invariant, `Relationship_valid` (the code is in the openEHR
  `subject_relationship` group). Nothing forbids `name`.
- The discriminator the spec itself supplies: §Attributes "If it is the
  patient, coded as **self**" = code `0` of TERM
  `SupportTerminology/codesets/openehr_terminology-vocabularies.adoc`
  §Subject Relationship (the group's other 35 codes are third parties).
- RM common `master04-generic_package.adoc` §Referring to Demographic
  Entities — "the subject of the record is never to be identified in any
  direct way" — is an ASSUMPTION about the SUBJECT, not an invariant, and it
  is exactly the `self` case.

**How to apply:** a local policy (data minimisation, jurisdiction) may only
refuse content the released spec does not oblige a server to accept, or it is
OFF by default and its posture is IXIT-declared (our ixit has no privacy key —
`docs/conformance/party/ferroehr/ixit.json` carries only instances/containers/
spec_profile/system_id/dump_location/signing/terminology/environment/smart).
Fixed at 85b57cc8e (#3252). STILL UNCONDITIONAL and untested by any case:
`identifiers` on both proxy classes — yet PARTY_IDENTIFIED §Description names
"name and **provider number** of an institution" as the class's typical case.
Sibling class: [[interval-and-coded-text-spec-overreach]].
