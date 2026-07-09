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
use http::{HeaderValue, header};
use serde::Serialize;

use ehrbase_sm::SmError;
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

/// The single SM → HTTP mapping, owned by the protocol adapter (ADR-011): a
/// native [`SmError`] carries only a `CALL_STATUS_TYPE`, and this adapter turns
/// its status into the ITS-REST 1.0.3 status code + body. The wire oracle
/// (ITS-REST) decides each row via [`ehrbase_sm::CallStatusType::api_error`].
/// (A free function, not `impl From<SmError> for ApiError`, because both types
/// are foreign to this crate — the orphan rule forbids that impl.)
#[must_use]
pub(crate) fn sm_api_error(e: SmError) -> ApiError {
    e.status.api_error(e.message)
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
}
