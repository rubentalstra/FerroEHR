//! Interval and multiplicity arithmetic shared by the AOM2 validity rules, the
//! flattener, and the cADL domain lowering.
//!
//! Two interval families meet here: `MULTIPLICITY_INTERVAL` (existence,
//! occurrences, cardinality —
//! `docs/specs/openehr/BASE/docs/foundation_types/master05-interval.adoc`
//! plus `docs/specs/openehr/AM/docs/AOM2/master04.5-constraint_model-class_definitions.adoc`
//! §`C_ATTRIBUTE`), read through the [`Bounds`] view; and the generic
//! `INTERVAL<T>` a primitive constraint carries.
//!
//! Two families of near-duplicate readings are deliberately kept side by side
//! rather than merged, because their callers depend on the difference: the
//! point-value extractors (`point_interval_value_i32` vs
//! `degenerate_point_value_i32`) and the bounds renderers (`display_bounds` vs
//! `display_bounds_always_range`). Each pair's doc comment states the
//! divergence.

use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_base::prelude::{Interval, MultiplicityInterval, ProperInterval};

/// A multiplicity bound, extracted from an RM attribute or a cADL constraint.
/// `upper == None` denotes an unbounded (∞) upper limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Inclusive lower bound.
    pub lower: i32,
    /// Inclusive upper bound; `None` = unbounded.
    pub upper: Option<i32>,
}

impl Bounds {
    /// A closed `{lower..upper}` bound.
    #[must_use]
    pub fn new(lower: i32, upper: Option<i32>) -> Self {
        Self { lower, upper }
    }

    /// True if `inner` is the same as, or narrower than (wholly contained
    /// within), `self` — the "conform, i.e. be the same or narrower" test the
    /// existence (VCAEX) and cardinality (VCACA) rules require (`master04.5`
    /// §Validity Rules: `C_ATTRIBUTE`).
    #[must_use]
    pub fn contains(self, inner: Bounds) -> bool {
        inner.lower >= self.lower
            && match (self.upper, inner.upper) {
                // `self` unbounded above ⇒ any inner upper is within it.
                (None, _) => true,
                // `self` bounded but `inner` unbounded ⇒ inner escapes above.
                (Some(_), None) => false,
                (Some(outer), Some(i)) => i <= outer,
            }
    }
}

/// [`Bounds`] view of a [`MultiplicityInterval`] (existence / occurrences /
/// cardinality bound), with `upper == None` denoting an unbounded (`*`) limit.
#[must_use]
pub(crate) fn bounds(mi: &MultiplicityInterval) -> Bounds {
    Bounds {
        lower: if mi.lower_unbounded {
            0
        } else {
            mi.lower.unwrap_or(0)
        },
        upper: if mi.upper_unbounded { None } else { mi.upper },
    }
}

/// The finite upper bound of a multiplicity interval, or `None` if unbounded.
pub(crate) fn finite_upper(mi: &MultiplicityInterval) -> Option<i32> {
    if mi.upper_unbounded { None } else { mi.upper }
}

/// The finite cardinality upper bound of an attribute, `None` if unbounded / no
/// cardinality.
pub(crate) fn finite_cardinality_upper(attr: &CAttribute) -> Option<i32> {
    attr.cardinality
        .as_ref()
        .and_then(|c| finite_upper(&c.interval))
}

/// Render [`Bounds`] as cADL multiplicity text, collapsing a degenerate bound:
/// `{1..1}` prints as `{1}`.
///
/// The sibling renderer [`display_bounds_always_range`] does NOT collapse. Both
/// feed user-visible validation-issue messages, so neither may adopt the
/// other's spelling.
pub(crate) fn display_bounds(b: Bounds) -> String {
    match b.upper {
        Some(u) if u == b.lower => format!("{{{}}}", b.lower),
        Some(u) => format!("{{{}..{u}}}", b.lower),
        None => format!("{{{}..*}}", b.lower),
    }
}

/// Render [`Bounds`] as cADL multiplicity text, always as a range: `{1..1}`
/// prints as `{1..1}`.
///
/// The sibling renderer [`display_bounds`] collapses a degenerate bound to
/// `{1}`. Both feed user-visible validation-issue messages, so neither may
/// adopt the other's spelling.
pub(crate) fn display_bounds_always_range(b: Bounds) -> String {
    match b.upper {
        Some(u) => format!("{{{}..{u}}}", b.lower),
        None => format!("{{{}..*}}", b.lower),
    }
}

/// The value of an integer `POINT_INTERVAL` (`{v}`), else `None`.
///
/// This is the STRUCTURAL reading: only the `POINT_INTERVAL` subtype answers,
/// and its unbounded flags are not consulted. A degenerate closed
/// `PROPER_INTERVAL` (`{n..n}`) yields `None` here but a value from
/// [`degenerate_point_value_i32`].
//
// TODO: unify the divergent interval point-of semantics — tracked as issue #1339.
pub(crate) fn point_interval_value_i32(iv: &Interval<i32>) -> Option<i32> {
    match iv {
        Interval::PointInterval(p) => p.lower,
        Interval::ProperInterval(_) => None,
    }
}

/// The value of a real `POINT_INTERVAL` (`{v}`), else `None`.
///
/// The real counterpart of [`point_interval_value_i32`] — the same structural
/// reading (a `PROPER_INTERVAL` never answers).
pub(crate) fn point_interval_value_f64(iv: &Interval<f64>) -> Option<f64> {
    match iv {
        Interval::PointInterval(p) => p.lower,
        Interval::ProperInterval(_) => None,
    }
}

