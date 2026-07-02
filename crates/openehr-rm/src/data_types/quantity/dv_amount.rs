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
use super::dv_ordered::{DvOrderedApi, DvOrderedData};
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
use openehr_foundation::primitive_types::ordered_numeric::OrderedNumeric;
use openehr_foundation::primitive_types::real::Real;

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
#[derive(Debug, Clone, PartialEq)]
pub struct DvAmountData<T: DvOrderedApi> {
    /// Embedded `DV_QUANTIFIED` parent state.
    pub quantified: DvQuantifiedData<T>,

    /// `accuracy_is_percent`: `Boolean` (0..1).
    ///
    /// If `True`, indicates that when this object was created, `accuracy`
    /// was recorded as a percent value; if `False`, as an absolute
    /// quantity value.
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
#[derive(Debug, Clone, PartialEq)]
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
    /// itself; transcribed here over the same `T: OrderedNumeric` this
    /// trait is already generic over (matching `DvQuantifiedApi::magnitude`'s
    /// own `T` parameter), rather than introducing a second, independent
    /// generic parameter — the spec gives no indication `number`'s type
    /// need differ from the class's own magnitude type.
    fn valid_percentage(_number: &T) -> bool
    where
        Self: Sized,
    {
        // TODO(port): `OrderedNumeric` (the blanket-implemented composition
        // of `Ordered + Numeric`) does not itself expose a way to compare
        // against the literal Rust values `0`/`100` generically — doing so
        // needs either a `From<Integer>`-style conversion or a
        // numeric-literal trait bound not yet part of `OrderedNumeric`'s
        // shape. Left unresolved pending that foundation-layer decision.
        todo!(
            "DvAmountApi::valid_percentage: needs a generic 0/100 comparison over T: OrderedNumeric"
        )
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

// TODO(port): the two class invariants below are not yet encoded as a
// `Validate` impl, per `.claude/rules/rm-transcription.md`'s "Invariants"
// section:
//
// - `Accuracy_is_percent_validity`: `accuracy = 0 implies not
//   accuracy_is_percent`
// - `Accuracy_validity`: `accuracy_is_percent implies valid_percentage
//   (accuracy)`

// TODO(port): `DvAmountApi` is not implemented for the `DvAmount` enum
// itself (contrast `dv_ordered.rs`'s `impl DvOrderedApi for DvOrdered`) —
// `DvAmountApi<T>` is generic over the magnitude type `T`, but the enum's
// four variants have four different concrete magnitude types
// (`DvQuantity`'s `f64`/`Real`, `DvCount`'s `Integer64`, etc.), so a single
// `impl DvAmountApi<T> for DvAmount` cannot be written without an
// associated-type or existential-`T` bridge not yet designed. Left as a
// documented gap; callers holding a concrete leaf type use that type's own
// `DvAmountApi` impl directly instead of going through the enum.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_amount.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_amount.adoc §DV_AMOUNT Class
//   confidence: medium
//   todos: 4
//   note: UNKNOWN_ACCURACY_VALUE drawn from class description prose (not a table row) since the published table has no Constants section for this class; valid_percentage stubbed pending a generic 0/100 comparison bound over T: OrderedNumeric; the two accuracy invariants recorded but not enforced; DvAmountApi has no impl for the DvAmount enum itself since its generic T varies per variant.
// ─────────────────────────────────────────────
