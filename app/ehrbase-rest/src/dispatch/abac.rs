//! The ABAC policy-enforcement point (`docs/enterprise/access-control.md` §5.7,
//! §7): the local patient gate + PDP pre/post checks, driven from the generic
//! [`mount`](super::mount) closure so no per-operation code is needed.
//!
//! - **Pre-checks** run before the backend call — they have the op id, the
//!   resolved path params, the query string, and the request body (composition
//!   template, contribution template set).
//! - **Post-checks** run after the backend call on a successful response, using
//!   the [`AuditObject`](crate::audit::AuditObject) the dispatch already set
//!   (owning EHR id + resource version uid) — no body re-parsing.
//!
//! Every deny is a `403` and every engine failure a `500` (fail-closed, v1
//! parity), each carrying the [`Principal`] on the response extensions so the
//! ATNA audit layer records it for free. Query execution is **not** handled here
//! — its scope + post-check live in the query path (§6.4, step 8). All of this
//! is inert unless an [`AbacGate`](crate::access::authz::AbacGate) is wired
//! (`abac.enabled`). End-to-end coverage through the assembled router (pre-check
//! deny/allow, post-check deny, and the ATNA deny record) lives in
//! `tests/abac_e2e.rs`.
//!
//! The PEP helpers return `Result<(), Response>` (the deny/error path is a ready
//! axum `Response`, which is a large type) — `result_large_err` is allowed
//! module-wide accordingly.
#![allow(clippy::result_large_err)]

use axum::response::{IntoResponse, Response};

use crate::access::authz::{
    AccessMode, Attr, AuthzRequest, Decision, ResourceKind, access_of, kind_of,
};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::Composition;

use super::RequestParts;
use crate::access::authn::{Principal, current_principal};
use crate::access::authz::AbacGate;
use crate::error::RestError;
use ehrbase_sm::Platform;

use crate::state::AppState;
use crate::{negotiate, params};

/// Whether an operation is ABAC-checked, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Not ABAC-checked (item tags, definition/demographic/admin — RBAC only —,
    /// and query, whose scope + check live in the query path, §6.4).
    Skip,
    /// Checked before the backend call.
    Pre,
    /// Checked after the backend call, on a successful response.
    Post,
}

/// The §7 enforcement matrix, keyed by operation id.
#[allow(clippy::match_same_arms)] // grouped by resource family; explicitness is the point
fn mode_of(op: &str) -> Mode {
    match op {
        "ehr_create" | "ehr_create_with_id" | "ehr_get_by_id" | "ehr_get_by_subject" => Mode::Pre,
        "composition_create" | "composition_update" | "composition_delete" => Mode::Pre,
        "contribution_create" => Mode::Pre,
        "composition_get" => Mode::Post,
        "contribution_get" => Mode::Post,
        _ if op.starts_with("versioned_composition_") => Mode::Post,
        _ if op.starts_with("ehr_status_") || op.starts_with("versioned_ehr_status_") => Mode::Pre,
        _ if op.starts_with("directory_") => Mode::Pre,
        // Item tags, query (§6.4), definition/demographic/admin: not ABAC-checked.
        _ => Mode::Skip,
    }
}

/// The ABAC preparation for a query execution (§6.4): the patient subject scope
/// to thread into the SQL, and whether the executor should collect the touched
/// EHR/template sets for the post-check. `Ok((None, false))` when ABAC is off.
/// `Err(response)` is a ready 403 (a missing configured patient claim).
pub(super) fn query_pre<S: Platform>(
    state: &AppState<S>,
    op: &'static str,
) -> Result<(Option<String>, bool), Response> {
    let Some(handle) = state.authz() else {
        return Ok((None, false));
    };
    let Some(abac) = handle.abac() else {
        return Ok((None, false));
    };
    if kind_of(op) != Some(ResourceKind::Query) {
        return Ok((None, false));
    }
    let principal = current_principal().unwrap_or_else(fallback_principal);
    let patient = resolve_patient_claim(abac, &principal)?;
    // Collect attributes whenever ABAC is on so the post-check has the touched
    // template set; the subject scope pre-filters rows to the caller's patient.
    Ok((patient, true))
}

