//! `DV_COUNT` — countable quantities.
//!
//! openEHR class: `DV_COUNT`, package `rm.data_types.quantity`.
//! Inherits: `DV_AMOUNT`.
//!
//! Countable quantities. Used for countable types such as pregnancies and
//! steps (taken by a physiotherapy patient), number of cigarettes smoked in
//! a day.
//!
//! Misuse: Not to be used for amounts of physical entities (which all have
//! units).
use super::dv_amount::{DvAmountApi, DvAmountData, UNKNOWN_ACCURACY_VALUE, combined_accuracy};
use super::dv_ordered::{DvOrderedApi, DvOrderedData};
use super::dv_quantified::DvQuantifiedApi;
// TODO(port): forward-references CODE_PHRASE (rm.data_types.text), not yet
// transcribed by the sibling package agent covering `data_types::text`.
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::primitive_types::real::Real;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "DV_COUNT";

/// `DV_COUNT` inherits `DV_AMOUNT` and adds a single attribute of its own,
/// `magnitude`.
///
/// # Covariant redefinition worked example (ADR-001 §6)
///
/// `magnitude`'s declared spec type is `Integer64` — a covariant narrowing
/// of the abstract `DV_QUANTIFIED.magnitude(): Ordered_Numeric` contract
/// down to a specific 64-bit integer type. Per ADR-001 §6 ("covariant
/// redefinition → narrowed type on the concrete struct"), and per this
/// package's own explicit worked-example instruction, `magnitude` is
/// transcribed as a bare `i64` **directly** on this struct — not wrapped in
/// the `openehr_foundation::primitive_types::integer64::Integer64` newtype,
/// and not left generic. This is a deliberate, more-literal-than-usual
/// choice: elsewhere in this package (`DvQuantity::magnitude: Real`), the
/// foundation newtype is used; here the bare primitive is used instead,
/// specifically because this is the field the task calls out as *the*
/// covariant-redefinition worked example for the whole package, and the
/// narrowing is most visible when the field's Rust type is the narrowest
/// possible representation (`i64`) rather than another layer of newtype
/// wrapping. No other field in this package deviates from the
/// newtype-wrapped convention.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvCount {
    /// Canonical `_type` discriminator (`"DV_COUNT"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_AMOUNT` parent state, self-typed per the F-bounded
    /// pattern documented on `DvOrderedData` in `dv_ordered.rs` (threaded
    /// through `DvQuantifiedData<T>`/`DvAmountData<T>`).
    #[serde(flatten)]
    pub amount: DvAmountData<DvCount>,

    /// `magnitude`: `Integer64` (1..1).
    ///
    /// **Covariant redefinition** (ADR-001 §6, this package's worked
    /// example): narrows the abstract `DV_QUANTIFIED.magnitude():
    /// Ordered_Numeric` to a 64-bit integer, transcribed as a bare `i64`
    /// directly on this struct rather than the `Integer64` newtype — see
    /// the struct-level doc comment for the full rationale.
    pub magnitude: i64,
}

/// ADR-002: `_type` string for `DV_COUNT`, single-sourced from
/// [`TYPE_NAME`].
impl TypeName for DvCount {
    const NAME: &'static str = TYPE_NAME;
}

impl DvCount {
    /// `normal_status`: accessor to the embedded parent state's attribute.
    pub fn normal_status(&self) -> Option<&CodePhrase> {
        self.amount.quantified.ordered.normal_status.as_ref()
    }

    /// `normal_range`: `DV_INTERVAL<DV_COUNT>` (0..1, redefined).
    ///
    /// Accessor into the embedded, self-typed `DvOrderedData<DvCount>` —
    /// see `DvQuantity`'s identical accessor for why no separate flat
    /// field is declared for this covariantly-redefined attribute.
    pub fn normal_range(&self) -> Option<&super::dv_interval::DvInterval<DvCount>> {
        self.amount.quantified.ordered.normal_range.as_deref()
    }

