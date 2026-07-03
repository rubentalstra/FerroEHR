//! `DV_INTERVAL<T>` — generic class defining an interval (range) of a
//! comparable type.
//!
//! openEHR class: `DV_INTERVAL<T>`, package `rm.data_types.quantity`.
//! Inherits: `DATA_VALUE`, `Interval<T>` (BASE foundation_types).
//!
//! Generic class defining an interval (i.e. range) of a comparable type. An
//! interval is a contiguous subrange of a comparable base type. Used to
//! define intervals of dates, times, quantities (whose units match) and so
//! on. The type parameter, `T`, must be a descendant of the type
//! `DV_ORDERED`, which is necessary (but not sufficient) for instances to be
//! compared (`strictly_comparable` is also needed).
//!
//! Without the `DV_INTERVAL` class, quite a few more `DV_` classes would be
//! needed to express logical intervals, namely interval versions of all the
//! date/time classes, and of quantity classes. Further, it allows the
//! semantics of intervals to be stated in one place unequivocally, including
//! the conditions for strict comparison.
//!
//! The basic semantics are derived from the class `Interval<T>`, described
//! in the support RM.
use super::dv_ordered::DvOrderedApi;
// TODO(port): forward-references DATA_VALUE (rm.data_types.basic), not yet
// transcribed by the sibling package agent covering `data_types::basic`.
use crate::data_types::data_value::DataValue;
use openehr_foundation::interval::interval::Interval;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "DV_INTERVAL";

/// `DV_INTERVAL<T>` inherits both `DATA_VALUE` and `Interval<T>` (BASE
/// foundation_types) per its `Inherit` row — a multiple-inheritance case
/// (ADR-001 §2/§3: composition of fields from all parents). The
/// `Interval<T>` parent is embedded by value as `range` rather than
/// flattened, matching the same composition shape already used by
/// `PointInterval<T>`/`ProperInterval<T>`
/// (`openehr_foundation::interval::{point_interval,proper_interval}`), so
/// `DV_INTERVAL<T>` stays structurally consistent with its own foundation
/// ancestor rather than duplicating the six `Interval<T>` attributes flat.
///
/// The class's own attribute table in the spec lists no additional
/// attributes of its own beyond what it inherits — its only genuinely new
/// content is the `Limits_consistent` class invariant (see below), which
/// tightens `Interval<T>`'s own (weaker, TODO-pending) invariants by adding
/// the `is_strictly_comparable_to` requirement specific to `DV_ORDERED`
/// limits.
///
/// `T: DvOrderedApi` matches the spec's `T` constraint ("must be a
/// descendant of the type `DV_ORDERED`"), per ADR-001 §5 (constrained
/// generic → generic with trait bound). `DvOrderedApi: Ordered` (see
/// `dv_ordered.rs`), so `T: DvOrderedApi` also satisfies `Interval<T>`'s own
/// `T: Ordered` bound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvInterval<T: DvOrderedApi> {
    /// Canonical `_type` discriminator (`"DV_INTERVAL"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    ///
    /// The function-path `default = "TypeTag::new"` form is mandatory on a
    /// generic container — bare `default` makes serde's derive add a
    /// spurious `T: Default` bound.
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<DvInterval<T>>,

    /// Embedded parent state from `Interval<T>` (BASE foundation_types); see
    /// the struct-level PORT NOTE for why this is composition rather than a
    /// flattened field list. Carries `lower`, `upper`, `lower_unbounded`,
    /// `upper_unbounded`, `lower_included`, `upper_included`.
    ///
    /// `#[serde(flatten)]` per this crate's established embedded-parent
    /// convention (`DvCodedText.text`, `DvUri.uri`), so the wire shape is
    /// `{"_type":"DV_INTERVAL","lower":…,"upper":…,"lower_included":…,…}`
    /// with the six `Interval<T>` fields at top level, not nested under an
    /// `"interval"`/`"range"` key.
    ///
    /// PORT NOTE: the previously-flagged cross-crate blocker is closed —
    /// `openehr_foundation::interval::interval::Interval<T>` now derives
    /// `Serialize`/`Deserialize` with `skip_serializing_if` on the optional
    /// `lower`/`upper` limits, so nothing blocks this flatten.
    #[serde(flatten)]
    pub range: Interval<T>,
}

