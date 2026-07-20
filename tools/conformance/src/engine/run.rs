//! The executor: runs the selected registered cases against one SUT and
//! assembles the [`RunResults`].
//!
//! Register seams (in application order, per case):
//! 1. **Fairness register** ([`crate::model::fairness`]) — foreign SUTs only
//!    (`SutKind::Foreign`): `extension` / `rm-version-sensitive` entries
//!    short-circuit to `NotApplicable` (running an extension route against a
//!    SUT that lacks it would only 404); `defect` entries reclassify nothing.
//!    An ehrbase-rs run NEVER consults it — the zero-drift guarantee.
//! 2. **Own-corpus adjudication register**
//!    ([`crate::model::adjudication`]) — every SUT: a `corpus-dialect` entry
//!    short-circuits to `Skipped` with its citation (standing rule 3: the
//!    defective golden is skipped, never edited).
//! 3. **Edition ladder** ([`crate::edition`]) — the case executes under the
//!    SUT's edition policy; findings are drained into the outcome.
//!
//! A case's own run function may also return
//! [`CaseError::NotApplicable`], the case-level counterpart of the fairness
//! register's N/A verdict: a native-API-only SM operation with no ITS-REST
//! wire binding anywhere is a first-class, cited not-applicable outcome (never
//! a "skip"), for EVERY SUT (the HTTP-only instrument cannot reach it). This
//! path applies to an ehrbase-rs run too — it is a permanent property of the
//! operation, not an SUT-specific adjudication, so it does not breach the
//! zero-drift guarantee.

use std::time::Instant;

use crate::edition::EditionRecorder;
use crate::engine::harness::{CaseError, RunContext, Transport};
use crate::engine::registry::registry;
use crate::model::adjudication::OwnRegister;
use crate::model::case::{Binding, Format, Profile, ScheduleTrace};
use crate::model::fairness::AdjudicationRegister;
use crate::model::profile::required_capabilities;
use crate::model::versions::SpecVersions;
use crate::reporting::results::{
    CaseOutcome, CaseStatus, CorpusPin, ProductIdentity, RunResults, SelectionInfo, SutIdentity,
};
use crate::sut::descriptor::{SutDescriptor, SutKind};

/// Errors raised while setting up a run (before any case executes).
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The ECC catalogue could not be loaded.
    #[error(transparent)]
    Catalog(#[from] crate::model::catalog::CatalogError),
}

/// The scope + declared context of a run.
#[derive(Debug)]
pub struct RunConfig {
    /// Only run cases whose id contains this substring.
    pub filter: Option<String>,
    /// Only run cases whose capability the given profile requires.
    pub profile: Option<Profile>,
    /// The formats to run (intersected per-case with the case's own formats).
    pub formats: Vec<Format>,
    /// The declared specification versions (recorded in the Statement),
    /// derived from vendored provenance.
    pub versions: SpecVersions,
    /// The declared auth mode (recorded in the Statement).
    pub auth_mode: String,
    /// The foreign-SUT fairness register — applied only when the descriptor
    /// is [`SutKind::Foreign`].
    pub fairness: Option<AdjudicationRegister>,
    /// The own-corpus adjudication register (vendored-data defects).
    pub own_adjudications: OwnRegister,
    /// The terminology server available to `TS` cases, if established.
    pub tx: Option<crate::ts::TxServer>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            filter: None,
            profile: None,
            formats: vec![Format::Json],
            versions: SpecVersions::latest(),
            auth_mode: "unknown".to_owned(),
            fairness: None,
            own_adjudications: OwnRegister::default(),
            tx: None,
        }
    }
}

impl RunConfig {
    fn selects_id(&self, id: &str) -> bool {
        self.filter.as_ref().is_none_or(|f| id.contains(f.as_str()))
    }

    fn selects_capability(&self, capability: crate::model::case::Capability) -> bool {
        self.profile
            .is_none_or(|p| required_capabilities(p).contains(&capability))
    }
}

fn trace_string(trace: ScheduleTrace) -> (Option<String>, Option<String>) {
    match trace {
        ScheduleTrace::Schedule(s) => (Some(s.to_owned()), None),
        ScheduleTrace::EccOriginal(reason) => (None, Some(reason.to_owned())),
    }
}

