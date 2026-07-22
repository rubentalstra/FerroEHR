//! The performance measurement machinery — the OPEN-LOOP driver that
//! executes a `kind: performance` case against a live SUT and produces the
//! re-checkable [`crate::perf::Measurement`] record.
//!
//! The offered load is a deterministic seeded arrival schedule (arrival `i`
//! fires at `start + i / rate`, the operation mix is realized by exact
//! largest-remainder interleaving), never closed-loop users: a stalled SUT
//! cannot slow the schedule down, and every latency is measured from the
//! PLANNED arrival instant — coordinated omission cannot hide stalls
//! (the `hdrhistogram` crate documents the same correction model).
//!
//! The wire realization of each mix operation follows the committed ITS-REST
//! operation bindings (`artifacts/bindings/its-rest/`): create EHR 201,
//! commit COMPOSITION 201 (uid via `ETag`), read COMPOSITION at version 200,
//! ad-hoc AQL 200. Anything else observed counts as an error arrival.
//!
//! The corpus is the `scale_ladder` recipe (contract:
//! `corpus/recipes/scale_ladder.md`): N EHRs at ~100 committed blood-pressure
//! composition versions each, payloads from [`crate::exec::recipes::bp_series`],
//! seeded strictly through the public API (never a database backdoor).

use std::io::Read;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use base64::Engine;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};

use crate::ixit::{AuthMode, Environment, Instance, Ixit};
use crate::perf::{
    ClassVerdict, Measurement, OperationMeasurement, PerformanceCase, class_verdict,
};

/// Latency histograms record microseconds in `1 µs ..= 10 min` at 3
/// significant figures — far past the client timeout, so a timeout can never
/// saturate the range.
const HDR_MAX_US: u64 = 600_000_000;

/// Per-request client timeout. A response slower than this is an error
/// arrival recorded at the timeout latency (the SLO is 1 s — a 60 s stall is
/// already a hard violation either way).
const CLIENT_TIMEOUT: Duration = Duration::from_mins(1);

/// The closed operation vocabulary a workload `mix` may name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerfOp {
    /// `POST /ehr` → 201 (`I_EHR_SERVICE.create_ehr`).
    EhrCreate,
    /// `POST /ehr/{ehr_id}/composition` → 201 (`I_EHR_COMPOSITION.create_composition`).
    CompositionCommit,
    /// `GET /ehr/{ehr_id}/composition/{version_uid}` → 200
    /// (`I_EHR_COMPOSITION.get_composition_at_version`).
    CompositionRead,
    /// `POST /query/aql` → 200 (`I_QUERY_SERVICE.execute_ad_hoc_query`).
    AdhocQuery,
}

impl PerfOp {
    /// All operations, in mix-vocabulary order.
    pub const ALL: &'static [PerfOp] = &[
        PerfOp::EhrCreate,
        PerfOp::CompositionCommit,
        PerfOp::CompositionRead,
        PerfOp::AdhocQuery,
    ];

    /// Parse a workload-mix operation name.
    ///
    /// # Errors
    /// The unknown name (the mix vocabulary is closed).
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "ehr_create" => Ok(PerfOp::EhrCreate),
            "composition_commit" => Ok(PerfOp::CompositionCommit),
            "composition_read" => Ok(PerfOp::CompositionRead),
            "adhoc_query" => Ok(PerfOp::AdhocQuery),
            other => Err(format!("unknown workload mix operation {other:?}")),
        }
    }

    /// The mix-vocabulary name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            PerfOp::EhrCreate => "ehr_create",
            PerfOp::CompositionCommit => "composition_commit",
            PerfOp::CompositionRead => "composition_read",
            PerfOp::AdhocQuery => "adhoc_query",
        }
    }
}

/// The blocking SUT client for the measurement machinery: base URL + auth
/// resolved once from the ixit `sut` instance; one connection pool shared by
/// every worker thread.
#[derive(Clone)]
pub struct PerfClient {
    client: reqwest::blocking::Client,
    base_url: String,
    authorization: Option<String>,
    extra_headers: Vec<(String, String)>,
}

impl std::fmt::Debug for PerfClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerfClient")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

/// One wire response the client observed (status + the two id-bearing
/// headers the bindings capture from).
#[derive(Debug)]
struct WireReply {
    status: u16,
    etag: Option<String>,
    location: Option<String>,
}

