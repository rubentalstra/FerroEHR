//! `CARE_ENTRY` — abstract parent of all clinical `ENTRY` subtypes.
//!
//! openEHR class: `CARE_ENTRY` (abstract), package `rm.ehr.entry`.
//! Inherits: `ENTRY`.
//!
//! The abstract parent of all clinical `ENTRY` subtypes. A `CARE_ENTRY`
//! defines protocol and guideline attributes for all clinical Entry
//! subtypes.
//!
//! `CARE_ENTRY` is abstract with attributes; per ADR-001 §3 it transcribes
//! as an embedded struct ([`CareEntryData`]) plus a marker/accessor trait
//! ([`CareEntryApi`]). `CARE_ENTRY` embeds `ENTRY` (in turn `CONTENT_ITEM`,
//! in turn `LOCATABLE`), so [`CareEntryData`] carries `entry:
//! super::entry::EntryData` as its own first field. Every `CARE_ENTRY`
//! descendant in the RM ([`super::observation::Observation`],
//! [`super::evaluation::Evaluation`], [`super::instruction::Instruction`],
//! [`super::action::Action`]) embeds `CareEntryData` in turn.
use crate::data_structures::item_structure::ItemStructure; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention.
use openehr_base::identification::object_ref::ObjectRef;
use serde::{Deserialize, Serialize};

/// Embedded attribute state of the abstract `CARE_ENTRY` class.
///
/// Per ADR-001 §3, concrete `CARE_ENTRY` descendants embed this struct by
/// composition rather than inheriting from it.
///
/// TODO(port): P4 — `#[serde(flatten)]` on `entry` requires `EntryData`
/// (this same batch) to itself derive `Serialize`/`Deserialize` end-to-end
/// through its own `LocatableData` dependency (sibling P4 wave over
/// `common/`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CareEntryData {
    /// Embedded `ENTRY` (in turn `CONTENT_ITEM`/`LOCATABLE`) state.
    #[serde(flatten)]
    pub entry: super::entry::EntryData,

    /// `protocol`: description of the method (i.e. how) the information in
    /// this entry was arrived at. For `OBSERVATION`s, this is a
    /// description of the method or instrument used. For `EVALUATION`s,
    /// how the evaluation was arrived at. For `INSTRUCTION`s, how to
    /// execute the Instruction. This may take the form of references to
    /// guidelines, including manually followed and executable; knowledge
    /// references such as a paper in Medline; clinical reasons within a
    /// larger care process.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub protocol: Option<ItemStructure>,

    /// `guideline_id`: optional external identifier of guideline creating
    /// this Entry if relevant.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub guideline_id: Option<ObjectRef>,
}

/// Marker/accessor trait shared by every `CARE_ENTRY` descendant.
pub trait CareEntryApi: super::entry::EntryApi {
    /// Access to the embedded [`CareEntryData`].
    fn care_entry_data(&self) -> &CareEntryData;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/care_entry.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / care_entry.adoc §CARE_ENTRY Class
//   confidence: high
//   todos: 2
//   note: abstract-with-attributes per ADR-001 §3 (CareEntryData + CareEntryApi); no attributes/functions/invariants of its own besides protocol/guideline_id; markers are the ItemStructure forward-reference import and the flatten TODO. P4: serde derives added (flatten on entry, Option fields skip-if-none); no _type of its own.
// ─────────────────────────────────────────────
