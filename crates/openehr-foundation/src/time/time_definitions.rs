//! `Time_Definitions` — constants and validity functions for date/time
//! classes.
//!
//! openEHR class: `Time_Definitions`, package `base.foundation_types.time`.
//! No stated `Inherit` relation in the per-class table.
//!
//! "Definitions for date/time classes. Note that the timezone limits are set
//! by where the international dateline is. Thus, time in New Zealand is
//! quoted using `+12:00`, not `-12:00`."

use crate::time::iso8601_parser::{parse_date, parse_date_time, parse_duration, parse_time};

/// `Time_Definitions` declares only constants and stateless validity
/// functions — no instance attributes anywhere in the per-class table — so
/// it is transcribed as a zero-sized unit struct carrying associated `const`
/// items and associated `fn`s, giving callers the spec's own namespacing
/// (`TimeDefinitions::valid_year(...)`, mirroring `Time_definitions
/// .valid_year(...)`) without an instance to construct.
///
/// `Iso8601_type` (see `iso8601_type.rs`) declares `Time_Definitions` as one
/// of its two parents in the `Inherit` row. Since this class has no
/// instance-level behaviour to abstract over — every member here is either a
/// compile-time constant or a pure function of explicit arguments, closer to
/// Eiffel's "class as a namespace of shared features" idiom than to a
/// behavioural interface — that inheritance is transcribed as direct calls
/// to `TimeDefinitions::*` from the `Iso8601Type` family rather than as a
/// second Rust supertrait bound. See the multiple-inheritance PORT NOTE on
/// `Iso8601Type` in `iso8601_type.rs` for the full reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeDefinitions;

impl TimeDefinitions {
    /// `Seconds_in_minute`: `Integer = 60`.
    ///
    /// Number of seconds in a minute.
    pub const SECONDS_IN_MINUTE: i32 = 60;

    /// `Minutes_in_hour`: `Integer = 60`.
    ///
    /// Number of minutes in an hour.
    pub const MINUTES_IN_HOUR: i32 = 60;

    /// `Hours_in_day`: `Integer = 24`.
    ///
    /// Number of clock hours in a day, i.e. 24.
    pub const HOURS_IN_DAY: i32 = 24;

    /// `Average_days_in_month`: `Real = 30.42`.
    ///
    /// Used for conversions of durations containing months to days and / or
    /// seconds.
    pub const AVERAGE_DAYS_IN_MONTH: f64 = 30.42;

    /// `Max_days_in_month`: `Integer = 31`.
    ///
    /// Maximum number of days in any month.
    pub const MAX_DAYS_IN_MONTH: i32 = 31;

    /// `Days_in_year`: `Integer = 365`.
    ///
    /// Calendar days in a normal year, i.e. 365.
    pub const DAYS_IN_YEAR: i32 = 365;

    /// `Average_days_in_year`: `Real = 365.24`.
    ///
    /// Used for conversions of durations containing years to days and / or
    /// seconds.
    pub const AVERAGE_DAYS_IN_YEAR: f64 = 365.24;

    /// `Days_in_leap_year`: `Integer = 366`.
    ///
    /// Calendar days in a standard leap year, i.e. 366.
    pub const DAYS_IN_LEAP_YEAR: i32 = 366;

    /// `Max_days_in_year`: `Integer`.
    ///
    /// Maximum number of days in a year, i.e. accounting for leap years. The
    /// spec table gives no literal value for this constant (unlike its
    /// siblings, which are all assigned inline); transcribed here as an
    /// alias for `Days_in_leap_year`, the only value consistent with the
    /// constant's own description ("accounting for leap years").
    pub const MAX_DAYS_IN_YEAR: i32 = Self::DAYS_IN_LEAP_YEAR;

    /// `Days_in_week`: `Integer = 7`.
    ///
    /// Number of days in a week.
    pub const DAYS_IN_WEEK: i32 = 7;

    /// `Months_in_year`: `Integer = 12`.
    ///
    /// Number of months in a year.
    pub const MONTHS_IN_YEAR: i32 = 12;

    /// `Min_timezone_hour`: `Integer = 12`.
    ///
    /// Minimum hour value of a timezone according to ISO 8601 (note that the
    /// -ve sign is supplied in the `Iso8601_timezone` class).
    pub const MIN_TIMEZONE_HOUR: i32 = 12;

