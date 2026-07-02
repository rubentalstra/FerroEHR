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
use crate::time::iso8601_duration::Iso8601Duration;
use crate::time::iso8601_timezone::Iso8601Timezone;
use crate::time::iso8601_type::{Iso8601Type, Iso8601TypeCore};
use crate::time::temporal::Temporal;
use crate::time::time_definitions::TimeDefinitions;

/// `Iso8601_time` embeds the `Iso8601_type` parent state (`value: String`)
/// via `Iso8601TypeCore`, per ADR-001 §3. This struct declares no attributes
/// of its own beyond the inherited `value`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iso8601Time {
    /// Embedded `Iso8601_type.value: String`.
    pub core: Iso8601TypeCore,
}

impl Iso8601Time {
    /// `hour(): Integer`.
    ///
    /// Extract the hour part of the date/time as an Integer.
    ///
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn hour(&self) -> i32 {
        todo!("Iso8601Time::hour: string parsing deferred to the internal engine")
    }

    /// `minute(): Integer`.
    ///
    /// Extract the minute part of the time as an Integer, or return 0 if
    /// not present.
    ///
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn minute(&self) -> i32 {
        todo!("Iso8601Time::minute: string parsing deferred to the internal engine")
    }

    /// `second(): Integer`.
    ///
    /// Extract the integral seconds part of the time (i.e. prior to any
    /// decimal sign) as an Integer, or return 0 if not present.
    ///
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn second(&self) -> i32 {
        todo!("Iso8601Time::second: string parsing deferred to the internal engine")
    }

    /// `fractional_second(): Real`.
    ///
    /// Pre: `not second_unknown`.
    ///
    /// Extract the fractional seconds part of the time (i.e. following to
    /// any decimal sign) as a Real, or return 0.0 if not present.
    ///
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn fractional_second(&self) -> f64 {
        todo!("Iso8601Time::fractional_second: string parsing deferred to the internal engine")
    }

    /// `timezone(): Iso8601_timezone`.
    ///
    /// Timezone; may be Void.
    ///
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn timezone(&self) -> Option<Iso8601Timezone> {
        todo!("Iso8601Time::timezone: string parsing deferred to the internal engine")
    }