impl PerfClient {
    /// Build the client from an ixit instance (credentials resolved from the
    /// referenced environment variables, exactly like the functional driver).
    ///
    /// # Errors
    /// A message when a credential env var is unset or the client cannot be
    /// built.
    pub fn from_instance(instance: &Instance) -> Result<Self, String> {
        let authorization = match &instance.auth {
            AuthMode::None => None,
            AuthMode::Basic {
                user_env,
                password_env,
            } => {
                let user = std::env::var(user_env)
                    .map_err(|_| format!("credential env {user_env} unset"))?;
                let pass = std::env::var(password_env)
                    .map_err(|_| format!("credential env {password_env} unset"))?;
                let token = base64::engine::general_purpose::STANDARD
                    .encode(format!("{user}:{pass}").as_bytes());
                Some(format!("Basic {token}"))
            }
            AuthMode::Bearer { token_env } => {
                let token = std::env::var(token_env)
                    .map_err(|_| format!("credential env {token_env} unset"))?;
                Some(format!("Bearer {token}"))
            }
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(CLIENT_TIMEOUT)
            .pool_max_idle_per_host(256)
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        Ok(Self {
            client,
            base_url: instance.base_url.trim_end_matches('/').to_owned(),
            authorization,
            extra_headers: instance.headers.clone().unwrap_or_default(),
        })
    }

    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<(&'static str, Vec<u8>)>,
        prefer_minimal: bool,
    ) -> Result<WireReply, String> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base_url))
            .header("Accept", "application/json");
        if let Some(auth) = &self.authorization {
            request = request.header("Authorization", auth);
        }
        for (name, value) in &self.extra_headers {
            request = request.header(name, value);
        }
        if prefer_minimal {
            request = request.header("Prefer", "return=minimal");
        }
        if let Some((content_type, bytes)) = body {
            request = request.header("Content-Type", content_type).body(bytes);
        }
        let response = request.send().map_err(|e| format!("transport: {e}"))?;
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let reply = WireReply {
            status: response.status().as_u16(),
            etag: header("etag"),
            location: header("location"),
        };
        // Drain the body so the pooled connection is reusable.
        let mut sink = Vec::new();
        let mut reader = response;
        let _drained = reader.read_to_end(&mut sink);
        Ok(reply)
    }
}

/// `W/"uid"` / `"uid"` → `uid` (the bindings' `strip: weak-quotes` capture).
fn strip_weak_quotes(etag: &str) -> String {
    etag.trim_start_matches("W/").trim_matches('"').to_owned()
}

