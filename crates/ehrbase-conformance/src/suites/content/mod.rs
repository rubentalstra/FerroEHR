//! master15/16/17.x — content (data-validation) truth tables (design §4.1).
//!
//! Table-driven cases that commit mutated compositions/entries/data-types and
//! assert accepted/rejected against the validation service, using the typed
//! fixture mutators ([`mutate`]). Not yet transcribed.

use crate::registry::CaseEntry;

mod composition;
mod data_types;
mod entry;
pub mod mutate;

/// The implemented content case entries (none yet).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut all = Vec::new();
    all.extend(composition::entries());
    all.extend(entry::entries());
    all.extend(data_types::entries());
    all
}
