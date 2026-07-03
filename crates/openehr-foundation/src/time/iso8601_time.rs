//! `Iso8601_time` — an ISO 8601 time, including partial and extended forms.
//!
//! openEHR class: `Iso8601_time`, package `base.foundation_types.time`.
//! Inherits: `Iso8601_type`.
//!
//! Represents an ISO 8601 time, including partial and extended forms. Value
//! may be:
//! * `hh:mm:ss[(,|.)sss][Z|±hh[:mm]]` (extended, preferred) or
//! * `hhmmss[(,|.)sss][Z|±hh[mm]]` (compact)
//! * or a partial invariant.
//!
//! See `Time_Definitions::valid_iso8601_time` for validity.
//!
//! NOTE (spec): a small deviation to the ISO 8601:2004 standard in this
//! class is that the time `24:00:00` is not allowed, for consistency with
//! `Iso8601_date_time`.
//!
//! # String-value representation, not a resolved instant
//!
//! Models an ISO 8601 string value with partial precision (e.g. `"10:30"`,
//! meaning "hour 10, minute 30, second unknown"), not a resolved time
//! instant. See the module-level doc on `iso8601_type.rs` for the full
//! rationale and the jiff-bridging plan for P17.
use crate::primitive_types::any::Any;
use crate::primitive_types::ordered::Ordered;
use crate::time::iso8601_arithmetic::{
    definite_duration_string_from_seconds, format_time_at_precision, signed_duration_from_seconds,
};
use crate::time::iso8601_duration::Iso8601Duration;
use crate::time::iso8601_parser::parse_time;
use crate::time::iso8601_timezone::Iso8601Timezone;
use crate::time::iso8601_type::{Iso8601Type, Iso8601TypeCore};
use crate::time::temporal::Temporal;
use crate::time::time_definitions::TimeDefinitions;
use serde::{Deserialize, Serialize};

/// `Iso8601_time` embeds the `Iso8601_type` parent state (`value: String`)
/// via `Iso8601TypeCore`, per ADR-001 §3. This struct declares no attributes
/// of its own beyond the inherited `value`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Iso8601Time {
    /// Embedded `Iso8601_type.value: String`.
    #[serde(flatten)]
    pub core: Iso8601TypeCore,
}

impl Iso8601Time {
    /// `hour(): Integer`.
    ///
    /// Extract the hour part of the date/time as an Integer.
    ///
    #[must_use]
    pub fn hour(&self) -> i32 {
        parse_time(&self.core.value).map_or(0, |parsed| parsed.hour)
    }

    /// `minute(): Integer`.
    ///
    /// Extract the minute part of the time as an Integer, or return 0 if
    /// not present.
    ///
    #[must_use]
    pub fn minute(&self) -> i32 {
        parse_time(&self.core.value)
            .map_or(0, super::iso8601_parser::ParsedIso8601Time::minute_value)
    }

    /// `second(): Integer`.
    ///
    /// Extract the integral seconds part of the time (i.e. prior to any
    /// decimal sign) as an Integer, or return 0 if not present.
    ///
    #[must_use]
    pub fn second(&self) -> i32 {
        parse_time(&self.core.value)
            .map_or(0, super::iso8601_parser::ParsedIso8601Time::second_value)
    }

    /// `fractional_second(): Real`.
    ///
    /// Pre: `not second_unknown`.
    ///
    /// Extract the fractional seconds part of the time (i.e. following to
    /// any decimal sign) as a Real, or return 0.0 if not present.
    ///
    #[must_use]
    pub fn fractional_second(&self) -> f64 {
        parse_time(&self.core.value).map_or(
            0.0,
            super::iso8601_parser::ParsedIso8601Time::fractional_second,
        )
    }

    /// `timezone(): Iso8601_timezone`.
    ///
    /// Timezone; may be Void.
    ///
    #[must_use]
    pub fn timezone(&self) -> Option<Iso8601Timezone> {
        parse_time(&self.core.value)
            .and_then(|parsed| parsed.timezone)
            .map(|timezone| Iso8601Timezone {
                core: Iso8601TypeCore {
                    value: timezone.as_iso8601_string(),
                },
            })
    }

