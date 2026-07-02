//! `Iso8601_duration` — an ISO 8601 duration.
//!
//! openEHR class: `Iso8601_duration`, package `base.foundation_types.time`.
//! Inherits: `Iso8601_type`.
//!
//! Represents an ISO 8601 duration, which may have multiple parts from
//! years down to seconds. The `value` attribute is a String in the format:
//! * `P[nnY][nnM][nnW][nnD][T[nnH][nnM][nnS]]`
//!
//! NOTE (spec): two deviations from ISO 8601 are supported — a negative
//! sign, and allowing the `W` designator to be mixed with other
//! designators.
//!
//! # String-value representation, not a resolved timespan
//!
//! Models an ISO 8601 duration *string* with nominal (year/month) and
//! definite (week/day/hour/minute/second) components, not a resolved
//! fixed-length timespan. See the module-level doc on `iso8601_type.rs` for
//! the full rationale and the jiff-bridging plan for P17.
use crate::primitive_types::any::Any;
use crate::primitive_types::ordered::Ordered;
use crate::time::iso8601_parser::{duration_from_seconds_string, parse_duration};
use crate::time::iso8601_type::{Iso8601Type, Iso8601TypeCore};
use crate::time::temporal::Temporal;
use serde::{Deserialize, Serialize};

/// `Iso8601_duration` embeds the `Iso8601_type` parent state (`value:
/// String`) via `Iso8601TypeCore`, per ADR-001 §3. This struct declares no
/// attributes of its own beyond the inherited `value`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Iso8601Duration {
    /// Embedded `Iso8601_type.value: String`.
    ///
    /// `#[serde(flatten)]` per P4: `DV_DURATION` (the sole embedder of this
    /// struct, via `#[serde(flatten)] pub iso8601: Iso8601Duration` on
    /// `openehr_rm::data_types::date_time::dv_duration::DvDuration`)
    /// declares `value` as a canonical top-level `String` property in its
    /// ITS-JSON schema, not a nested `{"core": {"value": ...}}` object;
    /// without flattening here, the double-embedding (`DvDuration.iso8601`
    /// flattened, but `core` not) would still emit the field nested one
    /// level too deep.
    #[serde(flatten)]
    pub core: Iso8601TypeCore,
}

impl Iso8601Duration {
    /// `years(): Integer`.
    ///
    /// Number of years in the `value`, i.e. the number preceding the `'Y'`
    /// in the `'YMD'` part, if one exists.
    ///
    #[must_use]
    pub fn years(&self) -> i32 {
        parse_duration(&self.core.value).map_or(0, |parsed| parsed.years)
    }

    /// `months(): Integer`.
    ///
    /// Number of months in the `value`, i.e. the value preceding the `'M'`
    /// in the `'YMD'` part, if one exists.
    ///
    #[must_use]
    pub fn months(&self) -> i32 {
        parse_duration(&self.core.value).map_or(0, |parsed| parsed.months)
    }

    /// `days(): Integer`.
    ///
    /// Number of days in the `value`, i.e. the number preceding the `'D'`
    /// in the `'YMD'` part, if one exists.
    ///
    #[must_use]
    pub fn days(&self) -> i32 {
        parse_duration(&self.core.value).map_or(0, |parsed| parsed.days)
    }

    /// `hours(): Integer`.
    ///
    /// Number of hours in the `value`, i.e. the number preceding the `'H'`
    /// in the `'HMS'` part, if one exists.
    ///
    #[must_use]
    pub fn hours(&self) -> i32 {
        parse_duration(&self.core.value).map_or(0, |parsed| parsed.hours)
    }

    /// `minutes(): Integer`.
    ///
    /// Number of minutes in the `value`, i.e. the number preceding the
    /// `'M'` in the `'HMS'` part, if one exists.
    ///
    #[must_use]
    pub fn minutes(&self) -> i32 {
        parse_duration(&self.core.value).map_or(0, |parsed| parsed.minutes)
    }

    /// `seconds(): Integer`.
    ///
    /// Number of seconds in the `value`, i.e. the integer number preceding
    /// the `'S'` in the `'HMS'` part, if one exists.
    ///
    #[must_use]
    pub fn seconds(&self) -> i32 {
        parse_duration(&self.core.value).map_or(0, |parsed| parsed.seconds)
    }

    /// `fractional_seconds(): Real`.
    ///
    /// Fractional seconds in the `value`, i.e. the decimal part of the
    /// number preceding the `'S'` in the `'HMS'` part, if one exists.
    ///
    #[must_use]
    pub fn fractional_seconds(&self) -> f64 {
        parse_duration(&self.core.value).map_or(0.0, |parsed| parsed.fractional_seconds)
    }

    /// `weeks(): Integer`.
    ///
    /// Number of weeks in the `value`, i.e. the value preceding the `W`, if
    /// one exists.
    ///
    #[must_use]
    pub fn weeks(&self) -> i32 {
        parse_duration(&self.core.value).map_or(0, |parsed| parsed.weeks)
    }

