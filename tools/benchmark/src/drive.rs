//! The open-loop executor (register 00 §1, register 01 §1).
//!
//! Takes a [`SutClient`] (the provably-ECC-identical conformance transport —
//! the fairness guarantee), a built [`Workload`], and a [`Recorder`]; provisions
//! the workload's templates, then dispatches every [`PlannedOp`] at its *planned*
//! offset from the run start. Dispatch is **open loop**: a slow response never
//! delays the next send (each op runs in its own task), and a late dispatch is
//! still recorded against the *planned* send time, so a saturated SUT cannot
//! flatter its tail (register 01 §1 coordinated-omission correction).
//!
//! Per-patient runtime identifiers (`ehr_id`, per-template composition object
//! uid + latest/historical version uids, directory + status version uids) are
//! resolved from a concurrency-safe table ([`Runtime`], a `DashMap`) at dispatch;
//! an op whose prerequisite id has not yet arrived polls briefly (≤2 s) before it
//! is recorded as a schedule-dependency error.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::Instant;

use conformance::harness::{AuthSlot, HttpRequest, HttpResponse, Method, Transport};
use conformance::testdata::fixtures;
use conformance::transport::SutClient;

use crate::measure::Recorder;
use crate::model::Workload;
use crate::{Action, BenchError, OpClass, PlannedOp, TemplateKind};

/// How long a dependent op polls for a prerequisite id before it is recorded as
/// a schedule-dependency miss.
const DEP_POLL_BUDGET: Duration = Duration::from_secs(2);
/// The poll interval while waiting for a prerequisite id.
const DEP_POLL_INTERVAL: Duration = Duration::from_millis(50);

// ── Template → fixture pairing (the single source of truth) ───────────────────

/// The vendored corpus fixtures a [`TemplateKind`] maps to. The OPT and the
/// canonical composition are a matched pair (identical `template_id`), so a
/// provisioned OPT always constrains the compositions committed against it.
#[derive(Debug, Clone, Copy)]
pub struct TemplateFixtures {
    /// The OPT's `template_id` (the wire identity used on every commit).
    pub template_id: &'static str,
    /// The OPT file under the `template.valid` corpus-dir key.
    pub opt_file: &'static str,
    /// The canonical-JSON composition under the `composition.canonical-json`
    /// corpus-dir key.
    pub composition_file: &'static str,
}

/// The ECC-corpus fixtures a [`TemplateKind`] provisions and renders from, or
/// `None` for the CKM-pack kinds (sourced from [`crate::pack`], not the
/// conformance corpus). The corpus pairings are vendored CNF fixtures already
/// exercised by the ECC suite: `nested.en.v1` (small event, ~5 KB),
/// `composition_evaluation_test` (large, ~25 KB, deeply nested),
/// `persistent_minimal.en.v1` (persistent category).
#[must_use]
pub fn template_fixtures(kind: TemplateKind) -> Option<TemplateFixtures> {
    Some(match kind {
        TemplateKind::Vitals => TemplateFixtures {
            template_id: "nested.en.v1",
            opt_file: "nested/nested.opt",
            composition_file: "nested.en.v1__full.json",
        },
        TemplateKind::Nested => TemplateFixtures {
            template_id: "composition_evaluation_test",
            opt_file: "validation/composition_evaluation_test.opt",
            composition_file: "composition_evaluation_test__full.json",
        },
        TemplateKind::Persistent => TemplateFixtures {
            template_id: "persistent_minimal.en.v1",
            opt_file: "minimal_persistent/persistent_minimal.opt",
            composition_file: "persistent_minimal.en.v1__full.json",
        },
        TemplateKind::CkmVitalSigns
        | TemplateKind::CkmLabResult
        | TemplateKind::CkmMedicationOrder
        | TemplateKind::CkmSummary
        | TemplateKind::CkmSynopsis => return None,
    })
}

/// The wire `template_id` a kind registers under (CKM-pack or ECC-corpus).
#[must_use]
pub fn template_id_of(kind: TemplateKind) -> &'static str {
    if let Some(tpl) = crate::pack::get(kind) {
        tpl.template_id
    } else {
        template_fixtures(kind).map_or("unknown", |f| f.template_id)
    }
}

