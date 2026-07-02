//! `ADDRESS` — address of contact.
//!
//! openEHR class: `ADDRESS` (concrete), package `rm.demographic`.
//!
//! Address of contact, which may be electronic or geographic.
use crate::common::archetyped::locatable::LocatableData;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class, single-sourcing the [`TypeName`] impl below
/// (ADR-002).
pub const TYPE_NAME: &str = "ADDRESS";

/// `ADDRESS` inherits `LOCATABLE` directly. `#[serde(flatten)]` folds
/// `LocatableData` into `ADDRESS`'s own JSON object; per ADR-002 the class
/// self-tags via its first field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Address {
    /// Canonical `_type` discriminator (`"ADDRESS"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `LOCATABLE` state.
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `details`: `ITEM_STRUCTURE` `[1..1]` — archetypable structured
    /// address.
    ///
    /// TODO(port): forward-reference to
    /// `crate::data_structures::item_structure::ItemStructure` (sibling
    /// agent's package).
    pub details: crate::data_structures::item_structure::ItemStructure,
}

impl TypeName for Address {
    const NAME: &'static str = TYPE_NAME;
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
//   note: type() named address_type() to avoid the Rust reserved keyword `type`. P4/ADR-002: self-tags via TypeTag<Self> first field (TypeName from TYPE_NAME); no-op struct-level rename deleted.
// ─────────────────────────────────────────────