    /// `is_decimal_sign_comma(): Boolean`.
    ///
    /// True if this time has a decimal part indicated by `,` (comma) rather
    /// than `.` (period).
    ///
    #[must_use]
    pub fn is_decimal_sign_comma(&self) -> bool {
        parse_duration(&self.core.value).is_some_and(|parsed| parsed.decimal_sign_comma)
    }

    /// `to_seconds(): Real`.
    ///
    /// Total number of seconds equivalent (including fractional) of entire
    /// duration. Where non-definite elements such as year and month (i.e.
    /// `'Y'` and `'M'`) are included, the corresponding 'average' durations
    /// from `Time_Definitions` are used to compute the result.
    ///
    #[must_use]
    pub fn to_seconds(&self) -> f64 {
        parse_duration(&self.core.value).map_or(0.0, |parsed| parsed.to_seconds())
    }

    /// `as_string(): String`.
    ///
    /// Return the duration string value.
    ///
    /// PORT NOTE: unlike `Iso8601Date`/`Iso8601Time`/`Iso8601DateTime`,
    /// which rely on the `Iso8601Type::as_string` default (delegating to
    /// the raw stored `value`), this accessor is individually re-declared
    /// on `Iso8601_duration`'s own per-class table with the description
    /// "Return the duration string value" (not "...in extended format"
    /// like its siblings) — `Iso8601_duration::is_extended` is `(effected)`
    /// to unconditionally return `true` (see below), so there is no
    /// compact-vs-extended reformatting distinction to make here in the
    /// first place; the `Iso8601Type::as_string` default is therefore
    /// already exactly correct for this type without an override. No
    /// override written; documented here so a reviewer does not need to
    /// re-derive why one is absent.
    #[must_use]
    pub fn as_string(&self) -> String {
        Iso8601Type::as_string(self)
    }

    /// `add` __alias__ `"+"` `(a_val: Iso8601_duration) -> Iso8601_duration`.
    ///
    /// Arithmetic addition of a duration to a duration, via conversion to
    /// seconds, using `Time_Definitions::AVERAGE_DAYS_IN_YEAR` and
    /// `Time_Definitions::AVERAGE_DAYS_IN_MONTH`.
    ///
    #[must_use]
    pub fn add(&self, a_val: &Iso8601Duration) -> Iso8601Duration {
        Self::from_seconds(self.to_seconds() + a_val.to_seconds())
    }

    /// `subtract` __alias__ `"-"` `(a_val: Iso8601_duration) -> Iso8601_duration`.
    ///
    /// Arithmetic subtraction of a duration from a duration, via conversion
    /// to seconds, using `Time_Definitions::AVERAGE_DAYS_IN_YEAR` and
    /// `Time_Definitions::AVERAGE_DAYS_IN_MONTH`.
    ///
    #[must_use]
    pub fn subtract(&self, a_val: &Iso8601Duration) -> Iso8601Duration {
        Self::from_seconds(self.to_seconds() - a_val.to_seconds())
    }

    /// `multiply` __alias__ `"*"` `(a_val: Real) -> Iso8601_duration`.
    ///
    /// Arithmetic multiplication a duration by a number.
    ///
    #[must_use]
    pub fn multiply(&self, a_val: f64) -> Iso8601Duration {
        Self::from_seconds(self.to_seconds() * a_val)
    }

    /// `divide` __alias__ `"/"` `(a_val: Real) -> Iso8601_duration`.
    ///
    /// Arithmetic division of a duration by a number.
    ///
    /// PORT NOTE: the spec gives no error channel or precondition for a zero
    /// or non-finite divisor. This implementation preserves the receiver for
    /// undefined divisors rather than fabricating an invalid ISO 8601 string.
    #[must_use]
    pub fn divide(&self, a_val: f64) -> Iso8601Duration {
        if a_val == 0.0 || !a_val.is_finite() {
            return self.clone();
        }
        Self::from_seconds(self.to_seconds() / a_val)
    }

    /// `negative` __alias__ `"-"` `(): Iso8601_duration`.
    ///
    /// Generate negative of current duration value.
    ///
    /// PORT NOTE: this is the concrete, `Iso8601_duration`-specific
    /// `negative()` from its own per-class table (a unary duration
    /// negation, returning `Iso8601_duration`), distinct from
    /// `primitive_types::numeric::Numeric::negative` (a same-type
    /// `Self -> Self` operator on `Numeric`-family types). `Iso8601_duration`
    /// does not implement the `Numeric` trait — it is not in the
    /// `Ordered_Numeric` hierarchy in this spec chapter, even though it
    /// supports arithmetic-like operators — so there is no name collision
    /// with a trait method to disambiguate here; both are simply named
    /// `negative` because the spec itself reuses the operator alias `"-"`
    /// across unrelated classes.
    ///
    #[must_use]
    pub fn negative(&self) -> Iso8601Duration {
        Self::from_seconds(-self.to_seconds())
    }

