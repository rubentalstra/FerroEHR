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
use super::dv_amount::{DvAmountApi, DvAmountData, UNKNOWN_ACCURACY_VALUE};
use super::dv_ordered::DvOrderedApi;
use super::dv_quantified::DvQuantifiedApi;
// TODO(port): forward-references CODE_PHRASE (rm.data_types.text), not yet
// transcribed by the sibling package agent covering `data_types::text`.
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::primitive_types::real::Real;

/// Canonical `_type` discriminator string for this class in serialized
/// form (serde derives wait until P4 per ADR-001 "Refinements").
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
#[derive(Debug, Clone, PartialEq)]
pub struct DvCount {
    /// Embedded `DV_AMOUNT` parent state, self-typed per the F-bounded
    /// pattern documented on `DvOrderedData` in `dv_ordered.rs` (threaded
    /// through `DvQuantifiedData<T>`/`DvAmountData<T>`).
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
    /// Sum of this `DV_COUNT` and `other`.
    ///
    /// TODO(port): no explicit `Post_result` body given at this level; the
    /// natural reading is `magnitude + other.magnitude` with the
    /// `DV_AMOUNT`-level accuracy-combination prose applying to the
    /// inherited `accuracy`/`accuracy_is_percent` fields, but that
    /// accuracy-combination logic itself is not yet encoded (same gap as
    /// `DvQuantity::add`).
    fn add(&self, _other: &Self) -> Self {
        todo!(
            "DvCount::add: accuracy-combination rule from DV_AMOUNT's description not yet encoded"
        )
    }

    /// `subtract` __alias__ `"-"` `(other: DV_COUNT) -> DV_COUNT`
    /// (redefined).
    ///
    /// Difference of this `DV_COUNT` and `other`.
    ///
    /// TODO(port): same accuracy-combination gap as `add` above.
    fn subtract(&self, _other: &Self) -> Self {
        todo!(
            "DvCount::subtract: accuracy-combination rule from DV_AMOUNT's description not yet encoded"
        )
    }

    fn is_equal_amount(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    /// `multiply` __alias__ `"*"` `(factor: Real) -> DV_COUNT` (redefined).
    ///
    /// Product of this `DV_COUNT` and `factor`.
    ///
    /// TODO(port): no explicit `Post_result` body given; multiplying an
    /// integral `magnitude` by a `Real` `factor` raises the question of
    /// whether/how the result rounds back to an integer (`DV_COUNT` has no
    /// `precision` attribute, unlike `DV_QUANTITY`), which the spec does
    /// not address at this level. Left `todo!()` rather than guessing a
    /// rounding rule.
    fn multiply(&self, _factor: &Real) -> Self {
        todo!(
            "DvCount::multiply: no explicit Post_result body, and no stated rounding rule for a Real factor against an integral magnitude"
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

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_count.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_count.adoc §DV_COUNT Class
//   confidence: medium
//   todos: 5
//   note: magnitude: i64 is the ADR-001 §6 covariant-redefinition worked example named by this task, transcribed as a bare i64 (not the Integer64 newtype) directly on the struct with a doc note; this creates a trait-bound mismatch (i64 has no OrderedNumeric impl) flagged explicitly for P17 triage; add/subtract/multiply stubbed todo!() for the same accuracy-combination/rounding-rule gaps as DvQuantity; forward-references CODE_PHRASE pending sibling data_types::text package.
// ─────────────────────────────────────────────
