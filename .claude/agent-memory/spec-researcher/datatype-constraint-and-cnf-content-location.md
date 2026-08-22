---
name: datatype-constraint-and-cnf-content-location
description: Where ADL1.4 date/time/duration constraint patterns + AOM1.4 C_TIME/C_DATE_TIME/C_DURATION/C_ORDINAL object-model fields + RM DV_PROPORTION/DV_URI invariants + BASE Interval invariants + the CNF master17.x content data-type test tables live; the syntax-vs-object-model divergences
metadata:
  type: reference
---

# Data-type constraint definitions + CNF content-chapter test tables

## ADL 1.4 date/time/duration CONSTRAINT PATTERN syntax
`AM/docs/ADL1.4/master05-cadl.adoc`:
- §"Date, Time and Date/Time" → "Patterns" L847-894 (pattern table +
  `yyyy-mm-ddThh:mm:ss` abstract form, `?`=optional `X`=not-allowed);
  timezone table L898-906; L854 "no way to state that timezone information be
  _prohibited_"; L910 absence=optional.
- §"Duration Constraints" → "Patterns" L934-953.
- **CORRECTED 2026-08-22 — L1415-1424 is NOT the "Base Lexer"**: those
  `V_ISO8601_{DATE,TIME,DATE_TIME,DURATION}_CONSTRAINT_PATTERN` entries live in
  master05-cadl.adoc's OWN `== Syntax Specification` (L1024) -> `=== Grammar`
  (L1028) / `=== Symbols` (L1283), which L1026 states is the **legacy lex/yacc
  (Eiffel ADL Workbench) specification**, a different formalism. The chapter has
  NO section titled "Base Lexer" and no `[[_base_lexer]]` anchor (grep = 0).
  The `<<_base_lexer,...>>` xrefs at L856/L938 resolve to
  `ADL1.4/masterAppC-syntax_spec.adoc` **L60 `== Base Lexer`** ->
  `include::{grammar_dir}/adl/base_lexer.g4[]` (L66). That grammar DEFINES the
  unprefixed `DATE_/TIME_/DATE_TIME_CONSTRAINT_PATTERN` (base_lexer.g4 L35-37)
  and `DURATION_CONSTRAINT_PATTERN` (L47) — so the prose citations are NOT
  dangling. See [[adl2-cadl-primitive-types-location]] + [[lexical-layer-per-language-location]].
  Pattern letters (from the .g4, matching the legacy listing): duration
  `P[yY]?[mM]?[Ww]?[dD]?('T'[hH]?[mM]?[sS]?)?` (NO fractional-second slot);
  TIME ends at SECOND_PATTERN + optional TZ_PATTERN, NO ms slot.

## CRUCIAL syntax-vs-object-model divergence (adjudication point)
The ADL1.4 *pattern syntax* cannot express milliseconds, timezone-prohibited,
or fractional-vs-integer-seconds. The AOM1.4 *object model* CAN, via extra
fields in the UML class tables (OPT 1.4 carries the object model):
- `AM/docs/UML/classes/org.openehr.am.aom14.c_time.adoc` — minute/second/
  **millisecond**/**timezone** `_validity` (each 0..1 VALIDITY_KIND) + range +
  assumed_value; cascade invariants.
- `...c_date_time.adoc` — month/day/hour/minute/second/**millisecond**/
  **timezone** `_validity` + range + assumed_value; full cascade invariants.
- `...c_duration.adoc` — years/months/weeks/days/hours/minutes/**seconds**/
  **fractional_seconds** `_allowed` (all 0..1 Boolean) + range + assumed_value.
  NO invariants row. `seconds_allowed` and `fractional_seconds_allowed` are
  SEPARATE booleans -> integer vs fractional seconds independently constrainable.
- `...c_date.adoc` — day/month `_validity` + range + assumed_value.
- VALIDITY_KIND enum = mandatory/optional/disallowed (+ deprecated dup):
  `BASE/docs/UML/classes/org.openehr.base.base_types.validity_kind.adoc`.