/// The last path segment of a `Location` header (the bindings' fallback
/// `ehr_id` capture on a `return=minimal` create).
fn location_last_segment(location: &str) -> Option<String> {
    location
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

// ── the seeded corpus (the scale_ladder recipe) ─────────────────────────────

/// The seeded `scale_ladder` corpus index: what the measurement operations
/// address. Persisted as a sidecar JSON so a re-run can skip re-seeding.
#[derive(Debug, Serialize, Deserialize)]
pub struct SeededCorpus {
    /// The corpus key this index realizes (e.g. `cnf.scale.10k`).
    pub corpus: String,
    /// Every seeded EHR id.
    pub ehr_ids: Vec<String>,
    /// Seeded compositions as `(ehr index, version_uid)`.
    pub compositions: Vec<(usize, String)>,
}

/// The volumetric shape of one `cnf.scale.*` corpus key per the
/// `scale_ladder` contract (EHR count; ~100 composition versions each).
///
/// # Errors
/// An unknown scale key.
pub fn scale_shape(corpus_key: &str) -> Result<(usize, usize), String> {
    match corpus_key {
        "cnf.scale.10k" => Ok((10_000, 100)),
        "cnf.scale.100k" => Ok((100_000, 100)),
        "cnf.scale.1m" => Ok((1_000_000, 100)),
        "cnf.scale.10m" => Ok((10_000_000, 100)),
        other => Err(format!("unknown scale corpus key {other:?}")),
    }
}

/// Seed the `scale_ladder` corpus through the public API: upload the
/// blood-pressure OPT (409 on re-run is fine), create `ehrs` EHRs, commit
/// `versions_per_ehr` [`crate::exec::recipes::bp_series`] compositions into
/// each. Deterministic content; parallel across `workers` threads.
///
/// # Errors
/// A message on any wire outcome outside the bindings' created/exists
/// outcomes, or a transport fault.
#[allow(clippy::too_many_lines)] // one seeding procedure, linear phases
pub fn seed_scale_ladder(
    client: &PerfClient,
    corpus_key: &str,
    opt_xml: &str,
    ehrs: usize,
    versions_per_ehr: usize,
    workers: usize,
    progress: &(dyn Fn(String) + Sync),
) -> Result<SeededCorpus, String> {
    // Template first — the compositions' constraint carrier.
    let upload = client.request(
        reqwest::Method::POST,
        "/definition/template/adl1.4",
        Some(("application/xml", opt_xml.as_bytes().to_vec())),
        false,
    )?;
    match upload.status {
        201 | 409 => {}
        other => return Err(format!("OPT upload returned {other} (expected 201/409)")),
    }

    let workers = workers.max(1);

    // Phase 1: EHRs.
    let ehr_slots: Vec<Mutex<Option<String>>> = (0..ehrs).map(|_| Mutex::new(None)).collect();
    let next_ehr = AtomicUsize::new(0);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let done = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next_ehr.fetch_add(1, Ordering::Relaxed);
                    if i >= ehrs {
                        break;
                    }
                    let outcome = client
                        .request(reqwest::Method::POST, "/ehr", None, true)
                        .and_then(|reply| {
                            if reply.status != 201 {
                                return Err(format!("create_ehr returned {}", reply.status));
                            }
                            reply
                                .location
                                .as_deref()
                                .and_then(location_last_segment)
                                .ok_or_else(|| "create_ehr: no Location ehr_id".to_owned())
                        });
                    match outcome {
                        Ok(id) => {
                            if let Ok(mut slot) = ehr_slots[i].lock() {
                                *slot = Some(id);
                            }
                        }
                        Err(e) => {
                            if let Ok(mut f) = failures.lock() {
                                f.push(e);
                            }
                            break;
                        }
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_multiple_of(1000) {
                        progress(format!("seeded {n}/{ehrs} EHRs"));
                    }
                }
            });
        }
    });
    if let Ok(f) = failures.lock()
        && let Some(first) = f.first()
    {
        return Err(format!("seeding EHRs failed: {first}"));
    }
    let mut ehr_ids = Vec::with_capacity(ehrs);
    for slot in &ehr_slots {
        let id = slot
            .lock()
            .ok()
            .and_then(|s| s.clone())
            .ok_or_else(|| "seeding EHRs left a gap".to_owned())?;
        ehr_ids.push(id);
    }

    // Phase 2: compositions — bp_series(j % 10) into EHR i, uid captured from
    // the ETag exactly as the create_composition binding does.
    let total = ehrs
        .checked_mul(versions_per_ehr)
        .ok_or_else(|| "corpus size overflows".to_owned())?;
    let bodies: Vec<Vec<u8>> = (0..10)
        .map(|k| {
            crate::exec::recipes::bp_series(k)
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::to_vec(&v).map_err(|e| e.to_string()))
        })
        .collect::<Result<_, _>>()?;
    let next_commit = AtomicUsize::new(0);
    let committed: Mutex<Vec<(usize, String)>> = Mutex::new(Vec::with_capacity(total));
    let done = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let mut local: Vec<(usize, String)> = Vec::new();
                loop {
                    let t = next_commit.fetch_add(1, Ordering::Relaxed);
                    if t >= total {
                        break;
                    }
                    let ehr_index = t / versions_per_ehr;
                    let series = t % 10;
                    let Some(body) = bodies.get(series) else {
                        break;
                    };
                    let Some(ehr_id) = ehr_ids.get(ehr_index) else {
                        break;
                    };
                    let outcome = client
                        .request(
                            reqwest::Method::POST,
                            &format!("/ehr/{ehr_id}/composition"),
                            Some(("application/json", body.clone())),
                            true,
                        )
                        .and_then(|reply| {
                            if reply.status != 201 {
                                return Err(format!(
                                    "create_composition returned {}",
                                    reply.status
                                ));
                            }
                            reply
                                .etag
                                .as_deref()
                                .map(strip_weak_quotes)
                                .ok_or_else(|| "create_composition: no ETag".to_owned())
                        });
                    match outcome {
                        Ok(uid) => local.push((ehr_index, uid)),
                        Err(e) => {
                            if let Ok(mut f) = failures.lock() {
                                f.push(e);
                            }
                            break;
                        }
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_multiple_of(10_000) {
                        progress(format!("committed {n}/{total} compositions"));
                    }
                }
                if let Ok(mut all) = committed.lock() {
                    all.append(&mut local);
                }
            });
        }
    });
    if let Ok(f) = failures.lock()
        && let Some(first) = f.first()
    {
        return Err(format!("seeding compositions failed: {first}"));
    }
    let mut compositions = committed
        .into_inner()
        .map_err(|_| "seeding lock poisoned".to_owned())?;
    if compositions.len() != total {
        return Err(format!(
            "seeded {}/{total} compositions only",
            compositions.len()
        ));
    }
    compositions.sort();
    Ok(SeededCorpus {
        corpus: corpus_key.to_owned(),
        ehr_ids,
        compositions,
    })
}