    /// `minute_unknown(): Boolean`.
    ///
    /// Indicates whether minute is unknown. If so, the time is of the form
    /// `"hh"`.
    ///
    #[must_use]
    pub fn minute_unknown(&self) -> bool {
        parse_time(&self.core.value)
            .is_none_or(super::iso8601_parser::ParsedIso8601Time::minute_unknown)
    }

    /// `second_unknown(): Boolean`.
    ///
    /// Indicates whether second is unknown. If so and month is known, the
    /// time is of the form `"hh:mm"` or `"hhmm"`.
    ///
    /// PORT NOTE: the spec description text says "if so **and month is
    /// known**", which is almost certainly a copy-paste artifact from the
    /// analogous `Iso8601_date_time.second_unknown` description (a bare
    /// `Iso8601_time` has no month component at all) — transcribed
    /// verbatim rather than silently corrected.
    ///
    #[must_use]
    pub fn second_unknown(&self) -> bool {
        parse_time(&self.core.value)
            .is_none_or(super::iso8601_parser::ParsedIso8601Time::second_unknown)
    }

    /// `is_decimal_sign_comma(): Boolean`.
    ///
    /// True if this time has a decimal part indicated by `','` (comma)
    /// rather than `'.'` (period).
    ///
    #[must_use]
    pub fn is_decimal_sign_comma(&self) -> bool {
        parse_time(&self.core.value).is_some_and(|parsed| parsed.decimal_sign_comma)
    }

    /// `has_fractional_second(): Boolean`.
    ///
    /// True if the `fractional_second` part is significant (i.e. even if =
    /// 0.0).
    ///
    #[must_use]
    pub fn has_fractional_second(&self) -> bool {
        parse_time(&self.core.value).is_some_and(|parsed| parsed.has_fractional_second)
    }

    /// `add` __alias__ `"+"` `(a_diff: Iso8601_duration) -> Iso8601_time`.
    ///
    /// Arithmetic addition of a duration to a time.
    ///
    /// Definite arithmetic per master06-time_types.adoc §Computational
    /// Functions and ADR-003 policy 1: the duration is an exact quantity
    /// (`to_seconds()`, averages for years/months), applied to the anchored
    /// civil time (unknown minute/second/fraction filled with 0, ADR-003
    /// policy 3) and the result truncated back to this time's original
    /// precision, with any timezone text preserved verbatim.
    ///
    /// PORT NOTE: a time-of-day has no date to carry overflow into, so the
    /// pending clock-wrapping policy is resolved as **wrap modulo 24h**
    /// (civil clock arithmetic, `jiff::civil::Time::wrapping_add`):
    /// `23:30 + PT1H = 00:30`. The spec is silent on overflow; wrapping is
    /// the only closed reading for a bare time value.
    ///
    /// PORT NOTE: the spec declares no error channel; on an unparseable
    /// receiver or duration, the receiver is returned unchanged — the same
    /// fallback convention as `Iso8601Duration::divide`.
    #[must_use]
    pub fn add(&self, a_diff: &Iso8601Duration) -> Iso8601Time {
        self.definite_shift(a_diff.to_seconds())
    }

    /// `subtract` __alias__ `"-"` `(a_diff: Iso8601_duration) -> Iso8601_time`.
    ///
    /// Arithmetic subtraction of a duration from a time.
    ///
    /// Definite arithmetic; see `add` for semantics, the wrap-modulo-24h
    /// policy, and the fallback PORT NOTE.
    #[must_use]
    pub fn subtract(&self, a_diff: &Iso8601Duration) -> Iso8601Time {
        self.definite_shift(-a_diff.to_seconds())
    }