/// The OPT 1.4 XML that provisions a [`TemplateKind`]: the vendored CKM OPT for
/// a CKM kind, else the ECC-corpus OPT.
///
/// # Errors
/// [`BenchError`] if the OPT cannot be read.
pub fn template_opt_xml(kind: TemplateKind) -> Result<String, BenchError> {
    if let Some(tpl) = crate::pack::get(kind) {
        return tpl.opt_text();
    }
    let file = template_fixtures(kind)
        .ok_or_else(|| BenchError::Fixture(format!("no OPT source for {kind:?}")))?
        .opt_file;
    fixtures::read_from("template.valid", file).map_err(|e| BenchError::Fixture(e.to_string()))
}

/// The composition skeleton for a [`TemplateKind`]: the committed CKM example
/// for a CKM kind, else the canonical-JSON corpus fixture.
///
/// # Errors
/// [`BenchError`] if the composition cannot be read or parsed.
pub fn template_composition(kind: TemplateKind) -> Result<Value, BenchError> {
    if let Some(tpl) = crate::pack::get(kind) {
        return tpl.skeleton();
    }
    let file = template_fixtures(kind)
        .ok_or_else(|| BenchError::Fixture(format!("no composition source for {kind:?}")))?
        .composition_file;
    let text = fixtures::read_from("composition.canonical-json", file)
        .map_err(|e| BenchError::Fixture(e.to_string()))?;
    serde_json::from_str(&text).map_err(BenchError::Json)
}

/// The [`TemplateKind`] an op commits against, if any (used to skip an excluded
/// template's ops at dispatch). Reads/queries/EHR/status/directory ops carry no
/// template.
#[must_use]
fn action_template(action: &Action) -> Option<TemplateKind> {
    match action {
        Action::CreateComposition { template, .. }
        | Action::UpdateComposition { template, .. }
        | Action::CommitContribution { template, .. }
        | Action::UploadOpt { template } => Some(*template),
        _ => None,
    }
}

// ── The driver ────────────────────────────────────────────────────────────────

/// The outcome of driving one workload: the recorder (populated with per-class
/// histograms), the throughput accounting, and any templates a SUT refused to
/// provision (surfaced loudly in the report, never a silent skip).
pub struct DriveOutcome {
    /// The recorder, ready for `summaries()`. Not `Debug` (holds
    /// `HdrHistogram`s), so [`DriveOutcome`]'s `Debug` skips it.
    pub recorder: Recorder,
    /// The measurement window (seconds) — the throughput denominator.
    pub window_s: f64,
    /// Measured (post-warmup) requests.
    pub requests: u64,
    /// Errored requests (non-expected status / transport / dependency miss).
    pub errors: u64,
    /// Sustained requests/second over the window.
    pub rps: f64,
    /// Fraction of requests that errored (`errors / (requests + errors)`).
    pub error_rate: f64,
    /// `template_id`s the SUT refused to provision, with the observed status.
    pub excluded_templates: Vec<String>,
}

impl std::fmt::Debug for DriveOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriveOutcome")
            .field("window_s", &self.window_s)
            .field("requests", &self.requests)
            .field("errors", &self.errors)
            .field("rps", &self.rps)
            .field("error_rate", &self.error_rate)
            .field("excluded_templates", &self.excluded_templates)
            .finish_non_exhaustive()
    }
}

/// A completion sample handed from a dispatch task to the recorder collector.
struct Sample {
    class: OpClass,
    /// The *planned* send offset from the run start (coordinated-omission base).
    planned: Duration,
    /// The completion offset from the run start, or `None` if the op errored.
    completion: Option<Duration>,
}

