//! Hand-written `Iso8601_date` spec behaviour: the accessor functions
//! (`is_partial`, `month_unknown`, `day_unknown`, `year`/`month`/`day`) and a
//! `PartialOrd` implementing range semantics over partial dates.
//!
//! Spec sources (vendored):
//! - `BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_date.adoc`
//!   (§Functions: the accessors; §Invariants).
//! - `BASE/docs/foundation_types/master06-time_types.adoc` (§Primitive Time
//!   Types: the accepted date forms; week dates excluded).
//!
//! NOTE: the openEHR spec gives NO date comparison algorithm (`Ordered` is
//! abstract, no `magnitude`), so the ordering here is our own design/extension:
//! a partial date denotes the interval of its completions, and `X < Y` holds
//! only when every completion of `X` precedes every completion of `Y` (decided
//! component-by-component on the shared prefix, undecidable once an unknown
//! component is reached). `partial_cmp` returns `Some(Equal)` ONLY for equal
//! raw strings — consistent with the derived `PartialEq` — so two
//! semantically-equal but differently-written values (compact `20200615` vs
//! extended `2020-06-15`) are reported as incomparable (`None`), never equal.

use std::cmp::Ordering;

use super::iso8601_date::Iso8601Date;
use super::iso8601_parse::{ParsedDate, parse_date};

impl Iso8601Date {
    /// Parsed components, or `None` when `value` is not a valid ISO 8601 date.
    fn parsed(&self) -> Option<ParsedDate> {
        parse_date(&self.value)
    }

    /// The year part (`Iso8601_date.year`), or `None` when the value does not
    /// parse. Year is always present in a valid date.
    #[must_use]
    pub fn year(&self) -> Option<u32> {
        self.parsed().map(|p| p.year)
    }

    /// The month part (`Iso8601_date.month`), or `None` when month is unknown
    /// or the value does not parse. (The spec's `month()` returns 0 when
    /// `month_unknown`; we report honest absence as `None`.)
    #[must_use]
    pub fn month(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.month)
    }

    /// The day part (`Iso8601_date.day`), or `None` when day is unknown or the
    /// value does not parse.
    #[must_use]
    pub fn day(&self) -> Option<u32> {
        self.parsed().and_then(|p| p.day)
    }

    /// `Iso8601_date.month_unknown`: true when the value is of the form `YYYY`
    /// (or does not parse).
    #[must_use]
    pub fn month_unknown(&self) -> bool {
        self.parsed().is_none_or(|p| p.month.is_none())
    }

    /// `Iso8601_date.day_unknown`: true when the value omits the day (or does
    /// not parse).
    #[must_use]
    pub fn day_unknown(&self) -> bool {
        self.parsed().is_none_or(|p| p.day.is_none())
    }

    /// `Iso8601_date.is_partial`: true when days or more is missing (a value
    /// that does not parse is treated as not a complete date).
    #[must_use]
    pub fn is_partial(&self) -> bool {
        self.parsed().is_none_or(|p| p.day.is_none())
    }
}

/// Range-semantics comparison of two parsed dates on their shared prefix. Never
/// returns `Some(Equal)`: an equal string is handled before parsing, so equal
/// components here (with differing strings) are incomparable (`None`).
fn cmp_date(a: &ParsedDate, b: &ParsedDate) -> Option<Ordering> {
    match a.year.cmp(&b.year) {
        Ordering::Equal => {}
        ord => return Some(ord),
    }
    match (a.month, b.month) {
        (Some(am), Some(bm)) => match am.cmp(&bm) {
            Ordering::Equal => {}
            ord => return Some(ord),
        },
        // One side's month is unknown while the years match: its completions
        // span the whole year and overlap the other's — undecidable.
        _ => return None,
    }
    match (a.day, b.day) {
        (Some(ad), Some(bd)) => match ad.cmp(&bd) {
            Ordering::Equal => None, // equal components, differing strings ⇒ incomparable
            ord => Some(ord),
        },
        _ => None,
    }
}

