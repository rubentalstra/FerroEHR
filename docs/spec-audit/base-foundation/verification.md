# A1 Spec Audit — Verify + Fix — chapter `base-foundation`

- **Chapter:** BASE 1.3.0 foundation_types (primitives, structures, interval,
  time types, terminology, functional)
- **Date:** 2026-07-11
- **Scope:** all 57 requirements `base-foundation-R1 … R57`
- **Result (defer-nothing pass):** 1 defect fixed (timezone bounds were
  symmetric 0–14 — the spec's asymmetric +14/−12 now enforced, with the
  ±00:00 corpus adjudication recorded). The interval/cardinality machinery
  and ISO 8601 validity functions verify clean.

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1–R3 | verified | `interval_impl.rs` `has`/`contains`/`intersects` (+ `multiplicity_interval_impl.rs` mirrors) with the exact bound logic; unit-tested |
| R4–R7 | verified | `dv_interval_impl.rs` enforces `Lower/Upper_included_valid` + `Limits_consistent` on every wire interval; foundation `Interval` instances have no independent wire surface (they appear via AOM constraints — chapter 13's walker uses the interval math) |
| R8 | verified | generated `Interval<T>` boolean fields non-optional |
| R9–R11 | verified-no-surface | `Point_interval`/`Proper_interval` never appear as wire instances; generated shapes pin the model |
| R12/R13 | verified | `multiplicity_interval_impl.rs` `is_open/is_optional/is_mandatory/is_prohibited`; `cardinality_impl.rs` `is_bag/is_list/is_set` — the constraint-evaluation primitives the validator consumes |
| R14–R20 | verified | `validate.rs` numeric bounds (year ≥ 0 via 4-digit form, month 1–12, calendar-exact `days_in_month` incl. leap years, hour 0–23, minute 0–59, second 0–60 lexical with strict numeric semantics, fractional < 1) |
| R21 | verified | hour range 0–23 everywhere — `24:00:00` rejected (the openEHR deviation) |
| R22–R28 | verified | `is_valid_iso_date/time/date_time/duration`: partial forms (right-truncation only), compact + extended, fractional seconds only, no week-dates, duration `W` mixing + leading sign (chapter-6 verdicts R7–R18 cross-reference) |
| R29–R39 | verified | the string validators realize the `Iso8601_*` invariant set (the structural classes have no wire instances — values travel as strings in `DV_DATE/TIME/DATE_TIME/DURATION.value`) |
| R40 | verified | `AVERAGE_DAYS_IN_YEAR = 365.24` / `AVERAGE_DAYS_IN_MONTH = 30.42` in `dv_ordered_impl.rs` AND the SQL `ext` duration function — both cited to `Time_definitions` |
| R41/R42 | fixed-in-this-pass | timezone bounds were symmetric (0–14 both signs): now `+` ≤ 14, `−` ≤ 12 (`is_valid_tz`); PORT NOTE: hour 0 accepted with a sign — the corpus/CNF carry `±00:00` in 42 files (corpus outranks the literal `hour > 0`) |
| R43/R44 | verified | minute 0–59; `Z`/±hh[mm] forms |
| R45 | verified-no-surface | no date-arithmetic surface exists (durations are magnitudes; date arithmetic is consumer-driven) — flagged; the nominal constants (R40) are in place for when one lands |
| R46 | verified | `value: String` non-optional on the generated ISO types |
| R47/R48 | verified | `TerminologyCode`/`TerminologyTerm` mandatory fields (typed at the SM seams — `UpdateVersion.lifecycle_state` etc.) |
| R49/R50 | verified | chapter-5 `Code_string_valid` + typed `terminology_id` slot |
| R51–R53 | verified | realized by `Vec`/`BTreeSet`/`BTreeMap` semantics in the generated model |
| R54–R57 | verified | `Integer` = `i32`, `Integer64` = `i64` (DV_COUNT), `Real` = `f64` on the wire (JSON number), UTF-8 strings; `Ordered` = Rust `PartialOrd` total-order usage in the interval math |

## Fixes applied

- **R41/R42** — `crates/openehr-rm/src/validate.rs::is_valid_tz`: asymmetric
  bounds (+14/−12) per `iso8601_timezone.adoc` + `time_definitions.adoc`;
  test `timezone_bounds_are_asymmetric`; ±00:00 corpus adjudication noted.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
