//! Hand-written `Iso8601_duration` spec behaviour: the accessor functions
//! (component counts, `to_seconds`, `is_partial`) and a `PartialOrd` ordering
//! durations by their total-seconds reduction.
//!
//! Spec sources (vendored):
//! - `BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_duration.adoc`
//!   (§Functions: the component accessors, `to_seconds`, `is_partial`; the
//!   `add`/`subtract` semantics that reduce via `to_seconds`).
//! - `BASE/docs/UML/classes/org.openehr.base.foundation_types.time_definitions.adoc`
//!   (§Constants `Average_days_in_year`/`Average_days_in_month` used by
//!   `to_seconds`).
//!
//! NOTE: the openEHR spec gives no duration comparison, but it DOES sanction
//! reducing a duration to a scalar via `to_seconds` (with the average-length
//! constants) for its own `add`/`subtract`. Ordering by that same scalar is our
//! own design/extension on that sanctioned reduction, so `P1M` (30.42 days) >
//! `P30D`. `partial_cmp` returns `Some(Equal)` ONLY for equal raw strings:
//! durations of equal magnitude but different spellings (`PT1H30M` vs `PT90M`)
//! are reported incomparable (`None`), consistent with the derived `PartialEq`.

use std::cmp::Ordering;

use super::iso8601_duration::Iso8601Duration;
use super::iso8601_parse::{ParsedDuration, parse_duration};

impl Iso8601Duration {
    /// Parsed components, or `None` when `value` is not a valid ISO 8601
    /// duration.
    fn parsed(&self) -> Option<ParsedDuration> {
        parse_duration(&self.value)
    }

    /// `Iso8601_duration.years`, or `None` when the value does not parse.
    #[must_use]
    pub fn years(&self) -> Option<u64> {
        self.parsed().map(|p| p.years)
    }

    /// `Iso8601_duration.months`, or `None` when the value does not parse.
    #[must_use]
    pub fn months(&self) -> Option<u64> {
        self.parsed().map(|p| p.months)
    }

    /// `Iso8601_duration.weeks`, or `None` when the value does not parse.
    #[must_use]
    pub fn weeks(&self) -> Option<u64> {
        self.parsed().map(|p| p.weeks)
    }

    /// `Iso8601_duration.days`, or `None` when the value does not parse.
    #[must_use]
    pub fn days(&self) -> Option<u64> {
        self.parsed().map(|p| p.days)
    }

    /// `Iso8601_duration.hours`, or `None` when the value does not parse.
    #[must_use]
    pub fn hours(&self) -> Option<u64> {
        self.parsed().map(|p| p.hours)
    }

    /// `Iso8601_duration.minutes`, or `None` when the value does not parse.
    #[must_use]
    pub fn minutes(&self) -> Option<u64> {
        self.parsed().map(|p| p.minutes)
    }

    /// `Iso8601_duration.seconds` (integral), or `None` when the value does not
    /// parse.
    #[must_use]
    pub fn seconds(&self) -> Option<u64> {
        self.parsed().map(|p| p.seconds)
    }

    /// `Iso8601_duration.fractional_seconds`, or `None` when the value does not
    /// parse.
    #[must_use]
    pub fn fractional_seconds(&self) -> Option<f64> {
        self.parsed().map(|p| p.fractional_seconds)
    }

    /// True when the value carries a leading negative sign (an openEHR
    /// deviation — `master06` §Primitive Time Types), or `None` when the value
    /// does not parse.
    #[must_use]
    pub fn is_negative(&self) -> Option<bool> {
        self.parsed().map(|p| p.negative)
    }

    /// `Iso8601_duration.is_partial`: always `False` for a duration (effected
    /// in the spec), or `None` when the value does not parse.
    #[must_use]
    pub fn is_partial(&self) -> Option<bool> {
        self.parsed().map(|_| false)
    }

    /// `Iso8601_duration.to_seconds`: total seconds equivalent (sign applied),
    /// with years/months reduced via `Time_definitions.Average_days_in_year`
    /// / `Average_days_in_month`. `None` when the value does not parse.
    #[must_use]
    pub fn to_seconds(&self) -> Option<f64> {
        self.parsed().map(ParsedDuration::to_seconds)
    }
}

