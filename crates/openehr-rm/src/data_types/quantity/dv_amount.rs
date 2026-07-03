//! `DV_AMOUNT` — abstract class defining the concept of relative quantified
//! 'amounts'.
//!
//! openEHR class: `DV_AMOUNT` (abstract), package `rm.data_types.quantity`.
//! Inherits: `DV_QUANTIFIED`.
//!
//! Abstract class defining the concept of relative quantified 'amounts'.
//! For relative quantities, the `+` and `-` operators are defined (unlike
//! descendants of `DV_ABSOLUTE_QUANTITY`, such as the date/time types).
use super::dv_count::DvCount;
use super::dv_ordered::DvOrderedApi;
use super::dv_proportion::DvProportion;
use super::dv_quantified::{DvQuantifiedApi, DvQuantifiedData};
use super::dv_quantity::DvQuantity;
// TODO(port): forward-references DV_DURATION (rm.data_types.date_time),
// not yet transcribed by the sibling package agent covering
// `data_types::date_time`. `DV_DURATION` is also a `DV_AMOUNT` descendant
// per PORT_MASTER_PLAN.md §7.1's RM class inventory ("Subtypes of DV_AMOUNT
// are DV_PROPORTION, DV_QUANTITY, DV_COUNT, and DV_DURATION"), so it must
// be a variant of the closed `DvAmount` enum below for that enum to be
// genuinely exhaustive over every `DV_AMOUNT` descendant in the RM, not
// just this package's three.
use crate::data_types::date_time::dv_duration::DvDuration;
use openehr_foundation::primitive_types::any::Any;
// PORT NOTE: `Ordered` is *not* imported here — the `less_than_amount`
// default body calls `magnitude().less_than(..)`, which resolves through the
// `T: OrderedNumeric` bound (`OrderedNumeric: Ordered`, already in scope)
// without needing `Ordered` itself imported.
use openehr_foundation::primitive_types::ordered_numeric::OrderedNumeric;
use openehr_foundation::primitive_types::real::Real;
use serde::{Deserialize, Serialize};

/// Shared attribute state of `DV_AMOUNT` and its descendants.
///
/// Per ADR-001 §3, embedded by every concrete `DV_AMOUNT` subtype
/// (`DV_QUANTITY`, `DV_COUNT`, `DV_PROPORTION`) rather than duplicated flat
/// at each level.
///
/// `T: DvOrderedApi` threads the same F-bounded self-type as
/// `DvQuantifiedData<T>`/`DvOrderedData<T>` (see `dv_ordered.rs`,
/// `dv_quantified.rs`), since `DV_AMOUNT`'s `Inherit` row is `DV_QUANTIFIED`
/// and the self-referential range attributes flow through unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvAmountData<T: DvOrderedApi> {
    /// Embedded `DV_QUANTIFIED` parent state.
    #[serde(flatten)]
    pub quantified: DvQuantifiedData<T>,

    /// `accuracy_is_percent`: `Boolean` (0..1).
    ///
    /// If `True`, indicates that when this object was created, `accuracy`
    /// was recorded as a percent value; if `False`, as an absolute
    /// quantity value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy_is_percent: Option<bool>,

    /// `accuracy`: `Real` (0..1, redefined).
    ///
    /// Accuracy of measurement, expressed either as a half-range percent
    /// value (`accuracy_is_percent` = `True`) or a half-range quantity. A
    /// value of `0` means that accuracy is 100%, i.e. no error.
    ///
    /// A value of `unknown_accuracy_value` means that accuracy was not
    /// recorded.
    ///
    /// **Covariant redefinition** (ADR-001 §6): the spec's attribute table
    /// marks this `0..1 (redefined)`, narrowing the abstract
    /// `DV_QUANTIFIED.accuracy: Any` down to `Real` specifically. This
    /// replaces (does not sit alongside) `DvQuantifiedData::accuracy` — see
    /// that field's own PORT NOTE for why the unredefined form is expected
    /// to stay unused; `DvAmountData::accuracy` is the field every concrete
    /// `DV_AMOUNT` descendant actually reads/writes.
    ///
    /// PORT NOTE: the previously-flagged cross-crate gap (`Real` lacking
    /// `Serialize`/`Deserialize` in `openehr-foundation`) is closed —
    /// `openehr-foundation` now carries `serde` and `Real` derives both,
    /// serializing as its bare inner `f64` (newtype transparency).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<Real>,
}

