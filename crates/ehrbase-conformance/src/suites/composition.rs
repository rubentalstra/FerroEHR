//! master07 — COMPOSITION cases (design §4.1: `suites/composition.rs`).
//!
//! Consumes `compositions/CANONICAL_JSON` + `CANONICAL_XML` fixtures and the
//! `valid_templates` OPTs they reference (a composition commit needs its OPT
//! uploaded first). Not yet transcribed — every master07 case reports as
//! `NotYetTranscribed` via the coverage guard.

use crate::registry::CaseEntry;

/// The implemented master07 case entries (none yet).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    Vec::new()
}
