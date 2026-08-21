---
name: interval-point-semantics-location
description: Where BASE Interval/Point_interval/Proper_interval semantics + "a single value IS a point interval" live (BASE UML classes, EL master03, AOM2 master04.2, ADL2 master04.5), plus the confirmed released-text defects
metadata:
  type: reference
---

Point-vs-proper interval semantics — spec navigation (answers "is |n..n| a point?").

Owning files:
- `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.interval.adoc`
  — `Interval<T>` (abstract): lower/upper are **0..1**; 4 flags 1..1; 4 invariants
  (Lower_included_valid, Upper_included_valid, Limits_consistent, Limits_comparable).
- `...point_interval.adoc` — `Point_interval<T>`: redefines the 4 flags with
  `{default = false/false/true/true}`, invariant `Inv_point: lower = upper`.
- `...proper_interval.adoc` — `Proper_interval<T>`: invariant `Inv_not_point: lower /= upper`.
- `...multiplicity_interval.adoc` / `...cardinality.adoc` — derived types.
- `docs/specs/openehr/BASE/docs/foundation_types/master05-interval.adoc` §Overview
  — the include host; says either subtype "may be attached" at runtime.
- `docs/specs/openehr/LANG/docs/EL/master03-basics.adoc` (~L93 table) — the crispest
  definition: Point_interval = "closed intervals whose boundaries are the same";
  Proper_interval = "intervals whose boundaries are different".
- `docs/specs/openehr/AM/docs/AOM2/master04.2-constraint_model-semantics.adoc`
  §Primitive Types (C_PRIMITIVE_OBJECT descendants), table `[[primitive-types]]`
  (~L124) — C_ORDERED row: "A single value (which is a point interval), a list of
  values (list of point intervals), a list of intervals, which may be mixed proper
  and point intervals." <- THE normative single-value==point-interval statement.
- `docs/specs/openehr/AM/docs/ADL2/master04.5-cadl_primitive_types.adoc`
  §Constraints on Ordered Types (L271) — "degenerate interval of the form `{N..N}`,
  i.e. effectively a single value"; §Interval of Integer (L308) `{|1000|}` = "point
  interval of 1000 (=fixed value)".
- `docs/specs/openehr/LANG/docs/odin/master07-leaf_data.adoc` §Intervals of Ordered
  Primitive Types (L130-145) — the 10 interval forms.
- AOM2 `master04.5` §Conformance semantics: C_ORDERED (L596) — conformance uses
  `Interval.contains`/`is_equal`, never a point predicate.

Load-bearing facts / defects confirmed in released text:
- **There is NO `is_point` predicate anywhere in the vendored tree** (grep: zero hits).
  Point-ness is expressed by TYPE (Point_interval), not by a function.
- **No invariant ties `lower_unbounded` to `lower` being Void.** Interval only has
  `lower_unbounded implies not lower_included` (+ upper twin). So the unbounded flags
  are NOT redundant with a set bound — and `has()`'s postcondition makes an unbounded
  side short-circuit to true regardless of the stored bound. Checking the flags is
  load-bearing, never redundant.
- Point_interval's flag redefinitions are `{default = ...}`, i.e. **defaults, not fixed
  values** — the model does not forbid an unbounded "point interval".
- **DEFECT: `Multiplicity_interval` inherits `Proper_interval`** (thus `Inv_not_point:
  lower /= upper`) yet defines `is_mandatory()` = `1..1` and `is_prohibited()` = `0..0`,
  both of which require lower == upper. Direct internal contradiction in released BASE.
- **DEFECT/gap: ODIN's interval syntax table has no bare `|N|` point form** (only
  `|N..M|`, `>N..M`, `N..<M`, `>N..<M`, `<N`, `>N`, `>=N`, `<=N`, `N +/-M`, `N±M`) —
  yet ADL2 master04.5 uses `{|1000|}` and `{|5.5|}` as "point interval". Parsers reading
  ODIN can only produce a bounds-equal interval for `|5..5|`.
- `Interval.has` Post uses `v` while the parameter is named `e`; `strictly_comparable_to`
  (Limits_comparable) is defined nowhere (see [[base-time-ordering-location]]).

RM side — the class that actually governs a committed interval:
- `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_interval.adoc`
  — `DV_INTERVAL<T>` inherits `DATA_VALUE` **+ BASE `Interval` directly** (never
  Point_/Proper_interval) and declares its OWN invariant
  `Limits_consistent: (not upper_unbounded and not lower_unbounded) implies
  (lower.is_strictly_comparable_to(upper) and lower <= upper)` — stronger than
  BASE's same-named one and spelled `is_strictly_comparable_to` (BASE spells it
  `strictly_comparable_to`). So the invariant set governing a wire DV_INTERVAL is
  BASE's four PLUS this one: any answer about "which invariant bites on an absent
  limit" must read this file, not only BASE `interval.adoc`.
- Consequence to remember: for `{lower_unbounded:false, upper_unbounded:false}` with
  absent limits, BOTH `Limits_consistent` forms fire their antecedent and then call
  a feature on a Void limit; no released text states Void-evaluation semantics (the
  RM guards such calls elsewhere, e.g. `history.adoc` `Events_valid` uses
  `/= Void and then`). The CNF schedule's own cells call it an opinion
  ("IMO should fail", forum `is-dv-interval-missing-invariants/2210`).
- `RM/docs/data_types/master00-amendment_record.adoc` L396 ("`DV_INTERVAL` now
  inherits from `INTERVAL`") + L503 (lower_unbounded/upper_unbounded were once
  FUNCTIONS) = the lineage; no revision ever added a bound-presence invariant.
