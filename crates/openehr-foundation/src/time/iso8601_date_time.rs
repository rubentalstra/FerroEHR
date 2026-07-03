//! `Iso8601_date_time` — an ISO 8601 date/time, including partial and
//! extended forms.
//!
//! openEHR class: `Iso8601_date_time`, package `base.foundation_types.time`.
//! Inherits: `Iso8601_type`.
//!
//! Represents an ISO 8601 date/time, including partial and extended forms.
//! Value may be:
//! * `YYYY-MM-DDThh:mm:ss[(,|.)sss][Z | ±hh[:mm]]` (extended, preferred) or
//! * `YYYYMMDDThhmmss[(,|.)sss][Z | ±hh[mm]]` (compact)
//! * or a partial variant.
//!
//! See `Time_Definitions::valid_iso8601_date_time` for validity.
//!
//! NOTE (spec): this class includes 2 deviations from ISO 8601:2004: for
//! partial date/times, any part of the date/time up to the month may be
//! missing, not just seconds and minutes as in the standard; and the time
//! `24:00:00` is not allowed, since it would mean the date was really on the
//! next day.
//!
//! # String-value representation, not a resolved instant
//!
//! Models an ISO 8601 string value with partial precision, not a resolved
//! calendar instant. See the module-level doc on `iso8601_type.rs` for the
//! full rationale and the jiff-bridging plan for P17.
use crate::primitive_types::any::Any;
use crate::primitive_types::ordered::Ordered;
use crate::time::iso8601_arithmetic::{
    definite_duration_string_from_seconds, format_date_time_at_precision, nominal_span,
    signed_duration_from_seconds,
};
use crate::time::iso8601_duration::Iso8601Duration;
use crate::time::iso8601_parser::{parse_date_time, parse_duration};
use crate::time::iso8601_timezone::Iso8601Timezone;
use crate::time::iso8601_type::{Iso8601Type, Iso8601TypeCore};
use crate::time::temporal::Temporal;
use crate::time::time_definitions::TimeDefinitions;
use serde::{Deserialize, Serialize};

/// `Iso8601_date_time` embeds the `Iso8601_type` parent state (`value:
/// String`) via `Iso8601TypeCore`, per ADR-001 §3. This struct declares no
/// attributes of its own beyond the inherited `value`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Iso8601DateTime {
    /// Embedded `Iso8601_type.value: String`.
    #[serde(flatten)]
    pub core: Iso8601TypeCore,
}

impl Iso8601DateTime {
    /// `year(): Integer`.
    ///
    /// Extract the year part of the date as an Integer.
    ///
    #[must_use]
    pub fn year(&self) -> i32 {
        parse_date_time(&self.core.value).map_or(0, |parsed| parsed.date.year)
    }

    /// `month(): Integer`.
    ///
    /// Pre: `not month_unknown`.
    ///
    /// Extract the month part of the date/time as an Integer, or return 0 if
    /// not present.
    ///
    #[must_use]
    pub fn month(&self) -> i32 {
        parse_date_time(&self.core.value).map_or(0, |parsed| parsed.date.month.unwrap_or(0))
    }

    /// `day(): Integer`.
    ///
    /// Pre: `not day_unknown`.
    ///
    /// Extract the day part of the date/time as an Integer, or return 0 if
    /// not present.
    ///
    #[must_use]
    pub fn day(&self) -> i32 {
        parse_date_time(&self.core.value).map_or(0, |parsed| parsed.date.day.unwrap_or(0))
    }

    /// `hour(): Integer`.
    ///
    /// Pre: `not hour_unknown`.
    ///
    /// Extract the hour part of the date/time as an Integer, or return 0 if
    /// not present.
    ///
    #[must_use]
    pub fn hour(&self) -> i32 {
        parse_date_time(&self.core.value).map_or(0, |parsed| parsed.time.hour)
    }

    /// `minute(): Integer`.
    ///
    /// Pre: `not minute_unknown`.
    ///
    /// Extract the minute part of the date/time as an Integer, or return 0
    /// if not present.
    ///
    #[must_use]
    pub fn minute(&self) -> i32 {
        parse_date_time(&self.core.value).map_or(0, |parsed| parsed.time.minute_value())
    }

