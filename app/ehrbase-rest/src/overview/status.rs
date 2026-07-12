//! Public operational endpoints: `/status`, health, `/management/info`, and the
//! ITS-REST `OPTIONS /` System-Options-and-Conformance endpoint. These are
//! mounted outside the authentication layer.

use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use ehrbase_sm::Platform;
use http::StatusCode;
use serde::Serialize;

use crate::extensions::provenance;
use crate::state::AppState;

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

async fn status() -> Json<ServerStatus> {
    Json(ServerStatus {
        status: "UP",
        server_version: env!("CARGO_PKG_VERSION"),
        openehr_rest_api_version: provenance::ITS_REST,
        timestamp: jiff::Timestamp::now().to_string(),
    })
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub(crate) fn router<S: Platform>(rest_root: &str) -> Router<AppState<S>> {
    Router::new()
        .route(&format!("{rest_root}/status"), get(status))
        .route("/health", get(health))
        .route(&format!("{rest_root}/status/health"), get(health))
}
