//! Hand-written ISO 8601 parsing + completion-range machinery shared by the
//! four `Iso8601_*` `*_impl.rs` siblings.
//!
//! The generated `Iso8601_*` types hold their value as a single `String`
//! (BMM: `Iso8601_type.value`). Ordering, the spec-declared accessor
//! functions, and duration reduction all need the value decomposed into
//! typed components, so this module parses the documented lexical forms into
//! plain component structs. A malformed string parses to `None`, which every
//! caller turns into an undecidable (`None`) comparison — the generated types
//! admit any `String`, so parsing can always fail and must never panic.
//!
//! Spec sources (vendored):
//! - `BASE/docs/UML/classes/org.openehr.base.foundation_types.time_definitions.adoc`
//!   (§Constants: the average-length constants used verbatim below; §Functions:
//!   `valid_iso8601_date`/`_time`/`_date_time`/`_duration`, `valid_second`,
//!   `valid_hour`).
//! - `BASE/docs/foundation_types/master06-time_types.adoc` (§Overview,
//!   §Primitive Time Types: the accepted forms and openEHR's deviations —
//!   week dates `YYYY-Www` excluded, `24:00:00` disallowed anywhere, the `W`
//!   duration designator mixable, negative durations, 4-digit years only).
//!
//! NOTE: the openEHR specs define THAT these types are `Ordered` (via the
//! `Ordered` ancestry of `Iso8601_type`) but give NO comparison algorithm —
//! `Ordered.less_than` is abstract and none of the four classes effect it, and
//! there is no `magnitude`. Partial-value ordering, timezone normalisation, and
//! duration comparison are therefore spec-silent: the algorithms here are our
//! own design/extension, grounded only in the range/completion semantics of
//! partial ISO 8601 values (see each `*_impl.rs`).
//!
//! NOTE: `Interval.Limits_comparable`
//! (`org.openehr.base.foundation_types.interval.adoc`) is stated in terms of
//! `lower.strictly_comparable_to(upper)`, but `strictly_comparable_to` is
//! referenced nowhere-else and defined by no class in the vendored BASE spec
//! (a spec defect). Our operational comparability predicate for the temporal
//! value spaces is exactly `PartialOrd::partial_cmp(..).is_some()` — two values
//! are comparable when their completion intervals are order-separated.

use std::cmp::Ordering;

// ── Time_Definitions constants (verbatim; §Constants) ────────────────────────
// The BMM `Time_Definitions` class is not emitted as a generated type (it
// carries only constants + validity functions), so its constants are mirrored
// here verbatim from
// `org.openehr.base.foundation_types.time_definitions.adoc` §Constants.

/// `Time_Definitions.Seconds_in_minute` = 60.
pub(crate) const SECONDS_IN_MINUTE: f64 = 60.0;
/// `Time_Definitions.Minutes_in_hour` = 60.
pub(crate) const MINUTES_IN_HOUR: f64 = 60.0;
/// `Time_Definitions.Hours_in_day` = 24.
pub(crate) const HOURS_IN_DAY: f64 = 24.0;
/// `Time_Definitions.Days_in_week` = 7.
pub(crate) const DAYS_IN_WEEK: f64 = 7.0;
/// `Time_Definitions.Average_days_in_month` = 30.42.
pub(crate) const AVERAGE_DAYS_IN_MONTH: f64 = 30.42;
/// `Time_Definitions.Average_days_in_year` = 365.24.
pub(crate) const AVERAGE_DAYS_IN_YEAR: f64 = 365.24;

/// Seconds in one clock hour (`Minutes_in_hour * Seconds_in_minute`).
pub(crate) const SECONDS_IN_HOUR: f64 = MINUTES_IN_HOUR * SECONDS_IN_MINUTE;
/// Seconds in one clock day (`Hours_in_day * Minutes_in_hour * Seconds_in_minute`).
pub(crate) const SECONDS_IN_DAY: f64 = HOURS_IN_DAY * SECONDS_IN_HOUR;

// ── Parsed component structs ─────────────────────────────────────────────────

/// A parsed ISO 8601 date. `year` is always present (the openEHR types have no
/// sensible value without it); `month`/`day` are absent for the partial forms
/// `YYYY` and `YYYY-MM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParsedDate {
    pub(crate) year: u32,
    pub(crate) month: Option<u32>,
    pub(crate) day: Option<u32>,
}

