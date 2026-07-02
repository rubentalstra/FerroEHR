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
use crate::time::iso8601_duration::Iso8601Duration;
use crate::time::iso8601_parser::parse_date;
use crate::time::iso8601_timezone::Iso8601Timezone;
use crate::time::iso8601_type::{Iso8601Type, Iso8601TypeCore};
use crate::time::temporal::Temporal;
use crate::time::time_definitions::TimeDefinitions;
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
        parse_date(&self.core.value).is_none_or(|parsed| parsed.month_unknown())
    }

    /// `day_unknown(): Boolean`.
    ///
    /// Indicates whether day in month is unknown. If so, and month is known,
    /// the date is of the form `"YYYY-MM"` or `"YYYYMM"`.
    ///
    #[must_use]
    pub fn day_unknown(&self) -> bool {
        parse_date(&self.core.value).is_none_or(|parsed| parsed.day_unknown())
    }

    /// `add` __alias__ `"+"` `(a_diff: Iso8601_duration) -> Iso8601_date`.
    ///
    /// Arithmetic addition of a duration to a date.
    ///
    /// TODO(port): definite-arithmetic addition; deferred to the internal
    /// engine (see the `time_types` chapter's "definite vs nominal" split —
    /// this is the `_add_()` definite form, contrast `add_nominal` below).
    #[must_use]
    pub fn add(&self, a_diff: &Iso8601Duration) -> Iso8601Date {
        let _ = a_diff;
        todo!("Iso8601Date::add: definite-duration addition deferred to the internal engine")
    }

    /// `subtract` __alias__ `"-"` `(a_diff: Iso8601_duration) -> Iso8601_date`.
    ///
    /// Arithmetic subtraction of a duration from a date.
    ///
    /// TODO(port): definite-arithmetic subtraction; deferred to the
    /// internal engine.
    #[must_use]
    pub fn subtract(&self, a_diff: &Iso8601Duration) -> Iso8601Date {
        let _ = a_diff;
        todo!(
            "Iso8601Date::subtract: definite-duration subtraction deferred to the internal engine"
        )
    }

    /// `diff` __alias__ `"-"` `(a_date: Iso8601_date) -> Iso8601_duration`.
    ///
    /// Difference of two dates.
    ///
    /// TODO(port): deferred to the internal engine.
    #[must_use]
    pub fn diff(&self, a_date: &Iso8601Date) -> Iso8601Duration {
        let _ = a_date;
        todo!("Iso8601Date::diff: deferred to the internal engine")
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
    /// TODO(port): nominal calendrical arithmetic; deferred to the internal
    /// engine (candidate for a direct `jiff` calendar-arithmetic bridge at
    /// P17, since this is exactly the "everyday" leap/short-month handling
    /// `jiff` already implements).
    #[must_use]
    pub fn add_nominal(&self, a_diff: &Iso8601Duration) -> Iso8601Date {
        let _ = a_diff;
        todo!("Iso8601Date::add_nominal: nominal-duration addition deferred to the internal engine")
    }

    /// `subtract_nominal` __alias__ `"--"` `(a_diff: Iso8601_duration) -> Iso8601_date`.
    ///
    /// Subtraction of nominal duration represented by `a_diff`. See
    /// `add_nominal` for semantics.
    ///
    /// TODO(port): nominal calendrical arithmetic; deferred to the internal
    /// engine.
    #[must_use]
    pub fn subtract_nominal(&self, a_diff: &Iso8601Duration) -> Iso8601Date {
        let _ = a_diff;
        todo!(
            "Iso8601Date::subtract_nominal: nominal-duration subtraction deferred to the internal engine"
        )
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

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso8601_date.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / iso8601_date.adoc §Iso8601_date Class
//   confidence: medium
//   todos: 5
//   note: string accessors, partiality, extended-form detection, and ordering delegate to the shared BASE ISO 8601 parser; arithmetic bodies remain TODO(port) because partial-date calendar arithmetic needs an explicit policy beyond the accessor grammar. The four spec invariants are transcribed as plain boolean methods (not a Validate impl, out of scope for foundation-types values) calling TimeDefinitions::* directly per the iso8601_type.rs multiple-inheritance note.
// ─────────────────────────────────────────────
