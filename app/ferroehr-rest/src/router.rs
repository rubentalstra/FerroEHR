// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Router assembly and the `tower-http` middleware stack.
//!
//! **Base-path rule** (ITS-REST overview `Resources.md` §Resource identification;
//! `Requests_and_responses.md`): every ITS-REST resource path is relative to the
//! configured API base path (`cfg.base_path`, default
//! `/ferroehr/rest/openehr/v1`). The generated API surface ([`crate::api`]) is
//! therefore nested under that base path; the `/rest/status` document hangs off
//! the `/ferroehr/rest` root, the
//! always-on health family (`/health`, `/health/liveness`,
//! `/health/readiness`) answers at the process root, and the System Options
//! manifest answers at the base-path root itself.
//!
//! **Layer order** (innermost → outermost, over the nested API): authentication ·
//! ATNA audit (SM System Log, wraps auth so it observes auth rejections) · HTTP
//! metrics · root span · **overload shedding** (bounded in-flight concurrency +
//! load shed; the API subtree's outermost layer, so a shed request is rejected
//! before auth, audit, or reading the request body — `crate::overload`). The
//! whole tree is then wrapped in the shared `tower-http` request stack
//! (request-id, tracing, CORS, body limit, timeout, compression), so a shed
//! `503` still carries a request id and is traced. Between the CORS layer and
//! the body-limit/timeout pair sits `align_transport_error_body`, which
//! re-renders the two responses those layers produce on their own (`408`,
//! `413`) in the openEHR `{ error, message }` shape. The System Options `OPTIONS`
//! endpoint is mounted **above** the CORS layer — `CorsLayer` treats every
//! `OPTIONS` as a CORS preflight and would short-circuit a conformance probe —
//! so it lives on the outer router with the CORS-wrapped application as its
//! fallback service.
//!
//! **Overload shedding is scoped to the API subtree only** (the openEHR API +
//! its extensions, nested under the base path): the public operational
//! endpoints — the health family, `/rest/status`, SMART discovery, and the
//! management surface — are siblings, so they are never shed and an operator
//! (or an orchestrator probe) can always reach an overloaded server. The bound
//! is `cfg.max_in_flight` (default 1024); `0` installs no layer. No openEHR spec
//! governs server overload — our own design (RFC 9110 §15.6.4).
//!
//! `NormalizePathLayer` is applied at serve time (it must wrap the router to run
//! before routing); see [`crate::serve_with`].

#![allow(
    clippy::disallowed_types,
    reason = "test fixtures and wire assertions are raw JSON by the testing rule \
              (.claude/rules/testing.md §Test-fixture construction); the carriers here are \
              cfg(test)-only, so #[expect] would be unfulfilled in the non-test build"
)]

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use http::{HeaderName, HeaderValue, header};
use openehr_its::rest::runtime::ApiError;
use tower::ServiceBuilder;
use tower_http::CompressionLevel;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::api::system;
use crate::extensions::access::authn::{self, Authenticator};
use crate::extensions::health;
use crate::extensions::management::{self, ManagementState};
use crate::extensions::openapi;
use crate::overview::{error, status};
use crate::smart;
use crate::state::AppState;

