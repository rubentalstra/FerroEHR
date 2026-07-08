//! The transcribed conformance cases, one module per schedule chapter
//! (design §4.1). The registry is assembled from [`entries`].
//!
//! The suite set grows chapter-by-chapter (design §8): the framework is valuable
//! from the honest zero state onward. Each entry cites its schedule reference and
//! the ITS-REST section its assertions concretize. Chapters not yet transcribed
//! have a module present with an empty (or partial) `entries()` — the coverage
//! guard reports every remaining case as `NotYetTranscribed`.

use crate::registry::CaseEntry;

pub mod content;
pub mod support;

mod admin;
mod composition;
mod contribution;
mod definition_adl14;
mod definition_query;
mod demographic;
mod directory;
mod ehr;
mod query;

/// All implemented case entries, in registration order.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut all = Vec::new();
    all.extend(ehr::entries()); // master06
    all.extend(composition::entries()); // master07
    all.extend(contribution::entries()); // master08
    all.extend(directory::entries()); // master09
    all.extend(definition_adl14::entries()); // master04
    all.extend(definition_query::entries()); // master05
    all.extend(query::entries()); // master11
    all.extend(admin::entries()); // master12 (OPTIONS)
    all.extend(demographic::entries()); // master10 (OPTIONS)
    all.extend(content::entries()); // master15/16/17.x
    all.extend(crate::sign::entries()); // runner-defined SIGN-* (§4.6)
    all
}
