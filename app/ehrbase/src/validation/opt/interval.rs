//! Interval / multiplicity primitives for the OPT 1.4 artefact pass (T20).
//!
//! The AOM occurrence / existence / cardinality invariants and the primitive
//! `Assumed_value_valid` membership tests all rest on the BASE interval
//! algebra: "a value is contained unless it falls below the lower limit or
//! above the upper limit, an unbounded/absent limit imposing no constraint on
//! that side" (`BASE/docs/foundation_types/master05-interval.adoc`;
//! `BASE/docs/UML/classes/org.openehr.base.foundation_types.interval.adoc`,
//! `has`). Integer membership is delegated to the BASE
//! [`MultiplicityInterval`] primitive (`Multiplicity_interval` is an
//! `Interval<Integer>`, the very interval the occurrence/cardinality validator
//! interrogates) so the boundary algebra is written once, in the spec crate.
//!
//! The `opt14` XSD models an interval as six independent components
//! (`lower`/`upper` optional, `lower_unbounded`/`upper_unbounded` flags,
//! `lower_included`/`upper_included` optional flags). The two bound accessors
//! [`iv_lower`]/[`iv_upper`] read those components directly (they surface the
//! numeric bounds the AOM invariants compare, not a membership decision).

use openehr_base::prelude::MultiplicityInterval;
use openehr_its::opt14::{Intervalofinteger, Intervalofreal};

/// The effective lower bound of an `opt14` integer interval: an unbounded or
/// absent lower limit reads as `0` (the AOM occurrence/existence floor).
fn iv_lower(iv: &Intervalofinteger) -> i32 {
    if iv.lower_unbounded {
        0
    } else {
        iv.lower.unwrap_or(0)
    }
}

/// The effective upper bound of an `opt14` integer interval: `None` for an
/// unbounded upper limit (`{lower..*}`), else the declared `upper`.
fn iv_upper(iv: &Intervalofinteger) -> Option<i32> {
    if iv.upper_unbounded { None } else { iv.upper }
}

/// Integer membership `v ∈ interval`, delegated to the BASE
/// `Multiplicity_interval.has` primitive (`Interval.has`,
/// `BASE/docs/UML/classes/org.openehr.base.foundation_types.interval.adoc`).
///
/// The `opt14` interval's six components map cleanly onto the BASE type; the
/// AOM leaf constraints (`C_INTEGER` lists/ranges, assumed-value checks) are
/// always closed inclusive intervals, so the inclusion flags are set to
/// `true` — an absent or unbounded limit still imposes no constraint on that
/// side, matching `Interval.has`.
fn int_in_range(v: i32, r: &Intervalofinteger) -> bool {
    MultiplicityInterval {
        lower: r.lower,
        upper: r.upper,
        lower_unbounded: r.lower_unbounded,
        upper_unbounded: r.upper_unbounded,
        lower_included: true,
        upper_included: true,
    }
    .has(v)
}

/// Real membership `v ∈ interval`, inlining the same `Interval.has` boundary
/// algebra for `Real` bounds.
///
/// PORT NOTE: the BASE constraint-evaluation primitive built at
/// `crates/openehr-base/src/foundation_types/interval/` exposes
/// `Multiplicity_interval` (an `Interval<Integer>`) for integer membership,
/// which [`int_in_range`] consumes. There is no ergonomic BASE entry point for
/// `Real` membership — `Interval<Real>` cannot satisfy the `Ord`-style bound
/// the generated boundary view is written against (`f64` is only `PartialOrd`),
/// and the generic `Interval<T>` boundary view is crate-private to
/// `openehr-base`. The boundary algebra is therefore inlined here for `Real`,
/// faithful to `Interval.has` (`master05-interval.adoc`): an absent/unbounded
/// limit imposes no constraint; present limits compare inclusively.
fn real_in_range(v: f64, r: &Intervalofreal) -> bool {
    let lower_ok = r.lower_unbounded || r.lower.is_none_or(|l| v >= l);
    let upper_ok = r.upper_unbounded || r.upper.is_none_or(|u| v <= u);
    lower_ok && upper_ok
}
