//! Hand-written `Iso8601_date_time` spec behaviour: the accessor functions and
//! a `PartialOrd` implementing range semantics over partial date/times with
//! timezone normalisation (which may roll the calendar date).
//!
//! Spec sources (vendored):
//! - `BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_date_time.adoc`
//!   (§Functions: the accessors; §Invariants).
//! - `BASE/docs/foundation_types/master06-time_types.adoc` (§Primitive Time
//!   Types: partial date/times may omit any part down to the month;
//!   `24:00:00` disallowed).
//!
//! NOTE: the ordering algorithm is our own design/extension (the openEHR spec
//! gives none — see `iso8601_parse.rs`). A partial date/time denotes the
//! interval of its completions on an absolute-seconds axis; both-zoned values
//! are normalised to UTC (a uniform offset shift that may cross midnight and
//! roll the day/month/year, computed with real calendar lengths); a zoned value
//! and an unzoned one are incomparable. `partial_cmp` returns `Some(Equal)`
//! ONLY for equal raw strings — a compact and an extended spelling of the same
//! instant are reported incomparable, never equal.

use std::cmp::Ordering;

use super::iso8601_date_time::Iso8601DateTime;
use super::iso8601_parse::{
    ParsedDateTime, date_time_completion_range, parse_date_time, range_before,
};

impl Iso8601DateTime {
    /// Parsed components, or `None` when `value` is not a valid ISO 8601
    /// date/time.
    fn parsed(&self) -> Option<ParsedDateTime> {
        parse_date_time(&self.value)
    }

    /// The year part (`Iso8601_date_time.year`), or `None` when the value does
    /// not parse.
    #[must_use]
    pub fn year(&self) -> Option<u32> {
        self.parsed().map(|p| p.date.year)
    }

    /// The month part, or `None` when unknown or the value does not parse.
    #[must_use]
    pub fn month(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.date.month)
    }

    /// The day part, or `None` when unknown or the value does not parse.
    #[must_use]
    pub fn day(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.date.day)
    }

    /// The hour part, or `None` when the time part is absent or the value does
    /// not parse.
    #[must_use]
    pub fn hour(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.time.map(|t| t.hour))
    }

    /// The minute part, or `None` when unknown/absent or the value does not
    /// parse.
    #[must_use]
    pub fn minute(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.time.and_then(|t| t.minute))
    }

    /// The integral seconds part, or `None` when unknown/absent or the value
    /// does not parse.
    #[must_use]
    pub fn second(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.time.and_then(|t| t.second))
    }

    /// The fractional seconds part, or `None` when absent or the value does not
    /// parse.
    #[must_use]
    pub fn fractional_second(&self) -> Option<f64> {
        self.parsed()
            .and_then(|p| p.time.and_then(|t| t.fractional_second))
    }

    /// The timezone offset in signed minutes (`Z` → `Some(0)`), or `None` when
    /// unzoned/timeless or the value does not parse.
    #[must_use]
    pub fn timezone(&self) -> Option<i32> {
        self.parsed().and_then(|p| p.time.and_then(|t| t.timezone))
    }

    /// `Iso8601_date_time.month_unknown` (or the value does not parse).
    #[must_use]
    pub fn month_unknown(&self) -> bool {
        self.parsed().is_none_or(|p| p.date.month.is_none())
    }

    /// `Iso8601_date_time.day_unknown` (or the value does not parse).
    #[must_use]
    pub fn day_unknown(&self) -> bool {
        self.parsed().is_none_or(|p| p.date.day.is_none())
    }

    /// `Iso8601_date_time.minute_unknown`: the minute is absent (no time part,
    /// or a time of the form `hh`), or the value does not parse.
    #[must_use]
    pub fn minute_unknown(&self) -> bool {
        self.parsed()
            .is_none_or(|p| p.time.and_then(|t| t.minute).is_none())
    }

    /// `Iso8601_date_time.second_unknown`: the second is absent, or the value
    /// does not parse.
    #[must_use]
    pub fn second_unknown(&self) -> bool {
        self.parsed()
            .is_none_or(|p| p.time.and_then(|t| t.second).is_none())
    }

    /// `Iso8601_date_time.is_partial`: true when seconds or more is missing (a
    /// value that does not parse is treated as not complete).
    #[must_use]
    pub fn is_partial(&self) -> bool {
        self.parsed()
            .is_none_or(|p| p.time.and_then(|t| t.second).is_none())
    }
}

/// True when the value carries a timezone (a time part with an offset).
fn is_zoned(dt: &ParsedDateTime) -> bool {
    dt.time.and_then(|t| t.timezone).is_some()
}

/// Range-semantics comparison of two parsed date/times. `None` unless both are
/// zoned or both unzoned. Never returns `Some(Equal)` — equal strings are
/// handled before parsing.
fn cmp_date_time(a: &ParsedDateTime, b: &ParsedDateTime) -> Option<Ordering> {
    if is_zoned(a) != is_zoned(b) {
        return None;
    }
    let ra = date_time_completion_range(a);
    let rb = date_time_completion_range(b);
    if range_before(&ra, &rb) {
        Some(Ordering::Less)
    } else if range_before(&rb, &ra) {
        Some(Ordering::Greater)
    } else {
        None
    }
}