/// `unknown_accuracy_value` constant.
///
/// Per the class description's own text: "In `DV_AMOUNT`, a value of `-1`
/// for the accuracy attribute is used for this purpose, and the constant
/// `unknown_accuracy_value = -1` is provided within the class to give a
/// symbolic name for the special value." Not itself listed as a row in the
/// published Attributes/Functions/Constants table for `DV_AMOUNT` (the
/// table has no separate "Constants" section for this class), but stated
/// unambiguously in the class description prose and cross-referenced from
/// the quantity-package overview.
pub const UNKNOWN_ACCURACY_VALUE: f64 = -1.0;

/// `DV_AMOUNT` is abstract and used polymorphically wherever an attribute is
/// declared of that type — most notably `DV_ABSOLUTE_QUANTITY.accuracy`
/// (this package, covariant redefinition of `DV_QUANTIFIED.accuracy: Any`;
/// see `dv_absolute_quantity.rs`). Per ADR-001 §4 (closed subtype set →
/// enum), every concrete `DV_AMOUNT` descendant across both this package
/// and the sibling `date_time` package is collected into this closed enum,
/// mirroring the same pattern already used for `DvOrdered` in
/// `dv_ordered.rs`.
///
/// Variant ownership: `Quantity`, `Count`, and `Proportion` are transcribed
/// in this package; `Duration` is owned by the sibling `date_time` package
/// (transcribed concurrently in a separate worktree) and referenced here
/// purely as a forward `use` path, per the RM class inventory's own
/// statement that `DV_DURATION` is a `DV_AMOUNT` subtype
/// (PORT_MASTER_PLAN.md §7.1).
///
/// PORT NOTE: `#[serde(untagged)]` per ADR-002, matching `DvOrdered`'s
/// conversion in `dv_ordered.rs` — abstract-set enums carry no tag of their
/// own; the `_type` discriminator is emitted (and dispatched on input) by
/// each concrete variant payload's own self-tagging `TypeTag<Self>` first
/// field, which rejects a mismatched `_type` string so untagged probing is
/// tag-driven. The former `#[serde(tag = "_type")]` + per-variant renames
/// would duplicate the payload's own tag. `Duration` self-tags in the
/// sibling `date_time` package's own ADR-002 pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DvAmount {
    /// `DV_QUANTITY`.
    Quantity(DvQuantity),
    /// `DV_COUNT`.
    Count(DvCount),
    /// `DV_PROPORTION`.
    Proportion(DvProportion),
    /// `DV_DURATION` (sibling `date_time` package).
    Duration(DvDuration),
}

/// Behaviour trait shared by every `DV_AMOUNT` descendant.
///
/// Extends [`DvQuantifiedApi`] (`DV_AMOUNT` inherits `DV_QUANTIFIED`) with
/// the amount-specific arithmetic members.
///
/// Symbolic operators (`+`, `-` binary, `-` unary, `*`) are named methods
/// per the RM transcription rules, not `std::ops` overloads.
pub trait DvAmountApi<T: OrderedNumeric>: DvQuantifiedApi<T> {
    /// `accuracy_is_percent`: optional percent-vs-absolute indicator.
    fn accuracy_is_percent(&self) -> Option<bool>;

    /// `accuracy`: optional accuracy value (redefined to `Real` at this
    /// level).
    fn accuracy(&self) -> Option<Real>;

    /// `valid_percentage(number: Ordered_Numeric) -> Boolean`.
    ///
    /// Test whether a number is a valid percentage, i.e. between 0 and
    /// 100.
    ///
    /// PORT NOTE: the spec types `number` as the abstract `Ordered_Numeric`
    /// itself, but the only place the spec ever applies this query is the
    /// `Accuracy_validity` invariant (`accuracy_is_percent implies
    /// valid_percentage (accuracy)`), whose operand is `accuracy: Real` —
    /// the parameter is therefore narrowed to `&Real` here, the one
    /// concrete `Ordered_Numeric` the spec actually feeds it. The earlier
    /// fully-generic `&T` shape was uncallable (no generic `0`/`100`
    /// comparison exists on `OrderedNumeric`); resolved per ADR-003 §8's
    /// "invariants become working methods" mandate rather than left
    /// `todo!()`.
    fn valid_percentage(number: &Real) -> bool
    where
        Self: Sized,
    {
        (0.0..=100.0).contains(&number.0)
    }

