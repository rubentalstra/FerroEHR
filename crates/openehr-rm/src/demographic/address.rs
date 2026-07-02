//! `ADDRESS` — address of contact.
//!
//! openEHR class: `ADDRESS` (concrete), package `rm.demographic`.
//!
//! Address of contact, which may be electronic or geographic.
use crate::common::archetyped::locatable::LocatableData;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "ADDRESS";

/// `ADDRESS` inherits `LOCATABLE` directly.
#[derive(Debug, Clone, PartialEq)]
pub struct Address {
    /// Inherited `LOCATABLE` state.
    pub locatable: LocatableData,

    /// `details`: `ITEM_STRUCTURE` `[1..1]` — archetypable structured
    /// address.
    ///
    /// TODO(port): forward-reference to
    /// `crate::data_structures::item_structure::ItemStructure` (sibling
    /// agent's package).
    pub details: crate::data_structures::item_structure::ItemStructure,
}

impl Address {
    /// Spec function `type(): DV_TEXT` — type of address, e.g.
    /// "electronic", "locality". Taken from value of the inherited `name`
    /// attribute.
    ///
    /// Invariant `Type_valid`: `type = name`.
    ///
    /// TODO(port): implement once `LocatableData.name: DvText` is concrete;
    /// this should simply clone `self.locatable.name`.
    pub fn address_type(&self) -> crate::data_types::text::dv_text::DvText {
        todo!("ADDRESS.type(): DV_TEXT — clone LocatableData.name once concrete")
    }
}

// TODO(port): invariant as a `Validate` impl:
//   - Type_valid: type = name

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions ADDRESS — docs/research/spec-cache/RM-1.1.0/uml_classes/address.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/address.adoc §ADDRESS Class
//   confidence: high
//   todos: 3
//   note: type() named address_type() to avoid the Rust reserved keyword `type`.
// ─────────────────────────────────────────────
