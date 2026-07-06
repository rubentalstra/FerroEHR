//! Router assembly and the `tower-http` middleware stack.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::middleware::from_fn_with_state;
use http::StatusCode;
use http::header::AUTHORIZATION;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::auth::{self, Authenticator};
use crate::state::AppState;
use crate::{dispatch, openapi, status};

/// Maximum accepted request body (16 MiB) — compositions/templates are large.
const REQUEST_BODY_LIMIT: usize = 16 * 1024 * 1024;
/// Per-request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Assemble the full application router: the ITS-REST API (behind the auth
/// layer) nested under the base path, the public status/health endpoints, the
/// optional Swagger UI, and the shared `tower-http` middleware stack.
///
/// `NormalizePathLayer` is applied at serve time (it must wrap the router to run
/// before routing); see [`crate::serve`].
pub fn router(state: AppState, authenticator: Arc<Authenticator>) -> Router {
    let cfg = state.config().clone();
    let rest_root = cfg
        .base_path
        .strip_suffix("/openehr/v1")
        .unwrap_or(&cfg.base_path)
        .to_owned();

    // The generated ITS-REST surface, gated by authentication. The ATNA audit
    // layer wraps auth (outermost) so it observes auth rejections too (§8.2);
    // installed only when a sender is wired in.
    let api = dispatch::api_router().layer(from_fn_with_state(authenticator, auth::middleware));
    let api = match state.audit() {
        Some(sender) => api.layer(from_fn_with_state(sender, crate::audit::middleware)),
        None => api,
    };

    let mut app = Router::new()
        .nest(&cfg.base_path, api)
        .merge(status::router(&rest_root));

    if cfg.swagger_ui {
        app = app.merge(openapi::swagger_router(
            &cfg.swagger_ui_path(),
            &cfg.openapi_json_path(),
        ));
    }

    let cors = if cfg.cors_permissive {
        CorsLayer::very_permissive()
    } else {
        CorsLayer::new()
    };

    app.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(SetSensitiveRequestHeadersLayer::new(std::iter::once(
                AUTHORIZATION,
            )))
            .layer(TraceLayer::new_for_http())
            .layer(CatchPanicLayer::new())
            .layer(cors)
            .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                REQUEST_TIMEOUT,
            ))
            .layer(CompressionLayer::new()),
    )
    .with_state(state)
}