/// Provision the workload's templates, dispatch its open-loop schedule against
/// the SUT, and return the populated recorder + throughput accounting.
///
/// The pre-registered [`Workload::window`] is the throughput denominator (an
/// open-loop schedule's honest measurement window); [`Workload::warmup`] is
/// handed to the recorder, which discards samples whose planned send falls in
/// the warmup floor symmetrically.
///
/// # Errors
/// [`BenchError`] only on a setup failure (a template that cannot be read).
/// A SUT that *rejects* a provisioning upload is recorded in
/// [`DriveOutcome::excluded_templates`], not raised.
pub async fn drive(
    client: &SutClient,
    workload: &Workload,
    mut recorder: Recorder,
) -> Result<DriveOutcome, BenchError> {
    recorder.set_warmup(workload.warmup);

    // Provision every template the workload declares (both packs), from its
    // vendored OPT XML. A SUT that rejects an upload has that template's kind
    // recorded as excluded; its scheduled ops are skipped (and counted) at
    // dispatch rather than dropped one-by-one as errors — fairness note below.
    let mut excluded_kinds: HashSet<TemplateKind> = HashSet::new();
    let mut failures: Vec<(TemplateKind, u16)> = Vec::new();
    for kind in &workload.provisioning {
        let xml = template_opt_xml(*kind)?;
        if let Some(status) = upload_opt_xml(client, &xml).await {
            excluded_kinds.insert(*kind);
            failures.push((*kind, status));
            eprintln!(
                "bench: SUT rejected template `{}` upload (HTTP {status}) — its ops are excluded from this run",
                template_id_of(*kind)
            );
        }
    }

    // The collector owns the recorder and folds samples in on a single task, so
    // the recorder needs no interior mutability under the concurrent dispatch.
    let (tx, mut rx) = mpsc::unbounded_channel::<Sample>();
    let collector = tokio::spawn(async move {
        while let Some(sample) = rx.recv().await {
            match sample.completion {
                Some(completion) => recorder.record(sample.class, sample.planned, completion),
                None => recorder.error(sample.class),
            }
        }
        recorder
    });

    // Dispatch in planned order (defensively sorted). Each op runs in its own
    // task; the loop only *sleeps until* the planned time then spawns, so a slow
    // response never delays the next send (open loop).
    let mut ops: Vec<PlannedOp> = workload.ops.clone();
    ops.sort_by_key(|op| op.at);

    let runtime = Arc::new(Runtime::new());
    let run_start = Instant::now();
    let mut tasks: JoinSet<()> = JoinSet::new();
    // Per-excluded-kind skip counter (the loud, counted alternative to a silent
    // per-op drop). `sleep_until` is absolute, so skipping the sleep for an
    // excluded op leaves every later op's planned dispatch time unchanged.
    let mut skipped: HashMap<TemplateKind, u64> = HashMap::new();

    for op in ops {
        if let Some(template) = action_template(&op.action)
            && excluded_kinds.contains(&template)
        {
            *skipped.entry(template).or_default() += 1;
            continue;
        }
        tokio::time::sleep_until(run_start + op.at).await;
        let client = client.clone();
        let runtime = Arc::clone(&runtime);
        let tx = tx.clone();
        tasks.spawn(async move {
            let planned = op.at;
            let ok = execute_op(&client, &runtime, &op).await;
            let completion = ok.then(|| run_start.elapsed());
            let _ = tx.send(Sample {
                class: op.class,
                planned,
                completion,
            });
        });
    }
    drop(tx);
    while tasks.join_next().await.is_some() {}

    let recorder = collector
        .await
        .map_err(|e| BenchError::Unexpected(format!("recorder collector task: {e}")))?;

    // Fold the per-kind skip counts into the loud exclusion notes.
    for (kind, count) in &skipped {
        eprintln!(
            "bench: {count} scheduled ops skipped for excluded template `{}`",
            template_id_of(*kind)
        );
    }
    let excluded_templates: Vec<String> = failures
        .iter()
        .map(|(kind, status)| {
            let n = skipped.get(kind).copied().unwrap_or(0);
            format!(
                "{} (upload → HTTP {status}; {n} scheduled ops skipped)",
                template_id_of(*kind)
            )
        })
        .collect();

    let window_s = workload.window.as_secs_f64();
    let requests = recorder.total_measured();
    let errors = recorder.total_errors();
    let total = requests + errors;
    let rps = if window_s > 0.0 {
        requests as f64 / window_s
    } else {
        0.0
    };
    let error_rate = if total > 0 {
        errors as f64 / total as f64
    } else {
        0.0
    };

    Ok(DriveOutcome {
        recorder,
        window_s,
        requests,
        errors,
        rps,
        error_rate,
        excluded_templates,
    })
}