    /// `second(): Integer`.
    ///
    /// Pre: `not second_unknown`.
    ///
    /// Extract the integral seconds part of the date/time (i.e. prior to any
    /// decimal sign) as an Integer, or return 0 if not present.
    ///
    #[must_use]
    pub fn second(&self) -> i32 {
        parse_date_time(&self.core.value).map_or(0, |parsed| parsed.time.second_value())
    }

    /// `fractional_second(): Real`.
    ///
    /// Extract the fractional seconds part of the date/time (i.e. following
    /// to any decimal sign) as a Real, or return 0.0 if not present.
    ///
    #[must_use]
    pub fn fractional_second(&self) -> f64 {
        parse_date_time(&self.core.value).map_or(0.0, |parsed| parsed.time.fractional_second())
    }

    /// `timezone(): Iso8601_timezone`.
    ///
    /// Timezone; may be Void.
    ///
    #[must_use]
    pub fn timezone(&self) -> Option<Iso8601Timezone> {
        parse_date_time(&self.core.value)
            .and_then(|parsed| parsed.time.timezone)
            .map(|timezone| Iso8601Timezone {
                core: Iso8601TypeCore {
                    value: timezone.as_iso8601_string(),
                },
            })
    }

    /// `month_unknown(): Boolean`.
    ///
    /// Indicates whether month in year is unknown.
    ///
    #[must_use]
    pub fn month_unknown(&self) -> bool {
        parse_date_time(&self.core.value).is_none_or(|parsed| parsed.date.month_unknown())
    }

    /// `day_unknown(): Boolean`.
    ///
    /// Indicates whether day in month is unknown.
    ///
    #[must_use]
    pub fn day_unknown(&self) -> bool {
        parse_date_time(&self.core.value).is_none_or(|parsed| parsed.date.day_unknown())
    }

    /// `minute_unknown(): Boolean`.
    ///
    /// Indicates whether minute in hour is known.
    ///
    /// PORT NOTE: the spec description text literally reads "Indicates
    /// whether minute in hour **is known**" — i.e. worded as the positive
    /// ("is known"), not the negative ("is unknown") the function's own
    /// name (`minute_unknown`) and every sibling accessor's phrasing
    /// implies. This looks like a copy-paste/wording artifact in the
    /// published table (compare `Iso8601_time.minute_unknown`, worded
    /// correctly as "Indicates whether minute is unknown"). Transcribed
    /// with the name-implied ("is unknown") semantics, consistent with
    /// every other `*_unknown` accessor in this file and its siblings, but
    /// flagged here since the table's own wording is internally
    /// inconsistent rather than silently "corrected" without comment.
    ///
    #[must_use]
    pub fn minute_unknown(&self) -> bool {
        parse_date_time(&self.core.value).is_none_or(|parsed| parsed.time.minute_unknown())
    }

    /// `second_unknown(): Boolean`.
    ///
    /// Indicates whether minute in hour is known.
    ///
    /// PORT NOTE: the spec table gives `second_unknown` the *exact same*
    /// description text as `minute_unknown` immediately above it
    /// ("Indicates whether minute in hour is known"), which is very likely
    /// a copy-paste error in the published table (a function named
    /// `second_unknown` describing "minute" is internally inconsistent).
    /// Transcribed with the name-implied semantics ("indicates whether
    /// second is unknown"), consistent with `Iso8601_time.second_unknown`'s
    /// correctly-worded description; flagged here rather than silently
    /// "corrected" without comment.
    ///
    #[must_use]
    pub fn second_unknown(&self) -> bool {
        parse_date_time(&self.core.value).is_none_or(|parsed| parsed.time.second_unknown())
    }

    /// `is_decimal_sign_comma(): Boolean`.
    ///
    /// True if this time has a decimal part indicated by `','` (comma)
    /// rather than `'.'` (period).
    ///
    #[must_use]
    pub fn is_decimal_sign_comma(&self) -> bool {
        parse_date_time(&self.core.value).is_some_and(|parsed| parsed.time.decimal_sign_comma)
    }

