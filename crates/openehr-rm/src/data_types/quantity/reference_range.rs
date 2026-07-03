//! `REFERENCE_RANGE<T>` — a named range associated with any `DV_ORDERED`
//! datum.
//!
//! openEHR class: `REFERENCE_RANGE<T>`, package `rm.data_types.quantity`.
//! Inherits: none listed (spec table has no `Inherit` row).
//!
//! Defines a named range to be associated with any `DV_ORDERED` datum. Each
//! such range is particular to the patient and context, e.g. sex, age, and
//! any other factor which affects ranges. May be used to represent normal,
//! therapeutic, dangerous, critical etc ranges.
use super::dv_interval::DvInterval;
use super::dv_ordered::DvOrderedApi;
// TODO(port): forward-references DV_TEXT (rm.data_types.text), not yet
// transcribed by the sibling package agent covering `data_types::text`.
use crate::data_types::text::dv_text::DvText;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "REFERENCE_RANGE";

/// `REFERENCE_RANGE<T>` has no `Inherit` row in its per-class table (unlike
/// `DV_INTERVAL<T>`, which inherits `DATA_VALUE` + `Interval<T>`); it is a
/// standalone generic struct carrying its two declared attributes directly.
///
/// `T: DvOrderedApi` matches the class description's "associated with any
/// `DV_ORDERED` datum" and the generic parameter written on `DV_INTERVAL<T>`
/// in `range`'s declared type — per ADR-001 §5 (constrained generic →
/// generic with trait bound), mirrored identically from `dv_interval.rs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceRange<T: DvOrderedApi> {
    /// Canonical `_type` discriminator (`"REFERENCE_RANGE"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    ///
    /// The function-path `default = "TypeTag::new"` form is mandatory on a
    /// generic container — bare `default` makes serde's derive add a
    /// spurious `T: Default` bound.
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<ReferenceRange<T>>,

    /// `meaning`: `DV_TEXT` (1..1).
    ///
    /// Term whose value indicates the meaning of this range, e.g. normal,
    /// critical, therapeutic etc.
    ///
    /// Genuinely nested on the wire (no flatten) — `meaning` and `range` are
    /// this class's own declared attributes, so they already sit at the top
    /// level of the `REFERENCE_RANGE` object beside `_type`, schema-verified.
    pub meaning: DvText,

    /// `range`: `DV_INTERVAL<T>` (1..1).
    ///
    /// The data range for this meaning, e.g. critical etc.
    pub range: DvInterval<T>,
}

/// ADR-002: `_type` string for `REFERENCE_RANGE`, single-sourced from
/// [`TYPE_NAME`]. Repeats the struct's own declared `T: DvOrderedApi` bound
/// (required for well-formedness) but adds **no** further bounds on `T`.
impl<T: DvOrderedApi> TypeName for ReferenceRange<T> {
    const NAME: &'static str = TYPE_NAME;
}

impl<T: DvOrderedApi> ReferenceRange<T> {
    /// `is_in_range(v: DV_ORDERED) -> Boolean`.
    ///
    /// Indicates if the value `v` is inside the range.
    ///
    /// PORT NOTE: the spec types `v` as the abstract `DV_ORDERED`, but a
    /// meaningful "in range" test can only compare `v` against this
    /// `ReferenceRange<T>`'s own `range: DV_INTERVAL<T>`, which is typed
    /// over the same concrete `T` — narrowed to `&T` here, per the same
    /// "concrete-type narrowing of an `Any`/ancestor-typed parameter"
    /// pattern used throughout `openehr-foundation` (see
    /// `Interval::is_equal`'s PORT NOTE).
    pub fn is_in_range(&self, v: &T) -> bool {
        // Delegates to the inherent `DvInterval::has`, which forwards to the
        // foundation `Interval::has` (membership semantics now fully
        // implemented at the foundation layer).
        self.range.has(v)
    }

