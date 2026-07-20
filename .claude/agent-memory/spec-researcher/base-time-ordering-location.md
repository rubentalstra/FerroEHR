---
name: base-time-ordering-location
description: Where BASE time-type ordering/comparison + Interval containment reqs live, and the spec silences (no less_than algorithm, no magnitude, no partial/timezone/duration compare rules)
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

Load-bearing SILENCES confirmed (these are our-own-design decision points):
- `Ordered.less_than` is ABSTRACT; NONE of the 4 Iso8601 classes effect/redefine
  it or give a postcondition -> the actual comparison ALGORITHM is spec-silent.
- NO `magnitude` in BASE foundation types at all (grep: only RM data_types/paths).
  Closest numeric anchor = `Iso8601_duration.to_seconds()`.
- Partial-vs-full comparison: silent. Timezone normalization/compare: silent
  (Time_Definitions only gives Min/Max_timezone_hour = 12/14).
- Duration P1M-vs-P30D incomparability: silent; only total via to_seconds
  (Average_days_in_year=365.24, Average_days_in_month=30.42).
- `Interval.Limits_comparable` invariant calls `lower.strictly_comparable_to(upper)`
  but `strictly_comparable_to` is DEFINED NOWHERE (spec defect). `Interval.has`
  post uses `v` while the param is named `e` (variable-name defect).
- valid_iso8601_time text allows seconds "00"-"60" (leap sec) but `valid_second`
  Post is `s < Seconds_in_minute(=60)` -> internal contradiction; invariant uses
  valid_second so 60 is rejected.
- Week dates YYYY-Www EXPLICITLY excluded (master06 overview). 24:00:00 disallowed.