/// The ABAC post-check for a query execution (§6.4): the PDP fan-out over the
/// touched template set. An empty result permits (v1 parity). The patient gate
/// is already enforced by the subject-scope pre-filter (rows outside the
/// caller's patient are never fetched), so it is not re-run per EHR here.
pub(super) async fn query_post<S: Platform>(
    state: &AppState<S>,
    op: &'static str,
    outcome: &ehrbase_sm::QueryOutcome,
) -> Result<(), Response> {
    let Some(handle) = state.authz() else {
        return Ok(());
    };
    let Some(abac) = handle.abac() else {
        return Ok(());
    };
    if kind_of(op) != Some(ResourceKind::Query) {
        return Ok(());
    }
    // Empty result set → nothing touched → permit (§6.4).
    if outcome.ehr_ids.is_empty() && outcome.template_ids.is_empty() {
        return Ok(());
    }
    let principal = current_principal().unwrap_or_else(fallback_principal);
    let patient = resolve_patient_claim(abac, &principal)?;
    let template =
        (!outcome.template_ids.is_empty()).then(|| Attr::Set(outcome.template_ids.clone()));
    let req = AuthzRequest {
        operation_id: op,
        kind: ResourceKind::Query,
        access: AccessMode::Execute,
        organization: organization(abac, &principal),
        patient: patient.map(Attr::One),
        template,
    };
    decide(abac, &principal, &req).await
}

/// Pre-check the request. `Err(response)` short-circuits the dispatch with a
/// ready 403/500; `Ok(())` lets it proceed.
pub(super) async fn pre_check<S: Platform>(
    state: &AppState<S>,
    op: &'static str,
    parts: &RequestParts,
) -> Result<(), Response> {
    let Some(handle) = state.authz() else {
        return Ok(());
    };
    let Some(abac) = handle.abac() else {
        return Ok(());
    };
    if mode_of(op) != Mode::Pre {
        return Ok(());
    }
    let Some(kind) = kind_of(op) else {
        return Ok(());
    };
    // DIRECTORY is unchecked unless a directory policy is configured (v1 parity).
    if kind == ResourceKind::Directory && !abac.directory_checked {
        return Ok(());
    }

    let principal = current_principal()
        .ok_or_else(|| forbidden(&fallback_principal(), "authentication required for ABAC"))?;

    let patient = resolve_patient_claim(abac, &principal)?;
    let ehr_id = parts.path.get("ehr_id").cloned();

    // The local patient gate (§5.7), before any engine call.
    patient_gate(
        abac,
        &principal,
        patient.as_deref(),
        op,
        ehr_id.as_deref(),
        parts,
    )
    .await?;

    // Resolve the template attribute per resource kind.
    let template = pre_template(abac, &principal, op, kind, parts).await?;

    let req = AuthzRequest {
        operation_id: op,
        kind,
        access: access_of(op).unwrap_or(AccessMode::Read),
        organization: organization(abac, &principal),
        patient: patient.map(Attr::One),
        template,
    };
    decide(abac, &principal, &req).await
}

/// Post-check a successful response using the resource ids the dispatch recorded.
pub(super) async fn post_check<S: Platform>(
    state: &AppState<S>,
    op: &'static str,
    resp: Response,
) -> Response {
    if !resp.status().is_success() {
        return resp;
    }
    let Some(handle) = state.authz() else {
        return resp;
    };
    let Some(abac) = handle.abac() else {
        return resp;
    };
    if mode_of(op) != Mode::Post {
        return resp;
    }
    let Some(kind) = kind_of(op) else {
        return resp;
    };
    let Some(principal) = current_principal() else {
        return resp;
    };

    let object = resp
        .extensions()
        .get::<crate::audit::AuditObject>()
        .cloned();
    let ehr_id = object.as_ref().and_then(|o| o.ehr_id.clone());
    let uid = object.as_ref().and_then(|o| o.uid.clone());

    let patient = match resolve_patient_claim(abac, &principal) {
        Ok(p) => p,
        Err(deny) => return deny,
    };
    if let Err(deny) = subject_gate(abac, &principal, patient.as_deref(), ehr_id.as_deref()).await {
        return deny;
    }

    // Template attribute: for a composition read, from the returned version uid.
    let template = match kind {
        ResourceKind::Composition => match post_template(abac, &principal, uid.as_deref()).await {
            Ok(t) => t.map(Attr::One),
            Err(deny) => return deny,
        },
        _ => None,
    };

    let req = AuthzRequest {
        operation_id: op,
        kind,
        access: access_of(op).unwrap_or(AccessMode::Read),
        organization: organization(abac, &principal),
        patient: patient.map(Attr::One),
        template,
    };
    match decide(abac, &principal, &req).await {
        Ok(()) => resp,
        Err(deny) => deny,
    }
}

