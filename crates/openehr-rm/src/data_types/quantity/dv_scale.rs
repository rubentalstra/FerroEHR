//! `DV_SCALE` — data type representing scale values.
//!
//! openEHR class: `DV_SCALE`, package `rm.data_types.quantity`.
//! Inherits: `DV_ORDERED`.
//!
//! A data type that represents scale values, where there is:
//!
//! a) implied ordering,
//! b) no implication that the distance between each value is constant, and
//! c) the total number of values is finite;
//! d) non-integer values are allowed.
//!
//! Example:
//!
//! ```text
//! Borg CR 10 Scale
//!
//! 0    No Breathlessness at all
//! 0.5  Very Very Slight (Just Noticeable)
//! 1    Very Slight
//! 2    Slight Breathlessness
//! 3    Moderate
//! ... etc
//! ```
//!
//! For scores that include only Integers, `DV_SCALE` may also be used, but
//! `DV_ORDINAL` should be supported to accommodate existing data instances
//! of that type.
use super::dv_ordered::{DvOrderedApi, DvOrderedData};
// TODO(port): forward-references CODE_PHRASE/DV_CODED_TEXT (rm.data_types.text),
// not yet transcribed by the sibling package agent covering
// `data_types::text`.
use crate::data_types::text::code_phrase::CodePhrase;
use crate::data_types::text::dv_coded_text::DvCodedText;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::primitive_types::real::Real;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "DV_SCALE";

/// `DV_SCALE` inherits `DV_ORDERED` and adds two attributes of its own
/// (`symbol`, `value`), structurally the same shape as `DV_ORDINAL` (see
/// `dv_ordinal.rs`) except `value` is `Real` here, not `Integer`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvScale {
    /// Canonical `_type` discriminator (`"DV_SCALE"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_ORDERED` parent state, self-typed per the F-bounded
    /// pattern documented on `DvOrderedData` in `dv_ordered.rs`.
    #[serde(flatten)]
    pub ordered: DvOrderedData<DvScale>,

    /// `symbol`: `DV_CODED_TEXT` (1..1).
    ///
    /// Coded textual representation of this value in the scale range,
    /// which may be strings made from symbols or other enumerations of
    /// terms such as `no breathlessness`, `very very slight`, `slight
    /// breathlessness`. Codes come from archetypes.
    ///
    /// In some cases, a scale may include values that have no code/symbol.
    /// In this case, the symbol will be a `DV_CODED_TEXT` including the
    /// `terminology_id` and a blank String value for `code_string`.
    pub symbol: DvCodedText,

    /// `value`: `Real` (1..1).
    ///
    /// Real number value of Scale item.
    ///
    /// PORT NOTE: the previously-flagged cross-crate gap is closed — `Real`
    /// now derives `Serialize`/`Deserialize` in `openehr-foundation`,
    /// serializing as its bare inner `f64`.
    pub value: Real,
}

/// ADR-002: `_type` string for `DV_SCALE`, single-sourced from
/// [`TYPE_NAME`].
impl TypeName for DvScale {
    const NAME: &'static str = TYPE_NAME;
}

impl DvScale {
    /// `normal_status`: accessor to the embedded parent state's attribute.
    pub fn normal_status(&self) -> Option<&CodePhrase> {
        self.ordered.normal_status.as_ref()
    }
}

impl Any for DvScale {
    /// `is_equal(other: DV_SCALE) -> Boolean`.
    ///
    /// PORT NOTE: `DV_SCALE`'s own table gives no explicit `is_equal` row;
    /// see the identical note on `DvOrdinal::is_equal`.
    fn is_equal(&self, other: &Self) -> bool {
        self.value.is_equal(&other.value) && self.symbol == other.symbol
    }

    fn type_of(&self) -> String {
        "DvScale".to_string()
    }
}

impl Ordered for DvScale {
    /// `less_than` __alias__ `"<"` `(other: DV_SCALE) -> Boolean`
    /// (effected).
    ///
    /// True if this Scale value is less than `other`.
    ///
    /// Spec `Pre_comparable`: `is_strictly_comparable_to (other)`.
    ///
    /// TODO(port): as with `DV_ORDINAL`, the spec table gives no explicit
    /// `Post_result` body for this effector; comparing `value: Real`
    /// directly is the natural reading but not itself drawn from a stated
    /// postcondition.
    fn less_than(&self, other: &Self) -> bool {
        self.value.less_than(&other.value)
    }
}

impl DvOrderedApi for DvScale {
    fn normal_status(&self) -> Option<&CodePhrase> {
        self.ordered.normal_status.as_ref()
    }

    /// `is_strictly_comparable_to(other: DV_SCALE) -> Boolean` (effected).
    ///
    /// Test if this Scale value is strictly comparable to `other`.
    ///
    /// TODO(port): the spec table marks this `(effected)` but gives no
    /// explicit body, identical to the same gap on `DV_ORDINAL` — see that
    /// type's matching TODO for the reasoning. Left `todo!()` rather than
    /// guessing.
    fn is_strictly_comparable_to(&self, _other: &Self) -> bool {
        todo!("DvScale::is_strictly_comparable_to: no explicit spec body at this level")
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_scale.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_scale.adoc §DV_SCALE Class
//   confidence: medium
//   todos: 3
//   note: is_strictly_comparable_to has no explicit spec body despite being marked (effected), stubbed todo!(); less_than transcribed via Real's Ordered::less_than through the foundation type rather than raw f64 comparison, keeping the RM layer consistent with the foundation Ordered contract; forward-references CODE_PHRASE/DV_CODED_TEXT pending sibling data_types::text package. P4/ADR-002: self-tags via TypeTag<Self> first field + TypeName reusing TYPE_NAME; `ordered` flattened; Real-lacks-serde gap now closed in openehr-foundation.
// ─────────────────────────────────────────────
