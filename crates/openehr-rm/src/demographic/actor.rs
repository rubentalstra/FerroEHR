//! `ACTOR` — ancestor of all real-world actor types.
//!
//! openEHR class: `ACTOR` (abstract), package `rm.demographic`.
//!
//! Ancestor of all real-world types, including people and organisations. An
//! actor is any real-world entity capable of taking on a role.
use super::agent::Agent;
use super::group::Group;
use super::organisation::Organisation;
use super::party::{PartyApi, PartyData};
use super::person::Person;
use serde::{Deserialize, Serialize};

/// Shared attribute state of `ACTOR` and its descendants.
///
/// Per ADR-001 §3, embeds the parent (`PARTY`) state plus this class's own
/// two attributes.
///
/// Per ADR-002, `ActorData` is an abstract-class embedded `*Data` struct
/// and carries **no** `_type` tag of its own; only the four concrete
/// leaves self-tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorData {
    /// Inherited `PARTY` state.
    #[serde(flatten)]
    pub party: PartyData,

    /// `languages`: `List<DV_TEXT>` `[0..1]` — languages which can be used
    /// to communicate with this actor, in preferred order of use (if known,
    /// else order irrelevant).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub languages: Option<Vec<crate::data_types::text::dv_text::DvText>>,

    /// `roles`: `List<PARTY_REF>` `[0..1]` — identifiers of the Version
    /// container for each Role played by this Party.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<openehr_base::identification::party_ref::PartyRef>>,
}

/// `ACTOR` is abstract in the spec; its four concrete descendants —
/// `PERSON`, `ORGANISATION`, `GROUP`, `AGENT` — are collected into this
/// closed `enum` per ADR-001 §4, nested one level inside the wider `Party`
/// enum (see `party.rs`) per the task's Phase-P1 refinement.
///
/// PORT NOTE: `#[serde(untagged)]` per ADR-002 — the enum itself carries no
/// tag; dispatch is driven by each variant payload's own `TypeTag` field
/// (`Person`/`Organisation`/`Group`/`Agent` each self-tag with their
/// canonical `_type`, and a `TypeTag` fails deserialization on a mismatched
/// `_type` string, making untagged probing tag-driven even though the four
/// payloads are structurally identical). The former
/// `#[serde(tag = "_type")]` + per-variant renames would duplicate the
/// payloads' own `_type` key and is removed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Actor {
    /// `PERSON`.
    Person(Person),
    /// `ORGANISATION`.
    Organisation(Organisation),
    /// `GROUP`.
    Group(Group),
    /// `AGENT`.
    Agent(Agent),
}

/// Marker/accessor trait shared by every `ACTOR` descendant, exposing the
/// abstract class's own attributes (and, transitively, `PARTY`'s) uniformly
/// whether the caller holds a concrete type or an `Actor` enum value.
pub trait ActorApi: PartyApi {
    /// Access to the embedded `ActorData`.
    fn actor_data(&self) -> &ActorData;

    /// `languages`: `List<DV_TEXT>` `[0..1]`.
    fn languages(&self) -> Option<&[crate::data_types::text::dv_text::DvText]> {
        self.actor_data().languages.as_deref()
    }

    /// `roles`: `List<PARTY_REF>` `[0..1]`.
    fn roles(&self) -> Option<&[openehr_base::identification::party_ref::PartyRef]> {
        self.actor_data().roles.as_deref()
    }
}

impl ActorApi for Actor {
    fn actor_data(&self) -> &ActorData {
        match self {
            Actor::Person(p) => &p.actor,
            Actor::Organisation(o) => &o.actor,
            Actor::Group(g) => &g.actor,
            Actor::Agent(a) => &a.actor,
        }
    }
}

impl PartyApi for Actor {
    fn party_data(&self) -> &PartyData {
        &self.actor_data().party
    }
}

// No invariants declared on `ACTOR` in its own spec table.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions ACTOR — docs/research/spec-cache/RM-1.1.0/uml_classes/actor.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/actor.adoc §ACTOR Class
//   confidence: high
//   todos: 0
//   note: Actor enum nested inside Party enum per task refinement; ActorApi extends PartyApi to keep the trait hierarchy matching the spec's inheritance chain. P4/ADR-002: #[serde(untagged)] enum, dispatch via each leaf's own TypeTag payload tag (former internal tag + renames removed); ActorData stays tag-less (abstract), flatten+skip-if-none per field.
// ─────────────────────────────────────────────
