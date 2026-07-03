//! Internal ISO 8601 arithmetic support for the openEHR BASE time classes —
//! the "internal engine" the `Iso8601_date` / `Iso8601_time` /
//! `Iso8601_date_time` arithmetic functions delegate to.
//!
//! All calendrical math is backed by `jiff` civil (naive) types; no
//! day-count tables are hand-rolled here. The policies implemented are the
//! ones fixed by ADR-003 (docs/ADRs/ADR-003-spec-gap-policies.md):
//!
//! 1. **Definite arithmetic** (`add`/`subtract`/`diff`, spec-stated,
//!    master06-time_types.adoc §Computational Functions): an
//!    `Iso8601_duration` is an exact quantity — years and months convert to
//!    days via `Time_Definitions::AVERAGE_DAYS_IN_YEAR` (365.24) /
//!    `AVERAGE_DAYS_IN_MONTH` (30.42), weeks to 7 days, everything to
//!    seconds (this is exactly `Iso8601_duration.to_seconds()`), applied as
//!    an exact `jiff::SignedDuration`. `diff` returns a normalized duration
//!    in definite units only (days and below — never years/months, which
//!    are nominal units).
//! 2. **Nominal arithmetic** (`add_nominal`/`subtract_nominal`,
//!    spec-stated): years/months/weeks/days are calendar units applied via
//!    `jiff::Span` on civil values (jiff's end-of-month clamping is the
//!    accepted behaviour: `2004-01-31 ++ P1M = 2004-02-29`); sub-day
//!    components are exact time.
//! 3. **Partial-precision anchoring** (ADR-003 policy — spec silent):
//!    a partial receiver is anchored by filling each unknown component with
//!    its minimum (month → 01, day → 01, minute/second/fraction → 0), the
//!    computation runs on the anchored `jiff` civil value, and the result
//!    string is truncated back to the receiver's original precision.
//!    Timezone text, when present, is preserved verbatim; the arithmetic
//!    itself is civil — ISO 8601 partial values carry no zone *rules*, only
//!    a fixed offset, so there is no DST to apply.
use crate::time::iso8601_parser::{
    ParsedIso8601Date, ParsedIso8601DateTime, ParsedIso8601Duration, ParsedIso8601Time,
    ParsedIso8601Timezone,
};
use jiff::civil::{Date, DateTime, Time};
use jiff::{SignedDuration, Span};

/// Nanoseconds in one second, as the various integer widths used below.
const NANOS_PER_SECOND: i128 = 1_000_000_000;
const NANOS_PER_MINUTE: i128 = 60 * NANOS_PER_SECOND;
const NANOS_PER_HOUR: i128 = 60 * NANOS_PER_MINUTE;
const NANOS_PER_DAY: i128 = 24 * NANOS_PER_HOUR;

/// Anchor a (possibly partial) parsed date to a complete `jiff` civil date,
/// filling unknown month/day with `01` per ADR-003 policy 3.
#[must_use]
pub fn anchored_jiff_date(parsed: ParsedIso8601Date) -> Option<Date> {
    let year = i16::try_from(parsed.year).ok()?;
    let month = i8::try_from(parsed.month.unwrap_or(1)).ok()?;
    let day = i8::try_from(parsed.day.unwrap_or(1)).ok()?;
    Date::new(year, month, day).ok()
}

/// Convert an exact number of seconds (possibly fractional and/or negative)
/// to a `jiff::SignedDuration`, per ADR-003 policy 1. Returns `None` for
/// non-finite or out-of-range inputs.
#[must_use]
pub fn signed_duration_from_seconds(seconds: f64) -> Option<SignedDuration> {
    SignedDuration::try_from_secs_f64(seconds).ok()
}

/// Build the nominal `jiff::Span` for a parsed duration, per ADR-003
/// policy 2: years/months/weeks/days become calendar units, the sub-day
/// components exact time, and the duration's leading sign negates the whole
/// span. Returns `None` if any component is out of `jiff`'s span range.
#[must_use]
pub fn nominal_span(parsed: &ParsedIso8601Duration) -> Option<Span> {
    // PORT NOTE: the parser stores `fractional_seconds` as an unsigned f64
    // in [0, 1); rounding to whole nanoseconds here matches the parser's own
    // nanosecond resolution for time-of-day fractions.
    #[allow(clippy::cast_possible_truncation)]
    let fraction_nanos = (parsed.fractional_seconds * 1e9).round() as i64;
    let span = Span::new()
        .try_years(parsed.years)
        .ok()?
        .try_months(parsed.months)
        .ok()?
        .try_weeks(parsed.weeks)
        .ok()?
        .try_days(parsed.days)
        .ok()?
        .try_hours(parsed.hours)
        .ok()?
        .try_minutes(parsed.minutes)
        .ok()?
        .try_seconds(parsed.seconds)
        .ok()?
        .try_nanoseconds(fraction_nanos)
        .ok()?;
    Some(if parsed.sign < 0 { span.negate() } else { span })
}