    /// `add` __alias__ `"+"` `(other: DV_AMOUNT) -> DV_AMOUNT`.
    ///
    /// Sum of this amount and another. The value of accuracy in the result
    /// is either:
    ///
    /// * the sum of the accuracies of the operands, if both present, or;
    /// * both operand accuracies are `unknown_accuracy_value`.
    ///
    /// If the accuracy value is a percentage in one operand and not in the
    /// other, the form in the result is that of the larger operand.
    ///
    /// Spec `Pre_comparable`: `is_strictly_comparable_to (other)`.
    ///
    /// PORT NOTE: the spec types both `other` and the result as the
    /// abstract `DV_AMOUNT` itself; narrowed to `&Self -> Self` here per
    /// the recurring pattern (see `dv_ordered.rs`'s
    /// `is_strictly_comparable_to` PORT NOTE) — every concrete `DV_AMOUNT`
    /// descendant transcribed in this package (`DV_QUANTITY`, `DV_COUNT`,
    /// `DV_PROPORTION`) in fact redefines this to the same-type shape in
    /// its own table.
    fn add(&self, other: &Self) -> Self
    where
        Self: Sized;

    /// `subtract` __alias__ `"-"` `(other: DV_AMOUNT) -> DV_AMOUNT`.
    ///
    /// Difference of this amount and another. The value of `accuracy` in
    /// the result is either:
    ///
    /// * the sum of the accuracies of the operands, if both present, or;
    /// * unknown, if either or both operand accuracies are unknown.
    ///
    /// If the `accuracy` value is a percentage in one operand and not in
    /// the other, the form in the result is that of the larger operand.
    ///
    /// Spec `Pre_comparable`: `is_strictly_comparable_to (other)`.
    fn subtract(&self, other: &Self) -> Self
    where
        Self: Sized;

    /// `is_equal(other: DV_AMOUNT) -> Boolean` (effected).
    ///
    /// Return `true` if this `DV_AMOUNT` is considered equal to `other`.
    fn is_equal_amount(&self, other: &Self) -> bool
    where
        Self: Sized;

    /// `multiply` __alias__ `"*"` `(factor: Real) -> DV_AMOUNT`.
    ///
    /// Product of this Amount and `factor`.
    fn multiply(&self, factor: &Real) -> Self
    where
        Self: Sized;

    /// `negative` __alias__ `"-"` `(): DV_AMOUNT`.
    ///
    /// Negated version of current object, such as used for representing a
    /// difference, e.g. a weight loss.
    fn negative(&self) -> Self
    where
        Self: Sized;

    /// `less_than` __alias__ `"<"` `(other: DV_AMOUNT) -> Boolean`
    /// (effected).
    ///
    /// True if this object is less than `other`. Based on comparison of
    /// `magnitude`.
    ///
    /// Spec `Post_result`: `Result = magnitude < other.magnitude`.
    fn less_than_amount(&self, other: &Self) -> bool
    where
        Self: Sized,
    {
        self.magnitude().less_than(&other.magnitude())
    }
}

impl<T: DvOrderedApi> Any for DvAmountData<T> {
    fn is_equal(&self, other: &Self) -> bool {
        self.accuracy_is_percent == other.accuracy_is_percent && self.accuracy == other.accuracy
    }

    fn type_of(&self) -> String {
        "DvAmountData".to_string()
    }
}

