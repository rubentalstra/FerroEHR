//! Router assembly and the `tower-http` middleware stack.
//!
//! **Base-path rule** (ITS-REST overview `Resources.md` §Resource identification;
//! `Requests_and_responses.md`): every ITS-REST resource path is relative to the
//! configured API base path (`cfg.base_path`, default
//! `/ehrbase/rest/openehr/v1`). The generated API surface ([`crate::api`]) is
//! therefore nested under that base path; the public operational endpoints
//! (`/rest/status`, health) hang off the `/ehrbase/rest` root, and the System
//! Options manifest answers at the base-path root itself.
//!
//! **Layer order** (innermost → outermost, over the nested API): authentication ·
//! ATNA audit (SM System Log, wraps auth so it observes auth rejections) · HTTP
//! metrics · root span · **overload shedding** (bounded in-flight concurrency +
//! load shed; the API subtree's outermost layer, so a shed request is rejected
//! before auth, audit, or reading the request body — [`crate::overload`]). The
//! whole tree is then wrapped in the shared `tower-http` request stack
//! (request-id, tracing, CORS, body limit, timeout, compression), so a shed
//! `503` still carries a request id and is traced. The System Options `OPTIONS`
//! endpoint is mounted **above** the CORS layer — `CorsLayer` treats every
//! `OPTIONS` as a CORS preflight and would short-circuit a conformance probe —
//! so it lives on the outer router with the CORS-wrapped application as its
//! fallback service.
//!
//! **Overload shedding is scoped to the API subtree only** (the openEHR API +
//! its extensions, nested under the base path): the public operational
//! endpoints — `/rest/status`, health, SMART discovery, and the management
//! surface — are siblings, so they are never shed and an operator can always
//! probe an overloaded server. The bound is `cfg.max_in_flight` (default 1024);
//! `0` installs no layer. No openEHR spec governs server overload — our own
//! design (RFC 9110 §15.6.4).
//!
//! `NormalizePathLayer` is applied at serve time (it must wrap the router to run
//! before routing); see [`crate::serve_with`].

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use http::header::AUTHORIZATION;
use openehr_its::rest::runtime::ApiError;
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

use crate::api::system;
use crate::extensions::access::authn::{self, Authenticator};
use crate::extensions::management::{self, ManagementState};
use crate::extensions::openapi;
use crate::overview::{error, status};
use crate::smart;
use crate::state::AppState;

/// Maximum accepted request body (16 MiB) — compositions/templates are large.
const REQUEST_BODY_LIMIT: usize = 16 * 1024 * 1024;
/// Per-request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Assemble the full application router: the ITS-REST API (behind the auth
/// layer) nested under the base path, the public status/health + SMART-discovery
/// endpoints, the System Options manifest, the optional Swagger UI, and the
/// shared `tower-http` middleware stack.
pub fn router<S: Platform>(state: AppState<S>, authenticator: Arc<Authenticator>) -> Router {
    let cfg = state.config().clone();
    let observability = state.observability().clone();
    let rest_root = cfg
        .server
        .base_path
        .strip_suffix("/openehr/v1")
        .unwrap_or(&cfg.server.base_path)
        .to_owned();

    // ── The generated ITS-REST surface, gated by authentication ──────────────
    // A known resource path called with a disallowed method renders `405` with
    // the openEHR `{ error, message }` body (overview §HTTP Methods), not axum's
    // bare text 405. (The paired `501` for a recognised-but-unimplemented
    // operation is produced at dispatch level via `ApiError::NotImplemented` —
    // see [`crate::overview::error::not_implemented_handler`]; axum routes by
    // path+method and offers no distinct "unrecognised method" seam, so a blanket
    // `501` fallback would wrongly convert genuine `404`s.)
    let api = crate::api::api_router::<S>()
        .method_not_allowed_fallback(error::method_not_allowed_handler);
    // Tenant resolution sits inside the auth layer so it runs *after*
    // authentication (the principal + its claims are established) and scopes the
    // handler in the tenant task-local. Only installed when tenancy is on — a
    // single-tenant server has no tenant middleware at all.
    let api = if cfg.tenancy.enabled {
        api.layer(from_fn_with_state(
            state.clone(),
            crate::extensions::access::tenant::middleware::<S>,
        ))
    } else {
        api
    };
    let api = api.layer(from_fn_with_state(
        authn::AuthLayer {
            authenticator: authenticator.clone(),
            authz: state.authz(),
        },
        authn::middleware,
    ));
    // Always install the ATNA audit layer; it early-returns when the platform's
    // SM `SystemLog` reports auditing off, so the no-audit case costs one check
    // per request.
    let api = api.layer(from_fn_with_state(
        state.clone(),
        crate::system_log::middleware::middleware::<S>,
    ));
    let api = api
        .layer(axum::middleware::from_fn(management::http_metrics))
        .layer(axum::middleware::from_fn(management::root_span));

    // ── Ingress overload protection (the API subtree's outermost layer) ──────
    // Bounded in-flight concurrency + load shedding: beyond `cfg.max_in_flight`
    // concurrent API requests the server sheds the excess immediately as
    // `503 Service Unavailable` + `Retry-After` rather than queueing them until
    // it runs out of memory. Being outermost on this subtree, a shed request
    // never reaches auth, audit, or the request body; scoped here so the public
    // status/health/discovery/management endpoints are never shed. No openEHR
    // spec governs server overload — our own design (RFC 9110 §15.6.4).
    let api = crate::overload::shed_layer(api, cfg.server.max_in_flight);

    let inner = mount_public_surface::<S>(&cfg, api, &rest_root);

    let cors = if cfg.server.cors_permissive {
        CorsLayer::very_permissive()
    } else {
        CorsLayer::new()
    };

    // Wrap the whole inner tree in the shared request stack and bind the state,
    // yielding a protocol-complete, state-less service.
    let inner: Router = inner
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(SetSensitiveRequestHeadersLayer::new(std::iter::once(
                    AUTHORIZATION,
                )))
                .layer(TraceLayer::new_for_http())
                .layer(CatchPanicLayer::custom(handle_panic))
                .layer(cors)
                .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    REQUEST_TIMEOUT,
                ))
                .layer(CompressionLayer::new()),
        )
        .with_state(state);

    // ── System Options and Conformance — `OPTIONS`, above the CORS layer ─────
    // The manifest advertises the **live** mounted-group set (System API G-1):
    // the four always-on standardised groups plus `/admin` when its group is
    // enabled. Its identity/conformance fields come from `cfg.server.identity`
    // (G-2/G-6).
    let mut endpoints = vec![
        "/ehr".to_owned(),
        "/definition".to_owned(),
        "/query".to_owned(),
        "/demographic".to_owned(),
    ];
    if cfg.admin.enabled {
        endpoints.push("/admin".to_owned());
    }
    let manifest = Arc::new(system::SystemManifest::new(
        cfg.server.identity.clone(),
        endpoints,
    ));

    // Mount at the API base-path root (System API G-3) and keep a bare-`/` alias
    // for naive root probes; every other request falls through to the
    // CORS-wrapped application. Both sit above CORS so `OPTIONS` is not eaten as
    // a preflight; real per-resource CORS preflights are on sub-paths and reach
    // the CORS layer via the fallback.
    let app: Router = Router::new()
        .route(&cfg.server.base_path, system::route(manifest.clone()))
        .route("/", system::route(manifest))
        .fallback_service(inner);

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

