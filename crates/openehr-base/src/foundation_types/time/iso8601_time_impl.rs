//! Hand-written `Iso8601_time` spec behaviour: the accessor functions
//! (`is_partial`, `minute_unknown`, `second_unknown`,
//! `hour`/`minute`/`second`/`fractional_second`/`timezone`) and a `PartialOrd`
//! implementing range semantics over partial times with timezone
//! normalisation.
//!
//! Spec sources (vendored):
//! - `BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_time.adoc`
//!   (§Functions: the accessors; §Invariants).
//! - `BASE/docs/UML/classes/org.openehr.base.foundation_types.time_definitions.adoc`
//!   (§Functions `valid_second`, `valid_hour`).
//! - `BASE/docs/foundation_types/master06-time_types.adoc` (§Primitive Time
//!   Types: `24:00:00` disallowed anywhere).
//!
//! NOTE: `Time_definitions.valid_iso8601_time` prose allows a seconds field of
//! `"00"`-`"60"` (a leap second), but the machine-checkable `valid_second`
//! post-condition is `s < Seconds_in_minute` (i.e. `s <= 59`) and the
//! `Iso8601_time.Second_valid` invariant is stated in terms of `valid_second`
//! — an internal spec contradiction. We enforce the invariant: a `:60` second
//! is not a valid time, so it does not parse and compares as incomparable
//! (`None`).
//!
//! NOTE: the ordering algorithm is our own design/extension (the openEHR spec
//! gives none — see `iso8601_parse.rs`). A partial time denotes the interval of
//! its completions; both-zoned times are normalised to UTC (a uniform offset
//! shift preserving order); a zoned time and an unzoned (local) time cannot be
//! safely ordered and are incomparable. `partial_cmp` returns `Some(Equal)`
//! ONLY for equal raw strings.

use std::cmp::Ordering;

use super::iso8601_parse::{ParsedTime, parse_time, range_before, time_completion_range};
use super::iso8601_time::Iso8601Time;

impl Iso8601Time {
    /// Parsed components, or `None` when `value` is not a valid ISO 8601 time.
    fn parsed(&self) -> Option<ParsedTime> {
        parse_time(&self.value)
    }

    /// The hour part (`Iso8601_time.hour`), or `None` when the value does not
    /// parse. Hour is always present in a valid time.
    #[must_use]
    pub fn hour(&self) -> Option<u32> {
        self.parsed().map(|p| p.hour)
    }

    /// The minute part (`Iso8601_time.minute`), or `None` when minute is
    /// unknown or the value does not parse.
    #[must_use]
    pub fn minute(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.minute)
    }

    /// The integral seconds part (`Iso8601_time.second`), or `None` when second
    /// is unknown or the value does not parse.
    #[must_use]
    pub fn second(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.second)
    }

    /// The fractional seconds part (`Iso8601_time.fractional_second`), or `None`
    /// when absent/not significant or the value does not parse.
    #[must_use]
    pub fn fractional_second(&self) -> Option<f64> {
        self.parsed().and_then(|p| p.fractional_second)
    }

    /// The timezone offset in signed minutes (`Iso8601_time.timezone` reduced to
    /// its offset; `Z` → `Some(0)`), or `None` when the value is unzoned or does
    /// not parse.
    #[must_use]
    pub fn timezone(&self) -> Option<i32> {
        self.parsed().and_then(|p| p.timezone)
    }

    /// `Iso8601_time.minute_unknown`: true when the value is of the form `hh`
    /// (or does not parse).
    #[must_use]
    pub fn minute_unknown(&self) -> bool {
        self.parsed().is_none_or(|p| p.minute.is_none())
    }

    /// `Iso8601_time.second_unknown`: true when the value omits the second (or
    /// does not parse).
    #[must_use]
    pub fn second_unknown(&self) -> bool {
        self.parsed().is_none_or(|p| p.second.is_none())
    }

    /// `Iso8601_time.is_partial`: true when seconds or more is missing (a value
    /// that does not parse is treated as not a complete time).
    #[must_use]
    pub fn is_partial(&self) -> bool {
        self.parsed().is_none_or(|p| p.second.is_none())
    }
}

/// Range-semantics comparison of two parsed times. `None` unless both are
/// zoned or both unzoned (a local time cannot be ordered against an absolute
/// one). Never returns `Some(Equal)` — equal strings are handled before parsing.
fn cmp_time(a: &ParsedTime, b: &ParsedTime) -> Option<Ordering> {
    match (a.timezone, b.timezone) {
        (Some(_), None) | (None, Some(_)) => return None,
        _ => {}
    }
    let ra = time_completion_range(a);
    let rb = time_completion_range(b);
    if range_before(&ra, &rb) {
        Some(Ordering::Less)
    } else if range_before(&rb, &ra) {
        Some(Ordering::Greater)
    } else {
        None
    }
}