/// A parsed ISO 8601 time. `hour` is always present; `minute`/`second` are
/// absent for the partial forms `hh` and `hh:mm`. `fractional_second` is only
/// meaningful when `second` is present. `timezone` is the offset in signed
/// minutes (`Z` → `Some(0)`, absent → `None`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParsedTime {
    pub(crate) hour: u32,
    pub(crate) minute: Option<u32>,
    pub(crate) second: Option<u32>,
    pub(crate) fractional_second: Option<f64>,
    pub(crate) timezone: Option<i32>,
}

/// A parsed ISO 8601 date/time: a date (possibly partial) with an optional
/// time part (present exactly when the source carried a `T` separator).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParsedDateTime {
    pub(crate) date: ParsedDate,
    pub(crate) time: Option<ParsedTime>,
}

/// A parsed ISO 8601 duration. Integer designator counts plus fractional
/// seconds and a sign flag (openEHR allows a leading `-` and mixing `W` with
/// other designators).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParsedDuration {
    pub(crate) negative: bool,
    pub(crate) years: u64,
    pub(crate) months: u64,
    pub(crate) weeks: u64,
    pub(crate) days: u64,
    pub(crate) hours: u64,
    pub(crate) minutes: u64,
    pub(crate) seconds: u64,
    pub(crate) fractional_seconds: f64,
}

/// The four decomposed time-of-day fields: `(hour, minute, second,
/// fractional_second)`, with the trailing components absent for a partial time.
type TimeParts = (u32, Option<u32>, Option<u32>, Option<f64>);

// ── Small lexical helpers (no panicking indexing/slicing) ─────────────────────

/// Parse `tok` as a `width`-digit zero-filled unsigned integer, rejecting any
/// token whose length differs or that contains a non-digit (so `"1"` is not a
/// valid two-digit month and `"+02"`/whitespace are rejected).
fn parse_fixed(tok: &str, width: usize) -> Option<u32> {
    if tok.len() != width || !tok.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    tok.parse::<u32>().ok()
}

/// True for a Gregorian leap year (`master06`/`Time_definitions.days_in_month`
/// calendar semantics).
pub(crate) fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// Calendar days in `month` (1..=12) of `year`, or `None` for an out-of-range
/// month (`Time_definitions.valid_day` uses `days_in_month`).
pub(crate) fn days_in_month(year: u32, month: u32) -> Option<u32> {
    let d = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return None,
    };
    Some(d)
}

/// Days since 1970-01-01 for a proleptic-Gregorian `(year, month, day)`
/// (Howard Hinnant's `days_from_civil`, a standard branch-free algorithm). Used
/// only to place date/times on a common absolute-seconds axis for timezone
/// normalisation. Inputs are validated dates, so the result is well-defined.
#[allow(clippy::cast_possible_wrap)] // year/month/day are small validated ranges
fn days_from_civil(year: u32, month: u32, day: u32) -> i64 {
    let y = i64::from(year) - i64::from(month <= 2);
    let m = i64::from(month);
    let d = i64::from(day);
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 }; // Mar=0 … Feb=11
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

// ── Date parsing ──────────────────────────────────────────────────────────────

/// Parse an ISO 8601 date in extended (`YYYY-MM-DD`, `YYYY-MM`, `YYYY`) or
/// compact (`YYYYMMDD`, `YYYYMM`, `YYYY`) form. Week dates (`YYYY-Www`) and
/// expanded (>4-digit / signed) years are rejected (`master06` §Primitive Time
/// Types); component combinations must be calendar-valid
/// (`Time_definitions.valid_iso8601_date`).
pub(crate) fn parse_date(s: &str) -> Option<ParsedDate> {
    if s.is_empty() || s.bytes().any(|b| b == b'W' || b == b'w') {
        return None;
    }
    let (year, month, day) = if s.contains('-') {
        let mut parts = s.split('-');
        let year = parse_fixed(parts.next()?, 4)?;
        let month = match parts.next() {
            Some(m) => Some(parse_fixed(m, 2)?),
            None => None,
        };
        let day = match parts.next() {
            Some(d) => Some(parse_fixed(d, 2)?),
            None => None,
        };
        if parts.next().is_some() {
            return None;
        }
        (year, month, day)
    } else {
        match s.len() {
            4 => (parse_fixed(s.get(0..4)?, 4)?, None, None),
            6 => (
                parse_fixed(s.get(0..4)?, 4)?,
                Some(parse_fixed(s.get(4..6)?, 2)?),
                None,
            ),
            8 => (
                parse_fixed(s.get(0..4)?, 4)?,
                Some(parse_fixed(s.get(4..6)?, 2)?),
                Some(parse_fixed(s.get(6..8)?, 2)?),
            ),
            _ => return None,
        }
    };
    validate_date(year, month, day)
}

