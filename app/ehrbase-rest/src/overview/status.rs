//! Public operational endpoints: `/status`, health, `/management/info`, and the
//! ITS-REST `OPTIONS /` System-Options-and-Conformance endpoint. These are
//! mounted outside the authentication layer.

use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use http::StatusCode;
use serde::Serialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use ehrbase::telemetry::provenance;

/// `/rest/status` body — server and conformance-target versions. The
/// `openehr_rest_api_version` is the single shared provenance identity
/// ([`provenance::ITS_REST`]) — the tested development-edition contract that
/// management `/info` and the System Options manifest also report — not the
/// retired `1.0.3` release label.
#[derive(Debug, Serialize)]
struct ServerStatus {
    status: &'static str,
    server_version: &'static str,
    openehr_rest_api_version: &'static str,
    timestamp: String,
}

/// Server status (`GET /ehrbase/rest/status`).
///
/// OUR OWN SURFACE — no openEHR spec governs an operational status endpoint.
/// Reports the server version and the tested ITS-REST contract identity.
/// Unauthenticated (mounted outside the auth layer).
#[utoipa::path(
    get, path = "/ehrbase/rest/status", tag = "status",
    responses(
        (status = 200, description = "Server up; a JSON `{status, server_version, \
                                      openehr_rest_api_version, timestamp}` \
                                      object.",
         body = serde_json::Value)
    )
)]
async fn status() -> Json<ServerStatus> {
    Json(ServerStatus {
        status: "UP",
        server_version: env!("CARGO_PKG_VERSION"),
        openehr_rest_api_version: provenance::ITS_REST,
        timestamp: jiff::Timestamp::now().to_string(),
    })
}

/// Liveness probe at the process root (`GET /health`).
///
/// OUR OWN SURFACE — no openEHR spec governs a health endpoint. Returns the
/// plain-text body `OK`. Unauthenticated (mounted outside the auth layer).
#[utoipa::path(
    get, path = "/health", tag = "status",
    responses(
        (status = 200, description = "Server process alive; plain-text `OK`.",
         body = String)
    )
)]
async fn root_health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Liveness probe under the REST root
/// (`GET /ehrbase/rest/status/health`).
///
/// OUR OWN SURFACE — no openEHR spec governs a health endpoint. Returns the
/// plain-text body `OK`. Unauthenticated (mounted outside the auth layer).
#[utoipa::path(
    get, path = "/ehrbase/rest/status/health", tag = "status",
    responses(
        (status = 200, description = "Server process alive; plain-text `OK`.",
         body = String)
    )
)]
async fn status_health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub(crate) fn router(rest_root: &str) -> Router<AppState> {
    Router::new()
        .route(&format!("{rest_root}/status"), get(status))
        .route("/health", get(root_health))
        .route(&format!("{rest_root}/status/health"), get(status_health))
}

/// The public status/health surface's `OpenAPI` document (paths at the default
/// REST root; a non-default base path shifts them uniformly). These endpoints
/// are unauthenticated (mounted outside the auth layer). No openEHR spec governs
/// an operational status/health endpoint — our own surface.
pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    OpenApiRouter::<AppState>::new()
        .routes(routes!(status))
        .routes(routes!(root_health))
        .routes(routes!(status_health))
        .into_openapi()
}