    /// `other_reference_ranges`: `List<REFERENCE_RANGE<DV_COUNT>>` (0..1,
    /// redefined).
    pub fn other_reference_ranges(
        &self,
    ) -> Option<&[super::reference_range::ReferenceRange<DvCount>]> {
        self.amount
            .quantified
            .ordered
            .other_reference_ranges
            .as_deref()
    }
}

impl Any for DvCount {
    /// `is_equal(other: DV_COUNT) -> Boolean`.
    ///
    /// PORT NOTE: `DV_COUNT`'s own table gives no explicit `is_equal` row
    /// (inherited from `DV_AMOUNT.is_equal` unchanged); this default body
    /// compares `magnitude` directly, mirroring the same situation on
    /// `DvOrdinal`/`DvScale`/`DvQuantity`.
    fn is_equal(&self, other: &Self) -> bool {
        self.magnitude == other.magnitude
    }

    fn type_of(&self) -> String {
        "DvCount".to_string()
    }
}

impl Ordered for DvCount {
    /// `less_than` __alias__ `"<"` `(other: DV_COUNT) -> Boolean`
    /// (effected).
    ///
    /// True if `other` is less than this Quantified object. Based on
    /// comparison of `magnitude`.
    ///
    /// Spec `Post_result`: `Result = magnitude < other.magnitude`.
    fn less_than(&self, other: &Self) -> bool {
        self.magnitude < other.magnitude
    }
}

impl DvOrderedApi for DvCount {
    fn normal_status(&self) -> Option<&CodePhrase> {
        self.amount.quantified.ordered.normal_status.as_ref()
    }

    fn ordered_data(&self) -> Option<&DvOrderedData<Self>> {
        Some(&self.amount.quantified.ordered)
    }

    /// `is_strictly_comparable_to(other: DV_ORDERED) -> Boolean`
    /// (effected).
    ///
    /// Return `true`.
    ///
    /// PORT NOTE: the spec table's own `Meaning` cell for this row reads
    /// verbatim "Return True" — `DV_COUNT` instances are always considered
    /// strictly comparable to one another, unlike `DV_QUANTITY` (which
    /// requires matching `units`).
    fn is_strictly_comparable_to(&self, _other: &Self) -> bool {
        true
    }
}

// PORT NOTE: `DvAmountApi<T: OrderedNumeric>`/`DvQuantifiedApi<T:
// OrderedNumeric>` require `T: OrderedNumeric`, but `magnitude` above is a
// bare `i64` per this file's covariant-redefinition worked-example
// instruction (see the struct-level doc comment) rather than the
// `openehr_foundation::primitive_types::integer64::Integer64` newtype that
// actually implements `Ordered`/`Numeric`/`OrderedNumeric`. This impl
// therefore does not satisfy the trait bound as written — `i64` has no
// `OrderedNumeric` impl in `openehr-foundation`. Left in place (rather than
// silently switched back to the `Integer64` newtype, which would undercut
// the worked-example instruction) since Phase A does not require
// compilation; flagged here explicitly so a P17 (make-it-compile) triage
// finds this note rather than rediscovering the mismatch from a raw
// compiler error. TODO(port): resolve at P17 by either (a) giving `i64` an
// `OrderedNumeric` impl in `openehr-foundation` specifically for this
// covariant-redefinition case, or (b) accepting that this file's `magnitude:
// i64` and its `DvAmountApi`/`DvQuantifiedApi` impls are two different
// design pulls (bare-primitive narrowing vs. generic-trait-bound
// compatibility) that cannot both be satisfied without foundation-layer
// changes.
impl DvQuantifiedApi<i64> for DvCount {
    fn magnitude_status(&self) -> Option<&str> {
        self.amount.quantified.magnitude_status.as_deref()
    }

    /// `magnitude(): Integer64` (effected, covariantly narrowed — see the
    /// struct-level doc comment) — the declared `magnitude` attribute
    /// doubles as the effected `DV_QUANTIFIED.magnitude()` accessor.
    fn magnitude(&self) -> i64 {
        self.magnitude
    }

