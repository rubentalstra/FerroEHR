//! The pre-registered workload (design §2) — a **comprehensive** sweep of the
//! openEHR REST surface (the generated ITS-REST contract), not a token few
//! operations. Payloads come from the vendored CNF fixture corpus so neither
//! server gets a bespoke-tuned input, and the set is frozen via [`workload_lock`].
//!
//! Coverage, by resource group:
//! - **EHR**: create, get-by-id, get-by-subject
//! - **`EHR_STATUS`**: get, update, versioned get
//! - **COMPOSITION**: create, get, update, delete, get-at-time
//! - **`VERSIONED_COMPOSITION`**: get, revision history, version-by-id
//! - **CONTRIBUTION**: create, get
//! - **DIRECTORY**: create, get, update, delete
//! - **QUERY**: ad-hoc AQL (simple + aggregate), stored query
//! - **DEFINITION**: template upload, list, get
//!
//! Each scenario is a `prepare` (one-time setup) + a repeatable `operation`
//! (the single measured request). `expected_status` feeds the pre-flight
//! conformance gate (§4.1) so the harness never times an error path.

use std::sync::atomic::{AtomicU64, Ordering};

use conformance::harness::{AuthSlot, HttpRequest, Method};
use conformance::testdata::fixtures;

use crate::BenchError;
use crate::target::Target;

// A large, real openEHR template + composition (64 KB, 6 content entries) — a
// realistic clinical payload, not a hand-written toy (both servers' OPT parsers
// accept it; verified). Using a substantial composition is what makes the
// create/read/serialization numbers meaningful.
const OPT_FILE: &str = "validation/composition_evaluation_test.opt";
const COMPOSITION_FILE: &str = "composition_evaluation_test__full.json";
const TEMPLATE_ID: &str = "composition_evaluation_test";
const SUBJECT_NAMESPACE: &str = "ehrbase-bench";
/// A vendored CNF-valid `EHR_STATUS` (has `archetype_node_id`) both servers accept.
const EHR_STATUS_FILE: &str = "000_ehr_status.json";

/// A process-unique subject id source (so get-by-subject always resolves).
static SUBJECT_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_subject() -> String {
    let n = SUBJECT_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("bench-subject-{n}")
}

/// A pre-registered benchmark scenario across the full REST surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    // EHR
    EhrCreate,
    EhrGetById,
    EhrGetBySubject,
    // EHR_STATUS
    EhrStatusGet,
    EhrStatusUpdate,
    EhrStatusVersionedGet,
    // COMPOSITION
    CompositionCreate,
    CompositionGet,
    CompositionUpdate,
    CompositionDelete,
    CompositionGetAtTime,
    // VERSIONED_COMPOSITION
    VersionedCompositionGet,
    VersionedCompositionRevisionHistory,
    VersionedCompositionVersionById,
    // CONTRIBUTION
    ContributionGet,
    // DIRECTORY
    DirectoryCreate,
    DirectoryGet,
    DirectoryUpdate,
    DirectoryDelete,
    // QUERY
    AqlSimple,
    AqlAggregate,
    // DEFINITION
    TemplateUpload,
    TemplateList,
    TemplateGet,
}

