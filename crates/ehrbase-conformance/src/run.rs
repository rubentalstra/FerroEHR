//! The runner (design §4.3–§4.5): executes the selected implemented cases
//! against a SUT transport and assembles the [`RunResults`], including the full
//! inventory classification so the report shows honest total coverage even at
//! the zero state.

use std::time::Instant;

use crate::case::{Format, Profile};
use crate::harness::{CaseError, RunContext, Transport};
use crate::registry::{Registration, registry};
use crate::results::{
    CaseOutcome, CaseStatus, CorpusPin, InventoryClass, RunResults, SelectionInfo, SutIdentity,
};
use crate::schedule::{self, ScheduleError};

/// Errors raised while setting up a run (before any case executes).
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The schedule could not be parsed/classified.
    #[error(transparent)]
    Schedule(#[from] ScheduleError),
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
    /// The declared RM version (recorded in the statement).
    pub rm_version: String,
    /// The declared auth mode (recorded in the statement).
    pub auth_mode: String,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            filter: None,
            profile: None,
            formats: vec![Format::Json],
            rm_version: "1.2.0".to_owned(),
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
/// [`RunError::Schedule`] if the vendored schedule cannot be parsed/classified.
pub async fn run(transport: &dyn Transport, config: &RunConfig) -> Result<RunResults, RunError> {
    let reg = registry();

    // Full inventory classification (the honest total-coverage view).
    let schedule = schedule::parse_default()?;
    let inventory: Vec<InventoryClass> = schedule
        .inventory()?
        .into_iter()
        .map(|item| {
            let kind = match reg.classify(&item) {
                Registration::Implemented(_) => "implemented".to_owned(),
                Registration::Excluded(reason) => reason.label(),
            };
            InventoryClass {
                key: item.key,
                id: item.id,
                chapter: item.chapter.label().to_owned(),
                kind,
            }
        })
        .collect();

    // Execute the implemented, selected cases.
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
                id: meta.id.to_owned(),
                chapter: meta.chapter.label().to_owned(),
                capability: format!("{:?}", meta.capability),
                profiles: meta.profiles.iter().map(|p| format!("{p:?}")).collect(),
                provenance: format!("{:?}", meta.provenance),
                format: format!("{format:?}").to_lowercase(),
                status,
                passed_data_sets,
                total_data_sets,
                message,
                schedule_ref: meta.schedule_ref.to_owned(),
                duration_ms,
            });
        }
    }

    Ok(RunResults {
        sut: SutIdentity {
            base_url: transport.describe(),
            rm_version: config.rm_version.clone(),
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
        inventory,
    })
}
