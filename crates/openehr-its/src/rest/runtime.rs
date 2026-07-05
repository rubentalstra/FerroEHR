//! Hand-written ITS-REST runtime (ADR-005): the API error type the generated
//! server traits return, mapped to an HTTP response. The DTOs, per-group server
//! traits, and route tables are generated (`emit-rest`) into [`super::generated`];
//! `ehrbase-rest` implements the traits and wires axum.

use axum::response::{IntoResponse, Response};
use http::StatusCode;

/// The error a REST handler may return; carries the HTTP status the openEHR
/// ITS-REST contract prescribes.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("precondition failed: {0}")]
    PreconditionFailed(String),
    #[error("unprocessable entity: {0}")]
    Unprocessable(String),
    #[error("unsupported media type: {0}")]
    UnsupportedMediaType(String),
    #[error("not acceptable: {0}")]
    NotAcceptable(String),
    #[error("not implemented")]
    NotImplemented,
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    /// The HTTP status for this error.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::PreconditionFailed(_) => StatusCode::PRECONDITION_FAILED,
            ApiError::Unprocessable(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::UnsupportedMediaType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ApiError::NotAcceptable(_) => StatusCode::NOT_ACCEPTABLE,
            ApiError::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status(), self.to_string()).into_response()
    }
}
