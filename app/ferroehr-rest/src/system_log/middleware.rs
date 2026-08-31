// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The IHE ATNA audit tower layer: request, resolved [`AuditEvent`], emit.
//!
//! One middleware installed outermost on the API router, wrapping the auth layer
//! so it observes every response, including the auth rejections the inner
//! handlers never see. ATNA requires one record per audited access describing
//! who acted, on what, with what outcome, from where and when; this layer
//! assembles that and hands it to the SM System Log component, the only
//! normative openEHR requirement being "System Log | IHE ATNA-compliant system
//! log" (SM `master02-overview.adoc` §openEHR Platform Model). The DICOM PS3.15
//! §A.5 rendering and the syslog transport belong to `ferroehr::system_log`.
//!
//! It reads three response extensions the rest of the stack sets, so no handler
//! carries audit code: [`AuditOpId`] (the matched operation id, from the generic
//! dispatch), [`AuditObject`] (the resource ids, from the `ResourceMeta` the
//! dispatch holds) and [`Principal`] (the caller, republished by the auth
//! middleware). Every generated operation is classified in
//! [`crate::system_log::classify`], and an id with no explicit entry fails closed
//! to the audited default rather than being dropped; where no `ResourceMeta`
//! exists the participant-object id comes from the request path.
//!
//! A genuine authentication additionally emits a login record (DICOM `EventID`
//! 110114, `EventTypeCode` 110122) unless `suppress_login_events` is on. Genuine
//! means a real authentication event: the auth layer marks it only on a Basic
//! verified-credential cache miss, where the credentials were actually checked.
//! Auth rejections always emit a failure record, with a 401's caller `UNKNOWN` —
//! surveillance of failed access is the point of ATNA.
//!
//! Emission is non-blocking (`try_send` onto a bounded queue), so the request
//! path never blocks on auditing. Under `fail_mode=closed` a rejected operation
//! record turns the response into `503`; the auth records never gate it.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::{HeaderValue, StatusCode, header};
use openehr_its::rest::runtime::ApiError;

use ferroehr::system_log::event::{
    AuditEvent, EmitOutcome, EventActionCode, EventOutcome, EventType, ObjectClass,
};

use crate::extensions::access::authn::{FreshAuthentication, Principal};
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::system_log::classify::audit_for;

