//! `ORGANISATION` — a legally constituted body.
//!
//! openEHR class: `ORGANISATION` (concrete), package `rm.demographic`.
//!
//! Generic description of organisations. An organisation is a legally
//! constituted body whose existence (in general) outlives the existence of
//! parties considered to be part of it.
use super::actor::ActorData;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class, single-sourcing the [`TypeName`] impl below
/// (ADR-002).
pub const TYPE_NAME: &str = "ORGANISATION";

/// `ORGANISATION` declares no attributes or invariants of its own beyond
/// what `ACTOR` provides. `#[serde(flatten)]` folds `ActorData` into this
/// struct's own JSON object; per ADR-002 the class self-tags via its first
/// field, and the `Actor`/`Party` enums dispatch on that payload tag
/// untagged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Organisation {
    /// Canonical `_type` discriminator (`"ORGANISATION"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `ACTOR` state (and transitively `PARTY`).
    #[serde(flatten)]
    pub actor: ActorData,
}

impl TypeName for Organisation {
    const NAME: &'static str = TYPE_NAME;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions ORGANISATION — docs/research/spec-cache/RM-1.1.0/uml_classes/organisation.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/organisation.adoc §ORGANISATION Class
//   confidence: high
//   todos: 0
//   note: no attributes/invariants beyond ACTOR. P4/ADR-002: self-tags via TypeTag<Self> first field (TypeName from TYPE_NAME); Actor/Party enums dispatch untagged on this payload tag.
// ─────────────────────────────────────────────
