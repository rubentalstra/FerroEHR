---
name: oas-required-does-not-forbid-empty-arrays
description: Citation pitfall — `Clstr.yaml required: [items]` is SATISFIED by `items: []` (no minItems anywhere in the OAS); only the BMM cardinality grounds an empty-list refusal
metadata:
  type: project
---

Many `*_no_items` case cores cite ITS-REST OAS
`schemas/data_structures/Clstr.yaml §required (items)` as the ground for
refusing a **present-but-empty** list. That citation does not carry the claim:
`required` in JSON Schema constrains KEY PRESENCE only, and the released
`Clstr.yaml` declares no `minItems`. `{"items": []}` is schema-VALID.

The only released ground for refusing an empty `1..*` container is the vendored
**BMM** `cardinality: {lower: 1, upper_unbounded: true}` (LANG
`bmm_container_property.adoc` §Attributes — "Cardinality of this container"),
corroborated where the RM text also states it as an invariant
(`DV_PARAGRAPH.Items_valid: not items.is_empty`; `PARTY.Identities_valid`).
RM class tables never write `1..*`, so the docs table alone can never settle it.

**How to apply:** when a case core's defect is *present-but-empty*, the `required:`
citation is a mis-citation to flag (catalogue bin, citation-only — the expectation
itself stands on the BMM). When the defect is a *missing* member, `required:` is
the right citation. `cnf.composition.lab_result.cluster_no_items` (member deleted)
and `cnf.composition.minimal_event.cluster_no_items` (member emptied) are the two
shapes, and several cores cite both grounds for whichever one they carry.

See [[nonempty-1star-containers]] for the full RM lower==1 set.
