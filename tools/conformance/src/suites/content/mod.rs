//! Content data-validation suites (schedule master15 COMPOSITION + master16
//! ENTRY + master17.x `DATA_VALUE)`: the "author a constraining archetype/OPT,
//! commit truth-table data-set instances, assert accept/reject" chapter
//! (`docs/design/conformance/12-content-composition-entry.md` +
//! `13-data-values.md`; master03-overview §Data Validation Conformance Test
//! Design).
//!
//! Three case modules over shared authoring/driving machinery:
//!
//! - [`composition`] — master15 COMPOSITION.content cardinality ×
//!   COMPOSITION.context occurrences (12 cases, ECC-VAL-001..012).
//! - [`entry`] — master16 OBSERVATION / HISTORY / EVENT / `ITEM_STRUCTURE`
//!   (26 cases, ECC-VAL-013..038).
//! - [`data_types`] — master17.x `DATA_VALUE` leaf value constraints (register
//!   13; authored by the sibling worker; 81 cases, ECC-VAL-039..119).
//!
//! The shared machinery ([`author`], [`drive`], [`mutate`]) realises master15
//! §Implementation notes ("we suggest to automate the archetype/template test
//! cases generation instead of creating each constraint combination manually"):
//! it parses a vendored base OPT into the typed [`openehr_its::opt14`] model,
//! tightens exactly the constraint under test, re-serialises to ADL 1.4 XML,
//! and provisions it — a genuine ingested constraint the SUT builds a
//! `WebTemplate` from, never a fabricated pass. Every authored OPT family +
//! generated composition variant is declared in `testdata/MANIFEST.tsv` as a
//! `generated:` fixture (register 12 G-2/G-8).

use crate::engine::registry::CaseEntry;

pub mod author;
pub mod composition;
pub mod data_types;
pub mod drive;
pub mod entry;
pub mod mutate;

/// Every registered content data-validation case (COMPOSITION + ENTRY +
/// `DATA_VALUE`), in schedule order.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut all = Vec::new();
    all.extend(composition::entries());
    all.extend(entry::entries());
    all.extend(data_types::entries());
    all
}