/// Per-request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Assembles the full application router.
///
/// Mounts the ITS-REST API (behind the auth layer) nested under the base path,
/// the always-on public health family, the status + SMART-discovery endpoints,
/// the System Options manifest, the optional Swagger UI, and the shared
/// `tower-http` middleware stack.
pub fn router(state: AppState, authenticator: Arc<Authenticator>) -> Router {
    let cfg = state.config().clone();
    let mgmt_rbac = management_rbac(&state);
    let observability = state.observability().clone();
    let rest_root = cfg
        .server
        .base_path
        .strip_suffix("/openehr/v1")
        .unwrap_or(&cfg.server.base_path)
        .to_owned();

    // ── The generated ITS-REST surface, gated by authentication ──────────────
    // A known path called with a method it does not serve renders `405` with
    // the openEHR `{ error, message }` body (overview §HTTP Methods); axum
    // supplies the mandatory `Allow` (RFC 9110 §15.5.6) from the matched route's
    // method set, because `crate::overview::error::method_not_allowed_handler`
    // sets none of its own. An unrecognized method is answered `405` too: axum
    // exposes no such seam, and a catch-all fallback would report every unknown
    // *path* `501` instead of `404`.
    // NOTE: the overview's SHOULD-`501` (`Requests_and_responses.md` §HTTP
    // Methods) is honoured for a recognised but unimplemented *operation* via
    // `ApiError::NotImplemented`; `405` is a predefined code in its own table.
    let api =
        crate::api::api_router().method_not_allowed_fallback(error::method_not_allowed_handler);
    // Tenant resolution sits inside the auth layer so it runs *after*
    // authentication (the principal + its claims are established) and scopes the
    // handler in the tenant task-local. Only installed when tenancy is on — a
    // single-tenant server has no tenant middleware at all.
    let api = if cfg.tenancy.enabled {
        api.layer(from_fn_with_state(
            state.clone(),
            crate::extensions::access::tenant::middleware,
        ))
    } else {
        api
    };
    let api = api.layer(from_fn_with_state(
        authn::AuthLayer {
            authenticator: Arc::clone(&authenticator),
            authz: state.authz(),
        },
        authn::middleware,
    ));
    // Always install the ATNA audit layer; it early-returns when the platform's
    // SM `SystemLog` reports auditing off, so the no-audit case costs one check
    // per request.
    let api = api.layer(from_fn_with_state(
        state.clone(),
        crate::system_log::middleware::middleware,
    ));
    // The principal-keyed rate-limit tier (`crate::rate_limit`): added before the
    // authentication layer, which in axum's outermost-last ordering is what puts
    // it INSIDE — the subject it keys on only exists once authentication has run.
    let api = match crate::rate_limit::principal_layer(&cfg.server.rate_limit) {
        Some(tier) => api.layer(tier),
        None => api,
    };
    // Per-route-family body limits (`crate::limits`): the clinical tier for the
    // ordinary surface, the bulk tier for template upload and the `/message`
    // imports. Above the audit layer so an over-large body is refused before it
    // is read, buffered, or audited; the outer `RequestBodyLimitLayer` remains
    // the ceiling across the whole tree.
    let api = api.layer(from_fn_with_state(state.clone(), crate::limits::middleware));
    let api = api
        .layer(axum::middleware::from_fn(
            management::http_metrics::http_metrics,
        ))
        .layer(axum::middleware::from_fn(
            management::http_metrics::root_span,
        ));

    // ── Ingress overload protection (the API subtree's outermost layer) ──────
    // Bounded in-flight concurrency + load shedding: beyond `cfg.max_in_flight`
    // concurrent API requests the server sheds the excess immediately as
    // `503 Service Unavailable` + `Retry-After` rather than queueing them until
    // it runs out of memory. Being outermost on this subtree, a shed request
    // never reaches auth, audit, or the request body; scoped here so the public
    // health/status/discovery/management endpoints are never shed. No openEHR
    // spec governs server overload — our own design (RFC 9110 §15.6.4).
    let api = crate::overload::shed_layer(api, cfg.server.max_in_flight);

    let inner = mount_public_surface(&cfg, api, &rest_root);

    let cors = if cfg.server.cors_permissive {
        CorsLayer::very_permissive()
    } else {
        CorsLayer::new()
    };

    // The address-keyed tier covers the WHOLE tree, the always-on health family
    // included: an orchestrator probe is cheap and its rate is nowhere near this
    // ceiling, while a flood arriving at any path is refused here before it
    // reaches authentication.
    let inner = match crate::rate_limit::address_layer(&cfg.server.rate_limit) {
        Some(tier) => inner.layer(tier),
        None => inner,
    };

    let inner: Router = with_shared_stack(inner, cfg.server.limits.ceiling(), cors, state);

    // ── System Options and Conformance — `OPTIONS`, above the CORS layer ─────
    // The manifest advertises the **live** mounted-group set (System API): the
    // four always-on standardised groups plus `/admin` when its group is
    // enabled, so it never advertises a group that answers 404. Its
    // identity/conformance fields come from `cfg.server.identity`. Extension
    // families are deliberately absent — the operation's job is "exposing
    // service capabilities for a conformance manifest", and they declare
    // themselves through the served OpenAPI document instead.
    // NOTE: no released text says what `endpoints` must contain — the System
    // API chapter is a stub and the OAS `schemas/others/Options.yaml` gives only
    // `array of string` — so the content is adjudicated as above.
    let mut endpoints = vec![
        "/ehr".to_owned(),
        "/definition".to_owned(),
        "/query".to_owned(),
        "/demographic".to_owned(),
    ];
    if cfg.admin.enabled {
        endpoints.push("/admin".to_owned());
    }
    let manifest = Arc::new(system::options::SystemManifest::new(
        cfg.server.identity.clone(),
        endpoints,
    ));

    // Mount at the API base-path root — the ONE location the System API
    // defines (`system.openapi.yaml`: servers `{baseUrl}/v1`, path `/` — the
    // operation lives at the API root, nowhere else; the former bare-`/`
    // alias was our own duplication and is removed, #420). Every other
    // request falls through to the CORS-wrapped application. The mount sits
    // above CORS so `OPTIONS` is not eaten as a preflight; real per-resource
    // CORS preflights are on sub-paths and reach the CORS layer via the
    // fallback.
    let app: Router = Router::new()
        .route(&cfg.server.base_path, system::options::route(manifest))
        .fallback_service(inner);

    // Merge the management surface only when enabled AND not bound to a separate
    // port (the binary serves the separate-port case on its own listener).
    let app = if observability.management.enabled && observability.management.port.is_none() {
        let mgmt = management::router(ManagementState::from_observability(
            observability,
            authenticator,
            mgmt_rbac,
        ));
        app.merge(mgmt)
    } else {
        app
    };
    with_security_headers(app)
}

