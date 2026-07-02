//! `DV_QUANTITY` — quantitified type representing scientific quantities.
//!
//! openEHR class: `DV_QUANTITY`, package `rm.data_types.quantity`.
//! Inherits: `DV_AMOUNT`.
//!
//! Quantitified type representing scientific quantities, i.e. quantities
//! expressed as a magnitude and units. Units are expressed in the UCUM
//! syntax (Unified Code for Units of Measure, by Gunther Schadow and
//! Clement J. McDonald of The Regenstrief Institute) (case-sensitive form)
//! by default, or another system if `units_system` is set.
//!
//! Can also be used for time durations, where it is more convenient to
//! treat these as simply a number of seconds rather than days, months,
//! years (in the latter case, `DV_DURATION` may be used).
use super::dv_amount::{DvAmountApi, DvAmountData, UNKNOWN_ACCURACY_VALUE};
use super::dv_ordered::DvOrderedApi;
use super::dv_quantified::DvQuantifiedApi;
// TODO(port): forward-references CODE_PHRASE (rm.data_types.text), not yet
// transcribed by the sibling package agent covering `data_types::text`.
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::integer::Integer;
use openehr_foundation::primitive_types::numeric::Numeric;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::primitive_types::real::Real;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "DV_QUANTITY";

/// `DV_QUANTITY` inherits `DV_AMOUNT` and adds five attributes of its own
/// (`magnitude`, `precision`, `units`, `units_system`,
/// `units_display_name`). Its own table also re-lists `normal_range` and
/// `other_reference_ranges` with a `(redefined)` marker, narrowing their
/// generic parameter from the unparameterized `DV_ORDERED`-level form to
/// `DV_QUANTITY` specifically — this narrowing is **already fully captured**
/// by the F-bounded instantiation `DvOrderedData<DvQuantity>` embedded
/// (transitively, via `amount.quantified.ordered`) below: once the generic
/// parameter `T` resolves to `DvQuantity`, `normal_range`'s declared type
/// there is already `Option<Box<DvInterval<DvQuantity>>>`, the exact
/// narrowed type the `(redefined)` row calls for. No separate flat
/// duplicate field is declared on this struct — doing so would create two
/// copies of the same conceptual attribute with no single source of truth,
/// which the `LOCATABLE_REF.id` worked example (ADR-001 §6,
/// `openehr_base::identification::locatable_ref`) avoids by *not* also
/// embedding the wider parent type; here the equivalent avoidance is
/// achieved by the generic already being self-typed rather than by
/// flattening. `self.amount.quantified.ordered.normal_range` /
/// `.other_reference_ranges` are the sole accessors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvQuantity {
    /// Canonical `_type` discriminator (`"DV_QUANTITY"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_AMOUNT` parent state, self-typed per the F-bounded
    /// pattern documented on `DvOrderedData` in `dv_ordered.rs` (threaded
    /// through `DvQuantifiedData<T>`/`DvAmountData<T>`).
    #[serde(flatten)]
    pub amount: DvAmountData<DvQuantity>,

    /// `magnitude`: `Real` (1..1).
    ///
    /// Numeric magnitude of the quantity.
    ///
    /// PORT NOTE: the previously-flagged cross-crate gap is closed — `Real`
    /// now derives `Serialize`/`Deserialize` in `openehr-foundation`,
    /// serializing as its bare inner `f64`; the round-trip test at the
    /// bottom of this file asserts the full canonical wire shape.
    pub magnitude: Real,

    /// `precision`: `Integer` (0..1).
    ///
    /// Precision to which the value of the quantity is expressed, in terms
    /// of number of decimal places. The value 0 implies an integral
    /// quantity. The value -1 implies no limit, i.e. any number of decimal
    /// places.
    ///
    /// PORT NOTE: `Integer`'s previously-flagged serde gap is closed the
    /// same way as `magnitude`'s `Real` above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<Integer>,

    /// `units`: `String` (1..1).
    ///
    /// Quantity units, expressed as a code or syntax string from either
    /// UCUM (the default) or the units system specified in
    /// `units_system`, when set.
    ///
    /// In either case, the value is the code or syntax — normally formed
    /// of standard ASCII — which is in principal not the same as the
    /// display string, although in simple cases such as 'm' (for meters)
    /// it will be.
    ///
    /// If the `units_display_name` field is set, this may be used for
    /// display. If not, the implementations must effect the resolution of
    /// the `units` value to a display form locally, e.g. by lookup of
    /// reference tables, request to a terminology service etc.
    ///
    /// Example values from UCUM: "kg/m^2", "mm[Hg]", "ms-1", "km/h".
    pub units: String,

    /// `units_system`: `String` (0..1).
    ///
    /// Optional field used to specify a units system from which codes in
    /// `units` are defined. Value is a URI identifying a terminology
    /// containing units concepts from the HL7 FHIR terminologies list.
    ///
    /// If not set, the UCUM standard (case-sensitive codes) is assumed as
    /// the units system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units_system: Option<String>,

    /// `units_display_name`: `String` (0..1).
    ///
    /// Optional field containing the displayable form of the `units`
    /// field, e.g. `'°C'`.
    ///
    /// If not set, the application environment needs to determine the
    /// displayable form.
    ///
    /// Note: the display name may be language-dependent for various older
    /// and non-systematic units. For this reason, it is not recommended to
    /// add unit display names to archetypes, only to templates (for
    /// localisation purposes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units_display_name: Option<String>,
}