    /// `has_fractional_second(): Boolean`.
    ///
    /// True if the `fractional_second` part is significant (i.e. even if =
    /// 0.0).
    ///
    #[must_use]
    pub fn has_fractional_second(&self) -> bool {
        parse_date_time(&self.core.value).is_some_and(|parsed| parsed.time.has_fractional_second)
    }

    /// `add` __alias__ `"+"` `(a_diff: Iso8601_duration) -> Iso8601_date_time`.
    ///
    /// Arithmetic addition of a duration to a date/time.
    ///
    /// Definite arithmetic per master06-time_types.adoc §Computational
    /// Functions and ADR-003 policy 1: the duration is an exact quantity
    /// (`to_seconds()`, averages 365.24/30.42 for years/months), applied as
    /// an exact `jiff::SignedDuration` to the anchored civil date-time
    /// (unknown minute/second/fraction filled with 0, ADR-003 policy 3),
    /// and the result truncated back to this value's original precision.
    /// Timezone text is preserved verbatim; the arithmetic is civil (no
    /// DST — a fixed ISO 8601 offset carries no zone rules).
    ///
    /// PORT NOTE: the spec declares no error channel; on an unparseable
    /// receiver or an out-of-range result (year outside 0000–9999), the
    /// receiver is returned unchanged — the same fallback convention as
    /// `Iso8601Duration::divide`.
    #[must_use]
    pub fn add(&self, a_diff: &Iso8601Duration) -> Iso8601DateTime {
        self.definite_shift(a_diff.to_seconds())
    }

    /// `subtract` __alias__ `"-"` `(a_diff: Iso8601_duration) -> Iso8601_date_time`.
    ///
    /// Arithmetic subtraction of a duration from a date/time.
    ///
    /// Definite arithmetic; see `add` for semantics, policy, and the
    /// fallback PORT NOTE.
    #[must_use]
    pub fn subtract(&self, a_diff: &Iso8601Duration) -> Iso8601DateTime {
        self.definite_shift(-a_diff.to_seconds())
    }

    /// `diff` __alias__ `"-"` `(a_date_time: Iso8601_date_time) -> Iso8601_duration`.
    ///
    /// Difference of two date/times.
    ///
    /// Definite arithmetic per ADR-003 policy 1: the difference of the two
    /// anchored civil date-times (receiver minus argument, fractional
    /// seconds included), returned as a normalized `Iso8601_duration` in
    /// definite units only (days and below — never years/months).
    ///
    /// PORT NOTE: operand order is receiver minus argument; see
    /// `Iso8601Date::diff`. Unparseable operands yield the zero duration.
    #[must_use]
    pub fn diff(&self, a_date_time: &Iso8601DateTime) -> Iso8601Duration {
        let seconds = match (
            parse_date_time(&self.core.value)
                .and_then(super::iso8601_parser::ParsedIso8601DateTime::as_jiff_datetime),
            parse_date_time(&a_date_time.core.value)
                .and_then(super::iso8601_parser::ParsedIso8601DateTime::as_jiff_datetime),
        ) {
            (Some(left), Some(right)) => left.duration_since(right).as_secs_f64(),
            _ => 0.0,
        };
        Iso8601Duration {
            core: Iso8601TypeCore {
                value: definite_duration_string_from_seconds(seconds),
            },
        }
    }

