//! `CAPABILITY` — capability of a role.
//!
//! openEHR class: `CAPABILITY` (concrete), package `rm.demographic`.
//!
//! Capability of a role, such as "ehr modifier", "health care provider".
//! Capability should be backed up by credentials.
use crate::common::archetyped::locatable::LocatableData;
use crate::data_types::quantity::dv_interval::DvInterval;

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class (serde derives deferred to P4/P5 per ADR-001
/// §Refinements).
pub const TYPE_NAME: &str = "CAPABILITY";

/// `CAPABILITY` inherits `LOCATABLE` directly.
///
/// PORT NOTE: no `Functions`/`Invariants` sections appear in this class's
/// spec table (unlike its `ADDRESS`/`CONTACT`/`PARTY_IDENTITY` siblings,
/// each of which derives a `type()`/`purpose()` function plus a
/// `Xxx_valid: xxx = name` invariant from the inherited `name` attribute).
/// Transcribed literally as-is — no `type()`-style accessor invented for
/// `CAPABILITY`.
#[derive(Debug, Clone, PartialEq)]
pub struct Capability {
    /// Inherited `LOCATABLE` state.
    pub locatable: LocatableData,

    /// `credentials`: `ITEM_STRUCTURE` `[1..1]` — the qualifications of the
    /// performer of the role for this capability. This might include
    /// professional qualifications and official identifications such as
    /// provider numbers etc.
    ///
    /// TODO(port): forward-reference to
    /// `crate::data_structures::item_structure::ItemStructure` (sibling
    /// agent's package).
    pub credentials: crate::data_structures::item_structure::ItemStructure,

    /// `time_validity`: `DV_INTERVAL<DV_DATE>` `[0..1]` — valid time
    /// interval for the credentials of this capability.
    pub time_validity: Option<DvInterval<crate::data_types::date_time::dv_date::DvDate>>,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions CAPABILITY — docs/research/spec-cache/RM-1.1.0/uml_classes/capability.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/capability.adoc §CAPABILITY Class
//   confidence: high
//   todos: 1
//   note: no Functions/Invariants in this class's own table, unlike ADDRESS/CONTACT/PARTY_IDENTITY — no type()-style accessor invented; ambiguity flagged in the transcription report.
// ─────────────────────────────────────────────
