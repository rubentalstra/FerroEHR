//! `ROLE` — a role performed by an Actor.
//!
//! openEHR class: `ROLE` (concrete), package `rm.demographic`.
//!
//! Generic description of a role performed by an Actor. The role
//! corresponds to a competency of the Party. Roles are used to define the
//! responsibilities undertaken by a Party for a purpose. Roles should have
//! credentials qualifying the performer to perform the role.
use super::capability::Capability;
use super::party::{PartyApi, PartyData};
use crate::data_types::quantity::dv_interval::DvInterval;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class, single-sourcing the [`TypeName`] impl below
/// (ADR-002).
pub const TYPE_NAME: &str = "ROLE";

/// `ROLE` — inherits `PARTY` directly (see `party.adoc` `Inherit`: `PARTY`).
///
/// PORT NOTE: `ROLE` is a sibling of `ACTOR` under `PARTY`, not a
/// descendant of `ACTOR` — see `party.rs`'s `Party` enum, which has
/// `Actor(Actor)` and `Role(Role)` as its only two variants, matching this
/// directly. `#[serde(flatten)]` folds `PartyData` into `ROLE`'s own JSON
/// object. Per ADR-002 the class self-tags via its first field, so a `Role`
/// carries its own `_type: "ROLE"` exactly like the four `ACTOR` leaves —
/// resolving the previous P4 wave's flagged asymmetry where `Role` reached
/// the wire tag-less through `Party`'s enum-level dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Role {
    /// Canonical `_type` discriminator (`"ROLE"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `PARTY` state.
    #[serde(flatten)]
    pub party: PartyData,

    /// `time_validity`: `DV_INTERVAL<DV_DATE>` `[0..1]` — valid time
    /// interval for this role.
    ///
    /// TODO(port): `DvInterval<T>`'s `T: DvOrdered` bound and `DvDate`'s
    /// concrete shape are owned by the `data_types` sibling package;
    /// forward-referenced here.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_validity: Option<DvInterval<crate::data_types::date_time::dv_date::DvDate>>,

    /// `performer`: `PARTY_REF` `[1..1]` — reference to Version container
    /// of Actor playing the role.
    pub performer: openehr_base::identification::party_ref::PartyRef,

    /// `capabilities`: `List<CAPABILITY>` `[0..1]` — the capabilities of
    /// this role.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub capabilities: Option<Vec<Capability>>,
}

impl TypeName for Role {
    const NAME: &'static str = TYPE_NAME;
}

impl PartyApi for Role {
    fn party_data(&self) -> &PartyData {
        &self.party
    }
}

impl Role {
    /// Invariant `Capabilities_valid`: `capabilities /= Void implies not
    /// capabilities.empty` (ADR-003 §8).
    #[must_use]
    pub fn invariant_capabilities_valid(&self) -> bool {
        self.capabilities.as_ref().is_none_or(|c| !c.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::identification::hier_object_id::HierObjectId;
    use openehr_base::identification::object_id::ObjectId;
    use openehr_base::identification::party_ref::PartyRef;
    use openehr_base::identification::uid_based_id::{UidBasedId, UidBasedIdData};

    fn role(capabilities: Option<Vec<Capability>>) -> Role {
        Role {
            type_tag: TypeTag::new(),
            party: PartyData {
                locatable: LocatableData {
                    name: DvText::Text {
                        type_tag: TypeTag::new(),
                        data: DvTextData {
                            value: "general practitioner".to_string(),
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
                identities: Vec::new(),
                contacts: None,
                details: None,
                reverse_relationships: None,
                relationships: None,
            },
            time_validity: None,
            performer: PartyRef {
                type_tag: TypeTag::new(),
                namespace: "demographic".to_string(),
                r#type: "PERSON".to_string(),
                id: ObjectId::UidBased(UidBasedId::HierObjectId(HierObjectId {
                    type_tag: TypeTag::new(),
                    uid_based_id: UidBasedIdData {
                        value: "8849182c-82ad-4088-a07f-48ead4180515".to_string(),
                    },
                })),
            },
            capabilities,
        }
    }

    #[test]
    fn capabilities_valid_rejects_present_but_empty() {
        assert!(role(None).invariant_capabilities_valid()); // None: valid
        assert!(!role(Some(Vec::new())).invariant_capabilities_valid()); // empty: invalid
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions ROLE — docs/research/spec-cache/RM-1.1.0/uml_classes/role.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/role.adoc §ROLE Class
//   confidence: high
//   todos: 1
//   note: Role is a direct PARTY descendant (sibling of ACTOR, not a subtype of it) — matches Party enum's two top-level variants. P4/ADR-002: self-tags via TypeTag<Self> first field (TypeName from TYPE_NAME), no-op struct-level rename deleted; Party dispatches untagged on this payload tag, so ROLE now carries its own _type (previous wave's flag resolved).
// ─────────────────────────────────────────────
