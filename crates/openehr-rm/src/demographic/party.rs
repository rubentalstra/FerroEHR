//! `PARTY` — ancestor of all Party types.
//!
//! openEHR class: `PARTY` (abstract), package `rm.demographic`.
//!
//! Ancestor of all Party types, including real world entities and their
//! roles. A Party is any entity which can participate in an activity. The
//! `name` attribute inherited from `LOCATABLE` is used to indicate the
//! actual type of party (note that the actual names, i.e. identities of
//! parties, are indicated in the `identities` attribute, not the `name`
//! attribute).
//!
//! NOTE (spec): it is strongly recommended that the inherited attribute
//! `uid` be populated in `PARTY` objects, using the UID copied from the
//! `object_id()` of the `uid` field of the enclosing `VERSION` object. For
//! example, the `ORIGINAL_VERSION.uid`
//! `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2` would be copied to
//! the `uid` field of the `PARTY` object. This recommendation is elevated to
//! a hard invariant (`Uid_mandatory`) in this class's own table — see
//! below.
use super::actor::Actor;
use super::party_identity::PartyIdentity;
use super::party_relationship::PartyRelationship;
use super::role::Role;
use crate::common::archetyped::locatable::LocatableData;
use serde::{Deserialize, Serialize};
// TODO(port): `crate::common::generic` back-reference types (`CONTACT`,
// `LOCATABLE_REF`) — forward-referenced below; sibling agent owns
// `common/generic`.

/// Shared attribute state of `PARTY` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait). Every concrete `PARTY` descendant (via the `Actor`/`Role`
/// branches) embeds this struct, which itself embeds `LOCATABLE`'s state
/// per ADR-001 §Refinements.
///
/// PORT NOTE: `uid` on `LocatableData` is spec-declared `0..1` on
/// `LOCATABLE` (optional), but `PARTY`'s own `Uid_mandatory` invariant
/// (`uid /= Void`) narrows it to effectively required for every `PARTY`
/// instance. This is not encoded as a covariant field-type narrowing (the
/// declared type `UID_BASED_ID` is unchanged, only its optionality is
/// invariant-constrained), so the field stays `Option<UidBasedId>` on
/// `LocatableData` and the requirement is documented here and left as a
/// `Validate`-impl `TODO(port)` rather than changed to a non-`Option` field
/// — changing the field type would be inventing a structural deviation the
/// spec does not make (it is `LOCATABLE.uid: UID_BASED_ID [0..1]` in every
/// other subtype), and this class only makes it invariant-mandatory, not
/// syntactically required.
///
/// Per ADR-002, `PartyData` is an abstract-class embedded `*Data` struct
/// and carries **no** `_type` tag of its own; only the concrete leaves
/// (`PERSON`, `ORGANISATION`, `GROUP`, `AGENT`, `ROLE`) self-tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyData {
    /// Inherited `LOCATABLE` state (`name`, `archetype_node_id`, `uid`,
    /// `links`, `archetype_details`, `feeder_audit`).
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `identities`: `List<PARTY_IDENTITY>` — identities used by the party
    /// to identify itself, such as legal name, stage names, aliases,
    /// nicknames and so on.
    ///
    /// Cardinality `1..1` in the spec table (always present, though the
    /// `List` itself may be empty — the spec does not state a
    /// non-empty invariant on `identities` the way it does for `contacts`).
    pub identities: Vec<PartyIdentity>,

    /// `contacts`: `List<CONTACT>` `[0..1]` — contacts for this party.
    ///
    /// TODO(port): `CONTACT` lives in this same `demographic` module
    /// (`contact.rs`); imported directly once wired.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub contacts: Option<Vec<super::contact::Contact>>,

    /// `details`: `ITEM_STRUCTURE` `[0..1]` — all other details for this
    /// Party.
    ///
    /// TODO(port): forward-reference to `crate::data_structures::item_structure`'s
    /// closed `ItemStructure` enum (owned by a sibling agent's package).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub details: Option<crate::data_structures::item_structure::ItemStructure>,

    /// `reverse_relationships`: `List<LOCATABLE_REF>` `[0..1]` — references
    /// to relationships in which this Party takes part as target.
    ///
    /// TODO(port): forward-reference to `crate::common::generic::LocatableRef`
    /// (sibling agent owns `common/generic`); using `openehr_base`'s
    /// `LocatableRef` name pending confirmation of where the RM re-exports
    /// or wraps it.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reverse_relationships:
        Option<Vec<openehr_base::identification::locatable_ref::LocatableRef>>,

    /// `relationships`: `List<PARTY_RELATIONSHIP>` `[0..1]` — relationships
    /// in which this Party takes part as source.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub relationships: Option<Vec<PartyRelationship>>,
}

