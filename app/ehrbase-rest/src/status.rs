//! Public operational endpoints: `/status`, health, `/management/info`, and the
//! ITS-REST `OPTIONS /` System-Options-and-Conformance endpoint. These are
//! mounted outside the authentication layer.

use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use ehrbase_sm::Platform;
use http::{HeaderValue, StatusCode, header};
use serde::Serialize;

use crate::state::AppState;

/// The openEHR REST API version this server targets.
const OPENEHR_REST_API_VERSION: &str = "1.0.3";
/// The HTTP methods this API surface supports (the `Allow` header on `OPTIONS`).
const ALLOW_METHODS: &str = "GET, POST, PUT, DELETE, OPTIONS";

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

/// `Options` body (ITS-REST overview `schemas/others/Options.yaml`) — the
/// system-capabilities / conformance manifest served by `OPTIONS /`.
#[derive(Debug, Serialize)]
struct Options {
    solution: &'static str,
    solution_version: &'static str,
    vendor: &'static str,
    restapi_specs_version: &'static str,
    conformance_profile: &'static str,
    endpoints: Vec<&'static str>,
}

/// `OPTIONS /` — "System Options and Conformance" (ITS-REST overview
/// `paths` `/` `options`, response `200_options`): a `200` with the `Allow`
/// header and an `Options` body describing the service (map row R32; SHOULD).
///
/// Mounted in [`crate::router`] **above** the `tower-http` `CorsLayer` — that
/// layer treats every `OPTIONS` as a CORS preflight and short-circuits it, so a
/// conformance `OPTIONS /` must be routed before CORS sees it.
pub(crate) async fn system_options() -> impl IntoResponse {
    let body = Options {
        solution: "ehrbase-rs",
        solution_version: env!("CARGO_PKG_VERSION"),
        vendor: "ehrbase-rs",
        restapi_specs_version: OPENEHR_REST_API_VERSION,
        // The conformance profile this CDR targets (CNF master03 profiles).
        conformance_profile: "STANDARD",
        endpoints: vec!["/ehr", "/definition", "/query"],
    };
    let mut resp = Json(body).into_response();
    resp.headers_mut()
        .insert(header::ALLOW, HeaderValue::from_static(ALLOW_METHODS));
    resp
}

/// Build the public status/health router, hung off the REST root
/// (`/ehrbase/rest`). This is the product probe surface; the ops surface
/// (`/management/*`, including `info`) lives in [`crate::management`] and is off
/// by default. The `OPTIONS /` conformance endpoint is mounted separately in
/// [`crate::router`] (above the CORS layer — see [`system_options`]).
pub(crate) fn router<S: Platform>(rest_root: &str) -> Router<AppState<S>> {
    Router::new()
        .route(&format!("{rest_root}/status"), get(status))
        .route("/health", get(health))
        .route(&format!("{rest_root}/status/health"), get(health))
}
