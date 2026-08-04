---
name: parse-vs-semantic-400-422-split
description: The released grounding for 400-vs-422 — responses/422.yaml "could be converted to a resource" + the OAS request-body `required:` lists; missing-mandatory = 400, template/invariant failure = 422
metadata:
  type: project
---

The docs text alone does NOT decide the parse/semantic boundary — it only gives
`Requests_and_responses.md` §HTTP status codes: 400 "malformed request syntax,
syntactically invalid content" vs 422 "well-formed but … semantic errors". The
RELEASED OAS fills that silence (no conflict, so it is citable):

- `ITS-REST/specifications/responses/422.yaml`: "returned when the content type
  and syntax is correct, **could be converted to a resource**, but there are
  semantic validation errors, such as the underlying template is not known or is
  not validating the supplied resource".
- `responses/400.yaml`: "could not be parsed or is invalid (… **syntactically
  invalid header, parameter or content**)".
- The request-body schemas declare the members: `schemas/common/Locatable.yaml`
  `required: [name, archetype_node_id]`, `data_structures/Clstr.yaml`
  `required: [items]`, `data_types/DvQuantity.yaml` `required: [magnitude, units]`,
  `ehr/EhrStatus.yaml` `required: [subject, is_queryable, is_modifiable]`,
  `common/Link.yaml` `required: [meaning, type, target]`.

**The line:** a body missing an OAS-required member / carrying a foreign `_type`
in a closed slot / an empty `1..*` list CANNOT be converted ⇒ **400**. A body that
converts and then fails a template constraint, an RM invariant, or a terminology
binding ⇒ **422**. Same as the owner ruling on issue #1727 (2026-08-03), and the
same line AMB-36 already applies to `import_ehr_extract-invalid`.

**Confirmed live (2026-08-04 run):** of 807 authored `rejected` decision-table
rows, every row whose `violates` carries `rm_schema: <attr> is mandatory` got 400
(69) and every pure `constraint(…)` (656), `rm_invariant(…)` (39) and `iso8601`
(40) row got 422 — the SUT's split is exact, so a red row of that shape is a
CATALOGUE re-attribution, not an app defect.

**Exception that is NOT this rule — Simplified Formats.** A FLAT/STRUCTURED body
is not the resource; its schema IS the Web Template, and 422 names "the
underlying template … is not validating the supplied resource". So the master04
§Validation items (mandatory ctx language/territory, datatype vs OPT, cardinality,
terminology bindings) are **422**, and `app/ferroehr-rest/src/api/ehr/composition.rs`
`typed_composition` blanket-400 is an APP defect. Only template-INDEPENDENT FLAT
format violations (`|other` + `|code`, malformed path, unknown `ctx/` key) are
arguably 400 — spec-silent, register material.