/// ADR-002: `_type` string for `DV_QUANTITY`, single-sourced from
/// [`TYPE_NAME`].
impl TypeName for DvQuantity {
    const NAME: &'static str = TYPE_NAME;
}

impl DvQuantity {
    /// `normal_status`: accessor to the embedded parent state's attribute.
    pub fn normal_status(&self) -> Option<&CodePhrase> {
        self.amount.quantified.ordered.normal_status.as_ref()
    }

    /// `normal_range`: `DV_INTERVAL<DV_QUANTITY>` (0..1, redefined).
    ///
    /// Accessor into the embedded, self-typed `DvOrderedData<DvQuantity>`
    /// — see the struct-level doc comment for why no separate flat field
    /// is declared for this covariantly-redefined attribute.
    pub fn normal_range(&self) -> Option<&super::dv_interval::DvInterval<DvQuantity>> {
        self.amount.quantified.ordered.normal_range.as_deref()
    }

    /// `other_reference_ranges`: `List<REFERENCE_RANGE<DV_QUANTITY>>` (0..1,
    /// redefined).
    ///
    /// Accessor into the embedded, self-typed `DvOrderedData<DvQuantity>`.
    pub fn other_reference_ranges(
        &self,
    ) -> Option<&[super::reference_range::ReferenceRange<DvQuantity>]> {
        self.amount
            .quantified
            .ordered
            .other_reference_ranges
            .as_deref()
    }

    /// `is_integral(): Boolean`.
    ///
    /// True if `precision` = 0, meaning that the `magnitude` is a whole
    /// number.
    pub fn is_integral(&self) -> bool {
        matches!(self.precision, Some(Integer(0)))
    }
}

impl Any for DvQuantity {
    /// `is_equal(other: DV_QUANTITY) -> Boolean`.
    ///
    /// PORT NOTE: `DV_QUANTITY`'s own table gives no explicit `is_equal`
    /// row (it inherits `DV_AMOUNT.is_equal` unchanged); this default body
    /// compares every declared attribute directly as the most literal
    /// reading, mirroring `DvOrdinal`/`DvScale`'s identical situation.
    fn is_equal(&self, other: &Self) -> bool {
        self.magnitude.is_equal(&other.magnitude)
            && self.precision == other.precision
            && self.units == other.units
            && self.units_system == other.units_system
            && self.units_display_name == other.units_display_name
    }

    fn type_of(&self) -> String {
        "DvQuantity".to_string()
    }
}

