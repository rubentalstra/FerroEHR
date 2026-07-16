//! The management surface: health, info, Prometheus, metrics, env, and loggers.
//!
//! **No openEHR spec governs this — our own operational surface** (the
//! classification register `docs/design/its-rest/extensions.md` verdict for
//! `management/`: pure ops, spec-silent by design). The design authority is the
//! observability binding design `docs/design/observability.md` (cited as
//! "binding doc §N" throughout this subtree).
//!
//! Every endpoint is **off by default**, each opt-in via [`ManagementConfig`],
//! gated by its own access-level layer (reusing the authentication primitives),
//! and optionally served from a separate internal port (binding doc §2/§3).
//! Observability must never widen the clinical API's attack surface.

mod env;
mod health_routes;
pub mod http_metrics;
mod info_routes;
mod logger_routes;
mod metrics;

use ehrbase::config::management::{AccessLevel, ManagementConfig};
use ehrbase::telemetry::build_info::BuildInfo;
use ehrbase::telemetry::health::{HealthRegistry};
use ehrbase::telemetry::log_reload::LogReload;

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use http::{HeaderMap, StatusCode, header};
use metrics_exporter_prometheus::PrometheusHandle;
use serde_json::Value;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::extensions::access::authn::Authenticator;
use crate::overview::error::RestError;
use openehr_its::rest::runtime::ApiError;

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
/// [`AppState`](crate::state::AppState): the management configuration, the telemetry
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

// ── OpenAPI document (the full management surface, documented unconditionally) ─

/// The management surface's `OpenAPI` document — every management operation,
/// documented **unconditionally** (the document describes the product surface;
/// the live [`router`] mounts only the opted-in endpoints). Built natively with
/// `utoipa-axum` so each operation's route + `OpenAPI` path come from the one
/// `#[utoipa::path]` handler; only the `OpenApi` half is kept here (the mounted,
/// access-gated router is built by [`router`]).
///
/// No openEHR spec governs the management surface — our own operational design.
#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    OpenApiRouter::<ManagementState>::new()
        .routes(routes!(aggregate_health))
        .routes(routes!(liveness))
        .routes(routes!(readiness))
        .routes(routes!(info_view))
        .routes(routes!(prometheus_text))
        .routes(routes!(metrics_list))
        .routes(routes!(metrics_detail))
        .routes(routes!(env_view))
        .routes(routes!(loggers_get, loggers_post, loggers_reset))
        .into_openapi()
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// Aggregate health (all registered indicators). 200 when UP/DEGRADED, 503 when
/// DOWN. Access-level gated.
#[utoipa::path(
    get, path = "/management/health", tag = "management",
    responses(
        (status = 200, description = "Aggregate health UP or DEGRADED.", body = serde_json::Value),
        (status = 503, description = "Aggregate health DOWN.", body = serde_json::Value)
    )
)]
async fn aggregate_health(State(s): State<ManagementState>) -> Response {
    health_routes::aggregate(s.health).await
}

// The handlers below have no I/O of their own but must be `async` — axum only
// implements `Handler` for async functions; the `unused_async` allow records
// that the work is synchronous and the async is the framework contract.

/// Kubernetes-style liveness probe (public when probes are enabled).
#[utoipa::path(
    get, path = "/management/health/liveness", tag = "management",
    responses((status = 200, description = "Process alive.", body = serde_json::Value))
)]
#[allow(clippy::unused_async)]
async fn liveness() -> Response {
    health_routes::liveness()
}

/// Kubernetes-style readiness probe (public when probes are enabled). 503 when
/// not ready.
#[utoipa::path(
    get, path = "/management/health/readiness", tag = "management",
    responses(
        (status = 200, description = "Ready to serve.", body = serde_json::Value),
        (status = 503, description = "Not ready.", body = serde_json::Value)
    )
)]
async fn readiness(State(s): State<ManagementState>) -> Response {
    health_routes::readiness(s.health).await
}

/// Build/spec provenance (`/info`): version, git, spec pins. Access-level gated.
#[utoipa::path(
    get, path = "/management/info", tag = "management",
    responses((status = 200, description = "Build + spec provenance.", body = serde_json::Value))
)]
#[allow(clippy::unused_async)]
async fn info_view(State(s): State<ManagementState>) -> Json<BuildInfo> {
    info_routes::info(s.build_info)
}