// ── the open-loop measured run ──────────────────────────────────────────────

/// Exact-share deterministic mix interleaving (largest-remainder error
/// diffusion): arrival `i`'s operation is the mix entry with the largest
/// accumulated credit. Two runners with the same mix produce the same
/// sequence.
struct MixSequencer {
    ops: Vec<(PerfOp, f64)>,
    credit: Vec<f64>,
}

impl MixSequencer {
    fn new(mix: &[(String, crate::perf::Percent)]) -> Result<Self, String> {
        let ops = mix
            .iter()
            .map(|(name, share)| PerfOp::parse(name).map(|op| (op, share.0)))
            .collect::<Result<Vec<_>, _>>()?;
        if ops.is_empty() {
            return Err("workload mix is empty".to_owned());
        }
        let credit = vec![0.0; ops.len()];
        Ok(Self { ops, credit })
    }

    fn next(&mut self) -> PerfOp {
        for (credit, (_, share)) in self.credit.iter_mut().zip(&self.ops) {
            *credit += *share;
        }
        let mut best = 0;
        for i in 1..self.credit.len() {
            if self.credit[i] > self.credit[best] {
                best = i;
            }
        }
        self.credit[best] -= 100.0;
        self.ops.get(best).map_or(PerfOp::EhrCreate, |(op, _)| *op)
    }
}

/// The per-operation aggregation a run collects.
struct OpRecorder {
    histogram: Histogram<u64>,
    errors: u64,
}

/// One completed arrival: operation, latency from the PLANNED instant, and
/// whether the wire outcome matched the binding's expected kind.
struct Completion {
    op: PerfOp,
    latency_us: u64,
    ok: bool,
    recorded: bool,
}

/// The ad-hoc AQL the `adhoc_query` mix operation executes — the
/// blood-pressure read the corpus contract publishes, scoped to one EHR.
const ADHOC_AQL: &str = "SELECT c/uid/value, o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude \
     FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o [openEHR-EHR-OBSERVATION.blood_pressure.v2] \
     WHERE e/ehr_id/value = $ehr_id LIMIT 10";

