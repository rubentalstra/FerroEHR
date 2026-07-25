//! The public status surface hanging off the REST root: the `/rest/status`
//! product status document, mounted outside the authentication layer.
//!
//! NOTE: no openEHR spec governs an operational status or health endpoint —
//! our own operational surface. Health is a separate contract with its own
//! clients and lives entirely in the process-root family (`/health`,
//! `/health/liveness`, `/health/readiness` — [`crate::extensions::health`]);
//! this module serves only the status document.

use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use ehrbase::telemetry::provenance;

/// `/rest/status` body — server and conformance-target versions. The
/// `openehr_rest_api_version` is the single shared provenance identity
/// ([`provenance::ITS_REST`]) — the released ITS-REST contract version that
/// management `/info` and the System Options manifest also report.
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

pub(crate) fn router(rest_root: &str) -> Router<AppState> {
    Router::new().route(&format!("{rest_root}/status"), get(status))
}

/// The public status surface's `OpenAPI` document (the path is at the default
/// REST root; a non-default base path shifts it uniformly). The operation is
/// unauthenticated (mounted outside the auth layer). No openEHR spec governs an
/// operational status endpoint — our own surface.
pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    OpenApiRouter::<AppState>::new()
        .routes(routes!(status))
        .into_openapi()
}
