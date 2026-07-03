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
use super::dv_amount::{DvAmountApi, DvAmountData, UNKNOWN_ACCURACY_VALUE};
use super::dv_ordered::{DvOrderedApi, DvOrderedData};
use super::dv_quantified::DvQuantifiedApi;
use super::proportion_kind::ProportionKind;
// TODO(port): forward-references CODE_PHRASE (rm.data_types.text), not yet
// transcribed by the sibling package agent covering `data_types::text`.
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::integer::Integer;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::primitive_types::real::Real;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into the [`TypeName`] impl below (ADR-002).
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvProportion {
    /// Canonical `_type` discriminator (`"DV_PROPORTION"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_AMOUNT` parent state, self-typed per the F-bounded
    /// pattern documented on `DvOrderedData` in `dv_ordered.rs` (threaded
    /// through `DvQuantifiedData<T>`/`DvAmountData<T>`).
    #[serde(flatten)]
    pub amount: DvAmountData<DvProportion>,

    /// `numerator`: `Real` (1..1).
    ///
    /// Numerator of ratio.
    ///
    /// PORT NOTE: the previously-flagged cross-crate gap is closed — `Real`
    /// now derives `Serialize`/`Deserialize` in `openehr-foundation`,
    /// serializing as its bare inner `f64`.
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
    ///
    /// Now carries `#[serde(rename = "type")]` for the Rust keyword
    /// collision (this one is functional — it is a field-level rename, not
    /// the struct-level container rename shown inert on `DvBoolean`); the
    /// value itself round-trips through `ProportionKind`'s own
    /// `#[serde(into = "i32", try_from = "i32")]` (see `proportion_kind.rs`).
    #[serde(rename = "type")]
    pub type_: ProportionKind,

    /// `precision`: `Integer` (0..1).
    ///
    /// Precision to which the `numerator` and `denominator` values of the
    /// proportion are expressed, in terms of number of decimal places. The
    /// value 0 implies an integral quantity. The value -1 implies no
    /// limit, i.e. any number of decimal places.
    ///
    /// Invariant `Precision_validity`: `precision = 0 implies is_integral`
    /// (enforced by [`DvProportion::invariant_precision_validity`]).
    ///
    /// PORT NOTE: the previously-flagged `openehr-foundation`-lacks-serde gap
    /// for `Integer` is closed the same way as `numerator`/`denominator`'s
    /// `Real` above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<Integer>,
}

