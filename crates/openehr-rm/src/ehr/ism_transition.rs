//! `ISM_TRANSITION` — a transition in the Instruction State Machine.
//!
//! openEHR class: `ISM_TRANSITION`, package `rm.ehr.entry`.
//! Inherits: `PATHABLE`.
//!
//! Model of a transition in the Instruction State Machine, caused by a
//! careflow step. The attributes document the careflow step as well as
//! the ISM transition.
//!
//! # `PATHABLE`, not `LOCATABLE` (settled hazard)
//!
//! `ISM_TRANSITION` inherits `PATHABLE` directly
//! (`docs/research/spec-cache/RM-1.1.0/uml_classes/ism_transition.adoc`
//! §Inherit), **not** `LOCATABLE`. See the identical note on
//! [`super::event_context::EventContext`] for the full `PATHABLE`-vs-
//! `LOCATABLE` reasoning; the short version: `PATHABLE` is attribute-free
//! (transcribes as a trait per ADR-001 §1), so there is **no**
//! `LocatableData` embed and **no** `uid`/`name`/`archetype_node_id`
//! fields on this struct — a settled hazard, not to be relitigated
//! (`.claude/rules/rm-transcription.md`).
use crate::data_types::text::dv_coded_text::DvCodedText; // TODO(port): forward-reference; not yet transcribed.
use crate::data_types::text::dv_text::DvText; // TODO(port): forward-reference; not yet transcribed.
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
///
/// Being `PATHABLE`-not-`LOCATABLE` changes this class's *fields* (no
/// `LocatableData` embed), not its `_type`: the pinned ITS-JSON schema
/// defines `ISM_TRANSITION` as a concrete class with its own `_type`
/// const, so it self-tags like every other concrete class.
pub const TYPE_NAME: &str = "ISM_TRANSITION";

/// `ISM_TRANSITION` — the ISM state, transition, and careflow step
/// recorded by an [`super::action::Action`]. No `LocatableData` embed
/// (settled `PATHABLE`-not-`LOCATABLE` hazard), so this is a plain struct
/// with no `#[serde(flatten)]` fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsmTransition {
    /// Canonical `_type` discriminator (`"ISM_TRANSITION"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `current_state`: the ISM current state. Coded by openEHR
    /// terminology group "Instruction states".
    ///
    /// Invariant `Current_state_valid`: `terminology
    /// (Terminology_id_openehr).has_code_for_group_id
    /// (Group_id_instruction_states, current_state.defining_code)`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    pub current_state: DvCodedText,

    /// `transition`: the ISM transition which occurred to arrive in
    /// `current_state`. Coded by openEHR terminology group "Instruction
    /// transitions".
    ///
    /// Invariant `Transition_valid`: `transition /= Void implies
    /// terminology (Terminology_id_openehr).has_code_for_group_id
    /// (Group_id_instruction_transitions, transition.defining_code)`.
    ///
    /// TODO(port): invariant not yet enforced.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transition: Option<DvCodedText>,

    /// `careflow_step`: the step in the careflow process which occurred
    /// as part of generating this action, e.g. "dispense",
    /// "start_administration". This attribute represents the clinical
    /// label for the activity, as opposed to `current_state` which
    /// represents the state machine (ISM) computable form. Defined in
    /// archetype.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub careflow_step: Option<DvCodedText>,

    /// `reason`: optional possibility of adding one or more reasons for
    /// this careflow step having been taken. Multiple reasons may occur
    /// in medication management for example.
    ///
    /// RM 1.1.0 attribute table gives this as `List<DV_TEXT>` (cardinality
    /// `0..1` on the list itself), transcribed as `Option<Vec<DvText>>`
    /// per the crate's `List<T>` convention.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<Vec<DvText>>,
}

impl TypeName for IsmTransition {
    const NAME: &'static str = TYPE_NAME;
}

impl IsmTransition {
    // TODO(port): `PATHABLE` functions forwarded via the not-yet-
    // transcribed `Pathable` trait (forward-referenced as
    // `crate::common::pathable::Pathable`); `impl Pathable for
    // IsmTransition` once that trait lands. `parent()` must resolve to
    // `Weak<..>`/index, never an owning back-reference.
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/ism_transition.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / ism_transition.adoc §ISM_TRANSITION Class
//   confidence: high
//   todos: 5
//   note: PATHABLE-not-LOCATABLE settled hazard applied; reason is List<DV_TEXT> per the 1.1.0 attribute table and prose ("An optional _reason_ property (of type List<DV_TEXT>)"), not a bare DvText; two terminology-binding invariants and the Pathable-function forwarding left unimplemented, plus 2 forward-reference imports. P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl (no-op struct-level rename removed) — PATHABLE-only changes fields, not _type; Option fields skip-if-none.
// ─────────────────────────────────────────────
