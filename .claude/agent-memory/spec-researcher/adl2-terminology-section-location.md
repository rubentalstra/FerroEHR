---
name: adl2-terminology-section-location
description: Where the ADL2 `terminology` section syntax (07.13) + its terminology-integration semantics (master08) vs the AOM2 VT* validity codes live, incl. confirmed released-text example defects and the zero-codes fact
metadata:
  type: reference
---

ADL2 `terminology` section navigation:

- **Syntax chapter** — `docs/specs/openehr/AM/docs/ADL2/master07.13-adl_terminology.adoc` (511 lines).
  Sections: Overview (L3-74) / Term_definitions Sub-section (L77-241) / Value_sets Sub-section
  (L243-290) / Term_bindings Sub-section (L293-475) / Deprecated Terminology Section Features
  (L477-511: at-codes-as-identifiers, `terminologies_available` ignored-and-not-serialised,
  ADL1.4 split `constraint_definitions`/`constraint_bindings` merged, ADL1.4 `items` wrapper
  deprecated & removed by tools).
- **This chapter carries ZERO validity codes.** It is syntax + prose only and explicitly delegates
  semantics forward: "The following section, <<Terminology Integration>> describes the semantics"
  (L5) = `ADL2/master08-terminology_integration.adoc` (headings: Requirements / Term Constraint
  Basics / From Constraints to Concrete Codes in Data).
- **The VT* codes live in `AM/docs/AOM2/master07-terminology_package.adoc` §Validity Rules,
  L62-80**: VTVSID (L62), VTVSMD (L65), VTVSUQ (L68), VTSD (L71), VTLC (L74), VTTBK (L77),
  VTCBK (L80). Same file L11-20 = the authoritative attribute list (term_definitions mandatory;
  term_bindings / value_sets / **terminology_extracts** optional) and L26-44 = specialisation
  code-depth ('.' markers; constraint/ac codes are a FLAT code space, not depth-marked).
- **`terminology_extracts` has NO ADL2 syntax**: it appears only in AOM2 master07 L20 and the UML
  class files (`AM/docs/UML/classes/org.openehr.am.aom2.archetype_terminology.adoc`,
  `...operational_template.adoc`, `...p_operational_template.adoc`) — the ADL2 syntax chapter
  never shows it. Template/OPT-only structure.
- **Specialisation-side terminology rules** are NOT in 07.13 — they are `ADL2/master09.09-spec_terminology.adoc`
  (value_sets replace; term_definitions are the sum over the lineage) + `master09.10-spec_bindings.adoc`.

**Confirmed released-text defects in master07.13 examples** (verbatim, useful as `// NOTE:` anchors):
- L28-33 at-coded overview: `value_sets` members `at0000/at0001/at0002` while the `["en"]`
  term_definitions block defines only at0001/at0002/ac1 (at0000 defined only under `["de"]`) —
  self-violates L79 "all translations" + VTLC.
- L262-267 at-coded Value_sets example: members `"at0000","at0001","at0002"` but the sibling
  term_definitions define `at1,at2,at3` — mismatched, and at0000 is the root concept code.
- L363-364: binding path keys `"/data[at002]/…"` — `at002` is a typo for `at0002`.
- L466-469: value-set binding host is `snomed.info` while every other example uses `snomedct.info`.
- L428 prose writes a binding path with an inline rubric `events[at0003|1 minute|]`, which the
  path grammar in `ADL2/master05-paths.adoc` does not admit (`path_segment: attr_name ('[' object_id ']')?`).

**Semantics companion — `ADL2/master08-terminology_integration.adoc` (452 lines).** Three headings only:
Requirements (L5-36), Term Constraint Basics (L38-381), From Constraints to Concrete Codes in Data
(L383-451). Structural facts: **ZERO `[.rule]`/`[.principle]` blocks, ZERO uppercase RFC2119 keywords**
(only 3 lowercase must/should, none a conformance rule) — it is explanatory prose + 4 numbered
"alternatives" (#1 local value_sets L188-220; #2 + at-code bindings L237-287; #3 + ac-code value-set
binding L306-362; #4 external binding only, no value_sets L373-381), all shown twice (at-coded /
id-coded tabs). Load-bearing sentences: L293 (execution environment decides whether the external
terminology "should be used; if so, the local value set definition and at-code bindings can be
ignored" — the ONLY resolution-priority statement, and it is permissive), L404 ("'source form'
archetypes and templates always use internal coding and optionally binding" + substitution "is
specified as an option at the point of operational template generation"), L406 (`[acN]`/`[atN]` →
`[acN@ttttt]`/`[atN@ttttt]`), L385/L391 (two legal data storage choices: internal at-codes or bound
external codes), L389 (large value sets → external code "or else if not available, plain text").
Cross-refs resolve to: `<<cADL_Terminology_Constraints>>` = master04.5 **L510** §Terminology
Constraints (the `@ttttt` form is DEFINED at master04.5 **L689-695** §Operational Binding
Constraints — "`ttttt` is the namespace identifier of a binding"; node-selection is "assumed to be
part of the OPT generator tool"); `<<_node_identifiers_2>>` = the asciidoc auto-id of the SECOND
"Node Identifiers" heading = **master09.05 L5** (the primary statement of the same rule is
master04.3 **L212**).

**master08 released-text defects/tensions** (verbatim anchors): L73 vs L89 define `["at0010"]`
TWICE in the same `["en"]` block ("Specific Substance/Agent" vs "Insect allergen") and at0010 is
also the ELEMENT node id (L53/L417) while being listed as a value-set member (L194/L243/L312) — the
id-coded twin has no such collision (id11 vs at10..at14), so the at-coded variant is the broken one.
L378 alternative #4 uses host `http://snomedct.info/id/...` while every other master08 example uses
`http://snomed.info/id/...` (L248-252, L277-281, L317-322) — **note the inconsistency runs the OTHER
way in master07.13**, so the two chapters disagree on the SNOMED host overall. L36 has a grammar
slip ("managed using in the `term_binding` section").

**master08 silences** (asked repeatedly; no vendored text): what a conformant DATUM must carry — no
mention of `CODE_PHRASE`/`terminology_id`/preferred term/rubric anywhere; no mapping from a binding
namespace key (`snomed_ct`) to the `terminology_id` written into data; `terminology_extracts` is
never mentioned in master08 at all (only AOM2 master07 L20, where preferred-term rubrics live); no
LOINC URI form anywhere in ADL2 (LOINC named in prose only, L13/L17/L385/L402); no precedence
algorithm when local members + at-code bindings + an ac-code value-set binding all coexist and the
OPT did NOT request substitution; no behaviour for `[acN@ns]` naming a namespace with no binding for
acN; no data-commit-time validation rule or code.

**Chapter-level silences** (no vendored text): binding STRENGTH (no FHIR-style required/extensible
notion anywhere in 07.13), terminology-key/namespace lexical rules or registry, `members` ordering
significance, whether `value_sets.id` may differ from its ODIN key, URI scheme validation,
per-language provenance rules, and any at-code/ac-code numeric range rule.
