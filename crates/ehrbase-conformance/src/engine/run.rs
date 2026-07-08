//! The runner (design v4): executes the selected registered cases against a
//! SUT transport and assembles the [`RunResults`]. Coverage accounting is
//! catalogue-driven (`reporting`), not tied to any legacy corpus.

use std::time::Instant;

use crate::case::{Format, Profile};
use crate::harness::{CaseError, RunContext, Transport};
use crate::registry::registry;
use crate::results::{CaseOutcome, CaseStatus, CorpusPin, RunResults, SelectionInfo, SutIdentity};
use crate::version::SpecVersions;

/// Errors raised while setting up a run (before any case executes).
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The ECC catalogue could not be loaded.
    #[error(transparent)]
    Catalog(#[from] crate::catalog::CatalogError),
}

/// The scope + declared context of a run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Only run cases whose id contains this substring.
    pub filter: Option<String>,
    /// Only run cases required by this profile.
    pub profile: Option<Profile>,
    /// The formats to run (intersected per-case with the case's own formats).
    pub formats: Vec<Format>,
    /// The declared specification versions (recorded in the statement).
    /// Only [`SpecVersions::latest`] is supported today.
    pub versions: SpecVersions,
    /// The declared auth mode (recorded in the statement).
    pub auth_mode: String,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            filter: None,
            profile: None,
            formats: vec![Format::Json],
            versions: SpecVersions::latest(),
            auth_mode: "unknown".to_owned(),
        }
    }
}

impl RunConfig {
    fn selects_id(&self, id: &str) -> bool {
        self.filter.as_ref().is_none_or(|f| id.contains(f.as_str()))
    }

    fn selects_profile(&self, profiles: &[Profile]) -> bool {
        self.profile.is_none_or(|p| profiles.contains(&p))
    }
}

/// Execute the selected cases against `transport` and assemble the results.
///
/// # Errors
/// [`RunError::Catalog`] if the ECC catalogue cannot be loaded.
pub async fn run(transport: &dyn Transport, config: &RunConfig) -> Result<RunResults, RunError> {
    let reg = registry();
    // The ECC catalogue: every outcome carries our own case number.
    let catalog = crate::catalog::Catalog::load_default()?;

    // Execute the registered, selected cases.
    let mut cases = Vec::new();
    for entry in reg.entries() {
        let meta = &entry.meta;
        if !config.selects_id(meta.id) || !config.selects_profile(meta.profiles) {
            continue;
        }
        for &format in &config.formats {
            if !meta.formats.contains(&format) {
                continue;
            }
            let ctx = RunContext { transport, format };
            let start = Instant::now();
            let result = (entry.run)(&ctx).await;
            let duration_ms = start.elapsed().as_millis();
            let (status, passed_data_sets, total_data_sets, message) = match result {
                Ok(report) => (CaseStatus::Passed, report.passed, report.total, None),
                Err(CaseError::Assertion(msg)) => (CaseStatus::Failed, 0, 0, Some(msg)),
                Err(CaseError::Skipped(msg)) => (CaseStatus::Skipped, 0, 0, Some(msg)),
                Err(e @ (CaseError::Transport(_) | CaseError::Codec(_))) => {
                    (CaseStatus::Errored, 0, 0, Some(e.to_string()))
                }
            };
            cases.push(CaseOutcome {
                ecc_id: catalog
                    .by_primary_ref(meta.id)
                    .map(|e| e.ecc_id.clone())
                    .unwrap_or_default(),
                id: meta.id.to_owned(),
                title: meta.title.to_owned(),
                capability: format!("{:?}", meta.capability),
                profiles: meta.profiles.iter().map(|p| format!("{p:?}")).collect(),
                format: format!("{format:?}").to_lowercase(),
                status,
                passed_data_sets,
                total_data_sets,
                message,
                citation: meta.citation.to_owned(),
                duration_ms,
            });
        }
    }

    Ok(RunResults {
        sut: SutIdentity {
            base_url: transport.describe(),
            versions: config.versions.clone(),
            auth_mode: config.auth_mode.clone(),
        },
        corpus: CorpusPin::default(),
        started: jiff::Timestamp::now().to_string(),
        selection: SelectionInfo {
            filter: config.filter.clone(),
            profile: config.profile.map(|p| format!("{p:?}").to_lowercase()),
            formats: config
                .formats
                .iter()
                .map(|f| format!("{f:?}").to_lowercase())
                .collect(),
        },
        cases,
    })
}