/// Resolve the patient claim, if the gate is configured. A configured-but-missing
/// claim is a 403 (v1 threw 500 — PORT NOTE: the improvement).
fn resolve_patient_claim(
    abac: &AbacGate,
    principal: &Principal,
) -> Result<Option<String>, Response> {
    let Some(claim) = &abac.patient_claim else {
        return Ok(None);
    };
    match crate::access::authz::claim_string(&principal.claims, claim) {
        Some(v) => Ok(Some(v)),
        None => Err(forbidden(
            principal,
            &format!("ABAC requires a '{claim}' claim on the access token"),
        )),
    }
}

/// The organization attribute, if the claim is configured and present.
fn organization(abac: &AbacGate, principal: &Principal) -> Option<String> {
    abac.organization_claim
        .as_ref()
        .and_then(|c| crate::access::authz::claim_string(&principal.claims, c))
}

/// The full pre-check patient gate (§5.7): the subject match for target-EHR ops,
/// plus the by-subject special case (compare the claim to the request subject).
async fn patient_gate(
    abac: &AbacGate,
    principal: &Principal,
    patient: Option<&str>,
    op: &str,
    ehr_id: Option<&str>,
    parts: &RequestParts,
) -> Result<(), Response> {
    if op == "ehr_get_by_subject" {
        // The request param IS the subject (v1); compare the claim to it.
        if let Some(patient) = patient {
            let subject = params::query_param(parts.query.as_deref(), "subject_id");
            if let Some(subject) = subject
                && subject != patient
            {
                return Err(forbidden(principal, "patient scope (subject mismatch)"));
            }
        }
        return Ok(());
    }
    subject_gate(abac, principal, patient, ehr_id).await
}

/// The subject-match gate over a target EHR: the claim must equal the EHR's
/// subject external ref (a subject-less EHR passes; §5.7).
async fn subject_gate(
    abac: &AbacGate,
    principal: &Principal,
    patient: Option<&str>,
    ehr_id: Option<&str>,
) -> Result<(), Response> {
    let (Some(patient), Some(ehr_id)) = (patient, ehr_id) else {
        return Ok(());
    };
    let subject = (abac.resolvers.subject)(ehr_id.to_owned())
        .await
        .map_err(|e| engine_error(principal, &format!("subject resolution: {e}")))?;
    match subject {
        Some(subject) if subject != patient => {
            Err(forbidden(principal, "patient scope (subject mismatch)"))
        }
        _ => Ok(()),
    }
}

/// The template attribute for a pre-checked op.
async fn pre_template(
    abac: &AbacGate,
    principal: &Principal,
    op: &str,
    kind: ResourceKind,
    parts: &RequestParts,
) -> Result<Option<Attr>, Response> {
    match op {
        "composition_create" | "composition_update" => {
            Ok(composition_template(parts).map(Attr::One))
        }
        "composition_delete" => {
            // The template of the preceding (current) version being deleted.
            let Some(uid) = parts.path.get("uid_based_id") else {
                return Ok(None);
            };
            let Some(vo_id) = vo_id_of(uid) else {
                return Ok(None);
            };
            let template = (abac.resolvers.template_of_version)(vo_id, None)
                .await
                .map_err(|e| engine_error(principal, &format!("template resolution: {e}")))?;
            Ok(template.map(Attr::One))
        }
        "contribution_create" => {
            let templates = contribution_templates(parts, principal)?;
            Ok((!templates.is_empty()).then_some(Attr::Set(templates)))
        }
        _ => {
            // EHR / EHR_STATUS / DIRECTORY carry no template.
            let _ = kind;
            Ok(None)
        }
    }
}

/// The template of a returned composition version (post-check), via the version
/// uid the dispatch recorded.
async fn post_template(
    abac: &AbacGate,
    principal: &Principal,
    uid: Option<&str>,
) -> Result<Option<String>, Response> {
    let Some(uid) = uid else {
        return Ok(None);
    };
    let Some(vo_id) = vo_id_of(uid) else {
        return Ok(None);
    };
    let version = version_of(uid);
    (abac.resolvers.template_of_version)(vo_id, version)
        .await
        .map_err(|e| engine_error(principal, &format!("template resolution: {e}")))
}