    /// `add_nominal` __alias__ `"++"` `(a_diff: Iso8601_duration) -> Iso8601_date_time`.
    ///
    /// Addition of nominal duration represented by `a_diff`. See
    /// `Iso8601_date.add_nominal` for semantics.
    ///
    /// PORT NOTE: the published per-class table types this function's
    /// *return* value as `Iso8601_date`, not `Iso8601_date_time`, even
    /// though the function is declared on the `Iso8601_date_time` class and
    /// its own description explicitly delegates to
    /// `Iso8601_date._add_nominal_()` for semantics ("nominal addition of a
    /// date/time" naturally yields another date/time, not a bare date).
    /// This looks like a copy-paste artifact from `Iso8601_date`'s own
    /// `add_nominal` row rather than an intentional narrowing. Transcribed
    /// here returning `Iso8601DateTime` (matching the receiver type and the
    /// analogous same-type pattern every other `add`/`subtract`/
    /// `add_nominal`/`subtract_nominal` pair in this module follows, e.g.
    /// `Iso8601Date::add_nominal -> Iso8601Date`), not the table's literal
    /// `Iso8601_date`, since a return type that discards the time-of-day
    /// components on every nominal-add call would be a significant, silent
    /// data-loss bug rather than a plausible spec intent — flagged loudly
    /// here rather than propagated silently.
    ///
    /// Nominal calendrical arithmetic per master06-time_types.adoc
    /// §Computational Functions and ADR-003 policy 2: years/months/weeks/
    /// days as calendar units via `jiff::Span` on the anchored civil
    /// date-time (end-of-month clamping per jiff), sub-day components as
    /// exact time; result truncated back to this value's original
    /// precision. See the fallback PORT NOTE on `add`.
    #[must_use]
    pub fn add_nominal(&self, a_diff: &Iso8601Duration) -> Iso8601DateTime {
        self.nominal_shift(a_diff, false)
    }

    /// `subtract_nominal` __alias__ `"--"` `(a_diff: Iso8601_duration) -> Iso8601_date_time`.
    ///
    /// Subtraction of nominal duration represented by `a_diff`. See
    /// `add_nominal` for semantics.
    ///
    /// PORT NOTE: same table-typo reasoning as `add_nominal` above applies
    /// here — the published table also types this return value as
    /// `Iso8601_date`; transcribed as `Iso8601DateTime` for the same
    /// same-type-pattern and data-loss reasoning.
    ///
    /// Nominal calendrical arithmetic; see `add_nominal` for semantics,
    /// policy, and the fallback PORT NOTE on `add`.
    #[must_use]
    pub fn subtract_nominal(&self, a_diff: &Iso8601Duration) -> Iso8601DateTime {
        self.nominal_shift(a_diff, true)
    }

    /// Shared definite-arithmetic engine call: anchor (ADR-003 policy 3),
    /// apply the exact seconds as a `jiff::SignedDuration` on the civil
    /// date-time, truncate back to the receiver's precision.
    fn definite_shift(&self, seconds: f64) -> Iso8601DateTime {
        let result = parse_date_time(&self.core.value).and_then(|parsed| {
            let anchored = parsed.as_jiff_datetime()?;
            let duration = signed_duration_from_seconds(seconds)?;
            let shifted = anchored.checked_add(duration).ok()?;
            format_date_time_at_precision(parsed, shifted, parsed.extended, false)
        });
        result.map_or_else(|| self.clone(), Self::from_value)
    }

    /// Shared nominal-arithmetic engine call: anchor, apply the duration's
    /// components as a calendar `jiff::Span` (negated for subtraction) on
    /// the civil date-time, truncate back to the receiver's precision.
    fn nominal_shift(&self, a_diff: &Iso8601Duration, negate: bool) -> Iso8601DateTime {
        let result = parse_date_time(&self.core.value).and_then(|parsed| {
            let anchored = parsed.as_jiff_datetime()?;
            let span = parse_duration(&a_diff.core.value)
                .as_ref()
                .and_then(nominal_span)?;
            let span = if negate { span.negate() } else { span };
            let shifted = anchored.checked_add(span).ok()?;
            format_date_time_at_precision(parsed, shifted, parsed.extended, false)
        });
        result.map_or_else(|| self.clone(), Self::from_value)
    }

    /// Internal constructor from an already-formatted value string.
    fn from_value(value: String) -> Iso8601DateTime {
        Iso8601DateTime {
            core: Iso8601TypeCore { value },
        }
    }
}

impl Any for Iso8601DateTime {
    fn is_equal(&self, other: &Self) -> bool {
        self.core == other.core
    }

    fn type_of(&self) -> String {
        "Iso8601_date_time".to_string()
    }
}