/// Upload an OPT 1.4 from raw XML text. Returns `None` on an accepted upload
/// (`201`/`204`, or `409`/`200` for an already-present template — provisioning
/// is idempotent), or `Some(status)` on a rejection the caller records loudly.
async fn upload_opt_xml(client: &SutClient, xml: &str) -> Option<u16> {
    let req = HttpRequest::new(Method::Post, "/definition/template/adl1.4")
        .with_auth(AuthSlot::Regular)
        .text_body(xml.to_owned(), "application/xml");
    match client.send(req).await {
        Ok(resp) if matches!(resp.status, 200 | 201 | 204 | 409) => None,
        Ok(resp) => Some(resp.status),
        Err(_) => Some(0),
    }
}

// ── Per-patient runtime state ─────────────────────────────────────────────────

/// The most-recent composition of one template for one patient.
#[derive(Debug, Clone, Default)]
struct CompState {
    /// The versioned-object uid (the `{object}` path segment).
    object_uid: String,
    /// The latest version uid (`{object}::{system}::{ver}`) — the If-Match value.
    latest_ovid: String,
    /// A prior version uid, once at least one update has happened.
    historical_ovid: Option<String>,
}

/// Everything the driver learns about one ward patient at runtime.
#[derive(Debug, Default)]
struct PatientState {
    ehr_id: Option<String>,
    /// The current `EHR_STATUS` version uid (the status-update If-Match value).
    status_ovid: Option<String>,
    directory_present: bool,
    /// The current directory FOLDER version uid (the dir-update If-Match value).
    directory_ovid: Option<String>,
    comps: HashMap<TemplateKind, CompState>,
    /// The template of the most-recently written composition (resolves the
    /// template-less read ops: read-latest / read-version / history).
    recent_template: Option<TemplateKind>,
}

/// The concurrency-safe per-patient runtime table.
struct Runtime {
    patients: DashMap<usize, PatientState>,
}

impl Runtime {
    fn new() -> Self {
        Self {
            patients: DashMap::new(),
        }
    }
}

// ── Op execution ──────────────────────────────────────────────────────────────