/// Enforce the `Iso8601_date` validity invariants: a day requires a month
/// (`Partial_validity`), and present components must be calendar-valid
/// (`Month_valid`/`Day_valid`).
fn validate_date(year: u32, month: Option<u32>, day: Option<u32>) -> Option<ParsedDate> {
    if let Some(m) = month {
        if !(1..=12).contains(&m) {
            return None;
        }
        if let Some(d) = day {
            let max = days_in_month(year, m)?;
            if d < 1 || d > max {
                return None;
            }
        }
    } else if day.is_some() {
        return None; // day present but month unknown — invalid partial
    }
    Some(ParsedDate { year, month, day })
}

// ── Time parsing ──────────────────────────────────────────────────────────────

/// Parse an ISO 8601 time (`hh[:mm[:ss[(.|,)fff]]][Z|±hh[:mm]]` extended or the
/// compact form). Rejects `24:00:00` anywhere and a `60` (leap) second
/// (`master06`; `Time_definitions.valid_second` Post `s < Seconds_in_minute`),
/// see the leap-second NOTE in `iso8601_time_impl.rs`.
pub(crate) fn parse_time(s: &str) -> Option<ParsedTime> {
    // Split off the timezone: it starts at the first 'Z', '+' or '-'.
    let tz_start = s.bytes().position(|b| b == b'Z' || b == b'+' || b == b'-');
    let (main, timezone) = match tz_start {
        Some(i) => {
            let main = s.get(0..i)?;
            let tz = s.get(i..)?;
            (main, Some(parse_timezone(tz)?))
        }
        None => (s, None),
    };
    let (hour, minute, second, fractional_second) = parse_time_main(main)?;
    Some(ParsedTime {
        hour,
        minute,
        second,
        fractional_second,
        timezone,
    })
}

/// Parse the time-of-day part (no timezone) in extended or compact form.
fn parse_time_main(main: &str) -> Option<TimeParts> {
    // Separate an optional fractional-seconds tail introduced by '.' or ','.
    let (body, frac) = split_fraction(main);
    let (hour, minute, second) = if body.contains(':') {
        let mut parts = body.split(':');
        let hour = parse_fixed(parts.next()?, 2)?;
        let minute = match parts.next() {
            Some(m) => Some(parse_fixed(m, 2)?),
            None => None,
        };
        let second = match parts.next() {
            Some(sec) => Some(parse_fixed(sec, 2)?),
            None => None,
        };
        if parts.next().is_some() {
            return None;
        }
        (hour, minute, second)
    } else {
        match body.len() {
            2 => (parse_fixed(body.get(0..2)?, 2)?, None, None),
            4 => (
                parse_fixed(body.get(0..2)?, 2)?,
                Some(parse_fixed(body.get(2..4)?, 2)?),
                None,
            ),
            6 => (
                parse_fixed(body.get(0..2)?, 2)?,
                Some(parse_fixed(body.get(2..4)?, 2)?),
                Some(parse_fixed(body.get(4..6)?, 2)?),
            ),
            _ => return None,
        }
    };
    // A fractional part is only meaningful on a present second.
    let fractional_second = match frac {
        Some(f) if second.is_some() => Some(f),
        Some(_) => return None,
        None => None,
    };
    validate_time(hour, minute, second, fractional_second)
}

/// Split a trailing `(.|,)digits` fractional part off the time body, returning
/// the body and the parsed fraction (a value in `[0.0, 1.0)`), or `(main, None)`
/// when absent. A malformed fraction makes the whole time unparseable.
fn split_fraction(main: &str) -> (&str, Option<f64>) {
    if let Some(i) = main.bytes().position(|b| b == b'.' || b == b',') {
        match (main.get(0..i), main.get(i + 1..)) {
            (Some(body), Some(digits))
                if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) =>
            {
                let frac = format!("0.{digits}").parse::<f64>().ok();
                (body, frac.or(Some(f64::NAN)))
            }
            // Malformed fraction: signal it as NaN so validate_time rejects it.
            _ => (main, Some(f64::NAN)),
        }
    } else {
        (main, None)
    }
}