/// `PARTY` is abstract in the spec. Its only concrete descendants are the
/// two immediate subtypes `ACTOR` (itself abstract, further split into
/// `PERSON`/`ORGANISATION`/`GROUP`/`AGENT`) and `ROLE` (concrete).
///
/// Per the task's Phase-P1 refinement, this is modelled as **nested**
/// closed enums rather than one flat five-way enum: `Party` has exactly two
/// variants (`Actor`, `Role`), and `Actor` (a separate enum, see
/// `actor.rs`) has the four concrete leaves. This mirrors the two-level
/// spec hierarchy (`PARTY` → `ACTOR` → `{PERSON, ORGANISATION, GROUP,
/// AGENT}`; `PARTY` → `ROLE`) directly rather than flattening it, matching
/// ADR-001 §4's "closed subtype set → enum" rule applied at each level of
/// the hierarchy the spec itself declares.
///
/// PORT NOTE: `#[serde(untagged)]` per ADR-002 — both levels of the nested
/// enum (`Party` here, `Actor` in `actor.rs`) carry no tag of their own;
/// dispatch is driven by each concrete leaf payload's own `TypeTag` field
/// (`PERSON`/`ORGANISATION`/`GROUP`/`AGENT` via the `Actor` branch, `ROLE`
/// directly), whose `Deserialize` fails on a mismatched `_type` string, so
/// untagged probing selects exactly the variant whose class name matches.
/// This resolves the previous P4 wave's flagged asymmetry (a manual
/// wire-enum here, and before that an internally tagged `Actor` under which
/// `Role` reached the wire with no `_type` at all): every concrete `PARTY`
/// leaf, `Role` included, now self-tags identically, and the abstract
/// `PARTY`/`ACTOR` layers emit nothing — matching the pinned schema's
/// absence of abstract-class definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Party {
    /// The `ACTOR` branch (`PERSON`, `ORGANISATION`, `GROUP`, or `AGENT`).
    Actor(Actor),
    /// `ROLE`.
    Role(Role),
}

/// Marker/accessor trait shared by every `PARTY` descendant, exposing the
/// abstract class's attributes uniformly whether the caller holds a
/// concrete type or a `Party` enum value.
pub trait PartyApi {
    /// Access to the embedded `PartyData`.
    fn party_data(&self) -> &PartyData;

    /// `identities`: `List<PARTY_IDENTITY>`.
    fn identities(&self) -> &[PartyIdentity] {
        &self.party_data().identities
    }

    /// `contacts`: `List<CONTACT>` `[0..1]`.
    fn contacts(&self) -> Option<&[super::contact::Contact]> {
        self.party_data().contacts.as_deref()
    }

    /// `details`: `ITEM_STRUCTURE` `[0..1]`.
    fn details(&self) -> Option<&crate::data_structures::item_structure::ItemStructure> {
        self.party_data().details.as_ref()
    }

    /// `reverse_relationships`: `List<LOCATABLE_REF>` `[0..1]`.
    fn reverse_relationships(
        &self,
    ) -> Option<&[openehr_base::identification::locatable_ref::LocatableRef]> {
        self.party_data().reverse_relationships.as_deref()
    }

    /// `relationships`: `List<PARTY_RELATIONSHIP>` `[0..1]`.
    fn relationships(&self) -> Option<&[PartyRelationship]> {
        self.party_data().relationships.as_deref()
    }

    /// Spec function `type(): DV_TEXT` — type of party, such as `PERSON`,
    /// `ORGANISATION`, etc. Role name, e.g. "general practitioner", "nurse",
    /// "private citizen". Taken from the inherited `name` attribute.
    ///
    /// Invariant `Type_valid`: `type = name`.
    ///
    /// TODO(port): implement once `LocatableData.name: DvText` is concrete
    /// (sibling agent owns `common/archetyped`); this should simply clone
    /// `self.party_data().locatable.name`.
    fn party_type(&self) -> crate::data_types::text::dv_text::DvText {
        todo!("PARTY.type(): DV_TEXT — clone LocatableData.name once concrete")
    }
}

