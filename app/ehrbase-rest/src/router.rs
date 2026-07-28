//! Router assembly and the `tower-http` middleware stack.
//!
//! **Base-path rule** (ITS-REST overview `Resources.md` §Resource identification;
//! `Requests_and_responses.md`): every ITS-REST resource path is relative to the
//! configured API base path (`cfg.base_path`, default
//! `/ehrbase/rest/openehr/v1`). The generated API surface ([`crate::api`]) is
//! therefore nested under that base path; the `/rest/status` document hangs off
//! the `/ehrbase/rest` root, the
//! always-on health family (`/health`, `/health/liveness`,
//! `/health/readiness`) answers at the process root, and the System Options
//! manifest answers at the base-path root itself.
//!
//! **Layer order** (innermost → outermost, over the nested API): authentication ·
//! ATNA audit (SM System Log, wraps auth so it observes auth rejections) · HTTP
//! metrics · root span · **overload shedding** (bounded in-flight concurrency +
//! load shed; the API subtree's outermost layer, so a shed request is rejected
//! before auth, audit, or reading the request body — [`crate::overload`]). The
//! whole tree is then wrapped in the shared `tower-http` request stack
//! (request-id, tracing, CORS, body limit, timeout, compression), so a shed
//! `503` still carries a request id and is traced. Between the CORS layer and
//! the body-limit/timeout pair sits [`align_transport_error_body`], which
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

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
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

use crate::api::system;
use crate::extensions::access::authn::{self, Authenticator};
use crate::extensions::health;
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
/// layer) nested under the base path, the always-on public health family, the
/// status + SMART-discovery endpoints, the System Options manifest, the optional
/// Swagger UI, and the shared `tower-http` middleware stack.
pub fn router(state: AppState, authenticator: Arc<Authenticator>) -> Router {
    let cfg = state.config().clone();
    let observability = state.observability().clone();
    let rest_root = cfg
        .server
        .base_path
        .strip_suffix("/openehr/v1")
        .unwrap_or(&cfg.server.base_path)
        .to_owned();

    // ── The generated ITS-REST surface, gated by authentication ──────────────
    // A known resource path called with a method it does not serve renders
    // `405` with the openEHR `{ error, message }` body (overview §HTTP
    // Methods), not axum's bare text `405`. The mandatory `Allow` header
    // (RFC 9110 §15.5.6) is added by axum itself from the matched route's
    // registered method set — precisely because
    // `crate::overview::error::method_not_allowed_handler` does not set one of
    // its own (axum only fills `Allow` in when the fallback response leaves it
    // absent). A `405` raised from a *matched* handler skips that machinery
    // entirely and states its own `Allow`
    // (`crate::overview::error::method_not_allowed_response`, used by the
    // config-gated admin group).
    //
    // NOTE (settled deviation, registered as `AMB-60` in
    // `tools/cnf-runner/artifacts/registers/ambiguities.yaml`): the overview's
    // paired SHOULD — "A server receiving an unrecognized or unimplemented
    // method SHOULD respond with the `501 Not Implemented` status code"
    // (`Requests_and_responses.md` §HTTP Methods) — is answered `405` here as
    // well. axum routes by path+method and exposes no "unrecognized method"
    // seam, so the only way to reach a blanket `501` would be a catch-all
    // fallback, which would also swallow every genuinely unknown *path* and
    // report it `501` instead of `404`. `405` is a predefined code in the
    // spec's own status table and so cannot conflict with it
    // (`Requests_and_responses.md` §HTTP status codes: "Additional status codes
    // MAY be used as long as they do not conflict with the predefined codes").
    // `501` is still emitted where the spec's other half applies — a
    // recognised but unimplemented *operation* — through
    // `ApiError::NotImplemented`.
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
        crate::system_log::middleware::middleware,
    ));
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

    // Wrap the whole inner tree in the shared request stack and bind the state,
    // yielding a protocol-complete, state-less service.
    //
    // The stack is applied in two halves with [`align_transport_error_body`]
    // between them: `TimeoutLayer` and `RequestBodyLimitLayer` render their own
    // responses without ever reaching [`crate::overview::error`], so the
    // mapping must sit OUTSIDE both to observe and re-render them. Splitting the
    // application is also what makes the mapping expressible — `Router::layer`
    // normalizes the response body back to `axum::body::Body` at each
    // application, so the mapper takes a plain [`Response`] rather than the
    // layer stack's nested body type. The relative order of the layers
    // themselves is unchanged (outermost → innermost: request-id, tracing,
    // catch-panic, CORS, body limit, timeout, compression).
    let inner: Router = inner
        .layer(
            ServiceBuilder::new()
                .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    REQUEST_TIMEOUT,
                ))
                .layer(CompressionLayer::new()),
        )
        .layer(axum::middleware::map_response(align_transport_error_body))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(SetSensitiveRequestHeadersLayer::new(std::iter::once(
                    AUTHORIZATION,
                )))
                .layer(TraceLayer::new_for_http())
                .layer(CatchPanicLayer::custom(handle_panic))
                .layer(cors),
        )
        .with_state(state);

    // ── System Options and Conformance — `OPTIONS`, above the CORS layer ─────
    // The manifest advertises the **live** mounted-group set (System API):
    // the four always-on standardised groups plus `/admin` when its group is
    // enabled. Its identity/conformance fields come from `cfg.server.identity`
    //.
    //
    // NOTE (settled decision, registered as `AMB-158` in
    // `tools/cnf-runner/artifacts/registers/ambiguities.yaml`): the list
    // carries ONLY the standardised ITS-REST groups —
    // the extension families this server also serves (terminology, tenancy,
    // event subscriptions, the FHIR connector, management, …) are deliberately
    // absent. No released sentence says what `endpoints` must contain: the
    // System API chapter is a stub with no field semantics
    // (`docs/specs/openehr/ITS-REST/specifications/docs/system/Description.md`
    // — Purpose, Related Documents and Status only), and the member itself is
    // grounded only by the released OAS (`schemas/others/Options.yaml`:
    // `array of string`, no description, one example listing exactly the
    // released groups) — a docs-text-silent ground that prescribes no
    // particular content. Two reasons
    // for the omission: the operation's stated job is "exposing service
    // capabilities for a conformance manifest", so a non-openEHR path listed
    // there would sit inside a conformance claim; and the extension surface has
    // its own honest declaration layers — the served OpenAPI document (every
    // extension operation flagged our-own-extension) and the published
    // Conformance Statement's "Additional non-openEHR surface" section. The
    // list still tracks what is actually mounted, so it never advertises a
    // group that answers 404.
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

/// Build the standalone management router (separate-port mode). The binary
/// serves this on the management listener when `management.port` is set.
pub fn management_router(state: &AppState, authenticator: Arc<Authenticator>) -> Router {
    management::router(ManagementState::from_observability(
        state.observability().clone(),
        authenticator,
    ))
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