/// [`CatchPanicLayer`] handler: render a panicked handler as the standard
/// openEHR `{ error, message }` error body with a `500`, the same shape every
/// other error path emits ([`crate::overview::error`]) — never tower-http's
/// default `text/plain` body. The panic payload is logged for diagnosis but
/// **never echoed into the response** (it may carry internal detail). No openEHR
/// spec governs the error-body shape (it is a MAY, ITS-REST
/// `Requests_and_responses.md` §HTTP status codes) — our own design keeps every
/// error path consistent.
// The by-value `Box<dyn Any>` parameter is dictated by tower-http's
// `ResponseForPanic` closure signature (`CatchPanicLayer::custom`), not a choice.
#[allow(clippy::needless_pass_by_value)]
fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response {
    let detail = if let Some(s) = err.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_owned()
    };
    tracing::error!(panic = %detail, "request handler panicked");
    error::RestError(ApiError::Internal(
        "the server encountered an internal error".to_owned(),
    ))
    .into_response()
}

/// Mount the public, pre-auth surface around the API tree: status/health,
/// the config-gated SMART `/.well-known/smart-configuration` document (served
/// pre-auth, SMART master04 §Service Discovery; an empty router when SMART is
/// disabled, so the merge is a no-op and the path is absent), and the
/// config-gated Swagger UI + `OpenAPI` documents.
fn mount_public_surface<S: Platform>(
    cfg: &crate::config::AppConfig,
    api: Router<AppState<S>>,
    rest_root: &str,
) -> Router<AppState<S>> {
    // The SMART discovery document is a pure function of static configuration
    // (the openEHR/FHIR base URLs + OIDC issuer), built once inside the router.
    let discovery = smart::discovery::router::<S>(cfg, rest_root);

    let mut inner = Router::new()
        .nest(&cfg.server.base_path, api)
        .merge(status::router(rest_root))
        .merge(discovery);

    if cfg.server.swagger_ui {
        inner = inner.merge(openapi::swagger_router::<S>(cfg));
    }
    inner
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use axum::body::to_bytes;
    use http::StatusCode;

    use super::handle_panic;

    /// F-34: a panicked handler renders the standard openEHR `{ error, message }`
    /// JSON 500 body (never tower-http's default text/plain), and the panic
    /// payload is never echoed into the response.
    #[tokio::test]
    async fn catch_panic_renders_openehr_json_body() {
        let resp = handle_panic(Box::new("secret panic detail"));
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            resp.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"], "Internal Server Error");
        let message = body["message"].as_str().unwrap();
        assert!(
            !message.contains("secret panic detail"),
            "panic payload must not leak into the response body"
        );
    }
}