    /// `minute_unknown(): Boolean`.
    ///
    /// Indicates whether minute is unknown. If so, the time is of the form
    /// `"hh"`.
    ///
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn minute_unknown(&self) -> bool {
        todo!("Iso8601Time::minute_unknown: string parsing deferred to the internal engine")
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
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn second_unknown(&self) -> bool {
        todo!("Iso8601Time::second_unknown: string parsing deferred to the internal engine")
    }

    /// `is_decimal_sign_comma(): Boolean`.
    ///
    /// True if this time has a decimal part indicated by `','` (comma)
    /// rather than `'.'` (period).
    ///
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn is_decimal_sign_comma(&self) -> bool {
        todo!("Iso8601Time::is_decimal_sign_comma: string parsing deferred to the internal engine")
    }

    /// `has_fractional_second(): Boolean`.
    ///
    /// True if the `fractional_second` part is significant (i.e. even if =
    /// 0.0).
    ///
    /// TODO(port): requires parsing `core.value`; deferred to the internal
    /// engine.
    pub fn has_fractional_second(&self) -> bool {
        todo!("Iso8601Time::has_fractional_second: string parsing deferred to the internal engine")
    }

    /// `add` __alias__ `"+"` `(a_diff: Iso8601_duration) -> Iso8601_time`.
    ///
    /// Arithmetic addition of a duration to a time.
    ///
    /// TODO(port): definite-arithmetic addition; deferred to the internal
    /// engine.
    pub fn add(&self, a_diff: &Iso8601Duration) -> Iso8601Time {
        let _ = a_diff;
        todo!("Iso8601Time::add: definite-duration addition deferred to the internal engine")
    }

    /// `subtract` __alias__ `"-"` `(a_diff: Iso8601_duration) -> Iso8601_time`.
    ///
    /// Arithmetic subtraction of a duration from a time.
    ///
    /// TODO(port): definite-arithmetic subtraction; deferred to the
    /// internal engine.
    pub fn subtract(&self, a_diff: &Iso8601Duration) -> Iso8601Time {
        let _ = a_diff;
        todo!(
            "Iso8601Time::subtract: definite-duration subtraction deferred to the internal engine"
        )
    }

    /// `diff` __alias__ `"-"` `(a_time: Iso8601_time) -> Iso8601_duration`.
    ///
    /// Difference of two times.
    ///
    /// TODO(port): deferred to the internal engine.
    pub fn diff(&self, a_time: &Iso8601Time) -> Iso8601Duration {
        let _ = a_time;
        todo!("Iso8601Time::diff: deferred to the internal engine")
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
    /// effector requires comparing parsed, partial-aware hour/minute/second
    /// components, deferred to the internal engine (see `Iso8601Date
    /// ::less_than` for the analogous note).
    fn less_than(&self, other: &Self) -> bool {
        let _ = other;
        todo!(
            "Iso8601Time::less_than: partial-aware time comparison deferred to the internal engine"
        )
    }
}

impl Temporal for Iso8601Time {}

impl Iso8601Type for Iso8601Time {
    fn core(&self) -> &Iso8601TypeCore {
        &self.core
    }

    /// `is_partial(): Boolean` (effected).
    ///
    /// True if this time is partial, i.e. if seconds or more is missing.
    ///
    /// TODO(port): equivalent to `minute_unknown() or second_unknown()`;
    /// deferred to the internal engine.
    fn is_partial(&self) -> bool {
        todo!(
            "Iso8601Time::is_partial: depends on minute_unknown/second_unknown, deferred to the internal engine"
        )
    }

    /// `is_extended(): Boolean` (effected).
    ///
    /// True if this time uses `'-'`, `':'` separators.
    ///
    /// TODO(port): requires inspecting `core.value`; deferred to the
    /// internal engine.
    fn is_extended(&self) -> bool {
        todo!("Iso8601Time::is_extended: deferred to the internal engine")
    }
}

// PORT NOTE: see the equivalent note in `iso8601_date.rs` — `Time_Definitions`
// is not a Rust supertrait here; these invariants call `TimeDefinitions::*`
// directly, and are encoded as plain boolean-returning methods rather than a
// `Validate` impl (see the equivalent note in `iso8601_date.rs`).
impl Iso8601Time {
    /// __Hour_valid__: `valid_hour(hour, minute, second)`.
    pub fn invariant_hour_valid(&self) -> bool {
        TimeDefinitions::valid_hour(self.hour(), self.minute(), self.second())
    }

    /// __Minute_valid__: `not minute_unknown implies valid_minute (minute)`.
    pub fn invariant_minute_valid(&self) -> bool {
        self.minute_unknown() || TimeDefinitions::valid_minute(self.minute())
    }

    /// __Second_valid__: `not second_unknown implies valid_second (second)`.
    pub fn invariant_second_valid(&self) -> bool {
        self.second_unknown() || TimeDefinitions::valid_second(self.second())
    }

    /// __Fractional_second_valid__: `has_fractional_second implies (not
    /// second_unknown and valid_fractional_second (fractional_second))`.
    pub fn invariant_fractional_second_valid(&self) -> bool {
        !self.has_fractional_second()
            || (!self.second_unknown()
                && TimeDefinitions::valid_fractional_second(self.fractional_second()))
    }

    /// __Partial_validity__: `minute_unknown implies second_unknown`.
    pub fn invariant_partial_validity(&self) -> bool {
        !self.minute_unknown() || self.second_unknown()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso8601_time.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / iso8601_time.adoc §Iso8601_time Class
//   confidence: medium
//   todos: 14
//   note: every accessor/arithmetic body needing string parsing is stubbed todo!() pending the jiff-backed internal engine at P17; second_unknown's description text ("...and month is known") looks like a copy-paste artifact from Iso8601_date_time, transcribed verbatim and flagged rather than silently corrected; invariants are plain boolean methods, not a Validate impl (out of scope for foundation-types values).
// ─────────────────────────────────────────────