/// The matched ITS-REST operation id, inserted by the dispatch layer onto the
/// response. Present only when a request reached the generic dispatcher.
///
/// The extension routers are mounted through the same `crate::api::mount` as the
/// generated groups, which inserts this uniformly, so extension traffic arrives
/// carrying its operation id. Those ids have no explicit entry in
/// [`crate::system_log::classify`], which fails an unrecognised id closed to the
/// audited default, so the operation record still fires.
#[derive(Debug, Clone, Copy)]
pub struct AuditOpId(pub &'static str);

/// The resource ids a handler touched, populated from the `ResourceMeta` the
/// dispatch already holds. Present for header-bearing operations.
#[derive(Debug, Clone, Default)]
pub struct AuditObject {
    /// The owning EHR id (for subject enrichment + the patient participant).
    pub ehr_id: Option<String>,
    /// The resource identifier (version uid / contribution uid / object URI).
    pub uid: Option<String>,
}

/// The ATNA audit middleware, routing emission through the platform's SM
/// `SystemLog` component.
pub async fn middleware(State(state): State<AppState>, req: Request, next: Next) -> Response {
    // The SM System Log master switch: disabled means no per-request work.
    if !state.backend().audit_enabled() {
        return next.run(req).await;
    }

    // Capture request-side facts before the request is consumed.
    let client_ip = client_ip(&req);
    let path = req.uri().path().to_owned();
    let timestamp = jiff::Timestamp::now();

    let resp = next.run(req).await;
    let status = resp.status();

    let op = resp.extensions().get::<AuditOpId>().copied();
    let principal = resp.extensions().get::<Principal>().cloned();
    let object = resp.extensions().get::<AuditObject>().cloned();
    // Republished by the tenant-resolution middleware, whose task-local scope
    // has exited by the time this outermost layer runs.
    let tenant = resp
        .extensions()
        .get::<ferroehr::extensions::tenant_context::TenantContext>()
        .map(|t| t.tenant_id);
    let fresh_auth = resp.extensions().get::<FreshAuthentication>().is_some();

    let mut op_rejected = false;

    // The operation record: `audit_for` fails an unrecognised op id closed to
    // the audited default, so only a deliberate opt-out yields `None`.
    if let Some(AuditOpId(op)) = op
        && let Some((action, object_class)) = audit_for(op)
    {
        let mut event = AuditEvent::new(action, object_class, outcome_from_status(status));
        // The concrete operation is the `EventTypeCode` (DICOM PS3.15 §A.5
        // EventIdentification); the id is ours.
        event.event_type = Some(EventType::RestOperation(op));
        fill_common(&mut event, principal.as_ref(), client_ip.clone(), timestamp);
        event.ehr_id = object
            .as_ref()
            .and_then(|o| o.ehr_id.clone())
            .or_else(|| ehr_id_from_path(&path));
        event.object_id = object
            .as_ref()
            .and_then(|o| o.uid.clone())
            .or_else(|| object_id_from_path(op, &path));
        event.tenant_id = tenant;
        op_rejected = state.backend().emit(event) == EmitOutcome::Rejected;
    }

    // The authentication record: a rejected access attempt is a failed
    // user-authentication event (DICOM PS3.15 §A.5.1 `EventID` 110114 with
    // `EventTypeCode` 110122), and the success record fires only on a genuine
    // authentication event, gated by `suppress_login_events`.
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        let mut event = AuditEvent::new(
            EventActionCode::Execute,
            ObjectClass::Authentication,
            outcome_from_status(status),
        );
        event.event_type = Some(EventType::Login);
        // A 401 has no principal, so the caller is `UNKNOWN`; a 403 has one.
        fill_common(&mut event, principal.as_ref(), client_ip, timestamp);
        event.tenant_id = tenant;
        let _ = state.backend().emit(event);
    } else if fresh_auth && !state.backend().suppress_login_events() {
        let mut event = AuditEvent::new(
            EventActionCode::Execute,
            ObjectClass::Authentication,
            EventOutcome::Success,
        );
        event.event_type = Some(EventType::Login);
        fill_common(&mut event, principal.as_ref(), client_ip, timestamp);
        event.tenant_id = tenant;
        let _ = state.backend().emit(event);
    }

    if op_rejected {
        // Fail-closed: an auditable operation whose record cannot be delivered
        // must not be reported as having succeeded. No openEHR spec governs the
        // error-body shape (a MAY, `Requests_and_responses.md` §HTTP status
        // codes) — our own design keeps every error path consistent.
        let mut resp = RestError(ApiError::ServiceUnavailable(
            "audit trail unavailable (fail-closed)".to_owned(),
        ))
        .into_response();
        resp.headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
        return resp;
    }
    resp
}

/// Maps an HTTP status to the DICOM `EventOutcomeIndicator` (PS3.15 §A.5.1).
///
/// 1xx to 3xx is `0` success (`304 Not Modified` is a successful conditional
/// read, RFC 9110 §15.4), 4xx is `4` minor failure, 5xx is `8` serious failure.
/// `12` major failure denotes a system-level abnormal termination and is not
/// inferable from an HTTP status, so it is never emitted here.
fn outcome_from_status(status: StatusCode) -> EventOutcome {
    if status.is_informational() || status.is_success() || status.is_redirection() {
        EventOutcome::Success
    } else if status.is_server_error() {
        EventOutcome::SeriousFailure
    } else {
        EventOutcome::MinorFailure
    }
}