/// Enforce the time validity invariants: `24:00:00` (and any hour 24) is
/// disallowed anywhere (`master06`), a partial time needs minute-before-second
/// (`Partial_validity`), and minute/second are range-checked. A `60` second is
/// rejected (`valid_second` Post `s < Seconds_in_minute`).
fn validate_time(
    hour: u32,
    minute: Option<u32>,
    second: Option<u32>,
    fractional_second: Option<f64>,
) -> Option<TimeParts> {
    if hour >= 24 {
        return None;
    }
    if let Some(m) = minute {
        if m >= 60 {
            return None;
        }
    } else if second.is_some() {
        return None; // second present but minute unknown — invalid partial
    }
    if let Some(sec) = second
        && sec >= 60
    {
        return None; // leap second `:60` rejected per valid_second
    }
    if let Some(f) = fractional_second
        && !(f.is_finite() && (0.0..1.0).contains(&f))
    {
        return None; // includes the NaN split_fraction uses to flag malformed input
    }
    Some((hour, minute, second, fractional_second))
}

/// Parse a timezone designator: `Z` → 0, `±hh[:mm]` / `±hh[mm]` → signed
/// minutes. `hh` is `00`..=`14` (`Time_definitions.Max_timezone_hour`), `mm`
/// `00`..=`59`.
fn parse_timezone(tz: &str) -> Option<i32> {
    if tz == "Z" {
        return Some(0);
    }
    let sign = match tz.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = tz.get(1..)?;
    let (hh, mm) = if let Some((h, m)) = rest.split_once(':') {
        (parse_fixed(h, 2)?, parse_fixed(m, 2)?)
    } else {
        match rest.len() {
            2 => (parse_fixed(rest.get(0..2)?, 2)?, 0),
            4 => (
                parse_fixed(rest.get(0..2)?, 2)?,
                parse_fixed(rest.get(2..4)?, 2)?,
            ),
            _ => return None,
        }
    };
    if hh > 14 || mm > 59 {
        return None;
    }
    #[allow(clippy::cast_possible_wrap)] // hh<=14, mm<=59 — far inside i32
    let minutes = sign * (hh as i32 * 60 + mm as i32);
    Some(minutes)
}

// ── Date/time parsing ──────────────────────────────────────────────────────────

/// Parse an ISO 8601 date/time: a date, optionally followed by `T` and a time.
pub(crate) fn parse_date_time(s: &str) -> Option<ParsedDateTime> {
    if let Some((d, t)) = s.split_once('T') {
        let date = parse_date(d)?;
        let time = parse_time(t)?;
        Some(ParsedDateTime {
            date,
            time: Some(time),
        })
    } else {
        Some(ParsedDateTime {
            date: parse_date(s)?,
            time: None,
        })
    }
}

// ── Duration parsing ────────────────────────────────────────────────────────────

