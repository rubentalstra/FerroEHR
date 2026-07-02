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

/// `ROLE` — inherits `PARTY` directly (see `party.adoc` `Inherit`: `PARTY`).
///
/// PORT NOTE: `ROLE` is a sibling of `ACTOR` under `PARTY`, not a
/// descendant of `ACTOR` — see `party.rs`'s `Party` enum, which has
/// `Actor(Actor)` and `Role(Role)` as its only two variants, matching this
/// directly.
#[derive(Debug, Clone, PartialEq)]
pub struct Role {
    /// Inherited `PARTY` state.
    pub party: PartyData,

    /// `time_validity`: `DV_INTERVAL<DV_DATE>` `[0..1]` — valid time
    /// interval for this role.
    ///
    /// TODO(port): `DvInterval<T>`'s `T: DvOrdered` bound and `DvDate`'s
    /// concrete shape are owned by the `data_types` sibling package;
    /// forward-referenced here.
    pub time_validity: Option<DvInterval<crate::data_types::date_time::dv_date::DvDate>>,

    /// `performer`: `PARTY_REF` `[1..1]` — reference to Version container
    /// of Actor playing the role.
    pub performer: openehr_base::identification::party_ref::PartyRef,

    /// `capabilities`: `List<CAPABILITY>` `[0..1]` — the capabilities of
    /// this role.
    pub capabilities: Option<Vec<Capability>>,
}

impl PartyApi for Role {
    fn party_data(&self) -> &PartyData {
        &self.party
    }
}

// TODO(port): invariant as a `Validate` impl:
//   - Capabilities_valid: capabilities /= Void implies not capabilities.empty

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions ROLE — docs/research/spec-cache/RM-1.1.0/uml_classes/role.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/role.adoc §ROLE Class
//   confidence: high
//   todos: 2
//   note: Role is a direct PARTY descendant (sibling of ACTOR, not a subtype of it) — matches Party enum's two top-level variants.
// ─────────────────────────────────────────────
