//! `ELEMENT` — the leaf variant of `ITEM`.
//!
//! openEHR class: `ELEMENT`, package `rm.data_structures.representation`.
//!
//! The leaf variant of `ITEM`, to which a `DATA_VALUE` instance is
//! attached.

use super::item::{ItemApi, ItemData};
// PORT NOTE: `DATA_VALUE` and `DV_CODED_TEXT`/`DV_TEXT` belong to the
// `rm.data_types` package, transcribed concurrently by a sibling agent, not
// this one. Forward-referenced per the invoking task's instruction; the
// exact module path assumes the standard "one directory per spec package"
// layout (`data_types::data_value`, `data_types::text::dv_coded_text`,
// `data_types::text::dv_text`) documented in PORT_MASTER_PLAN.md §9 and
// used as the convention for `common::archetyped::locatable` elsewhere in
// this file set; reconcile the exact path once `data_types` lands.
use crate::data_types::data_value::DataValue;
use crate::data_types::text::dv_coded_text::DvCodedText;
use crate::data_types::text::dv_text::DvText;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `ELEMENT` class.
///
/// Embeds the shared `ITEM` state (per ADR-001 §3) plus its own attributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    /// Canonical `_type` discriminator (`"ELEMENT"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `ITEM` (and transitively `LOCATABLE`) state.
    #[serde(flatten)]
    pub item: ItemData,

    /// `null_flavour`: flavour of null value, e.g. `253|unknown|`,
    /// `271|no information|`, `272|masked|`, and `273|not applicable|`.
    ///
    /// Cardinality `0..1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null_flavour: Option<DvCodedText>,

    /// `value`: property representing the leaf value object of `ELEMENT`.
    /// In real data, any concrete subtype of `DATA_VALUE` can be used.
    ///
    /// Cardinality `0..1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<DataValue>,

    /// `null_reason`: optional specific reason for null value; if set,
    /// `null_flavour` must be set. Null reason may apply only to a
    /// minority of clinical data, commonly needed in reporting contexts.
    ///
    /// Cardinality `0..1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null_reason: Option<DvText>,
}

impl TypeName for Element {
    const NAME: &'static str = TYPE_NAME;
}

impl ItemApi for Element {
    fn item_data(&self) -> &ItemData {
        &self.item
    }
}

impl Element {
    /// `is_null`: `True` if value logically not known, e.g. if
    /// indeterminate, not asked etc.
    ///
    /// Invariant `Inv_is_null_valid`: `is_null() = (value = Void)`. This
    /// invariant is exactly the function's own defining contract, so the
    /// implementation is definitional rather than a separately-checked
    /// invariant: `is_null()` returns `true` iff `value` is `None`.
    pub fn is_null(&self) -> bool {
        self.value.is_none()
    }

    // TODO(port): invariants below are recorded as doc text pending the RM
    // `Validate` trait framework (`.claude/rules/rm-transcription.md`
    // "Invariants"); none are enforced by a constructor yet.
    //
    // - `Inv_null_flavour_indicated`: `is_null() xor null_flavour = Void`.
    //   I.e. exactly one of {value is absent, null_flavour is present}
    //   holds — a value cannot be null without a null_flavour, and a
    //   non-null value cannot carry a null_flavour.
    // - `Inv_null_flavour_valid`: `is_null implies
    //   terminology(Terminology_id_openehr).has_code_for_group_id(
    //   Group_id_null_flavour, null_flavour.defining_code)`. Requires a
    //   `TERMINOLOGY_SERVICE` lookup (see `openehr-terminology`) to verify
    //   `null_flavour.defining_code` is a member of the openEHR
    //   `null flavours` terminology group — not checkable from `Element`'s
    //   own fields alone.
    // - `Inv_null_reason_valid`: `null_reason /= Void implies is_null()`.
    //   A `null_reason` may only be set when the element is actually null.
}

pub const TYPE_NAME: &str = "ELEMENT";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.representation §ELEMENT — docs/research/spec-cache/RM-1.1.0/uml_classes/element.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-representation_package.adoc §Class Descriptions / element.adoc §ELEMENT Class
//   confidence: high
//   todos: 1
//   note: is_null() is implemented (definitional); the three remaining Inv_* invariants are recorded as doc text pending the Validate-trait framework, one of which needs a terminology-service lookup. P4/ADR-002: self-tag (TypeName + first-field TypeTag) added; value/null_flavour field types (DataValue enum, DvCodedText) are a sibling agent's conversion.
// ─────────────────────────────────────────────
