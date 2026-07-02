//! `GROUP` — a real-world group of parties.
//!
//! openEHR class: `GROUP` (concrete), package `rm.demographic`.
//!
//! A group is a real world group of parties which is created by another
//! party, usually an organisation, for some specific purpose. A typical
//! clinical example is that of the specialist care team, e.g. "cardiology
//! team". The members of the group usually work together.
use super::actor::ActorData;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "GROUP";

/// `GROUP` declares no attributes or invariants of its own beyond what
/// `ACTOR` provides.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// Inherited `ACTOR` state (and transitively `PARTY`).
    pub actor: ActorData,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions GROUP — docs/research/spec-cache/RM-1.1.0/uml_classes/group.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/group.adoc §GROUP Class
//   confidence: high
//   todos: 0
//   note: no attributes/invariants beyond ACTOR.
// ─────────────────────────────────────────────