    /// `diff` __alias__ `"-"` `(a_time: Iso8601_time) -> Iso8601_duration`.
    ///
    /// Difference of two times.
    ///
    /// Definite arithmetic per ADR-003 policy 1: the difference of the two
    /// anchored times-of-day (receiver minus argument, fractional seconds
    /// included), returned as a normalized `Iso8601_duration` in definite
    /// units. Timezone offsets are not applied — arithmetic is civil
    /// (ADR-003 policy 3).
    ///
    /// PORT NOTE: operand order is receiver minus argument; see
    /// `Iso8601Date::diff`. Unparseable operands yield the zero duration.
    #[must_use]
    pub fn diff(&self, a_time: &Iso8601Time) -> Iso8601Duration {
        let seconds = match (parse_time(&self.core.value), parse_time(&a_time.core.value)) {
            (Some(left), Some(right)) => {
                left.seconds_since_midnight() - right.seconds_since_midnight()
            }
            _ => 0.0,
        };
        Iso8601Duration {
            core: Iso8601TypeCore {
                value: definite_duration_string_from_seconds(seconds),
            },
        }
    }

    /// Shared definite-arithmetic engine call: anchor (ADR-003 policy 3),
    /// apply the exact seconds with wrap-modulo-24h clock semantics,
    /// truncate back to the receiver's precision.
    fn definite_shift(&self, seconds: f64) -> Iso8601Time {
        let result = parse_time(&self.core.value).and_then(|parsed| {
            let anchored = parsed.as_jiff_time()?;
            let duration = signed_duration_from_seconds(seconds)?;
            let shifted = anchored.wrapping_add(duration);
            Some(format_time_at_precision(
                parsed,
                shifted,
                parsed.extended,
                false,
            ))
        });
        result.map_or_else(
            || self.clone(),
            |value| Iso8601Time {
                core: Iso8601TypeCore { value },
            },
        )
    }
}

impl Any for Iso8601Time {
    fn is_equal(&self, other: &Self) -> bool {
        self.core == other.core
    }

    fn type_of(&self) -> String {
        "Iso8601_time".to_string()
    }
}

impl Ordered for Iso8601Time {
    /// `less_than` __alias__ `"<"` `(other: Iso8601_time) -> Boolean`.
    ///
    /// PORT NOTE: not itself declared on `Iso8601_time`'s per-class table —
    /// inherited abstractly from `Ordered` via `Temporal`. A faithful
    /// effector compares parsed, partial-aware hour/minute/second
    /// components (see `Iso8601Date::less_than` for the analogous note).
    fn less_than(&self, other: &Self) -> bool {
        match (parse_time(&self.core.value), parse_time(&other.core.value)) {
            (Some(left), Some(right)) => {
                left.seconds_since_midnight() < right.seconds_since_midnight()
            }
            _ => self.core.value < other.core.value,
        }
    }
}

impl Temporal for Iso8601Time {}

impl Iso8601Type for Iso8601Time {
    fn core(&self) -> &Iso8601TypeCore {
        &self.core
    }

    /// `as_string(): String`.
    ///
    /// Return the string value in extended format: a compact-form value
    /// (`"103005+0230"`) is reformatted with `:` separators (and the
    /// canonical `±hh:mm` timezone form) at its original precision,
    /// effecting the "in extended format" contract the
    /// `Iso8601Type::as_string` default cannot honour without parsing. An
    /// unparseable value is returned verbatim.
    fn as_string(&self) -> String {
        parse_time(&self.core.value)
            .and_then(|parsed| {
                let anchored = parsed.as_jiff_time()?;
                Some(format_time_at_precision(parsed, anchored, true, true))
            })
            .unwrap_or_else(|| self.core.value.clone())
    }

    /// `is_partial(): Boolean` (effected).
    ///
    /// True if this time is partial, i.e. if seconds or more is missing.
    ///
    fn is_partial(&self) -> bool {
        self.minute_unknown() || self.second_unknown()
    }

    /// `is_extended(): Boolean` (effected).
    ///
    /// True if this time uses `'-'`, `':'` separators.
    ///
    fn is_extended(&self) -> bool {
        parse_time(&self.core.value).is_some_and(|parsed| parsed.extended)
    }
}

// PORT NOTE: see the equivalent note in `iso8601_date.rs` — `Time_Definitions`
// is not a Rust supertrait here; these invariants call `TimeDefinitions::*`
// directly, and are encoded as plain boolean-returning methods rather than a
// `Validate` impl (see the equivalent note in `iso8601_date.rs`).
impl Iso8601Time {
    /// __`Hour_valid`__: `valid_hour(hour, minute, second)`.
    #[must_use]
    pub fn invariant_hour_valid(&self) -> bool {
        TimeDefinitions::valid_hour(self.hour(), self.minute(), self.second())
    }

