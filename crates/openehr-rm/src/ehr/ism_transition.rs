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
use crate::data_types::text::dv_coded_text::DvCodedText;
use crate::data_types::text::dv_text::DvText;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_term::{
    OpenehrTerminologyGroupIdentifiers, TerminologyAccess, TerminologyCode, TerminologyService,
};
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
    /// (Group_id_instruction_states, current_state.defining_code)` — see
    /// [`IsmTransition::invariant_current_state_valid`].
    pub current_state: DvCodedText,

    /// `transition`: the ISM transition which occurred to arrive in
    /// `current_state`. Coded by openEHR terminology group "Instruction
    /// transitions".
    ///
    /// Invariant `Transition_valid`: `transition /= Void implies
    /// terminology (Terminology_id_openehr).has_code_for_group_id
    /// (Group_id_instruction_transitions, transition.defining_code)` — see
    /// [`IsmTransition::invariant_transition_valid`].
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
    /// Invariant `Current_state_valid`: `terminology (Terminology_id_openehr)
    /// .has_code_for_group_id (Group_id_instruction_states,
    /// current_state.defining_code)`.
    ///
    /// Terminology-bound invariant (ADR-003 §8).
    #[must_use]
    pub fn invariant_current_state_valid(&self, terminology: &TerminologyService) -> bool {
        Self::code_in_group(
            terminology,
            OpenehrTerminologyGroupIdentifiers::GROUP_ID_INSTRUCTION_STATES,
            &self.current_state,
        )
    }

    /// Invariant `Transition_valid`: `transition /= Void implies terminology
    /// (Terminology_id_openehr).has_code_for_group_id
    /// (Group_id_instruction_transitions, transition.defining_code)`.
    ///
    /// Terminology-bound invariant (ADR-003 §8): vacuously true when
    /// `transition` is absent.
    #[must_use]
    pub fn invariant_transition_valid(&self, terminology: &TerminologyService) -> bool {
        self.transition.as_ref().is_none_or(|transition| {
            Self::code_in_group(
                terminology,
                OpenehrTerminologyGroupIdentifiers::GROUP_ID_INSTRUCTION_TRANSITIONS,
                transition,
            )
        })
    }

    /// Shared helper: `terminology(openehr).has_code_for_group_id(group_id,
    /// coded.defining_code)`.
    fn code_in_group(
        terminology: &TerminologyService,
        group_id: &str,
        coded: &DvCodedText,
    ) -> bool {
        terminology
            .terminology(OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR)
            .is_some_and(|access| {
                access.has_code_for_group_id(
                    group_id,
                    &TerminologyCode::new(
                        coded.defining_code.terminology_id.value(),
                        coded.defining_code.code_string.clone(),
                    ),
                )
            })
    }

    // TODO(port): `PATHABLE` functions forwarded via the not-yet-
    // transcribed `Pathable` trait (forward-referenced as
    // `crate::common::pathable::Pathable`); `impl Pathable for
    // IsmTransition` once that trait lands. `parent()` must resolve to
    // `Weak<..>`/index, never an owning back-reference.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_text::DvTextData;
    use openehr_base::identification::object_id::ObjectIdData;
    use openehr_base::identification::terminology_id::TerminologyId;

    fn coded(code: &str) -> DvCodedText {
        DvCodedText {
            type_tag: TypeTag::new(),
            text: DvTextData {
                value: "state".to_string(),
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
            defining_code: CodePhrase {
                type_tag: TypeTag::new(),
                terminology_id: TerminologyId {
                    type_tag: TypeTag::new(),
                    object_id: ObjectIdData {
                        value: "openehr".to_string(),
                    },
                },
                code_string: code.to_string(),
                preferred_term: None,
            },
        }
    }

    #[test]
    fn current_state_valid_checks_the_instruction_states_group() {
        let terminology = TerminologyService::bundled().expect("bundled terminology parses");
        // 245 = "active" in the openEHR "instruction states" group.
        let ism = IsmTransition {
            type_tag: TypeTag::new(),
            current_state: coded("245"),
            transition: None,
            careflow_step: None,
            reason: None,
        };
        assert!(ism.invariant_current_state_valid(terminology));
        // transition = None: vacuously valid.
        assert!(ism.invariant_transition_valid(terminology));

        let bogus = IsmTransition {
            type_tag: TypeTag::new(),
            current_state: coded("999999"),
            transition: None,
            careflow_step: None,
            reason: None,
        };
        assert!(!bogus.invariant_current_state_valid(terminology));
    }

    #[test]
    fn transition_valid_checks_the_instruction_transitions_group() {
        let terminology = TerminologyService::bundled().expect("bundled terminology parses");
        let mut ism = IsmTransition {
            type_tag: TypeTag::new(),
            current_state: coded("245"),
            transition: Some(coded("535")), // "initiate"
            careflow_step: None,
            reason: None,
        };
        assert!(ism.invariant_transition_valid(terminology));

        ism.transition = Some(coded("999999"));
        assert!(!ism.invariant_transition_valid(terminology));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/ism_transition.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / ism_transition.adoc §ISM_TRANSITION Class
//   confidence: high
//   todos: 1
//   note: PATHABLE-not-LOCATABLE settled hazard applied; reason is List<DV_TEXT> per the 1.1.0 attribute table. P5/ADR-003 §8: Current_state_valid and Transition_valid implemented as terminology-bound checks (&TerminologyService, instruction states / instruction transitions groups), pinned by unit tests. The one remaining TODO(port) is the PATHABLE `Pathable`-trait function forwarding, awaiting common::pathable — legitimate cited deferral. P4/ADR-002: self-tagging TypeTag<Self> + TypeName; Option fields skip-if-none.
// ─────────────────────────────────────────────