impl TypeName for DvProportion {
    const NAME: &'static str = TYPE_NAME;
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
    ///
    /// PORT NOTE: this class's table declares `magnitude(): Real`, but the
    /// spec-accurate `Real::divide` effector it delegates to returns
    /// `Double` (`Real.divide`'s own row narrows the result type) — the
    /// published tables disagree across the two classes; the `Double`
    /// result is converted back to the declared `Real` return type
    /// explicitly here (both are `f64`-backed per ADR-001 §7).
    pub fn magnitude(&self) -> Real {
        Real(self.numerator.divide(&self.denominator).0)
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

    fn ordered_data(&self) -> Option<&DvOrderedData<Self>> {
        Some(&self.amount.quantified.ordered)
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

impl DvQuantifiedApi<Real> for DvProportion {
    fn magnitude_status(&self) -> Option<&str> {
        self.amount.quantified.magnitude_status.as_deref()
    }

    /// Delegates to the inherent [`DvProportion::magnitude`] (the effected
    /// `numerator/denominator` division).
    fn magnitude(&self) -> Real {
        DvProportion::magnitude(self)
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
    /// TODO(port): published-spec defect — the table prints no `Post_result`,
    /// and there is no canonical result *representation* for a ratio sum. A
    /// magnitude-preserving rule exists (common-denominator addition:
    /// `a/b + c/d = (ad+cb)/(bd)`, or numerator addition when denominators
    /// match), but for `pk_fraction`/`pk_integer_fraction` the choice of
    /// result denominator is underdetermined, and the reference behaviour is
    /// not stated. Left `todo!()` rather than committing to one
    /// representation; revisit against reference behaviour at P17/P18.
    fn add(&self, _other: &Self) -> Self {
        todo!(
            "DvProportion::add: published-spec defect — no Post_result and no canonical result representation for a ratio sum (P17/P18)"
        )
    }

    /// `subtract` __alias__ `"-"` `(other: DV_PROPORTION) -> DV_PROPORTION`
    /// (redefined).
    ///
    /// Difference between two strictly comparable proportions.
    ///
    /// TODO(port): same published-spec-defect ratio-representation gap as
    /// `add` above (P17/P18).
    fn subtract(&self, _other: &Self) -> Self {
        todo!(
            "DvProportion::subtract: published-spec defect — no Post_result and no canonical result representation for a ratio difference (P17/P18)"
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
    /// TODO(port): published-spec defect — the table prints no `Post_result`,
    /// and scaling the numerator by a `Real` `factor`
    /// (`(numerator*factor)/denominator`, the only choice that preserves the
    /// denominator-dependent `Unitary`/`Percent` validity, cf. `negative`)
    /// breaks `Is_integral_validity`/`Fraction_validity` for the
    /// `pk_fraction`/`pk_integer_fraction` kinds (a `Real` factor generally
    /// makes the numerator non-integral). No result is both spec-faithful and
    /// invariant-preserving across all five kinds, so this stays `todo!()`;
    /// revisit against reference behaviour at P17/P18.
    fn multiply(&self, _factor: &Real) -> Self {
        todo!(
            "DvProportion::multiply: published-spec defect — a Real factor breaks integral-numerator validity for fraction kinds, no invariant-preserving body across all five kinds (P17/P18)"
        )
    }

    /// `negative` __alias__ `"-"` `(): DV_PROPORTION`.
    ///
    /// Negated version of this proportion: the `numerator` is negated and the
    /// `denominator` (and hence `type_`, `precision`) are carried over
    /// unchanged, giving `magnitude() == -original.magnitude()`.
    ///
    /// PORT NOTE: `DV_PROPORTION`'s own table does not list `negative` (it is
    /// inherited from `DV_AMOUNT.negative`, and `DvAmountApi` requires a body
    /// with no default). Negating a ratio's *numerator* versus its
    /// *denominator* both satisfy `magnitude() == -original.magnitude()`, but
    /// only numerator-negation preserves the denominator-dependent
    /// type-validity invariants — `Unitary_validity` (denominator = 1),
    /// `Percent_validity` (denominator = 100), and `Is_integral_validity`
    /// (both integral: `-n` stays integral) all remain satisfied, whereas
    /// negating the denominator would break the first two. This breaks the
    /// tie the earlier `todo!()` flagged: numerator-negation is the unique
    /// type-safe representation, so it is implemented rather than left
    /// unfinished.
    fn negative(&self) -> Self {
        DvProportion {
            numerator: Real(-self.numerator.0),
            ..self.clone()
        }
    }
}

/// The seven `DV_PROPORTION` class invariants, as working `invariant_*`
/// methods per ADR-003 decision 8 (invariants become `is_valid()`-family
/// methods now; the walker/accumulator `Validate` framework remains the P11
/// deliverable).
///
/// PORT NOTE: `Type_validity`, and the type-kind-dependent invariants, are
/// partially structural once `type_: ProportionKind` is a closed enum rather
/// than a bare `Integer` (see `type_`'s own doc comment) — `Type_validity` in
/// particular is true by construction — but the value-relationship invariants
/// (`denominator = 1` for `pk_unitary`, `denominator = 100` for `pk_percent`,
/// integrality for the fraction kinds, `denominator /= 0`) constrain the
/// runtime relationship between `type_` and `numerator`/`denominator`, which
/// no enum alone can enforce, so each is transcribed literally.
impl DvProportion {
    /// `Type_validity`: `valid_proportion_kind (type)`.
    ///
    /// True by construction here (any [`ProportionKind`] value is valid), but
    /// transcribed literally through the spec's own
    /// [`ProportionKind::valid_proportion_kind`] against the enum's `i32`
    /// discriminant.
    pub fn invariant_type_validity(&self) -> bool {
        ProportionKind::valid_proportion_kind(i32::from(self.type_))
    }

    /// `Precision_validity`: `precision = 0 implies is_integral`.
    pub fn invariant_precision_validity(&self) -> bool {
        !matches!(self.precision, Some(Integer(0))) || self.is_integral()
    }

    /// `Is_integral_validity`: `is_integral implies (numerator.floor =
    /// numerator and denominator.floor = denominator)`.
    ///
    /// PORT NOTE: the spec's `numerator.floor = numerator` is transcribed as
    /// "the value has no fractional part" (`fract() == 0.0` on the backing
    /// `f64`), equivalent for finite values and avoiding a float-to-float
    /// `== floor()` comparison.
    pub fn invariant_is_integral_validity(&self) -> bool {
        !self.is_integral()
            || (self.numerator.0.fract() == 0.0 && self.denominator.0.fract() == 0.0)
    }

    /// `Fraction_validity`: `(type = pk_fraction or type =
    /// pk_integer_fraction) implies is_integral`.
    pub fn invariant_fraction_validity(&self) -> bool {
        !matches!(
            self.type_,
            ProportionKind::Fraction | ProportionKind::IntegerFraction
        ) || self.is_integral()
    }

    /// `Unitary_validity`: `type = pk_unitary implies denominator = 1`.
    ///
    /// PORT NOTE: the exact `denominator = 1` comparison is spec-intended
    /// (unitary proportions carry an exact integer denominator); compared via
    /// the `Real` newtype so it reads as an exact value check rather than a
    /// raw-`f64` literal comparison.
    pub fn invariant_unitary_validity(&self) -> bool {
        self.type_ != ProportionKind::Unitary || self.denominator == Real(1.0)
    }

    /// `Percent_validity`: `type = pk_percent implies denominator = 100`.
    ///
    /// PORT NOTE: exact `denominator = 100` comparison via the `Real`
    /// newtype, as for `invariant_unitary_validity`.
    pub fn invariant_percent_validity(&self) -> bool {
        self.type_ != ProportionKind::Percent || self.denominator == Real(100.0)
    }

    /// `Valid_denominator`: `denominator /= 0.0`.
    pub fn invariant_valid_denominator(&self) -> bool {
        self.denominator.0 != 0.0
    }

    /// All seven class invariants combined, as a single validity check per
    /// ADR-003 decision 8.
    pub fn is_valid(&self) -> bool {
        self.invariant_type_validity()
            && self.invariant_precision_validity()
            && self.invariant_is_integral_validity()
            && self.invariant_fraction_validity()
            && self.invariant_unitary_validity()
            && self.invariant_percent_validity()
            && self.invariant_valid_denominator()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_quantified::DvQuantifiedData;

    fn proportion(
        numerator: f64,
        denominator: f64,
        kind: ProportionKind,
        precision: Option<i32>,
    ) -> DvProportion {
        DvProportion {
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
            numerator: Real(numerator),
            denominator: Real(denominator),
            type_: kind,
            precision: precision.map(Integer),
        }
    }

    /// Spec: "a magnitude function which is computed as the result of the
    /// numerator/denominator division".
    #[test]
    fn magnitude_is_numerator_over_denominator() {
        assert_eq!(
            proportion(30.0, 100.0, ProportionKind::Percent, None).magnitude(),
            Real(0.3)
        );
        assert_eq!(
            proportion(1.0, 2.0, ProportionKind::Fraction, Some(0)).magnitude(),
            Real(0.5)
        );
    }

    /// Spec `is_integral`: "True if ... precision is 0."
    #[test]
    fn is_integral_reflects_precision_zero() {
        assert!(proportion(1.0, 2.0, ProportionKind::Fraction, Some(0)).is_integral());
        assert!(!proportion(1.5, 2.0, ProportionKind::Ratio, Some(2)).is_integral());
        assert!(!proportion(1.0, 2.0, ProportionKind::Ratio, None).is_integral());
    }

    /// Spec `is_strictly_comparable_to`: "Return True if the type of this
    /// proportion is the same as the type of other."
    #[test]
    fn strictly_comparable_when_types_match() {
        let a = proportion(30.0, 100.0, ProportionKind::Percent, None);
        let b = proportion(40.0, 100.0, ProportionKind::Percent, None);
        assert!(a.is_strictly_comparable_to(&b));
        let ratio = proportion(1.0, 128.0, ProportionKind::Ratio, None);
        assert!(!a.is_strictly_comparable_to(&ratio));
    }

    /// Spec `Percent_validity`: `type = pk_percent implies denominator = 100`.
    #[test]
    fn percent_validity_invariant() {
        assert!(
            proportion(30.0, 100.0, ProportionKind::Percent, None).invariant_percent_validity()
        );
        assert!(
            !proportion(30.0, 50.0, ProportionKind::Percent, None).invariant_percent_validity()
        );
        // Non-percent kinds are unconstrained by this invariant.
        assert!(proportion(1.0, 50.0, ProportionKind::Ratio, None).invariant_percent_validity());
    }

    /// Spec `Unitary_validity`: `type = pk_unitary implies denominator = 1`.
    #[test]
    fn unitary_validity_invariant() {
        assert!(proportion(5.0, 1.0, ProportionKind::Unitary, None).invariant_unitary_validity());
        assert!(!proportion(5.0, 2.0, ProportionKind::Unitary, None).invariant_unitary_validity());
    }

    /// Spec `Fraction_validity`: `(pk_fraction or pk_integer_fraction)
    /// implies is_integral`.
    #[test]
    fn fraction_validity_invariant() {
        assert!(
            proportion(1.0, 2.0, ProportionKind::Fraction, Some(0)).invariant_fraction_validity()
        );
        // Fraction kind but precision != 0 (not integral): violated.
        assert!(
            !proportion(1.0, 2.0, ProportionKind::Fraction, Some(2)).invariant_fraction_validity()
        );
    }

    /// Spec `Valid_denominator`: `denominator /= 0.0`.
    #[test]
    fn valid_denominator_invariant() {
        assert!(proportion(1.0, 2.0, ProportionKind::Ratio, None).invariant_valid_denominator());
        assert!(!proportion(1.0, 0.0, ProportionKind::Ratio, None).invariant_valid_denominator());
    }

    /// `is_valid` combines all seven; a well-formed percentage passes and a
    /// bad-denominator percentage fails.
    #[test]
    fn is_valid_combines_all_invariants() {
        assert!(proportion(30.0, 100.0, ProportionKind::Percent, None).is_valid());
        assert!(!proportion(30.0, 50.0, ProportionKind::Percent, None).is_valid());
    }

    /// `negative` negates the numerator, preserving the denominator (and thus
    /// `type_`/`precision`), so `magnitude()` flips sign and type validity is
    /// preserved.
    #[test]
    fn negative_negates_numerator_and_preserves_type_validity() {
        let percent = proportion(30.0, 100.0, ProportionKind::Percent, None);
        let neg = percent.negative();
        assert_eq!(neg.numerator, Real(-30.0));
        assert_eq!(neg.denominator, Real(100.0));
        assert_eq!(neg.type_, ProportionKind::Percent);
        assert_eq!(neg.magnitude(), Real(-0.3));
        // Denominator-dependent type validity survives negation.
        assert!(neg.invariant_percent_validity());
    }

    /// `is_simple` routes through the overridden `ordered_data()`.
    #[test]
    fn is_simple_reflects_the_embedded_ordered_state() {
        assert!(proportion(1.0, 2.0, ProportionKind::Ratio, None).is_simple());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_proportion.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_proportion.adoc §DV_PROPORTION Class
//   confidence: medium
//   todos: 4
//   note: multiple inheritance (PROPORTION_KIND + DV_AMOUNT) handled per ADR-001 §2 — PROPORTION_KIND is a constants-only class reached via direct ProportionKind::* calls, not a supertrait; type_: ProportionKind is a closed-enum judgment call over the spec's literal Integer typing. magnitude()/is_integral()/is_strictly_comparable_to implemented per table; all seven class invariants now working invariant_* methods (+ is_valid) per ADR-003 §8, unit-tested. negative() implemented via numerator negation (the unique type-validity-preserving representation — cf. its PORT NOTE). add/subtract/multiply kept as genuine published-spec-defect todo!()s (no Post_result and no canonical/invariant-preserving result representation for ratio arithmetic — P17/P18). is_equal's own Meaning cell literally says "DV_AMOUNT" not "DV_PROPORTION" (flagged, transcribed verbatim). Remaining TODO: forward-reference CODE_PHRASE pending the sibling data_types::text package (present in-tree; reconciled at P17). P4: Serialize/Deserialize added; `amount` flattened; `type_` carries a functional #[serde(rename = "type")] and serializes via ProportionKind's own i32 encoding; Real/Integer serde gaps closed in openehr-foundation.
// ─────────────────────────────────────────────
