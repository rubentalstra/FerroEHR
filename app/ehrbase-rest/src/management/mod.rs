//! The management surface (binding doc §2/§3): health, info, Prometheus,
//! metrics, env, and loggers — every endpoint **off by default**, each opt-in
//! via [`ManagementConfig`], gated by its own access-level layer (reusing the
//! P11 authentication primitives), and optionally served from a separate
//! internal port. Observability must never widen the clinical API's attack
//! surface.

pub mod config;
mod env;
pub mod health;
mod http_metrics;
pub mod info;
mod loggers;
mod metrics;

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use http::{HeaderMap, StatusCode, header};
use metrics_exporter_prometheus::PrometheusHandle;
use serde_json::Value;

use crate::access::authn::Authenticator;
use crate::error::RestError;
use openehr_its::rest::runtime::ApiError;

pub use config::{AccessLevel, EndpointLevels, ManagementConfig};
pub use health::{Health, HealthIndicator, HealthRegistry, HealthStatus};
pub use http_metrics::{
    AUTH_FAILURES, HTTP_ACTIVE_REQUESTS, HTTP_REQUEST_BODY_SIZE, HTTP_REQUEST_DURATION,
    HTTP_RESPONSE_BODY_SIZE, http_metrics, root_span,
};
pub use info::BuildInfo;
pub use loggers::LogReload;

/// Everything the management router needs. Assembled by the binary (which owns
/// the telemetry handles, the health indicators, and the effective config) and
/// handed to [`router`]. Cheap to clone.
#[derive(Clone)]
pub struct ManagementState {
    /// The management configuration (which endpoints, what access levels).
    pub config: ManagementConfig,
    /// The authentication layer, reused for access gating.
    pub authenticator: Arc<Authenticator>,
    /// The Prometheus render handle (present iff the recorder is installed).
    pub prometheus: Option<PrometheusHandle>,
    /// The runtime log-filter control (present iff the reloadable filter is set).
    pub log_reload: Option<LogReload>,
    /// The health-indicator registry.
    pub health: HealthRegistry,
    /// Build/spec provenance for `/info` and the build-info gauge.
    pub build_info: BuildInfo,
    /// The effective configuration snapshot for `/env` (redacted at render).
    pub env_snapshot: Arc<Value>,
}

impl std::fmt::Debug for ManagementState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagementState")
            .field("config", &self.config)
            .field("prometheus", &self.prometheus.is_some())
            .field("log_reload", &self.log_reload.is_some())
            .field("health_indicators", &!self.health.is_empty())
            .finish_non_exhaustive()
    }
}

impl ManagementState {
    /// Assemble the management state from the observability bundle the binary
    /// built and the shared authenticator.
    #[must_use]
    pub fn from_observability(obs: Observability, authenticator: Arc<Authenticator>) -> Self {
        Self {
            config: obs.management,
            authenticator,
            prometheus: obs.prometheus,
            log_reload: obs.log_reload,
            health: obs.health,
            build_info: obs.build_info,
            env_snapshot: obs.env_snapshot,
        }
    }
}

/// The observability inputs the binary assembles and carries in
/// [`AppState`](crate::AppState): the management configuration, the telemetry
/// render/reload handles, the health registry, build provenance, and the
/// redacted config snapshot. Everything defaults **off** (management disabled,
/// no handles, empty registry) so a server without observability is the clean
/// default, not a special case.
#[derive(Clone)]
pub struct Observability {
    /// The management surface configuration.
    pub management: ManagementConfig,
    /// The Prometheus render handle (present iff the recorder is installed).
    pub prometheus: Option<PrometheusHandle>,
    /// The runtime log-filter control (present iff the reloadable filter is set).
    pub log_reload: Option<LogReload>,
    /// The health-indicator registry.
    pub health: HealthRegistry,
    /// Build/spec provenance.
    pub build_info: BuildInfo,
    /// The redacted effective-config snapshot for `/management/env`.
    pub env_snapshot: Arc<Value>,
}

impl Default for Observability {
    fn default() -> Self {
        Self {
            management: ManagementConfig::default(),
            prometheus: None,
            log_reload: None,
            health: HealthRegistry::default(),
            build_info: BuildInfo::current(),
            env_snapshot: Arc::new(Value::Null),
        }
    }
}