/// Render an exact number of seconds as a normalized ISO 8601 duration
/// string in **definite units only** (days and below — never years or
/// months, which are nominal units), per ADR-003 policy 1. Fractional
/// seconds are kept at nanosecond resolution and trimmed of trailing zeros.
#[must_use]
pub fn definite_duration_string_from_seconds(seconds: f64) -> String {
    if !seconds.is_finite() {
        return "PT0S".to_string();
    }
    // PORT NOTE: decomposition runs on whole nanoseconds (i128) so the
    // day/hour/minute splits are exact; the f64→i128 rounding matches the
    // parser's nanosecond resolution for fractional seconds.
    #[allow(clippy::cast_possible_truncation)]
    let total_nanos = (seconds.abs() * 1e9).round() as i128;
    if total_nanos == 0 {
        return "PT0S".to_string();
    }
    let days = total_nanos / NANOS_PER_DAY;
    let mut rem = total_nanos % NANOS_PER_DAY;
    let hours = rem / NANOS_PER_HOUR;
    rem %= NANOS_PER_HOUR;
    let minutes = rem / NANOS_PER_MINUTE;
    rem %= NANOS_PER_MINUTE;
    let whole_seconds = rem / NANOS_PER_SECOND;
    let fraction_nanos = rem % NANOS_PER_SECOND;

    let mut out = String::new();
    if seconds < 0.0 {
        out.push('-');
    }
    out.push('P');
    if days > 0 {
        out.push_str(&format!("{days}D"));
    }
    if hours > 0 || minutes > 0 || whole_seconds > 0 || fraction_nanos > 0 {
        out.push('T');
        if hours > 0 {
            out.push_str(&format!("{hours}H"));
        }
        if minutes > 0 {
            out.push_str(&format!("{minutes}M"));
        }
        if fraction_nanos > 0 {
            out.push_str(&format!(
                "{whole_seconds}.{}S",
                fraction_digits(fraction_nanos)
            ));
        } else if whole_seconds > 0 {
            out.push_str(&format!("{whole_seconds}S"));
        }
    }
    out
}

/// Render a sub-second nanosecond count as trimmed decimal-fraction digits
/// (at least one digit, at most nine).
#[must_use]
pub fn fraction_digits(nanos: i128) -> String {
    let mut digits = format!("{nanos:09}");
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }
    digits
}

/// Format a timezone component. With `force_extended` the canonical
/// extended form (`±hh:mm`) is produced (for `as_string()`'s "extended
/// format" contract); otherwise the original text is reproduced verbatim
/// per ADR-003 policy 3 (arithmetic preserves timezone text unchanged).
#[must_use]
pub fn format_timezone(timezone: ParsedIso8601Timezone, force_extended: bool) -> String {
    if !force_extended {
        return timezone.as_iso8601_string();
    }
    if timezone.is_zulu {
        return "Z".to_string();
    }
    let sign = if timezone.sign < 0 { '-' } else { '+' };
    match timezone.minute {
        Some(minute) => format!("{sign}{:02}:{minute:02}", timezone.hour),
        None => format!("{sign}{:02}", timezone.hour),
    }
}

/// Format a computed `jiff` civil date back at the **receiver's** original
/// precision (year / year-month / full) and separator form, per ADR-003
/// policy 3. Returns `None` when the result year falls outside the four-digit
/// ISO 8601 range this crate's grammar accepts (`0000`–`9999`).
#[must_use]
pub fn format_date_at_precision(
    receiver: ParsedIso8601Date,
    result: Date,
    extended: bool,
) -> Option<String> {
    let year = i32::from(result.year());
    if !(0..=9999).contains(&year) {
        return None;
    }
    let month = result.month();
    let day = result.day();
    Some(match (receiver.month.is_some(), receiver.day.is_some()) {
        (false, _) => format!("{year:04}"),
        (true, false) if extended => format!("{year:04}-{month:02}"),
        (true, false) => format!("{year:04}{month:02}"),
        (true, true) if extended => format!("{year:04}-{month:02}-{day:02}"),
        (true, true) => format!("{year:04}{month:02}{day:02}"),
    })
}

/// Format a computed `jiff` civil time back at the **receiver's** original
/// precision (hour / hour-minute / full, with a fraction only if the
/// receiver had one) and separator form, per ADR-003 policy 3. The
/// receiver's timezone text is appended verbatim unless `force_extended_tz`
/// is set (the `as_string()` extended-format path).
#[must_use]
pub fn format_time_at_precision(
    receiver: ParsedIso8601Time,
    result: Time,
    extended: bool,
    force_extended_tz: bool,
) -> String {
    let mut out = format!("{:02}", result.hour());
    if receiver.minute.is_some() {
        if extended {
            out.push(':');
        }
        out.push_str(&format!("{:02}", result.minute()));
        if receiver.second.is_some() {
            if extended {
                out.push(':');
            }
            out.push_str(&format!("{:02}", result.second()));
            if receiver.has_fractional_second {
                out.push(if receiver.decimal_sign_comma {
                    ','
                } else {
                    '.'
                });
                out.push_str(&fraction_digits(i128::from(result.subsec_nanosecond())));
            }
        }
    }
    if let Some(timezone) = receiver.timezone {
        out.push_str(&format_timezone(timezone, force_extended_tz));
    }
    out
}