/// Applies the shared request stack and binds the state, yielding a
/// protocol-complete, state-less service.
///
/// The stack is applied in two halves with [`align_transport_error_body`]
/// between them: `TimeoutLayer` and `RequestBodyLimitLayer` render their own
/// responses without ever reaching [`crate::overview::error`], so the mapping
/// must sit OUTSIDE both. Order (outermost → innermost): request-id, tracing,
/// catch-panic, CORS, body-limit ceiling, timeout, compression.
///
/// `body_ceiling` is the widest tier any route accepts
/// ([`BodyLimits::ceiling`](ferroehr::config::server::BodyLimits::ceiling)); the
/// narrower per-family tier is applied inside, by [`crate::limits`].
fn with_shared_stack(
    inner: Router<AppState>,
    body_ceiling: usize,
    cors: CorsLayer,
    state: AppState,
) -> Router {
    inner
        .layer(
            ServiceBuilder::new()
                .layer(RequestBodyLimitLayer::new(body_ceiling))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    REQUEST_TIMEOUT,
                ))
                // NOTE (docs.rs tower-http `CompressionLayer::quality`):
                // Fastest — the default brotli/gzip levels spend visible
                // per-response CPU for a marginal ratio gain on ~KB bodies.
                .layer(CompressionLayer::new().quality(CompressionLevel::Fastest)),
        )
        .layer(axum::middleware::map_response(align_transport_error_body))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(SetSensitiveRequestHeadersLayer::new(std::iter::once(
                    AUTHORIZATION,
                )))
                .layer(TraceLayer::new_for_http().make_span_with(path_only_span))
                .layer(CatchPanicLayer::custom(handle_panic))
                .layer(cors),
        )
        .with_state(state)
}

/// The response headers every surface carries, per the OWASP HTTP Headers Cheat
/// Sheet — outermost, so they reach transport-layer responses (the `413` from the
/// body-limit layer, the `408` from the timeout layer, the `500` from the panic
/// handler) as well as handler ones.
///
/// The set is chosen for what this surface actually is — a clinical JSON API —
/// rather than copied wholesale from browser advice:
///
/// - `Cache-Control: no-store` is the load-bearing one. Responses carry patient
///   data, and the cheat sheet names `no-store` for exactly that. It does not
///   affect openEHR's `ETag`/`If-Match` concurrency control, which is a
///   precondition mechanism rather than a caching one.
/// - `X-Content-Type-Options: nosniff` stops a browser or proxy from re-guessing
///   a declared `application/json` as something executable.
/// - `Referrer-Policy: strict-origin-when-cross-origin` keeps request paths — which
///   carry `ehr_id` and version identifiers — out of cross-origin `Referer`
///   headers.
/// - `Cross-Origin-Resource-Policy: same-site` refuses cross-site embedding of
///   these responses.
/// - `X-Frame-Options: DENY` and a minimal CSP are here for the HTML this origin
///   can serve (Swagger UI, and any error page a proxy renders). The cheat sheet
///   is explicit that CSP "might be meaningless in the response of a REST API
///   that returns content that is not going to be rendered", so the value is the
///   defensive minimum rather than a policy pretending to govern scripts:
///   nothing may load, and nothing may frame it.
///
/// Deliberately absent: `Strict-Transport-Security`, because it is a property of
/// the TLS edge rather than of this server — RFC 6797 §7.2 requires a browser to
/// IGNORE it over plain HTTP, and this server is commonly reached over HTTP behind
/// a terminating proxy that owns the header. Setting it here would be inert at
/// best and misleading at worst. `X-XSS-Protection` and `X-Powered-By` are absent
/// because the cheat sheet says to remove them, and hyper sends no `Server`
/// header to begin with.
fn with_security_headers(app: Router) -> Router {
    app.layer(
        ServiceBuilder::new()
            .layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                header::REFERRER_POLICY,
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                HeaderName::from_static("cross-origin-resource-policy"),
                HeaderValue::from_static("same-site"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                header::X_FRAME_OPTIONS,
                HeaderValue::from_static("DENY"),
            ))
            // `if_not_present`, not `overriding`: a surface on this origin that
            // is actually RENDERED needs its own policy, and being outermost
            // this layer would otherwise erase it on the way out. Swagger UI
            // sets one (`crate::extensions::openapi`); everything else gets this
            // default, which says nothing loads and nothing frames.
            .layer(SetResponseHeaderLayer::if_not_present(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
            )),
    )
}