impl Ordered for Iso8601DateTime {
    /// `less_than` __alias__ `"<"` `(other: Iso8601_date_time) -> Boolean`.
    ///
    /// PORT NOTE: not itself declared on `Iso8601_date_time`'s per-class
    /// table — inherited abstractly from `Ordered` via `Temporal`. A
    /// faithful effector compares parsed, partial-aware
    /// year/month/day/hour/minute/second components.
    fn less_than(&self, other: &Self) -> bool {
        match (
            parse_date_time(&self.core.value),
            parse_date_time(&other.core.value),
        ) {
            (Some(left), Some(right)) => {
                (
                    left.date.year,
                    left.date.month.unwrap_or(0),
                    left.date.day.unwrap_or(0),
                    left.time.hour,
                    left.time.minute_value(),
                    left.time.second_value(),
                    left.time.nanosecond,
                ) < (
                    right.date.year,
                    right.date.month.unwrap_or(0),
                    right.date.day.unwrap_or(0),
                    right.time.hour,
                    right.time.minute_value(),
                    right.time.second_value(),
                    right.time.nanosecond,
                )
            }
            _ => self.core.value < other.core.value,
        }
    }
}

impl Temporal for Iso8601DateTime {}

impl Iso8601Type for Iso8601DateTime {
    fn core(&self) -> &Iso8601TypeCore {
        &self.core
    }

    /// `as_string(): String`.
    ///
    /// Return the string value in extended format: a compact-form value
    /// (`"20040215T103005"`) is reformatted with `-`/`:` separators (and
    /// the canonical `±hh:mm` timezone form) at its original precision,
    /// effecting the "in extended format" contract the
    /// `Iso8601Type::as_string` default cannot honour without parsing. An
    /// unparseable value is returned verbatim.
    fn as_string(&self) -> String {
        parse_date_time(&self.core.value)
            .and_then(|parsed| {
                let anchored = parsed.as_jiff_datetime()?;
                format_date_time_at_precision(parsed, anchored, true, true)
            })
            .unwrap_or_else(|| self.core.value.clone())
    }

    /// `is_partial(): Boolean` (effected).
    ///
    /// True if this date time is partial, i.e. if seconds or more is
    /// missing.
    ///
    fn is_partial(&self) -> bool {
        self.month_unknown() || self.day_unknown() || self.minute_unknown() || self.second_unknown()
    }

    /// `is_extended(): Boolean` (effected).
    ///
    /// True if this date/time uses `'-'`, `':'` separators.
    ///
    fn is_extended(&self) -> bool {
        parse_date_time(&self.core.value).is_some_and(|parsed| parsed.extended)
    }
}

// PORT NOTE: see the equivalent note in `iso8601_date.rs` — `Time_Definitions`
// is not a Rust supertrait here; these invariants call `TimeDefinitions::*`
// directly, and are encoded as plain boolean-returning methods rather than a
// `Validate` impl.
impl Iso8601DateTime {
    /// __`Year_valid`__: `valid_year (year)`.
    #[must_use]
    pub fn invariant_year_valid(&self) -> bool {
        TimeDefinitions::valid_year(self.year())
    }

    /// __`Month_valid`__: `valid_month (month)`.
    ///
    /// PORT NOTE: unlike `Iso8601_date`'s equivalent invariant (guarded by
    /// `not month_unknown implies ...`), the spec's `Iso8601_date_time`
    /// table states this invariant unconditionally. Transcribed literally
    /// as written, even though it appears to conflict with the
    /// `Partial_validity_month` invariant below (`not month_unknown`) —
    /// i.e. the spec's own invariant set for this class implies
    /// `month_unknown` can never actually be true for a *valid* instance,
    /// which is a stronger constraint than `Iso8601_date` imposes. Not
    /// "fixed" to match `Iso8601_date`'s conditional form; transcribed
    /// exactly as the table states.
    #[must_use]
    pub fn invariant_month_valid(&self) -> bool {
        TimeDefinitions::valid_month(self.month())
    }

    /// __`Day_valid`__: `valid_day(year, month, day)`.
    ///
    /// PORT NOTE: same unconditional-vs-`Iso8601_date` observation as
    /// `invariant_month_valid` above applies here.
    #[must_use]
    pub fn invariant_day_valid(&self) -> bool {
        TimeDefinitions::valid_day(self.year(), self.month(), self.day())
    }

    /// __`Hour_valid`__: `valid_hour (hour, minute, second)`.
    #[must_use]
    pub fn invariant_hour_valid(&self) -> bool {
        TimeDefinitions::valid_hour(self.hour(), self.minute(), self.second())
    }

