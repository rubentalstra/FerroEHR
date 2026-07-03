//! `Iso8601_timezone` — an ISO 8601 timezone string.
//!
//! openEHR class: `Iso8601_timezone`, package `base.foundation_types.time`.
//! Inherits: `Iso8601_type`.
//!
//! ISO8601 timezone string, in format:
//! * `Z | ±hh[mm]`
//!
//! where:
//! * `hh` is `"00"` - `"23"` (0-filled to two digits)
//! * `mm` is `"00"` - `"59"` (0-filled to two digits)
//! * `Z` is a literal meaning UTC (modern replacement for GMT), i.e.
//!   timezone `+0000`
//!
//! # String-value representation, not a resolved offset
//!
//! Models an ISO 8601 timezone-offset *string* (e.g. `"Z"`, `"+02:00"`),
//! not a resolved UTC-offset value. See the module-level doc on
//! `iso8601_type.rs` for the full rationale and the jiff-bridging plan for
//! P17.
use crate::primitive_types::any::Any;
use crate::primitive_types::ordered::Ordered;
use crate::time::iso8601_arithmetic::format_timezone;
use crate::time::iso8601_parser::parse_timezone;
use crate::time::iso8601_type::{Iso8601Type, Iso8601TypeCore};
use crate::time::temporal::Temporal;
use crate::time::time_definitions::TimeDefinitions;

/// `Iso8601_timezone` embeds the `Iso8601_type` parent state (`value:
/// String`) via `Iso8601TypeCore`, per ADR-001 §3. This struct declares no
/// attributes of its own beyond the inherited `value`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iso8601Timezone {
    /// Embedded `Iso8601_type.value: String`.
    pub core: Iso8601TypeCore,
}

impl Iso8601Timezone {
    /// `hour(): Integer`.
    ///
    /// Extract the hour part of timezone, as an Integer in the range `00 -
    /// 14`.
    ///
    #[must_use]
    pub fn hour(&self) -> i32 {
        parse_timezone(&self.core.value).map_or(0, |parsed| parsed.hour)
    }

    /// `minute(): Integer`.
    ///
    /// Extract the hour part of timezone, as an Integer, usually either 0
    /// or 30.
    ///
    /// PORT NOTE: the spec description text literally says "Extract the
    /// **hour** part of timezone" for the `minute()` function — almost
    /// certainly a copy-paste artifact from the `hour()` row immediately
    /// above it (the described range/typical-values wording, "usually
    /// either 0 or 30", is unambiguously about minutes, not hours).
    /// Transcribed with the name-implied semantics (extracting the minute
    /// component), matching this class's own function name; flagged here
    /// rather than silently corrected.
    ///
    #[must_use]
    pub fn minute(&self) -> i32 {
        parse_timezone(&self.core.value).map_or(
            0,
            super::iso8601_parser::ParsedIso8601Timezone::minute_value,
        )
    }

    /// `sign(): Integer`.
    ///
    /// Direction of timezone expressed as +1 or -1.
    ///
    #[must_use]
    pub fn sign(&self) -> i32 {
        parse_timezone(&self.core.value).map_or(0, |parsed| parsed.sign)
    }

    /// `minute_unknown(): Boolean`.
    ///
    /// Indicates whether minute part known.
    ///
    #[must_use]
    pub fn minute_unknown(&self) -> bool {
        parse_timezone(&self.core.value)
            .is_none_or(super::iso8601_parser::ParsedIso8601Timezone::minute_unknown)
    }

    /// `is_gmt(): Boolean`.
    ///
    /// True if timezone is UTC, i.e. `+0000`.
    ///
    #[must_use]
    pub fn is_gmt(&self) -> bool {
        parse_timezone(&self.core.value)
            .is_some_and(super::iso8601_parser::ParsedIso8601Timezone::is_gmt)
    }
}

impl Any for Iso8601Timezone {
    fn is_equal(&self, other: &Self) -> bool {
        self.core == other.core
    }

    fn type_of(&self) -> String {
        "Iso8601_timezone".to_string()
    }
}

