//! `ACTION` — a clinical action that has been performed.
//!
//! openEHR class: `ACTION`, package `rm.ehr.entry`.
//! Inherits: `CARE_ENTRY`.
//!
//! Used to record a clinical action that has been performed, which may
//! have been ad hoc, or due to the execution of an Activity in an
//! Instruction workflow. Every Action corresponds to a careflow step of
//! some kind or another.
use crate::data_structures::item_structure::ItemStructure; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention.
use crate::data_types::date_time::dv_date_time::DvDateTime; // TODO(port): forward-reference; not yet transcribed.
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
pub const TYPE_NAME: &str = "ACTION";

/// `ACTION` — a clinical action that has been performed.
///
/// `ACTION` inherits `CARE_ENTRY`, so it embeds
/// [`super::care_entry::CareEntryData`]. `#[serde(flatten)]` folds
/// `CareEntryData` into `ACTION`'s own JSON object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    /// Canonical `_type` discriminator (`"ACTION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `CARE_ENTRY` (in turn `ENTRY`/`CONTENT_ITEM`/`LOCATABLE`)
    /// state.
    #[serde(flatten)]
    pub care_entry: super::care_entry::CareEntryData,

    /// `time`: point in time at which this action completed.
    pub time: DvDateTime,

    /// `ism_transition`: details of transition in the Instruction state
    /// machine caused by this Action.
    pub ism_transition: super::ism_transition::IsmTransition,

    /// `instruction_details`: details of the Instruction that caused this
    /// Action to be performed, if there was one.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instruction_details: Option<super::instruction_details::InstructionDetails>,

    /// `description`: description of the action that has been performed,
    /// in the form of an archetyped structure.
    pub description: ItemStructure,
}

impl TypeName for Action {
    const NAME: &'static str = TYPE_NAME;
}

impl super::entry::EntryApi for Action {
    fn entry_data(&self) -> &super::entry::EntryData {
        &self.care_entry.entry
    }
}

impl super::care_entry::CareEntryApi for Action {
    fn care_entry_data(&self) -> &super::care_entry::CareEntryData {
        &self.care_entry
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/action.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / action.adoc §ACTION Class
//   confidence: high
//   todos: 2
//   note: concrete leaf embedding CareEntryData; ism_transition and instruction_details reference the two other PATHABLE-only siblings transcribed in this same batch, both already real (same-crate) types, not forward-ref stubs; both todo markers are forward-reference import comments (ItemStructure, DvDateTime). P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl (no-op struct-level rename removed); flatten on care_entry, instruction_details skip-if-none.
// ─────────────────────────────────────────────
