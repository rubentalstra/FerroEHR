//! Axum handlers for the logger-reload surface (model:
//! [`ferroehr::telemetry::log_reload::LogReload`]).

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 8): genuinely open operational JSON (config \
              dump, management env, validity-checker input, OpenAPI schema literals)"
)]

use axum::Json;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use ferroehr::telemetry::log_reload::LogReload;

#[derive(Debug, Serialize)]
pub(super) struct LoggersView {
    /// The effective filter directives.
    filter: String,
    /// The boot-time filter directives (the reset target).
    boot_filter: String,
}

/// The `POST` request body.
#[derive(Debug, Deserialize)]
pub(super) struct SetFilter {
    /// The new filter directive set (`EnvFilter` syntax).
    filter: String,
}

fn view(reload: &LogReload) -> LoggersView {
    LoggersView {
        filter: reload.current(),
        boot_filter: reload.boot_filter().to_owned(),
    }
}

/// `GET /management/loggers`.
pub(super) fn get(reload: &LogReload) -> Json<LoggersView> {
    Json(view(reload))
}

/// `POST /management/loggers`.
pub(super) fn set(reload: &LogReload, body: &SetFilter) -> Response {
    match reload.set(&body.filter) {
        Ok(()) => (StatusCode::OK, Json(view(reload))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("invalid filter: {e}") })),
        )
            .into_response(),
    }
}

/// `DELETE /management/loggers`.
pub(super) fn reset(reload: &LogReload) -> Response {
    match reload.reset() {
        Ok(()) => (StatusCode::OK, Json(view(reload))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("reset failed: {e}") })),
        )
            .into_response(),
    }
}