/// Fills the participant-common fields: the requesting user (the DICOM source
/// `ActiveParticipant`), its network address, and the event time.
///
/// An absent principal leaves `user_id` empty, which the emitter renders as the
/// configured value-if-missing.
fn fill_common(
    event: &mut AuditEvent,
    principal: Option<&Principal>,
    client_ip: Option<String>,
    timestamp: jiff::Timestamp,
) {
    event.user_id = principal.map(|p| p.subject.clone()).unwrap_or_default();
    event.user_is_requestor = true;
    // The bearer token's `jti` (RFC 7519 §4.1.7) is the minimal token identity
    // IHE BALP `OAUTHaccessTokenUse.Minimal` records; token contents are never
    // logged.
    event.token_id = principal.and_then(token_id);
    event.client_ip = client_ip;
    event.timestamp = timestamp;
}

/// Returns the `jti` claim of a Bearer principal's validated token, if present
/// and a non-empty string.
fn token_id(principal: &Principal) -> Option<String> {
    principal
        .claims
        .get("jti")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// Returns the client network address for the DICOM `NetworkAccessPointID`: the
/// `X-Forwarded-For` first hop, else the TCP peer address, else `None`.
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

/// Extracts the `ehr_id` path parameter, the segment after the first exact `ehr`
/// segment.
///
/// `POST /ehr` has none, so its EHR id comes from the response `AuditObject`.
fn ehr_id_from_path(path: &str) -> Option<String> {
    segment_after(path, "ehr")
}

/// Derives the participant-object id from the request path for operations whose
/// dispatch carries no `ResourceMeta`: template ids, qualified stored-query
/// names, and demographic party uids.
///
/// `None` for list-style operations and for ad-hoc queries, which have no single
/// object.
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

/// Returns the path segment immediately following the first exact `marker`.
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
    fn outcome_codes_track_http_status() {
        assert_eq!(outcome_from_status(StatusCode::OK), EventOutcome::Success);
        assert_eq!(
            outcome_from_status(StatusCode::NO_CONTENT),
            EventOutcome::Success
        );
        // 304 Not Modified is a successful conditional read (RFC 9110 §15.4.5);
        // redirection is not a failed action.
        assert_eq!(
            outcome_from_status(StatusCode::NOT_MODIFIED),
            EventOutcome::Success
        );
        assert_eq!(
            outcome_from_status(StatusCode::BAD_REQUEST),
            EventOutcome::MinorFailure
        );
        assert_eq!(
            outcome_from_status(StatusCode::UNAUTHORIZED),
            EventOutcome::MinorFailure
        );
        assert_eq!(
            outcome_from_status(StatusCode::FORBIDDEN),
            EventOutcome::MinorFailure
        );
        assert_eq!(
            outcome_from_status(StatusCode::NOT_FOUND),
            EventOutcome::MinorFailure
        );
        assert_eq!(
            outcome_from_status(StatusCode::INTERNAL_SERVER_ERROR),
            EventOutcome::SeriousFailure
        );
        assert_eq!(
            outcome_from_status(StatusCode::SERVICE_UNAVAILABLE),
            EventOutcome::SeriousFailure
        );
    }

    #[test]
    fn ehr_id_from_various_paths() {
        assert_eq!(
            ehr_id_from_path("/ferroehr/rest/openehr/v1/ehr/abc-123/composition/x"),
            Some("abc-123".to_owned())
        );
        assert_eq!(
            ehr_id_from_path("/ferroehr/rest/openehr/v1/ehr/abc-123"),
            Some("abc-123".to_owned())
        );
        // Create has no id segment.
        assert_eq!(ehr_id_from_path("/ferroehr/rest/openehr/v1/ehr"), None);
        // Admin delete carries the id after `ehr`.
        assert_eq!(
            ehr_id_from_path("/ferroehr/rest/openehr/v1/admin/ehr/zzz"),
            Some("zzz".to_owned())
        );
        // "ehr_status" / "versioned_ehr_status" are not the exact `ehr` segment.
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
            object_id_from_path("query_execute_stored_query", "/x/query/eu.ferroehr::q1"),
            Some("eu.ferroehr::q1".to_owned())
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