/// Format a computed `jiff` civil date-time back at the **receiver's**
/// original precision and separator form, per ADR-003 policy 3. The date
/// part of a valid `Iso8601_date_time` is always complete (the grammar
/// requires year, month, day, hour); only minute/second/fraction vary.
#[must_use]
pub fn format_date_time_at_precision(
    receiver: ParsedIso8601DateTime,
    result: DateTime,
    extended: bool,
    force_extended_tz: bool,
) -> Option<String> {
    let date_part = format_date_at_precision(receiver.date, result.date(), extended)?;
    let time_part =
        format_time_at_precision(receiver.time, result.time(), extended, force_extended_tz);
    Some(format!("{date_part}T{time_part}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::iso8601_parser::{parse_date, parse_duration, parse_time};

    #[test]
    fn definite_duration_string_normalizes_to_days_and_below() {
        // 30.42 days (the definite P1M) = 2 628 288 s.
        assert_eq!(
            definite_duration_string_from_seconds(2_628_288.0),
            "P30DT10H4M48S"
        );
        assert_eq!(
            definite_duration_string_from_seconds(-2_628_288.0),
            "-P30DT10H4M48S"
        );
        assert_eq!(definite_duration_string_from_seconds(0.0), "PT0S");
        assert_eq!(definite_duration_string_from_seconds(0.25), "PT0.25S");
        assert_eq!(definite_duration_string_from_seconds(86_400.0), "P1D");
        assert_eq!(definite_duration_string_from_seconds(5_400.0), "PT1H30M");
    }

    #[test]
    fn nominal_span_carries_all_components_and_sign() {
        let parsed = parse_duration("-P1Y2M3W4DT5H6M7.5S").unwrap();
        let span = nominal_span(&parsed).unwrap();
        assert_eq!(span.get_years(), -1);
        assert_eq!(span.get_months(), -2);
        assert_eq!(span.get_weeks(), -3);
        assert_eq!(span.get_days(), -4);
        assert_eq!(span.get_hours(), -5);
        assert_eq!(span.get_minutes(), -6);
        assert_eq!(span.get_seconds(), -7);
        assert_eq!(span.get_nanoseconds(), -500_000_000);
    }

    #[test]
    fn anchoring_fills_unknown_components_with_minimums() {
        let partial = parse_date("2004-02").unwrap();
        let anchored = anchored_jiff_date(partial).unwrap();
        assert_eq!(anchored, jiff::civil::date(2004, 2, 1));
        let year_only = parse_date("2004").unwrap();
        assert_eq!(
            anchored_jiff_date(year_only).unwrap(),
            jiff::civil::date(2004, 1, 1)
        );
    }

    #[test]
    fn date_formatting_truncates_to_receiver_precision() {
        let receiver = parse_date("2004-02").unwrap();
        let result = jiff::civil::date(2004, 3, 1);
        assert_eq!(
            format_date_at_precision(receiver, result, receiver.extended).unwrap(),
            "2004-03"
        );
        let compact = parse_date("200402").unwrap();
        assert_eq!(
            format_date_at_precision(compact, result, compact.extended).unwrap(),
            "200403"
        );
        // Out-of-grammar year is rejected, not rendered.
        let receiver = parse_date("0001").unwrap();
        assert!(format_date_at_precision(receiver, jiff::civil::date(-1, 1, 1), true).is_none());
    }

    #[test]
    fn time_formatting_preserves_timezone_text_verbatim() {
        let receiver = parse_time("10:30:05.250+0230").unwrap();
        let result = jiff::civil::time(11, 0, 5, 250_000_000);
        assert_eq!(
            format_time_at_precision(receiver, result, receiver.extended, false),
            "11:00:05.25+0230"
        );
        // as_string's extended path canonicalizes the offset separator.
        assert_eq!(
            format_time_at_precision(receiver, result, true, true),
            "11:00:05.25+02:30"
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — master06-time_types.adoc §Computational Functions, implemented per ADR-003 policies 1–3
//   source_loc: n/a (arithmetic engine utility)
//   confidence: high
//   todos: 0
//   note: definite arithmetic via to_seconds->SignedDuration (averages 365.24/30.42), nominal via jiff Span on civil values with end-of-month clamping, partial-precision anchoring (min-fill) + truncate-to-receiver-precision formatting, timezone text preserved verbatim; all calendrical math delegated to jiff 0.2.
// ─────────────────────────────────────────────