/// Parse an ISO 8601 duration `[-]P[nY][nM][nW][nD][T[nH][nM][nS(.f)]]`
/// (`Time_definitions.valid_iso8601_duration`, with the openEHR deviations: a
/// leading `-` and `W` mixable with other designators). Requires at least one
/// component; only the seconds field may carry a fraction.
pub(crate) fn parse_duration(s: &str) -> Option<ParsedDuration> {
    let mut it = s.chars().peekable();
    let negative = match it.peek() {
        Some('-') => {
            it.next();
            true
        }
        _ => false,
    };
    if it.next()? != 'P' {
        return None;
    }
    let mut d = ParsedDuration {
        negative,
        years: 0,
        months: 0,
        weeks: 0,
        days: 0,
        hours: 0,
        minutes: 0,
        seconds: 0,
        fractional_seconds: 0.0,
    };
    let mut in_time = false;
    let mut seen_any = false;
    loop {
        match it.peek() {
            None => break,
            Some('T') => {
                if in_time {
                    return None;
                }
                in_time = true;
                it.next();
            }
            Some(c) if c.is_ascii_digit() => {
                let mut num = String::new();
                while let Some(c) = it.peek() {
                    if c.is_ascii_digit() {
                        num.push(*c);
                        it.next();
                    } else {
                        break;
                    }
                }
                let mut frac = String::new();
                let has_frac = matches!(it.peek(), Some('.' | ','));
                if has_frac {
                    it.next();
                    while let Some(c) = it.peek() {
                        if c.is_ascii_digit() {
                            frac.push(*c);
                            it.next();
                        } else {
                            break;
                        }
                    }
                }
                let designator = it.next()?;
                let intval = num.parse::<u64>().ok()?;
                let fracval = if has_frac {
                    if frac.is_empty() {
                        return None;
                    }
                    format!("0.{frac}").parse::<f64>().ok()?
                } else {
                    0.0
                };
                match (in_time, designator) {
                    (false, 'Y') => d.years = intval,
                    (false, 'M') => d.months = intval,
                    (false, 'W') => d.weeks = intval,
                    (false, 'D') => d.days = intval,
                    (true, 'H') => d.hours = intval,
                    (true, 'M') => d.minutes = intval,
                    (true, 'S') => {
                        d.seconds = intval;
                        d.fractional_seconds = fracval;
                    }
                    _ => return None,
                }
                if has_frac && !(in_time && designator == 'S') {
                    return None; // only the seconds field carries a fraction
                }
                seen_any = true;
            }
            _ => return None,
        }
    }
    if !seen_any {
        return None; // `P`/`PT` with no components is not a duration
    }
    Some(d)
}

impl ParsedDuration {
    /// Total seconds equivalent, sign applied
    /// (`Iso8601_duration.to_seconds`): non-definite years/months reduce via
    /// the `Time_definitions` average-length constants.
    #[allow(clippy::cast_precision_loss)] // duration counts are small; f64 is exact for them
    pub(crate) fn to_seconds(self) -> f64 {
        let magnitude = self.years as f64 * AVERAGE_DAYS_IN_YEAR * SECONDS_IN_DAY
            + self.months as f64 * AVERAGE_DAYS_IN_MONTH * SECONDS_IN_DAY
            + self.weeks as f64 * DAYS_IN_WEEK * SECONDS_IN_DAY
            + self.days as f64 * SECONDS_IN_DAY
            + self.hours as f64 * SECONDS_IN_HOUR
            + self.minutes as f64 * SECONDS_IN_MINUTE
            + self.seconds as f64
            + self.fractional_seconds;
        if self.negative { -magnitude } else { magnitude }
    }
}

// ── Completion-range comparison (range semantics over partials) ──────────────────

/// The set of instants a (possibly partial) time value denotes, as a half-open
/// second interval `[lo, hi)` when `upper_open` (a partial spans a range) or a
/// degenerate point `[lo, lo]` when not (a fully-specified instant). `lo`/`hi`
/// share whatever axis the builder chose (seconds-of-day for `Iso8601_time`,
/// absolute seconds for `Iso8601_date_time`); comparison only ever happens
/// between ranges on the same axis.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CompletionRange {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
    pub(crate) upper_open: bool,
}

/// True if every instant of `x` precedes every instant of `y` — the
/// range-semantics `<` (our own design; see the module NOTE). The lower bound
/// is always inclusive; `x`'s upper bound is exclusive exactly when `x` is a
/// partial (open) range, so two ranges meeting at a single boundary value are
/// separated only if the boundary is not actually attained by `x`.
pub(crate) fn range_before(x: &CompletionRange, y: &CompletionRange) -> bool {
    match x.hi.partial_cmp(&y.lo) {
        Some(Ordering::Less) => true,
        Some(Ordering::Equal) => x.upper_open,
        _ => false,
    }
}