    /// `accuracy_unknown(): Boolean` (effected via `DV_AMOUNT`'s
    /// special-value convention: an `accuracy` of `unknown_accuracy_value`
    /// (-1) means accuracy was not recorded).
    ///
    /// PORT NOTE: an absent (`None`) accuracy is also treated as unknown —
    /// see `DvQuantity::accuracy_unknown` for the same flagged reading.
    fn accuracy_unknown(&self) -> bool {
        match self.amount.accuracy {
            None => true,
            Some(a) => a.is_equal(&Real(UNKNOWN_ACCURACY_VALUE)),
        }
    }

    fn is_equal_quantified(&self, other: &Self) -> bool {
        self.is_equal(other)
    }
}

impl DvAmountApi<i64> for DvCount {
    fn accuracy_is_percent(&self) -> Option<bool> {
        self.amount.accuracy_is_percent
    }

    fn accuracy(&self) -> Option<Real> {
        self.amount.accuracy
    }

    /// `add` __alias__ `"+"` `(other: DV_COUNT) -> DV_COUNT` (redefined).
    ///
    /// Sum of this `DV_COUNT` and `other`: the (integer) magnitudes are
    /// summed, and the inherited `accuracy`/`accuracy_is_percent` follow
    /// `DV_AMOUNT`'s combination prose, encoded once in [`combined_accuracy`]
    /// (the sum when both known, unknown if either is unknown, larger-operand
    /// form when the two forms differ).
    ///
    /// PORT NOTE: `DV_COUNT`'s table gives no `Post_result`, so — as with
    /// `DvQuantity::add` — the receiver's other fields (any reference ranges /
    /// `normal_status`) are carried over via `clone`, consistent with the
    /// already-shipped `negative()`.
    fn add(&self, other: &Self) -> Self {
        let (accuracy, accuracy_is_percent) = combined_accuracy(
            self.magnitude as f64,
            self.amount.accuracy,
            self.amount.accuracy_is_percent,
            other.magnitude as f64,
            other.amount.accuracy,
            other.amount.accuracy_is_percent,
        );
        let mut result = self.clone();
        result.magnitude = self.magnitude + other.magnitude;
        result.amount.accuracy = accuracy;
        result.amount.accuracy_is_percent = accuracy_is_percent;
        result
    }

    /// `subtract` __alias__ `"-"` `(other: DV_COUNT) -> DV_COUNT`
    /// (redefined).
    ///
    /// Difference of this `DV_COUNT` and `other`. Same accuracy-combination
    /// and field-carry-over rules as [`Self::add`]; the magnitude is the
    /// difference of the two magnitudes.
    fn subtract(&self, other: &Self) -> Self {
        let (accuracy, accuracy_is_percent) = combined_accuracy(
            self.magnitude as f64,
            self.amount.accuracy,
            self.amount.accuracy_is_percent,
            other.magnitude as f64,
            other.amount.accuracy,
            other.amount.accuracy_is_percent,
        );
        let mut result = self.clone();
        result.magnitude = self.magnitude - other.magnitude;
        result.amount.accuracy = accuracy;
        result.amount.accuracy_is_percent = accuracy_is_percent;
        result
    }