- AOM/OPT-1.4 XSD is NOT vendored in docs/specs (only RM ITS-XML + terminology
  XSDs exist); the object-model field list = the AOM1.4 UML class tables.
- The OPT-1.4 XSD *is* vendored under `crates/openehr-its/schemas/xml/`:
  `its-xml-1.0.2-nsv1/ALL/Archetype.xsd` (C_DATE L275, C_DATE_TIME L293,
  C_TIME L314, C_DURATION L332) and `its-xml-2.0.0-nsv2/AM/Release-1.4/
  Archetype.xsd` (C_TIME L316, C_DURATION L333). In BOTH lineages C_TIME and
  C_DATE_TIME carry only `pattern | timezone_validity | range | assumed_value`
  and C_DURATION only `pattern | range | assumed_value`; grep for
  `millisecond|seconds_allowed|fractional_seconds` across either whole bundle
  returns ZERO. So `millisecond_validity` / `seconds_allowed` /
  `fractional_seconds_allowed` are modelled in AOM 1.4 (c_time L27,
  c_date_time L39, c_duration L50/L54) and unserializable in every published
  ITS-XML bundle; `timezone_validity` IS serializable.

## C_ORDINAL / DV_SCALE (Q7 pattern)
`...aom14.c_ordinal.adoc` (=`C_DV_ORDINAL` in ADL2 naming) has only
`list: List<ORDINAL>`; `...aom14.ordinal.adoc` ORDINAL = symbol(CODE_PHRASE)+
value(**Integer**). NO C_SCALE/C_DV_SCALE anywhere in AM (grep empty) =>
DV_SCALE unconstrainable in OPT1.4 beyond generic C_COMPLEX_OBJECT/C_REAL.
Domain-extension appendix `AM/docs/AOM1.4/masterAppA-domain_extension.adoc` L10
= NON-NORMATIVE ("intended only as an example"); oAP not vendored.

## RM data-type invariants
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_proportion.adoc` — 7
  invariants incl. `Fraction_validity: (type=pk_fraction or pk_integer_fraction)
  implies is_integral`; `is_integral()` = precision==0. So type 3/4 => precision
  MUST be 0; precision=1 with type 3/4 rejected under Fraction_validity.
  PROPORTION_KIND: `...proportion_kind.adoc` (ratio0/unitary1/percent2/
  fraction3/integer_fraction4).
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_uri.adoc` — ONLY invariant
  `Value_valid: not value.is_empty`. RFC-3986 conformance is PROSE only
  (Description), NO formal invariant. Prose `RM/docs/data_types/master10-uri_package.adoc`.

## BASE Interval invariants
`BASE/docs/UML/classes/org.openehr.base.foundation_types.interval.adoc` — 4
invariants: Lower/Upper_included_valid, Limits_consistent (lower<=upper when
both bounded), Limits_comparable (strictly_comparable_to — DEFINED NOWHERE,
defect). `proper_interval.adoc` adds Inv_not_point: lower/=upper. SILENCE:
NO invariant requires a bound value present when *_unbounded=false (lower/upper
are 0..1, no coupling invariant) — spec gap in BASE 1.3.0.
**The gap was NEVER filled in any generation**: the same 4-invariant set is
byte-identical in BASE BMM 1.0.4 / 1.1.0 / 1.2.0 / 1.3.0, and RM `DV_INTERVAL`
carries only `Limits_consistent` in RM 1.0.2→1.2.0 (checked in
`tools/openehr-codegen/vendor/bmm/components/{BASE,RM}/json/`). So "an earlier
revision carried a bound-required invariant" is FALSE — never claim it.
The CNF side says so itself: `master17.3` rows for
`{NULL,NULL,false,false,true,true}` (L480 / L651 / L728 / L783 / L915 / L957)
verdict `rejected` with the comment "IMO should fail, see
…/is-dv-interval-missing-invariants/2210" or "RM/Schema: value is mandatory for
lower and upper" — an opinion/schema appeal, never an invariant citation.