impl Ordered for DvQuantity {
    /// `less_than` __alias__ `"<"` `(other: DV_QUANTITY) -> Boolean`
    /// (effected).
    ///
    /// True if this Quantified object is less than `other`. Based on
    /// comparison of `magnitude`. Only valid if
    /// `is_strictly_comparable_to()` is `True`.
    ///
    /// Spec `Post_result`: `Result = magnitude < other.magnitude`.
    fn less_than(&self, other: &Self) -> bool {
        self.magnitude.less_than(&other.magnitude)
    }
}

impl DvOrderedApi for DvQuantity {
    fn normal_status(&self) -> Option<&CodePhrase> {
        self.amount.quantified.ordered.normal_status.as_ref()
    }

    /// `is_strictly_comparable_to(other: DV_QUANTITY) -> Boolean`
    /// (effected).
    ///
    /// True if this quantity and `other` have the same `units` and also
    /// `units_system` if it exists.
    ///
    /// PORT NOTE: the spec types `other` as the abstract `DV_ORDERED`, but
    /// the comparison body itself (`units`/`units_system` equality) is only
    /// meaningful between two `DV_QUANTITY` instances; narrowed to `&Self`
    /// per the recurring pattern.
    fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        self.units == other.units && self.units_system == other.units_system
    }
}

impl DvQuantifiedApi<Real> for DvQuantity {
    fn magnitude_status(&self) -> Option<&str> {
        self.amount.quantified.magnitude_status.as_deref()
    }

    /// `magnitude(): Real` (effected) — the declared `magnitude` attribute
    /// doubles as the effected `DV_QUANTIFIED.magnitude()` accessor.
    fn magnitude(&self) -> Real {
        self.magnitude
    }

    /// `accuracy_unknown(): Boolean` (effected via `DV_AMOUNT`'s
    /// special-value convention: an `accuracy` of `unknown_accuracy_value`
    /// (-1) means accuracy was not recorded).
    ///
    /// PORT NOTE: an absent (`None`) accuracy is also treated as unknown —
    /// the spec models "not recorded" through the -1 sentinel on a 0..1
    /// attribute, so absence has no distinct stated semantics; flagged
    /// rather than silently chosen.
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

impl DvAmountApi<Real> for DvQuantity {
    fn accuracy_is_percent(&self) -> Option<bool> {
        self.amount.accuracy_is_percent
    }

    fn accuracy(&self) -> Option<Real> {
        self.amount.accuracy
    }

    /// `add` __alias__ `"+"` `(other: DV_QUANTITY) -> DV_QUANTITY`
    /// (redefined).
    ///
    /// Sum of this `DV_QUANTITY` and `other`.
    ///
    /// TODO(port): the spec gives no explicit `Post_result` body for this
    /// redefined effector at the `DV_QUANTITY` level (unlike, say,
    /// `less_than`'s explicit `Result = magnitude < other.magnitude`); the
    /// accuracy-combination rule inherited from `DV_AMOUNT.add`'s own
    /// description ("if accuracies are present in both quantities, they
    /// are added ... if either or both quantities has an unknown accuracy,
    /// the accuracy of the result is also unknown ... if only one has
    /// accuracy_is_percent = True, accuracy is expressed in the result in
    /// the form used in the larger of the two quantities") is nontrivial
    /// branching logic not yet encoded; stubbed pending that.
    fn add(&self, _other: &Self) -> Self {
        todo!(
            "DvQuantity::add: accuracy-combination rule from DV_AMOUNT's description not yet encoded"
        )
    }

    /// `subtract` __alias__ `"-"` `(other: DV_QUANTITY) -> DV_QUANTITY`
    /// (redefined).
    ///
    /// Difference of this `DV_QUANTITY` and `other`.
    ///
    /// TODO(port): same accuracy-combination gap as `add` above.
    fn subtract(&self, _other: &Self) -> Self {
        todo!(
            "DvQuantity::subtract: accuracy-combination rule from DV_AMOUNT's description not yet encoded"
        )
    }

    fn is_equal_amount(&self, other: &Self) -> bool {
        self.is_equal(other)
    }

