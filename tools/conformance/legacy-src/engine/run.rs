//! The runner (design v4): executes the selected registered cases against a
//! SUT transport and assembles the [`RunResults`]. Coverage accounting is
//! catalogue-driven (`reporting`), not tied to any legacy corpus.

use std::time::Instant;

use crate::adjudication::AdjudicationRegister;
use crate::case::{Format, Profile};
use crate::harness::{CaseError, RunContext, Transport};
use crate::registry::registry;
use crate::results::{
    CaseOutcome, CaseStatus, CorpusPin, ProductIdentity, RunResults, SelectionInfo, SutIdentity,
};
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
    /// The product under test (recorded in the identity, §3a.1). Defaults to
    /// `ehrbase-rs @ <workspace version>` so existing runs are unchanged.
    pub product: ProductIdentity,
    /// The declared specification versions (recorded in the statement).
    /// Only [`SpecVersions::latest`] is supported today.
    pub versions: SpecVersions,
    /// The declared auth mode (recorded in the statement).
    pub auth_mode: String,
    /// The upstream fairness adjudication register (§3a.4), applied **only** for
    /// non-`ehrbase-rs` SUTs. `None` (the default, and every `ehrbase-rs` run)
    /// means today's behaviour, byte-for-byte — the zero-drift gate on our own
    /// baseline.
    pub adjudications: Option<AdjudicationRegister>,
    /// The terminology server the run has available (B4 `TS` area): the
    /// spun-up `wiremock` fixture (CI default) or a real server
    /// (`--tx-server-url`). `None` when no terminology server was established —
    /// the FHIR-tx / fault-injection `TS` cases then skip with a stated reason.
    pub tx: Option<crate::ts::TxServer>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            filter: None,
            profile: None,
            formats: vec![Format::Json],
            product: ProductIdentity::default(),
            versions: SpecVersions::latest(),
            auth_mode: "unknown".to_owned(),
            adjudications: None,
            tx: None,
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

    // The upstream fairness adjudication register (§3a.4) is consulted **only**
    // for a non-`ehrbase-rs` SUT: our own product's verdict is never touched by
    // a register, guaranteeing the zero-drift baseline regardless of flags.
    let adjudications = if config.product.is_ehrbase_rs() {
        None
    } else {
        config.adjudications.as_ref()
    };

    // Execute the registered, selected cases.
    let mut cases = Vec::new();
    for entry in reg.entries() {
        let meta = &entry.meta;
        if !config.selects_id(meta.id) || !config.selects_profile(meta.profiles) {
            continue;
        }
        let ecc_id = catalog
            .by_primary_ref(meta.id)
            .map(|e| e.ecc_id.clone())
            .unwrap_or_default();
        // An `extension` / `rm-version-sensitive` adjudication short-circuits the
        // case to NotApplicable (never executed — running an extension route
        // against a SUT that lacks it would only 404). A `defect` adjudication
        // reclassifies nothing at runtime: the case runs and its failure stands.
        let na = adjudications
            .and_then(|r| r.lookup(&ecc_id, meta.area.tag()))
            .filter(|adj| adj.disposition.is_not_applicable());

        for &format in &config.formats {
            if !meta.formats.contains(&format) {
                continue;
            }
            let (status, passed_data_sets, total_data_sets, message, citation, duration_ms) =
                if let Some(adj) = na {
                    (
                        CaseStatus::NotApplicable,
                        0,
                        0,
                        Some(adj.reason.clone()),
                        adj.citation.clone(),
                        0,
                    )
                } else {
                    let ctx = RunContext {
                        transport,
                        format,
                        tx: config.tx.as_ref(),
                    };
                    let start = Instant::now();
                    let result = (entry.run)(&ctx).await;
                    let duration_ms = start.elapsed().as_millis();
                    let (status, passed, total, message) = match result {
                        Ok(report) => (CaseStatus::Passed, report.passed, report.total, None),
                        Err(CaseError::Assertion(msg)) => (CaseStatus::Failed, 0, 0, Some(msg)),
                        Err(CaseError::Skipped(msg)) => (CaseStatus::Skipped, 0, 0, Some(msg)),
                        Err(e @ (CaseError::Transport(_) | CaseError::Codec(_))) => {
                            (CaseStatus::Errored, 0, 0, Some(e.to_string()))
                        }
                    };
                    (
                        status,
                        passed,
                        total,
                        message,
                        meta.citation.to_owned(),
                        duration_ms,
                    )
                };
            cases.push(CaseOutcome {
                ecc_id: ecc_id.clone(),
                id: meta.id.to_owned(),
                title: meta.title.to_owned(),
                capability: format!("{:?}", meta.capability),
                profiles: meta.profiles.iter().map(|p| format!("{p:?}")).collect(),
                format: format!("{format:?}").to_lowercase(),
                status,
                passed_data_sets,
                total_data_sets,
                message,
                citation,
                schedule_ref: meta.schedule_ref.map(str::to_owned),
                duration_ms,
            });
        }
    }

    Ok(RunResults {
        sut: SutIdentity {
            base_url: transport.describe(),
            product: config.product.clone(),
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
        // The terminology run record (fixture + recorded exchange) is attached
        // by the runner binary, which owns the fixture lifecycle.
        terminology: None,
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adjudication::AdjudicationRegister;
    use crate::harness::{HttpRequest, HttpResponse, TransportError};

    /// A transport that answers every request with `404` — so any *executed*
    /// case fails/errors, letting a test distinguish "ran" from "short-circuited
    /// to `NotApplicable`".
    struct NotFound;

    #[async_trait::async_trait]
    impl Transport for NotFound {
        async fn send(&self, _request: HttpRequest) -> Result<HttpResponse, TransportError> {
            Ok(HttpResponse {
                status: 404,
                headers: Vec::new(),
                body: Vec::new(),
            })
        }
        fn describe(&self) -> String {
            "mock://not-found".to_owned()
        }
    }

    const DEM_EXTENSION: &str = r#"
[[area]]
area = "DEM"
disposition = "extension"
reason = "Upstream EHRbase has no demographic REST API."
citation = "docs/plans/x1-comparison.md §2c"
"#;

    fn dem_config(product: ProductIdentity) -> RunConfig {
        RunConfig {
            filter: Some("dem/".to_owned()),
            product,
            adjudications: Some(AdjudicationRegister::parse(DEM_EXTENSION).expect("register")),
            ..RunConfig::default()
        }
    }

    #[tokio::test]
    async fn non_self_sut_extension_area_reports_not_applicable() {
        let product = ProductIdentity {
            name: "ehrbase-java".to_owned(),
            version: "2.34.0".to_owned(),
            image_digest: None,
        };
        let results = run(&NotFound, &dem_config(product)).await.expect("run ok");
        assert!(!results.cases.is_empty(), "DEM cases must be selected");
        for c in &results.cases {
            assert_eq!(
                c.status,
                CaseStatus::NotApplicable,
                "{} should be N/A under the extension register",
                c.ecc_id
            );
            assert!(
                c.citation.contains("x1-comparison"),
                "N/A citation comes from the register"
            );
        }
        // NotApplicable is excluded from the pass/fail denominators.
        assert_eq!(results.passed(), 0);
        assert_eq!(results.failed(), 0);
        assert_eq!(results.not_applicable(), results.executed());
        // The identity carries the upstream product.
        assert_eq!(results.sut.product.name, "ehrbase-java");
        assert!(!results.sut.is_ehrbase_rs());
    }

    #[tokio::test]
    async fn self_sut_ignores_the_register_entirely() {
        // Same register, but our own product: the register must be ignored, so
        // the cases actually execute (against the 404 mock → not N/A). This is
        // the zero-drift guarantee for our baseline.
        let results = run(&NotFound, &dem_config(ProductIdentity::default()))
            .await
            .expect("run ok");
        assert!(!results.cases.is_empty(), "DEM cases must be selected");
        assert_eq!(
            results.not_applicable(),
            0,
            "an ehrbase-rs SUT never reclassifies via the register"
        );
        assert!(results.sut.is_ehrbase_rs());
    }
}
