//! `INSTRUCTION_DETAILS` — details of the Instruction causing an Action.
//!
//! openEHR class: `INSTRUCTION_DETAILS`, package `rm.ehr.entry`.
//! Inherits: `PATHABLE`.
//!
//! Used to record details of the Instruction causing an [`super::action::Action`].
//!
//! # `PATHABLE`, not `LOCATABLE` (settled hazard)
//!
//! `INSTRUCTION_DETAILS` inherits `PATHABLE` directly
//! (`docs/research/spec-cache/RM-1.1.0/uml_classes/instruction_details.adoc`
//! §Inherit), **not** `LOCATABLE`. See the identical note on
//! [`super::event_context::EventContext`] for the full `PATHABLE`-vs-
//! `LOCATABLE` reasoning; the short version: `PATHABLE` is attribute-free
//! (transcribes as a trait per ADR-001 §1), so there is **no**
//! `LocatableData` embed and **no** `uid`/`name`/`archetype_node_id`
//! fields on this struct — a settled hazard, not to be relitigated
//! (`.claude/rules/rm-transcription.md`).
use crate::data_structures::item_structure::ItemStructure; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention (data_structures has no UML subpackage grouping, unlike data_types).
use openehr_base::identification::locatable_ref::LocatableRef;

/// Canonical `_type` discriminator string for this class in serialized
/// form.
pub const TYPE_NAME: &str = "INSTRUCTION_DETAILS";

/// `INSTRUCTION_DETAILS` — a reference back to the causing
/// [`super::instruction::Instruction`]/[`super::activity::Activity`], plus
/// optional workflow-engine state, recorded on an
/// [`super::action::Action`].
#[derive(Debug, Clone, PartialEq)]
pub struct InstructionDetails {
    /// `instruction_id`: reference to causing Instruction.
    pub instruction_id: LocatableRef,

    /// `activity_id`: identifier of Activity within Instruction, in the
    /// form of its archetype path.
    ///
    /// Invariant `Activity_path_valid`: `not activity_id.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    pub activity_id: String,

    /// `wf_details`: various workflow engine state details, potentially
    /// including such things as:
    ///
    /// * condition that fired to cause this Action to be done (with
    ///   actual variables substituted);
    /// * list of notifications which actually occurred (with all
    ///   variables substituted);
    /// * other workflow engine state.
    ///
    /// This specification does not currently define the actual structure
    /// or semantics of this field.
    pub wf_details: Option<ItemStructure>,
}

impl InstructionDetails {
    // TODO(port): `PATHABLE` functions forwarded via the not-yet-
    // transcribed `Pathable` trait (forward-referenced as
    // `crate::common::pathable::Pathable`); `impl Pathable for
    // InstructionDetails` once that trait lands. `parent()` must resolve
    // to `Weak<..>`/index, never an owning back-reference.
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/instruction_details.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / instruction_details.adoc §INSTRUCTION_DETAILS Class
//   confidence: high
//   todos: 3
//   note: PATHABLE-not-LOCATABLE settled hazard applied; instruction_id uses the real openehr_base::identification::locatable_ref::LocatableRef (already transcribed in P1), not a forward-ref stub; the 3 markers are the ItemStructure import, the activity_id invariant, and the Pathable-function forwarding.
// ─────────────────────────────────────────────