    fn is_equal_amount(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    /// `multiply` __alias__ `"*"` `(factor: Real) -> DV_COUNT` (redefined).
    ///
    /// Product of this `DV_COUNT` and `factor`.
    ///
    /// TODO(port): genuine published-spec defect — multiplying an integral
    /// `magnitude` by a `Real` `factor` must yield a `DV_COUNT` (integer
    /// magnitude again), but `DV_COUNT` has no `precision` attribute (unlike
    /// `DV_QUANTITY`) and the spec states no rounding/truncation rule for
    /// coercing the `Real` product back to an integer. No spec-faithful body
    /// exists without inventing that rule, so this stays `todo!()` (unlike
    /// `add`/`subtract`, whose integer results are exact). Contrast
    /// `DvQuantity::multiply`, whose `Real` magnitude has no coercion
    /// problem. Revisit if a reference-behaviour rounding rule surfaces
    /// (P17/P18).
    fn multiply(&self, _factor: &Real) -> Self {
        todo!(
            "DvCount::multiply: published-spec defect — no stated rounding rule for coercing a Real (magnitude * factor) product back to DV_COUNT's integer magnitude"
        )
    }

    /// `negative` __alias__ `"-"` `(): DV_COUNT`.
    ///
    /// PORT NOTE: `DV_COUNT`'s own table does not re-list `negative` with a
    /// `(redefined)` marker; inherited from `DV_AMOUNT.negative` unchanged.
    /// Transcribed here because `DvAmountApi` requires it with no default
    /// body (see `dv_amount.rs`); the natural same-type negation is
    /// magnitude negation.
    fn negative(&self) -> Self {
        DvCount {
            magnitude: -self.magnitude,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_ordered::DvOrderedData;
    use crate::data_types::quantity::dv_quantified::DvQuantifiedData;

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

    fn count_with_accuracy(magnitude: i64, accuracy: f64) -> DvCount {
        let mut c = count(magnitude);
        c.amount.accuracy = Some(Real(accuracy));
        c
    }

    /// `is_strictly_comparable_to`: the spec Meaning cell says "Return True".
    #[test]
    fn always_strictly_comparable() {
        assert!(count(1).is_strictly_comparable_to(&count(999)));
    }

    /// `less_than`: `Result = magnitude < other.magnitude`.
    #[test]
    fn less_than_compares_magnitude() {
        assert!(count(1).less_than(&count(2)));
        assert!(!count(2).less_than(&count(1)));
        assert!(!count(2).less_than(&count(2)));
    }

    /// `add`: integer magnitudes sum.
    #[test]
    fn add_sums_integer_magnitudes() {
        assert_eq!(count(2).add(&count(3)).magnitude, 5);
        assert_eq!(count(2).add(&count(-5)).magnitude, -3);
    }

    /// `add`: accuracies sum when both present and known.
    #[test]
    fn add_sums_accuracies_when_both_present() {
        let result = count_with_accuracy(2, 0.5).add(&count_with_accuracy(3, 0.25));
        assert_eq!(result.magnitude, 5);
        assert_eq!(result.amount.accuracy, Some(Real(0.75)));
    }

    /// `subtract`: integer magnitudes subtract.
    #[test]
    fn subtract_differences_integer_magnitudes() {
        assert_eq!(count(5).subtract(&count(3)).magnitude, 2);
    }

    /// `negative`: magnitude flips sign (already shipped).
    #[test]
    fn negative_flips_magnitude() {
        assert_eq!(count(4).negative().magnitude, -4);
    }

    /// `is_simple` routes through the overridden `ordered_data()`.
    #[test]
    fn is_simple_reflects_the_embedded_ordered_state() {
        assert!(count(5).is_simple());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_count.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_count.adoc §DV_COUNT Class
//   confidence: medium
//   todos: 3
//   note: magnitude: i64 is the ADR-001 §6 covariant-redefinition worked example named by this task, transcribed as a bare i64 (not the Integer64 newtype) directly on the struct with a doc note; this creates a DvAmountApi/DvQuantifiedApi trait-bound mismatch (i64 has no OrderedNumeric impl) flagged explicitly for P17 triage. add/subtract implemented (integer magnitude sum/difference + DV_AMOUNT accuracy-combination prose via combined_accuracy); multiply kept as a genuine spec-defect todo!() (no stated rounding rule to coerce a Real product back to an integer magnitude — DV_COUNT has no precision attribute); negative already shipped; ordered_data() overridden so is_simple/is_normal reach the embedded state — all unit-tested. Remaining TODO: forward-reference CODE_PHRASE pending the sibling data_types::text package (present in-tree; reconciled at P17). P4/ADR-002: self-tags via TypeTag<Self> first field + TypeName reusing TYPE_NAME; `amount` flattened; magnitude is a bare primitive so no cross-crate serde dependency on this field.
// ─────────────────────────────────────────────
