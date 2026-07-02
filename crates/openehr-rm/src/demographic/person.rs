//! `PERSON` — a real-world human.
//!
//! openEHR class: `PERSON` (concrete), package `rm.demographic`.
//!
//! Generic description of persons. Provides a dedicated type to which
//! Person archetypes can be targeted.
use super::actor::ActorData;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class, single-sourcing the [`TypeName`] impl below
/// (ADR-002).
pub const TYPE_NAME: &str = "PERSON";

/// `PERSON` declares no attributes or invariants of its own beyond what
/// `ACTOR` provides — it exists purely to give Person archetypes a
/// dedicated concrete target type. `#[serde(flatten)]` folds `ActorData`
/// (and transitively `PartyData`/`LocatableData`) into this struct's own
/// JSON object; per ADR-002 the class self-tags via its first field, and
/// the `Actor`/`Party` enums dispatch on that payload tag untagged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    /// Canonical `_type` discriminator (`"PERSON"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `ACTOR` state (and transitively `PARTY`).
    #[serde(flatten)]
    pub actor: ActorData,
}

impl TypeName for Person {
    const NAME: &'static str = TYPE_NAME;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions PERSON — docs/research/spec-cache/RM-1.1.0/uml_classes/person.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/person.adoc §PERSON Class
//   confidence: high
//   todos: 0
//   note: no attributes/invariants beyond ACTOR; dedicated archetyping target type only. P4/ADR-002: self-tags via TypeTag<Self> first field (TypeName from TYPE_NAME); Actor/Party enums dispatch untagged on this payload tag.
// ─────────────────────────────────────────────
