---
name: base-time-ordering-location
description: Where BASE time-type ordering/comparison + Interval containment reqs live, the RM-side DV_* ordering definitions that DO exist (magnitude/strictly-comparable), and the real residual silence (magnitude of a reduced-precision value)
metadata:
  type: reference
---

BASE foundation time types + ordering — spec navigation.

Prose chapters: `docs/specs/openehr/BASE/docs/foundation_types/master06-time_types.adoc`
(overview only: §Computational Functions = definite vs nominal add/subtract;
lists constants `Average_days_in_month`/`Average_days_in_year`). The chapter
`include::`s class tables that are actually vendored under a SEPARATE dir:
`docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.<class>.adoc`
(ordered, temporal, any, iso8601_type, iso8601_date, iso8601_time,
iso8601_date_time, iso8601_duration, time_definitions, interval,
point_interval, proper_interval, ordered_numeric, numeric). master05-interval.adoc
= interval overview only; class tables in UML/classes.

BMM (codegen input): time types live under `primitive_types` (NOT
`class_definitions`) in
`tools/openehr-codegen/vendor/bmm/components/BASE/json/openehr_base_1.3.0.bmm.json`.
Ancestry: Iso8601_* -> Iso8601_type -> Temporal -> Ordered -> Any.

## READ THIS BEFORE CLAIMING "no ordering is defined" (RM side, corrected 2026-08-21)
The DECISIVE ordering text is in **RM**, not BASE — a partial/mixed-precision
ordering question must be answered from BOTH sides or the answer is wrong:
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_ordered.adoc` — `less_than`
  ("<") and `is_strictly_comparable_to` are BOTH abstract here ("Redefined in
  descendants" / "Effected in descendants").
- **The concrete descendants DO define a total order**:
  `org.openehr.rm.data_types.dv_date_time.adoc` / `.dv_date.adoc` / `.dv_time.adoc`
  each EFFECT `is_strictly_comparable_to` as **"True, for any two Date/times"**
  (resp. Dates / Times) and REDEFINE `less_than` with
  `Post_result: Result = magnitude < other.magnitude`, over an effected
  `magnitude()` ("seconds since the calendar origin `0001-01-01T00:00:00Z`" for
  DV_DATE_TIME). So the RM explicitly declares mixed-precision values comparable.
- These class files are `include::`d by
  `RM/docs/data_types/master07-date_time_package.adoc` §Class Descriptions (L153+),
  so they ARE part of "the RM date/time package" — a report that cites only that
  chapter's prose and concludes "no total order exists" is wrong.
- **The real residual silence**: nothing anywhere says what `magnitude()` is for a
  REDUCED-PRECISION value (`Value_valid` = `valid_iso8601_date_time(value)`, which
  admits partials), and BASE's Iso8601_* classes declare NO magnitude at all —
  grep `magnitude|less_than|compare` over
  `BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_*.adoc` = EMPTY.
  Frame the gap as "magnitude of a partial is undefined", never as "no order".
- The partial-date/time requirement + design prose is
  `RM/docs/data_types/master07-date_time_package.adoc` §Partial Date/Times (TWO
  sub-sections: L25 under §Requirements, L84 under §Design; ISO 8601 'reduced
  accuracy'). Its only NOTE (L151) is about extended-vs-basic syntax, not ordering.

Load-bearing SILENCES confirmed (these are our-own-design decision points):
- `Ordered.less_than` is ABSTRACT; NONE of the 4 Iso8601 classes effect/redefine
  it or give a postcondition -> the BASE-level comparison ALGORITHM is spec-silent.
- NO `magnitude` in BASE foundation types at all (grep: only RM data_types/paths).
  Closest numeric anchor = `Iso8601_duration.to_seconds()`.
- Partial-vs-full magnitude computation: silent. Timezone normalization/compare:
  silent (Time_Definitions only gives Min/Max_timezone_hour = 12/14).
- Duration P1M-vs-P30D incomparability: silent; only total via to_seconds
  (Average_days_in_year=365.24, Average_days_in_month=30.42).
- `Interval.Limits_comparable` invariant calls `lower.strictly_comparable_to(upper)`
  but `strictly_comparable_to` is DEFINED NOWHERE in BASE (spec defect; the RM's
  `DV_ORDERED.is_strictly_comparable_to` is a different, RM-side feature).
  `Interval.has` post uses `v` while the param is named `e` (variable-name defect).
- valid_iso8601_time text allows seconds "00"-"60" (leap sec) but `valid_second`
  Post is `s < Seconds_in_minute(=60)` -> internal contradiction; invariant uses
  valid_second so 60 is rejected.
- Week dates YYYY-Www EXPLICITLY excluded (master06 overview). 24:00:00 disallowed.

## Interval invariants (the exact four — cite these verbatim, not "the four-invariant set")
`org.openehr.base.foundation_types.interval.adoc` §Invariants:
`Lower_included_valid`, `Upper_included_valid`, `Limits_consistent`,
`Limits_comparable`. `lower`/`upper` are BOTH `0..1`, and the last two are guarded
by `(not upper_unbounded and not lower_unbounded) implies …` — so NO invariant can
bite on an absent, `*_unbounded`-flagged limit. See
[[interval-point-semantics-location]] and [[datatype-constraint-and-cnf-content-location]].
