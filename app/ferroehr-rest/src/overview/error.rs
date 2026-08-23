// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Mapping the contract's [`ApiError`] onto an HTTP response with an openEHR
//! error body.
//!
//! The generated server traits return [`openehr_its::rest::runtime::ApiError`],
//! which carries the ITS-REST status code but renders as a bare string. The
//! REST layer wraps it in [`RestError`] so handlers can use `?` and every error
//! leaves the server as a structured JSON body.
//!
//! ONE body shape, uniform across every non-2xx (adjudicated on #2604):
//! `{ "error", "message", "validationErrors" }` — the openEHR `Error`
//! object's members
//! (`docs/specs/openehr/ITS-REST/specifications/schemas/others/Error.yaml`:
//! `required: [message, validationErrors]`, additional members tolerated)
//! plus our `error` reason-phrase extra. A semantic-validation failure
//! ([`ApiError::ValidationFailed`], HTTP `422`) populates the list with its
//! `"<path>: <message>"` violations; every other error carries it empty.
//!
//! NOTE: the released assignment is narrow — only the OAS `400.yaml` /
//! `400_CONTRIBUTION.yaml` attach `Error.yaml`, the docs text makes the body
//! a MAY with no shape, and `422.yaml` declares no content at all — so the
//! uniform shape beyond the 400 surface is our own design, flagged here.

#![allow(
    clippy::disallowed_types,
    reason = "test fixtures and wire assertions are raw JSON by the testing rule \
              (.claude/rules/testing.md §Test-fixture construction); the carriers here are \
              cfg(test)-only, so #[expect] would be unfulfilled in the non-test build"
)]

use axum::response::{IntoResponse, Response};
use http::{HeaderValue, StatusCode, header};
use serde::Serialize;

use ferroehr::service::error::{ErrorChain, ServiceError};
use ferroehr::service::status::{CallStatusType, QUERY_TIMEOUT_TAG, SmError};
use ferroehr::versioning::object_version_id::VersionIdError;
use openehr_its::rest::runtime::ApiError;

/// A response-rendering wrapper over the contract's [`ApiError`].
#[derive(Debug)]
pub struct RestError(pub ApiError);

/// The client-visible message of a server-side fault (`500`) raised inside the
/// protocol adapter. Deliberately opaque, for the same reason the platform's
/// own 500-class message is: a codec/serializer diagnostic names Rust types,
/// RM element names and parser offsets — server-internal detail the client can
/// neither act on nor be trusted with. The detail rides one structured
/// `tracing` record instead ([`internal_fault`]). ITS-REST overview §HTTP
/// status codes fixes the `{ error, message }` shape but not the wording; the
/// opacity is our own design.
pub(crate) const INTERNAL_MESSAGE: &str = "the server encountered an internal error";

/// Record an adapter-side fault on the trace record and return the curated
/// opaque `500` [`ApiError`] its body carries.
///
/// `context` names the step that failed (a static call-site label); `detail` is
/// the raw diagnostic — a serde error, an XML codec failure, a middleware error
/// — which is written to `tracing` and NEVER to the wire.
pub(crate) fn internal_fault(context: &'static str, detail: &dyn std::fmt::Display) -> ApiError {
    tracing::error!(context, error = %detail, "protocol adapter: internal fault → 500");
    ApiError::Internal(INTERNAL_MESSAGE.to_owned())
}

