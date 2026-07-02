//! `DV_PROPORTION` — models a ratio of values.
//!
//! openEHR class: `DV_PROPORTION`, package `rm.data_types.quantity`.
//! Inherits: `PROPORTION_KIND`, `DV_AMOUNT` (multiple inheritance).
//!
//! Models a ratio of values, i.e. where the numerator and denominator are
//! both pure numbers. The `valid_proportion_kind` property of the
//! `PROPORTION_KIND` class is used to control the type attribute to be one
//! of a defined set.
//!
//! Used for recording titers (e.g. 1:128), concentration ratios, e.g. Na:K
//! (unitary denominator), albumin:creatinine ratio, and percentages, e.g.
//! red cell distribution width (RDW).
//!
//! Misuse: Should not be used to represent things like blood pressure which
//! are often written using a '/' character, giving the misleading
//! impression that the item is a ratio, when in fact it is a structured
//! value. Similarly, visual acuity, often written as (e.g.) "6/24" in
//! clinical notes is not a ratio but an ordinal (which includes non-numeric
//! symbols like CF = count fingers etc). Should not be used for
//! formulations.
use super::dv_amount::{DvAmountApi, DvAmountData};
use super::dv_ordered::{DvOrderedApi, DvOrderedData};
use super::proportion_kind::ProportionKind;
// TODO(port): forward-references CODE_PHRASE (rm.data_types.text), not yet
// transcribed by the sibling package agent covering `data_types::text`.
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::integer::Integer;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::primitive_types::real::Real;

/// Canonical `_type` discriminator string for this class in serialized
/// form (serde derives wait until P4 per ADR-001 "Refinements").
pub const TYPE_NAME: &str = "DV_PROPORTION";

/// `DV_PROPORTION`'s `Inherit` row lists **two** parents: `PROPORTION_KIND`
/// and `DV_AMOUNT` — genuine multiple inheritance (ADR-001 §2). Per the RM
/// transcription rule ("Multiple inheritance ... is composed fields from
/// all parents plus one trait per parent behaviour"):
///
/// * `DV_AMOUNT` state is embedded via `amount: DvAmountData<DvProportion>`
///   (self-typed per the same F-bounded pattern as every other
///   `DV_AMOUNT`/`DV_ORDERED` descendant in this package).
/// * `PROPORTION_KIND` is a **constants-only** class (see
///   `proportion_kind.rs`) — it declares no instance attributes, only
///   integer constants (`pk_ratio`, `pk_unitary`, etc.) and a single
///   function (`valid_proportion_kind`). Per the same "constants-class
///   inheritance" pattern already used for `Time_Definitions`
///   (`openehr_foundation::time::time_definitions`, see the ROSETTA row for
///   that class), `PROPORTION_KIND`'s constants/function are reached via
///   direct calls to `ProportionKind::*` rather than a supertrait or an
///   embedded zero-sized field.
#[derive(Debug, Clone, PartialEq)]
pub struct DvProportion {
    /// Embedded `DV_AMOUNT` parent state, self-typed per the F-bounded
    /// pattern documented on `DvOrderedData` in `dv_ordered.rs` (threaded
    /// through `DvQuantifiedData<T>`/`DvAmountData<T>`).
    pub amount: DvAmountData<DvProportion>,

    /// `numerator`: `Real` (1..1).
    ///
    /// Numerator of ratio.
    pub numerator: Real,

    /// `denominator`: `Real` (1..1).
    ///
    /// Denominator of ratio.
    ///
    /// Invariant `Valid_denominator`: `denominator /= 0.0`.
    pub denominator: Real,

    /// `type`: `Integer` (1..1).
    ///
    /// Indicates semantic type of proportion, including percent, unitary
    /// etc.
    ///
    /// PORT NOTE: named `type_` because `type` is a Rust reserved keyword,
    /// matching the same rename already used on
    /// `openehr_base::identification::object_ref::ObjectRef::r#type`
    /// (though that one uses the raw-identifier form `r#type` since it is
    /// a single-word attribute with no natural alternative spelling; here
    /// `type_` with a trailing underscore is used since a bare `r#type`
    /// would still visually collide with the Rust keyword in code that
    /// reads it, and the openEHR attribute's own semantic name —
    /// "proportion kind/type" — reads naturally with the trailing
    /// underscore convention).
    ///
    /// Invariant `Type_validity`: `valid_proportion_kind (type)`.
    ///
    /// Declared as the closed [`ProportionKind`] enum rather than a bare
    /// `Integer`, even though the spec's own table types this attribute
    /// `Integer` — the whole reason `PROPORTION_KIND` exists as a class is
    /// to name this closed set of five integer values, and modelling it as
    /// a closed Rust enum keeps `Type_validity` true by construction rather
    /// than needing a runtime range check against a bare integer. Flagged
    /// as a judgment call beyond the literal table declaration.
    pub type_: ProportionKind,