    /// __`Minute_valid`__: `not minute_unknown implies valid_minute (minute)`.
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

    /// __`Partial_validity`__: `minute_unknown implies second_unknown`.
    #[must_use]
    pub fn invariant_partial_validity(&self) -> bool {
        !self.minute_unknown() || self.second_unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(value: &str) -> Iso8601Time {
        Iso8601Time {
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
    fn definite_addition_within_the_day() {
        assert_eq!(
            time("10:30:00").add(&duration("PT1H30M")).core.value,
            "12:00:00"
        );
        assert_eq!(
            time("10:30:00").subtract(&duration("PT45M")).core.value,
            "09:45:00"
        );
    }

    #[test]
    fn addition_wraps_modulo_24h() {
        assert_eq!(time("23:30").add(&duration("PT1H")).core.value, "00:30");
        assert_eq!(
            time("00:15:00").subtract(&duration("PT30M")).core.value,
            "23:45:00"
        );
        // A whole definite day wraps back to the same clock time.
        assert_eq!(
            time("10:30:00").add(&duration("P1D")).core.value,
            "10:30:00"
        );
    }

    #[test]
    fn partial_precision_is_anchored_then_truncated() {
        // "10:30" anchors second to 0, shifts, keeps hh:mm precision.
        assert_eq!(time("10:30").add(&duration("PT90S")).core.value, "10:31");
        assert_eq!(time("10").add(&duration("PT1H")).core.value, "11");
        // Compact form is preserved.
        assert_eq!(time("1030").add(&duration("PT1H")).core.value, "1130");
    }

    #[test]
    fn timezone_text_is_preserved_verbatim() {
        assert_eq!(
            time("10:30:00+02:00").add(&duration("PT1H")).core.value,
            "11:30:00+02:00"
        );
        assert_eq!(
            time("10:30:00Z").add(&duration("PT1H")).core.value,
            "11:30:00Z"
        );
        assert_eq!(
            time("103000+0230").add(&duration("PT1H")).core.value,
            "113000+0230"
        );
    }

    #[test]
    fn fractional_seconds_stay_exact() {
        assert_eq!(
            time("10:30:05.250").add(&duration("PT0.5S")).core.value,
            "10:30:05.75"
        );
        // Comma decimal sign is preserved.
        assert_eq!(
            time("10:30:05,25").add(&duration("PT0.5S")).core.value,
            "10:30:05,75"
        );
    }

    #[test]
    fn diff_is_antisymmetric_and_keeps_fractions() {
        assert_eq!(
            time("12:00:00").diff(&time("10:30:00")).core.value,
            "PT1H30M"
        );
        assert_eq!(
            time("10:30:00").diff(&time("12:00:00")).core.value,
            "-PT1H30M"
        );
        assert_eq!(
            time("10:30:05.75").diff(&time("10:30:05.25")).core.value,
            "PT0.5S"
        );
    }

    #[test]
    fn as_string_reformats_compact_to_extended() {
        assert_eq!(time("103005").as_string(), "10:30:05");
        assert_eq!(time("103005.5+0230").as_string(), "10:30:05.5+02:30");
        assert_eq!(time("10:30:05Z").as_string(), "10:30:05Z");
        assert_eq!(time("1030").as_string(), "10:30");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso8601_time.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / iso8601_time.adoc §Iso8601_time Class
//   confidence: high
//   todos: 0
//   note: string accessors, partiality, extended-form detection, and ordering delegate to the shared BASE ISO 8601 parser; add/subtract/diff implemented per ADR-003 policies 1+3 via iso8601_arithmetic.rs, with the clock-wrapping policy resolved as wrap-modulo-24h (jiff civil Time::wrapping_add, PORT NOTE on add), partial-precision anchoring + truncation, and timezone text preserved verbatim; as_string now effects the extended-format contract. second_unknown's description text ("...and month is known") remains flagged as a published copy-paste artifact; invariants are plain boolean methods, not a Validate impl.
// ─────────────────────────────────────────────