fn perform(
    client: &PerfClient,
    op: PerfOp,
    corpus: &SeededCorpus,
    arrival: u64,
    commit_bodies: &[Vec<u8>],
) -> Result<bool, String> {
    // Deterministic corpus addressing: a large odd stride cycles the pools.
    let stride = arrival
        .checked_mul(2_654_435_761)
        .unwrap_or(arrival)
        .max(arrival);
    match op {
        PerfOp::EhrCreate => {
            let reply = client.request(reqwest::Method::POST, "/ehr", None, true)?;
            Ok(reply.status == 201)
        }
        PerfOp::CompositionCommit => {
            let n = corpus.ehr_ids.len().max(1);
            let index = usize::try_from(stride).unwrap_or(usize::MAX) % n;
            let ehr_id = corpus
                .ehr_ids
                .get(index)
                .ok_or_else(|| "corpus has no EHRs".to_owned())?;
            let body = commit_bodies
                .get(usize::try_from(arrival).unwrap_or(usize::MAX) % commit_bodies.len().max(1))
                .ok_or_else(|| "no commit bodies".to_owned())?;
            let reply = client.request(
                reqwest::Method::POST,
                &format!("/ehr/{ehr_id}/composition"),
                Some(("application/json", body.clone())),
                true,
            )?;
            Ok(reply.status == 201)
        }
        PerfOp::CompositionRead => {
            let n = corpus.compositions.len().max(1);
            let index = usize::try_from(stride).unwrap_or(usize::MAX) % n;
            let (ehr_index, uid) = corpus
                .compositions
                .get(index)
                .ok_or_else(|| "corpus has no compositions".to_owned())?;
            let ehr_id = corpus
                .ehr_ids
                .get(*ehr_index)
                .ok_or_else(|| "corpus composition references a missing EHR".to_owned())?;
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/composition/{uid}"),
                None,
                false,
            )?;
            Ok(reply.status == 200)
        }
        PerfOp::AdhocQuery => {
            let n = corpus.ehr_ids.len().max(1);
            let index = usize::try_from(stride).unwrap_or(usize::MAX) % n;
            let ehr_id = corpus
                .ehr_ids
                .get(index)
                .ok_or_else(|| "corpus has no EHRs".to_owned())?;
            let body = serde_json::json!({
                "q": ADHOC_AQL,
                "query_parameters": { "ehr_id": ehr_id }
            });
            let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
            let reply = client.request(
                reqwest::Method::POST,
                "/query/aql",
                Some(("application/json", bytes)),
                false,
            )?;
            Ok(reply.status == 200)
        }
    }
}

/// Drive one performance case's open-loop workload and produce its
/// re-checkable measurement record (verdict computed by
/// [`crate::perf::class_verdict`] from the encoded histograms).
///
/// `warmup_s` is the case's normative warmup; `duration_s` is the case's
/// sustained window or an officially EXTENDED one (the hours ladder — a
/// longer hold of the same offered load is a stricter demonstration).
/// The CLI never passes a window shorter than the case's; only the offline
/// test harness drives synthetic second-scale windows.
///
/// # Errors
/// A message on schedule construction or aggregation failure (individual
/// arrival faults are error observations, not run failures).
pub fn drive_case(
    case: &PerformanceCase,
    client: &PerfClient,
    corpus: &SeededCorpus,
    environment: &Environment,
    warmup_s: u64,
    duration_s: u64,
    progress: &(dyn Fn(String) + Sync),
) -> Result<Measurement, String> {
    case.check_invariants()?;
    let window = run_window(
        client,
        corpus,
        &case.workload.mix,
        case.workload.arrival_rate.0,
        warmup_s,
        duration_s,
        progress,
    )?;
    let (verdict, violations) =
        class_verdict(case, window.offered_load_sustained, &window.operations)?;
    Ok(Measurement {
        case: case.id.clone(),
        class: case.class,
        environment: environment.clone(),
        offered_load_sustained: window.offered_load_sustained,
        warmup_s,
        duration_s,
        operations: window.operations,
        verdict,
        violations,
    })
}

/// One executed open-loop window's raw outcome — the shared core the class
/// runs (conformance) and the knee stress ladder (exploration) both drive.
#[derive(Debug)]
pub struct WindowOutcome {
    /// Measured arrivals over the actual measured span (arrivals/s).
    pub offered_load_sustained: f64,
    /// Per-operation records (encoded HDR histograms, re-checkable).
    pub operations: Vec<OperationMeasurement>,
    /// Whether the GENERATOR failed to hold the schedule (dispatch lagged
    /// more than 2% past the planned span) — the honest stop signal for a
    /// stress climb: beyond this point the instrument, not the SUT, is the
    /// bottleneck.
    pub generator_bound: bool,
}

