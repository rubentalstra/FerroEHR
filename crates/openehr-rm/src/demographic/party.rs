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
/// `LocatableData` and the requirement is enforced by the working
/// [`PartyApi::invariant_uid_mandatory`] check (ADR-003 §8) rather than by
/// changing the field to a non-`Option` type — changing the field type would
/// be inventing a structural deviation the spec does not make (it is
/// `LOCATABLE.uid: UID_BASED_ID [0..1]` in every other subtype), and this
/// class only makes it invariant-mandatory, not syntactically required.
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
    /// Invariant `Type_valid`: `type = name` — see
    /// [`PartyApi::invariant_type_valid`].
    fn party_type(&self) -> crate::data_types::text::dv_text::DvText {
        self.party_data().locatable.name.clone()
    }

    /// Invariant `Type_valid`: `type = name` (ADR-003 §8). Structurally
    /// guaranteed by [`PartyApi::party_type`]'s definition (it clones
    /// `name`), evaluated literally here.
    fn invariant_type_valid(&self) -> bool {
        self.party_type() == self.party_data().locatable.name
    }

    /// Invariant `Contacts_valid`: `contacts /= Void implies not
    /// contacts.is_empty` (ADR-003 §8).
    fn invariant_contacts_valid(&self) -> bool {
        self.party_data()
            .contacts
            .as_ref()
            .is_none_or(|contacts| !contacts.is_empty())
    }

    /// Invariant `Is_archetype_root`: `is_archetype_root` (ADR-003 §8);
    /// derived from `archetype_details /= Void`.
    fn invariant_is_archetype_root(&self) -> bool {
        self.party_data().locatable.archetype_details.is_some()
    }

    /// Invariant `Uid_mandatory`: `uid /= Void` (ADR-003 §8). `PARTY`
    /// narrows the inherited optional `LOCATABLE.uid` to effectively
    /// required — see the [`PartyData`] PORT NOTE.
    fn invariant_uid_mandatory(&self) -> bool {
        self.party_data().locatable.uid.is_some()
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

// Invariants implemented as working `PartyApi` default methods (ADR-003 §8):
//   - Type_valid: type = name        → `invariant_type_valid`
//   - Contacts_valid                 → `invariant_contacts_valid`
//   - Is_archetype_root              → `invariant_is_archetype_root`
//   - Uid_mandatory: uid /= Void     → `invariant_uid_mandatory`
//
// The remaining two invariants reference the referenced Party's own
// relationship lists / an external object repository, which a
// spec-transcription crate cannot resolve — kept as cited TODOs rather than
// inventing a repository/identity abstraction the spec class does not define:
//   TODO(port): Relationships_validity: relationships /= Void implies
//     (not relationships.is_empty and then relationships.for_all(r |
//      r.source = self)) — the `r.source = self` conjunct needs object
//     identity of `self` behind a `PARTY_REF`, resolved via the demographic
//     object graph; deferred to P11/P15 (service layer / Validate framework).
//   TODO(port): Reverse_relationships_validity: reverse_relationships /= Void
//     implies (not reverse_relationships.empty and then
//     reverse_relationships.for_all(item |
//       repository("demographics").all_party_relationships.has_object(item)
//       and then repository(...).object(item).target = self)) — references a
//     `repository(...)` service with no analogue in this crate; deferred to
//     P11/P15.

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

    fn person(data: PartyData) -> Party {
        Party::Actor(Actor::Person(Person {
            type_tag: TypeTag::new(),
            actor: ActorData {
                party: data,
                languages: None,
                roles: None,
            },
        }))
    }

    /// `PARTY` invariants (ADR-003 §8), exercised through the abstract
    /// `Party` slot's `PartyApi` default methods.
    #[test]
    fn party_invariants_hold_and_fail_as_specified() {
        // Fresh party: no uid, no contacts, empty identities.
        let party = person(party_data("PERSON"));
        assert!(party.identities().is_empty());
        assert!(party.invariant_contacts_valid()); // contacts None: valid
        assert!(!party.invariant_uid_mandatory()); // uid None: fails Uid_mandatory
        assert!(party.invariant_type_valid()); // type() == name by construction
        assert_eq!(party.party_type(), party.party_data().locatable.name);

        // With a uid and a present-but-empty contacts list.
        let mut data = party_data("PERSON");
        data.locatable.uid = Some(UidBasedId::HierObjectId(HierObjectId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: "8849182c-82ad-4088-a07f-48ead4180515".to_string(),
            },
        }));
        data.contacts = Some(Vec::new());
        let party = person(data);
        assert!(party.invariant_uid_mandatory()); // uid present: holds
        assert!(!party.invariant_contacts_valid()); // present-but-empty: fails
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions PARTY — docs/research/spec-cache/RM-1.1.0/uml_classes/party.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/party.adoc §PARTY Class
//   confidence: medium
//   todos: 6
//   note: Party/Actor nested-enum shape per task refinement. P5/ADR-003 §8: PartyApi.party_type() (clones name) plus Type_valid, Contacts_valid, Is_archetype_root, Uid_mandatory implemented as working default methods, pinned by a PARTY-invariants unit test. The 6 remaining TODO(port) are forward-ref import/field comments plus the 2 genuinely-underdetermined invariants (Relationships_validity's `r.source = self` and Reverse_relationships_validity's `repository("demographics")`), kept as cited P11/P15 deferrals — both need the demographic object graph/identity, absent in a spec-transcription crate. P4/ADR-002: Party/Actor #[serde(untagged)]; PartyData tag-less.
// ─────────────────────────────────────────────