/// Execute one planned op against the SUT, updating the runtime table on
/// success. Returns whether the op is a measured success (expected status), or
/// `false` for a dependency miss / unexpected status / transport failure — all
/// recorded as errors and debug-logged.
// A flat dispatch over every `Action` variant; splitting the match would scatter
// one op's request/id-resolution/runtime-update logic across helpers and read
// worse, so the length is inherent to the enum's width.
#[allow(clippy::too_many_lines)]
async fn execute_op(client: &SutClient, runtime: &Runtime, op: &PlannedOp) -> bool {
    let patient = op.patient;
    match &op.action {
        Action::CreateEhr { status } => {
            let req = json_req(Method::Post, "/ehr".to_owned(), status)
                .header("prefer", "return=representation");
            match send_expect(client, req, &[201]).await {
                Some(resp) => {
                    let ehr_id = ehr_id_from(&resp);
                    let status_ovid = resp
                        .json()
                        .ok()
                        .and_then(|v| string_at(&v, "/ehr_status/uid/value"));
                    match ehr_id {
                        Some(id) => {
                            let mut p = runtime.patients.entry(patient).or_default();
                            p.ehr_id = Some(id);
                            p.status_ovid = status_ovid;
                            true
                        }
                        None => miss("create-ehr: no ehr_id in response"),
                    }
                }
                None => false,
            }
        }
        Action::ReadEhr => match resolve_ehr(runtime, patient).await {
            Some(ehr) => hit(client, HttpRequest::get(format!("/ehr/{ehr}")), &[200]).await,
            None => miss("read-ehr: ehr_id unresolved"),
        },
        Action::CreateComposition { template, payload } => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("create-composition: ehr_id unresolved");
            };
            let req = json_req(Method::Post, format!("/ehr/{ehr}/composition"), payload)
                .header("prefer", "return=representation");
            match send_expect(client, req, &[201]).await {
                Some(resp) => match version_uid_from(&resp) {
                    Some(ovid) => {
                        record_composition(runtime, patient, *template, &ovid, false);
                        true
                    }
                    None => miss("create-composition: no version uid in response"),
                },
                None => false,
            }
        }
        Action::UpdateComposition { template, payload } => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("update-composition: ehr_id unresolved");
            };
            let Some(comp) = resolve_comp(runtime, patient, *template).await else {
                return miss("update-composition: no prior composition");
            };
            let req = json_req(
                Method::Put,
                format!("/ehr/{ehr}/composition/{}", comp.object_uid),
                payload,
            )
            .header("if-match", comp.latest_ovid.clone())
            .header("prefer", "return=representation");
            match send_expect(client, req, &[200]).await {
                Some(resp) => match version_uid_from(&resp) {
                    Some(ovid) => {
                        record_composition(runtime, patient, *template, &ovid, true);
                        true
                    }
                    None => miss("update-composition: no version uid in response"),
                },
                None => false,
            }
        }
        Action::ReadLatestComposition => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("read-latest: ehr_id unresolved");
            };
            let Some(comp) = resolve_recent_comp(runtime, patient).await else {
                return miss("read-latest: no prior composition");
            };
            hit(
                client,
                HttpRequest::get(format!("/ehr/{ehr}/composition/{}", comp.object_uid)),
                &[200],
            )
            .await
        }
        Action::ReadCompositionVersion => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("read-version: ehr_id unresolved");
            };
            let Some(comp) = resolve_recent_comp(runtime, patient).await else {
                return miss("read-version: no prior composition");
            };
            // Prefer a historical version once one exists; otherwise the latest
            // version uid is itself a valid specific-version read.
            let ovid = comp.historical_ovid.as_ref().unwrap_or(&comp.latest_ovid);
            hit(
                client,
                HttpRequest::get(format!("/ehr/{ehr}/composition/{ovid}")),
                &[200],
            )
            .await
        }
        Action::CommitContribution { payload, .. } => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("contribution: ehr_id unresolved");
            };
            let req = json_req(Method::Post, format!("/ehr/{ehr}/contribution"), payload);
            send_expect(client, req, &[201]).await.is_some()
        }
        Action::AqlPatient { query } => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("aql-patient: ehr_id unresolved");
            };
            let q = query.replace("{{ehr_id}}", &ehr);
            let body = serde_json::json!({ "q": q });
            hit_body(client, Method::Post, "/query/aql".to_owned(), &body, &[200]).await
        }
        Action::AqlWard { query } => {
            let body = serde_json::json!({ "q": query });
            hit_body(client, Method::Post, "/query/aql".to_owned(), &body, &[200]).await
        }
        Action::ReadDirectory => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("dir-read: ehr_id unresolved");
            };
            // 404 is tolerated: the open-loop schedule does not guarantee a
            // prior directory write (openEHR ITS-REST DIRECTORY: GET on an EHR
            // with no directory is 404). It is a legitimate empty read, not a
            // server defect, so it is a measured success.
            hit(
                client,
                HttpRequest::get(format!("/ehr/{ehr}/directory")),
                &[200, 204, 404],
            )
            .await
        }
        Action::UpdateDirectory { payload } => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("dir-update: ehr_id unresolved");
            };
            let existing = runtime
                .patients
                .get(&patient)
                .and_then(|p| p.directory_ovid.clone());
            let req = if let Some(ovid) = existing {
                json_req(Method::Put, format!("/ehr/{ehr}/directory"), payload)
                    .header("if-match", ovid)
                    .header("prefer", "return=representation")
            } else {
                json_req(Method::Post, format!("/ehr/{ehr}/directory"), payload)
                    .header("prefer", "return=representation")
            };
            match send_expect(client, req, &[200, 201]).await {
                Some(resp) => {
                    if let Some(ovid) = version_uid_from(&resp) {
                        let mut p = runtime.patients.entry(patient).or_default();
                        p.directory_present = true;
                        p.directory_ovid = Some(ovid);
                    }
                    true
                }
                None => false,
            }
        }
        Action::ReadRevisionHistory => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("history-read: ehr_id unresolved");
            };
            let Some(comp) = resolve_recent_comp(runtime, patient).await else {
                return miss("history-read: no prior composition");
            };
            hit(
                client,
                HttpRequest::get(format!(
                    "/ehr/{ehr}/versioned_composition/{}/revision_history",
                    comp.object_uid
                )),
                &[200],
            )
            .await
        }
        Action::UpdateStatus { payload } => {
            let Some(ehr) = resolve_ehr(runtime, patient).await else {
                return miss("status-update: ehr_id unresolved");
            };
            let Some(ovid) = resolve_status_ovid(client, runtime, patient, &ehr).await else {
                return miss("status-update: ehr_status version unresolved");
            };
            let req = json_req(Method::Put, format!("/ehr/{ehr}/ehr_status"), payload)
                .header("if-match", ovid)
                .header("prefer", "return=representation");
            match send_expect(client, req, &[200]).await {
                Some(resp) => {
                    if let Some(new_ovid) = version_uid_from(&resp) {
                        runtime.patients.entry(patient).or_default().status_ovid = Some(new_ovid);
                    }
                    true
                }
                None => false,
            }
        }
        Action::UploadOpt { template } => {
            let Ok(xml) = template_opt_xml(*template) else {
                return miss("opt-upload: template fixture unreadable");
            };
            // Provisioning re-upload; the already-present 409/204 is expected.
            upload_opt_xml(client, &xml).await.is_none()
        }
        Action::ListTemplates => {
            hit(
                client,
                HttpRequest::get("/definition/template/adl1.4"),
                &[200],
            )
            .await
        }
    }
}

