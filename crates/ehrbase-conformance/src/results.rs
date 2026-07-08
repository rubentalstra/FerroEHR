//! The serializable run-results model (design §4.5): what `results.json` holds
//! and what the Markdown/badge renderers consume. Kept as stable strings (chapter
//! labels, reason labels, status names) so the JSON is a durable, tool-readable
//! record independent of the Rust enum layout.

use serde::{Deserialize, Serialize};

/// The full result set of one conformance run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResults {
    /// The SUT under test.
    pub sut: SutIdentity,
    /// The pinned CNF corpus the schedule came from.
    pub corpus: CorpusPin,
    /// When the run started (ISO 8601).
    pub started: String,
    /// The selection that scoped this run.
    pub selection: SelectionInfo,
    /// The outcome of every executed (implemented) case × format.
    pub cases: Vec<CaseOutcome>,
    /// The classification of every identified schedule case (the honest total
    /// coverage — implemented and excluded alike).
    pub inventory: Vec<InventoryClass>,
}

impl RunResults {
    /// The process exit code implied by these results: `2` if any case errored
    /// (runner/SUT fault), `1` if any failed (conformance finding), else `0`.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        if self.cases.iter().any(|c| c.status == CaseStatus::Errored) {
            2
        } else {
            i32::from(self.cases.iter().any(|c| c.status == CaseStatus::Failed))
        }
    }

    /// Cases that passed.
    #[must_use]
    pub fn passed(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| c.status == CaseStatus::Passed)
            .count()
    }

    /// Cases that failed.
    #[must_use]
    pub fn failed(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| c.status == CaseStatus::Failed)
            .count()
    }

    /// The number of identified schedule cases (the coverage denominator).
    #[must_use]
    pub fn identified(&self) -> usize {
        self.inventory.len()
    }
}

/// The SUT identity recorded with the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SutIdentity {
    /// The ITS-REST base URL.
    pub base_url: String,
    /// The declared RM version (a property of the claim, not a deviation — §2.1).
    pub rm_version: String,
    /// The declared auth mode (e.g. `"basic (RBAC off)"`).
    pub auth_mode: String,
}

/// The pinned CNF corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusPin {
    /// The upstream repository.
    pub repo: String,
    /// The pinned commit.
    pub commit: String,
}

impl Default for CorpusPin {
    fn default() -> Self {
        Self {
            repo: "openEHR/specifications-CNF".to_owned(),
            commit: "33251d2a".to_owned(),
        }
    }
}

/// The selection that scoped the run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelectionInfo {
    /// The id-substring filter, if any.
    pub filter: Option<String>,
    /// The profile filter, if any.
    pub profile: Option<String>,
    /// The formats run.
    pub formats: Vec<String>,
}

/// The status of one executed case × format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseStatus {
    /// All data sets passed.
    Passed,
    /// A conformance assertion failed.
    Failed,
    /// A runner/SUT error (transport) — not a conformance finding.
    Errored,
    /// The case was skipped for a stated reason.
    Skipped,
}

/// The outcome of one executed case in one format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseOutcome {
    /// The case id.
    pub id: String,
    /// The chapter label (e.g. `"master06"`).
    pub chapter: String,
    /// The capability label.
    pub capability: String,
    /// The profiles that require the case's capability.
    pub profiles: Vec<String>,
    /// The provenance label.
    pub provenance: String,
    /// The wire format this outcome is for.
    pub format: String,
    /// The status.
    pub status: CaseStatus,
    /// Data sets that passed.
    pub passed_data_sets: u32,
    /// Data sets attempted.
    pub total_data_sets: u32,
    /// The failure/skip message, if any.
    pub message: Option<String>,
    /// The schedule reference.
    pub schedule_ref: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u128,
}

/// The classification of one identified schedule case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryClass {
    /// The stable classification key.
    pub key: String,
    /// The raw case id.
    pub id: String,
    /// The chapter label.
    pub chapter: String,
    /// `"implemented"` or an exclusion-reason label.
    pub kind: String,
}

impl InventoryClass {
    /// Whether this entry is implemented.
    #[must_use]
    pub fn is_implemented(&self) -> bool {
        self.kind == "implemented"
    }
}
