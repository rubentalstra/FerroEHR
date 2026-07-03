//! Internal ISO 8601 parsing support for the openEHR BASE time classes.
//!
//! This is intentionally narrower than a general ISO 8601 parser: it accepts
//! the forms named by BASE 1.2.0 `Time_Definitions` and the five
//! `Iso8601_*` classes, including openEHR's duration deviations (leading
//! negative sign and mixed `W` with other duration components).
use crate::time::time_definitions::TimeDefinitions;
use jiff::civil::{Date, DateTime, Time};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedIso8601Date {
    pub year: i32,
    pub month: Option<i32>,
    pub day: Option<i32>,
    pub extended: bool,
}

impl ParsedIso8601Date {
    #[must_use]
    pub fn month_unknown(self) -> bool {
        self.month.is_none()
    }

    #[must_use]
    pub fn day_unknown(self) -> bool {
        self.day.is_none()
    }

    #[must_use]
    pub fn as_complete_jiff_date(self) -> Option<Date> {
        let year = i16::try_from(self.year).ok()?;
        let month = i8::try_from(self.month?).ok()?;
        let day = i8::try_from(self.day?).ok()?;
        Date::new(year, month, day).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedIso8601Timezone {
    pub sign: i32,
    pub hour: i32,
    pub minute: Option<i32>,
    pub extended: bool,
    pub is_zulu: bool,
}

impl ParsedIso8601Timezone {
    #[must_use]
    pub fn minute_unknown(self) -> bool {
        self.minute.is_none()
    }

    #[must_use]
    pub fn minute_value(self) -> i32 {
        self.minute.unwrap_or(0)
    }

    #[must_use]
    pub fn offset_minutes(self) -> i32 {
        self.sign * (self.hour * TimeDefinitions::MINUTES_IN_HOUR + self.minute_value())
    }

    #[must_use]
    pub fn is_gmt(self) -> bool {
        self.is_zulu || self.offset_minutes() == 0
    }

    #[must_use]
    pub fn as_iso8601_string(self) -> String {
        if self.is_zulu {
            return "Z".to_string();
        }
        let sign = if self.sign < 0 { '-' } else { '+' };
        match (self.minute, self.extended) {
            (Some(minute), true) => format!("{sign}{:02}:{minute:02}", self.hour),
            (Some(minute), false) => format!("{sign}{:02}{minute:02}", self.hour),
            (None, _) => format!("{sign}{:02}", self.hour),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedIso8601Time {
    pub hour: i32,
    pub minute: Option<i32>,
    pub second: Option<i32>,
    pub nanosecond: i32,
    pub has_fractional_second: bool,
    pub decimal_sign_comma: bool,
    pub timezone: Option<ParsedIso8601Timezone>,
    pub extended: bool,
}

impl ParsedIso8601Time {
    #[must_use]
    pub fn minute_unknown(self) -> bool {
        self.minute.is_none()
    }

    #[must_use]
    pub fn second_unknown(self) -> bool {
        self.second.is_none()
    }

    #[must_use]
    pub fn minute_value(self) -> i32 {
        self.minute.unwrap_or(0)
    }

    #[must_use]
    pub fn second_value(self) -> i32 {
        self.second.unwrap_or(0)
    }

    #[must_use]
    pub fn fractional_second(self) -> f64 {
        f64::from(self.nanosecond) / 1_000_000_000.0
    }

    #[must_use]
    pub fn seconds_since_midnight(self) -> f64 {
        f64::from(self.hour * TimeDefinitions::MINUTES_IN_HOUR * TimeDefinitions::SECONDS_IN_MINUTE)
            + f64::from(self.minute_value() * TimeDefinitions::SECONDS_IN_MINUTE)
            + f64::from(self.second_value())
            + self.fractional_second()
    }

    #[must_use]
    pub fn as_jiff_time(self) -> Option<Time> {
        let hour = i8::try_from(self.hour).ok()?;
        let minute = i8::try_from(self.minute_value()).ok()?;
        let second = i8::try_from(self.second_value()).ok()?;
        Time::new(hour, minute, second, self.nanosecond).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedIso8601DateTime {
    pub date: ParsedIso8601Date,
    pub time: ParsedIso8601Time,
    pub extended: bool,
}

impl ParsedIso8601DateTime {
    #[must_use]
    pub fn as_jiff_datetime(self) -> Option<DateTime> {
        let date = self.date.as_complete_jiff_date()?;
        let time = self.time.as_jiff_time()?;
        Some(DateTime::from_parts(date, time))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParsedIso8601Duration {
    pub sign: i32,
    pub years: i32,
    pub months: i32,
    pub weeks: i32,
    pub days: i32,
    pub hours: i32,
    pub minutes: i32,
    pub seconds: i32,
    pub fractional_seconds: f64,
    pub decimal_sign_comma: bool,
}

impl ParsedIso8601Duration {
    #[must_use]
    pub fn to_seconds(self) -> f64 {
        let seconds_per_day = f64::from(
            TimeDefinitions::HOURS_IN_DAY
                * TimeDefinitions::MINUTES_IN_HOUR
                * TimeDefinitions::SECONDS_IN_MINUTE,
        );
        let unsigned = f64::from(self.years)
            * TimeDefinitions::AVERAGE_DAYS_IN_YEAR
            * seconds_per_day
            + f64::from(self.months) * TimeDefinitions::AVERAGE_DAYS_IN_MONTH * seconds_per_day
            + f64::from(self.weeks) * f64::from(TimeDefinitions::DAYS_IN_WEEK) * seconds_per_day
            + f64::from(self.days) * seconds_per_day
            + f64::from(
                self.hours * TimeDefinitions::MINUTES_IN_HOUR * TimeDefinitions::SECONDS_IN_MINUTE,
            )
            + f64::from(self.minutes * TimeDefinitions::SECONDS_IN_MINUTE)
            + f64::from(self.seconds)
            + self.fractional_seconds;
        f64::from(self.sign) * unsigned
    }
}

#[must_use]
pub fn parse_date(input: &str) -> Option<ParsedIso8601Date> {
    if input.is_empty() {
        return None;
    }
    if input.contains('-') {
        let mut parts = input.split('-');
        let year = parse_fixed_i32(parts.next()?, 4)?;
        let month = parse_optional_fixed_i32(parts.next(), 2)?;
        let day = parse_optional_fixed_i32(parts.next(), 2)?;
        if parts.next().is_some() {
            return None;
        }
        let parsed = ParsedIso8601Date {
            year,
            month,
            day,
            extended: true,
        };
        valid_parsed_date(parsed).then_some(parsed)
    } else {
        match input.len() {
            4 => {
                let parsed = ParsedIso8601Date {
                    year: parse_fixed_i32(input, 4)?,
                    month: None,
                    day: None,
                    extended: false,
                };
                valid_parsed_date(parsed).then_some(parsed)
            }
            6 => {
                let parsed = ParsedIso8601Date {
                    year: parse_fixed_i32(&input[0..4], 4)?,
                    month: Some(parse_fixed_i32(&input[4..6], 2)?),
                    day: None,
                    extended: false,
                };
                valid_parsed_date(parsed).then_some(parsed)
            }
            8 => {
                let parsed = ParsedIso8601Date {
                    year: parse_fixed_i32(&input[0..4], 4)?,
                    month: Some(parse_fixed_i32(&input[4..6], 2)?),
                    day: Some(parse_fixed_i32(&input[6..8], 2)?),
                    extended: false,
                };
                valid_parsed_date(parsed).then_some(parsed)
            }
            _ => None,
        }
    }
}

#[must_use]
pub fn parse_timezone(input: &str) -> Option<ParsedIso8601Timezone> {
    if input == "Z" {
        return Some(ParsedIso8601Timezone {
            sign: 1,
            hour: 0,
            minute: Some(0),
            extended: false,
            is_zulu: true,
        });
    }
    let mut chars = input.chars();
    let sign = match chars.next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let rest = chars.as_str();
    let parsed = if rest.contains(':') {
        let mut parts = rest.split(':');
        let hour = parse_fixed_i32(parts.next()?, 2)?;
        let minute = parse_optional_fixed_i32(parts.next(), 2)?;
        if parts.next().is_some() {
            return None;
        }
        ParsedIso8601Timezone {
            sign,
            hour,
            minute,
            extended: true,
            is_zulu: false,
        }
    } else {
        match rest.len() {
            2 => ParsedIso8601Timezone {
                sign,
                hour: parse_fixed_i32(rest, 2)?,
                minute: None,
                extended: false,
                is_zulu: false,
            },
            4 => ParsedIso8601Timezone {
                sign,
                hour: parse_fixed_i32(&rest[0..2], 2)?,
                minute: Some(parse_fixed_i32(&rest[2..4], 2)?),
                extended: false,
                is_zulu: false,
            },
            _ => return None,
        }
    };
    valid_parsed_timezone(parsed).then_some(parsed)
}

#[must_use]
pub fn parse_time(input: &str) -> Option<ParsedIso8601Time> {
    let (time_part, timezone) = split_timezone(input)?;
    if time_part.is_empty() {
        return None;
    }
    let extended = time_part.contains(':') || timezone.is_some_and(|tz| tz.extended);
    let (clock, fraction) = split_fraction(time_part)?;
    let (hour, minute, second) = if clock.contains(':') {
        let mut parts = clock.split(':');
        let hour = parse_fixed_i32(parts.next()?, 2)?;
        let minute = parse_optional_fixed_i32(parts.next(), 2)?;
        let second = parse_optional_fixed_i32(parts.next(), 2)?;
        if parts.next().is_some() || minute.is_none() {
            return None;
        }
        (hour, minute, second)
    } else {
        match clock.len() {
            2 => (parse_fixed_i32(clock, 2)?, None, None),
            4 => (
                parse_fixed_i32(&clock[0..2], 2)?,
                Some(parse_fixed_i32(&clock[2..4], 2)?),
                None,
            ),
            6 => (
                parse_fixed_i32(&clock[0..2], 2)?,
                Some(parse_fixed_i32(&clock[2..4], 2)?),
                Some(parse_fixed_i32(&clock[4..6], 2)?),
            ),
            _ => return None,
        }
    };
    if fraction.is_some() && second.is_none() {
        return None;
    }
    let (nanosecond, decimal_sign_comma) = match fraction {
        Some((sep, digits)) => (parse_fraction_to_nanos(digits)?, sep == ','),
        None => (0, false),
    };
    let parsed = ParsedIso8601Time {
        hour,
        minute,
        second,
        nanosecond,
        has_fractional_second: fraction.is_some(),
        decimal_sign_comma,
        timezone,
        extended,
    };
    valid_parsed_time(parsed).then_some(parsed)
}

#[must_use]
pub fn parse_date_time(input: &str) -> Option<ParsedIso8601DateTime> {
    let (date_part, time_part) = input.split_once('T')?;
    let date = parse_date(date_part)?;
    if date.month.is_none() || date.day.is_none() {
        return None;
    }
    let time = parse_time(time_part)?;
    let parsed = ParsedIso8601DateTime {
        date,
        time,
        extended: date.extended || time.extended,
    };
    parsed.as_jiff_datetime()?;
    Some(parsed)
}

#[must_use]
pub fn parse_duration(input: &str) -> Option<ParsedIso8601Duration> {
    let (sign, body) = if let Some(rest) = input.strip_prefix('-') {
        (-1, rest)
    } else {
        (1, input)
    };
    let body = body.strip_prefix('P')?;
    if body.is_empty() {
        return None;
    }
    let mut parsed = ParsedIso8601Duration {
        sign,
        years: 0,
        months: 0,
        weeks: 0,
        days: 0,
        hours: 0,
        minutes: 0,
        seconds: 0,
        fractional_seconds: 0.0,
        decimal_sign_comma: false,
    };
    let mut in_time = false;
    let mut seen_any = false;
    let mut seen_years = false;
    let mut seen_months = false;
    let mut seen_weeks = false;
    let mut seen_days = false;
    let mut seen_hours = false;
    let mut seen_minutes = false;
    let mut seen_seconds = false;
    let mut date_rank = 0;
    let mut time_rank = 0;
    let mut number_start = 0;
    let bytes = body.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'T' {
            if in_time || number_start != index {
                return None;
            }
            in_time = true;
            index += 1;
            number_start = index;
            continue;
        }
        if bytes[index].is_ascii_alphabetic() {
            let number = &body[number_start..index];
            if number.is_empty() {
                return None;
            }
            match (in_time, bytes[index]) {
                (false, b'Y') if !seen_years && date_rank < 1 => {
                    parsed.years = parse_unsigned_component(number)?;
                }
                (false, b'M') if !seen_months && date_rank < 2 => {
                    parsed.months = parse_unsigned_component(number)?;
                }
                (false, b'W') if !seen_weeks && date_rank < 3 => {
                    parsed.weeks = parse_unsigned_component(number)?;
                }
                (false, b'D') if !seen_days && date_rank < 4 => {
                    parsed.days = parse_unsigned_component(number)?;
                }
                (true, b'H') if !seen_hours && time_rank < 1 => {
                    parsed.hours = parse_unsigned_component(number)?;
                }
                (true, b'M') if !seen_minutes && time_rank < 2 => {
                    parsed.minutes = parse_unsigned_component(number)?;
                }
                (true, b'S') if !seen_seconds && time_rank < 3 => {
                    let (whole, fraction) = split_duration_seconds(number)?;
                    parsed.seconds = whole;
                    if let Some((sep, digits)) = fraction {
                        parsed.fractional_seconds =
                            f64::from(parse_fraction_to_nanos(digits)?) / 1_000_000_000.0;
                        parsed.decimal_sign_comma = sep == ',';
                    }
                }
                _ => return None,
            }
            match (in_time, bytes[index]) {
                (false, b'Y') => {
                    seen_years = true;
                    date_rank = 1;
                }
                (false, b'M') => {
                    seen_months = true;
                    date_rank = 2;
                }
                (false, b'W') => {
                    seen_weeks = true;
                    date_rank = 3;
                }
                (false, b'D') => {
                    seen_days = true;
                    date_rank = 4;
                }
                (true, b'H') => {
                    seen_hours = true;
                    time_rank = 1;
                }
                (true, b'M') => {
                    seen_minutes = true;
                    time_rank = 2;
                }
                (true, b'S') => {
                    seen_seconds = true;
                    time_rank = 3;
                }
                _ => {}
            }
            seen_any = true;
            index += 1;
            number_start = index;
            continue;
        }
        index += 1;
    }
    if number_start != body.len() || !seen_any {
        return None;
    }
    Some(parsed)
}

#[must_use]
pub fn days_since_origin(date: Date) -> i32 {
    let Ok(origin) = Date::new(1, 1, 1) else {
        return 0;
    };
    date.duration_since(origin).as_secs().div_euclid(86_400) as i32
}

#[must_use]
pub fn datetime_seconds_since_origin(datetime: DateTime) -> f64 {
    let Ok(origin) = DateTime::new(1, 1, 1, 0, 0, 0, 0) else {
        return 0.0;
    };
    let duration = datetime.duration_since(origin);
    duration.as_secs_f64()
}

#[must_use]
pub fn duration_from_seconds_string(seconds: f64) -> String {
    let sign = if seconds.is_sign_negative() { "-" } else { "" };
    let abs = seconds.abs();
    if abs.fract() == 0.0 {
        format!("{sign}PT{abs:.0}S")
    } else {
        let mut rendered = format!("{abs:.9}");
        while rendered.ends_with('0') {
            rendered.pop();
        }
        if rendered.ends_with('.') {
            rendered.push('0');
        }
        format!("{sign}PT{rendered}S")
    }
}

fn valid_parsed_date(parsed: ParsedIso8601Date) -> bool {
    if !TimeDefinitions::valid_year(parsed.year) {
        return false;
    }
    match (parsed.month, parsed.day) {
        (None, None) => true,
        (Some(month), None) => TimeDefinitions::valid_month(month),
        (Some(month), Some(day)) => TimeDefinitions::valid_day(parsed.year, month, day),
        (None, Some(_)) => false,
    }
}

fn valid_parsed_timezone(parsed: ParsedIso8601Timezone) -> bool {
    if parsed.sign != 1 && parsed.sign != -1 {
        return false;
    }
    if parsed.hour < 0 {
        return false;
    }
    if parsed.sign == -1 && parsed.hour == 0 {
        return false;
    }
    if parsed.sign == -1 && parsed.hour > TimeDefinitions::MIN_TIMEZONE_HOUR {
        return false;
    }
    if parsed.sign == 1 && parsed.hour > TimeDefinitions::MAX_TIMEZONE_HOUR {
        return false;
    }
    parsed.minute.is_none_or(TimeDefinitions::valid_minute)
}

fn valid_parsed_time(parsed: ParsedIso8601Time) -> bool {
    if parsed.hour < 0 || parsed.hour >= TimeDefinitions::HOURS_IN_DAY {
        return false;
    }
    if let Some(minute) = parsed.minute
        && !TimeDefinitions::valid_minute(minute)
    {
        return false;
    }
    if let Some(second) = parsed.second
        && !TimeDefinitions::valid_second(second)
    {
        return false;
    }
    !parsed.has_fractional_second || parsed.second.is_some()
}

fn split_timezone(input: &str) -> Option<(&str, Option<ParsedIso8601Timezone>)> {
    if let Some(time) = input.strip_suffix('Z') {
        return Some((time, parse_timezone("Z")));
    }
    let offset_start = input
        .char_indices()
        .skip(1)
        .find_map(|(idx, ch)| (ch == '+' || ch == '-').then_some(idx));
    match offset_start {
        Some(idx) => {
            let (time, tz) = input.split_at(idx);
            Some((time, Some(parse_timezone(tz)?)))
        }
        None => Some((input, None)),
    }
}

fn split_fraction(input: &str) -> Option<(&str, Option<(char, &str)>)> {
    let dot = input.find('.');
    let comma = input.find(',');
    let idx = match (dot, comma) {
        (Some(_), Some(_)) => return None,
        (Some(idx), None) | (None, Some(idx)) => idx,
        (None, None) => return Some((input, None)),
    };
    let (whole, fraction) = input.split_at(idx);
    let sep = fraction.chars().next()?;
    let digits = &fraction[sep.len_utf8()..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((whole, Some((sep, digits))))
}

fn split_duration_seconds(input: &str) -> Option<(i32, Option<(char, &str)>)> {
    let (whole, fraction) = split_fraction(input)?;
    Some((parse_unsigned_component(whole)?, fraction))
}

fn parse_fixed_i32(input: &str, len: usize) -> Option<i32> {
    if input.len() != len || !input.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    input.parse().ok()
}

fn parse_optional_fixed_i32(input: Option<&str>, len: usize) -> Option<Option<i32>> {
    input
        .map(|value| parse_fixed_i32(value, len))
        .map_or(Some(None), |value| value.map(Some))
}

fn parse_unsigned_component(input: &str) -> Option<i32> {
    if input.is_empty() || !input.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    input.parse().ok()
}

fn parse_fraction_to_nanos(digits: &str) -> Option<i32> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut nanos = 0_i32;
    let mut scale = 100_000_000_i32;
    for byte in digits.bytes().take(9) {
        nanos += i32::from(byte - b'0') * scale;
        scale /= 10;
    }
    Some(nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_forms_follow_base_spec() {
        assert!(parse_date("2024").is_some());
        assert!(parse_date("2024-02").is_some());
        assert!(parse_date("20240229").is_some());
        assert!(parse_date("2023-02-29").is_none());
    }

    #[test]
    fn time_forms_follow_base_spec() {
        let time = parse_time("10:30:05.250+02:30").unwrap();
        assert_eq!(time.hour, 10);
        assert_eq!(time.minute, Some(30));
        assert_eq!(time.second, Some(5));
        assert_eq!(time.fractional_second(), 0.25);
        assert_eq!(time.timezone.unwrap().offset_minutes(), 150);
        assert!(parse_time("24:00:00").is_none());
        assert!(parse_timezone("-00").is_none());
    }

    #[test]
    fn duration_supports_openehr_deviations() {
        let duration = parse_duration("-P1Y2M3W4DT5H6M7.5S").unwrap();
        assert_eq!(duration.sign, -1);
        assert_eq!(duration.years, 1);
        assert_eq!(duration.weeks, 3);
        assert_eq!(duration.fractional_seconds, 0.5);
        assert!(parse_duration("P").is_none());
        assert!(parse_duration("P1D1Y").is_none());
        assert!(parse_duration("P0Y0Y").is_none());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.time — implementation support for Time_Definitions and Iso8601_* classes
//   source_loc: n/a (parser utility)
//   confidence: medium
//   todos: 0
//   note: parser accepts the concrete ISO 8601 forms listed by BASE 1.2.0, plus openEHR's duration deviations; calendar validation delegates to jiff 0.2.31 civil types where the spec requires Gregorian correctness.
// ─────────────────────────────────────────────
