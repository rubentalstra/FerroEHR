//! Mapping the contract's [`ApiError`] onto an HTTP response with an openEHR
//! error body.
//!
//! The generated server traits return [`openehr_its::rest::runtime::ApiError`],
//! which carries the ITS-REST status code but renders as a bare string. The
//! REST layer wraps it in [`RestError`] so handlers can use `?` and every error
//! leaves the server as a structured JSON body.
//!
//! Two body shapes, both from the ITS-REST 1.0.3 spec:
//!
//! * A semantic-validation failure ([`ApiError::ValidationFailed`], HTTP `422`)
//!   renders the openEHR `Error` object —
//!   `docs/specs/openehr/ITS-REST/specifications/schemas/others/Error.yaml`:
//!   `{ "message", "validationErrors": ["<path>: <message>", …] }` — via the
//!   generated [`openehr_its::rest::generated::ehr::Error`] DTO.
//!
//!   PORT NOTE: `422_COMPOSITION.yaml` declares no `content`/`schema` (the 422
//!   body is spec-silent); the `Error` object is formally bound only to the
//!   `400` response. Reusing that `{ message, validationErrors[] }` shape for
//!   the `422` validation case is a deliberate, documented choice.
//! * Every other error renders `{ "error", "message" }` (the status reason
//!   phrase + human-readable detail).

use axum::response::{IntoResponse, Response};
use http::{HeaderValue, StatusCode, header};
use serde::Serialize;

use ehrbase_sm::{CallStatusType, SmError};
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

/// The single SM → HTTP mapping, owned by the protocol adapter (the crate split,
/// `docs/design/sm-platform/08-target-architecture.md` §5): a native [`SmError`]
/// carries only a `CALL_STATUS_TYPE`, and this adapter turns its status into the
/// ITS-REST 1.0.3 status code. The wire oracle (ITS-REST) decides each row;
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
        S::EhrCreateFailDuplicateId
        | S::CompositionAlreadyExists
        | S::EhrForSubjectAlreadyExists => ApiError::Conflict(message),
        S::CompositionArchetypeInvalid
        | S::InvalidArchetype
        | S::InvalidTemplate
        | S::InvalidArtefact
        | S::InvalidQuery
        | S::DefinitionUnknown
        | S::ContentInvalid => ApiError::Unprocessable(message),
        S::NotImplemented => ApiError::NotImplemented,
        // `success` is not an error; mapping it is defensively a 500.
        S::Success | S::FileNotWritable | S::Exception => ApiError::Internal(message),
    }
}

impl From<SmError> for RestError {
    fn from(e: SmError) -> Self {
        Self(sm_api_error(e))
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
fn status_error_response(status: StatusCode, message: &str) -> Response {
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
// TODO(w3e-integrate): mount these in `crate::router` — attach
// `method_not_allowed_handler` as the `MethodRouter::fallback` (or a
// `MethodNotAllowedLayer`) on the resource routers so a known-route wrong-method
// renders `405` with the openEHR body, and route `not_implemented_handler` for
// unrecognized/unimplemented HTTP methods so they render `501`. This crate only
// owns the `overview/` handlers; the router wiring lives outside it.

/// Axum fallback for a request whose method is **recognized but not allowed** on
/// the matched resource → `405 Method Not Allowed` (overview §HTTP Methods).
pub(crate) async fn method_not_allowed_handler() -> Response {
    status_error_response(
        StatusCode::METHOD_NOT_ALLOWED,
        "the request method is not allowed on this resource",
    )
}

/// Axum handler for a request whose method is **unrecognized or unimplemented**
/// → `501 Not Implemented` (overview §HTTP Methods).
pub(crate) async fn not_implemented_handler() -> Response {
    status_error_response(
        StatusCode::NOT_IMPLEMENTED,
        "the request method is not implemented",
    )
}

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
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
        resp
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

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
    async fn unimplemented_method_renders_501_openehr_body() {
        let (status, body) = handler_body(super::not_implemented_handler().await).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(body["error"], "Not Implemented");
        assert!(body.get("message").and_then(Value::as_str).is_some());
    }
}
