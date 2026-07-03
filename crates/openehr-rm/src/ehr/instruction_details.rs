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
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
///
/// Being `PATHABLE`-not-`LOCATABLE` changes this class's *fields* (no
/// `LocatableData` embed), not its `_type`: the pinned ITS-JSON schema
/// defines `INSTRUCTION_DETAILS` as a concrete class with its own `_type`
/// const, so it self-tags like every other concrete class.
pub const TYPE_NAME: &str = "INSTRUCTION_DETAILS";

/// `INSTRUCTION_DETAILS` — a reference back to the causing
/// [`super::instruction::Instruction`]/[`super::activity::Activity`], plus
/// optional workflow-engine state, recorded on an
/// [`super::action::Action`]. No `LocatableData` embed (settled
/// `PATHABLE`-not-`LOCATABLE` hazard), so this is a plain struct with no
/// `#[serde(flatten)]` fields.
///
/// TODO(port): P4 — `instruction_id: LocatableRef` needs
/// `openehr_base::identification::locatable_ref::LocatableRef` to derive
/// `Serialize`/`Deserialize` (sibling P4 wave over `openehr-base`, in
/// progress but not yet reaching this specific file as of this pass).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstructionDetails {
    /// Canonical `_type` discriminator (`"INSTRUCTION_DETAILS"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `instruction_id`: reference to causing Instruction.
    pub instruction_id: LocatableRef,

    /// `activity_id`: identifier of Activity within Instruction, in the
    /// form of its archetype path.
    ///
    /// Invariant `Activity_path_valid`: `not activity_id.is_empty` — see
    /// [`InstructionDetails::invariant_activity_path_valid`].
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub wf_details: Option<ItemStructure>,
}

impl TypeName for InstructionDetails {
    const NAME: &'static str = TYPE_NAME;
}

impl InstructionDetails {
    /// Invariant `Activity_path_valid`: `not activity_id.is_empty`
    /// (ADR-003 §8).
    #[must_use]
    pub fn invariant_activity_path_valid(&self) -> bool {
        !self.activity_id.is_empty()
    }

    // TODO(port): `PATHABLE` functions forwarded via the not-yet-
    // transcribed `Pathable` trait (forward-referenced as
    // `crate::common::pathable::Pathable`); `impl Pathable for
    // InstructionDetails` once that trait lands. `parent()` must resolve
    // to `Weak<..>`/index, never an owning back-reference.
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_base::identification::uid_based_id::{UidBasedId, UidBasedIdData};

    fn instruction_details(activity_id: &str) -> InstructionDetails {
        InstructionDetails {
            type_tag: TypeTag::new(),
            instruction_id: LocatableRef {
                type_tag: TypeTag::new(),
                namespace: "local".to_string(),
                r#type: "INSTRUCTION".to_string(),
                id: UidBasedId::HierObjectId(
                    openehr_base::identification::hier_object_id::HierObjectId {
                        type_tag: TypeTag::new(),
                        uid_based_id: UidBasedIdData {
                            value: "8849182c-82ad-4088-a07f-48ead4180515::demo::1".to_string(),
                        },
                    },
                ),
                path: Some("/activities[at0001]".to_string()),
            },
            activity_id: activity_id.to_string(),
            wf_details: None,
        }
    }

    #[test]
    fn activity_path_valid_rejects_empty_activity_id() {
        assert!(instruction_details("/activities[at0001]").invariant_activity_path_valid());
        assert!(!instruction_details("").invariant_activity_path_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/instruction_details.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / instruction_details.adoc §INSTRUCTION_DETAILS Class
//   confidence: high
//   todos: 3
//   note: PATHABLE-not-LOCATABLE settled hazard applied; instruction_id uses the real openehr_base LocatableRef. P5/ADR-003 §8: Activity_path_valid invariant implemented (not is_empty), pinned by a unit test. The 3 remaining TODO(port) are the ItemStructure import comment, the P4 LocatableRef-serde note, and the PATHABLE `Pathable`-trait function forwarding (legitimate cited deferral, awaits common::pathable). P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl; wf_details skip-if-none.
// ─────────────────────────────────────────────