impl PartialOrd for Iso8601DateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.value == other.value {
            return Some(Ordering::Equal); // consistent with the derived PartialEq
        }
        cmp_date_time(&self.parsed()?, &other.parsed()?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // test assertions
mod tests {
    use super::*;

    fn dt(v: &str) -> Iso8601DateTime {
        Iso8601DateTime {
            value: v.to_owned(),
        }
    }

    // ── full-vs-full ordering ────────────────────────────────────────────────

    #[test]
    fn full_date_times_order_by_instant() {
        assert_eq!(
            dt("2020-06-15T09:00:00").partial_cmp(&dt("2020-06-15T17:30:00")),
            Some(Ordering::Less)
        );
        assert_eq!(
            dt("2020-06-15T00:00:00").partial_cmp(&dt("2020-06-14T23:59:59")),
            Some(Ordering::Greater)
        );
        assert_eq!(
            dt("2020-06-15T12:00:00").partial_cmp(&dt("2020-06-15T12:00:00")),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn fractional_seconds_break_ties() {
        assert_eq!(
            dt("2020-06-15T12:00:00.25").partial_cmp(&dt("2020-06-15T12:00:00.75")),
            Some(Ordering::Less)
        );
    }

    // ── partial range semantics ──────────────────────────────────────────────

    #[test]
    fn partial_before_separated_full() {
        // 2020-06 spans June, entirely before July 1st.
        assert_eq!(
            dt("2020-06").partial_cmp(&dt("2020-07-01T00:00:00")),
            Some(Ordering::Less)
        );
        // The year 2019 is entirely before any 2020 instant.
        assert_eq!(
            dt("2019").partial_cmp(&dt("2020-01-01T00:00:00")),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn overlapping_partials_are_incomparable() {
        assert_eq!(dt("2020-06").partial_cmp(&dt("2020-06-15T12:00:00")), None);
        assert_eq!(
            dt("2020-06-15T12").partial_cmp(&dt("2020-06-15T12:30:00")),
            None
        );
    }

    #[test]
    fn missing_time_spans_the_day() {
        // The bare date 2020-06-15 spans the whole day, overlapping noon.
        assert_eq!(
            dt("2020-06-15").partial_cmp(&dt("2020-06-15T12:00:00")),
            None
        );
        // ... and is entirely before the next day.
        assert_eq!(
            dt("2020-06-15").partial_cmp(&dt("2020-06-16T00:00:00")),
            Some(Ordering::Less)
        );
    }

    // ── timezone matrix (crossing day / month / year boundaries) ─────────────

    #[test]
    fn both_zoned_normalise_to_utc() {
        // 2020-06-15T12:00Z vs 2020-06-15T13:00+02:00 (= 11:00 UTC).
        assert_eq!(
            dt("2020-06-15T12:00:00Z").partial_cmp(&dt("2020-06-15T13:00:00+02:00")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn timezone_shift_rolls_across_midnight() {
        // 2020-06-15T23:00-02:00 = 2020-06-16T01:00 UTC, later than 2020-06-16T00:30Z.
        assert_eq!(
            dt("2020-06-15T23:00:00-02:00").partial_cmp(&dt("2020-06-16T00:30:00Z")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn timezone_shift_rolls_across_year_boundary() {
        // 2019-12-31T23:00-02:00 = 2020-01-01T01:00 UTC, later than 2020-01-01T00:00Z.
        assert_eq!(
            dt("2019-12-31T23:00:00-02:00").partial_cmp(&dt("2020-01-01T00:00:00Z")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn zoned_vs_unzoned_is_incomparable() {
        assert_eq!(
            dt("2020-06-15T12:00:00Z").partial_cmp(&dt("2020-06-15T12:00:00")),
            None
        );
    }

    // ── mixed / malformed ────────────────────────────────────────────────────

    #[test]
    fn compact_vs_extended_same_instant_is_incomparable() {
        assert_eq!(
            dt("20200615T120000").partial_cmp(&dt("2020-06-15T12:00:00")),
            None
        );
    }

    #[test]
    fn malformed_values_are_incomparable() {
        assert_eq!(dt("garbage").partial_cmp(&dt("2020-06-15T12:00:00")), None);
        assert_eq!(
            dt("2020-06-15T24:00:00").partial_cmp(&dt("2020-06-16T00:00:00")),
            None
        );
    }

    // ── accessors ────────────────────────────────────────────────────────────

    #[test]
    fn accessors_report_components_and_unknowns() {
        let full = dt("2020-06-15T13:45:30.5+02:00");
        assert_eq!(full.year(), Some(2020));
        assert_eq!(full.month(), Some(6));
        assert_eq!(full.day(), Some(15));
        assert_eq!(full.hour(), Some(13));
        assert_eq!(full.minute(), Some(45));
        assert_eq!(full.second(), Some(30));
        assert_eq!(full.fractional_second(), Some(0.5));
        assert_eq!(full.timezone(), Some(120));
        assert!(!full.is_partial());

        let date_only = dt("2020-06-15");
        assert_eq!(date_only.hour(), None);
        assert!(date_only.minute_unknown());
        assert!(date_only.second_unknown());
        assert!(date_only.is_partial());
        assert_eq!(date_only.timezone(), None);
    }
}
