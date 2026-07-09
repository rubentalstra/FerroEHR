//! The ATNA audit tower layer (binding doc §8.2).
//!
//! One middleware, installed **outermost** on the API router (wrapping the auth
//! layer) so it observes every response — including auth rejections the inner
//! handlers never see. It reads three response extensions the rest of the stack
//! sets with zero per-handler code:
//!
//! - [`AuditOpId`] — the matched ITS-REST operation id, inserted once in the
//!   generic dispatch path ([`crate::dispatch`]).
//! - [`AuditObject`] — the resource ids, populated from the `ResourceMeta` the
//!   dispatch already holds (via `negotiate::set_resource_headers`).
//! - [`Principal`] — the authenticated caller, republished onto the response by
//!   the auth middleware ([`crate::auth`]).
//!
//! Coverage is **total**: every generated operation is in the audit table
//! ([`crate::audit_table`]), so every matched request emits an operation
//! record. When the envelope carries no `ResourceMeta` (templates, stored
//! queries, demographic parties), the participant-object id is derived
//! generically from the request path ([`object_id_from_path`]). Successful
//! authentications additionally emit a login ("Application Activity") record
//! unless `suppress_login_events` (§4); auth rejections (401/403) emit a
//! failure record with the principal `UNKNOWN` on 401.
//!
//! Emission goes through the platform's SM [`SystemLog`](ehrbase_sm::SystemLog)
//! component ([`AppState::backend`]): the request path never blocks on auditing
//! (`SystemLog::emit` is a non-blocking `try_send`); fail-closed only rejects
//! auditable *operations* (§8.4).

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::StatusCode;

use ehrbase_sm::{AuditEvent, EmitOutcome, EventActionCode, EventOutcome, ObjectClass, Platform};

use crate::access::authn::Principal;
use crate::audit_table::audit_for;
use crate::state::AppState;

/// The matched ITS-REST operation id, inserted by the dispatch layer onto the
/// response (§8.2 step 1). Present iff a request reached the generic dispatcher.
#[derive(Debug, Clone, Copy)]
pub struct AuditOpId(pub &'static str);

/// The resource ids a handler touched, populated from the `ResourceMeta` the
/// dispatch already holds (§8.2 step 3). Present for header-bearing operations.
#[derive(Debug, Clone, Default)]
pub struct AuditObject {
    /// The owning EHR id (for subject enrichment + the patient participant).
    pub ehr_id: Option<String>,
    /// The resource identifier (version uid / contribution uid / object URI).
    pub uid: Option<String>,
}

/// The audit middleware. Installed via `from_fn_with_state(state, middleware)`;
/// emission is routed through the platform's SM `SystemLog` component.
pub async fn middleware<S: Platform>(
    State(state): State<AppState<S>>,
    req: Request,
    next: Next,
) -> Response {
    if !state.backend().audit_enabled() {
        return next.run(req).await;
    }

    // Capture request-side facts before the request is consumed.
    let client_ip = client_ip(&req);
    let path = req.uri().path().to_owned();
    let timestamp = jiff::Timestamp::now();

    let resp = next.run(req).await;
    let status = resp.status().as_u16();

    let op = resp.extensions().get::<AuditOpId>().copied();
    let principal = resp.extensions().get::<Principal>().cloned();
    let object = resp.extensions().get::<AuditObject>().cloned();

    let mut op_rejected = false;

    // 1) The operation record — every generated operation is audited (total
    //    coverage); fail-closed applies to these only.
    if let Some(AuditOpId(op)) = op
        && let Some((action, object_class)) = audit_for(op)
    {
        let mut event = AuditEvent::new(action, object_class, outcome_from_status(status));
        fill_common(&mut event, principal.as_ref(), client_ip.clone(), timestamp);
        event.ehr_id = object
            .as_ref()
            .and_then(|o| o.ehr_id.clone())
            .or_else(|| ehr_id_from_path(&path));
        event.object_id = object
            .as_ref()
            .and_then(|o| o.uid.clone())
            .or_else(|| object_id_from_path(op, &path));
        op_rejected = state.backend().emit(event) == EmitOutcome::Rejected;
    }

    // 2) The authentication record. Failures (401 unauthenticated, 403
    //    forbidden — e.g. the admin-scope gate) are ALWAYS audited (security
    //    surveillance is the point); `suppress_login_events` gates only the
    //    successful-login "Application Activity" record (§4).
    if status == 401 || status == 403 {
        let mut event = AuditEvent::new(
            EventActionCode::Execute,
            ObjectClass::ApplicationActivity,
            outcome_from_status(status),
        );
        // 401 → principal UNKNOWN; 403 → attributed to the authenticated caller.
        fill_common(&mut event, principal.as_ref(), client_ip, timestamp);
        let _ = state.backend().emit(event);
    } else if principal.is_some() && !state.backend().suppress_login_events() {
        let mut event = AuditEvent::new(
            EventActionCode::Execute,
            ObjectClass::ApplicationActivity,
            EventOutcome::Success,
        );
        fill_common(&mut event, principal.as_ref(), client_ip, timestamp);
        let _ = state.backend().emit(event);
    }

    if op_rejected {
        // Fail-closed on an auditable operation whose record cannot be
        // delivered: the deployment demanded auditing it cannot provide (§8.4).
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "audit trail unavailable (fail-closed)",
        )
            .into_response();
    }
    resp
}

/// Map an HTTP status code to a DICOM `EventOutcomeIndicator` (binding doc §8.2):
/// 2xx→Success, 5xx→SeriousFailure, everything else (incl. 401/403/4xx)→MinorFailure.
const fn outcome_from_status(status: u16) -> EventOutcome {
    match status {
        200..=299 => EventOutcome::Success,
        500..=599 => EventOutcome::SeriousFailure,
        _ => EventOutcome::MinorFailure,
    }
}