// ── Runtime updates + resolution ──────────────────────────────────────────────

/// Record a created/updated composition version for a patient, rotating the
/// previous latest into the historical slot on an update.
fn record_composition(
    runtime: &Runtime,
    patient: usize,
    template: TemplateKind,
    ovid: &str,
    is_update: bool,
) {
    let object_uid = object_uid_of(ovid);
    let mut p = runtime.patients.entry(patient).or_default();
    let prev_latest = p.comps.get(&template).map(|c| c.latest_ovid.clone());
    let entry = p.comps.entry(template).or_default();
    if is_update {
        entry.historical_ovid = prev_latest.or_else(|| entry.historical_ovid.clone());
    }
    entry.object_uid = object_uid;
    ovid.clone_into(&mut entry.latest_ovid);
    p.recent_template = Some(template);
}

/// Poll for the patient's `ehr_id` up to the dependency budget.
async fn resolve_ehr(runtime: &Runtime, patient: usize) -> Option<String> {
    poll(|| {
        runtime
            .patients
            .get(&patient)
            .and_then(|p| p.ehr_id.clone())
    })
    .await
}

/// Poll for a patient's most-recent composition of a template.
async fn resolve_comp(
    runtime: &Runtime,
    patient: usize,
    template: TemplateKind,
) -> Option<CompState> {
    poll(|| {
        runtime.patients.get(&patient).and_then(|p| {
            p.comps
                .get(&template)
                .filter(|c| !c.object_uid.is_empty())
                .cloned()
        })
    })
    .await
}

/// Poll for a patient's most-recently written composition (template-less reads).
async fn resolve_recent_comp(runtime: &Runtime, patient: usize) -> Option<CompState> {
    poll(|| {
        runtime.patients.get(&patient).and_then(|p| {
            p.recent_template
                .and_then(|t| p.comps.get(&t))
                .filter(|c| !c.object_uid.is_empty())
                .cloned()
        })
    })
    .await
}

