//! `Iso8601_date` — an ISO 8601 date, including partial and extended forms.
//!
//! openEHR class: `Iso8601_date`, package `base.foundation_types.time`.
//! Inherits: `Iso8601_type`.
//!
//! Represents an ISO 8601 date, including partial and extended forms. Value
//! may be:
//! * `YYYY-MM-DD` (extended, preferred)
//! * `YYYYMMDD` (compact)
//! * a partial invariant.
//!
//! See `Time_Definitions::valid_iso8601_date` for validity.
//!
//! # String-value representation, not a resolved instant
//!
//! Models an ISO 8601 string value with partial precision (e.g. `"2007-04"`,
//! meaning "year 2007, month 4, day unknown"), not a resolved calendar
//! instant. See the module-level doc on `iso8601_type.rs` for the full
//! rationale and the jiff-bridging plan for P17.
use crate::primitive_types::any::Any;
use crate::primitive_types::ordered::Ordered;
use crate::time::iso8601_arithmetic::{
    anchored_jiff_date, definite_duration_string_from_seconds, format_date_at_precision,
    nominal_span, signed_duration_from_seconds,
};
use crate::time::iso8601_duration::Iso8601Duration;
use crate::time::iso8601_parser::{parse_date, parse_duration};
use crate::time::iso8601_timezone::Iso8601Timezone;
use crate::time::iso8601_type::{Iso8601Type, Iso8601TypeCore};
use crate::time::temporal::Temporal;
use crate::time::time_definitions::TimeDefinitions;
use jiff::civil::{DateTime, Time};
use serde::{Deserialize, Serialize};

/// `Iso8601_date` embeds the `Iso8601_type` parent state (`value: String`)
/// via `Iso8601TypeCore`, per ADR-001 §3 (abstract-with-attributes → embedded
/// struct + marker trait). This struct declares no attributes of its own
/// beyond the inherited `value`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Iso8601Date {
    /// Embedded `Iso8601_type.value: String`.
    #[serde(flatten)]
    pub core: Iso8601TypeCore,
}

impl Iso8601Date {
    /// `year(): Integer`.
    ///
    /// Extract the year part of the date as an Integer.
    ///
    #[must_use]
    pub fn year(&self) -> i32 {
        parse_date(&self.core.value).map_or(0, |parsed| parsed.year)
    }

    /// `month(): Integer`.
    ///
    /// Pre: `not month_unknown`.
    ///
    /// Extract the month part of the date as an Integer, or return 0 if not
    /// present.
    ///
    #[must_use]
    pub fn month(&self) -> i32 {
        parse_date(&self.core.value).map_or(0, |parsed| parsed.month.unwrap_or(0))
    }

    /// `day(): Integer`.
    ///
    /// Pre: `not day_unknown`.
    ///
    /// Extract the day part of the date as an Integer, or return 0 if not
    /// present.
    ///
    #[must_use]
    pub fn day(&self) -> i32 {
        parse_date(&self.core.value).map_or(0, |parsed| parsed.day.unwrap_or(0))
    }

    /// `timezone(): Iso8601_timezone`.
    ///
    /// Timezone; may be Void.
    ///
    /// PORT NOTE: the spec's `Void`-may-be-returned wording maps to
    /// `Option<Iso8601Timezone>` — a date does not always carry a timezone
    /// component (dates rarely do; this accessor exists on the class table
    /// nonetheless, so transcribed faithfully with `Option`).
    ///
    #[must_use]
    pub fn timezone(&self) -> Option<Iso8601Timezone> {
        None
    }

    /// `month_unknown(): Boolean`.
    ///
    /// Indicates whether month in year is unknown. If so, the date is of the
    /// form `"YYYY"`.
    ///
    #[must_use]
    pub fn month_unknown(&self) -> bool {
        parse_date(&self.core.value)
            .is_none_or(super::iso8601_parser::ParsedIso8601Date::month_unknown)
    }

