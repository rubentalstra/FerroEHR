//! `OBSERVATION` — Entry subtype for objective/patient-reported clinical
//! data in the past or present.
//!
//! openEHR class: `OBSERVATION`, package `rm.ehr.entry`.
//! Inherits: `CARE_ENTRY`.
//!
//! Entry subtype for all clinical data in the past or present, i.e. which
//! (by the time it is recorded) has already occurred. `OBSERVATION` data
//! is expressed using the class `HISTORY<T>`, which guarantees that it is
//! situated in time. `OBSERVATION` is used for all notionally objective
//! (i.e. measured in some way) observations of phenomena, and
//! patient-reported phenomena, e.g. pain.
//!
//! Not to be used for recording opinion or future statements of any kind,
//! including instructions, intentions, plans etc.
use crate::data_structures::history::History; // TODO(port): forward-reference; not yet transcribed. Generic HISTORY<T: ITEM_STRUCTURE> per ADR-001 §5 / PORT_MASTER_PLAN.md §7.2.
use crate::data_structures::item_structure::ItemStructure; // TODO(port): forward-reference; not yet transcribed.

/// Canonical `_type` discriminator string for this class in serialized
/// form.
pub const TYPE_NAME: &str = "OBSERVATION";

/// `OBSERVATION` — Entry subtype for objective/patient-reported clinical
/// data in the past or present.
///
/// `OBSERVATION` inherits `CARE_ENTRY`, so it embeds
/// [`super::care_entry::CareEntryData`].
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    /// Embedded `CARE_ENTRY` (in turn `ENTRY`/`CONTENT_ITEM`/`LOCATABLE`)
    /// state.
    pub care_entry: super::care_entry::CareEntryData,

    /// `data`: the data of this observation, in the form of a history of
    /// values which may be of any complexity.
    pub data: History<ItemStructure>,

    /// `state`: optional recording of the state of subject of this
    /// observation during the observation process, in the form of a
    /// separate history of values which may be of any complexity. State
    /// may also be recorded within the History of the `data` attribute.
    pub state: Option<History<ItemStructure>>,
}

impl super::entry::EntryApi for Observation {
    fn entry_data(&self) -> &super::entry::EntryData {
        &self.care_entry.entry
    }
}

impl super::care_entry::CareEntryApi for Observation {
    fn care_entry_data(&self) -> &super::care_entry::CareEntryData {
        &self.care_entry
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/observation.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / observation.adoc §OBSERVATION Class
//   confidence: high
//   todos: 2
//   note: concrete leaf embedding CareEntryData; data/state both History<ItemStructure> per the published table (not a bare ItemStructure) — both markers are forward-reference imports (History, ItemStructure) pending data_structures transcription.
// ─────────────────────────────────────────────