/// Resolve the current `EHR_STATUS` version uid: the cached value from EHR
/// creation / a prior status write, else a `GET /ehr_status` (an unmeasured
/// lookup to build a correct `If-Match`; identical for every SUT).
async fn resolve_status_ovid(
    client: &SutClient,
    runtime: &Runtime,
    patient: usize,
    ehr: &str,
) -> Option<String> {
    if let Some(ovid) = runtime
        .patients
        .get(&patient)
        .and_then(|p| p.status_ovid.clone())
    {
        return Some(ovid);
    }
    let resp = client
        .send(HttpRequest::get(format!("/ehr/{ehr}/ehr_status")))
        .await
        .ok()?;
    let ovid = version_uid_from(&resp)
        .or_else(|| resp.json().ok().and_then(|v| string_at(&v, "/uid/value")))?;
    runtime.patients.entry(patient).or_default().status_ovid = Some(ovid.clone());
    Some(ovid)
}

/// Poll a closure returning `Some` until it yields a value or the dependency
/// budget elapses.
async fn poll<T>(mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + DEP_POLL_BUDGET;
    loop {
        if let Some(v) = f() {
            return Some(v);
        }
        if Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(DEP_POLL_INTERVAL).await;
    }
}

// ── HTTP helpers ──────────────────────────────────────────────────────────────

/// Build a JSON-body request with the regular credential.
fn json_req(method: Method, path: String, value: &Value) -> HttpRequest {
    HttpRequest {
        method,
        path,
        headers: vec![("content-type".to_owned(), "application/json".to_owned())],
        body: Some(serde_json::to_vec(value).unwrap_or_default()),
        auth: AuthSlot::Regular,
    }
}

/// Send a request, returning the response iff its status is expected (else
/// debug-logs and returns `None` so the caller records an error).
async fn send_expect(
    client: &SutClient,
    req: HttpRequest,
    expected: &[u16],
) -> Option<HttpResponse> {
    let method = req.method.as_str();
    let path = req.path.clone();
    match client.send(req).await {
        Ok(resp) if expected.contains(&resp.status) => Some(resp),
        Ok(resp) => {
            eprintln!(
                "bench: {method} {path} → HTTP {} (expected {expected:?})",
                resp.status
            );
            None
        }
        Err(e) => {
            eprintln!("bench: {method} {path} transport error: {e}");
            None
        }
    }
}

/// Send a bodyless request, returning success iff the status is expected.
async fn hit(client: &SutClient, req: HttpRequest, expected: &[u16]) -> bool {
    send_expect(client, req, expected).await.is_some()
}

/// Send a JSON-body request, returning success iff the status is expected.
async fn hit_body(
    client: &SutClient,
    method: Method,
    path: String,
    value: &Value,
    expected: &[u16],
) -> bool {
    send_expect(client, json_req(method, path, value), expected)
        .await
        .is_some()
}

/// Record a schedule-dependency miss (a prerequisite id never arrived).
fn miss(reason: &str) -> bool {
    eprintln!("bench: schedule-dependency miss — {reason}");
    false
}

// ── Response parsing ──────────────────────────────────────────────────────────

/// The version uid from a versioned write: the `ETag` — parsed by the
/// conformance wire layer, which handles the weak form `W/"…"` the
/// development edition emits (ITS-REST overview §"`ETag` and Last-Modified");
/// a hand-rolled quote-strip kept the `W/` prefix and poisoned every stored
/// uid — else the last path segment of `Location`.
fn version_uid_from(resp: &HttpResponse) -> Option<String> {
    if let Some(raw) = resp.header("etag")
        && let Ok(etag) = conformance::wire::headers::parse_etag(raw)
    {
        return Some(etag.value);
    }
    resp.header("location").and_then(last_path_segment)
}

/// The `ehr_id` from an EHR creation: the representation body, else the last
/// path segment of `Location`.
fn ehr_id_from(resp: &HttpResponse) -> Option<String> {
    if let Ok(v) = resp.json()
        && let Some(id) = string_at(&v, "/ehr_id/value")
    {
        return Some(id);
    }
    resp.header("location").and_then(last_path_segment)
}

/// The versioned-object uid (the part before `::`) of a version uid.
fn object_uid_of(ovid: &str) -> String {
    ovid.split("::").next().unwrap_or(ovid).to_owned()
}