fn binding_string(binding: Binding) -> String {
    match binding {
        Binding::Rest(b) => b.to_owned(),
        Binding::NoRestBinding(sm_op) => format!("no ITS-REST binding ({sm_op})"),
        Binding::NativeApiOnly(what) => format!("native API only ({what})"),
    }
}

/// Execute the selected cases against `sut` over `transport` and assemble
/// the results.
///
/// # Errors
/// [`RunError::Catalog`] if the ECC catalogue cannot be loaded.
#[expect(
    clippy::too_many_lines,
    reason = "the case-execution loop assembles each per-format CaseOutcome inline; \
              extracting it would only trade line count for a many-argument helper"
)]
pub async fn run(
    transport: &dyn Transport,
    sut: &SutDescriptor,
    config: &RunConfig,
) -> Result<RunResults, RunError> {
    let reg = registry();
    let catalog = crate::model::catalog::Catalog::load_default()?;

    // Fairness register: foreign SUTs only. Our own product's verdict is
    // never touched by it, regardless of flags.
    let fairness = match sut.kind {
        SutKind::Ours => None,
        SutKind::Foreign => config.fairness.as_ref(),
    };

    let mut cases = Vec::new();
    for entry in reg.entries() {
        let meta = &entry.meta;
        if !config.selects_id(meta.id) || !config.selects_capability(meta.capability) {
            continue;
        }
        let ecc_id = catalog
            .by_primary_ref(meta.id)
            .map(|e| e.ecc_id.clone())
            .unwrap_or_default();

        let not_applicable = fairness
            .and_then(|r| r.lookup(&ecc_id, meta.area.tag()))
            .filter(|adj| adj.disposition.is_not_applicable());
        let own_skip = config
            .own_adjudications
            .lookup(&ecc_id)
            .filter(|a| a.disposition == crate::model::adjudication::OwnDisposition::CorpusDialect);

        let (schedule_ref, ecc_original) = trace_string(meta.schedule);

        for &format in &config.formats {
            if !meta.formats.contains(&format) {
                continue;
            }
            let mut outcome = CaseOutcome {
                ecc_id: ecc_id.clone(),
                id: meta.id.to_owned(),
                title: meta.title.to_owned(),
                capability: format!("{:?}", meta.capability),
                format: format!("{format:?}").to_lowercase(),
                status: CaseStatus::Skipped,
                passed_data_sets: 0,
                total_data_sets: 0,
                schedule_rows: None,
                message: None,
                citation: meta.citation.to_owned(),
                schedule_ref: schedule_ref.clone(),
                ecc_original: ecc_original.clone(),
                binding: binding_string(meta.binding),
                edition_level: None,
                edition_findings: Vec::new(),
                duration_ms: 0,
            };
            if let Some(adj) = not_applicable {
                outcome.status = CaseStatus::NotApplicable;
                outcome.message = Some(adj.reason.clone());
                outcome.citation = adj.citation.clone();
            } else if let Some(own) = own_skip {
                outcome.status = CaseStatus::Skipped;
                outcome.message = Some(format!("adjudicated: {}", own.reason));
                outcome.citation = own.citation.clone();
            } else {
                let recorder = EditionRecorder::default();
                let ctx = RunContext {
                    transport,
                    format,
                    sut,
                    edition_policy: sut.edition_policy,
                    edition: &recorder,
                    tx: config.tx.as_ref(),
                };
                let start = Instant::now();
                let result = (entry.run)(&ctx).await;
                outcome.duration_ms = start.elapsed().as_millis();
                match result {
                    Ok(report) => {
                        outcome.status = CaseStatus::Passed;
                        outcome.passed_data_sets = report.passed;
                        outcome.total_data_sets = report.total;
                        outcome.schedule_rows = report.schedule_rows;
                    }
                    Err(CaseError::Assertion(msg)) => {
                        outcome.status = CaseStatus::Failed;
                        outcome.message = Some(msg);
                    }
                    Err(CaseError::Skipped(msg)) => {
                        outcome.status = CaseStatus::Skipped;
                        outcome.message = Some(msg);
                    }
                    Err(CaseError::NotApplicable(msg)) => {
                        // A case-level N/A: the SM operation has no ITS-REST
                        // wire binding anywhere (native-API-only), so it is a
                        // first-class, cited not-applicable verdict — never a
                        // skip (owner ruling). The case's own `citation` (the
                        // SM citation) stays; the message carries the native-
                        // test evidence pointer.
                        outcome.status = CaseStatus::NotApplicable;
                        outcome.message = Some(msg);
                    }
                    Err(e @ (CaseError::Transport(_) | CaseError::Codec(_))) => {
                        outcome.status = CaseStatus::Errored;
                        outcome.message = Some(e.to_string());
                    }
                }
                outcome.edition_level = recorder.floor().map(|e| e.label().to_owned());
                outcome.edition_findings = recorder
                    .take()
                    .into_iter()
                    .map(|f| format!("{}: {}", f.edition.label(), f.what))
                    .collect();
            }
            cases.push(outcome);
        }
    }

    Ok(RunResults {
        sut: SutIdentity {
            base_url: transport.describe(),
            product: ProductIdentity {
                name: sut.name.clone(),
                version: sut.product_label.clone(),
                image_digest: None,
            },
            kind: sut.kind,
            edition_policy: sut.edition_policy,
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
        terminology: None,
        cases,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::engine::harness::{HttpRequest, HttpResponse, Transport, TransportError};
    use crate::model::case::Format;
    use crate::reporting::results::CaseStatus;
    use crate::sut::builtin;

    use super::{RunConfig, run};

    /// A transport that fails (and records that it was touched) if a case ever
    /// issues a request — a native-API-only N/A case must produce its verdict
    /// with no wire call at all.
    struct NoWire {
        touched: AtomicBool,
    }

    #[async_trait::async_trait]
    impl Transport for NoWire {
        async fn send(&self, _request: HttpRequest) -> Result<HttpResponse, TransportError> {
            self.touched.store(true, Ordering::SeqCst);
            Err(TransportError::Http(
                "a native-API-only N/A case must not touch the wire".to_owned(),
            ))
        }
        fn describe(&self) -> String {
            "no-wire (test)".to_owned()
        }
    }

    /// The outcome-model reclassification (issue #146, family B): a
    /// native-API-only SM operation with no ITS-REST binding anywhere returns
    /// [`crate::engine::harness::CaseError::NotApplicable`], which the executor
    /// records as [`CaseStatus::NotApplicable`] — a first-class, cited verdict,
    /// never a skip — for our own SUT, without issuing any wire request. Driven
    /// through the whole MESSAGING area (all 10 cases are native-API-only).
    #[tokio::test]
    async fn native_api_only_cases_report_not_applicable_without_touching_the_wire() {
        let transport = NoWire {
            touched: AtomicBool::new(false),
        };
        let sut = builtin::ehrbase_rs(
            "http://test.invalid/ehrbase/rest/openehr/v1".to_owned(),
            None,
            None,
        );
        let config = RunConfig {
            filter: Some("msg/".to_owned()),
            formats: vec![Format::Json],
            ..RunConfig::default()
        };
        let results = run(&transport, &sut, &config).await.expect("run executes");

        assert_eq!(
            results.cases.len(),
            10,
            "the 10 MESSAGING cases are selected (JSON only)"
        );
        for case in &results.cases {
            assert_eq!(
                case.status,
                CaseStatus::NotApplicable,
                "{}: a native-API-only Messaging case must be N/A, got {:?}",
                case.id,
                case.status
            );
            assert!(
                !case.citation.trim().is_empty(),
                "{}: an N/A verdict must carry its SM citation",
                case.id
            );
            let message = case.message.as_deref().unwrap_or_default();
            assert!(
                message.contains("app/ehrbase/tests/"),
                "{}: an N/A message must carry the native-test evidence pointer, got {message:?}",
                case.id
            );
        }
        assert!(
            !transport.touched.load(Ordering::SeqCst),
            "no native-API-only N/A case may issue a wire request"
        );
    }
}