impl<T: DvOrderedApi> DvAmountData<T> {
    /// True if the recorded accuracy means "unknown": either the attribute
    /// is absent (`0..1`, not recorded) or it holds the spec's
    /// `unknown_accuracy_value` sentinel (`-1`).
    ///
    /// PORT NOTE: the spec models "not recorded" through the `-1` sentinel
    /// on a `0..1` attribute; absence has no distinct stated semantics, so
    /// both encodings are treated as unknown — the same flagged reading as
    /// every concrete leaf's `accuracy_unknown()`.
    pub fn accuracy_unknown(&self) -> bool {
        match self.accuracy {
            None => true,
            Some(a) => a.0 == UNKNOWN_ACCURACY_VALUE,
        }
    }

    /// `Accuracy_is_percent_validity` class invariant, as a working method
    /// per ADR-003 decision 8:
    ///
    /// `accuracy = 0 implies not accuracy_is_percent`.
    pub fn invariant_accuracy_is_percent_validity(&self) -> bool {
        self.accuracy != Some(Real(0.0)) || !self.accuracy_is_percent.unwrap_or(false)
    }

    /// `Accuracy_validity` class invariant, as a working method per
    /// ADR-003 decision 8:
    ///
    /// `accuracy_is_percent implies valid_percentage (accuracy)`.
    ///
    /// PORT NOTE: `valid_percentage`'s body is duplicated inline here
    /// (`0 <= accuracy <= 100`) rather than routed through
    /// `DvAmountApi::valid_percentage`, since `DvAmountData` is the
    /// embedded state struct, not itself a `DvAmountApi` implementor. An
    /// absent `accuracy` under `accuracy_is_percent = True` fails the
    /// invariant (there is no percentage value to be valid).
    pub fn invariant_accuracy_validity(&self) -> bool {
        !self.accuracy_is_percent.unwrap_or(false)
            || self.accuracy.is_some_and(|a| (0.0..=100.0).contains(&a.0))
    }
}

/// Accuracy-combination rule for `DV_AMOUNT.add`/`subtract`, shared by every
/// concrete leaf's `add`/`subtract` (`DvQuantity`, `DvCount`,
/// `DvProportion`), transcribed from `DV_AMOUNT`'s own prose (the spec
/// gives no formal `Post_result`):
///
/// * the result accuracy is the sum of the operand accuracies, if both
///   present (and neither unknown), or;
/// * unknown, if either or both operand accuracies are unknown;
/// * if the accuracy value is a percentage in one operand and not in the
///   other, the form in the result is that of the larger operand.
///
/// PORT NOTE: two interpretation points the prose leaves open, resolved
/// here and flagged:
///
/// 1. "unknown" in the result is encoded as **absence** (`None` accuracy,
///    `None` flag) rather than the `-1` sentinel — `accuracy_unknown()`
///    treats both identically, and absence cannot be mistaken for a real
///    measurement.
/// 2. "larger operand" is read as larger absolute `magnitude`; when the two
///    operands' forms differ, the other operand's accuracy is converted
///    into the winning form relative to its own magnitude (`a% of |m|` ↔
///    absolute half-range) before summing — summing a percent with an
///    absolute number directly would be dimensionally meaningless. A
///    zero-magnitude operand converts to `0` accuracy in the percent
///    direction (no finite percentage exists), flagged rather than silently
///    NaN.
pub fn combined_accuracy(
    lhs_magnitude: f64,
    lhs_accuracy: Option<Real>,
    lhs_accuracy_is_percent: Option<bool>,
    rhs_magnitude: f64,
    rhs_accuracy: Option<Real>,
    rhs_accuracy_is_percent: Option<bool>,
) -> (Option<Real>, Option<bool>) {
    let (Some(lhs_acc), Some(rhs_acc)) = (lhs_accuracy, rhs_accuracy) else {
        return (None, None);
    };
    if lhs_acc.0 == UNKNOWN_ACCURACY_VALUE || rhs_acc.0 == UNKNOWN_ACCURACY_VALUE {
        return (None, None);
    }

    let lhs_percent = lhs_accuracy_is_percent.unwrap_or(false);
    let rhs_percent = rhs_accuracy_is_percent.unwrap_or(false);
    // When the two forms differ, the result takes the *larger* operand's form
    // ("the form in the result is that of the larger operand"); when they
    // match, either yields the same value. (The equal-forms and larger-lhs
    // arms are the same value — `lhs_percent` — so they are merged.)
    let result_percent = if lhs_percent == rhs_percent || lhs_magnitude.abs() >= rhs_magnitude.abs()
    {
        lhs_percent
    } else {
        rhs_percent
    };

    let into_result_form = |accuracy: f64, is_percent: bool, magnitude: f64| -> f64 {
        if is_percent == result_percent {
            accuracy
        } else if result_percent {
            // absolute half-range → percent of the operand's own magnitude
            if magnitude == 0.0 {
                0.0
            } else {
                accuracy / magnitude.abs() * 100.0
            }
        } else {
            // percent of the operand's own magnitude → absolute half-range
            accuracy * magnitude.abs() / 100.0
        }
    };

    let sum = into_result_form(lhs_acc.0, lhs_percent, lhs_magnitude)
        + into_result_form(rhs_acc.0, rhs_percent, rhs_magnitude);
    let flag = if lhs_accuracy_is_percent.is_some() || rhs_accuracy_is_percent.is_some() {
        Some(result_percent)
    } else {
        None
    };
    (Some(Real(sum)), flag)
}