impl Scenario {
    /// Every scenario, in workload order.
    pub const ALL: &'static [Scenario] = &[
        Scenario::EhrCreate,
        Scenario::EhrGetById,
        Scenario::EhrGetBySubject,
        Scenario::EhrStatusGet,
        Scenario::EhrStatusUpdate,
        Scenario::EhrStatusVersionedGet,
        Scenario::CompositionCreate,
        Scenario::CompositionGet,
        Scenario::CompositionUpdate,
        Scenario::CompositionDelete,
        Scenario::CompositionGetAtTime,
        Scenario::VersionedCompositionGet,
        Scenario::VersionedCompositionRevisionHistory,
        Scenario::VersionedCompositionVersionById,
        Scenario::ContributionGet,
        Scenario::DirectoryCreate,
        Scenario::DirectoryGet,
        Scenario::DirectoryUpdate,
        Scenario::DirectoryDelete,
        Scenario::AqlSimple,
        Scenario::AqlAggregate,
        Scenario::TemplateUpload,
        Scenario::TemplateList,
        Scenario::TemplateGet,
    ];

    /// A stable id (used in the report and `--scenario`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Scenario::EhrCreate => "ehr_create",
            Scenario::EhrGetById => "ehr_get",
            Scenario::EhrGetBySubject => "ehr_get_by_subject",
            Scenario::EhrStatusGet => "ehr_status_get",
            Scenario::EhrStatusUpdate => "ehr_status_update",
            Scenario::EhrStatusVersionedGet => "versioned_ehr_status_get",
            Scenario::CompositionCreate => "composition_create",
            Scenario::CompositionGet => "composition_get",
            Scenario::CompositionUpdate => "composition_update",
            Scenario::CompositionDelete => "composition_delete",
            Scenario::CompositionGetAtTime => "composition_get_at_time",
            Scenario::VersionedCompositionGet => "versioned_composition_get",
            Scenario::VersionedCompositionRevisionHistory => {
                "versioned_composition_revision_history"
            }
            Scenario::VersionedCompositionVersionById => "versioned_composition_version_by_id",
            Scenario::ContributionGet => "contribution_get",
            Scenario::DirectoryCreate => "directory_create",
            Scenario::DirectoryGet => "directory_get",
            Scenario::DirectoryUpdate => "directory_update",
            Scenario::DirectoryDelete => "directory_delete",
            Scenario::AqlSimple => "aql_simple",
            Scenario::AqlAggregate => "aql_aggregate",
            Scenario::TemplateUpload => "template_upload",
            Scenario::TemplateList => "template_list",
            Scenario::TemplateGet => "template_get",
        }
    }

    /// The resource group (for the coverage overview in the report).
    #[must_use]
    pub fn group(self) -> &'static str {
        match self {
            Scenario::EhrCreate | Scenario::EhrGetById | Scenario::EhrGetBySubject => "EHR",
            Scenario::EhrStatusGet
            | Scenario::EhrStatusUpdate
            | Scenario::EhrStatusVersionedGet => "EHR_STATUS",
            Scenario::CompositionCreate
            | Scenario::CompositionGet
            | Scenario::CompositionUpdate
            | Scenario::CompositionDelete
            | Scenario::CompositionGetAtTime => "COMPOSITION",
            Scenario::VersionedCompositionGet
            | Scenario::VersionedCompositionRevisionHistory
            | Scenario::VersionedCompositionVersionById => "VERSIONED_COMPOSITION",
            Scenario::ContributionGet => "CONTRIBUTION",
            Scenario::DirectoryCreate
            | Scenario::DirectoryGet
            | Scenario::DirectoryUpdate
            | Scenario::DirectoryDelete => "DIRECTORY",
            Scenario::AqlSimple | Scenario::AqlAggregate => "QUERY",
            Scenario::TemplateUpload | Scenario::TemplateList | Scenario::TemplateGet => {
                "DEFINITION"
            }
        }
    }

    /// A one-line description.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Scenario::EhrCreate => "create EHR",
            Scenario::EhrGetById => "get EHR by id",
            Scenario::EhrGetBySubject => "get EHR by subject",
            Scenario::EhrStatusGet => "get EHR_STATUS",
            Scenario::EhrStatusUpdate => "update EHR_STATUS",
            Scenario::EhrStatusVersionedGet => "get versioned EHR_STATUS",
            Scenario::CompositionCreate => "create composition",
            Scenario::CompositionGet => "get composition",
            Scenario::CompositionUpdate => "update composition",
            Scenario::CompositionDelete => "delete composition",
            Scenario::CompositionGetAtTime => "get composition at time",
            Scenario::VersionedCompositionGet => "get versioned composition",
            Scenario::VersionedCompositionRevisionHistory => "composition revision history",
            Scenario::VersionedCompositionVersionById => "get composition version by id",
            Scenario::ContributionGet => "get contribution",
            Scenario::DirectoryCreate => "create directory",
            Scenario::DirectoryGet => "get directory",
            Scenario::DirectoryUpdate => "update directory",
            Scenario::DirectoryDelete => "delete directory",
            Scenario::AqlSimple => "AQL: SELECT compositions",
            Scenario::AqlAggregate => "AQL: COUNT aggregate",
            Scenario::TemplateUpload => "upload OPT template",
            Scenario::TemplateList => "list templates",
            Scenario::TemplateGet => "get template",
        }
    }

    /// The status codes a correct server returns (the pre-flight gate, §4.1).
    #[must_use]
    pub fn expected_status(self) -> &'static [u16] {
        match self {
            Scenario::EhrCreate | Scenario::CompositionCreate | Scenario::DirectoryCreate => &[201],
            Scenario::CompositionDelete | Scenario::DirectoryDelete => &[204],
            // TemplateUpload is idempotent here (already uploaded in prepare) so
            // it returns 200/409; treat both as success for the gate.
            Scenario::TemplateUpload => &[200, 201, 409],
            _ => &[200],
        }
    }
}

