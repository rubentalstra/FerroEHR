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

    /// `Inv_null_flavour_indicated`: `is_null() xor null_flavour = Void`.
    ///
    /// I.e. exactly one of {value is absent, null_flavour is present} holds —
    /// a value cannot be null without a null_flavour, and a non-null value
    /// cannot carry a null_flavour. Equivalent to `is_null() ==
    /// null_flavour.is_some()`. Working `invariant_*` method per ADR-003 §8
    /// (the deep `Validate` walker remains the P11 deliverable).
    pub fn invariant_null_flavour_indicated(&self) -> bool {
        self.is_null() == self.null_flavour.is_some()
    }

    /// `Inv_null_reason_valid`: `null_reason /= Void implies is_null()`.
    ///
    /// A `null_reason` may only be set when the element is actually null.
    /// Working `invariant_*` method per ADR-003 §8.
    pub fn invariant_null_reason_valid(&self) -> bool {
        self.null_reason.is_none() || self.is_null()
    }

    // TODO(port): invariant `Inv_null_flavour_valid`: `is_null implies
    // terminology(Terminology_id_openehr).has_code_for_group_id(
    // Group_id_null_flavour, null_flavour.defining_code)`. Requires a
    // `TERMINOLOGY_SERVICE` lookup (see `openehr-terminology`) to verify
    // `null_flavour.defining_code` is a member of the openEHR `null flavours`
    // terminology group — not checkable from `Element`'s own fields alone;
    // deferred to the terminology-bound `Validate` pass at P11.
}

pub const TYPE_NAME: &str = "ELEMENT";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::data_structures::representation::item::ItemData;

    fn locatable(name: &str, node_id: &str) -> LocatableData {
        serde_json::from_value(serde_json::json!({
            "name": { "_type": "DV_TEXT", "value": name },
            "archetype_node_id": node_id,
        }))
        .unwrap()
    }

    fn dv_text(value: &str) -> DvText {
        serde_json::from_value(serde_json::json!({ "_type": "DV_TEXT", "value": value })).unwrap()
    }

    fn coded_null(value: &str, code: &str) -> DvCodedText {
        serde_json::from_value(serde_json::json!({
            "_type": "DV_CODED_TEXT",
            "value": value,
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": code,
            },
        }))
        .unwrap()
    }

    fn element(value: Option<DataValue>, null_flavour: Option<DvCodedText>) -> Element {
        Element {
            type_tag: TypeTag::new(),
            item: ItemData {
                locatable: locatable("systolic", "at0004"),
            },
            null_flavour,
            value,
            null_reason: None,
        }
    }

    /// Spec `is_null` / `Inv_is_null_valid`: `is_null() = (value = Void)`.
    #[test]
    fn is_null_tracks_value_absence() {
        assert!(!element(Some(DataValue::Text(dv_text("120"))), None).is_null());
        assert!(element(None, Some(coded_null("unknown", "253"))).is_null());
    }

    /// Spec `Inv_null_flavour_indicated`: `is_null() xor null_flavour = Void`.
    #[test]
    fn null_flavour_indicated_invariant() {
        // value present, no null_flavour → valid
        assert!(
            element(Some(DataValue::Text(dv_text("120"))), None).invariant_null_flavour_indicated()
        );
        // value absent, null_flavour present → valid
        assert!(
            element(None, Some(coded_null("unknown", "253"))).invariant_null_flavour_indicated()
        );
        // value absent, null_flavour absent → violates (null without a flavour)
        assert!(!element(None, None).invariant_null_flavour_indicated());
        // value present, null_flavour present → violates (non-null with a flavour)
        assert!(
            !element(
                Some(DataValue::Text(dv_text("120"))),
                Some(coded_null("unknown", "253"))
            )
            .invariant_null_flavour_indicated()
        );
    }

    /// Spec `Inv_null_reason_valid`: `null_reason /= Void implies is_null()`.
    #[test]
    fn null_reason_valid_invariant() {
        // no null_reason → trivially valid
        assert!(element(Some(DataValue::Text(dv_text("120"))), None).invariant_null_reason_valid());
        // null_reason set on a non-null element → violates
        let mut e = element(Some(DataValue::Text(dv_text("120"))), None);
        e.null_reason = Some(dv_text("patient refused"));
        assert!(!e.invariant_null_reason_valid());
        // null_reason set on a genuinely null element → valid
        let mut e = element(None, Some(coded_null("no information", "271")));
        e.null_reason = Some(dv_text("no reason provided"));
        assert!(e.invariant_null_reason_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.representation §ELEMENT — docs/research/spec-cache/RM-1.1.0/uml_classes/element.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-representation_package.adoc §Class Descriptions / element.adoc §ELEMENT Class
//   confidence: high
//   todos: 1
//   note: is_null() implemented (definitional); Inv_null_flavour_indicated and Inv_null_reason_valid now working invariant_* methods (ADR-003 §8) with unit tests; only Inv_null_flavour_valid remains TODO(port) (needs a terminology-service group lookup, P11). P4/ADR-002: self-tag (TypeName + first-field TypeTag) added; value/null_flavour field types (DataValue enum, DvCodedText) are a sibling agent's conversion.
// ─────────────────────────────────────────────
