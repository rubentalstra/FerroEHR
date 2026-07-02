//! `AGENT` — any non-human, non-organisation actor.
//!
//! openEHR class: `AGENT` (concrete), package `rm.demographic`.
//!
//! Generic concept of any kind of agent, including devices, software
//! systems, but not humans or organisations.
use super::actor::ActorData;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "AGENT";

/// `AGENT` declares no attributes or invariants of its own beyond what
/// `ACTOR` provides.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    /// Inherited `ACTOR` state (and transitively `PARTY`).
    pub actor: ActorData,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions AGENT — docs/research/spec-cache/RM-1.1.0/uml_classes/agent.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/agent.adoc §AGENT Class
//   confidence: high
//   todos: 0
//   note: no attributes/invariants beyond ACTOR; PORT NOTE — spec name AGENT collides conceptually with no Rust std/crate type here, kept verbatim.
// ─────────────────────────────────────────────