    /// __`Minute_valid`__: `not minute_unknown implies valid_minute(minute)`.
    #[must_use]
    pub fn invariant_minute_valid(&self) -> bool {
        self.minute_unknown() || TimeDefinitions::valid_minute(self.minute())
    }

    /// __`Second_valid`__: `not second_unknown implies valid_second (second)`.
    #[must_use]
    pub fn invariant_second_valid(&self) -> bool {
        self.second_unknown() || TimeDefinitions::valid_second(self.second())
    }

    /// __`Fractional_second_valid`__: `has_fractional_second implies (not
    /// second_unknown and valid_fractional_second (fractional_second))`.
    #[must_use]
    pub fn invariant_fractional_second_valid(&self) -> bool {
        !self.has_fractional_second()
            || (!self.second_unknown()
                && TimeDefinitions::valid_fractional_second(self.fractional_second()))
    }

    /// __`Partial_validity_year`__: `not month_unknown`.
    ///
    /// PORT NOTE: the spec's own label ("`Partial_validity`_**year**") does
    /// not match its stated condition (`not month_unknown`, about month,
    /// not year) — transcribed exactly as the table states regardless.
    #[must_use]
    pub fn invariant_partial_validity_year(&self) -> bool {
        !self.month_unknown()
    }

    /// __`Partial_validity_month`__: `not month_unknown`.
    ///
    /// PORT NOTE: the spec table gives `Partial_validity_year` and
    /// `Partial_validity_month` the *identical* condition (`not
    /// month_unknown`) — transcribed as two separate methods matching the
    /// two separate named invariants in the table, even though they are
    /// currently behaviourally identical; this mirrors the ambiguity in
    /// the published spec rather than silently collapsing it to one.
    #[must_use]
    pub fn invariant_partial_validity_month(&self) -> bool {
        !self.month_unknown()
    }

    /// __`Partial_validity_day`__: `not day_unknown`.
    #[must_use]
    pub fn invariant_partial_validity_day(&self) -> bool {
        !self.day_unknown()
    }

    /// __`Partial_validity_hour`__: `not hour_unknown`.
    ///
    /// PORT NOTE: the invariant condition references `hour_unknown`, which
    /// is not itself declared as a `Functions` row anywhere in this class's
    /// per-class table (unlike `month_unknown`/`day_unknown`/
    /// `minute_unknown`/`second_unknown`, which are all declared).
    /// `Iso8601_date_time.hour()`'s own `Pre` clause (`not hour_unknown`)
    /// confirms the concept exists and is used elsewhere in this same
    /// table, so this is a genuine spec gap (an implied accessor that is
    /// never formally declared as a `Functions` row), not an oversight in
    /// this transcription. Since `valid_iso8601_date_time`'s explicit
    /// grammar always includes the hour field, the invariant is implemented
    /// as "the date-time parses under that grammar".
    #[must_use]
    pub fn invariant_partial_validity_hour(&self) -> bool {
        parse_date_time(&self.core.value).is_some()
    }