/// Extract the composition template id from the request body: the canonical
/// pointer `/archetype_details/template_id/value` (JSON or XML), or the
/// `templateId` query param for a FLAT/STRUCTURED body.
fn composition_template(parts: &RequestParts) -> Option<String> {
    let h = &parts.headers;
    if negotiate::is_flat_body(h) || negotiate::is_structured_body(h) {
        return params::query_param(parts.query.as_deref(), "templateId");
    }
    let value = negotiate::rm_value::<Composition>(h, &parts.body).ok()?;
    value
        .pointer("/archetype_details/template_id/value")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// The set of COMPOSITION template ids in a contribution payload. A COMPOSITION
/// version without a template is unresolvable → fail-closed (403, §6.3).
fn contribution_templates(
    parts: &RequestParts,
    principal: &Principal,
) -> Result<Vec<String>, Response> {
    let Ok(value) = negotiate::json_value(&parts.headers, &parts.body) else {
        // A malformed body: let the dispatch return the 400; nothing to check.
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    if let Some(versions) = value.get("versions").and_then(|v| v.as_array()) {
        for version in versions {
            let data = version.get("data");
            let is_composition =
                data.and_then(|d| d.get("_type")).and_then(|t| t.as_str()) == Some("COMPOSITION");
            if is_composition {
                match data
                    .and_then(|d| d.pointer("/archetype_details/template_id/value"))
                    .and_then(|v| v.as_str())
                {
                    Some(t) => out.push(t.to_owned()),
                    None => {
                        return Err(forbidden(
                            principal,
                            "contribution COMPOSITION version has no template id",
                        ));
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Run the engine and map the verdict: permit → `Ok`, deny → 403, error → 500.
async fn decide(
    abac: &AbacGate,
    principal: &Principal,
    req: &AuthzRequest<'_>,
) -> Result<(), Response> {
    match abac.engine.decide(req).await {
        Ok(Decision::Permit) => Ok(()),
        Ok(Decision::Deny) => Err(forbidden(principal, "policy denied")),
        Err(e) => Err(engine_error(principal, &e.to_string())),
    }
}

/// The `vo_id` of an `OBJECT_VERSION_ID` (`{vo_id}::{system}::{version}`), or the
/// whole string when it is a bare uid.
fn vo_id_of(uid: &str) -> Option<String> {
    let head = uid.split("::").next().unwrap_or(uid);
    (!head.is_empty()).then(|| head.to_owned())
}

/// The trailing version number of an `OBJECT_VERSION_ID`, if present.
fn version_of(uid: &str) -> Option<String> {
    // The VERSION_TREE_ID lexical form: `N` or `N.B.V` (trunk or branch —
    // RM common master06 §Version tree). Anything else carries no version.
    let tail = uid.rsplit("::").next()?;
    let mut parts = tail.split('.');
    let is_number = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(t), None, None, None) if is_number(t) => Some(tail.to_owned()),
        (Some(t), Some(b), Some(v), None) if is_number(t) && is_number(b) && is_number(v) => {
            Some(tail.to_owned())
        }
        _ => None,
    }
}

/// A 403 carrying the principal (audited by the ATNA layer).
fn forbidden(principal: &Principal, detail: &str) -> Response {
    let mut resp =
        RestError(ApiError::Forbidden(format!("access denied: {detail}"))).into_response();
    resp.extensions_mut().insert(principal.clone());
    resp
}

/// A 500 (fail-closed) carrying the principal.
fn engine_error(principal: &Principal, detail: &str) -> Response {
    let mut resp = RestError(ApiError::Internal(format!(
        "authorization unavailable: {detail}"
    )))
    .into_response();
    resp.extensions_mut().insert(principal.clone());
    resp
}

/// A principal placeholder for the (unreachable) no-principal path.
fn fallback_principal() -> Principal {
    Principal {
        subject: "UNKNOWN".to_owned(),
        scopes: Vec::new(),
        roles: Vec::new(),
        claims: serde_json::Map::new(),
        method: crate::access::authn::AuthMethod::Bearer,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::access::authz::engine::{AuthzError, PolicyEngine};
    use async_trait::async_trait;
    use http::StatusCode;

    use super::*;
    use crate::access::authn::AuthMethod;
    use crate::access::authz::{AuthzResolvers, ResolveError};

    /// A counting engine: records how often `decide` is called (the patient gate
    /// must deny *without* reaching it), always permits.
    #[derive(Debug, Default)]
    struct CountingEngine(Arc<AtomicUsize>);

    #[async_trait]
    impl PolicyEngine for CountingEngine {
        async fn decide(&self, _req: &AuthzRequest<'_>) -> Result<Decision, AuthzError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Decision::Permit)
        }
    }

    fn gate_with_subject(subject: Option<&'static str>, calls: Arc<AtomicUsize>) -> AbacGate {
        let subject = subject.map(str::to_owned);
        AbacGate {
            engine: Arc::new(CountingEngine(calls)),
            resolvers: AuthzResolvers {
                subject: Arc::new(move |_ehr_id: String| {
                    let subject = subject.clone();
                    Box::pin(async move { Ok::<_, ResolveError>(subject) })
                }),
                template_of_version: Arc::new(|_vo: String, _v: Option<String>| {
                    Box::pin(async move { Ok::<_, ResolveError>(None) })
                }),
            },
            organization_claim: None,
            patient_claim: Some("patient_id".to_owned()),
            directory_checked: false,
        }
    }

    fn principal_with_patient(patient: &str) -> Principal {
        let mut claims = serde_json::Map::new();
        claims.insert("patient_id".to_owned(), serde_json::json!(patient));
        Principal {
            subject: "alice".to_owned(),
            scopes: Vec::new(),
            roles: vec!["USER".to_owned()],
            claims,
            method: AuthMethod::Bearer,
        }
    }

    #[tokio::test]
    async fn subject_mismatch_denies_without_engine_call() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = gate_with_subject(Some("P2"), calls.clone());
        let principal = principal_with_patient("P1");
        let err = subject_gate(&gate, &principal, Some("P1"), Some("ehr-1"))
            .await
            .expect_err("mismatch must deny");
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "no engine call on gate deny"
        );
    }

    #[tokio::test]
    async fn subject_match_passes_gate() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = gate_with_subject(Some("P1"), calls);
        let principal = principal_with_patient("P1");
        assert!(
            subject_gate(&gate, &principal, Some("P1"), Some("ehr-1"))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn null_subject_passes_gate() {
        // A subject-less EHR is not patient-scoped (v1 parity, §5.7).
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = gate_with_subject(None, calls);
        let principal = principal_with_patient("P1");
        assert!(
            subject_gate(&gate, &principal, Some("P1"), Some("ehr-1"))
                .await
                .is_ok()
        );
    }

    #[test]
    fn missing_patient_claim_is_forbidden() {
        let calls = Arc::new(AtomicUsize::new(0));
        let gate = gate_with_subject(Some("P1"), calls);
        // A principal with no patient claim configured-but-absent → 403.
        let principal = Principal {
            subject: "alice".to_owned(),
            scopes: Vec::new(),
            roles: vec!["USER".to_owned()],
            claims: serde_json::Map::new(),
            method: AuthMethod::Bearer,
        };
        let err = resolve_patient_claim(&gate, &principal).expect_err("missing claim");
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn mode_matrix() {
        assert_eq!(mode_of("ehr_create"), Mode::Pre);
        assert_eq!(mode_of("ehr_get_by_subject"), Mode::Pre);
        assert_eq!(mode_of("composition_create"), Mode::Pre);
        assert_eq!(mode_of("composition_delete"), Mode::Pre);
        assert_eq!(mode_of("composition_get"), Mode::Post);
        assert_eq!(mode_of("versioned_composition_get"), Mode::Post);
        assert_eq!(mode_of("contribution_create"), Mode::Pre);
        assert_eq!(mode_of("contribution_get"), Mode::Post);
        assert_eq!(mode_of("ehr_status_update"), Mode::Pre);
        assert_eq!(mode_of("versioned_ehr_status_get"), Mode::Pre);
        assert_eq!(mode_of("directory_create"), Mode::Pre);
        // Query is handled in the query path, not the generic PEP.
        assert_eq!(mode_of("query_execute_adhoc_query"), Mode::Skip);
        assert_eq!(mode_of("ehr_tags_get"), Mode::Skip);
        assert_eq!(mode_of("definition_template_adl1.4_upload"), Mode::Skip);
    }

    #[test]
    fn object_version_id_parsing() {
        let uid = "0197f1c2-3aa0-7000-8000-000000000001::ehrbase.local::2";
        assert_eq!(
            vo_id_of(uid).as_deref(),
            Some("0197f1c2-3aa0-7000-8000-000000000001")
        );
        assert_eq!(version_of(uid), Some("2".to_owned()));
        // A branch VERSION_TREE_ID is carried whole (RM common master06
        // §Version tree).
        assert_eq!(
            version_of("0197f1c2-3aa0-7000-8000-000000000001::sys::2.1.3"),
            Some("2.1.3".to_owned())
        );
        // A bare uid (no version) → itself, no version.
        assert_eq!(vo_id_of("abc").as_deref(), Some("abc"));
        assert_eq!(version_of("abc"), None);
    }
}