impl PartialOrd for Iso8601Date {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.value == other.value {
            return Some(Ordering::Equal); // consistent with the derived PartialEq
        }
        cmp_date(&self.parsed()?, &other.parsed()?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // test assertions
mod tests {
    use super::*;

    fn date(v: &str) -> Iso8601Date {
        Iso8601Date {
            value: v.to_owned(),
        }
    }

    // ── full-vs-full ordering ────────────────────────────────────────────────

    #[test]
    fn full_dates_order_component_wise() {
        assert_eq!(
            date("2020-01-01").partial_cmp(&date("2020-06-15")),
            Some(Ordering::Less)
        );
        assert_eq!(
            date("2020-12-31").partial_cmp(&date("2020-06-15")),
            Some(Ordering::Greater)
        );
        assert_eq!(
            date("2019-12-31").partial_cmp(&date("2020-01-01")),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn equal_strings_are_equal() {
        assert_eq!(
            date("2020-06-15").partial_cmp(&date("2020-06-15")),
            Some(Ordering::Equal)
        );
        assert_eq!(
            date("2020-06").partial_cmp(&date("2020-06")),
            Some(Ordering::Equal)
        );
        assert_eq!(
            date("2020").partial_cmp(&date("2020")),
            Some(Ordering::Equal)
        );
    }

    // ── partial range semantics ──────────────────────────────────────────────

    #[test]
    fn partial_year_before_full_date_when_separated() {
        // 2019 spans all of 2019, entirely before 2020-06-15.
        assert_eq!(
            date("2019").partial_cmp(&date("2020-06-15")),
            Some(Ordering::Less)
        );
        assert_eq!(
            date("2021").partial_cmp(&date("2020-06-15")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn overlapping_partials_are_incomparable() {
        // 2020 spans all of 2020, overlapping 2020-06-15 ⇒ undecidable.
        assert_eq!(date("2020").partial_cmp(&date("2020-06-15")), None);
        // 2020-06 spans June 2020, overlapping 2020-06-15.
        assert_eq!(date("2020-06").partial_cmp(&date("2020-06-15")), None);
    }

    #[test]
    fn equal_precision_partials_order() {
        assert_eq!(
            date("2020-06").partial_cmp(&date("2020-07")),
            Some(Ordering::Less)
        );
        assert_eq!(
            date("2020-08").partial_cmp(&date("2020-07")),
            Some(Ordering::Greater)
        );
        assert_eq!(
            date("2019").partial_cmp(&date("2020")),
            Some(Ordering::Less)
        );
    }

    // ── mixed compact / extended ─────────────────────────────────────────────

    #[test]
    fn compact_vs_extended_same_instant_is_incomparable() {
        // Same components, different strings ⇒ None (decision 4).
        assert_eq!(date("20200615").partial_cmp(&date("2020-06-15")), None);
    }

    #[test]
    fn compact_dates_order_among_themselves() {
        assert_eq!(
            date("20200615").partial_cmp(&date("20200616")),
            Some(Ordering::Less)
        );
        assert_eq!(
            date("20201231").partial_cmp(&date("20200101")),
            Some(Ordering::Greater)
        );
    }

    // ── malformed / excluded forms ───────────────────────────────────────────

    #[test]
    fn malformed_values_are_incomparable() {
        assert_eq!(date("not-a-date").partial_cmp(&date("2020")), None);
        assert_eq!(date("2020-13").partial_cmp(&date("2020-06")), None); // month 13 invalid
        assert_eq!(date("2020-02-30").partial_cmp(&date("2020-02-28")), None); // Feb 30 invalid
    }

    #[test]
    fn week_dates_are_rejected() {
        assert_eq!(date("2020-W01").partial_cmp(&date("2020-06-15")), None);
        assert!(date("2020-W01").year().is_none());
    }

    #[test]
    fn leap_day_valid_only_in_leap_years() {
        assert_eq!(
            date("2020-02-29").partial_cmp(&date("2020-03-01")),
            Some(Ordering::Less)
        );
        assert_eq!(date("2021-02-29").partial_cmp(&date("2021-03-01")), None); // 2021 not a leap year
    }

    // ── accessors ────────────────────────────────────────────────────────────

    #[test]
    fn accessors_report_components_and_unknowns() {
        let full = date("2020-06-15");
        assert_eq!(full.year(), Some(2020));
        assert_eq!(full.month(), Some(6));
        assert_eq!(full.day(), Some(15));
        assert!(!full.month_unknown());
        assert!(!full.day_unknown());
        assert!(!full.is_partial());

        let year_only = date("2020");
        assert_eq!(year_only.year(), Some(2020));
        assert_eq!(year_only.month(), None);
        assert!(year_only.month_unknown());
        assert!(year_only.day_unknown());
        assert!(year_only.is_partial());

        let year_month = date("2020-06");
        assert_eq!(year_month.month(), Some(6));
        assert!(!year_month.month_unknown());
        assert!(year_month.day_unknown());
        assert!(year_month.is_partial());
    }
}