/// The single point value an integer interval denotes (`{n}` **or** the closed
/// degenerate `{n..n}`), or `None` for a range or unbounded interval.
///
/// This is the SEMANTIC reading, and it diverges from
/// [`point_interval_value_i32`] twice: it also answers for a closed
/// `PROPER_INTERVAL` whose bounds coincide, and it refuses a `POINT_INTERVAL`
/// carrying an unbounded flag.
//
// TODO: unify the divergent interval point-of semantics — tracked as issue #1339.
pub(crate) fn degenerate_point_value_i32(iv: &Interval<i32>) -> Option<i32> {
    match iv {
        Interval::PointInterval(p) if !p.lower_unbounded && !p.upper_unbounded => p.lower,
        Interval::ProperInterval(ProperInterval::ProperInterval(d))
            if d.lower_included
                && d.upper_included
                && !d.lower_unbounded
                && !d.upper_unbounded
                && d.lower == d.upper =>
        {
            d.lower
        }
        _ => None,
    }
}

/// The `(lower, upper)` bounds of an interval as `f64`, each `None` when open or
/// unbounded, plus the two inclusivity flags. A `MultiplicityInterval` variant
/// (structurally possible on the generic enum but never produced for a domain
/// leaf constraint) yields fully-open bounds, so membership is undecided and the
/// conservative `true` answer stands.
pub(crate) fn interval_bounds_f64<T: Copy + Into<f64>>(
    iv: &Interval<T>,
) -> (Option<f64>, Option<f64>, bool, bool) {
    let (lower, upper, lower_unbounded, upper_unbounded, lower_included, upper_included) = match iv
    {
        Interval::PointInterval(p) => (
            p.lower,
            p.upper,
            p.lower_unbounded,
            p.upper_unbounded,
            p.lower_included,
            p.upper_included,
        ),
        Interval::ProperInterval(ProperInterval::ProperInterval(p)) => (
            p.lower,
            p.upper,
            p.lower_unbounded,
            p.upper_unbounded,
            p.lower_included,
            p.upper_included,
        ),
        Interval::ProperInterval(ProperInterval::MultiplicityInterval(_)) => {
            return (None, None, true, true);
        }
    };
    (
        if lower_unbounded {
            None
        } else {
            lower.map(Into::into)
        },
        if upper_unbounded {
            None
        } else {
            upper.map(Into::into)
        },
        lower_included,
        upper_included,
    )
}

/// True if the real interval `iv` contains `v` (honouring open/closed bounds).
pub(crate) fn real_interval_contains(iv: &Interval<f64>, v: f64) -> bool {
    bounds_admit(v, interval_bounds_f64(iv))
}

/// True if the integer interval `iv` contains `v` (honouring open/closed bounds).
pub(crate) fn int_interval_contains(iv: &Interval<i32>, v: i32) -> bool {
    bounds_admit(f64::from(v), interval_bounds_f64(iv))
}

/// Interval membership over `f64` bounds, shared by the real/integer tests.
pub(crate) fn bounds_admit(v: f64, bounds: (Option<f64>, Option<f64>, bool, bool)) -> bool {
    let (lower, upper, lower_included, upper_included) = bounds;
    if let Some(lo) = lower
        && (v < lo || (!lower_included && (v - lo).abs() < f64::EPSILON))
    {
        return false;
    }
    if let Some(hi) = upper
        && (v > hi || (!upper_included && (v - hi).abs() < f64::EPSILON))
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_containment_is_same_or_narrower() {
        let one_to_one = Bounds::new(1, Some(1));
        assert!(one_to_one.contains(Bounds::new(1, Some(1))));
        assert!(!one_to_one.contains(Bounds::new(0, Some(0)))); // {0} not within {1..1}
        let star = Bounds::new(0, None);
        assert!(star.contains(Bounds::new(1, Some(5))));
        assert!(!Bounds::new(1, Some(5)).contains(star)); // {0..*} escapes {1..5}
    }

    /// The two renderers differ ONLY on a degenerate bound, and both spellings
    /// are load-bearing message text.
    #[test]
    fn the_two_bounds_renderers_differ_on_a_degenerate_bound() {
        let point = Bounds::new(1, Some(1));
        assert_eq!(display_bounds(point), "{1}");
        assert_eq!(display_bounds_always_range(point), "{1..1}");
        let range = Bounds::new(0, Some(3));
        assert_eq!(display_bounds(range), "{0..3}");
        assert_eq!(display_bounds_always_range(range), "{0..3}");
        let open = Bounds::new(2, None);
        assert_eq!(display_bounds(open), "{2..*}");
        assert_eq!(display_bounds_always_range(open), "{2..*}");
    }

    /// The two point-value extractors disagree on a degenerate closed
    /// `PROPER_INTERVAL` — pinned so the divergence cannot be "fixed" silently
    /// ahead of its adjudication (issue #1339).
    #[test]
    fn the_two_point_value_extractors_disagree_on_a_degenerate_proper_interval() {
        let degenerate: Interval<i32> = Interval::ProperInterval(ProperInterval::ProperInterval(
            openehr_base::prelude::ProperIntervalData {
                lower: Some(4),
                upper: Some(4),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        ));
        assert_eq!(point_interval_value_i32(&degenerate), None);
        assert_eq!(degenerate_point_value_i32(&degenerate), Some(4));
    }
}
