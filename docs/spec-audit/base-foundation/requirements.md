# Spec audit — chapter `base-foundation` (Phase 1: Extract)

- **Date:** 2026-07-11
- **Component:** BASE — `base.foundation_types` (BASE 1.3.0)
- **Spec files read** (relative to `docs/specs/openehr/`):
  - `BASE/docs/foundation_types/master03-primitive_types.adoc`
  - `BASE/docs/foundation_types/master04-structure_types.adoc`
  - `BASE/docs/foundation_types/master05-interval.adoc`
  - `BASE/docs/foundation_types/master06-time_types.adoc`
  - `BASE/docs/foundation_types/master07-terminology.adoc`
  - `BASE/docs/foundation_types/master08-functional.adoc`
  - The per-class normative content is `include::`d from
    `BASE/docs/UML/classes/org.openehr.base.foundation_types.*.adoc`
    (interval, point_interval, proper_interval, multiplicity_interval,
    cardinality, time_definitions, iso8601_date, iso8601_time,
    iso8601_date_time, iso8601_duration, iso8601_timezone, iso8601_type,
    terminology_code, terminology_term, code_phrase, list). Those class files
    were read in full and are the actual citation targets below.

**Note on file correction:** the master files listed in the task contain only
`include::` directives; every invariant, validity function, and function
post-condition lives in the `docs/UML/classes/org.openehr.base.foundation_types.*.adoc`
class files. Citations below point at the class files (the normative source),
naming the parent master file for context.

---

## Requirements

