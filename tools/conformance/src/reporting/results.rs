//! The serializable run-results model (design v4): what `results.json` holds
//! and what the Markdown/badge renderers consume. Kept as stable strings so
//! the JSON is a durable, tool-readable record independent of the Rust enum
//! layout.

use serde::{Deserialize, Serialize};

use crate::edition::EditionPolicy;
use crate::model::versions::SpecVersions;
use crate::sut::descriptor::SutKind;

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
    /// The terminology server the run had available + the recorded FHIR-tx
    /// exchange (B4 `TS` area). Absent when the run had no terminology server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminology: Option<TerminologyRun>,
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

    /// Cases reported as not applicable to this SUT (an adjudicated extension /
    /// RM-version-sensitive route — excluded from pass/fail and capability
    /// math, §3a.3).
    #[must_use]
    pub fn not_applicable(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| c.status == CaseStatus::NotApplicable)
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
    /// The product under test (name, version, image digest) — first-class so a
    /// run's artifacts unambiguously state *which server* they measured (§3a.1).
    /// `#[serde(default)]` keeps pre-X1 `results.json` files readable via
    /// `report --from`.
    #[serde(default)]
    pub product: ProductIdentity,
    /// The SUT class (ours vs foreign) — gates Certificate emission and the
    /// fairness register.
    #[serde(default = "default_sut_kind")]
    pub kind: SutKind,
    /// The edition policy the run executed under (pinned for our CI, auto
    /// for bring-your-own-endpoint targets).
    #[serde(default = "default_edition_policy")]
    pub edition_policy: EditionPolicy,
    /// The declared specification versions (a property of the claim).
    #[serde(default)]
    pub versions: SpecVersions,
    /// The declared auth mode (e.g. `"basic (RBAC off)"`).
    pub auth_mode: String,
}

impl SutIdentity {
    /// Whether the SUT is this project's own product (`ehrbase-rs`, case-
    /// insensitive). The Conformance Statement + Certificate are self-assessment
    /// artifacts emitted only for our own product (§3a.2), and the upstream
    /// adjudication register is consulted only for non-self SUTs (§3a.4).
    #[must_use]
    pub fn is_ehrbase_rs(&self) -> bool {
        self.product.is_ehrbase_rs()
    }
}

/// The product under test: what server, which version, which image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductIdentity {
    /// The product name (e.g. `"ehrbase-rs"`, `"ehrbase-java"`).
    pub name: String,
    /// The product version (e.g. the workspace version, or `"2.34.0"`).
    pub version: String,
    /// The container image digest (`sha256:…`) when the run targeted a pinned
    /// image, else absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_digest: Option<String>,
}

impl ProductIdentity {
    /// This project's own product name — the identity every existing run
    /// defaults to, so our baseline stays stable.
    pub const EHRBASE_RS: &'static str = "ehrbase-rs";

    /// Whether this identity is our own product (`ehrbase-rs`, case-insensitive).
    #[must_use]
    pub fn is_ehrbase_rs(&self) -> bool {
        self.name.eq_ignore_ascii_case(Self::EHRBASE_RS)
    }
}

impl Default for ProductIdentity {
    /// Defaults to `ehrbase-rs @ <this crate's version>` (= the workspace
    /// version), so a run with no `--sut-*` flags — and any pre-X1
    /// `results.json` reparsed via `report --from` — identifies as our own
    /// product exactly as before.
    fn default() -> Self {
        Self {
            name: Self::EHRBASE_RS.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            image_digest: None,
        }
    }
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

/// The terminology server a run had available (B4 `TS` area) plus the recorded
/// FHIR-tx exchange — "recording the wiremock exchange in the report".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminologyRun {
    /// The FHIR R4 base URL the harness targeted.
    pub base_url: String,
    /// The server mode: `"fixture"` (the spun-up `wiremock` FHIR-tx) or
    /// `"real"` (`--tx-server-url`).
    pub mode: String,
    /// The recorded FHIR-tx exchange (received requests). For the fixture this
    /// is the harness's own liveness self-check plus anything a SUT wired to it
    /// sent; for a real server it is empty (the runner cannot observe a remote
    /// server's inbound requests).
    #[serde(default)]
    pub exchanges: Vec<TxExchange>,
}

/// One recorded FHIR-tx request against the terminology server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxExchange {
    /// The HTTP method.
    pub method: String,
    /// The request path (e.g. `/ValueSet/$expand`).
    pub path: String,
    /// The raw query string, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
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
    /// The case is not applicable to this SUT — an adjudicated extension route
    /// or an RM-version-sensitive comparison the SUT cannot be expected to
    /// satisfy (§3a.3/§3a.4). Excluded from pass/fail counts and from
    /// capability computation; reported in its own section. Distinct from
    /// `Skipped` (which is an in-run "could not determine" from a case's own
    /// probe), this is a committed, cited adjudication about the SUT.
    NotApplicable,
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
    /// The wire format this outcome is for.
    pub format: String,
    /// The status.
    pub status: CaseStatus,
    /// Data sets that passed.
    pub passed_data_sets: u32,
    /// Data sets attempted.
    pub total_data_sets: u32,
    /// The data-set rows the governing schedule table defines, where the
    /// schedule tabulates one — `total_data_sets < schedule_rows` is a
    /// logged coverage bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_rows: Option<u32>,
    /// The failure/skip message, if any.
    pub message: Option<String>,
    /// The spec citation.
    pub citation: String,
    /// The CNF-schedule trace reference (the case's [`crate::model::case::ScheduleTrace`]),
    /// when the case maps directly to a `<SERVICE>.<operation>` schedule id.
    /// Carried into `results.json` so the Conformance Certificate's
    /// per-conformance-point table is self-contained (regenerable via
    /// `report --from`). Absent for ECC-original cases with no direct id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_ref: Option<String>,
    /// For a case with no normative schedule backing: why it exists
    /// (schedule-stub derivation or extension) — a stub-derived case is
    /// never presented as schedule-conformant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ecc_original: Option<String>,
    /// The ITS-REST binding the case drives (or the explicit
    /// no-binding/native-only fact).
    #[serde(default)]
    pub binding: String,
    /// The lowest edition rung the case's assertions matched, when below the
    /// newest (the case's edition finding level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition_level: Option<String>,
    /// The individual edition observations recorded during the case.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edition_findings: Vec<String>,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u128,
}

fn default_sut_kind() -> SutKind {
    SutKind::Ours
}

fn default_edition_policy() -> EditionPolicy {
    EditionPolicy::Pinned(crate::edition::Edition::Release110)
}
