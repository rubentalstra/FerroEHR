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

use ehrbase_sm::Platform;

use crate::auth::{self, Authenticator};
use crate::management::{self, ManagementState};
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
pub fn router<S: Platform>(state: AppState<S>, authenticator: Arc<Authenticator>) -> Router {
    let cfg = state.config().clone();
    let observability = state.observability().clone();
    let rest_root = cfg
        .base_path
        .strip_suffix("/openehr/v1")
        .unwrap_or(&cfg.base_path)
        .to_owned();

    // The generated ITS-REST surface, gated by authentication. Layers, innermost
    // → outermost: auth · ATNA audit (§8.2, wraps auth so it observes auth
    // rejections) · HTTP metrics (§1.2) · root span (§1.1, outermost so the span
    // and metrics cover the whole request incl. auth). The metrics/span layers
    // sit on the API router so `MatchedPath` resolves to the route template.
    let api = dispatch::api_router::<S>().layer(from_fn_with_state(
        auth::AuthLayer {
            authenticator: authenticator.clone(),
            authz: state.authz(),
        },
        auth::middleware,
    ));
    // Always install the ATNA audit layer; it early-returns when the platform's
    // SM `SystemLog` reports auditing off (`backend().audit_enabled()`), so the
    // no-audit case costs one check per request.
    let api = api.layer(from_fn_with_state(
        state.clone(),
        crate::audit::middleware::<S>,
    ));
    let api = api
        .layer(axum::middleware::from_fn(management::http_metrics))
        .layer(axum::middleware::from_fn(management::root_span));

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

    let app = app
        .layer(
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
        .with_state(state);

    // Merge the management surface only when enabled AND not bound to a separate
    // port (the binary serves the separate-port case on its own listener).
    if observability.management.enabled && observability.management.port.is_none() {
        let mgmt = management::router(ManagementState::from_observability(
            observability,
            authenticator,
        ));
        app.merge(mgmt)
    } else {
        app
    }
}

/// Build the standalone management router (separate-port mode). The binary
/// serves this on the management listener when `management.port` is set.
pub fn management_router<S: Platform>(
    state: &AppState<S>,
    authenticator: Arc<Authenticator>,
) -> Router {
    management::router(ManagementState::from_observability(
        state.observability().clone(),
        authenticator,
    ))
}