    fn from_seconds(seconds: f64) -> Iso8601Duration {
        let value = if seconds.is_finite() {
            duration_from_seconds_string(seconds)
        } else {
            "PT0S".to_string()
        };
        Iso8601Duration {
            core: Iso8601TypeCore { value },
        }
    }
}

impl Any for Iso8601Duration {
    fn is_equal(&self, other: &Self) -> bool {
        self.core == other.core
    }

    fn type_of(&self) -> String {
        "Iso8601_duration".to_string()
    }
}

impl Ordered for Iso8601Duration {
    /// `less_than` __alias__ `"<"` `(other: Iso8601_duration) -> Boolean`.
    ///
    /// PORT NOTE: not itself declared on `Iso8601_duration`'s per-class
    /// table — inherited abstractly from `Ordered` via `Temporal`. A
    /// faithful effector compares `to_seconds()` results (the spec's own
    /// natural total-order comparator for durations, given
    /// `to_seconds`'s existence and description).
    fn less_than(&self, other: &Self) -> bool {
        self.to_seconds() < other.to_seconds()
    }
}

impl Temporal for Iso8601Duration {}

impl Iso8601Type for Iso8601Duration {
    /// `is_extended(): Boolean` (effected).
    ///
    /// Returns `true`, per the spec table verbatim ("Returns True").
    fn is_extended(&self) -> bool {
        true
    }

    /// `is_partial(): Boolean` (effected).
    ///
    /// Returns `false`, per the spec table verbatim ("Returns False").
    fn is_partial(&self) -> bool {
        false
    }

    fn core(&self) -> &Iso8601TypeCore {
        &self.core
    }
}

// PORT NOTE: see the equivalent note in `iso8601_date.rs` — `Time_Definitions`
// is not a Rust supertrait here; `to_seconds` (above) is the only member
// that references `Time_Definitions` constants, not these invariants.
// Encoded as plain boolean-returning methods rather than a `Validate` impl.
impl Iso8601Duration {
    /// __`Years_valid`__: `years >= 0`.
    #[must_use]
    pub fn invariant_years_valid(&self) -> bool {
        self.years() >= 0
    }

    /// __`Months_valid`__: `months >= 0`.
    #[must_use]
    pub fn invariant_months_valid(&self) -> bool {
        self.months() >= 0
    }

    /// __`Weeks_valid`__: `weeks >= 0`.
    #[must_use]
    pub fn invariant_weeks_valid(&self) -> bool {
        self.weeks() >= 0
    }

    /// __`Days_valid`__: `days >= 0`.
    #[must_use]
    pub fn invariant_days_valid(&self) -> bool {
        self.days() >= 0
    }

    /// __`Hours_valid`__: `hours >= 0`.
    #[must_use]
    pub fn invariant_hours_valid(&self) -> bool {
        self.hours() >= 0
    }

    /// __`Minutes_valid`__: `minutes >= 0`.
    #[must_use]
    pub fn invariant_minutes_valid(&self) -> bool {
        self.minutes() >= 0
    }

    /// __`Seconds_valid`__: `seconds >= 0`.
    #[must_use]
    pub fn invariant_seconds_valid(&self) -> bool {
        self.seconds() >= 0
    }

    /// __`Fractional_second_valid`__: `fractional_second >= 0.0 and
    /// fractional_second < 1.0`.
    ///
    /// PORT NOTE: the invariant's own condition text names
    /// `fractional_second` (singular), but this class's only matching
    /// accessor is `fractional_seconds()` (plural) — transcribed calling
    /// `fractional_seconds()`, the only member on this class's own table
    /// the invariant could plausibly refer to; the singular/plural
    /// mismatch is presumably editorial, consistent with this class's
    /// several other minor wording inconsistencies noted elsewhere in this
    /// file.
    #[must_use]
    pub fn invariant_fractional_second_valid(&self) -> bool {
        let fs = self.fractional_seconds();
        (0.0..1.0).contains(&fs)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso8601_duration.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / iso8601_duration.adoc §Iso8601_duration Class
//   confidence: medium
//   todos: 0
//   note: component accessors, to_seconds, ordering, and duration arithmetic delegate to the shared BASE ISO 8601 parser; arithmetic emits a seconds-only ISO duration after applying the spec's average year/month conversion. as_string relies on the Iso8601Type default with no override (documented why); invariant_fractional_second_valid's singular/plural naming mismatch against fractional_seconds() flagged, not silently reconciled. P4: added #[serde(flatten)] on `core` so DV_DURATION.value sits at the top level per the ITS-JSON schema, matching DvDuration's own #[serde(flatten)] on its `iso8601` field.
// ─────────────────────────────────────────────
