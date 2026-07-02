//! `DV_ABSOLUTE_QUANTITY` — abstract class defining the concept of
//! quantified entities whose values are absolute with respect to an origin.
//!
//! openEHR class: `DV_ABSOLUTE_QUANTITY` (abstract), package
//! `rm.data_types.quantity`.
//! Inherits: `DV_QUANTIFIED`.
//!
//! Abstract class defining the concept of quantified entities whose values
//! are absolute with respect to an origin. Dates and Times are the main
//! example.
use super::dv_amount::DvAmount;
use super::dv_ordered::DvOrderedApi;
use super::dv_quantified::{DvQuantifiedApi, DvQuantifiedData};
use openehr_foundation::primitive_types::ordered_numeric::OrderedNumeric;

/// Shared attribute state of `DV_ABSOLUTE_QUANTITY` and its descendants.
///
/// Per ADR-001 §3, embedded by every concrete `DV_ABSOLUTE_QUANTITY`
/// subtype. Per the package overview ("The main example of absolute
/// quantities are the temporal concepts date, time and date/time"), the
/// concrete descendants of this class are `DV_DATE`, `DV_TIME`,
/// `DV_DATE_TIME` (owned by the sibling `date_time` package, not this one —
/// see the task's variant-ownership note on `dv_ordered::DvOrdered`).
///
/// `T: DvOrderedApi` threads the same F-bounded self-type as
/// `DvQuantifiedData<T>` (see `dv_ordered.rs`, `dv_quantified.rs`), since
/// `DV_ABSOLUTE_QUANTITY`'s `Inherit` row is `DV_QUANTIFIED`.
#[derive(Debug, Clone, PartialEq)]
pub struct DvAbsoluteQuantityData<T: DvOrderedApi> {
    /// Embedded `DV_QUANTIFIED` parent state.
    pub quantified: DvQuantifiedData<T>,

    /// `accuracy`: `DV_AMOUNT` (0..1, redefined).
    ///
    /// **Covariant redefinition** (ADR-001 §6): the spec's attribute table
    /// marks this `0..1 (redefined)`, narrowing the abstract
    /// `DV_QUANTIFIED.accuracy: Any` down to `DV_AMOUNT` specifically —
    /// distinct from `DV_AMOUNT`'s own redefinition of the same slot to
    /// `Real` (see `dv_amount.rs::DvAmountData::accuracy`). The class gives
    /// no further description of this attribute beyond the table row
    /// itself.
    ///
    /// PORT NOTE: `DvAmount` here is the closed-enum encoding of the
    /// abstract `DV_AMOUNT` class (per ADR-001 §4), analogous to how
    /// `DvOrdered` closes `DV_ORDERED` — see `dv_amount.rs`. No concrete
    /// leaf in *this* package (`data_types.quantity`) is a `DV_AMOUNT`
    /// descendant that is not already a `DV_ORDERED`-cycle participant
    /// (`DV_QUANTITY`, `DV_COUNT`, `DV_PROPORTION` are all `DV_AMOUNT`
    /// descendants, so `DvAmount` can validly hold any of them here).
    pub accuracy: Option<DvAmount>,
}

/// Behaviour trait shared by every `DV_ABSOLUTE_QUANTITY` descendant.
///
/// Extends [`DvQuantifiedApi`] (`DV_ABSOLUTE_QUANTITY` inherits
/// `DV_QUANTIFIED`) with the absolute-quantity-specific members.
///
/// Per the class description: "For this reason, the operations `add`,
/// `subtract` and `diff` are defined rather than `+` or `-`" — i.e. unlike
/// `DV_AMOUNT` (which reuses `+`/`-` as aliases for `add`/`subtract`),
/// `DV_ABSOLUTE_QUANTITY`'s own table gives `add`/`subtract`/`diff` no
/// `__alias__` row at all, matching the prose's explicit point that the
/// symbolic operators are deliberately *not* reused here.
pub trait DvAbsoluteQuantityApi<T: OrderedNumeric>: DvQuantifiedApi<T> {
    /// `accuracy`: optional accuracy value (redefined to `DV_AMOUNT` at
    /// this level).
    fn accuracy(&self) -> Option<&DvAmount>;

    /// `add(a_diff: DV_AMOUNT) -> DV_ABSOLUTE_QUANTITY` (abstract).
    ///
    /// Addition of a differential amount to this quantity.
    ///
    /// The value of accuracy in the result is either:
    ///
    /// * the sum of the accuracies of the operands, if both present, or;
    /// * unknown, if either or both operand accuracies are unknown.
    ///
    /// PORT NOTE: the spec types the result as the abstract
    /// `DV_ABSOLUTE_QUANTITY` itself; narrowed to `Self` here per the
    /// recurring pattern (see `dv_ordered.rs`'s `is_strictly_comparable_to`
    /// PORT NOTE).
    fn add(&self, a_diff: &DvAmount) -> Self
    where
        Self: Sized;

    /// `subtract(a_diff: DV_AMOUNT) -> DV_ABSOLUTE_QUANTITY` (abstract).
    ///
    /// Result of subtracting a differential amount from this quantity.
    ///
    /// The value of `accuracy` in the result is either:
    ///
    /// * the sum of the accuracies of the operands, if both present, or;
    /// * unknown, if either or both operand accuracies are unknown.
    fn subtract(&self, a_diff: &DvAmount) -> Self
    where
        Self: Sized;

    /// `diff(other: DV_ABSOLUTE_QUANTITY) -> DV_AMOUNT` (abstract).
    ///
    /// Difference of two quantities.
    ///
    /// The value of accuracy in the result is either:
    ///
    /// * the sum of the accuracies of the operands, if both present, or;
    /// * unknown, if either or both operand accuracies are unknown.
    ///
    /// PORT NOTE: the spec's `__alias__ "-"` marker on this row is
    /// transcribed as a named method (`diff`), not a `std::ops::Sub`
    /// overload, per the RM transcription rule for symbolic operators.
    fn diff(&self, other: &Self) -> DvAmount
    where
        Self: Sized;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_absolute_quantity.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_absolute_quantity.adoc §DV_ABSOLUTE_QUANTITY Class
//   confidence: medium
//   todos: 0
//   note: no concrete leaf transcribed in this pass (the date_time DV_DATE/DV_TIME/DV_DATE_TIME descendants belong to the sibling package); DvAbsoluteQuantityData/DvAbsoluteQuantityApi are the abstract shape those types are expected to embed/implement once that package lands.
// ─────────────────────────────────────────────
