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
//! From those + the request (client IP, `ehr_id` path segment) + the response
//! status it builds a transport-agnostic [`AuditEvent`] and hands it to the
//! non-blocking [`AuditSender`]. The request path never blocks on auditing.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::StatusCode;

use ehrbase_audit::{
    AuditEvent, AuditSender, EmitOutcome, EventActionCode, EventOutcome, ObjectClass, audit_for,
};

use crate::auth::Principal;

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

/// The audit middleware. Installed via `from_fn_with_state(sender, middleware)`.
pub async fn middleware(State(sender): State<AuditSender>, req: Request, next: Next) -> Response {
    if !sender.enabled() {
        return next.run(req).await;
    }

    // Capture request-side facts before the request is consumed.
    let client_ip = client_ip(&req);
    let ehr_id_from_path = ehr_id_from_path(req.uri().path());
    let timestamp = jiff::Timestamp::now();

    let resp = next.run(req).await;
    let status = resp.status().as_u16();

    let op = resp.extensions().get::<AuditOpId>().copied();
    let principal = resp.extensions().get::<Principal>().cloned();
    let object = resp.extensions().get::<AuditObject>().cloned();

    let Some((event, is_operation)) = classify_event(
        op,
        principal.as_ref(),
        object.as_ref(),
        status,
        client_ip,
        ehr_id_from_path,
        timestamp,
        &sender,
    ) else {
        return resp;
    };

    match sender.emit(event) {
        // Fail-closed on an auditable operation whose record cannot be
        // delivered: the deployment demanded auditing it cannot provide (§8.4).
        EmitOutcome::Rejected if is_operation => (
            StatusCode::SERVICE_UNAVAILABLE,
            "audit trail unavailable (fail-closed)",
        )
            .into_response(),
        _ => resp,
    }
}

/// Decide whether/how to audit this response, returning the event and whether it
/// is an auditable *operation* (vs a login/authentication application-activity
/// event, which never triggers fail-closed `503`).
#[allow(clippy::too_many_arguments)]
fn classify_event(
    op: Option<AuditOpId>,
    principal: Option<&Principal>,
    object: Option<&AuditObject>,
    status: u16,
    client_ip: Option<String>,
    ehr_id_from_path: Option<String>,
    timestamp: jiff::Timestamp,
    sender: &AuditSender,
) -> Option<(AuditEvent, bool)> {
    match op {
        // A matched operation: audit it if it is in scope (§2 table).
        Some(AuditOpId(op)) => match audit_for(op) {
            Some((action, object_class)) => {
                let outcome = EventOutcome::from_http_status(status);
                let mut event = AuditEvent::new(action, object_class, outcome);
                fill_common(
                    &mut event,
                    principal,
                    object,
                    client_ip,
                    ehr_id_from_path,
                    timestamp,
                );
                Some((event, true))
            }
            // An out-of-scope (unaudited) operation reached by an authenticated
            // caller is a login / application-activity event (§4), suppressible.
            None => login_event(principal, client_ip, timestamp, sender),
        },
        // No operation matched: an auth rejection (401/403) is an authentication
        // event (outcome minor; user UNKNOWN on 401). Everything else (status /
        // health / unmatched) is not audited.
        None => {
            if status == 401 || status == 403 {
                let mut event = AuditEvent::new(
                    EventActionCode::Execute,
                    ObjectClass::ApplicationActivity,
                    EventOutcome::from_http_status(status),
                );
                fill_common(&mut event, principal, None, client_ip, None, timestamp);
                Some((event, false))
            } else {
                None
            }
        }
    }
}

/// A successful-authentication (login) application-activity event, gated by
/// `suppress_login_events`.
fn login_event(
    principal: Option<&Principal>,
    client_ip: Option<String>,
    timestamp: jiff::Timestamp,
    sender: &AuditSender,
) -> Option<(AuditEvent, bool)> {
    if principal.is_none() || sender.suppress_login_events() {
        return None;
    }
    let mut event = AuditEvent::new(
        EventActionCode::Execute,
        ObjectClass::ApplicationActivity,
        EventOutcome::Success,
    );
    fill_common(&mut event, principal, None, client_ip, None, timestamp);
    Some((event, false))
}

fn fill_common(
    event: &mut AuditEvent,
    principal: Option<&Principal>,
    object: Option<&AuditObject>,
    client_ip: Option<String>,
    ehr_id_from_path: Option<String>,
    timestamp: jiff::Timestamp,
) {
    event.user_id = principal.map(|p| p.subject.clone()).unwrap_or_default();
    event.user_is_requestor = true;
    event.client_ip = client_ip;
    event.timestamp = timestamp;
    // The object metadata the envelope carried wins; else the path's ehr_id.
    event.ehr_id = object.and_then(|o| o.ehr_id.clone()).or(ehr_id_from_path);
    event.object_id = object.and_then(|o| o.uid.clone());
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
    let mut segments = path.split('/').filter(|s| !s.is_empty());
    while let Some(seg) = segments.next() {
        if seg == "ehr" {
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
}
