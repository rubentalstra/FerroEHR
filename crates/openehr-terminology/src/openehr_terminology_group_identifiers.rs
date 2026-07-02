//! `OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS` — list of identifiers for groups
//! in the openEHR terminology.
//!
//! openEHR class: `OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS` (RM 1.1.0,
//! `rm.support.terminology`). Constants-only class → zero-sized struct with
//! associated consts and fns, per the P1 precedent (`TimeDefinitions`,
//! `BASIC_DEFINITIONS`); `TERMINOLOGY_SERVICE`'s `Inherit` of this class is
//! realised as direct `OpenehrTerminologyGroupIdentifiers::*` calls.

/// `OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS` (constants-only spec class).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenehrTerminologyGroupIdentifiers;

impl OpenehrTerminologyGroupIdentifiers {
    /// Spec constant `Terminology_id_openehr` — name of openEHR's own
    /// terminology.
    pub const TERMINOLOGY_ID_OPENEHR: &'static str = "openehr";

    /// Spec constant `Group_id_audit_change_type`.
    pub const GROUP_ID_AUDIT_CHANGE_TYPE: &'static str = "audit change type";
    /// Spec constant `Group_id_attestation_reason`.
    pub const GROUP_ID_ATTESTATION_REASON: &'static str = "attestation reason";
    /// Spec constant `Group_id_composition_category`.
    pub const GROUP_ID_COMPOSITION_CATEGORY: &'static str = "composition category";
    /// Spec constant `Group_id_event_math_function`.
    pub const GROUP_ID_EVENT_MATH_FUNCTION: &'static str = "event math function";
    /// Spec constant `Group_id_instruction_states`.
    pub const GROUP_ID_INSTRUCTION_STATES: &'static str = "instruction states";
    /// Spec constant `Group_id_instruction_transitions`.
    pub const GROUP_ID_INSTRUCTION_TRANSITIONS: &'static str = "instruction transitions";
    /// Spec constant `Group_id_null_flavours`.
    pub const GROUP_ID_NULL_FLAVOURS: &'static str = "null flavours";
    /// Spec constant `Group_id_property`.
    pub const GROUP_ID_PROPERTY: &'static str = "property";
    /// Spec constant `Group_id_participation_function`.
    pub const GROUP_ID_PARTICIPATION_FUNCTION: &'static str = "participation function";
    /// Spec constant `Group_id_participation_mode`.
    pub const GROUP_ID_PARTICIPATION_MODE: &'static str = "participation mode";
    /// Spec constant `Group_id_setting`.
    pub const GROUP_ID_SETTING: &'static str = "setting";
    /// Spec constant `Group_id_term_mapping_purpose`.
    pub const GROUP_ID_TERM_MAPPING_PURPOSE: &'static str = "term mapping purpose";
    /// Spec constant `Group_id_subject_relationship`.
    pub const GROUP_ID_SUBJECT_RELATIONSHIP: &'static str = "subject relationship";
    /// Spec constant `Group_id_version_life_cycle_state`. The constant's
    /// spec name says `life_cycle` while its value says `lifecycle` — both
    /// preserved exactly as published.
    pub const GROUP_ID_VERSION_LIFE_CYCLE_STATE: &'static str = "version lifecycle state";

    /// The 14 group identifiers above, for membership checks.
    /// PORT NOTE: helper, not a spec member.
    const ALL_GROUP_IDS: [&'static str; 14] = [
        Self::GROUP_ID_AUDIT_CHANGE_TYPE,
        Self::GROUP_ID_ATTESTATION_REASON,
        Self::GROUP_ID_COMPOSITION_CATEGORY,
        Self::GROUP_ID_EVENT_MATH_FUNCTION,
        Self::GROUP_ID_INSTRUCTION_STATES,
        Self::GROUP_ID_INSTRUCTION_TRANSITIONS,
        Self::GROUP_ID_NULL_FLAVOURS,
        Self::GROUP_ID_PROPERTY,
        Self::GROUP_ID_PARTICIPATION_FUNCTION,
        Self::GROUP_ID_PARTICIPATION_MODE,
        Self::GROUP_ID_SETTING,
        Self::GROUP_ID_TERM_MAPPING_PURPOSE,
        Self::GROUP_ID_SUBJECT_RELATIONSHIP,
        Self::GROUP_ID_VERSION_LIFE_CYCLE_STATE,
    ];

    /// Spec function `valid_terminology_group_id(an_id): Boolean` — validity
    /// function to test if an identifier is in the set defined by this class.
    ///
    /// PORT NOTE: the published table types the `an_id` parameter as
    /// `Boolean`; that is an evident editorial defect (the twin
    /// `valid_code_set_id` types it `String`), transcribed here as `&str`
    /// and flagged rather than silently ignored.
    #[must_use]
    pub fn valid_terminology_group_id(an_id: &str) -> bool {
        Self::ALL_GROUP_IDS.contains(&an_id)
    }
}

#[cfg(test)]
mod tests {
    use super::OpenehrTerminologyGroupIdentifiers as Ids;

    #[test]
    fn validity_function_accepts_exactly_the_published_set() {
        assert!(Ids::valid_terminology_group_id("audit change type"));
        assert!(Ids::valid_terminology_group_id("version lifecycle state"));
        assert!(!Ids::valid_terminology_group_id("openehr"));
        assert!(!Ids::valid_terminology_group_id("no such group"));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support §terminology_package — docs/research/spec-cache/RM-1.1.0/support/uml_classes/openehr_terminology_group_identifiers.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: openehr_terminology_group_identifiers.adoc (15 constants + 1 function)
//   confidence: high
//   todos: 0
//   note: spec table's Boolean-typed an_id parameter is an editorial defect, transcribed as &str with a PORT NOTE
// ─────────────────────────────────────────────