    /// `day_unknown(): Boolean`.
    ///
    /// Indicates whether day in month is unknown. If so, and month is known,
    /// the date is of the form `"YYYY-MM"` or `"YYYYMM"`.
    ///
    #[must_use]
    pub fn day_unknown(&self) -> bool {
        parse_date(&self.core.value)
            .is_none_or(super::iso8601_parser::ParsedIso8601Date::day_unknown)
    }

    /// `add` __alias__ `"+"` `(a_diff: Iso8601_duration) -> Iso8601_date`.
    ///
    /// Arithmetic addition of a duration to a date.
    ///
    /// Definite arithmetic per master06-time_types.adoc §Computational
    /// Functions and ADR-003 policy 1: the duration is treated as an exact
    /// quantity (`to_seconds()`, converting years/months via
    /// `Time_Definitions::AVERAGE_DAYS_IN_YEAR`/`AVERAGE_DAYS_IN_MONTH`),
    /// applied as an exact `jiff::SignedDuration` to the anchored civil
    /// date (ADR-003 policy 3: unknown month/day filled with 01), and the
    /// result truncated back to this date's original precision.
    ///
    /// PORT NOTE: the spec declares no error channel for the arithmetic
    /// functions; on an unparseable receiver (cannot occur for validated
    /// values) or an out-of-range result (year outside 0000–9999), the
    /// receiver is returned unchanged — the same fallback convention as
    /// `Iso8601Duration::divide`'s undefined-divisor case.
    #[must_use]
    pub fn add(&self, a_diff: &Iso8601Duration) -> Iso8601Date {
        self.definite_shift(a_diff.to_seconds())
    }

    /// `subtract` __alias__ `"-"` `(a_diff: Iso8601_duration) -> Iso8601_date`.
    ///
    /// Arithmetic subtraction of a duration from a date.
    ///
    /// Definite arithmetic; see `add` for semantics, policy, and the
    /// fallback PORT NOTE.
    #[must_use]
    pub fn subtract(&self, a_diff: &Iso8601Duration) -> Iso8601Date {
        self.definite_shift(-a_diff.to_seconds())
    }