    /// `precision`: `Integer` (0..1).
    ///
    /// Precision to which the `numerator` and `denominator` values of the
    /// proportion are expressed, in terms of number of decimal places. The
    /// value 0 implies an integral quantity. The value -1 implies no
    /// limit, i.e. any number of decimal places.
    ///
    /// Invariant `Precision_validity`: `precision = 0 implies is_integral`.
    pub precision: Option<Integer>,
}

impl DvProportion {
    /// `normal_status`: accessor to the embedded parent state's attribute.
    pub fn normal_status(&self) -> Option<&CodePhrase> {
        self.amount.quantified.ordered.normal_status.as_ref()
    }

    /// `normal_range`: `DV_INTERVAL<DV_PROPORTION>` (0..1, redefined).
    ///
    /// Accessor into the embedded, self-typed `DvOrderedData<DvProportion>`
    /// — see `DvQuantity`'s identical accessor for why no separate flat
    /// field is declared for this covariantly-redefined attribute.
    pub fn normal_range(&self) -> Option<&super::dv_interval::DvInterval<DvProportion>> {
        self.amount.quantified.ordered.normal_range.as_deref()
    }

    /// `other_reference_ranges`: `List<REFERENCE_RANGE<DV_PROPORTION>>`
    /// (0..1, redefined).
    pub fn other_reference_ranges(
        &self,
    ) -> Option<&[super::reference_range::ReferenceRange<DvProportion>]> {
        self.amount
            .quantified
            .ordered
            .other_reference_ranges
            .as_deref()
    }

    /// `magnitude(): Real` (effected).
    ///
    /// Effective magnitude represented by ratio.
    ///
    /// PORT NOTE: the spec gives no `Post_result` for this effector, but
    /// the class description states plainly "consists of numerator and
    /// denominator Real values, and a magnitude function which is computed
    /// as the result of the numerator/denominator division" — transcribed
    /// directly from that prose.
    pub fn magnitude(&self) -> Real {
        self.numerator.divide(&self.denominator)
    }

    /// `is_integral(): Boolean`.
    ///
    /// True if the `numerator` and `denominator` values are integers, i.e.
    /// if `precision` is 0.
    pub fn is_integral(&self) -> bool {
        matches!(self.precision, Some(Integer(0)))
    }
}

impl Any for DvProportion {
    /// `is_equal(other: DV_PROPORTION) -> Boolean` (effected).
    ///
    /// Return `true` if this `DV_AMOUNT` is considered equal to `other`.
    ///
    /// PORT NOTE: the table's own `Meaning` cell literally says "this
    /// `DV_AMOUNT`" (not "this `DV_PROPORTION`") — transcribed verbatim as
    /// a likely copy-paste artifact from `DV_AMOUNT.is_equal`'s own row,
    /// flagged rather than silently corrected.
    fn is_equal(&self, other: &Self) -> bool {
        self.numerator.is_equal(&other.numerator)
            && self.denominator.is_equal(&other.denominator)
            && self.type_ == other.type_
            && self.precision == other.precision
    }

    fn type_of(&self) -> String {
        "DvProportion".to_string()
    }
}

impl Ordered for DvProportion {
    /// `less_than` __alias__ `"<"` `(other: DV_PROPORTION) -> Boolean`
    /// (effected).
    ///
    /// True if this Proportion is less than `other`. Only valid if
    /// `is_strictly_comparable_to()` is `True`.
    ///
    /// Spec `Post_result`: `Result = magnitude < other.magnitude`.
    fn less_than(&self, other: &Self) -> bool {
        self.magnitude().less_than(&other.magnitude())
    }
}

impl DvOrderedApi for DvProportion {
    fn normal_status(&self) -> Option<&CodePhrase> {
        self.amount.quantified.ordered.normal_status.as_ref()
    }

    /// `is_strictly_comparable_to(other: DV_ORDERED) -> Boolean`
    /// (effected).
    ///
    /// Return `true` if the `type` of this proportion is the same as the
    /// `type` of `other`.
    ///
    /// PORT NOTE: the spec types `other` as the abstract `DV_ORDERED`;
    /// narrowed to `&Self` per the recurring pattern.
    fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        self.type_ == other.type_
    }
}

impl DvAmountApi<Real> for DvProportion {
    fn accuracy_is_percent(&self) -> Option<bool> {
        self.amount.accuracy_is_percent
    }

    fn accuracy(&self) -> Option<Real> {
        self.amount.accuracy
    }

    /// `add` __alias__ `"+"` `(other: DV_PROPORTION) -> DV_PROPORTION`
    /// (redefined).
    ///
    /// Sum of two strictly comparable proportions.
    ///
    /// TODO(port): the spec gives no explicit `Post_result` body — summing
    /// two ratios is not simply summing numerators and denominators
    /// independently (that would not preserve the ratio semantics), and the
    /// correct combination rule (common-denominator addition? treating
    /// `magnitude()` as the operand?) is not stated. Left `todo!()` rather
    /// than guessing.
    fn add(&self, _other: &Self) -> Self {
        todo!(
            "DvProportion::add: no explicit Post_result body, and no stated ratio-combination rule"
        )
    }