impl std::fmt::Debug for Observability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observability")
            .field("management", &self.management)
            .field("prometheus", &self.prometheus.is_some())
            .field("log_reload", &self.log_reload.is_some())
            .field("health_indicators", &!self.health.is_empty())
            .finish_non_exhaustive()
    }
}

/// Build the management router (a self-contained `Router` with its own state).
///
/// Only the opted-in endpoints are mounted; a disabled endpoint is simply
/// absent → `404`. When [`ManagementConfig::enabled`] is `false`, the router is
/// empty. This same router is either merged into the main app (when
/// `management.port` is unset) or served on its own listener by the binary.
pub fn router(state: ManagementState) -> Router {
    if !state.config.enabled {
        return Router::new();
    }

    let base = state.config.base_path.trim_end_matches('/').to_owned();
    let cfg = state.config.clone();
    let mut router = Router::new();

    // The per-endpoint access-level layer builder: an owned clone of the
    // authenticator (so `router.with_state(state)` below can still move `state`)
    // wrapped with the required level for one route.
    let authenticator = state.authenticator.clone();
    let mk = |level: AccessLevel| {
        from_fn_with_state(
            AccessGuard {
                authenticator: authenticator.clone(),
                level,
            },
            access_middleware,
        )
    };

    // ── Health (aggregate) ──────────────────────────────────────────────────
    if cfg.endpoints.health.is_mounted() {
        router = router.route(
            &format!("{base}/health"),
            get(aggregate_health).route_layer(mk(cfg.endpoints.health)),
        );
    }

    // ── Liveness / readiness probes (public when enabled) ───────────────────
    if cfg.probes_enabled {
        router = router
            .route(&format!("{base}/health/liveness"), get(liveness))
            .route(&format!("{base}/health/readiness"), get(readiness));
    }

    // ── Info ─────────────────────────────────────────────────────────────────
    if cfg.endpoints.info.is_mounted() {
        router = router.route(
            &format!("{base}/info"),
            get(info_view).route_layer(mk(cfg.endpoints.info)),
        );
    }

    // ── Prometheus text exposition (only when the recorder is installed) ─────
    if cfg.endpoints.prometheus.is_mounted() && state.prometheus.is_some() {
        router = router.route(
            &format!("{base}/prometheus"),
            get(prometheus_text).route_layer(mk(cfg.endpoints.prometheus)),
        );
    }

    // ── Actuator-style JSON metrics view (only when the recorder is installed)
    if cfg.endpoints.metrics.is_mounted() && state.prometheus.is_some() {
        router = router
            .route(
                &format!("{base}/metrics"),
                get(metrics_list).route_layer(mk(cfg.endpoints.metrics)),
            )
            .route(
                &format!("{base}/metrics/{{name}}"),
                get(metrics_detail).route_layer(mk(cfg.endpoints.metrics)),
            );
    }

    // ── Env (redacted config) ──────────────────────────────────────────────
    if cfg.endpoints.env.is_mounted() {
        router = router.route(
            &format!("{base}/env"),
            get(env_view).route_layer(mk(cfg.endpoints.env)),
        );
    }

    // ── Loggers (only when a reloadable filter is present) ──────────────────
    if cfg.endpoints.loggers.is_mounted() && state.log_reload.is_some() {
        router = router.route(
            &format!("{base}/loggers"),
            get(loggers_get)
                .post(loggers_post)
                .delete(loggers_reset)
                .route_layer(mk(cfg.endpoints.loggers)),
        );
    }

    router.with_state(state)
}

// ── Handlers ────────────────────────────────────────────────────────────────

async fn aggregate_health(State(s): State<ManagementState>) -> Response {
    health::aggregate(s.health).await
}

// The handlers below have no I/O of their own but must be `async` — axum only
// implements `Handler` for async functions; the `unused_async` allow records
// that the work is synchronous and the async is the framework contract.

#[allow(clippy::unused_async)]
async fn liveness() -> Response {
    health::liveness()
}

async fn readiness(State(s): State<ManagementState>) -> Response {
    health::readiness(s.health).await
}