/// Prometheus text exposition. 503 when the recorder is not installed.
/// Access-level gated.
#[utoipa::path(
    get, path = "/management/prometheus", tag = "management",
    responses(
        (status = 200, description = "Prometheus exposition text.", content_type = "text/plain"),
        (status = 503, description = "Metrics recorder not installed.", body = serde_json::Value)
    )
)]
#[allow(clippy::unused_async)]
async fn prometheus_text(State(s): State<ManagementState>) -> Response {
    match &s.prometheus {
        Some(handle) => metrics::prometheus(handle),
        None => recorder_unavailable(),
    }
}

/// Actuator-style JSON list of known metric names. 503 when the recorder is not
/// installed. Access-level gated.
#[utoipa::path(
    get, path = "/management/metrics", tag = "management",
    responses(
        (status = 200, description = "Known metric names.", body = serde_json::Value),
        (status = 503, description = "Metrics recorder not installed.", body = serde_json::Value)
    )
)]
#[allow(clippy::unused_async)]
async fn metrics_list(State(s): State<ManagementState>) -> Response {
    match &s.prometheus {
        Some(handle) => metrics::list(handle).into_response(),
        None => recorder_unavailable(),
    }
}

/// Actuator-style JSON detail for one metric. 404 when the metric is unknown,
/// 503 when the recorder is not installed. Access-level gated.
#[utoipa::path(
    get, path = "/management/metrics/{name}", tag = "management",
    params(("name" = String, Path, description = "The metric name.")),
    responses(
        (status = 200, description = "The metric's current value(s).", body = serde_json::Value),
        (status = 404, description = "Unknown metric.", body = serde_json::Value)
    )
)]
#[allow(clippy::unused_async)]
async fn metrics_detail(State(s): State<ManagementState>, path: Path<String>) -> Response {
    match &s.prometheus {
        Some(handle) => metrics::detail(handle, path),
        None => recorder_unavailable(),
    }
}

/// The redacted effective-configuration snapshot (`/env`). Access-level gated.
#[utoipa::path(
    get, path = "/management/env", tag = "management",
    responses((status = 200, description = "Redacted effective configuration.", body = serde_json::Value))
)]
#[allow(clippy::unused_async)]
async fn env_view(State(s): State<ManagementState>) -> Json<Value> {
    env::env(&s.env_snapshot)
}

/// The effective log-filter directives + boot filter. 503 when no reloadable
/// filter is installed. Access-level gated.
#[utoipa::path(
    get, path = "/management/loggers", tag = "management",
    responses(
        (status = 200, description = "Effective + boot log filter.", body = serde_json::Value),
        (status = 503, description = "No reloadable filter installed.", body = serde_json::Value)
    )
)]
#[allow(clippy::unused_async)]
async fn loggers_get(State(s): State<ManagementState>) -> Response {
    match &s.log_reload {
        Some(reload) => logger_routes::get(reload).into_response(),
        None => recorder_unavailable(),
    }
}

/// Swap the live log filter. Body: `{"filter": "ehrbase=debug,sqlx=warn"}`. 400
/// on a parse error, 503 when no reloadable filter is installed. Access-level
/// gated.
#[utoipa::path(
    post, path = "/management/loggers", tag = "management",
    request_body(content = serde_json::Value, description = "`{\"filter\": \"<env-filter directives>\"}`"),
    responses(
        (status = 200, description = "Filter applied.", body = serde_json::Value),
        (status = 400, description = "Malformed filter directives.", body = serde_json::Value),
        (status = 503, description = "No reloadable filter installed.", body = serde_json::Value)
    )
)]
#[allow(clippy::unused_async)]
async fn loggers_post(
    State(s): State<ManagementState>,
    body: Json<logger_routes::SetFilter>,
) -> Response {
    match &s.log_reload {
        Some(reload) => logger_routes::set(reload, &body.0),
        None => recorder_unavailable(),
    }
}

/// Reset the log filter to the boot filter. 503 when no reloadable filter is
/// installed. Access-level gated.
#[utoipa::path(
    delete, path = "/management/loggers", tag = "management",
    responses(
        (status = 200, description = "Filter reset to boot value.", body = serde_json::Value),
        (status = 503, description = "No reloadable filter installed.", body = serde_json::Value)
    )
)]
#[allow(clippy::unused_async)]
async fn loggers_reset(State(s): State<ManagementState>) -> Response {
    match &s.log_reload {
        Some(reload) => logger_routes::reset(reload),
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
                let authenticated = self
                    .authenticator
                    .authenticate(headers)
                    .await
                    .map_err(|_| unauthorized(&self.authenticator))?;
                if self.level == AccessLevel::AdminOnly
                    && let Some(scope) = self.authenticator.admin_scope()
                    && !authenticated.principal.scopes.iter().any(|s| s == scope)
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
