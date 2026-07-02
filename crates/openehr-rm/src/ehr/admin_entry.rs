//! `ADMIN_ENTRY` — administrative Entry subtype.
//!
//! openEHR class: `ADMIN_ENTRY`, package `rm.ehr.entry`.
//! Inherits: `ENTRY`.
//!
//! Entry subtype for administrative information, i.e. information about
//! setting up the clinical process, but not itself clinically relevant.
//! Archetypes will define contained information.
//!
//! Used for administrative details of admission, episode, ward location,
//! discharge, appointment (if not stored in a practice management or
//! appointments system).
//!
//! Not to be used for any clinically significant information.
use crate::data_structures::item_structure::ItemStructure; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention.

/// Canonical `_type` discriminator string for this class in serialized
/// form.
pub const TYPE_NAME: &str = "ADMIN_ENTRY";

/// `ADMIN_ENTRY` — administrative Entry subtype.
///
/// `ADMIN_ENTRY` inherits `ENTRY` directly (not `CARE_ENTRY` — it is the
/// non-clinical sibling of the `CARE_ENTRY` subtree), so it embeds
/// [`super::entry::EntryData`] directly rather than
/// [`super::care_entry::CareEntryData`].
#[derive(Debug, Clone, PartialEq)]
pub struct AdminEntry {
    /// Embedded `ENTRY` (in turn `CONTENT_ITEM`/`LOCATABLE`) state.
    pub entry: super::entry::EntryData,

    /// `data`: content of the Admin Entry. The data of the Entry; modelled
    /// in archetypes.
    pub data: ItemStructure,
}

impl super::entry::EntryApi for AdminEntry {
    fn entry_data(&self) -> &super::entry::EntryData {
        &self.entry
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/admin_entry.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / admin_entry.adoc §ADMIN_ENTRY Class
//   confidence: high
//   todos: 1
//   note: concrete leaf embedding EntryData directly (inherits ENTRY, not CARE_ENTRY); no attributes/functions/invariants of its own beyond data; the sole marker is the ItemStructure forward-reference import.
// ─────────────────────────────────────────────
