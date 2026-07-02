//! `EVALUATION` — Entry type for evaluation statements.
//!
//! openEHR class: `EVALUATION`, package `rm.ehr.entry`.
//! Inherits: `CARE_ENTRY`.
//!
//! Entry type for evaluation statements. Used for all kinds of statements
//! which evaluate other information, such as interpretations of
//! observations, diagnoses, differential diagnoses, hypotheses, risk
//! assessments, goals and plans.
//!
//! Should not be used for actionable statements such as medication orders
//! — these are represented using the `INSTRUCTION` type.
use crate::data_structures::item_structure::ItemStructure; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention.

/// Canonical `_type` discriminator string for this class in serialized
/// form.
pub const TYPE_NAME: &str = "EVALUATION";

/// `EVALUATION` — Entry type for evaluation statements.
///
/// `EVALUATION` inherits `CARE_ENTRY`, so it embeds
/// [`super::care_entry::CareEntryData`]. Per the entry-package narrative,
/// the design of `EVALUATION` is deliberately minimal: in addition to the
/// inherited attributes, it declares only `data`.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// Embedded `CARE_ENTRY` (in turn `ENTRY`/`CONTENT_ITEM`/`LOCATABLE`)
    /// state.
    pub care_entry: super::care_entry::CareEntryData,

    /// `data`: the data of this evaluation, in the form of a spatial data
    /// structure.
    pub data: ItemStructure,
}

impl super::entry::EntryApi for Evaluation {
    fn entry_data(&self) -> &super::entry::EntryData {
        &self.care_entry.entry
    }
}

impl super::care_entry::CareEntryApi for Evaluation {
    fn care_entry_data(&self) -> &super::care_entry::CareEntryData {
        &self.care_entry
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/evaluation.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / evaluation.adoc §EVALUATION Class
//   confidence: high
//   todos: 1
//   note: concrete leaf embedding CareEntryData; single attribute (data: ItemStructure), matching the spec's deliberately minimal design; the sole marker is the ItemStructure forward-reference import.
// ─────────────────────────────────────────────
