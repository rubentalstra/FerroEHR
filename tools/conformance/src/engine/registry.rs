//! The case registry: the executable set of registered ECC cases.
//!
//! Every entry is one of **our** cases, publicly identified by its
//! `ECC-<AREA>-<NNN>` number (allocated in `inventory/ecc-catalog.tsv`,
//! guarded by `tests/coverage.rs`). The schedule trace and ITS-REST binding
//! are mandatory fields of [`CaseMeta`] itself
//! ([`crate::model::case::ScheduleTrace`] / [`crate::model::case::Binding`]),
//! so the coverage guard can verify the derivation square on every case.

use std::sync::LazyLock;

use crate::engine::harness::CaseRun;
use crate::model::case::CaseMeta;

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

/// The process-wide registry, built from the suites.
static REGISTRY: LazyLock<Registry> = LazyLock::new(|| Registry {
    entries: crate::suites::entries(),
});

/// The process-wide registry of registered conformance cases.
#[must_use]
pub fn registry() -> &'static Registry {
    &REGISTRY
}
