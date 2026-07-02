//! `PARTY_IDENTITY` — an identity owned by a Party.
//!
//! openEHR class: `PARTY_IDENTITY` (concrete), package `rm.demographic`.
//!
//! An identity "owned" by a Party, such as a person name or company name,
//! and which is used by the Party to identify itself. Actual structure is
//! archetyped.
use crate::common::archetyped::locatable::LocatableData;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "PARTY_IDENTITY";

/// `PARTY_IDENTITY` inherits `LOCATABLE` directly.
#[derive(Debug, Clone, PartialEq)]
pub struct PartyIdentity {
    /// Inherited `LOCATABLE` state.
    pub locatable: LocatableData,

    /// `details`: `ITEM_STRUCTURE` `[1..1]` — the value of the identity.
    /// This will often take the form of a parseable string or a small
    /// structure of strings.
    ///
    /// TODO(port): forward-reference to
    /// `crate::data_structures::item_structure::ItemStructure` (sibling
    /// agent's package).
    pub details: crate::data_structures::item_structure::ItemStructure,
}

impl PartyIdentity {
    /// Spec function `purpose(): DV_TEXT` — purpose of identity, e.g.
    /// "legal", "stagename", "nickname", "tribal name", "trading name".
    /// Taken from value of the inherited `name` attribute.
    ///
    /// Invariant `Purpose_valid`: `purpose = name`.
    ///
    /// TODO(port): implement once `LocatableData.name: DvText` is concrete;
    /// this should simply clone `self.locatable.name`.
    pub fn purpose(&self) -> crate::data_types::text::dv_text::DvText {
        todo!("PARTY_IDENTITY.purpose(): DV_TEXT — clone LocatableData.name once concrete")
    }
}

// TODO(port): invariant as a `Validate` impl:
//   - Purpose_valid: purpose = name

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions PARTY_IDENTITY — docs/research/spec-cache/RM-1.1.0/uml_classes/party_identity.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/party_identity.adoc §PARTY_IDENTITY Class
//   confidence: high
//   todos: 3
//   note: details is REQUIRED (1..1) per the spec table, unlike ADDRESS/CAPABILITY's own required-details siblings.
// ─────────────────────────────────────────────
