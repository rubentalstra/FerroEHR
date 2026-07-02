//! `ACTIVITY` — a single activity within an `INSTRUCTION`.
//!
//! openEHR class: `ACTIVITY`, package `rm.ehr.entry`.
//! Inherits: `LOCATABLE`.
//!
//! Defines a single activity within an Instruction, such as a medication
//! administration.
use crate::common::archetyped::locatable::LocatableData; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention.
use crate::data_structures::item_structure::ItemStructure; // TODO(port): forward-reference; not yet transcribed.
use crate::data_types::encapsulated::dv_parsable::DvParsable; // TODO(port): forward-reference; not yet transcribed.

/// Canonical `_type` discriminator string for this class in serialized
/// form.
pub const TYPE_NAME: &str = "ACTIVITY";

/// `ACTIVITY` — a single activity within an [`super::instruction::Instruction`].
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), `LOCATABLE`'s state is embedded as `pub locatable: LocatableData`
/// rather than simulated via a Rust supertrait.
#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    /// Embedded `LOCATABLE` state.
    pub locatable: LocatableData,

    /// `timing`: timing of the activity, in the form of a parsable string.
    /// If used, the preferred syntax is ISO8601 'R' format, but other
    /// formats may be used including HL7 GTS.
    ///
    /// May be omitted if:
    /// * timing is represented structurally in the `description` attribute
    ///   (e.g. via archetyped elements), or
    /// * unavailable, e.g. imported legacy data; in such cases,
    ///   `INSTRUCTION.narrative` should carry text that indicates the
    ///   timing of its `activities`.
    pub timing: Option<DvParsable>,

    /// `action_archetype_id`: Perl-compliant regular expression pattern,
    /// enclosed in `//` delimiters, indicating the valid identifiers of
    /// archetypes for Actions corresponding to this Activity specification.
    ///
    /// Defaults to `/.*/`, meaning any archetype.
    ///
    /// Invariant `Action_archetype_id_valid`: `not
    /// action_archetype_id.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; default value (`/.*/`) not yet applied by a `Default` impl.
    pub action_archetype_id: String,

    /// `description`: description of the activity, in the form of an
    /// archetyped structure.
    pub description: ItemStructure,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/activity.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / activity.adoc §ACTIVITY Class
//   confidence: high
//   todos: 4
//   note: LOCATABLE parent embedded per ADR-001 §3, matching the sibling ehr_status.rs/ehr_access.rs pattern; 3 markers are forward-reference imports (LocatableData, ItemStructure, DvParsable), plus the action_archetype_id invariant/default-value gap.
// ─────────────────────────────────────────────