/// Execute one open-loop window: `rate` arrivals/s of `mix` for
/// `warmup_s + duration_s`, recording only the post-warmup span.
///
/// # Errors
/// A message on schedule construction or aggregation failure (individual
/// arrival faults are error observations, not run failures).
#[allow(clippy::too_many_lines)] // one measured-window procedure: schedule → collect → aggregate
pub fn run_window(
    client: &PerfClient,
    corpus: &SeededCorpus,
    mix: &[(String, crate::perf::Percent)],
    rate: f64,
    warmup_s: u64,
    duration_s: u64,
    progress: &(dyn Fn(String) + Sync),
) -> Result<WindowOutcome, String> {
    if !(rate.is_finite() && rate > 0.0) {
        return Err("arrival rate must be positive".to_owned());
    }
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    // arrival counts and nanosecond intervals are far below the lossy ranges
    let (warmup_arrivals, measured_arrivals, interval) = {
        let warmup = (rate * warmup_s as f64).round() as u64;
        let measured = (rate * duration_s as f64).round() as u64;
        let interval = Duration::from_nanos((1e9 / rate).round() as u64);
        (warmup, measured, interval)
    };
    if measured_arrivals == 0 {
        return Err("measurement window schedules zero arrivals".to_owned());
    }

    let mut sequencer = MixSequencer::new(mix)?;
    let commit_bodies: Vec<Vec<u8>> = (0..10)
        .map(|k| {
            crate::exec::recipes::bp_series(k)
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::to_vec(&v).map_err(|e| e.to_string()))
        })
        .collect::<Result<_, _>>()?;

    let (tx, rx) = mpsc::channel::<Completion>();
    let collector = std::thread::spawn(move || {
        let mut recorders: Vec<(PerfOp, OpRecorder)> = Vec::new();
        let mut generator_faults: u64 = 0;
        for done in rx {
            if !done.recorded {
                continue;
            }
            if done.latency_us == u64::MAX {
                generator_faults = generator_faults.saturating_add(1);
                continue;
            }
            let index = if let Some(index) = recorders.iter().position(|(op, _)| *op == done.op) {
                index
            } else {
                let Ok(histogram) = Histogram::new_with_bounds(1, HDR_MAX_US, 3) else {
                    // Statically-valid bounds; treat the impossible
                    // failure as a generator fault rather than panic.
                    generator_faults = generator_faults.saturating_add(1);
                    continue;
                };
                recorders.push((
                    done.op,
                    OpRecorder {
                        histogram,
                        errors: 0,
                    },
                ));
                recorders.len() - 1
            };
            if let Some((_, recorder)) = recorders.get_mut(index) {
                let value = done.latency_us.clamp(1, HDR_MAX_US);
                let _saturated = recorder.histogram.record(value);
                if !done.ok {
                    recorder.errors = recorder.errors.saturating_add(1);
                }
            }
        }
        (recorders, generator_faults)
    });

    let total = warmup_arrivals.saturating_add(measured_arrivals);
    let start = Instant::now();
    let dispatched_measured = Arc::new(AtomicU64::new(0));
    progress(format!(
        "open-loop schedule: {total} arrivals at {rate}/s ({warmup_s}s warmup + {duration_s}s measured)"
    ));

    // The dispatch span (last arrival fired − start) is captured before the
    // scope waits for in-flight workers, so trailing responses never inflate
    // the sustained-load denominator.
    let dispatch_span = std::thread::scope(|scope| {
        let interval_ns = u64::try_from(interval.as_nanos()).unwrap_or(u64::MAX);
        for i in 0..total {
            let planned = start + Duration::from_nanos(interval_ns.saturating_mul(i));
            let now = Instant::now();
            if planned > now {
                std::thread::sleep(planned - now);
            }
            let op = sequencer.next();
            let recorded = i >= warmup_arrivals;
            if recorded {
                dispatched_measured.fetch_add(1, Ordering::Relaxed);
            }
            let tx = tx.clone();
            let client = client.clone();
            let bodies = &commit_bodies;
            let corpus_ref = corpus;
            scope.spawn(move || {
                let outcome = perform(&client, op, corpus_ref, i, bodies);
                let latency = planned.elapsed();
                let latency_us = u64::try_from(latency.as_micros().min(u128::from(HDR_MAX_US)))
                    .unwrap_or(HDR_MAX_US);
                let (ok, latency_us) = match outcome {
                    Ok(ok) => (ok, latency_us),
                    Err(_) => (false, latency_us),
                };
                let _closed = tx.send(Completion {
                    op,
                    latency_us,
                    ok,
                    recorded,
                });
            });
            if i % 1000 == 999 {
                progress(format!("dispatched {}/{total} arrivals", i + 1));
            }
        }
        drop(tx);
        start.elapsed()
    });

    let (recorders, generator_faults) = collector
        .join()
        .map_err(|_| "collector thread panicked".to_owned())?;
    if generator_faults > 0 {
        return Err(format!("{generator_faults} generator faults"));
    }

    // Offered load actually sustained: measured arrivals over the actual
    // measured span (>= the planned window when the generator lagged, which
    // honestly deflates the sustained rate).
    let planned_span_s = warmup_s.saturating_add(duration_s);
    #[allow(clippy::cast_precision_loss)] // spans/counts << 2^52
    let (offered_load_sustained, generator_bound) = {
        let actual_span = dispatch_span.as_secs_f64().max(planned_span_s as f64);
        let measured_span = actual_span - warmup_s as f64;
        let offered = if measured_span > 0.0 {
            dispatched_measured.load(Ordering::Relaxed) as f64 / measured_span
        } else {
            0.0
        };
        let lagged = dispatch_span.as_secs_f64() > planned_span_s as f64 * 1.02;
        (offered, lagged)
    };

    let mut operations: Vec<OperationMeasurement> = Vec::new();
    for (op, recorder) in &recorders {
        operations.push(OperationMeasurement::from_histogram(
            op.as_str(),
            &recorder.histogram,
            recorder.errors,
        )?);
    }
    operations.sort_by(|a, b| a.operation.cmp(&b.operation));

    Ok(WindowOutcome {
        offered_load_sustained,
        operations,
        generator_bound,
    })
}

