//! The case registry: the set of registered ECC cases (design v4).
//!
//! The registry is the executable side of the ehrbase-rs Conformance
//! Catalogue: every entry is one of **our** cases, publicly identified by its
//! `ECC-<AREA>-<NNN>` number (allocated in `inventory/ecc-catalog.tsv`,
//! guarded by `tests/coverage.rs`). There is no classification against the
//! legacy CNF corpus — that corpus is design-time reference reading, not a
//! runtime dependency.

use std::sync::LazyLock;

use crate::case::CaseMeta;
use crate::harness::CaseRun;

/// One registered case: the metadata plus its run function.
#[derive(Debug, Clone, Copy)]
pub struct CaseEntry {
    /// The static case metadata.
    pub meta: CaseMeta,
    /// The function that executes the case against a SUT.
    pub run: CaseRun,
}

/// The static registry of registered conformance cases.
#[derive(Debug)]
pub struct Registry {
    entries: Vec<CaseEntry>,
}

impl Registry {
    /// The registered case entries.
    #[must_use]
    pub fn entries(&self) -> &[CaseEntry] {
        &self.entries
    }

    /// The entry whose registration key equals `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&CaseEntry> {
        self.entries.iter().find(|e| e.meta.id == key)
    }
}

/// Build the registered case set from the suites.
fn build_entries() -> Vec<CaseEntry> {
    crate::suites::entries()
}

/// The process-wide registry.
static REGISTRY: LazyLock<Registry> = LazyLock::new(|| Registry {
    entries: build_entries(),
});

/// The process-wide registry of registered conformance cases.
#[must_use]
pub fn registry() -> &'static Registry {
    &REGISTRY
}
