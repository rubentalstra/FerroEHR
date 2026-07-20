//! `GET /management/info` — the axum handler over the platform
//! [`BuildInfo`](ehrbase::telemetry::build_info::BuildInfo) model
//! (the model lives in the platform, wire here).

use axum::response::Json;

use ehrbase::telemetry::build_info::BuildInfo;

/// Render the build/spec provenance as JSON.
pub(super) fn info(build: BuildInfo) -> Json<BuildInfo> {
    Json(build)
}