/// Convenience: the ixit precondition for a measured run — the `sut`
/// instance and a present environment block.
///
/// # Errors
/// A message naming the missing piece (the environment block is mandatory
/// for performance runs).
pub fn measured_run_context(ixit: &Ixit) -> Result<(&Instance, &Environment), String> {
    let instance = ixit.default_instance()?;
    let environment = ixit.environment.as_ref().ok_or_else(|| {
        "ixit has no environment block (mandatory for performance runs)".to_owned()
    })?;
    Ok((instance, environment))
}

/// Whether a measurement's verdict re-derives to the same value from its own
/// embedded histograms (the tamper check the verdict pipeline runs).
///
/// # Errors
/// As [`class_verdict`].
pub fn rederive_verdict(
    case: &PerformanceCase,
    measurement: &Measurement,
) -> Result<(ClassVerdict, Vec<String>), String> {
    class_verdict(
        case,
        measurement.offered_load_sustained,
        &measurement.operations,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    #[test]
    fn mix_sequencer_realizes_exact_shares() {
        let mix = vec![
            ("composition_read".to_owned(), crate::perf::Percent(61.0)),
            ("adhoc_query".to_owned(), crate::perf::Percent(30.0)),
            ("composition_commit".to_owned(), crate::perf::Percent(8.0)),
            ("ehr_create".to_owned(), crate::perf::Percent(1.0)),
        ];
        let mut seq = MixSequencer::new(&mix).unwrap();
        let mut counts = std::collections::HashMap::new();
        for _ in 0..1000 {
            *counts.entry(seq.next()).or_insert(0_u64) += 1;
        }
        assert_eq!(counts[&PerfOp::CompositionRead], 610);
        assert_eq!(counts[&PerfOp::AdhocQuery], 300);
        assert_eq!(counts[&PerfOp::CompositionCommit], 80);
        assert_eq!(counts[&PerfOp::EhrCreate], 10);
    }

    #[test]
    fn mix_vocabulary_is_closed() {
        assert!(PerfOp::parse("composition_read").is_ok());
        assert!(PerfOp::parse("delete_everything").is_err());
        for op in PerfOp::ALL {
            assert_eq!(PerfOp::parse(op.as_str()).unwrap(), *op);
        }
    }

    #[test]
    fn header_captures_match_the_bindings() {
        assert_eq!(
            strip_weak_quotes("W/\"abc::sys::1\""),
            "abc::sys::1".to_owned()
        );
        assert_eq!(strip_weak_quotes("\"abc\""), "abc".to_owned());
        assert_eq!(
            location_last_segment("http://sut/ehr/42").as_deref(),
            Some("42")
        );
        assert_eq!(location_last_segment(""), None);
    }

    #[test]
    fn scale_shapes_follow_the_ladder() {
        assert_eq!(scale_shape("cnf.scale.10k").unwrap(), (10_000, 100));
        assert_eq!(scale_shape("cnf.scale.10m").unwrap(), (10_000_000, 100));
        assert!(scale_shape("cnf.scale.5k").is_err());
    }
}