/// The `tower-http` request span, recording the path WITHOUT its query string.
///
/// `DefaultMakeSpan` records `uri = %request.uri()`, which is the whole target
/// including the query — and `subject_id`, an external patient identifier, is a
/// query parameter by the specification's own definition (ITS-REST EHR API,
/// `GET /ehr?subject_id=…&subject_namespace=…`). At `log.filter = debug` that
/// would put a patient identifier into the ordinary application log, which is a
/// place identified data must never reach. Dropping the query keeps the path —
/// where the ids are system-assigned `ehr_id`/version uids that the audit log
/// records anyway — and loses the one component that carries an external
/// identifier.
///
/// The route TEMPLATE (never a concrete path) is recorded separately by
/// [`management::http_metrics::root_span`], which is the span telemetry consumes.
fn path_only_span(request: &http::Request<axum::body::Body>) -> tracing::Span {
    tracing::debug_span!(
        "request",
        method = %request.method(),
        path = %request.uri().path(),
        version = ?request.version(),
    )
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
#[expect(
    clippy::needless_pass_by_value,
    reason = "tower-http's `CatchPanicLayer` handler contract hands the panic \
              payload over by value"
)]
fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response {
    let detail = if let Some(s) = err.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_owned()
    };
    tracing::error!(panic = %detail, "request handler panicked");
    error::RestError(ApiError::Internal(error::INTERNAL_MESSAGE.to_owned())).into_response()
}

/// Align the two **transport-layer** error responses the `tower-http` stack
/// renders on its own onto the openEHR `{ error, message }` body every other
/// error path emits ([`crate::overview::error`]).
///
/// `TimeoutLayer` builds its `408` as a bare status with an empty body
/// (`tower_http::timeout`) and `RequestBodyLimitLayer` builds its `413` as
/// `text/plain; charset=utf-8` (`tower_http::limit`); neither passes through
/// [`crate::overview::error::RestError`], so without this mapping the two would
/// be the only error responses on the server in a foreign shape. `408` is a
/// predefined status in the spec's table (ITS-REST `Requests_and_responses.md`
/// §HTTP status codes — "Request maximum execution time is reached, therefore
/// the server aborted the request"); `413` is one of the additional codes the
/// same section permits ("Additional status codes MAY be used as long as they
/// do not conflict with the predefined codes"). The error BODY itself is only a
/// MAY there, so this is consistency across our own surface rather than a
/// conformance requirement.
///
/// A response that already carries a JSON body was produced by a handler (the
/// AQL query-execution timeout renders its own `408` through `RestError`) and
/// is passed through untouched.
async fn align_transport_error_body(response: Response) -> Response {
    let status = response.status();
    let message = match status {
        StatusCode::REQUEST_TIMEOUT => {
            "the request exceeded the maximum execution time and was aborted"
        }
        StatusCode::PAYLOAD_TOO_LARGE => {
            "the request body exceeds the maximum size accepted by this server"
        }
        _ => return response,
    };
    let already_rendered = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if already_rendered {
        return response;
    }
    error::status_error_response(status, message)
}

/// Mount the public, pre-auth surface around the API tree: the always-on health
/// family, the REST-root status surface, the config-gated SMART
/// `/.well-known/smart-configuration` document (served pre-auth, SMART master04
/// §Service Discovery; an empty router when SMART is disabled, so the merge is a
/// no-op and the path is absent), and the config-gated Swagger UI + `OpenAPI`
/// documents.
///
/// NOTE: no openEHR spec governs this — our own operational surface; disposition
/// recorded on issue #305. The health family (`/health`, `/health/liveness`,
/// `/health/readiness` — [`crate::extensions::health`]) is mounted
/// **unconditionally and ungated** here, deliberately outside the API subtree:
/// an orchestrator must be able to probe the server without credentials, without
/// any configuration having been enabled, and without the overload-shed layer
/// being able to shed the probe.
fn mount_public_surface(
    cfg: &crate::config::AppConfig,
    api: Router<AppState>,
    rest_root: &str,
) -> Router<AppState> {
    // The SMART discovery document is a pure function of static configuration
    // (the openEHR/FHIR base URLs + OIDC issuer), built once inside the router.
    let discovery = smart::discovery::router(cfg, rest_root);

    let mut inner = Router::new()
        .nest(&cfg.server.base_path, api)
        .merge(health::router())
        .merge(status::router(rest_root))
        .merge(discovery);

    if cfg.server.swagger_ui {
        inner = inner.merge(openapi::swagger_router(cfg));
    }
    inner
}

