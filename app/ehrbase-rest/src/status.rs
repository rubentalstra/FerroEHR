//! Public operational endpoints: `/status`, health, and `/management/info`.
//! These are mounted outside the authentication layer.

use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use ehrbase_sm::Platform;
use http::StatusCode;
use serde::Serialize;

use crate::state::AppState;

/// The openEHR REST API version this server targets.
const OPENEHR_REST_API_VERSION: &str = "1.0.3";

/// `/rest/status` body — server and conformance-target versions.
#[derive(Debug, Serialize)]
struct ServerStatus {
    status: &'static str,
    server_version: &'static str,
    openehr_rest_api_version: &'static str,
    timestamp: String,
}

async fn status() -> Json<ServerStatus> {
    Json(ServerStatus {
        status: "UP",
        server_version: env!("CARGO_PKG_VERSION"),
        openehr_rest_api_version: OPENEHR_REST_API_VERSION,
        timestamp: jiff::Timestamp::now().to_string(),
    })
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Build the public status/health router, hung off the REST root
/// (`/ehrbase/rest`). This is the product probe surface; the ops surface
/// (`/management/*`, including `info`) lives in [`crate::management`] and is
/// off by default.
pub(crate) fn router<S: Platform>(rest_root: &str) -> Router<AppState<S>> {
    Router::new()
        .route(&format!("{rest_root}/status"), get(status))
        .route("/health", get(health))
        .route(&format!("{rest_root}/status/health"), get(health))
}