    /// `multiply` __alias__ `"*"` `(factor: Real) -> DV_QUANTITY`
    /// (redefined).
    ///
    /// Product of this `DV_QUANTITY` and `factor`.
    ///
    /// TODO(port): no explicit `Post_result` body given; the natural
    /// reading is `magnitude * factor` with `units`/`units_system`/
    /// `units_display_name` carried over unchanged, but this is not itself
    /// drawn from a stated postcondition, so left `todo!()` rather than
    /// guessing the accuracy-scaling behaviour that should accompany a
    /// multiply.
    fn multiply(&self, _factor: &Real) -> Self {
        todo!(
            "DvQuantity::multiply: no explicit Post_result body, accuracy-scaling behaviour unspecified"
        )
    }

    /// `negative` __alias__ `"-"` `(): DV_QUANTITY`.
    ///
    /// PORT NOTE: `DV_QUANTITY`'s own table does not re-list `negative`
    /// with a `(redefined)` marker (unlike `add`/`subtract`/`multiply`,
    /// which are explicitly marked `(redefined)`); it is inherited from
    /// `DV_AMOUNT.negative` unchanged. Transcribed here anyway (rather than
    /// omitted) because the `DvAmountApi` trait requires it and no default
    /// body is provided at the trait level (see `dv_amount.rs`); the
    /// natural same-type negation is magnitude negation with units
    /// preserved.
    fn negative(&self) -> Self {
        DvQuantity {
            magnitude: self.magnitude.negative(),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_ordered::DvOrderedData;
    use crate::data_types::quantity::dv_quantified::DvQuantifiedData;

    /// Canonical-JSON round-trip test for `DvQuantity`: magnitude 7.5,
    /// units "kg", every other field `None`/default.
    ///
    /// PORT NOTE: this exercises the full flattened
    /// `DvOrderedData<T> -> DvQuantifiedData<T> -> DvAmountData<T> ->
    /// DvQuantity` chain and asserts the canonical wire shape (ADR-002
    /// `_type` self-tag first in key order, snake_case, null omission).
    #[test]
    fn quantity_round_trips_through_canonical_json() {
        let quantity = DvQuantity {
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
            magnitude: Real(7.5),
            precision: None,
            units: "kg".to_string(),
            units_system: None,
            units_display_name: None,
        };

        let json = serde_json::to_string(&quantity).expect("serialize");
        assert!(
            json.starts_with(r#"{"_type":"DV_QUANTITY","#),
            "canonical JSON must lead with the _type discriminator: {json}"
        );
        assert_eq!(
            json,
            r#"{"_type":"DV_QUANTITY","magnitude":7.5,"units":"kg"}"#
        );

        let back: DvQuantity = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, quantity);

        // A missing `_type` is tolerated in a concrete-declared slot
        // (ADR-002 / ITS-JSON), while a wrong one must be rejected.
        let untagged: DvQuantity = serde_json::from_str(r#"{"magnitude":7.5,"units":"kg"}"#)
            .expect("missing _type tolerated");
        assert_eq!(untagged, quantity);
        let wrong: Result<DvQuantity, _> =
            serde_json::from_str(r#"{"_type":"DV_COUNT","magnitude":7.5,"units":"kg"}"#);
        assert!(wrong.is_err(), "mismatched _type must be rejected");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_quantity.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_quantity.adoc §DV_QUANTITY Class
//   confidence: medium
//   todos: 4
//   note: normal_range/other_reference_ranges are duplicated (flat top-level fields plus the copies inside DvOrderedData<DvQuantity>) pending a resolution of which is canonical, flagged with a TODO; add/subtract/multiply are stubbed todo!() since DV_AMOUNT's accuracy-combination rule is prose, not a stated postcondition, and DV_QUANTITY's own table gives no Post_result for any of the three. P4/ADR-002: self-tags via TypeTag<Self> first field + TypeName reusing TYPE_NAME; `amount` flattened (schema-verified, full DvOrderedData/DvQuantifiedData/DvAmountData chain flattens); Real/Integer serde gaps closed in openehr-foundation; round-trip test pins _type-first wire shape, missing-tag tolerance, wrong-tag rejection.
// ─────────────────────────────────────────────