impl PartyApi for Party {
    fn party_data(&self) -> &PartyData {
        match self {
            Party::Actor(a) => a.party_data(),
            Party::Role(r) => &r.party,
        }
    }
}

// TODO(port): invariants as a `Validate` impl (context + path + error
// accumulator, per `.claude/rules/rm-transcription.md` "Invariants"):
//   - Type_valid: type = name
//   - Contacts_valid: contacts /= Void implies not contacts.is_empty
//   - Relationships_validity: relationships /= Void implies
//     (not relationships.is_empty and then
//      relationships.for_all(r | r.source = self))
//   - Reverse_relationships_validity: reverse_relationships /= Void implies
//     (not reverse_relationships.empty and then
//      reverse_relationships.for_all(item |
//        repository("demographics").all_party_relationships.has_object(item)
//        and then
//        repository("demographics").all_party_relationships.object(item).target = self))
//     — this invariant references a `repository(...)` construct with no
//     concrete Rust analogue in a spec-transcription crate (it presumes an
//     external object repository/service); left as `TODO(port)` rather than
//     inventing a repository abstraction the spec class itself does not
//     define.
//   - Is_archetype_root: is_archetype_root
//   - Uid_mandatory: uid /= Void — see the PORT NOTE on `PartyData` above.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use crate::demographic::actor::ActorData;
    use crate::demographic::person::Person;
    use openehr_base::identification::hier_object_id::HierObjectId;
    use openehr_base::identification::object_id::ObjectId;
    use openehr_base::identification::party_ref::PartyRef;
    use openehr_base::identification::uid_based_id::{UidBasedId, UidBasedIdData};
    use openehr_foundation::serde_support::TypeTag;

    fn party_data(name: &str) -> PartyData {
        PartyData {
            locatable: LocatableData {
                name: DvText::Text {
                    type_tag: TypeTag::new(),
                    data: DvTextData {
                        value: name.to_string(),
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
            identities: vec![],
            contacts: None,
            details: None,
            reverse_relationships: None,
            relationships: None,
        }
    }

    /// ADR-002: a `PARTY`-typed (abstract) slot round-trips both an `ACTOR`
    /// leaf (`PERSON`, two untagged levels deep) and a `ROLE` (one level)
    /// purely via each payload's own `TypeTag` — including that `Role` now
    /// carries its own `_type: "ROLE"`, the previous wave's unresolved flag.
    #[test]
    fn party_slot_round_trips_person_and_role_via_untagged_chain() {
        let person = Party::Actor(Actor::Person(Person {
            type_tag: TypeTag::new(),
            actor: ActorData {
                party: party_data("person party"),
                languages: None,
                roles: None,
            },
        }));
        let person_json = serde_json::to_value(&person).expect("serialize Party::Actor(Person)");
        assert_eq!(person_json["_type"], "PERSON");
        let person_back: Party =
            serde_json::from_value(person_json).expect("deserialize PERSON into Party slot");
        assert_eq!(person_back, person);

        let role = Party::Role(Role {
            type_tag: TypeTag::new(),
            party: party_data("role party"),
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
            capabilities: None,
        });
        let role_json = serde_json::to_value(&role).expect("serialize Party::Role");
        assert_eq!(role_json["_type"], "ROLE");
        let role_back: Party =
            serde_json::from_value(role_json).expect("deserialize ROLE into Party slot");
        assert_eq!(role_back, role);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions PARTY — docs/research/spec-cache/RM-1.1.0/uml_classes/party.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/party.adoc §PARTY Class
//   confidence: medium
//   todos: 8
//   note: Party/Actor nested-enum shape per task refinement; Uid_mandatory invariant narrows optionality not type, left as Validate TODO; Reverse_relationships_validity references an undefined repository() construct. P4/ADR-002: Party and Actor both #[serde(untagged)], dispatch via each concrete leaf's own TypeTag (manual wire enum deleted; Role now self-tags — previous wave's flag resolved); PartyData stays tag-less (abstract), flatten+skip-if-none per field; round-trip pinned by in-file test.
// ─────────────────────────────────────────────