| id | requirement | citation | category | risk |
|---|---|---|---|---|
| base-foundation-R1 | `Interval<T>.has(e)` membership must follow the exact bound logic: `Result = (lower_unbounded or lower_included and v >= lower) or v > lower and (upper_unbounded or upper_included and v <= upper or v < upper)` — open bounds admit −∞/+∞; closed bounds include the limit value; open (non-included) bounds are strict `<`/`>`. | `master05-interval` → `interval.adoc` §Interval Class, `has()` Post_result (lines 46-53) | behaviour | high |
| base-foundation-R2 | `Interval<T>.contains(other)` must be true iff **all** points of `other` lie inside the current interval (proper containment of one interval by another). | `interval.adoc` §Interval Class, `contains()` (lines 62-67) | behaviour | medium |
| base-foundation-R3 | `Interval<T>.intersects(other)` must be true iff there is any overlap — at least one limit of `other` is strictly inside this interval's limits. | `interval.adoc` §Interval Class, `intersects()` (lines 55-60) | behaviour | medium |
| base-foundation-R4 | Interval invariant `Lower_included_valid`: `lower_unbounded implies not lower_included` — an unbounded lower boundary must not be marked as included. Reject a constructed interval violating this. | `interval.adoc` §Invariants (line 77) | invariant | high |
| base-foundation-R5 | Interval invariant `Upper_included_valid`: `upper_unbounded implies not upper_included` — an unbounded upper boundary must not be marked as included. | `interval.adoc` §Invariants (line 80) | invariant | high |
| base-foundation-R6 | Interval invariant `Limits_consistent`: `(not upper_unbounded and not lower_unbounded) implies lower <= upper` — a two-sided interval must have `lower <= upper`. Reject an inverted interval. | `interval.adoc` §Invariants (line 83) | invariant | high |
| base-foundation-R7 | Interval invariant `Limits_comparable`: when both limits are bounded they must be `strictly_comparable_to` each other (same comparable type). | `interval.adoc` §Invariants (line 86) | invariant | medium |
| base-foundation-R8 | `Interval<T>` structural attributes `lower_unbounded`, `upper_unbounded`, `lower_included`, `upper_included` are each mandatory `Boolean` (`1..1`); `lower`/`upper` are optional (`0..1`). | `interval.adoc` §Attributes (lines 18-40) | mandatory-attr | medium |
| base-foundation-R9 | `Point_interval<T>` invariant `Inv_point`: `lower = upper` — a point interval's two limits must be equal. | `point_interval.adoc` §Invariants (line 43) | invariant | high |
| base-foundation-R10 | `Point_interval<T>` boundary defaults: `lower_unbounded = false`, `upper_unbounded = false`, `lower_included = true`, `upper_included = true` — a point interval is a closed, bounded single value. | `point_interval.adoc` §Attributes (lines 18-40) | invariant | medium |
| base-foundation-R11 | `Proper_interval<T>` invariant `Inv_not_point`: `lower /= upper` — a proper interval must not collapse to a point. | `proper_interval.adoc` §Invariants (line 16) | invariant | high |
| base-foundation-R12 | `Multiplicity_interval` is a `Proper_interval<Integer>`; `is_open()` ⇔ `0..*`, `is_optional()` ⇔ `0..1`, `is_mandatory()` ⇔ `1..1`, `is_prohibited()` ⇔ `0..0`. These predicates must reflect exactly these interval configurations. | `master05-interval` → `multiplicity_interval.adoc` §Functions (lines 30-44) | behaviour | high |
| base-foundation-R13 | `Cardinality.interval` is a mandatory `Multiplicity_interval` (`1..1`); `is_ordered` and `is_unique` are mandatory Booleans. `is_bag()` ⇔ (not ordered, not unique), `is_list()` ⇔ (ordered, not unique), `is_set()` ⇔ (not ordered, unique). | `master05-interval` → `cardinality.adoc` §Attributes+Functions (lines 15-41) | behaviour | medium |
| base-foundation-R14 | `Time_Definitions.valid_year(y)`: `Result = y >= 0` — reject negative years. | `master06-time_types` → `time_definitions.adoc` `valid_year` Post (lines 79-85) | validity-fn | high |
| base-foundation-R15 | `Time_Definitions.valid_month(m)`: `Result = m >= 1 and m <= 12` — reject month 0 or >12. | `time_definitions.adoc` `valid_month` Post (lines 87-93) | validity-fn | high |
| base-foundation-R16 | `Time_Definitions.valid_day(y,m,d)`: `Result = d >= 1 and d <= days_in_month(m, y)` — day must be calendar-exact against the Gregorian month/year (e.g. reject `2021-02-31`, reject `2021-02-29` in a non-leap year). | `time_definitions.adoc` `valid_day` Post (lines 95-103); `valid_iso8601_date` "must be correct with respect to the Gregorian calendar" (line 154) | validity-fn | high |
| base-foundation-R17 | `Time_Definitions.valid_hour(h,m,s)`: `Result = (h >= 0 and h < 24) or (h = 24 and m = 0 and s = 0)`. | `time_definitions.adoc` `valid_hour` Post (lines 105-113) | validity-fn | high |
| base-foundation-R18 | `Time_Definitions.valid_minute(m)`: `Result = m >= 0 and m < 60`. | `time_definitions.adoc` `valid_minute` Post (lines 115-121) | validity-fn | medium |
| base-foundation-R19 | `Time_Definitions.valid_second(s)`: `Result = s >= 0 and s < 60` (note: the `valid_iso8601_time` string grammar tolerates `ss` "00"-"60" for a leap second, but the numeric `valid_second` upper bound is strict `< 60`). | `time_definitions.adoc` `valid_second` Post (lines 123-129) | validity-fn | medium |
| base-foundation-R20 | `Time_Definitions.valid_fractional_second(fs)`: `Result = fs >= 0.0 and fs < 1.0` — reject a fractional-second ≥ 1.0 or negative. | `time_definitions.adoc` `valid_fractional_second` Post (lines 131-137) | validity-fn | high |
| base-foundation-R21 | The time `24:00:00` (or `240000`) must be rejected **anywhere** (dates, times, date-times) — a documented deviation from ISO 8601:2019; midnight is `00:00:00`. | `master06-time_types` §Primitive Time Types (line 35); `iso8601_time.adoc` NOTE (line 17); `iso8601_date_time.adoc` (line 20) | rejection-duty | high |
| base-foundation-R22 | `valid_iso8601_date(s)` must accept only: `YYYY-MM-DD` / `YYYY-MM` / `YYYY` (extended) or `YYYYMMDD` / `YYYYMM` (compact), with `YYYY`=0000-9999 zero-filled, `MM`=01-12, `DD`=01-31, the combination Gregorian-correct. | `time_definitions.adoc` `valid_iso8601_date` (lines 139-154) | validity-fn | high |
| base-foundation-R23 | `valid_iso8601_time(s)` must accept `hh:mm:ss[(,|.)s+][Z|±hh[:mm]]` / compact, or partial `hh:mm` / `hhmm` / `hh`, with `hh`=00-23, `mm`=00-59, `ss`=00-60, optional fractional-second and timezone. | `time_definitions.adoc` `valid_iso8601_time` (lines 156-179) | validity-fn | high |
| base-foundation-R24 | `valid_iso8601_date_time(s)` must accept `YYYY-MM-DDThh:mm:ss[(,|.)s+][Z|±hh[:mm]]` / compact, or the partial forms down to `YYYY-MM-DDThh` (`T` and `Z` are literals). | `time_definitions.adoc` `valid_iso8601_date_time` (lines 180-191) | validity-fn | high |
| base-foundation-R25 | `valid_iso8601_duration(s)` must accept `P[nnY][nnM][nnW][nnD][T[nnH][nnM][nnS]]`, with the `W` (week) designator permitted **in combination** with other designators (openEHR deviation from ISO). | `time_definitions.adoc` `valid_iso8601_duration` (lines 192-212); `master06` §Overview (line 30) | validity-fn | medium |
| base-foundation-R26 | ISO 8601 duration may carry a **negative sign** (whole-duration negation, e.g. `-P3M`), supporting 'adjusted age'. | `master06-time_types` §Overview (line 31); `iso8601_duration.adoc` NOTE (line 13), `negative` alias `-` (lines 100-102) | behaviour | medium |
| base-foundation-R27 | Partial `Iso8601_date_time` may omit any part down to (but not including) the month — a wider partial rule than ISO 8601:2019 (which allows only missing seconds/minutes). | `master06-time_types` §Overview (lines 32-34); `iso8601_date_time.adoc` Description (line 19) | behaviour | medium |
| base-foundation-R28 | Only **fractional seconds** are supported — no fractional minutes or hours (`hh,hhh` / `mm,mm` are excluded); year numbers are 4-digit only; the `YYYY-Www-D` week-date form is excluded. | `master06-time_types` §Overview (lines 23-25) | rejection-duty | medium |
| base-foundation-R29 | `Iso8601_date` invariant `Year_valid`: `valid_year(year)`. | `iso8601_date.adoc` §Invariants (line 101) | invariant | high |
| base-foundation-R30 | `Iso8601_date` invariant `Month_valid`: `not month_unknown implies valid_month(month)`. | `iso8601_date.adoc` §Invariants (line 104) | invariant | high |
| base-foundation-R31 | `Iso8601_date` invariant `Day_valid`: `not day_unknown implies valid_day(year, month, day)` (calendar-exact). | `iso8601_date.adoc` §Invariants (line 107) | invariant | high |
| base-foundation-R32 | `Iso8601_date` invariant `Partial_validity`: `month_unknown implies day_unknown` — a date must not carry a known day when the month is unknown. Reject `YYYY--DD`-style partials. | `iso8601_date.adoc` §Invariants (line 110) | invariant | high |
| base-foundation-R33 | `Iso8601_time` invariants: `Hour_valid` = `valid_hour(hour,minute,second)`; `Minute_valid` = `not minute_unknown implies valid_minute(minute)`; `Second_valid` = `not second_unknown implies valid_second(second)`. | `iso8601_time.adoc` §Invariants (lines 97-103) | invariant | high |
| base-foundation-R34 | `Iso8601_time` invariant `Fractional_second_valid`: `has_fractional_second implies (not second_unknown and valid_fractional_second(fractional_second))` — a fractional second requires a known integral second. | `iso8601_time.adoc` §Invariants (line 106) | invariant | medium |
| base-foundation-R35 | `Iso8601_time` invariant `Partial_validity`: `minute_unknown implies second_unknown` — cannot have a known second when the minute is unknown. | `iso8601_time.adoc` §Invariants (line 109) | invariant | high |
| base-foundation-R36 | `Iso8601_date_time` invariants require `valid_year/valid_month/valid_day/valid_hour` on the present parts, `Minute_valid`/`Second_valid` guarded by their `_unknown` flags, and `Fractional_second_valid` as for time. | `iso8601_date_time.adoc` §Invariants (lines 139-158) | invariant | high |
| base-foundation-R37 | `Iso8601_date_time` partial-validity chain: `minute_unknown implies second_unknown` (and the higher-order partial rules) — the omission order must be trailing-only. | `iso8601_date_time.adoc` §Invariants `Partial_validity_*` (lines 160-173) | invariant | high |
| base-foundation-R38 | `Iso8601_duration` invariants: each component `years`, `months`, `weeks`, `days`, `hours`, `minutes`, `seconds` must be `>= 0` — negativity is expressed on the whole duration (sign), never per-component. Reject a per-component negative. | `iso8601_duration.adoc` §Invariants (lines 104-123) | invariant | high |
| base-foundation-R39 | `Iso8601_duration` invariant `Fractional_second_valid`: `fractional_second >= 0.0 and fractional_second < 1.0`. | `iso8601_duration.adoc` §Invariants (line 126) | invariant | medium |
| base-foundation-R40 | `Iso8601_duration.to_seconds()` must convert non-definite Y/M parts using `Time_definitions.Average_days_in_year` (365.24) and `Average_days_in_month` (30.42). | `iso8601_duration.adoc` `to_seconds` (lines 68-70); `time_definitions.adoc` constants (lines 40-41, 68-69) | behaviour | low |
| base-foundation-R41 | `Iso8601_timezone` invariant `Max_hour_valid`: `sign = 1 implies hour > 0 and hour <= 14` (Max_timezone_hour = 14). Reject `+15:00`. | `iso8601_timezone.adoc` §Invariants (line 64); `time_definitions.adoc` `Max_timezone_hour` (lines 63-65) | invariant | high |
| base-foundation-R42 | `Iso8601_timezone` invariant `Min_hour_valid`: `sign = -1 implies hour > 0 and hour <= 12` (Min_timezone_hour = 12). Reject `-13:00`. | `iso8601_timezone.adoc` §Invariants (line 61); `time_definitions.adoc` `Min_timezone_hour` (lines 59-61) | invariant | high |
| base-foundation-R43 | `Iso8601_timezone` invariant `Minute_valid`: `not minute_unknown implies valid_minute(minute)`; and `Sign_valid`: `sign = 1 or sign = -1`. | `iso8601_timezone.adoc` §Invariants (lines 67-70) | invariant | medium |
| base-foundation-R44 | `Iso8601_timezone` format is `Z | ±hh[mm]` with `hh`=00-23 syntactically and `mm`=00-59; `Z` means UTC (`+0000`). | `iso8601_timezone.adoc` Description (lines 8-17) | validity-fn | medium |
| base-foundation-R45 | Nominal duration addition (`add_nominal`/`++`) must use calendar semantics: `P1Y` → same date next year (29 Feb → 28 Feb in following non-leap year); `P1M` → same day next month or clamped when shorter (31 Jan +P1M → 28/29 Feb). Distinct from definite `add`/`+` which uses average constants. | `iso8601_date.adoc` `add_nominal` (lines 84-92); `master06-time_types` §Computational Functions (lines 47-60) | behaviour | low |
| base-foundation-R46 | `Iso8601_type.value` is a mandatory `String` (`1..1`) — every ISO 8601 type has a single canonical String representation. | `master06-time_types` → `iso8601_type.adoc` §Attributes (lines 18-20) | mandatory-attr | low |
| base-foundation-R47 | `Terminology_code.terminology_id` (String) and `Terminology_code.code_string` (String) are mandatory (`1..1`); `terminology_version` and `uri` are optional (`0..1`). Reject a Terminology_code missing terminology_id or code_string. | `master07-terminology` → `terminology_code.adoc` §Attributes (lines 18-32) | mandatory-attr | high |
| base-foundation-R48 | `Terminology_term.concept` is a mandatory `Terminology_code` (`1..1`) and `Terminology_term.text` a mandatory `String` (`1..1`). | `master07-terminology` → `terminology_term.adoc` §Attributes (lines 18-24) | mandatory-attr | medium |
| base-foundation-R49 | `CODE_PHRASE` invariant `Code_string_valid`: `not code_string.is_empty` — reject an empty code_string. | `master07-terminology` → `code_phrase.adoc` §Invariants (line 30) | rejection-duty | high |
| base-foundation-R50 | `CODE_PHRASE.terminology_id` is typed to `TERMINOLOGY_ID` (a concrete monomorphic slot) and `code_string` to `String`; both mandatory (`1..1`). A foreign `_type` in the `terminology_id` slot must be rejected. | `master07-terminology` → `code_phrase.adoc` §Attributes (lines 17-23) | rejection-duty | high |
| base-foundation-R51 | `List<T>` invariant `First_validity`: `not is_empty implies first /= Void`; `Last_validity`: `not is_empty implies last /= Void` — a non-empty list must yield non-void first/last. | `master04-structure_types` → `list.adoc` §Invariants | invariant | low |
| base-foundation-R52 | `Set<T>` membership must be unique and unordered; `List<T>` has implied order with non-unique membership; `Array<T>` is number-indexed. These container semantics must hold where the type is used. | `master04-structure_types` §Overview table (lines 12-16) | behaviour | low |
| base-foundation-R53 | `Hash<K:Ordered, V>` keys must be a descendant of `Ordered` (typically String/Integer). | `master04-structure_types` §Overview table (line 15) | mandatory-attr | low |
| base-foundation-R54 | `Integer` is a 32-bit integer and `Integer64` a 64-bit integer — distinct widths; a value typed `Integer` must fit 32 bits (relevant to e.g. DV_COUNT). | `master03-primitive_types` §Overview table (lines 17-18) | serialization | medium |
| base-foundation-R55 | `Real` is 32-bit and `Double` 64-bit real; `String` is Unicode with UTF-8 encoding assumed. | `master03-primitive_types` §Overview table (lines 19-24), §Unicode (line 34) | serialization | low |
| base-foundation-R56 | `Octet` is an 8-bit value and `Character` is a member of an 8-bit character-set. | `master03-primitive_types` §Overview table (lines 12-13) | serialization | low |
| base-foundation-R57 | `Ordered` comparison operators must satisfy their post-conditions (`>` ⇔ `not (other < self)` region, `>=` ⇔ `other <= self`, etc.), giving a total order used by `Interval` limit comparisons. | `master03-primitive_types` → `ordered.adoc` Post_result conditions | behaviour | low |