impl PartialOrd for Iso8601Duration {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.value == other.value {
            return Some(Ordering::Equal); // consistent with the derived PartialEq
        }
        let a = self.parsed()?.to_seconds();
        let b = other.parsed()?.to_seconds();
        match a.partial_cmp(&b)? {
            // Equal magnitude but (given the string check above) different
            // spellings ⇒ incomparable per decision 4, not equal.
            Ordering::Equal => None,
            ord => Some(ord),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // test assertions
mod tests {
    use super::*;

    fn dur(v: &str) -> Iso8601Duration {
        Iso8601Duration {
            value: v.to_owned(),
        }
    }

    // ── ordering by total-seconds reduction ──────────────────────────────────

    #[test]
    fn nominal_month_exceeds_thirty_days() {
        // P1M = 30.42 days > P30D (decision 5).
        assert_eq!(
            dur("P1M").partial_cmp(&dur("P30D")),
            Some(Ordering::Greater)
        );
        assert_eq!(
            dur("P1Y").partial_cmp(&dur("P365D")),
            Some(Ordering::Greater)
        ); // 365.24 > 365
    }

    #[test]
    fn plain_ordering() {
        assert_eq!(dur("PT30M").partial_cmp(&dur("PT1H")), Some(Ordering::Less));
        assert_eq!(dur("P2W").partial_cmp(&dur("P7D")), Some(Ordering::Greater));
    }

    #[test]
    fn equal_strings_are_equal() {
        assert_eq!(
            dur("PT1H30M").partial_cmp(&dur("PT1H30M")),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn equal_magnitude_different_spelling_is_incomparable() {
        // PT1H30M and PT90M are both 5400 s but written differently ⇒ None.
        assert_eq!(dur("PT1H30M").partial_cmp(&dur("PT90M")), None);
    }

    #[test]
    fn negative_durations_order_below_positive() {
        assert_eq!(dur("-P3M").partial_cmp(&dur("P0D")), Some(Ordering::Less));
        assert_eq!(dur("-P1Y").partial_cmp(&dur("-P1M")), Some(Ordering::Less));
        assert_eq!(
            dur("P1D").partial_cmp(&dur("-P1D")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn fractional_seconds_order() {
        assert_eq!(
            dur("PT0.5S").partial_cmp(&dur("PT1S")),
            Some(Ordering::Less)
        );
    }

    // ── malformed / weeks-mixing ─────────────────────────────────────────────

    #[test]
    fn weeks_mixed_with_other_designators_parse() {
        // openEHR deviation: W mixable with other designators.
        assert_eq!(
            dur("P1W3D").to_seconds(),
            Some(10.0 * super::super::iso8601_parse::SECONDS_IN_DAY)
        );
        assert_eq!(dur("P2W").partial_cmp(&dur("P1W")), Some(Ordering::Greater));
    }

    #[test]
    fn malformed_values_are_incomparable() {
        assert_eq!(dur("nonsense").partial_cmp(&dur("P1D")), None);
        assert_eq!(dur("P").partial_cmp(&dur("P1D")), None); // no components
        assert_eq!(dur("PT").partial_cmp(&dur("P1D")), None);
        assert_eq!(dur("1D").partial_cmp(&dur("P1D")), None); // missing leading P
        assert!(dur("P1.5Y").years().is_none()); // fraction only allowed on seconds
    }

    // ── accessors ────────────────────────────────────────────────────────────

    #[test]
    fn accessors_report_components() {
        let d = dur("P1Y2M3W4DT5H6M7.5S");
        assert_eq!(d.years(), Some(1));
        assert_eq!(d.months(), Some(2));
        assert_eq!(d.weeks(), Some(3));
        assert_eq!(d.days(), Some(4));
        assert_eq!(d.hours(), Some(5));
        assert_eq!(d.minutes(), Some(6));
        assert_eq!(d.seconds(), Some(7));
        assert_eq!(d.fractional_seconds(), Some(0.5));
        assert_eq!(d.is_partial(), Some(false));
        assert_eq!(d.is_negative(), Some(false));
        assert_eq!(dur("-P3M").is_negative(), Some(true));
    }

    #[test]
    fn to_seconds_uses_average_lengths() {
        // PT1H = 3600 s.
        assert_eq!(dur("PT1H").to_seconds(), Some(3600.0));
        // P1D = 86400 s.
        assert_eq!(dur("P1D").to_seconds(), Some(86_400.0));
    }
}
