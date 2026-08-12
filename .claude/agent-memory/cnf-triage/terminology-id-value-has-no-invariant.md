---
name: terminology-id-value-has-no-invariant
description: BASE declares NO Invariants row on TERMINOLOGY_ID; the master05 §Syntaxes production is self-refuted by its own examples and by released QUERY `snomed_ct(3.1)`
metadata:
  type: project
---

Confirmed first-hand 2026-08-12 while triaging the 5-row terminology cluster
(build 11ee41ea7; the enforcement landed in bdf68d3f1 / issue #2258).

**The released model does not declare the invariant the app enforces.**
`BASE/docs/UML/classes/org.openehr.base.base_types.terminology_id.adoc` has NO
`*Invariants*` row at all — its only value statement is the description line
"Lexical form: `name [ '(' version ')' ]`". The sibling
`…version_tree_id.adoc`, whose lexical form is described the same way, DOES
carry an Invariants row — and even there `Value_valid` is only
`not value.is_empty`. The BMM mirrors this exactly
(`openehr_base_1.3.0.bmm.json`: `VERSION_TREE_ID.invariants` = 7 entries,
`TERMINOLOGY_ID` has no `invariants` key; same for ARCHETYPE_ID, TEMPLATE_ID,
GENERIC_ID, OBJECT_VERSION_ID, HIER_OBJECT_ID). Neither released ITS
constrains the value beyond `string` (ITS-JSON
`openehr_rm_1.1.0_all.json` `TERMINOLOGY_ID.properties.value: {type: string}`;
ITS-XML `BaseTypes.xsd` `TERMINOLOGY_ID` = bare `xs:extension base="OBJECT_ID"`).

**The production is refuted by released text, twice.**
`master05-identification_package.adoc:273/277`
`terminology_id = name-str, [ '(', name-str, ')' ]`,
`name-str = letter, { letter | digit | '_' | '-' | '/' | '+' }`.
(a) Its own §Terminology Identifiers examples `ICD9(1999)`, `ICD10AM(3rd_ed)`,
`ICD10AM(4th_ed)` all start the version with a digit (upstream report #2283).
(b) **RELEASED QUERY 1.1.0** `docs/AQL/master03-syntax.adoc:239` publishes
`name/defining_code/terminology_id/value='snomed_ct(3.1)'` as the canonical
expansion of the node-predicate shortcut — `.` is outside `name-str`, so #2283's
"one rule wide" relaxation is one rule too narrow. #2283 does not mention this.

**Split reading that survives the released text:** the NAME part's production is
NOT contradicted by any released example (`SNOMED-CT`, `ICD10`, `ICD9`,
`ICD10AM` all conform), so refusing an interior space is defensible; the VERSION
part's character class is contradicted; and a URI-form id
(`http://…/CodeSystem/x`) is released-SILENT — grep of RM/BASE/QUERY/TERM/
ITS-REST finds no URI spelled as a `terminology_id` value (the `http://snomed.info/sct`
occurrences in QUERY §TERMINOLOGY are function arguments and `code_string`
operands, never a TERMINOLOGY_ID).

**How to apply:** any red row reading "expected `created`, observed
`validation_failed`" on a fixture whose CODE_PHRASE names an external
terminology — check the TERMINOLOGY_ID value against
`crates/openehr-base/src/v1_3/base_types/identification/terminology_id_impl.rs`
`is_valid_terminology_id` first. Fix path is the TEMPLATE
`tools/openehr-codegen/templates/openehr-base/base_types/identification/terminology_id_impl.rs`
+ `openehr-codegen -- emit` (both stamped copies are
`@generated-from-template`). See [[cnf-red-2026-08-12-terminology-id-grammar]].
