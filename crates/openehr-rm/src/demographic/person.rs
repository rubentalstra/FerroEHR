//! `PERSON` — a real-world human.
//!
//! openEHR class: `PERSON` (concrete), package `rm.demographic`.
//!
//! Generic description of persons. Provides a dedicated type to which
//! Person archetypes can be targeted.
use super::actor::ActorData;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "PERSON";

/// `PERSON` declares no attributes or invariants of its own beyond what
/// `ACTOR` provides — it exists purely to give Person archetypes a
/// dedicated concrete target type.
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    /// Inherited `ACTOR` state (and transitively `PARTY`).
    pub actor: ActorData,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions PERSON — docs/research/spec-cache/RM-1.1.0/uml_classes/person.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/person.adoc §PERSON Class
//   confidence: high
//   todos: 0
//   note: no attributes/invariants beyond ACTOR; dedicated archetyping target type only.
// ─────────────────────────────────────────────
