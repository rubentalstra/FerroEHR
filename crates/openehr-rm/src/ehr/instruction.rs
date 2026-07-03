//! `INSTRUCTION` — Entry type used to specify actions in the future.
//!
//! openEHR class: `INSTRUCTION`, package `rm.ehr.entry`.
//! Inherits: `CARE_ENTRY`.
//!
//! Used to specify actions in the future. Enables simple and complex
//! specifications to be expressed, including in a fully-computable
//! workflow form. Used for any actionable statement such as medication and
//! therapeutic orders, monitoring, recall and review. Enough details must
//! be provided for the specification to be directly executed by an actor,
//! either human or machine.
//!
//! Not to be used for plan items which are only specified in general
//! terms.
use crate::data_types::encapsulated::dv_parsable::DvParsable; // TODO(port): forward-reference; not yet transcribed.
use crate::data_types::text::dv_text::DvText; // TODO(port): forward-reference; not yet transcribed.
use serde::{Deserialize, Serialize};

// TODO(port): forward-reference — `DV_DATE_TIME` lives in
// rm.data_types.date_time (PORT_MASTER_PLAN.md §7.1), not yet transcribed.
use crate::data_types::date_time::dv_date_time::DvDateTime;
use openehr_foundation::serde_support::{TypeName, TypeTag};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
pub const TYPE_NAME: &str = "INSTRUCTION";

/// `INSTRUCTION` — Entry type used to specify actions in the future.
///
/// `INSTRUCTION` inherits `CARE_ENTRY`, so it embeds
/// [`super::care_entry::CareEntryData`]. `#[serde(flatten)]` folds
/// `CareEntryData` into `INSTRUCTION`'s own JSON object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instruction {
    /// Canonical `_type` discriminator (`"INSTRUCTION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `CARE_ENTRY` (in turn `ENTRY`/`CONTENT_ITEM`/`LOCATABLE`)
    /// state.
    #[serde(flatten)]
    pub care_entry: super::care_entry::CareEntryData,

    /// `narrative`: mandatory human-readable version of what the
    /// Instruction is about.
    pub narrative: DvText,

    /// `expiry_time`: optional expiry date/time to assist determination of
    /// when an Instruction can be assumed to have expired. This helps
    /// prevent false listing of Instructions as Active when they clearly
    /// must have been terminated in some way or other.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expiry_time: Option<DvDateTime>,

    /// `wf_definition`: optional workflow engine executable expression of
    /// the Instruction.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub wf_definition: Option<DvParsable>,

    /// `activities`: list of all activities in Instruction.
    ///
    /// Invariant `Activities_valid`: `activities /= Void implies not
    /// activities.is_empty` — see [`Instruction::invariant_activities_valid`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub activities: Option<Vec<super::activity::Activity>>,
}

impl TypeName for Instruction {
    const NAME: &'static str = TYPE_NAME;
}

impl Instruction {
    /// Invariant `Activities_valid`: `activities /= Void implies not
    /// activities.is_empty` (ADR-003 §8).
    #[must_use]
    pub fn invariant_activities_valid(&self) -> bool {
        self.activities.as_ref().is_none_or(|a| !a.is_empty())
    }
}

impl super::entry::EntryApi for Instruction {
    fn entry_data(&self) -> &super::entry::EntryData {
        &self.care_entry.entry
    }
}

impl super::care_entry::CareEntryApi for Instruction {
    fn care_entry_data(&self) -> &super::care_entry::CareEntryData {
        &self.care_entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::common::generic::party_proxy::{PartyProxy, PartyProxyData};
    use crate::common::generic::party_self::PartySelf;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_text::DvTextData;
    use openehr_base::identification::object_id::ObjectIdData;
    use openehr_base::identification::terminology_id::TerminologyId;

    fn dv_text(value: &str) -> DvText {
        DvText::Text {
            type_tag: TypeTag::new(),
            data: DvTextData {
                value: value.to_string(),
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
        }
    }

    fn code_phrase(terminology: &str, code: &str) -> CodePhrase {
        CodePhrase {
            type_tag: TypeTag::new(),
            terminology_id: TerminologyId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: terminology.to_string(),
                },
            },
            code_string: code.to_string(),
            preferred_term: None,
        }
    }

    fn instruction(activities: Option<Vec<super::super::activity::Activity>>) -> Instruction {
        Instruction {
            type_tag: TypeTag::new(),
            care_entry: super::super::care_entry::CareEntryData {
                entry: super::super::entry::EntryData {
                    content_item: super::super::content_item::ContentItemData {
                        locatable: LocatableData {
                            name: dv_text("Instruction"),
                            archetype_node_id: "at0000".to_string(),
                            uid: None,
                            links: None,
                            archetype_details: None,
                            feeder_audit: None,
                            parent: None,
                        },
                    },
                    language: code_phrase("ISO_639-1", "en"),
                    encoding: code_phrase("IANA_character-sets", "UTF-8"),
                    other_participations: None,
                    workflow_id: None,
                    subject: PartyProxy::PartySelf(PartySelf {
                        type_tag: TypeTag::new(),
                        party_proxy: PartyProxyData { external_ref: None },
                    }),
                    provider: None,
                },
                protocol: None,
                guideline_id: None,
            },
            narrative: dv_text("Take medication daily"),
            expiry_time: None,
            wf_definition: None,
            activities,
        }
    }

    #[test]
    fn activities_valid_rejects_present_but_empty() {
        assert!(instruction(None).invariant_activities_valid()); // None: valid
        assert!(!instruction(Some(Vec::new())).invariant_activities_valid()); // empty: invalid
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/instruction.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / instruction.adoc §INSTRUCTION Class
//   confidence: high
//   todos: 3
//   note: concrete leaf embedding CareEntryData; activities is Vec<Activity> not boxed (ACTIVITY is not itself recursive through INSTRUCTION). P5/ADR-003 §8: Activities_valid invariant implemented (present-implies-non-empty). The 3 remaining TODO(port) are forward-reference import comments (DvParsable, DvText, DvDateTime). P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl; flatten kept on care_entry, Option fields skip-if-none.
// ─────────────────────────────────────────────