    /// `Max_timezone_hour`: `Integer = 14`.
    ///
    /// Maximum hour value of a timezone according to ISO 8601.
    pub const MAX_TIMEZONE_HOUR: i32 = 14;

    /// `Nominal_days_in_month`: `Real = 30.42`.
    ///
    /// Used for conversions of durations containing months to days and / or
    /// seconds.
    ///
    /// PORT NOTE: the spec table gives this constant the identical value and
    /// description as `Average_days_in_month` above; transcribed as its own
    /// distinct constant (not an alias) since the spec declares them as two
    /// separate named constants, even though the published table does not
    /// explain why the "average" and "nominal" figures for months
    /// coincide numerically.
    pub const NOMINAL_DAYS_IN_MONTH: f64 = 30.42;

    /// `Nominal_days_in_year`: `Real = 365.24`.
    ///
    /// Used for conversions of durations containing years to days and / or
    /// seconds.
    ///
    /// PORT NOTE: same observation as `Nominal_days_in_month` above — this
    /// figure is identical to `Average_days_in_year` in the published table.
    pub const NOMINAL_DAYS_IN_YEAR: f64 = 365.24;

    /// `valid_year(y: Integer) -> Boolean`.
    ///
    /// Post: `Result = y >= 0`.
    #[must_use]
    pub fn valid_year(y: i32) -> bool {
        y >= 0
    }

    /// `valid_month(m: Integer) -> Boolean`.
    ///
    /// Post: `Result = m >= 1 and m <= Months_in_year`.
    #[must_use]
    pub fn valid_month(m: i32) -> bool {
        (1..=Self::MONTHS_IN_YEAR).contains(&m)
    }

    /// `valid_day(y: Integer, m: Integer, d: Integer) -> Boolean`.
    ///
    /// Post: `Result = d >= 1 and d <= days_in_month(m, y)`.
    ///
    /// PORT NOTE: the spec's postcondition calls a `days_in_month(m, y)`
    /// helper not declared as a `Time_Definitions` function. It is
    /// implemented here with standard proleptic Gregorian month lengths, the
    /// only reading consistent with the same table's leap-year constants and
    /// date-validity text.
    #[must_use]
    pub fn valid_day(y: i32, m: i32, d: i32) -> bool {
        d >= 1 && Self::days_in_month(m, y).is_some_and(|max| d <= max)
    }