#[allow(clippy::unused_async)]
async fn info_view(State(s): State<ManagementState>) -> Json<BuildInfo> {
    info::info(s.build_info)
}

#[allow(clippy::unused_async)]
async fn prometheus_text(State(s): State<ManagementState>) -> Response {
    match &s.prometheus {
        Some(handle) => metrics::prometheus(handle),
        None => recorder_unavailable(),
    }
}

#[allow(clippy::unused_async)]
async fn metrics_list(State(s): State<ManagementState>) -> Response {
    match &s.prometheus {
        Some(handle) => metrics::list(handle).into_response(),
        None => recorder_unavailable(),
    }
}

#[allow(clippy::unused_async)]
async fn metrics_detail(State(s): State<ManagementState>, path: Path<String>) -> Response {
    match &s.prometheus {
        Some(handle) => metrics::detail(handle, path),
        None => recorder_unavailable(),
    }
}

#[allow(clippy::unused_async)]
async fn env_view(State(s): State<ManagementState>) -> Json<Value> {
    env::env(&s.env_snapshot)
}

#[allow(clippy::unused_async)]
async fn loggers_get(State(s): State<ManagementState>) -> Response {
    match &s.log_reload {
        Some(reload) => loggers::get(reload).into_response(),
        None => recorder_unavailable(),
    }
}

#[allow(clippy::unused_async)]
async fn loggers_post(
    State(s): State<ManagementState>,
    body: Json<loggers::SetFilter>,
) -> Response {
    match &s.log_reload {
        Some(reload) => loggers::set(reload, &body.0),
        None => recorder_unavailable(),
    }
}

#[allow(clippy::unused_async)]
async fn loggers_reset(State(s): State<ManagementState>) -> Response {
    match &s.log_reload {
        Some(reload) => loggers::reset(reload),
        None => recorder_unavailable(),
    }
}

fn recorder_unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(
            serde_json::json!({ "error": "the corresponding telemetry facility is not installed" }),
        ),
    )
        .into_response()
}

// ── Access-level layer ────────────────────────────────────────────────────

/// The per-endpoint access guard: the shared authenticator plus the required
/// level for one route.
#[derive(Clone)]
struct AccessGuard {
    authenticator: Arc<Authenticator>,
    level: AccessLevel,
}

/// The access-level middleware: enforce the guard's level, then run the route
/// (or short-circuit with `401`/`403`/`404`). Installed per-route via the `mk`
/// closure in [`router`].
async fn access_middleware(
    State(guard): State<AccessGuard>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    match guard.check(req.headers()).await {
        Ok(()) => next.run(req).await,
        Err(resp) => resp,
    }
}

impl AccessGuard {
    /// Enforce the access level against the request headers.
    async fn check(&self, headers: &HeaderMap) -> Result<(), Response> {
        match self.level {
            // Defensive: an `Off` endpoint is never mounted; if reached, 404.
            AccessLevel::Off => Err(StatusCode::NOT_FOUND.into_response()),
            AccessLevel::Public => Ok(()),
            AccessLevel::Private | AccessLevel::AdminOnly => {
                // Auth disabled (dev): the surface is unauthenticated by design.
                if !self.authenticator.enabled() {
                    return Ok(());
                }
                let principal = self
                    .authenticator
                    .authenticate(headers)
                    .await
                    .map_err(|_| unauthorized(&self.authenticator))?;
                if self.level == AccessLevel::AdminOnly
                    && let Some(scope) = self.authenticator.admin_scope()
                    && !principal.scopes.iter().any(|s| s == scope)
                {
                    return Err(forbidden(scope));
                }
                Ok(())
            }
        }
    }
}

fn unauthorized(authenticator: &Authenticator) -> Response {
    let mut resp =
        RestError(ApiError::Unauthorized("authentication required".to_owned())).into_response();
    resp.headers_mut()
        .insert(header::WWW_AUTHENTICATE, authenticator.challenge());
    resp
}

fn forbidden(scope: &str) -> Response {
    RestError(ApiError::Forbidden(format!(
        "management endpoint requires the '{scope}' scope"
    )))
    .into_response()
}
