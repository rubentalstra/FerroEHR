//! `GROUP` — a real-world group of parties.
//!
//! openEHR class: `GROUP` (concrete), package `rm.demographic`.
//!
//! A group is a real world group of parties which is created by another
//! party, usually an organisation, for some specific purpose. A typical
//! clinical example is that of the specialist care team, e.g. "cardiology
//! team". The members of the group usually work together.
use super::actor::ActorData;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class, single-sourcing the [`TypeName`] impl below
/// (ADR-002).
pub const TYPE_NAME: &str = "GROUP";

/// `GROUP` declares no attributes or invariants of its own beyond what
/// `ACTOR` provides. `#[serde(flatten)]` folds `ActorData` into this
/// struct's own JSON object; per ADR-002 the class self-tags via its first
/// field, and the `Actor`/`Party` enums dispatch on that payload tag
/// untagged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    /// Canonical `_type` discriminator (`"GROUP"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `ACTOR` state (and transitively `PARTY`).
    #[serde(flatten)]
    pub actor: ActorData,
}

impl TypeName for Group {
    const NAME: &'static str = TYPE_NAME;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions GROUP — docs/research/spec-cache/RM-1.1.0/uml_classes/group.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/group.adoc §GROUP Class
//   confidence: high
//   todos: 0
//   note: no attributes/invariants beyond ACTOR. P4/ADR-002: self-tags via TypeTag<Self> first field (TypeName from TYPE_NAME); Actor/Party enums dispatch untagged on this payload tag.
// ─────────────────────────────────────────────