    /// __`Partial_validity_minute`__: `minute_unknown implies second_unknown`.
    #[must_use]
    pub fn invariant_partial_validity_minute(&self) -> bool {
        !self.minute_unknown() || self.second_unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date_time(value: &str) -> Iso8601DateTime {
        Iso8601DateTime {
            core: Iso8601TypeCore {
                value: value.to_string(),
            },
        }
    }

    fn duration(value: &str) -> Iso8601Duration {
        Iso8601Duration {
            core: Iso8601TypeCore {
                value: value.to_string(),
            },
        }
    }

    #[test]
    fn definite_and_nominal_month_addition_diverge() {
        // Definite P1M = exactly 30.42 days = 2 628 288 s.
        let dt = date_time("2004-02-15T10:00:00");
        assert_eq!(dt.add(&duration("P1M")).core.value, "2004-03-16T20:04:48");
        assert_eq!(
            dt.add_nominal(&duration("P1M")).core.value,
            "2004-03-15T10:00:00"
        );
    }

    #[test]
    fn definite_day_addition_is_exactly_24h() {
        assert_eq!(
            date_time("2004-02-28T23:30:00")
                .add(&duration("P1D"))
                .core
                .value,
            "2004-02-29T23:30:00"
        );
        assert_eq!(
            date_time("2004-02-29T23:30:00")
                .add(&duration("PT1H"))
                .core
                .value,
            "2004-03-01T00:30:00"
        );
    }

    #[test]
    fn nominal_addition_clamps_then_applies_exact_time() {
        // 2004-01-31 +1 nominal month clamps to 2004-02-29 (leap year),
        // then the exact 2h rolls over into the next day.
        assert_eq!(
            date_time("2004-01-31T23:00:00")
                .add_nominal(&duration("P1MT2H"))
                .core
                .value,
            "2004-03-01T01:00:00"
        );
        assert_eq!(
            date_time("2004-03-31T10:00:00")
                .subtract_nominal(&duration("P1M"))
                .core
                .value,
            "2004-02-29T10:00:00"
        );
    }

    #[test]
    fn partial_precision_is_anchored_then_truncated() {
        // Second unknown: anchored to :00, shifted, kept at hh:mm.
        assert_eq!(
            date_time("2004-02-15T10:30")
                .add(&duration("PT30M"))
                .core
                .value,
            "2004-02-15T11:00"
        );
        // Hour-only time part.
        assert_eq!(
            date_time("2004-02-15T10").add(&duration("PT1H")).core.value,
            "2004-02-15T11"
        );
    }

    #[test]
    fn timezone_text_is_preserved_verbatim_and_arithmetic_is_civil() {
        assert_eq!(
            date_time("2004-02-15T10:00:00Z")
                .add(&duration("PT1H"))
                .core
                .value,
            "2004-02-15T11:00:00Z"
        );
        assert_eq!(
            date_time("2004-02-15T10:00:00+02:00")
                .add_nominal(&duration("P1D"))
                .core
                .value,
            "2004-02-16T10:00:00+02:00"
        );
    }

    #[test]
    fn diff_returns_definite_units_and_is_antisymmetric() {
        let later = date_time("2004-03-16T20:04:48");
        let earlier = date_time("2004-02-15T10:00:00");
        assert_eq!(later.diff(&earlier).core.value, "P30DT10H4M48S");
        assert_eq!(earlier.diff(&later).core.value, "-P30DT10H4M48S");
        assert_eq!(
            date_time("2004-02-15T10:00:00.5")
                .diff(&date_time("2004-02-15T10:00:00"))
                .core
                .value,
            "PT0.5S"
        );
    }

    #[test]
    fn out_of_range_result_falls_back_to_receiver() {
        let dt = date_time("9999-12-31T23:00:00");
        assert_eq!(dt.add(&duration("P2D")).core.value, "9999-12-31T23:00:00");
    }

    #[test]
    fn as_string_reformats_compact_to_extended() {
        assert_eq!(
            date_time("20040215T103005").as_string(),
            "2004-02-15T10:30:05"
        );
        assert_eq!(
            date_time("20040215T1030+0230").as_string(),
            "2004-02-15T10:30+02:30"
        );
        assert_eq!(
            date_time("2004-02-15T10:30:05Z").as_string(),
            "2004-02-15T10:30:05Z"
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso8601_date_time.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / iso8601_date_time.adoc §Iso8601_date_time Class
//   confidence: medium
//   todos: 0
//   note: string accessors, partiality, extended-form detection, ordering, and the hour-validity spec gap delegate to the shared BASE ISO 8601 parser; add/subtract/diff (definite, averages as exact SignedDuration) and add_nominal/subtract_nominal (jiff Span calendar units, clamping) implemented per ADR-003 policies 1-3 via iso8601_arithmetic.rs with anchoring + truncation, timezone text verbatim, and a documented return-receiver fallback; as_string now effects the extended-format contract. Published-table inconsistencies remain flagged rather than silently corrected — add_nominal/subtract_nominal's stated Iso8601_date return type (transcribed as Iso8601DateTime, with reasoning), minute_unknown/second_unknown wording, unconditional Month_valid/Day_valid, duplicate Partial_validity_year/month, and Partial_validity_hour's undeclared hour_unknown() accessor.
// ─────────────────────────────────────────────
