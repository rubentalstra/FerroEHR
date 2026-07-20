//! Mapping the contract's [`ApiError`] onto an HTTP response with an openEHR
//! error body.
//!
//! The generated server traits return [`openehr_its::rest::runtime::ApiError`],
//! which carries the ITS-REST status code but renders as a bare string. The
//! REST layer wraps it in [`RestError`] so handlers can use `?` and every error
//! leaves the server as a structured JSON body.
//!
//! Two body shapes, both from the ITS-REST 1.1.0 spec:
//!
//! * A semantic-validation failure ([`ApiError::ValidationFailed`], HTTP `422`)
//!   renders the openEHR `Error` object —
//!   `docs/specs/openehr/ITS-REST/specifications/schemas/others/Error.yaml`:
//!   `{ "message", "validationErrors": ["<path>: <message>", …] }` — via the
//!   generated [`openehr_its::rest::generated::ehr::Error`] DTO.
//!
//!   NOTE: `422_COMPOSITION.yaml` declares no `content`/`schema` (the 422
//!   body is spec-silent); the `Error` object is formally bound only to the
//!   `400` response. Reusing that `{ message, validationErrors[] }` shape for
//!   the `422` validation case is a deliberate, documented choice.
//! * Every other error renders `{ "error", "message" }` (the status reason
//!   phrase + human-readable detail).

use axum::response::{IntoResponse, Response};
use http::{HeaderValue, StatusCode, header};
use serde::Serialize;

use ehrbase::service::error::ServiceError;
use ehrbase::service::status::{CallStatusType, QUERY_TIMEOUT_TAG, SmError};
use openehr_its::rest::generated::ehr::Error as ValidationErrorBody;
use openehr_its::rest::runtime::ApiError;

/// A response-rendering wrapper over the contract's [`ApiError`].
#[derive(Debug)]
pub struct RestError(pub ApiError);

impl From<ApiError> for RestError {
    fn from(e: ApiError) -> Self {
        Self(e)
    }
}

/// The single SM → HTTP mapping, owned by the protocol adapter: a native [`SmError`]
/// carries only a `CALL_STATUS_TYPE`, and this adapter turns its status into the
/// ITS-REST 1.1.0 status code. The wire oracle (ITS-REST) decides each row;
/// where the SM name and the wire disagree, the wire's status wins here. Living
/// in `ehrbase-rest` keeps `ehrbase-sm` protocol-free (no `openehr_its::rest`
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

/// The JSON error body the ITS-REST spec prescribes for most non-2xx responses.
#[derive(Debug, Serialize)]
struct ErrorBody {
    /// Machine-readable status label (the reason phrase, e.g. `Not Found`).
    error: String,
    /// Human-readable detail.
    message: String,
}

/// Render an arbitrary status as the `{ error, message }` openEHR error body.
/// Used for the method-status responses (`405`/`501`) that have no dedicated
/// [`ApiError`] variant (the contract's `ApiError` cannot represent `405`).
pub(crate) fn status_error_response(status: StatusCode, message: &str) -> Response {
    let body = ErrorBody {
        error: status.canonical_reason().unwrap_or("Error").to_owned(),
        message: message.to_owned(),
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
//
// "A server receiving an unrecognized or unimplemented method SHOULD respond
// with the `501 Not Implemented` status code. If a method is recognized but not
// allowed for the target resource, the response SHOULD be `405 Method Not
// Allowed`." These two axum fallbacks render that rule with the openEHR
// `{ error, message }` body instead of axum's default bare `405`/text.
//
// `method_not_allowed_handler` is mounted as the router's method fallback
// (`crate::router::router`), rendering `405` with the openEHR body. Operation-level
// `501 Not Implemented` rides `ApiError` (a blanket 501 method fallback would
// misreport unknown paths, `router.rs` doc).

/// Axum fallback for a request whose method is **recognized but not allowed** on
/// the matched resource → `405 Method Not Allowed` (overview §HTTP Methods).
pub(crate) async fn method_not_allowed_handler() -> Response {
    status_error_response(
        StatusCode::METHOD_NOT_ALLOWED,
        "the request method is not allowed on this resource",
    )
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
        // Semantic-validation failure → the ITS-REST `Error` object with the
        // per-path violations as `validationErrors` (`schemas/others/Error.yaml`);
        // every other error → the `{ error, message }` shape.
        let json = if let ApiError::ValidationFailed(errors) = self.0 {
            let body = ValidationErrorBody {
                message,
                validation_errors: errors
                    .into_iter()
                    .map(|e| format!("{}: {}", e.path, e.message))
                    .collect(),
            };
            serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec())
        } else {
            let body = ErrorBody {
                error: status.canonical_reason().unwrap_or("Error").to_owned(),
                message,
            };
            serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec())
        };
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
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        // ITS-REST `Error` shape: { message, validationErrors: [ "<path>: <message>" ] }.
        assert!(body.get("message").and_then(Value::as_str).is_some());
        let errors = body["validationErrors"]
            .as_array()
            .expect("validationErrors");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0], "/content[0]/data: value out of range");
        assert_eq!(errors[1], "/category: code not in group");
    }

    #[tokio::test]
    async fn other_errors_keep_the_error_message_shape() {
        let (status, body) = body_json(ApiError::NotFound("EHR x".to_owned())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "Not Found");
        assert!(body.get("message").and_then(Value::as_str).is_some());
        assert!(body.get("validationErrors").is_none());
    }

    async fn handler_body(resp: axum::response::Response) -> (StatusCode, Value) {
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn method_not_allowed_renders_405_openehr_body() {
        let (status, body) = handler_body(super::method_not_allowed_handler().await).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(body["error"], "Method Not Allowed");
        assert!(body.get("message").and_then(Value::as_str).is_some());
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
        use ehrbase::service::status::{CallStatusType, SmError};
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
        use ehrbase::service::status::{CallStatusType, SmError};
        let sm = SmError::new(CallStatusType::Conflict, "duplicate key");
        let resp = RestError::from(sm).into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }
}