    /// `diff` __alias__ `"-"` `(a_date: Iso8601_date) -> Iso8601_duration`.
    ///
    /// Difference of two dates.
    ///
    /// Definite arithmetic per ADR-003 policy 1: the difference of the two
    /// anchored civil dates (receiver minus argument), returned as a
    /// normalized `Iso8601_duration` in definite units only (days and
    /// below — never years/months, which are nominal units).
    ///
    /// PORT NOTE: the spec does not state the operand order; transcribed as
    /// receiver minus argument, matching the `"-"` operator alias reading
    /// (`self - a_date`). If either value is unparseable the result is the
    /// zero duration (see the fallback PORT NOTE on `add`).
    #[must_use]
    pub fn diff(&self, a_date: &Iso8601Date) -> Iso8601Duration {
        let seconds = match (
            parse_date(&self.core.value).and_then(anchored_jiff_date),
            parse_date(&a_date.core.value).and_then(anchored_jiff_date),
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

    /// `add_nominal` __alias__ `"++"` `(a_diff: Iso8601_duration) -> Iso8601_date`.
    ///
    /// Addition of nominal duration represented by `a_diff`. For example, a
    /// duration of `'P1Y'` means advance to the same date next year, with
    /// the exception of the date 29 February in a leap year, to which the
    /// addition of a nominal year will result in 28 February of the
    /// following year. Similarly, `'P1M'` is understood here as a nominal
    /// month, the addition of which will result in one of:
    /// * the same day in the following month, if it exists, or;
    /// * one or two days less where the following month is shorter, or;
    /// * in the case of adding a month to the date 31 Jan, the result will
    ///   be 28 Feb in a non-leap year (i.e. three less) and 29 Feb in a leap
    ///   year (i.e. two less).
    ///
    /// Nominal calendrical arithmetic per master06-time_types.adoc
    /// §Computational Functions and ADR-003 policy 2: years/months/weeks/
    /// days are applied as calendar units via `jiff::Span` on the anchored
    /// civil value (jiff's end-of-month clamping is exactly the spec's
    /// "one or two days less where the following month is shorter"
    /// behaviour), sub-day components as exact time; the result is
    /// truncated back to this date's original precision. See the fallback
    /// PORT NOTE on `add`.
    #[must_use]
    pub fn add_nominal(&self, a_diff: &Iso8601Duration) -> Iso8601Date {
        self.nominal_shift(a_diff, false)
    }

    /// `subtract_nominal` __alias__ `"--"` `(a_diff: Iso8601_duration) -> Iso8601_date`.
    ///
    /// Subtraction of nominal duration represented by `a_diff`. See
    /// `add_nominal` for semantics, policy, and the fallback PORT NOTE.
    #[must_use]
    pub fn subtract_nominal(&self, a_diff: &Iso8601Duration) -> Iso8601Date {
        self.nominal_shift(a_diff, true)
    }

    /// Shared definite-arithmetic engine call: anchor (ADR-003 policy 3),
    /// apply the exact seconds as a `jiff::SignedDuration` at civil
    /// midnight, truncate back to the receiver's precision.
    fn definite_shift(&self, seconds: f64) -> Iso8601Date {
        let result = parse_date(&self.core.value).and_then(|parsed| {
            let anchored = anchored_jiff_date(parsed)?;
            let duration = signed_duration_from_seconds(seconds)?;
            let shifted = DateTime::from_parts(anchored, Time::midnight())
                .checked_add(duration)
                .ok()?;
            format_date_at_precision(parsed, shifted.date(), parsed.extended)
        });
        result.map_or_else(|| self.clone(), Self::from_value)
    }

    /// Shared nominal-arithmetic engine call: anchor, apply the duration's
    /// components as a calendar `jiff::Span` (negated for subtraction),
    /// truncate back to the receiver's precision.
    fn nominal_shift(&self, a_diff: &Iso8601Duration, negate: bool) -> Iso8601Date {
        let result = parse_date(&self.core.value).and_then(|parsed| {
            let anchored = anchored_jiff_date(parsed)?;
            let span = parse_duration(&a_diff.core.value)
                .as_ref()
                .and_then(nominal_span)?;
            let span = if negate { span.negate() } else { span };
            let shifted = DateTime::from_parts(anchored, Time::midnight())
                .checked_add(span)
                .ok()?;
            format_date_at_precision(parsed, shifted.date(), parsed.extended)
        });
        result.map_or_else(|| self.clone(), Self::from_value)
    }

    /// Internal constructor from an already-formatted value string.
    fn from_value(value: String) -> Iso8601Date {
        Iso8601Date {
            core: Iso8601TypeCore { value },
        }
    }
}

impl Any for Iso8601Date {
    fn is_equal(&self, other: &Self) -> bool {
        self.core == other.core
    }

    fn type_of(&self) -> String {
        "Iso8601_date".to_string()
    }
}

impl Ordered for Iso8601Date {
    /// `less_than` __alias__ `"<"` `(other: Iso8601_date) -> Boolean`.
    ///
    /// PORT NOTE: not itself declared on `Iso8601_date`'s per-class table —
    /// inherited abstractly from `Ordered` via `Temporal`. A faithful
    /// effector compares parsed year/month/day components (partial-aware,
    /// per the class's own `month_unknown`/`day_unknown` semantics), not a
    /// lexical `core.value` string comparison, since e.g.
    /// `"2007-04"` (partial) must still order correctly against
    /// `"2007-04-15"`.
    fn less_than(&self, other: &Self) -> bool {
        match (parse_date(&self.core.value), parse_date(&other.core.value)) {
            (Some(left), Some(right)) => {
                (left.year, left.month.unwrap_or(0), left.day.unwrap_or(0))
                    < (right.year, right.month.unwrap_or(0), right.day.unwrap_or(0))
            }
            _ => self.core.value < other.core.value,
        }
    }
}

impl Temporal for Iso8601Date {}

impl Iso8601Type for Iso8601Date {
    fn core(&self) -> &Iso8601TypeCore {
        &self.core
    }

    /// `as_string(): String`.
    ///
    /// Return the string value in extended format: a compact-form value
    /// (`"20040215"`) is reformatted with `-` separators at its original
    /// precision (`"2004-02-15"`), effecting the "in extended format"
    /// contract the `Iso8601Type::as_string` default cannot honour without
    /// parsing. An unparseable value is returned verbatim.
    fn as_string(&self) -> String {
        parse_date(&self.core.value)
            .and_then(|parsed| {
                let anchored = anchored_jiff_date(parsed)?;
                format_date_at_precision(parsed, anchored, true)
            })
            .unwrap_or_else(|| self.core.value.clone())
    }

    /// `is_partial(): Boolean` (effected).
    ///
    /// True if this date is partial, i.e. if days or more is missing.
    ///
    fn is_partial(&self) -> bool {
        self.month_unknown() || self.day_unknown()
    }

    /// `is_extended(): Boolean` (effected).
    ///
    /// True if this date uses `'-'` separators.
    ///
    fn is_extended(&self) -> bool {
        parse_date(&self.core.value).is_some_and(|parsed| parsed.extended)
    }
}

// PORT NOTE: `Time_Definitions` is not a Rust supertrait of `Iso8601Type`
// (see the multiple-inheritance reasoning in `iso8601_type.rs`); the four
// invariants below call `TimeDefinitions::*` directly instead, faithfully
// transcribing the spec's `Iso8601_date` `Invariants` block. Encoded here as
// free functions rather than a `Validate` impl, since the per-RM-transcription
// convention for a `Validate` trait (context + path + error accumulator) is
// scoped to `openehr-rm` invariants proper; foundation-types invariants over
// a value's own accessor methods are transcribed as plain boolean-returning
// methods mirroring the spec text directly, consistent with how
// `primitive_types::boolean` documents (rather than encodes as `Validate`)
// its own class invariants.
impl Iso8601Date {
    /// __`Year_valid`__: `valid_year (year)`.
    #[must_use]
    pub fn invariant_year_valid(&self) -> bool {
        TimeDefinitions::valid_year(self.year())
    }

    /// __`Month_valid`__: `not month_unknown implies valid_month (month)`.
    #[must_use]
    pub fn invariant_month_valid(&self) -> bool {
        self.month_unknown() || TimeDefinitions::valid_month(self.month())
    }

    /// __`Day_valid`__: `not day_unknown implies valid_day (year, month, day)`.
    #[must_use]
    pub fn invariant_day_valid(&self) -> bool {
        self.day_unknown() || TimeDefinitions::valid_day(self.year(), self.month(), self.day())
    }

    /// __`Partial_validity`__: `month_unknown implies day_unknown`.
    #[must_use]
    pub fn invariant_partial_validity(&self) -> bool {
        !self.month_unknown() || self.day_unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> Iso8601Date {
        Iso8601Date {
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
        // Spec's own P1M example: nominal lands on the same day next month,
        // definite applies exactly 30.42 days (Feb 15 + 30 days = Mar 16 in
        // the 2004 leap year; the 0.42-day remainder stays sub-day and is
        // truncated away at date precision).
        let d = date("2004-02-15");
        assert_eq!(d.add_nominal(&duration("P1M")).core.value, "2004-03-15");
        assert_eq!(d.add(&duration("P1M")).core.value, "2004-03-16");
    }

    #[test]
    fn definite_and_nominal_year_addition_diverge_across_leap_day() {
        // 2004 is a leap year: definite P1Y = 365.24 days lands one day
        // short of the nominal same-date-next-year.
        let d = date("2004-02-15");
        assert_eq!(d.add(&duration("P1Y")).core.value, "2005-02-14");
        assert_eq!(d.add_nominal(&duration("P1Y")).core.value, "2005-02-15");
    }

    #[test]
    fn nominal_month_addition_clamps_to_end_of_month() {
        assert_eq!(
            date("2004-01-31").add_nominal(&duration("P1M")).core.value,
            "2004-02-29"
        );
        assert_eq!(
            date("2003-01-31").add_nominal(&duration("P1M")).core.value,
            "2003-02-28"
        );
    }

    #[test]
    fn nominal_year_addition_clamps_leap_day() {
        // Spec text: 29 Feb of a leap year ++ P1Y = 28 Feb of the next year.
        assert_eq!(
            date("2004-02-29").add_nominal(&duration("P1Y")).core.value,
            "2005-02-28"
        );
    }

    #[test]
    fn nominal_subtraction_clamps_too() {
        assert_eq!(
            date("2004-03-31")
                .subtract_nominal(&duration("P1M"))
                .core
                .value,
            "2004-02-29"
        );
    }

    #[test]
    fn definite_day_and_week_arithmetic_is_exact() {
        assert_eq!(
            date("2004-02-28").add(&duration("P1D")).core.value,
            "2004-02-29"
        );
        assert_eq!(
            date("2004-02-28").add(&duration("P2D")).core.value,
            "2004-03-01"
        );
        assert_eq!(
            date("2004-02-15").add(&duration("P1W")).core.value,
            "2004-02-22"
        );
        assert_eq!(
            date("2004-03-01").subtract(&duration("P1D")).core.value,
            "2004-02-29"
        );
    }

    #[test]
    fn partial_precision_is_anchored_then_truncated() {
        // "2004-02" anchors to 2004-02-01, shifts, and keeps YYYY-MM.
        assert_eq!(
            date("2004-02").add_nominal(&duration("P1M")).core.value,
            "2004-03"
        );
        assert_eq!(
            date("2004").add_nominal(&duration("P1Y")).core.value,
            "2005"
        );
        // Compact partials keep the compact form.
        assert_eq!(
            date("200402").add_nominal(&duration("P1M")).core.value,
            "200403"
        );
        // Definite month addition on a partial: 2004-02-01 + 30.42 days
        // lands on 2004-03-02, truncated back to month precision.
        assert_eq!(date("2004-02").add(&duration("P1M")).core.value, "2004-03");
    }

    #[test]
    fn diff_returns_definite_units_and_is_antisymmetric() {
        let later = date("2004-03-16");
        let earlier = date("2004-02-15");
        assert_eq!(later.diff(&earlier).core.value, "P30D");
        assert_eq!(earlier.diff(&later).core.value, "-P30D");
        // Anchored partials: 2004-01-01 minus 2003-01-01 = 365 days (2003
        // is not a leap year); never expressed in years/months.
        assert_eq!(date("2004").diff(&date("2003")).core.value, "P365D");
        assert_eq!(date("2005").diff(&date("2004")).core.value, "P366D");
    }

    #[test]
    fn out_of_range_result_falls_back_to_receiver() {
        let d = date("9999-12-31");
        assert_eq!(d.add(&duration("P2D")).core.value, "9999-12-31");
        let d = date("0000");
        assert_eq!(d.subtract(&duration("P1Y")).core.value, "0000");
    }

    #[test]
    fn as_string_reformats_compact_to_extended() {
        assert_eq!(date("20040215").as_string(), "2004-02-15");
        assert_eq!(date("200402").as_string(), "2004-02");
        assert_eq!(date("2004").as_string(), "2004");
        assert_eq!(date("2004-02-15").as_string(), "2004-02-15");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso8601_date.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / iso8601_date.adoc §Iso8601_date Class
//   confidence: high
//   todos: 0
//   note: string accessors, partiality, extended-form detection, and ordering delegate to the shared BASE ISO 8601 parser; add/subtract/diff (definite, averages 365.24/30.42 as exact SignedDuration) and add_nominal/subtract_nominal (jiff Span calendar units with end-of-month clamping) implemented per ADR-003 policies 1-3 via iso8601_arithmetic.rs, with partial-precision anchoring + truncation and a documented return-receiver fallback for out-of-range results; as_string now effects the extended-format contract. The four spec invariants remain plain boolean methods calling TimeDefinitions::* directly per the iso8601_type.rs multiple-inheritance note.
// ─────────────────────────────────────────────