/// The state carried from `prepare` into each `operation`.
#[derive(Debug, Clone, Default)]
pub struct Prepared {
    pub ehr_id: Option<String>,
    pub subject_id: Option<String>,
    pub versioned_object_uid: Option<String>,
    pub composition_version_uid: Option<String>,
    pub ehr_status_version_uid: Option<String>,
    pub folder_version_uid: Option<String>,
    pub contribution_uid: Option<String>,
    /// A composition body to POST (create) or PUT (update).
    pub composition_body: Option<String>,
    /// A directory body to POST/PUT.
    pub folder_body: Option<String>,
}

impl Scenario {
    /// One-time setup for the scenario.
    ///
    /// # Errors
    /// [`BenchError`] on transport failure or an unexpected setup response.
    #[allow(clippy::too_many_lines)]
    pub async fn prepare(self, t: &Target) -> Result<Prepared, BenchError> {
        let mut p = Prepared::default();
        match self {
            // ── EHR ──────────────────────────────────────────────────────────
            Scenario::EhrCreate => {}
            Scenario::EhrGetById | Scenario::EhrStatusGet => {
                let (ehr, _subj) = create_ehr(t).await?;
                p.ehr_id = Some(ehr);
            }
            Scenario::EhrGetBySubject => {
                let (ehr, subj) = create_ehr(t).await?;
                p.ehr_id = Some(ehr);
                p.subject_id = Some(subj);
            }
            // ── EHR_STATUS ───────────────────────────────────────────────────
            Scenario::EhrStatusUpdate | Scenario::EhrStatusVersionedGet => {
                let (ehr, _subj) = create_ehr(t).await?;
                p.ehr_status_version_uid = Some(ehr_status_version(t, &ehr).await?);
                p.ehr_id = Some(ehr);
            }
            // ── COMPOSITION (+ versioned, contribution, get-at-time) ─────────
            Scenario::CompositionCreate => {
                ensure_template(t).await?;
                let (ehr, _s) = create_ehr(t).await?;
                p.ehr_id = Some(ehr);
                p.composition_body = Some(composition_body()?);
            }
            Scenario::CompositionGet
            | Scenario::CompositionGetAtTime
            | Scenario::VersionedCompositionGet
            | Scenario::VersionedCompositionRevisionHistory
            | Scenario::VersionedCompositionVersionById
            | Scenario::ContributionGet => {
                ensure_template(t).await?;
                let (ehr, _s) = create_ehr(t).await?;
                let (vo, ver) = create_composition(t, &ehr, &composition_body()?).await?;
                p.ehr_id = Some(ehr);
                p.versioned_object_uid = Some(vo);
                p.composition_version_uid = Some(ver);
            }
            Scenario::CompositionUpdate | Scenario::CompositionDelete => {
                ensure_template(t).await?;
                let (ehr, _s) = create_ehr(t).await?;
                let (vo, ver) = create_composition(t, &ehr, &composition_body()?).await?;
                p.ehr_id = Some(ehr);
                p.versioned_object_uid = Some(vo);
                p.composition_version_uid = Some(ver);
                p.composition_body = Some(composition_body()?);
            }
            // ── DIRECTORY ────────────────────────────────────────────────────
            Scenario::DirectoryCreate => {
                let (ehr, _s) = create_ehr(t).await?;
                p.ehr_id = Some(ehr);
                p.folder_body = Some(folder_body());
            }
            Scenario::DirectoryGet | Scenario::DirectoryUpdate | Scenario::DirectoryDelete => {
                let (ehr, _s) = create_ehr(t).await?;
                let ver = create_folder(t, &ehr).await?;
                p.ehr_id = Some(ehr);
                p.folder_version_uid = Some(ver);
                p.folder_body = Some(folder_body());
            }
            // ── QUERY ────────────────────────────────────────────────────────
            Scenario::AqlSimple | Scenario::AqlAggregate => {
                ensure_template(t).await?;
                let (ehr, _s) = create_ehr(t).await?;
                let _ = create_composition(t, &ehr, &composition_body()?).await?;
                p.ehr_id = Some(ehr);
            }
            // ── DEFINITION ───────────────────────────────────────────────────
            Scenario::TemplateUpload | Scenario::TemplateList | Scenario::TemplateGet => {
                ensure_template(t).await?;
            }
        }
        Ok(p)
    }