/// The completion range of a time-of-day, on the seconds-of-day axis, shifted
/// to UTC when the value is zoned (a uniform shift preserves ordering; a bare
/// zoned time may land outside `[0, 86400)`, which is fine as a comparison
/// key). See `iso8601_time_impl.rs` for the timezone-compatibility rule.
#[allow(clippy::cast_precision_loss)] // hour/minute/second are small; f64 is exact
pub(crate) fn time_completion_range(t: &ParsedTime) -> CompletionRange {
    let base = f64::from(t.hour) * SECONDS_IN_HOUR;
    let (lo, hi, upper_open) = match (t.minute, t.second) {
        (Some(m), Some(s)) => {
            let point = base
                + f64::from(m) * SECONDS_IN_MINUTE
                + f64::from(s)
                + t.fractional_second.unwrap_or(0.0);
            (point, point, false)
        }
        (Some(m), None) => {
            let start = base + f64::from(m) * SECONDS_IN_MINUTE;
            (start, start + SECONDS_IN_MINUTE, true)
        }
        _ => (base, base + SECONDS_IN_HOUR, true),
    };
    let shift = t
        .timezone
        .map_or(0.0, |off| -f64::from(off) * SECONDS_IN_MINUTE);
    CompletionRange {
        lo: lo + shift,
        hi: hi + shift,
        upper_open,
    }
}

/// The completion range of a date/time on the absolute-seconds axis (days from
/// the civil epoch × 86400 + time-of-day), shifted to UTC when zoned. The
/// coarsest unknown component sets the range width — an unknown month spans the
/// year, an unknown day spans the month, a missing time spans the day — so a
/// timezone shift that crosses midnight rolls the date automatically (it is
/// plain arithmetic on the absolute axis). The definite-calendar day counts use
/// real month lengths + leap years.
///
/// NOTE: representing a zoned date/time on a single absolute-seconds axis so a
/// timezone offset can roll the calendar date is our own design/extension — no
/// openEHR spec governs partial-temporal ordering or timezone normalisation.
#[allow(clippy::cast_precision_loss)] // day counts + components are small; f64 is exact
pub(crate) fn date_time_completion_range(dt: &ParsedDateTime) -> CompletionRange {
    let d = &dt.date;
    let (lo, hi, upper_open) = match (d.month, d.day) {
        (None, _) => (
            year_start_seconds(d.year),
            year_start_seconds(d.year + 1),
            true,
        ),
        (Some(m), None) => {
            let (ny, nm) = if m == 12 {
                (d.year + 1, 1)
            } else {
                (d.year, m + 1)
            };
            (
                month_start_seconds(d.year, m),
                month_start_seconds(ny, nm),
                true,
            )
        }
        (Some(m), Some(day)) => {
            let day_start = days_from_civil(d.year, m, day) as f64 * SECONDS_IN_DAY;
            match &dt.time {
                None => (day_start, day_start + SECONDS_IN_DAY, true),
                Some(t) => intraday_range(day_start, t),
            }
        }
    };
    let shift = dt
        .time
        .and_then(|t| t.timezone)
        .map_or(0.0, |off| -f64::from(off) * SECONDS_IN_MINUTE);
    CompletionRange {
        lo: lo + shift,
        hi: hi + shift,
        upper_open,
    }
}

/// Absolute seconds at `year`-01-01 00:00:00.
#[allow(clippy::cast_precision_loss)] // day count is small; f64 is exact
fn year_start_seconds(year: u32) -> f64 {
    days_from_civil(year, 1, 1) as f64 * SECONDS_IN_DAY
}

/// Absolute seconds at `year`-`month`-01 00:00:00.
#[allow(clippy::cast_precision_loss)] // day count is small; f64 is exact
fn month_start_seconds(year: u32, month: u32) -> f64 {
    days_from_civil(year, month, 1) as f64 * SECONDS_IN_DAY
}

/// The intraday completion range (relative to `day_start`) for a known-day
/// date/time's time part, mirroring [`time_completion_range`]'s partial widths.
#[allow(clippy::cast_precision_loss)] // hour/minute/second are small; f64 is exact
fn intraday_range(day_start: f64, t: &ParsedTime) -> (f64, f64, bool) {
    let base = day_start + f64::from(t.hour) * SECONDS_IN_HOUR;
    match (t.minute, t.second) {
        (Some(m), Some(s)) => {
            let point = base
                + f64::from(m) * SECONDS_IN_MINUTE
                + f64::from(s)
                + t.fractional_second.unwrap_or(0.0);
            (point, point, false)
        }
        (Some(m), None) => {
            let start = base + f64::from(m) * SECONDS_IN_MINUTE;
            (start, start + SECONDS_IN_MINUTE, true)
        }
        _ => (base, base + SECONDS_IN_HOUR, true),
    }
}