    /// `subtract` __alias__ `"-"` `(other: DV_PROPORTION) -> DV_PROPORTION`
    /// (redefined).
    ///
    /// Difference between two strictly comparable proportions.
    ///
    /// TODO(port): same ratio-combination gap as `add` above.
    fn subtract(&self, _other: &Self) -> Self {
        todo!(
            "DvProportion::subtract: no explicit Post_result body, and no stated ratio-combination rule"
        )
    }

    fn is_equal_amount(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    /// `multiply` __alias__ `"*"` `(factor: Real) -> DV_PROPORTION`
    /// (redefined).
    ///
    /// Product of this Proportion and `factor`.
    ///
    /// TODO(port): no explicit `Post_result` body; multiplying a ratio by a
    /// scalar factor could scale the numerator, the denominator, or both in
    /// a way that changes `type_` validity (e.g. a `pk_unitary` proportion
    /// requires `denominator = 1`, which scaling the denominator would
    /// violate) — left `todo!()` rather than guessing which operand to
    /// scale.
    fn multiply(&self, _factor: &Real) -> Self {
        todo!(
            "DvProportion::multiply: no explicit Post_result body, and scaling numerator vs denominator affects type_ validity differently"
        )
    }

    /// `negative` __alias__ `"-"` `(): DV_PROPORTION`.
    ///
    /// PORT NOTE: `DV_PROPORTION`'s own table does not list `negative` at
    /// all (contrast `add`/`subtract`/`multiply`, all explicitly marked
    /// `(redefined)`, and `is_equal`/`less_than`/`is_strictly_comparable_to`,
    /// all explicitly marked `(effected)`) — `negative` is entirely absent
    /// from this class's own Functions table. It is nonetheless inherited
    /// from `DV_AMOUNT.negative` (never overridden means the parent
    /// implementation still applies conceptually), and `DvAmountApi`
    /// requires it with no default body (see `dv_amount.rs`).
    ///
    /// TODO(port): unlike `DvQuantity`/`DvCount` (where negating a single
    /// `magnitude` field is unambiguous), negating a ratio's *numerator*
    /// (`-numerator/denominator`) versus its *denominator*
    /// (`numerator/-denominator`) are both mathematically valid but
    /// distinct representations that both satisfy
    /// `magnitude() == -original.magnitude()`; the spec gives no guidance
    /// at any level for the ratio case specifically. Left `todo!()` rather
    /// than picking one arbitrarily.
    fn negative(&self) -> Self {
        todo!(
            "DvProportion::negative: negating numerator vs denominator both satisfy magnitude() == -original.magnitude(), spec gives no guidance for the ratio case"
        )
    }
}

// TODO(port): the seven class invariants below are not yet encoded as a
// `Validate` impl, per `.claude/rules/rm-transcription.md`'s "Invariants"
// section:
//
// - `Type_validity`: `valid_proportion_kind (type)`
// - `Precision_validity`: `precision = 0 implies is_integral`
// - `Is_integral_validity`: `is_integral implies (numerator.floor =
//   numerator and denominator.floor = denominator)`
// - `Fraction_validity`: `(type = pk_fraction or type = pk_integer_fraction)
//   implies is_integral`
// - `Unitary_validity`: `type = pk_unitary implies denominator = 1`
// - `Percent_validity`: `type = pk_percent implies denominator = 100`
// - `Valid_denominator`: `denominator /= 0.0`
//
// Several of these (`Type_validity`, `Unitary_validity`, `Percent_validity`,
// `Fraction_validity`) are partially structural once `type_: ProportionKind`
// is a closed enum rather than a bare `Integer` (see `type_`'s own doc
// comment) — `Type_validity` in particular is true by construction given
// the enum, but the *other* invariants (denominator = 1 for pk_unitary,
// etc.) still constrain the relationship between `type_` and
// `numerator`/`denominator`'s runtime values, which no enum alone can
// enforce.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_proportion.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_proportion.adoc §DV_PROPORTION Class
//   confidence: low
//   todos: 6
//   note: multiple inheritance (PROPORTION_KIND + DV_AMOUNT) handled per ADR-001 §2 — PROPORTION_KIND is a constants-only class reached via direct ProportionKind::* calls, not a supertrait; type_: ProportionKind is a closed-enum judgment call over the spec's literal Integer typing; add/subtract/multiply/negative all stubbed todo!() since ratio-combination rules for arithmetic are unstated at any level; is_equal's own Meaning cell literally says "DV_AMOUNT" not "DV_PROPORTION" (flagged, transcribed verbatim); the seven invariants recorded but not enforced; forward-references CODE_PHRASE pending sibling data_types::text package.
// ─────────────────────────────────────────────