/// Record an adapter-side fault AND its whole cause chain on the trace record,
/// and return the curated opaque `500` [`ApiError`] its body carries.
///
/// The sibling of [`internal_fault`] for a fault that carries a
/// [`std::error::Error`] source: the `cause` field is the walked
/// [`std::error::Error::source`] chain ([`ErrorChain`]), which is the only place
/// the underlying driver/codec diagnosis is readable — it never reaches the
/// wire.
pub(crate) fn internal_fault_caused(
    context: &'static str,
    error: &(dyn std::error::Error + 'static),
) -> ApiError {
    if let Some(cause) = error.source() {
        tracing::error!(
            context,
            error = %error,
            cause = %ErrorChain::new(cause),
            "protocol adapter: internal fault → 500"
        );
    } else {
        tracing::error!(context, error = %error, "protocol adapter: internal fault → 500");
    }
    ApiError::Internal(INTERNAL_MESSAGE.to_owned())
}

impl From<ApiError> for RestError {
    fn from(e: ApiError) -> Self {
        Self(e)
    }
}

impl From<VersionIdError> for RestError {
    /// A malformed `uid_based_id` / `version_uid` wire value is a `400`: the
    /// platform decoder ([`ferroehr::versioning::object_version_id`]) already
    /// classifies *why* the identifier was rejected, and its own
    /// [`ApiError`] mapping fixes the status — the adapter only lifts it into
    /// the response wrapper.
    fn from(e: VersionIdError) -> Self {
        Self(ApiError::from(e))
    }
}

/// The single SM → HTTP mapping, owned by the protocol adapter: a native [`SmError`]
/// carries only a `CALL_STATUS_TYPE`, and this adapter turns its status into the
/// ITS-REST 1.1.0 status code. The wire oracle (ITS-REST) decides each row;
/// where the SM name and the wire disagree, the wire's status wins here. Living
/// in `ferroehr-rest` keeps `ferroehr-sm` protocol-free (no `openehr_its::rest`
/// dependency). (A free function, not `impl From<SmError> for ApiError`, because
/// both types are foreign to this crate — the orphan rule forbids that impl.)
#[must_use]
pub(crate) fn sm_api_error(e: SmError) -> ApiError {
    use CallStatusType as S;
    let message = e.message;
    match e.status {
        S::AuthFailure => ApiError::Forbidden(message),
        S::PreconditionViolation | S::InvalidIdPattern => ApiError::BadRequest(message),
        S::ObjectVersionDoesNotExist
        | S::VersionedObjectDoesNotExist
        | S::EhrIdDoesNotExist
        | S::PartyIdDoesNotExist
        | S::CompositionDoesNotExist
        | S::ContributionDoesNotExist
        | S::ArtefactDoesNotExist
        | S::TemplateDoesNotExist
        | S::VersionDoesNotExist
        | S::SubjectIdDoesNotExist
        | S::VersionedCompositionDoesNotExist => ApiError::NotFound(message),
        S::VersionMismatch => ApiError::PreconditionFailed(message),
        // The specific conflicts plus the storage-classified generic conflict
        // (integrity/serialization) all map to `409`.
        S::EhrCreateFailDuplicateId
        | S::CompositionAlreadyExists
        | S::EhrForSubjectAlreadyExists
        | S::Conflict => ApiError::Conflict(message),
        S::CompositionArchetypeInvalid
        | S::InvalidArchetype
        | S::InvalidTemplate
        | S::InvalidArtefact
        | S::InvalidQuery
        | S::DefinitionUnknown
        | S::ContentInvalid => ApiError::Unprocessable(message),
        S::NotImplemented => ApiError::NotImplemented,
        // Backend resource exhaustion (pool acquire timeout; our overload
        // contract, spec-silent — RFC 9110 §15.6.4 is the HTTP authority) → 503.
        // `RestError::into_response` adds the `Retry-After` hint for any 503.
        S::ServiceOverloaded => ApiError::ServiceUnavailable(message),
        // `success` is not an error; mapping it is defensively a 500.
        S::Success | S::FileNotWritable | S::Exception => ApiError::Internal(message),
    }
}

impl From<SmError> for RestError {
    fn from(e: SmError) -> Self {
        Self(sm_api_error(e))
    }
}

impl From<ServiceError> for RestError {
    /// Map a service failure straight onto the wire error, preserving the
    /// structured per-path/per-code violations of
    /// [`ServiceError::ValidationFailed`] (the ITS-REST `Error` object) that the
    /// `SmError` bridge collapses into a flat message. Used by the wire methods
    /// that surface validation codes directly (the ADL2 upload).
    fn from(e: ServiceError) -> Self {
        Self(ApiError::from(e))
    }
}

/// The ONE JSON error body this server emits, uniform across every non-2xx.
///
/// The released assignment is narrow (adjudicated on #2604): only the OAS
/// `responses/400.yaml` / `400_CONTRIBUTION.yaml` attach
/// `schemas/others/Error.yaml` (`required: [message, validationErrors]`,
/// additional members tolerated) to a 400 JSON body, and the docs text
/// (`Requests_and_responses.md` §HTTP status codes) makes the body itself a
/// MAY with no shape assignment — the `{message, code, errors}` block there
/// is an example. Every other status's body is assigned by no source — our
/// own design, kept uniform so a client parses one shape everywhere; `error`
/// (the reason phrase) is our extra member on all of them.
#[derive(Debug, Serialize)]
struct ErrorBody {
    /// Machine-readable status label (the reason phrase, e.g. `Not Found`).
    error: String,
    /// Human-readable detail.
    message: String,
    /// Per-path violations (`"<path>: <message>"`); empty when the failure
    /// carries none — always emitted, so the 400 surface satisfies
    /// `Error.yaml`'s required member list.
    #[serde(rename = "validationErrors")]
    validation_errors: Vec<String>,
}

/// Render an arbitrary status as the `{ error, message }` openEHR error body.
/// Used for the method-status responses (`405`/`501`) that have no dedicated
/// [`ApiError`] variant (the contract's `ApiError` cannot represent `405`), and
/// for the transport-layer statuses (`408`/`413`) whose middleware default body
/// is aligned onto this shape ([`crate::router()`]).
pub(crate) fn status_error_response(status: StatusCode, message: &str) -> Response {
    let body = ErrorBody {
        error: status.canonical_reason().unwrap_or("Error").to_owned(),
        message: message.to_owned(),
        validation_errors: Vec::new(),
    };
    let json = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    resp
}

// ── HTTP-method status discipline (overview §HTTP Methods) ──────────────────
// "A server receiving an unrecognized or unimplemented method SHOULD respond
// with the `501 Not Implemented` status code. If a method is recognized but not
// allowed for the target resource, the response SHOULD be `405 Method Not
// Allowed`." [`method_not_allowed_handler`] is the API router's
// `method_not_allowed_fallback`, rendering the openEHR `{ error, message }`
// body with the `Allow` RFC 9110 §15.5.6 mandates; a recognised but
// unimplemented *operation* answers `501` via `ApiError::NotImplemented`.
// NOTE: no released text can be honoured for an *unrecognized method* here —
// axum exposes no such seam — so it is answered `405`, itself a predefined code
// in the spec's own status table; the rationale lives in `crate::router`.

/// Axum fallback for a request whose method is not served by the matched
/// resource → `405 Method Not Allowed` (overview §HTTP Methods) with the
/// openEHR `{ error, message }` body.
///
/// The mandatory `Allow` header (RFC 9110 §15.5.6) is **supplied by axum, not
/// here**, and deliberately so: only the router knows the matched path's method
/// set, and axum decorates a method-fallback response with the `Allow` it
/// accumulated from the route's registered methods — but only when the response
/// does not already carry one (`axum::routing::Route`'s `set_allow_header`;
/// <https://docs.rs/axum/0.8/axum/struct.Router.html#method.method_not_allowed_fallback>).
/// Setting a hand-built `Allow` here would therefore *replace* the accurate
/// per-route set with a guess. The header's presence is pinned by
/// `app/ferroehr-rest/tests/http.rs`.
///
/// A `405` produced from a **matched** handler (the config-gated admin group)
/// never reaches this decoration and must carry its own `Allow` — see
/// [`method_not_allowed_response`].
pub(crate) async fn method_not_allowed_handler() -> Response {
    status_error_response(
        StatusCode::METHOD_NOT_ALLOWED,
        "the request method is not allowed on this resource",
    )
}

/// Render a `405 Method Not Allowed` **from a matched handler**, with the
/// openEHR `{ error, message }` body and an explicit `Allow` header.
///
/// RFC 9110 §15.5.6: "The origin server MUST generate an Allow header field in
/// a 405 response containing a list of the target resource's currently
/// supported methods." A handler-produced `405` bypasses axum's allow-header
/// machinery entirely (that decoration only runs on the *method fallback*, i.e.
/// when no method route matched), so the caller states the set itself.
///
/// `allow` is the RFC 9110 §10.2.1 `Allow = #method` field value — a
/// comma-separated method list, or the empty string where the resource
/// currently supports no method at all ("An empty Allow field value indicates
/// that the resource allows no methods, which might occur in a 405 response if
/// the resource has been temporarily disabled by configuration").
///
/// # Panics
/// If `allow` is not a valid header field value — impossible for the method
/// tokens and the empty string this takes, all of which are compile-time
/// literals at the call sites.
pub(crate) fn method_not_allowed_response(allow: &'static str, message: &str) -> Response {
    let mut resp = status_error_response(StatusCode::METHOD_NOT_ALLOWED, message);
    resp.headers_mut()
        .insert(header::ALLOW, HeaderValue::from_static(allow));
    resp
}

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
        // 408 Request Timeout: a query-execution timeout is signalled by the
        // platform as an `exception` `SmError` whose message is prefixed with
        // [`QUERY_TIMEOUT_TAG`] (mapped to `ApiError::Internal` by `sm_api_error`).
        // Rendered here as `408` with the clean detail
        // (`Requests_and_responses.md` §HTTP status codes, row `408` — "Request
        // maximum execution time is reached, therefore the server aborted the
        // request"; `responses/408_Query.yaml`), stripping the sentinel.
        if let ApiError::Internal(raw) = &self.0
            && let Some(detail) = raw.strip_prefix(QUERY_TIMEOUT_TAG)
        {
            return status_error_response(StatusCode::REQUEST_TIMEOUT, detail);
        }
        let status = self.0.status();
        let message = self.0.to_string();
        // One uniform body (see [`ErrorBody`]): a semantic-validation failure
        // populates `validationErrors` with its per-path violations
        // (`schemas/others/Error.yaml`); every other error carries the empty
        // list.
        let validation_errors = if let ApiError::ValidationFailed(errors) = &self.0 {
            errors
                .iter()
                .map(|e| format!("{}: {}", e.path, e.message))
                .collect()
        } else {
            Vec::new()
        };
        let body = ErrorBody {
            error: status.canonical_reason().unwrap_or("Error").to_owned(),
            message,
            validation_errors,
        };
        let json = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
        let mut resp = (status, json).into_response();
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        // Every `503 Service Unavailable` carries a `Retry-After` hint: the
        // condition is transient by definition (RFC 9110 §15.6.4). This covers
        // both the overload-shed path and a storage-classified pool-exhaustion
        // `503`; no openEHR spec governs overload — our own design.
        if status == StatusCode::SERVICE_UNAVAILABLE {
            resp.headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;
    use http::StatusCode;
    use http_body_util::BodyExt;
    use openehr_its::rest::runtime::{ApiError, ValidationError};
    use serde_json::Value;

    use super::RestError;

    async fn body_json(err: ApiError) -> (StatusCode, Value) {
        let resp = RestError(err).into_response();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn validation_failure_renders_422_with_validation_errors() {
        let (status, body) = body_json(ApiError::ValidationFailed(vec![
            ValidationError {
                path: "/content[0]/data".to_owned(),
                message: "value out of range".to_owned(),
            },
            ValidationError {
                path: "/category".to_owned(),
                message: "code not in group".to_owned(),
            },
        ]))
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        // The uniform body: Error.yaml's members plus our `error` extra.
        assert_eq!(body["error"], "Unprocessable Entity");
        assert!(body.get("message").and_then(Value::as_str).is_some());
        let errors = body["validationErrors"]
            .as_array()
            .expect("validationErrors");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], "/content[0]/data: value out of range");
        assert_eq!(errors[1], "/category: code not in group");
    }

    #[tokio::test]
    async fn other_errors_carry_the_uniform_body_with_an_empty_list() {
        let (status, body) = body_json(ApiError::NotFound("EHR x".to_owned())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "Not Found");
        assert!(body.get("message").and_then(Value::as_str).is_some());
        // Always present (Error.yaml `required` on the assigned 400 surface;
        // uniform everywhere else) — empty when no per-path violations exist.
        assert_eq!(body["validationErrors"], serde_json::json!([]));
    }

    async fn handler_body(resp: axum::response::Response) -> (StatusCode, Value) {
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn method_not_allowed_renders_405_openehr_body() {
        let resp = super::method_not_allowed_handler().await;
        // The router fallback deliberately leaves `Allow` unset: axum fills it
        // in from the matched route's method set, and would NOT overwrite a
        // value the handler had already written (see the handler doc). The
        // header's presence on the wire is pinned end-to-end in
        // `tests/http.rs`.
        assert!(
            !resp.headers().contains_key(http::header::ALLOW),
            "the fallback must leave Allow to axum's per-route set"
        );
        let (status, body) = handler_body(resp).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(body["error"], "Method Not Allowed");
        assert!(body.get("message").and_then(Value::as_str).is_some());
    }

    #[tokio::test]
    async fn handler_produced_405_carries_an_allow_header() {
        // RFC 9110 §15.5.6: "The origin server MUST generate an Allow header
        // field in a 405 response containing a list of the target resource's
        // currently supported methods." A handler-produced 405 gets no axum
        // decoration, so the helper states the set itself.
        let resp = super::method_not_allowed_response("GET,HEAD", "nope");
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(
            resp.headers()
                .get(http::header::ALLOW)
                .and_then(|v| v.to_str().ok()),
            Some("GET,HEAD")
        );
        let (_status, body) = handler_body(resp).await;
        assert_eq!(body["error"], "Method Not Allowed");
        assert_eq!(body["message"], "nope");
    }

    #[tokio::test]
    async fn handler_produced_405_can_advertise_the_empty_method_set() {
        // RFC 9110 §10.2.1: "An empty Allow field value indicates that the
        // resource allows no methods, which might occur in a 405 response if
        // the resource has been temporarily disabled by configuration" — the
        // config-gated admin group's case. The header is PRESENT and empty,
        // never absent.
        let resp = super::method_not_allowed_response("", "disabled");
        assert_eq!(
            resp.headers()
                .get(http::header::ALLOW)
                .and_then(|v| v.to_str().ok()),
            Some("")
        );
    }

    #[tokio::test]
    async fn query_timeout_tag_renders_408_with_clean_message() {
        // Requests_and_responses.md §HTTP status codes, row 408: a query-execution
        // timeout is tagged on an `exception` message and rendered as 408 with the
        // sentinel stripped from the client-visible detail.
        let tagged = format!("{}query aborted after 5000ms", super::QUERY_TIMEOUT_TAG);
        let (status, body) = body_json(ApiError::Internal(tagged)).await;
        assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
        assert_eq!(body["error"], "Request Timeout");
        assert_eq!(body["message"], "query aborted after 5000ms");
    }

    #[tokio::test]
    async fn untagged_internal_stays_500() {
        // A genuine server fault (no timeout tag) still maps to 500.
        let (status, _body) = body_json(ApiError::Internal("boom".to_owned())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn service_overloaded_maps_to_503_with_retry_after() {
        // A storage-classified pool exhaustion (`ServiceOverloaded`)
        // surfaces as 503 + Retry-After (our overload contract; RFC 9110
        // §15.6.4).
        use ferroehr::service::status::{CallStatusType, SmError};
        let sm = SmError::new(CallStatusType::ServiceOverloaded, "overloaded");
        let resp = RestError::from(sm).into_response();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            resp.headers()
                .get(http::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok()),
            Some("1")
        );
    }

    #[tokio::test]
    async fn storage_conflict_maps_to_409() {
        // A storage-classified integrity/serialization conflict
        // (`Conflict`) surfaces as 409 (ITS-REST overview §HTTP status codes).
        use ferroehr::service::status::{CallStatusType, SmError};
        let sm = SmError::new(CallStatusType::Conflict, "duplicate key");
        let resp = RestError::from(sm).into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }
}