    /// `Range_is_simple` class invariant, as a working method per ADR-003
    /// decision 8 (invariants become `is_valid()`-family methods):
    ///
    /// `(range.lower_unbounded or else range.lower.is_simple) and
    /// (range.upper_unbounded or else range.upper.is_simple)`.
    ///
    /// Each bounded limit value (`T: DvOrderedApi`) must itself be *simple*
    /// (carry no reference ranges of its own) — a reference range whose
    /// limits themselves carried reference ranges would be ill-formed.
    /// `is_simple` here is `DvOrderedApi::is_simple`, now a working default
    /// body over the concrete limit type.
    ///
    /// PORT NOTE: `or else` is Eiffel's short-circuit disjunction, mapped to
    /// Rust's already-short-circuiting `||`. A bounded side whose limit value
    /// is nonetheless absent (the inconsistent state the foundation
    /// `Interval` documents on `effective_lower`) is treated as vacuously
    /// simple, matching the foundation's own handling of that case.
    pub fn invariant_range_is_simple(&self) -> bool {
        let interval = &self.range.range;
        (interval.lower_unbounded
            || interval
                .lower
                .as_ref()
                .is_none_or(super::dv_ordered::DvOrderedApi::is_simple))
            && (interval.upper_unbounded
                || interval
                    .upper
                    .as_ref()
                    .is_none_or(super::dv_ordered::DvOrderedApi::is_simple))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_amount::DvAmountData;
    use crate::data_types::quantity::dv_count::DvCount;
    use crate::data_types::quantity::dv_ordered::DvOrderedData;
    use crate::data_types::quantity::dv_quantified::DvQuantifiedData;
    use crate::data_types::text::dv_text::DvTextData;
    use openehr_foundation::interval::interval::Interval;

    fn count(magnitude: i64) -> DvCount {
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
            magnitude,
        }
    }

    fn count_interval(lower: DvCount, upper: DvCount) -> DvInterval<DvCount> {
        DvInterval {
            type_tag: TypeTag::new(),
            range: Interval {
                lower: Some(lower),
                upper: Some(upper),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        }
    }

    fn reference_range(range: DvInterval<DvCount>) -> ReferenceRange<DvCount> {
        ReferenceRange {
            type_tag: TypeTag::new(),
            meaning: DvText::Text {
                type_tag: TypeTag::new(),
                data: DvTextData {
                    value: "normal".to_string(),
                    hyperlink: None,
                    formatting: None,
                    mappings: None,
                    language: None,
                    encoding: None,
                },
            },
            range,
        }
    }

    /// Spec `is_in_range`: "Indicates if the value v is inside the range."
    #[test]
    fn is_in_range_delegates_to_the_interval() {
        let rr = reference_range(count_interval(count(0), count(10)));
        assert!(rr.is_in_range(&count(0)));
        assert!(rr.is_in_range(&count(5)));
        assert!(rr.is_in_range(&count(10)));
        assert!(!rr.is_in_range(&count(11)));
    }

    /// Spec `Range_is_simple`: each bounded limit must itself be simple (carry
    /// no reference ranges of its own).
    #[test]
    fn range_is_simple_requires_simple_limits() {
        let simple = reference_range(count_interval(count(0), count(10)));
        assert!(simple.invariant_range_is_simple());

        // A limit that itself carries a normal_range is not simple.
        let mut non_simple_upper = count(10);
        non_simple_upper.amount.quantified.ordered.normal_range =
            Some(Box::new(count_interval(count(0), count(10))));
        let bad = reference_range(count_interval(count(0), non_simple_upper));
        assert!(!bad.invariant_range_is_simple());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/reference_range.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / reference_range.adoc §REFERENCE_RANGE Class
//   confidence: high
//   todos: 1
//   note: is_in_range narrows the spec's abstract DV_ORDERED parameter to the concrete T (documented PORT NOTE) and now delegates cleanly to the inherent DvInterval::has (foundation Interval::has fully implemented — the prior stale todo removed); Range_is_simple implemented as invariant_range_is_simple() per ADR-003 §8 over the limits' DvOrderedApi::is_simple, unit-tested. Remaining TODO: forward-reference DV_TEXT pending the sibling data_types::text package (present in-tree; reconciled at P17). P4/ADR-002: self-tags via TypeTag<ReferenceRange<T>> first field (function-path default; TypeName impl adds no bounds on T beyond the struct's own DvOrderedApi); no flatten needed (schema-verified — meaning/range are this class's own attributes, already top-level beside _type).
// ─────────────────────────────────────────────