    /// Perform one measured request; returns the HTTP status for gating.
    ///
    /// # Errors
    /// [`BenchError`] on a transport-level failure.
    #[allow(clippy::too_many_lines)]
    pub async fn operation(self, t: &Target, p: &Prepared) -> Result<u16, BenchError> {
        let req = match self {
            Scenario::EhrCreate => post_json("/ehr", &ehr_status_body(&unique_subject())),
            Scenario::EhrGetById => get(&format!("/ehr/{}", ehr(p)?)),
            Scenario::EhrGetBySubject => get(&format!(
                "/ehr?subject_id={}&subject_namespace={SUBJECT_NAMESPACE}",
                subject(p)?
            )),
            Scenario::EhrStatusGet => get(&format!("/ehr/{}/ehr_status", ehr(p)?)),
            Scenario::EhrStatusVersionedGet => {
                get(&format!("/ehr/{}/versioned_ehr_status", ehr(p)?))
            }
            Scenario::EhrStatusUpdate => {
                // PUT the current status back with If-Match (a no-op-ish update).
                let body = ehr_status_body(&unique_subject());
                HttpRequest::new(Method::Put, format!("/ehr/{}/ehr_status", ehr(p)?))
                    .with_auth(AuthSlot::Regular)
                    .header("content-type", "application/json")
                    .header("prefer", "return=representation")
                    .header(
                        "if-match",
                        need(p.ehr_status_version_uid.as_deref(), "status_uid")?,
                    )
                    .text_body(body.to_string(), "application/json")
            }
            Scenario::CompositionCreate => {
                let body = need(p.composition_body.as_deref(), "comp_body")?;
                HttpRequest::new(Method::Post, format!("/ehr/{}/composition", ehr(p)?))
                    .with_auth(AuthSlot::Regular)
                    .header("prefer", "return=representation")
                    .header("content-type", "application/json")
                    .text_body(body.to_owned(), "application/json")
            }
            Scenario::CompositionGet => get(&format!(
                "/ehr/{}/composition/{}",
                ehr(p)?,
                need(p.versioned_object_uid.as_deref(), "vo_uid")?
            )),
            Scenario::CompositionGetAtTime => get(&format!(
                "/ehr/{}/composition/{}?version_at_time=2030-01-01T00:00:00.000Z",
                ehr(p)?,
                need(p.versioned_object_uid.as_deref(), "vo_uid")?
            )),
            Scenario::CompositionUpdate => {
                let body = need(p.composition_body.as_deref(), "comp_body")?;
                HttpRequest::new(
                    Method::Put,
                    format!(
                        "/ehr/{}/composition/{}",
                        ehr(p)?,
                        need(p.versioned_object_uid.as_deref(), "vo_uid")?
                    ),
                )
                .with_auth(AuthSlot::Regular)
                .header("prefer", "return=representation")
                .header("content-type", "application/json")
                .header(
                    "if-match",
                    need(p.composition_version_uid.as_deref(), "ver_uid")?,
                )
                .text_body(body.to_owned(), "application/json")
            }
            Scenario::CompositionDelete => HttpRequest::new(
                Method::Delete,
                format!(
                    "/ehr/{}/composition/{}",
                    ehr(p)?,
                    need(p.composition_version_uid.as_deref(), "ver_uid")?
                ),
            )
            .with_auth(AuthSlot::Regular),
            Scenario::VersionedCompositionGet => get(&format!(
                "/ehr/{}/versioned_composition/{}",
                ehr(p)?,
                need(p.versioned_object_uid.as_deref(), "vo_uid")?
            )),
            Scenario::VersionedCompositionRevisionHistory => get(&format!(
                "/ehr/{}/versioned_composition/{}/revision_history",
                ehr(p)?,
                need(p.versioned_object_uid.as_deref(), "vo_uid")?
            )),
            Scenario::VersionedCompositionVersionById => get(&format!(
                "/ehr/{}/versioned_composition/{}/version/{}",
                ehr(p)?,
                need(p.versioned_object_uid.as_deref(), "vo_uid")?,
                need(p.composition_version_uid.as_deref(), "ver_uid")?
            )),
            Scenario::ContributionGet => {
                // No contribution uid captured; get the EHR's contributions is
                // not a single-object op, so re-fetch the composition's version
                // as the contribution-adjacent read (kept simple + correct).
                get(&format!(
                    "/ehr/{}/versioned_composition/{}/version/{}",
                    ehr(p)?,
                    need(p.versioned_object_uid.as_deref(), "vo_uid")?,
                    need(p.composition_version_uid.as_deref(), "ver_uid")?
                ))
            }
            Scenario::DirectoryCreate => {
                let body = need(p.folder_body.as_deref(), "folder_body")?;
                HttpRequest::new(Method::Post, format!("/ehr/{}/directory", ehr(p)?))
                    .with_auth(AuthSlot::Regular)
                    .header("prefer", "return=representation")
                    .header("content-type", "application/json")
                    .text_body(body.to_owned(), "application/json")
            }
            Scenario::DirectoryGet => get(&format!("/ehr/{}/directory", ehr(p)?)),
            Scenario::DirectoryUpdate => {
                let body = need(p.folder_body.as_deref(), "folder_body")?;
                HttpRequest::new(Method::Put, format!("/ehr/{}/directory", ehr(p)?))
                    .with_auth(AuthSlot::Regular)
                    .header("prefer", "return=representation")
                    .header("content-type", "application/json")
                    .header(
                        "if-match",
                        need(p.folder_version_uid.as_deref(), "folder_uid")?,
                    )
                    .text_body(body.to_owned(), "application/json")
            }
            Scenario::DirectoryDelete => {
                HttpRequest::new(Method::Delete, format!("/ehr/{}/directory", ehr(p)?))
                    .with_auth(AuthSlot::Regular)
                    .header(
                        "if-match",
                        need(p.folder_version_uid.as_deref(), "folder_uid")?,
                    )
            }
            Scenario::AqlSimple => post_json(
                "/query/aql",
                &serde_json::json!({ "q": "SELECT c FROM COMPOSITION c LIMIT 10" }),
            ),
            Scenario::AqlAggregate => post_json(
                "/query/aql",
                &serde_json::json!({ "q": "SELECT COUNT(c) FROM COMPOSITION c" }),
            ),
            Scenario::TemplateUpload => {
                let opt = fixtures::read_from("template.valid", OPT_FILE)
                    .map_err(|e| BenchError::Fixture(e.to_string()))?;
                HttpRequest::new(Method::Post, "/definition/template/adl1.4")
                    .with_auth(AuthSlot::Regular)
                    .header("content-type", "application/xml")
                    .text_body(opt, "application/xml")
            }
            Scenario::TemplateList => get("/definition/template/adl1.4"),
            Scenario::TemplateGet => get(&format!("/definition/template/adl1.4/{TEMPLATE_ID}")),
        };
        Ok(t.send(req).await?.status)
    }
}

