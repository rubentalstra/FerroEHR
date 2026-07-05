//! Mapping the contract's [`ApiError`] onto an HTTP response with an openEHR
//! error body.
//!
//! The generated server traits return [`openehr_its::rest::runtime::ApiError`],
//! which carries the ITS-REST status code but renders as a bare string. The
//! REST layer wraps it in [`RestError`] so handlers can use `?` and every error
//! leaves the server as a structured JSON body (`{ "error", "message" }`) — the
//! shape the ITS-REST spec's error responses use.

use axum::response::{IntoResponse, Response};
use http::{HeaderValue, header};
use serde::Serialize;

use openehr_its::rest::runtime::ApiError;

/// A response-rendering wrapper over the contract's [`ApiError`].
#[derive(Debug)]
pub struct RestError(pub ApiError);

impl From<ApiError> for RestError {
    fn from(e: ApiError) -> Self {
        Self(e)
    }
}

/// The JSON error body the ITS-REST spec prescribes for non-2xx responses.
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
        let body = ErrorBody {
            error: status.canonical_reason().unwrap_or("Error").to_owned(),
            message: self.0.to_string(),
        };
        let json = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
        let mut resp = (status, json).into_response();
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        resp
    }
}