fn fill_common(
    event: &mut AuditEvent,
    principal: Option<&Principal>,
    client_ip: Option<String>,
    timestamp: jiff::Timestamp,
) {
    event.user_id = principal.map(|p| p.subject.clone()).unwrap_or_default();
    event.user_is_requestor = true;
    event.client_ip = client_ip;
    event.timestamp = timestamp;
}

/// The client network address: the `X-Forwarded-For` first hop, else the TCP
/// peer address (`ConnectInfo`), else `None`.
fn client_ip(req: &Request) -> Option<String> {
    if let Some(xff) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        && let Some(first) = xff.split(',').next()
    {
        let trimmed = first.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
}

/// Extract the `ehr_id` path parameter (the segment after the first exact `ehr`
/// segment), if any. `POST /ehr` (create) has none — the EHR id comes from the
/// response `AuditObject` instead.
fn ehr_id_from_path(path: &str) -> Option<String> {
    segment_after(path, "ehr")
}

/// Derive the participant-object id from the request path for the operations
/// whose dispatch carries no `ResourceMeta`: template ids, qualified stored
/// query names (§3: the query name is the search criteria), and demographic
/// party uids. Returns `None` for list-style operations (no single object) and
/// for ad-hoc queries (search criteria = `UNKNOWN` per §3).
fn object_id_from_path(op: &str, path: &str) -> Option<String> {
    match op {
        _ if op.starts_with("definition_template_adl1.4") => segment_after(path, "adl1.4"),
        _ if op.starts_with("definition_template_adl2") => segment_after(path, "adl2"),
        // /definition/query/{name} and /query/{name} — but never /query/aql.
        _ if op.starts_with("definition_query") => segment_after(path, "query"),
        "query_execute_stored_query"
        | "query_execute_stored_query_body"
        | "query_execute_stored_query_version"
        | "query_execute_stored_query_version_body" => segment_after(path, "query"),
        _ if op.starts_with("agent_") => segment_after(path, "agent"),
        _ if op.starts_with("group_") => segment_after(path, "group"),
        _ if op.starts_with("organisation_") => segment_after(path, "organisation"),
        _ if op.starts_with("person_") => segment_after(path, "person"),
        _ if op.starts_with("role_") => segment_after(path, "role"),
        _ if op.starts_with("versioned_party_") => segment_after(path, "versioned_party"),
        _ => None,
    }
}

/// The path segment immediately following the first exact `marker` segment.
fn segment_after(path: &str, marker: &str) -> Option<String> {
    let mut segments = path.split('/').filter(|s| !s.is_empty());
    while let Some(seg) = segments.next() {
        if seg == marker {
            return segments.next().filter(|s| !s.is_empty()).map(str::to_owned);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ehr_id_from_various_paths() {
        assert_eq!(
            ehr_id_from_path("/ehrbase/rest/openehr/v1/ehr/abc-123/composition/x"),
            Some("abc-123".to_owned())
        );
        assert_eq!(
            ehr_id_from_path("/ehrbase/rest/openehr/v1/ehr/abc-123"),
            Some("abc-123".to_owned())
        );
        // Create has no id segment.
        assert_eq!(ehr_id_from_path("/ehrbase/rest/openehr/v1/ehr"), None);
        // Admin delete carries the id after `ehr`.
        assert_eq!(
            ehr_id_from_path("/ehrbase/rest/openehr/v1/admin/ehr/zzz"),
            Some("zzz".to_owned())
        );
        // "ehr_status" / "versioned_ehr_status" are not the `ehr` segment.
        assert_eq!(ehr_id_from_path("/definition/template/adl1.4"), None);
    }

    #[test]
    fn object_id_from_template_paths() {
        assert_eq!(
            object_id_from_path(
                "definition_template_adl1.4_get",
                "/x/definition/template/adl1.4/vital_signs.v1"
            ),
            Some("vital_signs.v1".to_owned())
        );
        // Upload/list have no id segment.
        assert_eq!(
            object_id_from_path(
                "definition_template_adl1.4_upload",
                "/x/definition/template/adl1.4"
            ),
            None
        );
        assert_eq!(
            object_id_from_path(
                "definition_template_adl2_version_get",
                "/x/definition/template/adl2/t.v1/1.0.0"
            ),
            Some("t.v1".to_owned())
        );
    }

    #[test]
    fn object_id_from_query_paths() {
        // Stored query execution: the qualified name is the search criteria.
        assert_eq!(
            object_id_from_path("query_execute_stored_query", "/x/query/org.ehrbase::q1"),
            Some("org.ehrbase::q1".to_owned())
        );
        assert_eq!(
            object_id_from_path("definition_query_store.yaml", "/x/definition/query/org::q2"),
            Some("org::q2".to_owned())
        );
        // Ad-hoc queries stay UNKNOWN (no extraction).
        assert_eq!(
            object_id_from_path("query_execute_adhoc_query", "/x/query/aql"),
            None
        );
    }

    #[test]
    fn object_id_from_demographic_paths() {
        assert_eq!(
            object_id_from_path("person_get", "/x/demographic/person/p-1"),
            Some("p-1".to_owned())
        );
        assert_eq!(
            object_id_from_path("role_tags_delete", "/x/demographic/role/r-9/tags/k"),
            Some("r-9".to_owned())
        );
        assert_eq!(
            object_id_from_path("versioned_party_get", "/x/demographic/versioned_party/vp-3"),
            Some("vp-3".to_owned())
        );
        // Create has no id segment.
        assert_eq!(
            object_id_from_path("person_create", "/x/demographic/person"),
            None
        );
    }
}