impl PartialOrd for Iso8601Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.value == other.value {
            return Some(Ordering::Equal); // consistent with the derived PartialEq
        }
        cmp_time(&self.parsed()?, &other.parsed()?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // test assertions
mod tests {
    use super::*;

    fn time(v: &str) -> Iso8601Time {
        Iso8601Time {
            value: v.to_owned(),
        }
    }

    // ── full-vs-full ordering (incl. fractional seconds) ─────────────────────

    #[test]
    fn full_times_order_by_instant() {
        assert_eq!(
            time("09:00:00").partial_cmp(&time("17:30:00")),
            Some(Ordering::Less)
        );
        assert_eq!(
            time("23:59:59").partial_cmp(&time("00:00:01")),
            Some(Ordering::Greater)
        );
        assert_eq!(
            time("12:00:00").partial_cmp(&time("12:00:00")),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn fractional_seconds_break_ties() {
        assert_eq!(
            time("12:00:00.250").partial_cmp(&time("12:00:00.750")),
            Some(Ordering::Less)
        );
        assert_eq!(
            time("12:00:00.9").partial_cmp(&time("12:00:01")),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn same_fraction_written_differently_is_incomparable() {
        // Equal magnitude, different strings ⇒ None (decision 4).
        assert_eq!(time("12:00:00.5").partial_cmp(&time("12:00:00.50")), None);
        // ':00' seconds is a precise instant, distinct string from a fractional form.
        assert_eq!(time("12:00:00").partial_cmp(&time("12:00:00.000")), None);
    }

    // ── partial range semantics ──────────────────────────────────────────────

    #[test]
    fn partial_hour_before_later_full_time() {
        // 09 spans 09:00:00..10:00:00, entirely before 10:30.
        assert_eq!(
            time("09").partial_cmp(&time("10:30:00")),
            Some(Ordering::Less)
        );
        assert_eq!(
            time("11").partial_cmp(&time("10:30:00")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn overlapping_partial_times_are_incomparable() {
        // 09 spans the whole hour, overlapping 09:30.
        assert_eq!(time("09").partial_cmp(&time("09:30")), None);
        assert_eq!(time("09:30").partial_cmp(&time("09:30:15")), None);
    }

    #[test]
    fn equal_precision_partials_order() {
        assert_eq!(
            time("09:15").partial_cmp(&time("09:16")),
            Some(Ordering::Less)
        );
        assert_eq!(time("10").partial_cmp(&time("09")), Some(Ordering::Greater));
    }

    // ── timezone matrix ──────────────────────────────────────────────────────

    #[test]
    fn both_zoned_normalise_to_utc() {
        // 12:00Z vs 13:00+02:00 (= 11:00 UTC) ⇒ 12:00Z is later.
        assert_eq!(
            time("12:00:00Z").partial_cmp(&time("13:00:00+02:00")),
            Some(Ordering::Greater)
        );
        // +02:00 vs -05:00 across the local clock.
        assert_eq!(
            time("10:00:00+02:00").partial_cmp(&time("10:00:00-05:00")),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn compact_and_extended_timezone_forms_parse() {
        assert_eq!(
            time("120000Z").partial_cmp(&time("130000+0200")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn zoned_vs_unzoned_is_incomparable() {
        assert_eq!(time("12:00:00Z").partial_cmp(&time("12:00:00")), None);
        assert_eq!(time("12:00:00").partial_cmp(&time("12:00:00+02:00")), None);
    }

    // ── malformed / excluded forms ───────────────────────────────────────────

    #[test]
    fn leap_second_is_rejected() {
        assert_eq!(time("12:00:60").partial_cmp(&time("12:01:00")), None);
        assert!(time("12:00:60").second().is_none());
    }

    #[test]
    fn twenty_four_hundred_is_rejected() {
        assert_eq!(time("24:00:00").partial_cmp(&time("00:00:00")), None);
        assert!(time("24:00:00").hour().is_none());
    }

    #[test]
    fn malformed_values_are_incomparable() {
        assert_eq!(time("nonsense").partial_cmp(&time("12:00")), None);
        assert_eq!(time("12:60").partial_cmp(&time("12:00")), None); // minute 60 invalid
    }

    // ── accessors ────────────────────────────────────────────────────────────

    #[test]
    fn accessors_report_components_and_unknowns() {
        let full = time("13:45:30.5Z");
        assert_eq!(full.hour(), Some(13));
        assert_eq!(full.minute(), Some(45));
        assert_eq!(full.second(), Some(30));
        assert_eq!(full.fractional_second(), Some(0.5));
        assert_eq!(full.timezone(), Some(0));
        assert!(!full.minute_unknown());
        assert!(!full.second_unknown());
        assert!(!full.is_partial());

        let hour_only = time("13");
        assert_eq!(hour_only.hour(), Some(13));
        assert_eq!(hour_only.minute(), None);
        assert!(hour_only.minute_unknown());
        assert!(hour_only.second_unknown());
        assert!(hour_only.is_partial());

        assert_eq!(time("09:30:00+02:30").timezone(), Some(150));
        assert_eq!(time("09:30:00-05:00").timezone(), Some(-300));
    }
}