/// ADR-002: `_type` string for `DV_INTERVAL`, single-sourced from
/// [`TYPE_NAME`]. The impl repeats the struct's own declared
/// `T: DvOrderedApi` bound (required for the type `DvInterval<T>` to be
/// well-formed) but deliberately adds **no** further bounds — in particular
/// no `T: Serialize`/`T: Default` — so the tag never constrains the generic
/// parameter beyond the struct itself.
impl<T: DvOrderedApi> TypeName for DvInterval<T> {
    const NAME: &'static str = TYPE_NAME;
}

impl<T: DvOrderedApi> DvInterval<T> {
    // PORT NOTE: `DV_INTERVAL` declares no functions of its own in the
    // per-class table — `has`/`intersects`/`contains` are inherited from
    // `Interval<T>` (BASE foundation_types), whose bodies are now fully
    // implemented at the foundation layer (the `Interval::has` postcondition
    // ambiguity was resolved from the spec's Meaning column). The three
    // methods below are thin inherent forwarders onto the embedded `range`,
    // so callers (and the `DvOrderedApi` default bodies in `dv_ordered.rs`)
    // can write `dv_interval.has(v)` directly rather than reaching through
    // `dv_interval.range.has(v)`.

    /// `has(e: T) -> Boolean` — inherited from `Interval<T>`.
    ///
    /// True if the value `e` is properly contained in this interval; the
    /// open/closed/unbounded membership semantics live in the foundation
    /// `Interval::has`.
    pub fn has(&self, e: &T) -> bool {
        self.range.has(e)
    }

    /// `intersects(other: Interval) -> Boolean` — inherited from `Interval<T>`.
    pub fn intersects(&self, other: &DvInterval<T>) -> bool {
        self.range.intersects(&other.range)
    }

    /// `contains(other: Interval) -> Boolean` — inherited from `Interval<T>`.
    pub fn contains(&self, other: &DvInterval<T>) -> bool {
        self.range.contains(&other.range)
    }

    /// `Limits_consistent` class invariant, as a working method per ADR-003
    /// decision 8 (invariants become `is_valid()`-family methods):
    ///
    /// `(not upper_unbounded and not lower_unbounded) implies
    /// (lower.is_strictly_comparable_to(upper) and lower <= upper)`.
    ///
    /// This tightens (rather than duplicates) `Interval<T>`'s own
    /// `Limits_consistent` (`lower <= upper` only): at the
    /// `DV_INTERVAL<T: DV_ORDERED>` level the comparability check is
    /// `DvOrderedApi::is_strictly_comparable_to` specifically, which the
    /// foundation `Interval` cannot express (its `T: Ordered` bound carries
    /// no strict-comparability notion).
    ///
    /// PORT NOTE: when a bounded side's limit value is absent (the
    /// inconsistent state the foundation `Interval` documents on
    /// `effective_lower`), the implication holds vacuously — there is no pair
    /// of limits to compare — matching the foundation's own
    /// `limits_consistent` treatment.
    pub fn invariant_limits_consistent(&self) -> bool {
        if self.range.upper_unbounded || self.range.lower_unbounded {
            return true;
        }
        match (&self.range.lower, &self.range.upper) {
            (Some(lower), Some(upper)) => {
                lower.is_strictly_comparable_to(upper) && lower.less_than_or_equal(upper)
            }
            _ => true,
        }
    }
}

