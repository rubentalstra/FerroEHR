//! `ORGANISATION` — a legally constituted body.
//!
//! openEHR class: `ORGANISATION` (concrete), package `rm.demographic`.
//!
//! Generic description of organisations. An organisation is a legally
//! constituted body whose existence (in general) outlives the existence of
//! parties considered to be part of it.
use super::actor::ActorData;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "ORGANISATION";

/// `ORGANISATION` declares no attributes or invariants of its own beyond
/// what `ACTOR` provides.
#[derive(Debug, Clone, PartialEq)]
pub struct Organisation {
    /// Inherited `ACTOR` state (and transitively `PARTY`).
    pub actor: ActorData,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions ORGANISATION — docs/research/spec-cache/RM-1.1.0/uml_classes/organisation.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/organisation.adoc §ORGANISATION Class
//   confidence: high
//   todos: 0
//   note: no attributes/invariants beyond ACTOR.
// ─────────────────────────────────────────────