    /// Implementation helper for the `valid_day` postcondition's
    /// spec-referenced `days_in_month(m, y)`.
    #[must_use]
    pub fn days_in_month(m: i32, y: i32) -> Option<i32> {
        if !Self::valid_year(y) || !Self::valid_month(m) {
            return None;
        }
        let days = match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => Self::MAX_DAYS_IN_MONTH,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(y) => 29,
            2 => 28,
            _ => return None,
        };
        Some(days)
    }

    /// Proleptic Gregorian leap-year helper used by `days_in_month`.
    #[must_use]
    pub fn is_leap_year(y: i32) -> bool {
        y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
    }

    /// `valid_hour(h: Integer, m: Integer, s: Integer) -> Boolean`.
    ///
    /// Post: `Result = (h >= 0 and h < Hours_in_day) or (h = Hours_in_day and
    /// m = 0 and s = 0)`.
    #[must_use]
    pub fn valid_hour(h: i32, m: i32, s: i32) -> bool {
        (0..Self::HOURS_IN_DAY).contains(&h) || (h == Self::HOURS_IN_DAY && m == 0 && s == 0)
    }

    /// `valid_minute(m: Integer) -> Boolean`.
    ///
    /// Post: `Result = m >= 0 and m < Minutes_in_hour`.
    #[must_use]
    pub fn valid_minute(m: i32) -> bool {
        (0..Self::MINUTES_IN_HOUR).contains(&m)
    }

    /// `valid_second(s: Integer) -> Boolean`.
    ///
    /// Post: `Result = s >= 0 and s < Seconds_in_minute`.
    #[must_use]
    pub fn valid_second(s: i32) -> bool {
        (0..Self::SECONDS_IN_MINUTE).contains(&s)
    }

    /// `valid_fractional_second(fs: Double) -> Boolean`.
    ///
    /// Post: `Result = fs >= 0.0 and fs < 1.0`.
    ///
    /// PORT NOTE: the spec table types the `fs` parameter as `Double`, not
    /// `Real`, even though every call site of this function elsewhere in the
    /// `time` package (`Iso8601_time.fractional_second`,
    /// `Iso8601_duration.fractional_seconds`) declares its own
    /// `fractional_second`-shaped attribute as `Real`. Transcribed exactly
    /// as the table states (`f64`, matching this crate's `Double`) rather
    /// than "corrected" to `Real`'s backing type — the two are numerically
    /// identical in this crate (`Real` and `Double` are both `f64`, see
    /// `primitive_types::real`/`primitive_types::double` PORT NOTEs), so no
    /// call site is actually affected, but the mismatch is flagged here for
    /// visibility rather than silently resolved.
    #[must_use]
    pub fn valid_fractional_second(fs: f64) -> bool {
        (0.0..1.0).contains(&fs)
    }

    /// `valid_iso8601_date(s: String) -> Boolean`.
    ///
    /// String is a valid ISO 8601 date, i.e. takes the complete form
    /// `YYYY-MM-DD` (extended, preferred) or one of the partial forms
    /// `YYYY-MM` or `YYYY`; `YYYYMMDD` (compact) or a partial variant
    /// `YYYYMM`. The combinations of `YYYY`, `MM`, `DD` numbers must be
    /// correct with respect to the Gregorian calendar.
    ///
    #[must_use]
    pub fn valid_iso8601_date(s: &str) -> bool {
        parse_date(s).is_some()
    }

    /// `valid_iso8601_time(s: String) -> Boolean`.
    ///
    /// String is a valid ISO 8601 time, i.e. takes the extended form
    /// `hh:mm:ss[(,|.)s+][Z|±hh[:mm]]`, the compact form
    /// `hhmmss[(,|.)s+][Z|±hh[mm]]`, or one of the partial forms `hh:mm`
    /// (extended), `hhmm` or `hh` (compact), with an optional timezone
    /// indicator.
    ///
    #[must_use]
    pub fn valid_iso8601_time(s: &str) -> bool {
        parse_time(s).is_some()
    }

    /// `valid_iso8601_date_time(s: String) -> Boolean`.
    ///
    /// String is a valid ISO 8601 date-time, i.e. takes the extended form
    /// `YYYY-MM-DDThh:mm:ss[(,|.)s+][Z|±hh[:mm]]`, the compact form
    /// `YYYYMMDDThhmmss[(,|.)s+][Z|±hh[mm]]`, or one of the partial forms
    /// `YYYY-MM-DDThh:mm`/`YYYY-MM-DDThh` (extended) or
    /// `YYYYMMDDThhmm`/`YYYYMMDDThh` (compact).
    ///
    #[must_use]
    pub fn valid_iso8601_date_time(s: &str) -> bool {
        parse_date_time(s).is_some()
    }

    /// `valid_iso8601_duration(s: String) -> Boolean`.
    ///
    /// String is a valid ISO 8601 duration, i.e. takes the form
    /// `P[nnY][nnM][nnW][nnD][T[nnH][nnM][nnS]]`, where each `nn` represents
    /// a number of years, months, etc., and `nnW` represents a number of
    /// 7-day weeks. Per the openEHR deviation from the published standard,
    /// the `W` designator may appear alongside the other designators (used
    /// for expressing pregnancy duration).
    ///
    #[must_use]
    pub fn valid_iso8601_duration(s: &str) -> bool {
        parse_duration(s).is_some()
    }
}

// PERF(port): the parser is intentionally shared by `valid_iso8601_*` and
// the concrete `Iso8601_*` accessors so the crate has one parse-policy
// surface for the BASE time package. If this becomes hot, cache parsed forms
// at the caller layer rather than duplicating grammar code.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — docs/research/spec-cache/BASE-1.2.0/uml_classes/time_definitions.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master06-time_types.adoc §Class Definitions / time_definitions.adoc §Time_Definitions Class
//   confidence: medium
//   todos: 0
//   note: ISO 8601 validity functions delegate to the shared BASE time parser; valid_day implements the spec-referenced-but-undeclared days_in_month(m, y) helper with proleptic Gregorian month lengths; Max_days_in_year has no literal value in the table and is transcribed as an alias of Days_in_leap_year.
// ─────────────────────────────────────────────
