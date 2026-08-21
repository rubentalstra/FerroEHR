// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: MIT AND Apache-2.0

//! Hand-written ITS-REST runtime: the API error type the generated server
//! traits return, mapped to an HTTP response.
//!
//! The DTOs, per-group server traits, and route tables are generated
//! (`emit-rest`) into [`super::generated`]; `ferroehr-rest` implements the
//! traits and wires axum.

use axum::response::{IntoResponse, Response};
use http::StatusCode;

/// A single semantic-validation violation, keyed by the RM path of the
/// offending node.
///
/// Carried by [`ApiError::ValidationFailed`] so the REST layer can render the
/// ITS-REST error body (`schemas/others/Error.yaml`: `{ message,
/// validationErrors[] }`).
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The RM path (archetype `aqlPath` or RM instance path) of the violation.
    pub path: String,
    /// A human-readable description of the violation.
    pub message: String,
}

/// The error a REST handler may return; carries the HTTP status the openEHR
/// ITS-REST contract prescribes.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// `400 Bad Request` — a malformed request the server will not process.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// `401 Unauthorized` — the request carried no usable credentials.
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    /// `403 Forbidden` — authenticated but not permitted this operation.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// `404 Not Found` — the addressed resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// `409 Conflict` — the resource already exists, or the request conflicts
    /// with its current state.
    #[error("conflict: {0}")]
    Conflict(String),
    /// `412 Precondition Failed` — an `If-Match` precondition did not hold.
    #[error("precondition failed: {0}")]
    PreconditionFailed(String),
    /// `422 Unprocessable Content` — a well-formed payload the server cannot
    /// process, without a per-path violation list.
    #[error("unprocessable entity: {0}")]
    Unprocessable(String),
    /// A well-formed payload that failed semantic (template/RM/terminology)
    /// validation: an ITS-REST `422 Unprocessable Entity` with a structured
    /// list of per-path violations (ITS-REST `422.yaml`).
    #[error("{} validation error(s)", .0.len())]
    ValidationFailed(Vec<ValidationError>),
    /// `415 Unsupported Media Type` — the request `Content-Type` is not served.
    #[error("unsupported media type: {0}")]
    UnsupportedMediaType(String),
    /// `406 Not Acceptable` — no representation satisfies the request `Accept`.
    #[error("not acceptable: {0}")]
    NotAcceptable(String),
    /// `501 Not Implemented` — the operation is part of the contract but this
    /// server does not provide it.
    #[error("not implemented")]
    NotImplemented,
    /// The server is temporarily unable to handle the request. Used by the
    /// application's ingress overload-shedding layer (RFC 9110 §15.6.4 —
    /// `503 Service Unavailable` is the status for a server that is
    /// temporarily overloaded). No openEHR spec governs server overload
    /// semantics — this is our own design/extension.
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),
    /// `500 Internal Server Error` — an unexpected server-side failure.
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
            ApiError::Unprocessable(_) | ApiError::ValidationFailed(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            ApiError::UnsupportedMediaType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ApiError::NotAcceptable(_) => StatusCode::NOT_ACCEPTABLE,
            ApiError::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status(), self.to_string()).into_response()
    }
}
