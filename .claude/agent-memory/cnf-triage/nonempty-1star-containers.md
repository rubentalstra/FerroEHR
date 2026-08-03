---
name: nonempty-1star-containers
description: How to adjudicate an empty-list refusal — BMM cardinality.lower==1 IS item count (RM class tables never spell 1..*), plus the confirmed EXTRACT_MANIFEST.entities case
metadata:
  type: project
---

A `1..*` RM container is refused at PARSE since the typed-foundation phase
(`openehr_base::containers::NonEmptyVec`, emitted for every BMM container
property with `cardinality.lower == 1`). When a red row is an empty-list
refusal, adjudicate it like this — do NOT reach for "the reader over-refuses".

**Why:** RM class tables (`RM/docs/UML/classes/*.adoc`) put **existence**
(`0..1` / `1..1`) in the leading column and NEVER write `1..*` (zero
occurrences across the whole RM class-table set), so the docs table alone can
never settle emptiness. The item count lives in the vendored BMM
(`tools/openehr-codegen/vendor/bmm/components/RM/json/openehr_rm_*.bmm.json`,
`P_BMM_CONTAINER_PROPERTY.cardinality`, "Cardinality of this container" —
LANG `org.openehr.lang.bmm.bmm_container_property.adoc` §Attributes), and it
is corroborated wherever the RM text also spells the rule as an invariant:
`PARTY.identities` (lower 1 ↔ `Identities_valid: not identities.is_empty`) and
`DV_PARAGRAPH.items` (lower 1 ↔ `Items_valid: not items.is_empty`). No RM
attribute has lower==1 with text permitting emptiness. The RM 1.2.0 pin, the
RELEASED 1.1.0 BMM and 1.0.4 agree on every one of them.

The full RM 1.2.0 lower==1 set (all emit `NonEmptyVec`, all refuse `[]`):
`CONTRIBUTION.versions`, `REVISION_HISTORY_ITEM.audits`,
`REVISION_HISTORY.items`, `CLUSTER.items`, `DV_PARAGRAPH.items`,
`EXTRACT_MANIFEST.entities`, `ADDRESSED_MESSAGE.addressees`,
`PARTY.identities`, `CONTACT.addresses`. `is_mandatory` and `cardinality.lower`
co-vary perfectly in RM, so neither field alone distinguishes them.

**How to apply:** empty array on one of those attributes ⇒ the instance is
RM-invalid ⇒ the refusal is spec-correct ⇒ the defect is the fixture/case
(catalogue bin), fixed with the valid+invalid twins treatment — the catalogue
already carries the precedent verdict shape at
`corpus/MANIFEST.yaml` `cnf.demographic.person.invalid` (empty `identities`,
`verdict: invalid` + RM citation).

Confirmed instance (2026-08-03 phase-close run, the only red row of 916):
`I_EHR_EXTRACT_SERVICE.export_ehr_extracts-empty_manifest` expected `ok_empty`
with fixture `cnf.messaging.extract_spec.empty_manifest` (`entities: []`,
mis-verdicted `valid`); the SUT's 400 is right. Its sibling `-by_spec` sends
the identical shape with ONE entity and passes — the run's own control.

Live spec tension worth a register/upstream entry, NOT a licence for the
fixture: RM `ehr_extract/master04-common_package.adoc` §Content Specification
line 131-132 says "the request may not specify any entities (**it may rely on
query criteria**)", which the 1..* model makes unrepresentable — but the
carve-out is conditioned on `criteria`, and the fixture carries `criteria: []`,
so it is not the prose's case either way. See [[results-evidence-locations]].
