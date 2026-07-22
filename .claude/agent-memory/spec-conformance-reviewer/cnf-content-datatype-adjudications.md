---
name: cnf-content-datatype-adjudications
description: Verified spec facts for CNF content (master17.x) datatype/interval/temporal adjudications + the OPT-1.4 wire pitfalls that gate realizability
metadata:
  type: feedback
---

Verified spec facts when reviewing CNF content-family adjudications (the
`CONT-*` schedule cases + `ambiguities.yaml`). Re-verify against the cited
files; these were confirmed 2026-07-22.

**BASE Interval invariants (AMB-43 grounding)** —
`BASE/docs/UML/classes/org.openehr.base.foundation_types.interval.adoc`:
lower/upper are 0..1 (optional). EXACTLY FOUR invariants: Lower_included_valid,
Upper_included_valid, Limits_consistent (`(not upper_unbounded and not
lower_unbounded) implies lower <= upper`), Limits_comparable. **NONE requires a
bound VALUE present when `*_unbounded=false`** → a bounded-flag interval with
absent limits violates no RM invariant (accept). But inverted bounded limits
(lower>upper, both present) DO violate Limits_consistent (reject). `Interval.has`
post (line 52) is a TOTAL-ORDER comparison on the full value, not a component.

**Ordering for the interval inner types**: DV_DURATION less_than = magnitude()
comparison (seconds; `dv_duration.adoc` line 62,81); DV_ORDINAL/DV_SCALE order
by `value` and both have symbol 1..1 + value 1..1 mandatory
(`dv_ordinal.adoc`/`dv_scale.adoc`). A fully-null bound = absent Interval
attribute (accept, AMB-43); a partially-null bound = present-but-malformed inner
DV (reject on the inner type's 1..1).

**AMB-42 (temporal serialization gap)** — AOM1.4 UML defines
`millisecond_validity` on C_TIME/C_DATE_TIME and `seconds_allowed`/
`fractional_seconds_allowed` on C_DURATION (`c_time.adoc`,`c_duration.adoc`),
but ITS-XML 1.0.2 `Archetype.xsd` (ALL/) C_TIME/C_DATE_TIME sequence is ONLY
(pattern, timezone_validity, range, assumed_value) and C_DURATION is (pattern,
range, assumed_value). So millisecond + the seconds-vs-fractional split are
UNSERIALIZABLE (rows resting solely on them → per-row N/A). `timezone_validity`
IS an XSD element (VALIDITY_KIND 1001 mandatory/1002 optional/1003 disallowed,
Archetype.xsd:219) → stays TESTED. minute/second constraints ARE expressible via
the pattern (HH:??:xx) → stay tested.

**OPT-1.4 wire pitfall (recurring):** `C_PRIMITIVE_OBJECT` (Archetype.xsd:112)
extends C_DEFINED_OBJECT and its sequence is ONLY `<item>` (C_PRIMITIVE). It
CANNOT carry `<attributes>` — only `C_COMPLEX_OBJECT` (Archetype.xsd:75) has
`attributes`. When a corpus OPT constrains a DV_SCALE/DV_ORDINAL, the `symbol`
attribute is a SIBLING of `value` under the DV_x C_COMPLEX_OBJECT, never nested
inside the value's C_PRIMITIVE_OBJECT. A misplaced attributes block there is
schema-invalid and (strict SUT) can fail the whole template upload, silently
turning all rows on that template inconclusive.

**Runner realizability gate** (`exec/content_synth.rs unrealizable_row` +
`exec/mod.rs Provisioned::RowNotApplicable`): only excuses REJECTED rows whose
EVERY `violates` entry is inexpressible (all() combinator) — serializable axes
(ranges, timezone, pattern minute/second) keep a row driven. N/A cannot mask a
Failed row: `party.rs OutcomeRecord::from` gives Failed precedence; all-N/A
cases roll up to `ExcusedByRegister` (visible, cited), never a silent pass.
