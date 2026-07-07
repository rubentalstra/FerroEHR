//! master15 — COMPOSITION data-validation truth tables
//! (`master15-content_tc_composition.adoc`).
//!
//! Every CONT-COMP case constrains `COMPOSITION.content` **cardinality** (0..*,
//! 1..*, 3..*, 0..1, 1..1, 3..5) and/or `COMPOSITION.context` **occurrences**
//! (0..* vs 1..1). In the RM `COMPOSITION.content` is `0..*` and
//! `COMPOSITION.context` is `0..1`; a specific interval is expressible **only** in
//! an archetype/OPT. The vendored corpus ships no OPT per cardinality variant
//! (master15 §Implementation notes: the archetypes "should be generated"), so the
//! constraint the case exercises cannot be provisioned and the case is not
//! executable as specified — transcribed + cited, returning `Skipped`
//! ([`drive::skip_archetype`]), never a fabricated pass (design §2.2a, §4.5).

use crate::case::Chapter;
use crate::harness::{CaseFuture, RunContext};
use crate::registry::CaseEntry;

use super::drive::{self, meta};

/// The 12 CONT-COMP case ids (master15), each a `content`/`context` cardinality
/// combination — all archetype constraints (see the module docs).
const CASES: &[&str] = &[
    "CONT-COMP-content_card_any-context_any",
    "CONT-COMP-content_card_1plus-context_any",
    "CONT-COMP-content_card_3plus-context_any",
    "CONT-COMP-content_card_opt-context_any",
    "CONT-COMP-content_card_mand-context_any",
    "CONT-COMP-content_card_3to5-context_any",
    "CONT-COMP-content_card_any-context_mand",
    "CONT-COMP-content_card_1plus-context_mand",
    "CONT-COMP-content_card_3plus-context_mand",
    "CONT-COMP-content_card_opt-context_mand",
    "CONT-COMP-content_card_mand-context_mand",
    "CONT-COMP-content_card_3to5-context_mand",
];

/// The implemented master15 case entries (all 12 CONT-COMP cases).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    CASES
        .iter()
        .map(|&id| CaseEntry {
            meta: meta(id, Chapter::Master15, id),
            run,
        })
        .collect()
}

/// All 12 cases share the same non-executability reason (COMPOSITION
/// content/context cardinality is an archetype constraint); the specific interval
/// is identified by the case id / `schedule_ref`.
fn run<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move {
        drive::skip_archetype("COMPOSITION.content cardinality / COMPOSITION.context occurrences")
    })
}