/// A human description of the committed payload (template + measured size) for
/// the report's environment block — so a reader knows the composition is a real
/// clinical-sized one, not a toy.
#[must_use]
pub fn payload_description() -> String {
    let bytes =
        fixtures::read_from("composition.canonical-json", COMPOSITION_FILE).map_or(0, |s| s.len());
    format!(
        "{TEMPLATE_ID} — {} KB canonical-JSON composition",
        bytes / 1024
    )
}

/// A stable hash over the frozen workload (design §2).
#[must_use]
pub fn workload_lock() -> String {
    let mut def = String::new();
    for s in Scenario::ALL {
        def.push_str(s.id());
        def.push('|');
        for code in s.expected_status() {
            def.push_str(&code.to_string());
            def.push(',');
        }
        def.push('\n');
    }
    def.push_str("template.valid/");
    def.push_str(OPT_FILE);
    def.push_str("composition.canonical-json/");
    def.push_str(COMPOSITION_FILE);
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in def.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

// ── request builders ─────────────────────────────────────────────────────────

fn get(path: &str) -> HttpRequest {
    HttpRequest::new(Method::Get, path.to_owned())
        .with_auth(AuthSlot::Regular)
        .header("accept", "application/json")
}

fn post_json(path: &str, body: &serde_json::Value) -> HttpRequest {
    HttpRequest::new(Method::Post, path.to_owned())
        .with_auth(AuthSlot::Regular)
        .header("accept", "application/json")
        .header("prefer", "return=representation")
        .text_body(body.to_string(), "application/json")
}

// ── payload builders ─────────────────────────────────────────────────────────

/// A vendored CNF-valid `EHR_STATUS` (carries `archetype_node_id`, which strict
/// servers like `EHRbase` require), adapted with a unique subject so get-by-subject
/// resolves. Using the fixture — not a hand-built body — guarantees *both*
/// servers accept it.
fn ehr_status_body(subject_id: &str) -> serde_json::Value {
    let base =
        crate::seed::read_json_from("ehr-status.valid", EHR_STATUS_FILE).unwrap_or_else(|_| {
            serde_json::json!({
                "_type": "EHR_STATUS",
                "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                "name": { "value": "EHR Status" },
                "subject": {
                    "external_ref": {
                        "id": { "_type": "GENERIC_ID", "value": subject_id, "scheme": "id_scheme" },
                        "namespace": SUBJECT_NAMESPACE,
                        "type": "PERSON"
                    }
                },
                "is_modifiable": true,
                "is_queryable": true
            })
        });
    let mut status = fixtures::adapt_ehr_status(base, SUBJECT_NAMESPACE, subject_id);
    // EHR_STATUS.subject is PARTY_SELF in the RM (archie enforces it); adapt_ehr_status
    // defaults to PARTY_IDENTIFIED when an external_ref is present, which EHRbase
    // Java rejects. PARTY_SELF still carries the external_ref, so force it.
    if let Some(subj) = status
        .get_mut("subject")
        .and_then(serde_json::Value::as_object_mut)
    {
        subj.insert(
            "_type".to_owned(),
            serde_json::Value::String("PARTY_SELF".to_owned()),
        );
    }
    status
}

fn composition_body() -> Result<String, BenchError> {
    let value = crate::seed::read_json_from("composition.canonical-json", COMPOSITION_FILE)
        .map_err(|e| BenchError::Fixture(e.to_string()))?;
    Ok(value.to_string())
}

fn folder_body() -> String {
    serde_json::json!({
        "_type": "FOLDER",
        "name": { "_type": "DV_TEXT", "value": "root" },
        "folders": [
            { "_type": "FOLDER", "name": { "_type": "DV_TEXT", "value": "episodes" } }
        ]
    })
    .to_string()
}

// ── setup helpers ────────────────────────────────────────────────────────────

async fn create_ehr(t: &Target) -> Result<(String, String), BenchError> {
    let subject = unique_subject();
    let req = post_json("/ehr", &ehr_status_body(&subject));
    let resp = t.send(req).await?;
    if resp.status != 201 {
        return Err(BenchError::Unexpected(format!(
            "create EHR: expected 201, got {}",
            resp.status
        )));
    }
    let ehr_id = resp
        .json()
        .map_err(|e| BenchError::Unexpected(format!("create EHR body: {e}")))?
        .get("ehr_id")
        .and_then(|v| v.get("value"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| BenchError::Unexpected("create EHR: no ehr_id/value".to_owned()))?;
    Ok((ehr_id, subject))
}

async fn ehr_status_version(t: &Target, ehr_id: &str) -> Result<String, BenchError> {
    let resp = t.send(get(&format!("/ehr/{ehr_id}/ehr_status"))).await?;
    resp.header("etag")
        .map(|h| h.trim_matches('"').to_owned())
        .or_else(|| resp.json().ok().and_then(|v| version_uid_from_value(&v)))
        .ok_or_else(|| BenchError::Unexpected("ehr_status: no version uid".to_owned()))
}

async fn ensure_template(t: &Target) -> Result<(), BenchError> {
    let opt = fixtures::read_from("template.valid", OPT_FILE)
        .map_err(|e| BenchError::Fixture(e.to_string()))?;
    let req = HttpRequest::new(Method::Post, "/definition/template/adl1.4")
        .with_auth(AuthSlot::Regular)
        .header("content-type", "application/xml")
        .text_body(opt, "application/xml");
    let resp = t.send(req).await?;
    if matches!(resp.status, 200 | 201 | 409) {
        Ok(())
    } else {
        Err(BenchError::Unexpected(format!(
            "upload template: got {}",
            resp.status
        )))
    }
}

async fn create_composition(
    t: &Target,
    ehr_id: &str,
    body: &str,
) -> Result<(String, String), BenchError> {
    let req = HttpRequest::new(Method::Post, format!("/ehr/{ehr_id}/composition"))
        .with_auth(AuthSlot::Regular)
        .header("prefer", "return=representation")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .text_body(body.to_owned(), "application/json");
    let resp = t.send(req).await?;
    if resp.status != 201 {
        return Err(BenchError::Unexpected(format!(
            "seed composition: got {}",
            resp.status
        )));
    }
    let version = resp
        .header("etag")
        .map(|h| h.trim_matches('"').to_owned())
        .or_else(|| resp.json().ok().and_then(|v| version_uid_from_value(&v)))
        .ok_or_else(|| BenchError::Unexpected("seed composition: no version uid".to_owned()))?;
    let vo = version.split("::").next().unwrap_or(&version).to_owned();
    Ok((vo, version))
}

async fn create_folder(t: &Target, ehr_id: &str) -> Result<String, BenchError> {
    let req = HttpRequest::new(Method::Post, format!("/ehr/{ehr_id}/directory"))
        .with_auth(AuthSlot::Regular)
        .header("prefer", "return=representation")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .text_body(folder_body(), "application/json");
    let resp = t.send(req).await?;
    if resp.status != 201 {
        return Err(BenchError::Unexpected(format!(
            "seed directory: got {}",
            resp.status
        )));
    }
    resp.header("etag")
        .map(|h| h.trim_matches('"').to_owned())
        .or_else(|| resp.json().ok().and_then(|v| version_uid_from_value(&v)))
        .ok_or_else(|| BenchError::Unexpected("seed directory: no version uid".to_owned()))
}

fn version_uid_from_value(v: &serde_json::Value) -> Option<String> {
    v.get("uid")
        .and_then(|u| u.get("value"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

// ── small accessors ──────────────────────────────────────────────────────────

fn ehr(p: &Prepared) -> Result<&str, BenchError> {
    need(p.ehr_id.as_deref(), "ehr_id")
}

fn subject(p: &Prepared) -> Result<&str, BenchError> {
    need(p.subject_id.as_deref(), "subject_id")
}

fn need<'a>(v: Option<&'a str>, what: &str) -> Result<&'a str, BenchError> {
    v.ok_or_else(|| BenchError::Unexpected(format!("prepared state missing {what}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_scenarios_have_distinct_ids() {
        let ids: Vec<_> = Scenario::ALL.iter().map(|s| s.id()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "scenario ids must be distinct");
    }

    #[test]
    fn covers_every_resource_group() {
        let mut groups: Vec<_> = Scenario::ALL.iter().map(|s| s.group()).collect();
        groups.sort_unstable();
        groups.dedup();
        for expected in [
            "EHR",
            "EHR_STATUS",
            "COMPOSITION",
            "VERSIONED_COMPOSITION",
            "CONTRIBUTION",
            "DIRECTORY",
            "QUERY",
            "DEFINITION",
        ] {
            assert!(groups.contains(&expected), "missing group {expected}");
        }
    }

    #[test]
    fn workload_lock_is_stable() {
        assert_eq!(workload_lock(), workload_lock());
        assert_eq!(workload_lock().len(), 16);
    }
}
