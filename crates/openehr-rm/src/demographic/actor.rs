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

/// Shared attribute state of `ACTOR` and its descendants.
///
/// Per ADR-001 §3, embeds the parent (`PARTY`) state plus this class's own
/// two attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct ActorData {
    /// Inherited `PARTY` state.
    pub party: PartyData,

    /// `languages`: `List<DV_TEXT>` `[0..1]` — languages which can be used
    /// to communicate with this actor, in preferred order of use (if known,
    /// else order irrelevant).
    pub languages: Option<Vec<crate::data_types::text::dv_text::DvText>>,

    /// `roles`: `List<PARTY_REF>` `[0..1]` — identifiers of the Version
    /// container for each Role played by this Party.
    pub roles: Option<Vec<openehr_base::identification::party_ref::PartyRef>>,
}

/// `ACTOR` is abstract in the spec; its four concrete descendants —
/// `PERSON`, `ORGANISATION`, `GROUP`, `AGENT` — are collected into this
/// closed `enum` per ADR-001 §4, nested one level inside the wider `Party`
/// enum (see `party.rs`) per the task's Phase-P1 refinement.
#[derive(Debug, Clone, PartialEq)]
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
//   note: Actor enum nested inside Party enum per task refinement; ActorApi extends PartyApi to keep the trait hierarchy matching the spec's inheritance chain.
// ─────────────────────────────────────────────
