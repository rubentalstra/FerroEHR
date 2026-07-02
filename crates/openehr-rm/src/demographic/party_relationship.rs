//! `PARTY_RELATIONSHIP` — a relationship between parties.
//!
//! openEHR class: `PARTY_RELATIONSHIP` (concrete), package
//! `rm.demographic`.
//!
//! Generic description of a relationship between parties.
use crate::common::archetyped::locatable::LocatableData;
use crate::data_types::quantity::dv_interval::DvInterval;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "PARTY_RELATIONSHIP";

/// `PARTY_RELATIONSHIP` inherits `LOCATABLE` directly (not `PARTY`).
#[derive(Debug, Clone, PartialEq)]
pub struct PartyRelationship {
    /// Inherited `LOCATABLE` state.
    pub locatable: LocatableData,

    /// `details`: `ITEM_STRUCTURE` `[0..1]` — the detailed description of
    /// the relationship.
    ///
    /// TODO(port): forward-reference to
    /// `crate::data_structures::item_structure::ItemStructure` (sibling
    /// agent's package).
    pub details: Option<crate::data_structures::item_structure::ItemStructure>,

    /// `target`: `PARTY_REF` `[1..1]` — target of relationship.
    pub target: openehr_base::identification::party_ref::PartyRef,

    /// `time_validity`: `DV_INTERVAL<DV_DATE>` `[0..1]` — valid time
    /// interval for this relationship.
    pub time_validity: Option<DvInterval<crate::data_types::date_time::dv_date::DvDate>>,

    /// `source`: `PARTY_REF` `[1..1]` — source of relationship.
    pub source: openehr_base::identification::party_ref::PartyRef,
}

impl PartyRelationship {
    /// Spec function `type(): DV_TEXT` — type of relationship, such as
    /// "employment", "authority", "health provision". Taken from the
    /// inherited `name` attribute.
    ///
    /// Invariant `Type_validity`: `type = name`.
    ///
    /// TODO(port): implement once `LocatableData.name: DvText` is concrete;
    /// this should simply clone `self.locatable.name`.
    pub fn relationship_type(&self) -> crate::data_types::text::dv_text::DvText {
        todo!("PARTY_RELATIONSHIP.type(): DV_TEXT — clone LocatableData.name once concrete")
    }
}

// TODO(port): invariants as a `Validate` impl:
//   - Source_valid: source /= Void and then source.relationships.has(self)
//   - Target_valid: target /= Void and then
//     not target.reverse_relationships.has(self)
//     — both invariants dereference through an `OBJECT_REF`/`PARTY_REF` to
//     the referenced Party's own `relationships`/`reverse_relationships`
//     lists, which presumes a resolvable object graph/repository not
//     available in a spec-transcription crate; left as `TODO(port)`.
//   - Type_validity: type = name

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions PARTY_RELATIONSHIP — docs/research/spec-cache/RM-1.1.0/uml_classes/party_relationship.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/party_relationship.adoc §PARTY_RELATIONSHIP Class
//   confidence: high
//   todos: 4
//   note: Source_valid/Target_valid invariants require resolving through a Party object graph not modelled in this crate; left as TODO.
// ─────────────────────────────────────────────
