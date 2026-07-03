//! `PARTY_RELATIONSHIP` — a relationship between parties.
//!
//! openEHR class: `PARTY_RELATIONSHIP` (concrete), package
//! `rm.demographic`.
//!
//! Generic description of a relationship between parties.
use crate::common::archetyped::locatable::LocatableData;
use crate::data_types::quantity::dv_interval::DvInterval;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class, single-sourcing the [`TypeName`] impl below
/// (ADR-002).
pub const TYPE_NAME: &str = "PARTY_RELATIONSHIP";

/// `PARTY_RELATIONSHIP` inherits `LOCATABLE` directly (not `PARTY`).
/// `#[serde(flatten)]` folds `LocatableData` into `PARTY_RELATIONSHIP`'s own
/// JSON object; per ADR-002 the class self-tags via its first field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyRelationship {
    /// Canonical `_type` discriminator (`"PARTY_RELATIONSHIP"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `LOCATABLE` state.
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `details`: `ITEM_STRUCTURE` `[0..1]` — the detailed description of
    /// the relationship.
    ///
    /// TODO(port): forward-reference to
    /// `crate::data_structures::item_structure::ItemStructure` (sibling
    /// agent's package).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub details: Option<crate::data_structures::item_structure::ItemStructure>,

    /// `target`: `PARTY_REF` `[1..1]` — target of relationship.
    pub target: openehr_base::identification::party_ref::PartyRef,

    /// `time_validity`: `DV_INTERVAL<DV_DATE>` `[0..1]` — valid time
    /// interval for this relationship.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_validity: Option<DvInterval<crate::data_types::date_time::dv_date::DvDate>>,

    /// `source`: `PARTY_REF` `[1..1]` — source of relationship.
    pub source: openehr_base::identification::party_ref::PartyRef,
}

impl TypeName for PartyRelationship {
    const NAME: &'static str = TYPE_NAME;
}

impl PartyRelationship {
    /// Spec function `type(): DV_TEXT` — type of relationship, such as
    /// "employment", "authority", "health provision". Taken from the
    /// inherited `name` attribute.
    ///
    /// Invariant `Type_validity`: `type = name` — see
    /// [`PartyRelationship::invariant_type_validity`].
    #[must_use]
    pub fn relationship_type(&self) -> crate::data_types::text::dv_text::DvText {
        self.locatable.name.clone()
    }

    /// Invariant `Type_validity`: `type = name` (ADR-003 §8). Structurally
    /// guaranteed by [`PartyRelationship::relationship_type`] (it clones
    /// `name`), evaluated literally here.
    #[must_use]
    pub fn invariant_type_validity(&self) -> bool {
        self.relationship_type() == self.locatable.name
    }
}

// The remaining two invariants dereference through a `PARTY_REF` to the
// referenced Party's own `relationships`/`reverse_relationships` lists (and,
// for `Target_valid`, the identity of `self`), which presumes a resolvable
// demographic object graph not available in a spec-transcription crate:
//   TODO(port): Source_valid: source /= Void and then
//     source.relationships.has(self) — deferred to P11/P15 (service layer /
//     Validate framework); needs the demographic object graph.
//   TODO(port): Target_valid: target /= Void and then
//     not target.reverse_relationships.has(self) — deferred to P11/P15.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::dv_text::{DvText, DvTextApi, DvTextData};
    use openehr_base::identification::hier_object_id::HierObjectId;
    use openehr_base::identification::object_id::ObjectId;
    use openehr_base::identification::party_ref::PartyRef;
    use openehr_base::identification::uid_based_id::{UidBasedId, UidBasedIdData};

    fn party_ref() -> PartyRef {
        PartyRef {
            type_tag: TypeTag::new(),
            namespace: "demographic".to_string(),
            r#type: "PERSON".to_string(),
            id: ObjectId::UidBased(UidBasedId::HierObjectId(HierObjectId {
                type_tag: TypeTag::new(),
                uid_based_id: UidBasedIdData {
                    value: "8849182c-82ad-4088-a07f-48ead4180515".to_string(),
                },
            })),
        }
    }

    #[test]
    fn relationship_type_clones_name_and_type_validity_holds() {
        let relationship = PartyRelationship {
            type_tag: TypeTag::new(),
            locatable: LocatableData {
                name: DvText::Text {
                    type_tag: TypeTag::new(),
                    data: DvTextData {
                        value: "employment".to_string(),
                        hyperlink: None,
                        formatting: None,
                        mappings: None,
                        language: None,
                        encoding: None,
                    },
                },
                archetype_node_id: "at0000".to_string(),
                uid: None,
                links: None,
                archetype_details: None,
                feeder_audit: None,
                parent: None,
            },
            details: None,
            target: party_ref(),
            time_validity: None,
            source: party_ref(),
        };
        assert_eq!(relationship.relationship_type().value(), "employment");
        assert!(relationship.invariant_type_validity());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions PARTY_RELATIONSHIP — docs/research/spec-cache/RM-1.1.0/uml_classes/party_relationship.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/party_relationship.adoc §PARTY_RELATIONSHIP Class
//   confidence: high
//   todos: 3
//   note: Source_valid/Target_valid invariants require resolving through a Party object graph not modelled in this crate; left as TODO. P4/ADR-002: self-tags via TypeTag<Self> first field (TypeName from TYPE_NAME); no-op struct-level rename deleted.
// ─────────────────────────────────────────────
