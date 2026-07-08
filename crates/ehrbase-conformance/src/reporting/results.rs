//! The serializable run-results model (design v4): what `results.json` holds
//! and what the Markdown/badge renderers consume. Kept as stable strings so
//! the JSON is a durable, tool-readable record independent of the Rust enum
//! layout.

use serde::{Deserialize, Serialize};

use crate::version::SpecVersions;

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
    /// The outcome of every executed (registered) case × format.
    pub cases: Vec<CaseOutcome>,
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

    /// The number of executed case×format outcomes (the run denominator; the
    /// catalogue denominator lives in `CATALOG.md`).
    #[must_use]
    pub fn executed(&self) -> usize {
        self.cases.len()
    }
}

/// The SUT identity recorded with the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SutIdentity {
    /// The ITS-REST base URL.
    pub base_url: String,
    /// The declared specification versions (a property of the claim).
    #[serde(default)]
    pub versions: SpecVersions,
    /// The declared auth mode (e.g. `"basic (RBAC off)"`).
    pub auth_mode: String,
}

/// The pinned reference corpus this framework was designed against
/// (design-time reading only — recorded for provenance, never consulted at
/// runtime).
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
    /// The ECC id (our own catalogue number, e.g. `"ECC-EHR-005"`) — the
    /// primary public identity (design §3.1, v3.1). Empty only if the
    /// catalogue is missing (guarded against in `tests/coverage.rs`).
    #[serde(default)]
    pub ecc_id: String,
    /// The registration key (our descriptive `<area>/<case>` slug).
    pub id: String,
    /// The human title.
    pub title: String,
    /// The capability label.
    pub capability: String,
    /// The profiles that require the case's capability.
    pub profiles: Vec<String>,
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
    /// The spec citation.
    pub citation: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u128,
}