/// The last non-empty path segment of a URL/path.
fn last_path_segment(loc: &str) -> Option<String> {
    loc.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

/// A string at a JSON pointer.
fn string_at(v: &Value, pointer: &str) -> Option<String> {
    v.pointer(pointer)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_pairings_are_matched_opt_and_composition() {
        for kind in [
            TemplateKind::Vitals,
            TemplateKind::Nested,
            TemplateKind::Persistent,
        ] {
            let f = template_fixtures(kind).expect("ECC-corpus kind has fixtures");
            assert_eq!(
                std::path::Path::new(f.opt_file)
                    .extension()
                    .and_then(|e| e.to_str()),
                Some("opt")
            );
            assert_eq!(
                std::path::Path::new(f.composition_file)
                    .extension()
                    .and_then(|e| e.to_str()),
                Some("json")
            );
            assert!(!f.template_id.is_empty());
        }
    }

    #[test]
    fn ckm_kinds_route_through_the_pack_not_the_corpus() {
        for kind in crate::pack::KINDS {
            assert!(
                template_fixtures(kind).is_none(),
                "{kind:?} must not resolve to an ECC-corpus fixture"
            );
            // OPT + composition come from the vendored CKM pack.
            assert!(
                template_opt_xml(kind)
                    .expect("CKM OPT reads")
                    .contains("template")
            );
            assert!(
                template_composition(kind)
                    .expect("CKM skeleton parses")
                    .is_object()
            );
            assert_eq!(
                template_id_of(kind),
                crate::pack::get(kind).unwrap().template_id
            );
        }
    }

    #[test]
    fn action_template_identifies_composition_ops() {
        assert_eq!(
            action_template(&Action::CommitContribution {
                template: TemplateKind::CkmLabResult,
                payload: Value::Null,
            }),
            Some(TemplateKind::CkmLabResult)
        );
        assert_eq!(action_template(&Action::ReadEhr), None);
        assert_eq!(action_template(&Action::ReadLatestComposition), None);
    }

    #[test]
    fn object_uid_splits_on_double_colon() {
        assert_eq!(object_uid_of("abc-123::local.ehrbase.org::2"), "abc-123");
        assert_eq!(object_uid_of("no-colons"), "no-colons");
    }

    #[test]
    fn last_segment_of_a_location() {
        assert_eq!(
            last_path_segment("http://h/ehrbase/rest/openehr/v1/ehr/abc-123"),
            Some("abc-123".to_owned())
        );
        assert_eq!(last_path_segment("http://h/ehr/x/"), Some("x".to_owned()));
        assert_eq!(last_path_segment(""), None);
    }

    #[test]
    fn version_uid_prefers_etag_over_location() {
        let resp = HttpResponse {
            status: 201,
            headers: vec![
                ("etag".to_owned(), "\"v-uid::sys::1\"".to_owned()),
                (
                    "location".to_owned(),
                    "http://h/ehr/e/composition/other".to_owned(),
                ),
            ],
            body: Vec::new(),
        };
        assert_eq!(version_uid_from(&resp).as_deref(), Some("v-uid::sys::1"));
    }

    #[test]
    fn version_uid_strips_the_weak_etag_form() {
        // The development edition emits weak ETags (ITS-REST overview §"ETag
        // and Last-Modified"); the stored uid must be the bare
        // OBJECT_VERSION_ID — the C1 smoke run caught the unstripped form
        // 404-ing every subsequent read.
        let resp = HttpResponse {
            status: 201,
            headers: vec![("etag".to_owned(), "W/\"v-uid::sys::1\"".to_owned())],
            body: Vec::new(),
        };
        assert_eq!(version_uid_from(&resp).as_deref(), Some("v-uid::sys::1"));
    }

    #[test]
    fn ehr_id_from_representation_body() {
        let resp = HttpResponse {
            status: 201,
            headers: Vec::new(),
            body: br#"{"ehr_id":{"value":"the-ehr"}}"#.to_vec(),
        };
        assert_eq!(ehr_id_from(&resp).as_deref(), Some("the-ehr"));
    }
}
