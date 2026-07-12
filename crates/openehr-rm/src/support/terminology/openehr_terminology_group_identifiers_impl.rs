//! Hand-written spec constants/functions for
//! `OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/`
//! `org.openehr.rm.support.openehr_terminology_group_identifiers.adoc` — the
//! terminology-group identifier constants (each `1..1`) and
//! `valid_terminology_group_id`.

use super::openehr_terminology_group_identifiers::OpenehrTerminologyGroupIdentifiersData;

impl OpenehrTerminologyGroupIdentifiersData {
    /// `Terminology_id_openehr`: the name of the openEHR "terminology".
    pub const TERMINOLOGY_ID_OPENEHR: &'static str = "openehr";

    /// `Group_id_audit_change_type`.
    pub const GROUP_ID_AUDIT_CHANGE_TYPE: &'static str = "audit change type";
    /// `Group_id_attestation_reason`.
    pub const GROUP_ID_ATTESTATION_REASON: &'static str = "attestation reason";
    /// `Group_id_composition_category`.
    pub const GROUP_ID_COMPOSITION_CATEGORY: &'static str = "composition category";
    /// `Group_id_event_math_function`.
    pub const GROUP_ID_EVENT_MATH_FUNCTION: &'static str = "event math function";
    /// `Group_id_instruction_states`.
    pub const GROUP_ID_INSTRUCTION_STATES: &'static str = "instruction states";
    /// `Group_id_instruction_transitions`.
    pub const GROUP_ID_INSTRUCTION_TRANSITIONS: &'static str = "instruction transitions";
    /// `Group_id_null_flavours`.
    pub const GROUP_ID_NULL_FLAVOURS: &'static str = "null flavours";
    /// `Group_id_property`.
    pub const GROUP_ID_PROPERTY: &'static str = "property";
    /// `Group_id_participation_function`.
    pub const GROUP_ID_PARTICIPATION_FUNCTION: &'static str = "participation function";
    /// `Group_id_participation_mode`.
    pub const GROUP_ID_PARTICIPATION_MODE: &'static str = "participation mode";
    /// `Group_id_setting`.
    pub const GROUP_ID_SETTING: &'static str = "setting";
    /// `Group_id_term_mapping_purpose`.
    pub const GROUP_ID_TERM_MAPPING_PURPOSE: &'static str = "term mapping purpose";
    /// `Group_id_subject_relationship`.
    pub const GROUP_ID_SUBJECT_RELATIONSHIP: &'static str = "subject relationship";
    /// `Group_id_version_life_cycle_state`.
    pub const GROUP_ID_VERSION_LIFE_CYCLE_STATE: &'static str = "version lifecycle state";

    /// The complete enumerated group-identifier set (every `Group_id_*`
    /// constant above).
    pub const ALL_GROUP_IDS: &'static [&'static str] = &[
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

    /// `valid_terminology_group_id`: True iff `an_id` is a valid (enumerated)
    /// openEHR terminology group identifier.
    #[must_use]
    pub fn valid_terminology_group_id(an_id: &str) -> bool {
        Self::ALL_GROUP_IDS.contains(&an_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::openehr_terminology_group_identifiers::OpenehrTerminologyGroupIdentifiersData;

    /// The RM class enumerates exactly 14 group ids; `valid_terminology_group_id`
    /// accepts each and nothing else.
    #[test]
    fn group_id_set_is_the_enumerated_fourteen() {
        assert_eq!(
            OpenehrTerminologyGroupIdentifiersData::ALL_GROUP_IDS.len(),
            14
        );
        for id in OpenehrTerminologyGroupIdentifiersData::ALL_GROUP_IDS {
            assert!(
                OpenehrTerminologyGroupIdentifiersData::valid_terminology_group_id(id),
                "{id} must be valid"
            );
        }
        for bad in ["", "openehr", "audit_change_type", "composition"] {
            assert!(
                !OpenehrTerminologyGroupIdentifiersData::valid_terminology_group_id(bad),
                "{bad} must be invalid"
            );
        }
        assert_eq!(
            OpenehrTerminologyGroupIdentifiersData::TERMINOLOGY_ID_OPENEHR,
            "openehr"
        );
    }
}
