//! The case registry (design §4.2): the set of implemented cases plus the
//! classification of every schedule id into [`Registration::Implemented`] or
//! [`Registration::Excluded`] with a **structural** reason.
//!
//! The load-bearing invariant is that *every* identified schedule case is
//! classified — the coverage guard (`tests/coverage.rs`) proves it and pins the
//! full inventory to a committed snapshot, so an upstream/re-vendor change fails
//! the build until triaged. Exclusion reasons are structural
//! (not-yet-transcribed / upstream-placeholder / …), never "currently failing":
//! a failing case is a finding, not an exclusion (design §4.5).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::case::{CaseMeta, Chapter};
use crate::harness::CaseRun;
use crate::schedule::InventoryItem;

/// One implemented registry entry: the case metadata plus its run function.
#[derive(Debug, Clone, Copy)]
pub struct CaseEntry {
    /// The static case metadata.
    pub meta: CaseMeta,
    /// The function that executes the case against a SUT.
    pub run: CaseRun,
}

/// Why a schedule case is not an implemented registry entry. Every reason is
/// **structural** — a property of the schedule or of scope, never a masked
/// failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExclusionReason {
    /// A real case not yet transcribed (the honest backlog — design §8 step 1).
    NotYetTranscribed,
    /// A literal upstream placeholder heading (`aaaa`/`bbbb`).
    UpstreamPlaceholder,
    /// A second-or-later occurrence of a duplicated upstream id.
    UpstreamDuplicate,
    /// The schedule chapter is a TBD stub upstream (e.g. master11 prose).
    UpstreamTbd,
    /// Out of scope: not implemented, with the capability named.
    NotImplemented(&'static str),
    /// ADL2/OPT2 provisioning: the server returns an explicit `501` (OPTIONS-only
    /// per the profiles doc). Applied by id prefix `I_DEFINITION_ADL2`.
    Adl2Returns501,
}

impl ExclusionReason {
    /// A stable snake-case label for reports and machine artifacts.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            ExclusionReason::NotYetTranscribed => "not_yet_transcribed".to_owned(),
            ExclusionReason::UpstreamPlaceholder => "upstream_placeholder".to_owned(),
            ExclusionReason::UpstreamDuplicate => "upstream_duplicate".to_owned(),
            ExclusionReason::UpstreamTbd => "upstream_tbd".to_owned(),
            ExclusionReason::NotImplemented(what) => format!("not_implemented:{what}"),
            ExclusionReason::Adl2Returns501 => "adl2_returns_501".to_owned(),
        }
    }
}

/// The classification of one schedule case.
#[derive(Debug, Clone, Copy)]
pub enum Registration<'a> {
    /// The case is implemented by the registry.
    Implemented(&'a CaseEntry),
    /// The case is excluded for a structural reason.
    Excluded(ExclusionReason),
}

/// The static registry of implemented conformance cases.
#[derive(Debug)]
pub struct Registry {
    entries: Vec<CaseEntry>,
}

/// Per-chapter coverage counts (design §4.2), feeding the report.
#[derive(Debug, Clone, Default)]
pub struct ChapterCoverage {
    /// Cases implemented in this chapter.
    pub implemented: usize,
    /// Excluded cases in this chapter, tallied by reason.
    pub excluded: BTreeMap<ExclusionReason, usize>,
}

impl ChapterCoverage {
    /// Cases not yet transcribed in this chapter.
    #[must_use]
    pub fn not_yet(&self) -> usize {
        self.excluded
            .get(&ExclusionReason::NotYetTranscribed)
            .copied()
            .unwrap_or(0)
    }

    /// Total cases in this chapter.
    #[must_use]
    pub fn total(&self) -> usize {
        self.implemented + self.excluded.values().sum::<usize>()
    }
}

/// A whole-schedule coverage summary.
#[derive(Debug, Clone, Default)]
pub struct Coverage {
    /// Per-chapter breakdown, in schedule order.
    pub chapters: BTreeMap<Chapter, ChapterCoverage>,
}

impl Coverage {
    /// The total number of implemented cases across all chapters.
    #[must_use]
    pub fn total_implemented(&self) -> usize {
        self.chapters.values().map(|c| c.implemented).sum()
    }

    /// The total number of cases across all chapters.
    #[must_use]
    pub fn total_cases(&self) -> usize {
        self.chapters.values().map(ChapterCoverage::total).sum()
    }
}

impl Registry {
    /// The implemented case entries.
    #[must_use]
    pub fn entries(&self) -> &[CaseEntry] {
        &self.entries
    }

    /// The implemented entry whose id equals `id`, if any.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&CaseEntry> {
        self.entries.iter().find(|e| e.meta.id == id)
    }

    /// Classify one inventory item (design §4.2). Placeholder and duplicate
    /// occurrences are excluded first (structural); a real first occurrence is
    /// [`Registration::Implemented`] if the registry has it, else excluded by the
    /// structural default (`I_DEFINITION_ADL2*` → 501, otherwise not-yet).
    #[must_use]
    pub fn classify<'a>(&'a self, item: &InventoryItem) -> Registration<'a> {
        if item.placeholder {
            return Registration::Excluded(ExclusionReason::UpstreamPlaceholder);
        }
        if item.duplicate {
            return Registration::Excluded(ExclusionReason::UpstreamDuplicate);
        }
        if let Some(entry) = self.get(&item.key) {
            return Registration::Implemented(entry);
        }
        if item.id.starts_with("I_DEFINITION_ADL2") {
            return Registration::Excluded(ExclusionReason::Adl2Returns501);
        }
        Registration::Excluded(ExclusionReason::NotYetTranscribed)
    }

    /// Compute the per-chapter coverage summary over an inventory.
    #[must_use]
    pub fn coverage(&self, inventory: &[InventoryItem]) -> Coverage {
        let mut coverage = Coverage::default();
        for item in inventory {
            let chapter = coverage.chapters.entry(item.chapter).or_default();
            match self.classify(item) {
                Registration::Implemented(_) => chapter.implemented += 1,
                Registration::Excluded(reason) => {
                    *chapter.excluded.entry(reason).or_insert(0) += 1;
                }
            }
        }
        coverage
    }
}

/// Build the implemented case set. Grows chapter-by-chapter (design §8): the
/// framework is valuable from the honest zero state onward.
fn build_entries() -> Vec<CaseEntry> {
    crate::suites::entries()
}

/// The process-wide registry.
static REGISTRY: LazyLock<Registry> = LazyLock::new(|| Registry {
    entries: build_entries(),
});

/// The process-wide registry of implemented conformance cases.
#[must_use]
pub fn registry() -> &'static Registry {
    &REGISTRY
}
