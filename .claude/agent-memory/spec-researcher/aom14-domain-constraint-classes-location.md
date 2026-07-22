---
name: aom14-domain-constraint-classes-location
description: Where AOM1.4 C_QUANTITY / C_CODED_TEXT(C_CODE_PHRASE) / CONSTRAINT_REF definitions + their validation semantics (and spec silences) live in the vendored AM spec
metadata:
  type: reference
---

# AOM1.4 domain-type + reference constraint classes

## Naming reality (differs from RM/ADL2 spelling)
- There is NO `C_DV_QUANTITY` and NO `C_CODE_PHRASE` **class file** in AOM1.4.
  The classes are `C_QUANTITY` and `C_CODED_TEXT`.
- Amendment record `AM/docs/ADL1.4/master00-amendment_record.adoc` L108
  (SPEC-226) records the rename `C_CODED_TEXT`→`C_CODE_PHRASE`; ADL1.4 prose
  (master09) already uses `C_CODE_PHRASE`, but the UML export class file is
  still `c_coded_text.adoc`. Its attr is `terminology` (not `terminology_id`);
  ADL1.4 master09 example uses `terminology_id` + `code_string` — naming drift.
- Vendored `C_QUANTITY_ITEM` has ONLY `magnitude` (1..1 Interval<Real>) +
  `units` (0..1 String). **NO `precision` attribute** in the vendored version.

## Class tables (UML export)
- `AM/docs/UML/classes/org.openehr.am.aom14.c_quantity.adoc` — `property`
  (1..1 String), `list` (0..1 List<C_QUANTITY_ITEM>). NO Invariants row.
- `AM/docs/UML/classes/org.openehr.am.aom14.c_quantity_item.adoc` — magnitude,
  units. NO Invariants row.
- `AM/docs/UML/classes/org.openehr.am.aom14.c_coded_text.adoc` — `terminology`
  (1..1), `code_list` (0..1 List<String>) with the load-bearing meaning
  "No list means any code from the terminology is allowed", `reference` (0..1).
  NO Invariants row.
- `AM/docs/UML/classes/org.openehr.am.aom14.constraint_ref.adoc` — `reference`
  (1..1 String, "Reference to a constraint in the archetype local ontology").
  Invariant `Consistency: not any_allowed`.

## THE BIG CAVEAT: C_QUANTITY/C_CODED_TEXT/C_ORDINAL are NON-NORMATIVE
`AM/docs/AOM1.4/masterAppA-domain_extension.adoc` §Overview/§Scientific-Clinical
Computing Types: "The following model is intended only as an example, and does
not try to define any normative semantics of the particular constraint types
shown." The normative semantics live in the openEHR Archetype Profile (oAP)
spec, which is REFERENCED ({openehr_am_oap}) but NOT vendored. => AOM1.4 gives
NO formal valid_value/invariant for C_QUANTITY or C_CODED_TEXT instance
validation. Whether an instance's units must be in the C_QUANTITY_ITEM list,
and how `property` constrains instance units (property→units terminology map),
are SPEC-SILENT in the vendored AOM1.4. `AM/docs/ADL1.4/master09-customising_adl.adoc`
(§Introduction L5-77 C_QTY/property; L79-131 C_CODE_PHRASE) describes intent
only, no validity rules.

## CONSTRAINT_REF resolution + the unbound-ac-code silence
- Prose: `AM/docs/AOM1.4/master04-constraint_model_package.adoc` §Reference
  Objects (C_REFERENCE_OBJECT) L83-91 ("proxy for a set of constraints…
  actual definition is outside the archetype… expressed in the binding of the
  constraint reference (e.g. 'ac0004') to a query… into an external service").
- ADL1.4: `master05-cadl.adoc` §Placeholder Constraints L603-616 (ac-codes,
  external query repository); `master08-adl.adoc` §Constraint_definitions L414
  (ac-code MEANING only), §Term_bindings L430, §Constraint_bindings L481-497
  (ac-code→query URI per terminology).
- Validity: `master08-adl.adoc` §Coded Term Validity L558 — **VATDF** L563
  ("All constraint identifiers ('ac' codes) used in the definition part…must be
  defined in the constraint_definitions part") + **VACDF** L566. These require
  the ac-code be DEFINED (meaning) — they do NOT require a constraint_BINDING
  to exist.
- **SPEC SILENCE (adjudication point):** AOM1.4/ADL1.4 nowhere states what a
  data validator must do when a CONSTRAINT_REF's ac-code has no binding in the
  OPT's constraint_bindings. No "unbound = accept anything", no "unbound =
  reject". `Consistency: not any_allowed` is a structural invariant on the AOM
  node, not a runtime-resolution rule. => genuine `// NOTE:` decision point.
