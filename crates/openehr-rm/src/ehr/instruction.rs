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

// TODO(port): forward-reference — `DV_DATE_TIME` lives in
// rm.data_types.date_time (PORT_MASTER_PLAN.md §7.1), not yet transcribed.
use crate::data_types::date_time::dv_date_time::DvDateTime;

/// Canonical `_type` discriminator string for this class in serialized
/// form.
pub const TYPE_NAME: &str = "INSTRUCTION";

/// `INSTRUCTION` — Entry type used to specify actions in the future.
///
/// `INSTRUCTION` inherits `CARE_ENTRY`, so it embeds
/// [`super::care_entry::CareEntryData`].
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    /// Embedded `CARE_ENTRY` (in turn `ENTRY`/`CONTENT_ITEM`/`LOCATABLE`)
    /// state.
    pub care_entry: super::care_entry::CareEntryData,

    /// `narrative`: mandatory human-readable version of what the
    /// Instruction is about.
    pub narrative: DvText,

    /// `expiry_time`: optional expiry date/time to assist determination of
    /// when an Instruction can be assumed to have expired. This helps
    /// prevent false listing of Instructions as Active when they clearly
    /// must have been terminated in some way or other.
    pub expiry_time: Option<DvDateTime>,

    /// `wf_definition`: optional workflow engine executable expression of
    /// the Instruction.
    pub wf_definition: Option<DvParsable>,

    /// `activities`: list of all activities in Instruction.
    ///
    /// Invariant `Activities_valid`: `activities /= Void implies not
    /// activities.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    pub activities: Option<Vec<super::activity::Activity>>,
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

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/instruction.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / instruction.adoc §INSTRUCTION Class
//   confidence: high
//   todos: 4
//   note: concrete leaf embedding CareEntryData; activities is Vec<Activity> not boxed (ACTIVITY is not itself recursive through INSTRUCTION); Activities_valid invariant left unimplemented; 3 of the 4 markers are forward-reference import comments (DvParsable, DvText, DvDateTime).
// ─────────────────────────────────────────────