// PORT NOTE: `DATA_VALUE` (the other half of `DV_INTERVAL`'s `Inherit` row)
// is not yet embedded here — it is owned by the sibling `data_types::basic`
// package (not yet transcribed in this worktree). Expected to be composed
// alongside `range: Interval<T>` once that package lands, per the same
// multi-parent pattern noted in `dv_ordered.rs`.
#[allow(unused_imports)]
use DataValue as _DataValueForwardRef;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_amount::DvAmountData;
    use crate::data_types::quantity::dv_count::DvCount;
    use crate::data_types::quantity::dv_ordered::DvOrderedData;
    use crate::data_types::quantity::dv_quantified::DvQuantifiedData;
    use crate::data_types::quantity::dv_quantity::DvQuantity;
    use openehr_foundation::primitive_types::real::Real;

    fn count(m: i64) -> DvCount {
        DvCount {
            type_tag: TypeTag::new(),
            amount: DvAmountData {
                quantified: DvQuantifiedData {
                    ordered: DvOrderedData {
                        normal_status: None,
                        normal_range: None,
                        other_reference_ranges: None,
                    },
                    magnitude_status: None,
                    accuracy: None,
                },
                accuracy_is_percent: None,
                accuracy: None,
            },
            magnitude: m,
        }
    }

    fn count_interval(lower: i64, upper: i64) -> DvInterval<DvCount> {
        DvInterval {
            type_tag: TypeTag::new(),
            range: Interval {
                lower: Some(count(lower)),
                upper: Some(count(upper)),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        }
    }

    fn quantity(m: f64, units: &str) -> DvQuantity {
        DvQuantity {
            type_tag: TypeTag::new(),
            amount: DvAmountData {
                quantified: DvQuantifiedData {
                    ordered: DvOrderedData {
                        normal_status: None,
                        normal_range: None,
                        other_reference_ranges: None,
                    },
                    magnitude_status: None,
                    accuracy: None,
                },
                accuracy_is_percent: None,
                accuracy: None,
            },
            magnitude: Real(m),
            precision: None,
            units: units.to_string(),
            units_system: None,
            units_display_name: None,
        }
    }

    /// `has` forwards to the foundation `Interval::has` membership semantics
    /// (closed limits here, so both endpoints are included).
    #[test]
    fn has_delegates_to_the_embedded_interval() {
        let iv = count_interval(0, 10);
        assert!(iv.has(&count(0)));
        assert!(iv.has(&count(5)));
        assert!(iv.has(&count(10)));
        assert!(!iv.has(&count(11)));
        assert!(!iv.has(&count(-1)));
    }

    /// `intersects`/`contains` forward to the foundation `Interval`.
    #[test]
    fn intersects_and_contains_delegate_to_the_embedded_interval() {
        assert!(count_interval(0, 10).intersects(&count_interval(5, 15)));
        assert!(!count_interval(0, 10).intersects(&count_interval(11, 15)));
        assert!(count_interval(0, 10).contains(&count_interval(2, 8)));
        assert!(!count_interval(0, 10).contains(&count_interval(2, 12)));
    }

    /// `Limits_consistent`: bounded limits must be ordered `lower <= upper`.
    /// (`DvCount` is always strictly comparable, so only ordering matters.)
    #[test]
    fn limits_consistent_requires_ordered_limits() {
        assert!(count_interval(0, 10).invariant_limits_consistent());
        assert!(count_interval(5, 5).invariant_limits_consistent());
        let bad = DvInterval {
            type_tag: TypeTag::new(),
            range: Interval {
                lower: Some(count(10)),
                upper: Some(count(0)),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        };
        assert!(!bad.invariant_limits_consistent());
    }

    /// `Limits_consistent` additionally requires the limits to be *strictly
    /// comparable* at the `DV_ORDERED` level — two `DV_QUANTITY` limits with
    /// mismatched units are not comparable, so the invariant fails even
    /// though their magnitudes are ordered (this is what `DV_INTERVAL` adds
    /// over the foundation `Interval`).
    #[test]
    fn limits_consistent_requires_strict_comparability_of_limits() {
        let mismatched_units = DvInterval {
            type_tag: TypeTag::new(),
            range: Interval {
                lower: Some(quantity(0.0, "kg")),
                upper: Some(quantity(10.0, "mmHg")),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        };
        assert!(!mismatched_units.invariant_limits_consistent());
        let same_units = DvInterval {
            type_tag: TypeTag::new(),
            range: Interval {
                lower: Some(quantity(0.0, "kg")),
                upper: Some(quantity(10.0, "kg")),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        };
        assert!(same_units.invariant_limits_consistent());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_interval.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_interval.adoc §DV_INTERVAL Class
//   confidence: high
//   todos: 1
//   note: has/intersects/contains are now inherent forwarders onto the embedded Interval<T> (foundation bodies fully implemented — the prior "todo!() at the foundation layer" note was stale); Limits_consistent implemented as invariant_limits_consistent() per ADR-003 §8, tightening the foundation invariant with DvOrderedApi::is_strictly_comparable_to (unit-tested with both DvCount ordering and DvQuantity unit-mismatch). Remaining TODO: DATA_VALUE parent not yet embedded pending sibling data_types::basic package landing (P17). P4/ADR-002: self-tags via TypeTag<DvInterval<T>> first field (function-path default, no extra bounds on T in the TypeName impl beyond the struct's own DvOrderedApi); `range` carries #[serde(flatten)], schema-verified (six Interval fields sit flat beside _type).
// ─────────────────────────────────────────────