/// The RBAC rules the management `AdminOnly` gate applies: the live handle's
/// rule set, or a disabled rule set when no RBAC gate is wired (auth-only
/// deployments — authenticated is then enough, issue #1879).
fn management_rbac(state: &AppState) -> ferroehr::config::authz::RbacConfig {
    state
        .authz()
        .as_deref()
        .and_then(|h| h.rbac_rules().cloned())
        .unwrap_or_else(|| ferroehr::config::authz::RbacConfig {
            enabled: false,
            ..ferroehr::config::authz::RbacConfig::default()
        })
}

/// Build the standalone management router (separate-port mode). The binary
/// serves this on the management listener when `management.port` is set.
pub fn management_router(state: &AppState, authenticator: Arc<Authenticator>) -> Router {
    management::router(ManagementState::from_observability(
        state.observability().clone(),
        authenticator,
        management_rbac(state),
    ))
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::response::Response;
    use http::StatusCode;

    use super::{align_transport_error_body, handle_panic};

    /// a panicked handler renders the standard openEHR `{ error, message }`
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

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        assert_eq!(
            resp.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    /// The `tower-http` `TimeoutLayer` renders `408` as a bare status with an
    /// empty body; the alignment re-renders it in the openEHR
    /// `{ error, message }` shape. `408` is a predefined status in the spec's
    /// own table (ITS-REST `Requests_and_responses.md` §HTTP status codes —
    /// "Request maximum execution time is reached, therefore the server aborted
    /// the request"), so only the body shape is ours to choose.
    #[tokio::test]
    async fn layer_produced_408_is_rendered_as_the_openehr_error_body() {
        let mut bare = Response::new(Body::empty());
        *bare.status_mut() = StatusCode::REQUEST_TIMEOUT;
        let (status, body) = json_body(align_transport_error_body(bare).await).await;
        assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
        assert_eq!(body["error"], "Request Timeout");
        assert!(
            body.get("message")
                .and_then(serde_json::Value::as_str)
                .is_some()
        );
    }

    /// The `RequestBodyLimitLayer` renders `413` as `text/plain`; the alignment
    /// re-renders it in the openEHR `{ error, message }` shape. `413` is not in
    /// the spec's status table but is permitted as an additional,
    /// non-conflicting code (`Requests_and_responses.md` §HTTP status codes).
    #[tokio::test]
    async fn layer_produced_413_is_rendered_as_the_openehr_error_body() {
        let mut bare = Response::new(Body::from("length limit exceeded"));
        *bare.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
        bare.headers_mut().insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        let (status, body) = json_body(align_transport_error_body(bare).await).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(body["error"], "Payload Too Large");
        assert!(
            body["message"]
                .as_str()
                .is_some_and(|m| !m.contains("length limit exceeded")),
            "the tower-http text body must be replaced, not embedded"
        );
    }

    /// A handler-produced `408` (the AQL query-execution timeout, rendered
    /// through `RestError`) already carries the openEHR body and must pass
    /// through the alignment untouched — its message names the real cause.
    #[tokio::test]
    async fn handler_produced_408_passes_through_unchanged() {
        let original = crate::overview::error::status_error_response(
            StatusCode::REQUEST_TIMEOUT,
            "query aborted after 5000ms",
        );
        let (status, body) = json_body(align_transport_error_body(original).await).await;
        assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
        assert_eq!(body["message"], "query aborted after 5000ms");
    }

    /// Every other status is returned byte-for-byte: the alignment is scoped to
    /// the two layer-produced transport errors and must never touch a success
    /// or another error path.
    #[tokio::test]
    async fn other_statuses_are_untouched() {
        let mut ok = Response::new(Body::from("hello"));
        ok.headers_mut().insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("text/plain"),
        );
        let resp = align_transport_error_body(ok).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "text/plain"
        );
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(bytes.as_ref(), b"hello");
    }
}
