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
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
pub const TYPE_NAME: &str = "EVALUATION";

/// `EVALUATION` — Entry type for evaluation statements.
///
/// `EVALUATION` inherits `CARE_ENTRY`, so it embeds
/// [`super::care_entry::CareEntryData`]. Per the entry-package narrative,
/// the design of `EVALUATION` is deliberately minimal: in addition to the
/// inherited attributes, it declares only `data`. `#[serde(flatten)]` folds
/// `CareEntryData` into `EVALUATION`'s own JSON object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    /// Canonical `_type` discriminator (`"EVALUATION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `CARE_ENTRY` (in turn `ENTRY`/`CONTENT_ITEM`/`LOCATABLE`)
    /// state.
    #[serde(flatten)]
    pub care_entry: super::care_entry::CareEntryData,

    /// `data`: the data of this evaluation, in the form of a spatial data
    /// structure.
    pub data: ItemStructure,
}

impl TypeName for Evaluation {
    const NAME: &'static str = TYPE_NAME;
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
//   note: concrete leaf embedding CareEntryData; single attribute (data: ItemStructure), matching the spec's deliberately minimal design; the sole marker is the ItemStructure forward-reference import. P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl (no-op struct-level rename removed); flatten kept on care_entry.
// ─────────────────────────────────────────────
