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
- Base Lexer rules L1415-1424: `V_ISO8601_{DATE,TIME,DATE_TIME,DURATION}_CONSTRAINT_PATTERN`.
  Duration pattern letters = `P[yY]?[mM]?[wW]?[dD]?T[hH]?[mM]?[sS]?` | `P[yY]?[mM]?[wW]?[dD]?`
  (NO fractional-second slot; TIME pattern ends at `[sS?X][sS?X]`, NO ms slot).

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