## ISO8601 time exclusions
`BASE/docs/foundation_types/master06-time_types.adoc` L21-35 — L25: fractional
minutes/hours NOT supported, "only fractional seconds are supported"; no
expanded years, no Www week dates, 24:00:00 disallowed.

## Existence (Q8)
`...aom14.c_attribute.adoc` existence:Interval<Integer>[1..1], invariant
Existence_set lower>=0 upper<=1; `valid_value` conformance-test statement at
`AM/docs/AOM1.4/master04-constraint_model_package.adoc` L62; existence meaning
L33. AOM states meaning+conformance test; NO HTTP status (that's ITS-REST/CNF).

## CNF content-chapter data-type test tables (adjudicate expectations!)
`CNF/docs/platform_test_schedule/master17.x-content_tc_data_types-*.adoc`:
17.1 basic, 17.2 text, 17.3 quantity (DV_PROPORTION table L211-220 confirms
type3/4=>precision0), 17.4 date_time (C_TIME validity kinds incl millisecond+
timezone L133; C_DURATION seconds/fractional_seconds reject tables L54-71,
L117-127), 17.5 time_specification, 17.6 encapsulated, 17.7 uri
(CONT-DV_URI-validate_open L9-18: **"xyz" => REJECTED** "doesn't comply with
RFC3986" — STRICTER than the RM invariant AND than RFC3986, which permits bare
relative refs). Composition/entry validation: master15/master16.
LOAD-BEARING TENSION: CNF 17.7 rejects bare relative URIs though RM only
enforces not-empty and RFC3986 allows relative references.
More confirmed schedule defects, with line anchors:
- 17.4 L147 NOTE "our test data sets all include the `T` time marker" + every
  DV_TIME literal (L189-280) and the 17.3 interval-of-time tables (L743-765)
  print `T10:30:47`-style values; BASE `iso8601_time.adoc` §Description and
  `time_definitions.adoc` `valid_iso8601_time` (L157-163) admit NO leading `T`
  in either the extended or the compact form.
- 17.4 `CONT-DV_DATE_TIME-validate_range` T-precision block (L975-1000) compares
  values anchored `2021-10-24T…` against ranges anchored `1900-03-13T…`: the 4
  bounded rows expect `accepted` (impossible under `Interval.has`) and the 4
  `>=1900-03-13T11` rows expect `rejected` (also impossible — the value IS >=).
  The other 12 rows in the block ARE derivable; re-anchoring the values to
  1900-03-13 makes all 24 derivable.
- `master16-content_tc_entry.adoc` HISTORY tables: exactly TWO rows verdict
  "no events | absent | accepted" (L127 in `CONT-HIST-events_card_any-summary_ex_opt`,
  L175 in `CONT-HIST-events_card_opt-summary_ex_opt`) against RM
  `…data_structures.history.adoc` L49 `Events_valid`; all other such rows reject.
- 17.2 L103 `CONT-DV_CODED_TEXT-validate_ext_term` row
  `ABC|local|ac0001|[SNOMED_CT]` verdicts `rejected` "constraint_binding:
  terminology_id not found" — no AOM 1.4 ground exists for that rejection
  (`AM/docs/AOM1.4/master04-constraint_model_package.adoc` L83/L85 puts the
  constraint definition outside the archetype; ADL1.4 `master08-adl.adoc` L416,
  `master05-cadl.adoc` §Placeholder Constraints L603).
- DV_CODED_TEXT `value == rubric` is PROSE ONLY: `…dv_coded_text.adoc` L9 +
  L24, `…dv_text.adoc` L28, plus `RM/docs/data_types/master05-text_package.adoc`
  L52 + L110; the sole invariant on value is `dv_text.adoc` `Valid_value:
  not value.is_empty`, and DV_CODED_TEXT's class table has NO Invariants row.