impl Ordered for Iso8601Timezone {
    /// `less_than` __alias__ `"<"` `(other: Iso8601_timezone) -> Boolean`.
    ///
    /// PORT NOTE: not itself declared on `Iso8601_timezone`'s per-class
    /// table — inherited abstractly from `Ordered` via `Temporal`. A
    /// faithful effector compares the signed hour/minute offset (`sign() *
    /// (hour() * 60 + minute())`, treating an absent minute as zero).
    fn less_than(&self, other: &Self) -> bool {
        match (
            parse_timezone(&self.core.value),
            parse_timezone(&other.core.value),
        ) {
            (Some(left), Some(right)) => left.offset_minutes() < right.offset_minutes(),
            _ => self.core.value < other.core.value,
        }
    }
}

impl Temporal for Iso8601Timezone {}

impl Iso8601Type for Iso8601Timezone {
    /// `as_string(): String`.
    ///
    /// Return timezone string in extended format: a compact-form offset
    /// (`"+0230"`) is reformatted with a `:` separator (`"+02:30"`),
    /// effecting the "in extended format" contract the
    /// `Iso8601Type::as_string` default cannot honour without parsing.
    /// `"Z"` and hour-only offsets (`"+02"`) are already their own extended
    /// form; an unparseable value is returned verbatim.
    fn as_string(&self) -> String {
        parse_timezone(&self.core.value).map_or_else(
            || self.core.value.clone(),
            |parsed| format_timezone(parsed, true),
        )
    }

    /// `is_partial(): Boolean` (effected).
    ///
    /// True if this time zone is partial, i.e. if minutes is missing.
    ///
    fn is_partial(&self) -> bool {
        self.minute_unknown()
    }

    /// `is_extended(): Boolean` (effected).
    ///
    /// True if this time-zone uses `':'` separators.
    ///
    fn is_extended(&self) -> bool {
        parse_timezone(&self.core.value).is_some_and(|parsed| parsed.extended)
    }

    fn core(&self) -> &Iso8601TypeCore {
        &self.core
    }
}

// PORT NOTE: see the equivalent note in `iso8601_date.rs` — `Time_Definitions`
// is not a Rust supertrait here; these invariants call `TimeDefinitions::*`
// directly, and are encoded as plain boolean-returning methods rather than a
// `Validate` impl.
impl Iso8601Timezone {
    /// __`Min_hour_valid`__: `sign = -1 implies hour > 0 and hour <=
    /// Min_timezone_hour`.
    #[must_use]
    pub fn invariant_min_hour_valid(&self) -> bool {
        self.sign() != -1 || (self.hour() > 0 && self.hour() <= TimeDefinitions::MIN_TIMEZONE_HOUR)
    }

    /// __`Max_hour_valid`__: `sign = 1 implies hour > 0 and hour <=
    /// Max_timezone_hour`.
    ///
    /// PORT NOTE: the invariant's published `hour > 0` wording conflicts
    /// with the same class description's UTC forms (`Z`, `+0000`). This
    /// permits zero for positive/UTC offsets, preserving the format grammar
    /// and `is_gmt()` semantics.
    #[must_use]
    pub fn invariant_max_hour_valid(&self) -> bool {
        self.sign() != 1 || (self.hour() >= 0 && self.hour() <= TimeDefinitions::MAX_TIMEZONE_HOUR)
    }

    /// __`Minute_valid`__: `not minute_unknown implies valid_minute (minute)`.
    #[must_use]
    pub fn invariant_minute_valid(&self) -> bool {
        self.minute_unknown() || TimeDefinitions::valid_minute(self.minute())
    }

    /// __`Sign_valid`__: `sign = 1 or sign = -1`.
    #[must_use]
    pub fn invariant_sign_valid(&self) -> bool {
        self.sign() == 1 || self.sign() == -1
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso8601_timezone.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / iso8601_timezone.adoc §Iso8601_timezone Class
//   confidence: medium
//   todos: 0
//   note: minute()'s published description text ("Extract the hour part...") looks like a copy-paste artifact from hour() immediately above it in the table, transcribed with the name-implied minute semantics and flagged rather than silently corrected; Max_hour_valid permits +00/Z despite the table's conflicting hour>0 wording so UTC remains valid. Accessors and ordering delegate to the shared BASE ISO 8601 parser; as_string now effects the extended-format contract via iso8601_arithmetic::format_timezone.
// ─────────────────────────────────────────────
