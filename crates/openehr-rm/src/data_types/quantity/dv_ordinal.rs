//! `DV_ORDINAL` — data type representing integral score values.
//!
//! openEHR class: `DV_ORDINAL`, package `rm.data_types.quantity`.
//! Inherits: `DV_ORDERED`.
//!
//! A data type that represents integral score values, e.g. pain, Apgar
//! values, etc, where there is:
//!
//! a) implied ordering,
//! b) no implication that the distance between each value is constant, and
//! c) the total number of values is finite;
//! d) integer values only.
//!
//! Note that although the term 'ordinal' in mathematics means natural
//! numbers only, here any integer is allowed, since negative and zero
//! values are often used by medical professionals for values around a
//! neutral point. Examples of sets of ordinal values:
//!
//! * -3, -2, -1, 0, 1, 2, 3  -- reflex response values
//! *  0, 1, 2                -- Apgar values
//!
//! This class is used for recording any clinical datum which is customarily
//! recorded using symbolic values. Example: the results on a urinalysis
//! strip, e.g. `{neg, trace, +, ++, +++}` are used for leucocytes, protein,
//! nitrites etc; for non-haemolysed blood `{neg, trace, moderate}`; for
//! haemolysed blood `{small, moderate, large}`.
//!
//! For scores or scales that include Real numbers (or might in the future,
//! i.e. not fixed for all time, such as Apgar), use `DV_SCALE`. `DV_SCALE`
//! may also be used in future for representing purely Integer-based scales,
//! however, the `DV_ORDINAL` type should continue to be supported in
//! software implementations in order to accommodate existing data that are
//! instances of this type.
use super::dv_ordered::{DvOrderedApi, DvOrderedData};
// TODO(port): forward-references CODE_PHRASE/DV_CODED_TEXT (rm.data_types.text),
// not yet transcribed by the sibling package agent covering
// `data_types::text`.
use crate::data_types::text::code_phrase::CodePhrase;
use crate::data_types::text::dv_coded_text::DvCodedText;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::integer::Integer;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "DV_ORDINAL";

/// `DV_ORDINAL` inherits `DV_ORDERED` and adds two attributes of its own
/// (`symbol`, `value`). Per ADR-001 §3, the parent's shared state is
/// embedded as `ordered: DvOrderedData<Self>` (the F-bounded self-typed
/// instantiation described in `dv_ordered.rs`) rather than duplicated flat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvOrdinal {
    /// Canonical `_type` discriminator (`"DV_ORDINAL"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_ORDERED` parent state, self-typed per the F-bounded
    /// pattern documented on `DvOrderedData` in `dv_ordered.rs`.
    ///
    /// `DV_ORDINAL`'s own table shows no `(redefined)` row for
    /// `normal_range`/`other_reference_ranges`, but the F-bounded
    /// instantiation resolves identically whether or not a `(redefined)`
    /// row is present, since `Self` already names the concrete type.
    #[serde(flatten)]
    pub ordered: DvOrderedData<DvOrdinal>,

    /// `symbol`: `DV_CODED_TEXT` (1..1).
    ///
    /// Coded textual representation of this value in the enumeration,
    /// which may be strings made from `+` symbols, or other enumerations of
    /// terms such as `mild`, `moderate`, `severe`, or even the same number
    /// series as the values, e.g. 1, 2, 3.
    pub symbol: DvCodedText,

    /// `value`: `Integer` (1..1).
    ///
    /// Value in ordered enumeration of values. Any integer value can be
    /// used.
    ///
    /// PORT NOTE: the previously-flagged cross-crate gap is closed —
    /// `Integer` now derives `Serialize`/`Deserialize` in
    /// `openehr-foundation`, serializing as its bare inner `i32`.
    pub value: Integer,
}

/// ADR-002: `_type` string for `DV_ORDINAL`, single-sourced from
/// [`TYPE_NAME`].
impl TypeName for DvOrdinal {
    const NAME: &'static str = TYPE_NAME;
}

impl DvOrdinal {
    /// `normal_status`: accessor to the embedded parent state's attribute.
    pub fn normal_status(&self) -> Option<&CodePhrase> {
        self.ordered.normal_status.as_ref()
    }
}

impl Any for DvOrdinal {
    /// `is_equal(other: DV_ORDINAL) -> Boolean`.
    ///
    /// PORT NOTE: `DV_ORDINAL`'s own table gives no explicit `is_equal`
    /// row (unlike `DV_QUANTIFIED`/`DV_AMOUNT`, which do); this default
    /// value-equality body compares both declared attributes directly as
    /// the most literal reading of `Any`'s inherited contract, since the
    /// class gives no narrower definition of its own.
    fn is_equal(&self, other: &Self) -> bool {
        self.value.is_equal(&other.value) && self.symbol == other.symbol
    }

    fn type_of(&self) -> String {
        "DvOrdinal".to_string()
    }
}

impl Ordered for DvOrdinal {
    /// `less_than` __alias__ `"<"` `(other: DV_ORDINAL) -> Boolean`
    /// (effected).
    ///
    /// True if this Ordinal value is less than `other`.
    ///
    /// Spec `Pre_comparable`: `is_strictly_comparable_to (other)`.
    ///
    /// TODO(port): the spec gives no explicit `Post_result` body for this
    /// effector (unlike, say, `DV_QUANTIFIED.less_than`'s `Result =
    /// magnitude < other.magnitude`); comparing `value: Integer` directly
    /// is the natural reading given `DV_ORDINAL`'s own description ("any
    /// integer value can be used" as "Value in ordered enumeration"), but
    /// is not itself drawn from an explicit postcondition in the table.
    fn less_than(&self, other: &Self) -> bool {
        self.value.0 < other.value.0
    }
}

impl DvOrderedApi for DvOrdinal {
    fn normal_status(&self) -> Option<&CodePhrase> {
        self.ordered.normal_status.as_ref()
    }

    /// `is_strictly_comparable_to(other: DV_ORDINAL) -> Boolean` (effected).
    ///
    /// Test if this Ordinal is strictly comparable to `other`.
    ///
    /// TODO(port): the spec gives no explicit body for this effector at the
    /// `DV_ORDINAL` level (contrast `DV_QUANTITY`'s explicit "same units and
    /// units_system" rule, or `DV_COUNT`'s explicit "Return True"). The
    /// class description text ("Each symbol can be assigned any Integer
    /// value, providing a basis for computable comparison") suggests
    /// comparability may depend on the `symbol`'s terminology/coding system
    /// matching, but this is not stated as a postcondition. Left `todo!()`
    /// rather than guessing.
    fn is_strictly_comparable_to(&self, _other: &Self) -> bool {
        todo!("DvOrdinal::is_strictly_comparable_to: no explicit spec body at this level")
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_ordinal.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_ordinal.adoc §DV_ORDINAL Class
//   confidence: medium
//   todos: 3
//   note: is_strictly_comparable_to has no explicit spec body at this level (unlike DV_QUANTITY/DV_COUNT), stubbed todo!() rather than guessed; less_than compares value: Integer directly as the natural reading, though not itself drawn from an explicit Post_result; forward-references CODE_PHRASE/DV_CODED_TEXT pending sibling data_types::text package. P4/ADR-002: self-tags via TypeTag<Self> first field + TypeName reusing TYPE_NAME; `ordered` flattened (schema-verified — normal_status/normal_range/other_reference_ranges sit flat alongside value/symbol); Integer-lacks-serde gap now closed in openehr-foundation.
// ─────────────────────────────────────────────