// TODO(port): `DvAmountApi` is not implemented for the `DvAmount` enum
// itself (contrast `dv_ordered.rs`'s `impl DvOrderedApi for DvOrdered`) —
// `DvAmountApi<T>` is generic over the magnitude type `T`, but the enum's
// four variants have four different concrete magnitude types
// (`DvQuantity`'s `f64`/`Real`, `DvCount`'s `Integer64`, etc.), so a single
// `impl DvAmountApi<T> for DvAmount` cannot be written without an
// associated-type or existential-`T` bridge not yet designed. Left as a
// documented gap for P17 (make-it-compile) triage; callers holding a
// concrete leaf type use that type's own `DvAmountApi` impl directly
// instead of going through the enum.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_count::DvCount;
    use crate::data_types::quantity::dv_ordered::DvOrderedData;
    use crate::data_types::quantity::dv_quantified::DvQuantifiedData;

    fn amount_data(
        accuracy: Option<f64>,
        accuracy_is_percent: Option<bool>,
    ) -> DvAmountData<DvCount> {
        DvAmountData {
            quantified: DvQuantifiedData {
                ordered: DvOrderedData {
                    normal_status: None,
                    normal_range: None,
                    other_reference_ranges: None,
                },
                magnitude_status: None,
                accuracy: None,
            },
            accuracy_is_percent,
            accuracy: accuracy.map(Real),
        }
    }

    /// Spec: "Test whether a number is a valid percentage, i.e. between 0
    /// and 100."
    #[test]
    fn valid_percentage_accepts_zero_to_one_hundred_inclusive() {
        assert!(<DvCount as DvAmountApi<i64>>::valid_percentage(&Real(0.0)));
        assert!(<DvCount as DvAmountApi<i64>>::valid_percentage(&Real(50.5)));
        assert!(<DvCount as DvAmountApi<i64>>::valid_percentage(&Real(
            100.0
        )));
        assert!(!<DvCount as DvAmountApi<i64>>::valid_percentage(&Real(
            -0.1
        )));
        assert!(!<DvCount as DvAmountApi<i64>>::valid_percentage(&Real(
            100.1
        )));
    }

    /// `Accuracy_is_percent_validity`: `accuracy = 0 implies not
    /// accuracy_is_percent`.
    #[test]
    fn accuracy_is_percent_validity_invariant() {
        assert!(amount_data(Some(0.0), None).invariant_accuracy_is_percent_validity());
        assert!(amount_data(Some(0.0), Some(false)).invariant_accuracy_is_percent_validity());
        assert!(!amount_data(Some(0.0), Some(true)).invariant_accuracy_is_percent_validity());
        assert!(amount_data(Some(5.0), Some(true)).invariant_accuracy_is_percent_validity());
        assert!(amount_data(None, Some(true)).invariant_accuracy_is_percent_validity());
    }

    /// `Accuracy_validity`: `accuracy_is_percent implies valid_percentage
    /// (accuracy)`.
    #[test]
    fn accuracy_validity_invariant() {
        assert!(amount_data(Some(5.0), Some(true)).invariant_accuracy_validity());
        assert!(!amount_data(Some(101.0), Some(true)).invariant_accuracy_validity());
        assert!(!amount_data(None, Some(true)).invariant_accuracy_validity());
        // Not a percentage: any accuracy value is fine for this invariant.
        assert!(amount_data(Some(250.0), Some(false)).invariant_accuracy_validity());
        assert!(amount_data(Some(250.0), None).invariant_accuracy_validity());
    }

    #[test]
    fn accuracy_unknown_covers_absence_and_sentinel() {
        assert!(amount_data(None, None).accuracy_unknown());
        assert!(amount_data(Some(UNKNOWN_ACCURACY_VALUE), None).accuracy_unknown());
        assert!(!amount_data(Some(0.5), None).accuracy_unknown());
    }

    /// Spec prose: "the sum of the accuracies of the operands, if both
    /// present".
    #[test]
    fn combined_accuracy_sums_matching_forms() {
        // Both absolute.
        assert_eq!(
            combined_accuracy(10.0, Some(Real(0.5)), None, 20.0, Some(Real(0.25)), None),
            (Some(Real(0.75)), None)
        );
        // Both percent.
        assert_eq!(
            combined_accuracy(
                10.0,
                Some(Real(2.0)),
                Some(true),
                20.0,
                Some(Real(3.0)),
                Some(true)
            ),
            (Some(Real(5.0)), Some(true))
        );
    }

    /// Spec prose: "unknown, if either or both operand accuracies are
    /// unknown".
    #[test]
    fn combined_accuracy_is_unknown_when_either_operand_unknown() {
        assert_eq!(
            combined_accuracy(10.0, None, None, 20.0, Some(Real(0.25)), None),
            (None, None)
        );
        assert_eq!(
            combined_accuracy(
                10.0,
                Some(Real(UNKNOWN_ACCURACY_VALUE)),
                None,
                20.0,
                Some(Real(0.25)),
                None
            ),
            (None, None)
        );
    }

    /// Spec prose: "If the accuracy value is a percentage in one operand
    /// and not in the other, the form in the result is that of the larger
    /// operand."
    #[test]
    fn combined_accuracy_mixed_forms_take_the_larger_operands_form() {
        // Larger operand (magnitude 100) is percent: 2% stays 2%; the
        // absolute 0.5 on magnitude 10 converts to 5%; sum = 7%.
        assert_eq!(
            combined_accuracy(
                100.0,
                Some(Real(2.0)),
                Some(true),
                10.0,
                Some(Real(0.5)),
                Some(false)
            ),
            (Some(Real(7.0)), Some(true))
        );
        // Larger operand (magnitude 100) is absolute: 1.0 stays; the 10%
        // on magnitude 10 converts to 1.0 absolute; sum = 2.0 absolute.
        assert_eq!(
            combined_accuracy(
                100.0,
                Some(Real(1.0)),
                Some(false),
                10.0,
                Some(Real(10.0)),
                Some(true)
            ),
            (Some(Real(2.0)), Some(false))
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_amount.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_amount.adoc §DV_AMOUNT Class
//   confidence: medium
//   todos: 2
//   note: UNKNOWN_ACCURACY_VALUE drawn from class description prose (not a table row) since the published table has no Constants section for this class; valid_percentage's parameter narrowed from the uncallable generic Ordered_Numeric to &Real (the invariant's only operand), now a working default; the two accuracy invariants implemented as invariant_* methods on DvAmountData per ADR-003 §8 and unit-tested; combined_accuracy transcribes DV_AMOUNT's prose accuracy rule (unknown-propagation + larger-operand form with documented conversion interpretation) for reuse by DvQuantity/DvCount/DvProportion add/subtract; DvAmountApi still has no impl for the DvAmount enum itself since its generic T varies per variant (remaining TODO). P4: DvAmountData<T> derives Serialize/Deserialize with `quantified` flattened; Real-lacks-serde gap now closed in openehr-foundation. ADR-002: DvAmountData is abstract, NO _type tag; DvAmount converted from #[serde(tag = "_type")] to #[serde(untagged)] — dispatch via each payload's own TypeTag (per-variant renames removed).
// ─────────────────────────────────────────────
