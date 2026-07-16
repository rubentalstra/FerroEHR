//! openEHR **ITS-REST 1.0.3** server surface (`axum`) — the protocol adapter
//! over the SM native API (`ehrbase-sm`).
//!
//! The crate is organised **per ITS-REST specification** (the development-edition
//! register, `docs/design/its-rest/README.md`), one folder per spec area:
//!
//! - [`overview`] — the cross-cutting Overview protocol (content negotiation,
//!   committal headers, resource identification, common params, the HTTP
//!   status/error table); [`RestError`] renders the openEHR error body.
//! - [`api`] — the resource APIs, one module per group (`ehr`, `query`,
//!   `definition`, `demographic`, `admin`) implementing the generated
//!   `openehr_its::rest` contract, plus the hand-written `system` API
//!   (`OPTIONS /` conformance manifest). [`api::api_router`] is the hub over the
//!   generated `ROUTES` tables.
//! - [`formats`] — the Simplified Formats wire (FLAT / STRUCTURED media types).
//! - [`smart`] — SMART App Launch (service discovery + scope enforcement),
//!   config-gated, off by default.
//! - [`system_log`] — the SM System Log component at the wire (the IHE ATNA
//!   audit middleware + operation classification).
//! - [`extensions`] — everything the specs do **not** govern, quarantined and
//!   flagged: authentication + authorization ([`extensions::access`]),
//!   management/observability, `OpenAPI` serving, terminology, eventing, FHIR, and
//!   multi-tenancy — each config-gated so a stock server exposes only the
//!   standardised ITS-REST surface.
//!
//! [`router`] assembles these under the configured base path with the
//! `tower-http` middleware stack. The adapter is generic over the platform
//! concrete `EhrbaseService` (W-14 B+C: no trait seam, no stub backend) — the
//! `ehrbase` crate monomorphizes it over its DB-backed `EhrbaseService` via
//! [`AppState::with_backend`], and the tests over a mock.
//!
//! **Authentication** (Stage 1) is HTTP Basic + OAuth2/OIDC bearer, applied as
//! one middleware over the API router; the coarse RBAC gate + fine-grained ABAC
//! PEP compose on top when wired ([`extensions::access`]). Auth is out of band
//! per the spec (`overview/Requests_and_responses.md` §Authentication).

pub mod api;
pub mod config;
pub mod extensions;
pub mod formats;
mod overload;
pub mod overview;
pub mod router;
pub mod smart;
pub mod state;
pub mod system_log;

// `access` (authn + authz config the binary wires) and `management`
// (observability the binary assembles) are part of the crate's public surface;
// the binary + tests reach every item at its defining module —
// `ehrbase_rest::extensions::access::…` / `ehrbase_rest::extensions::management::…`
// (no re-exports). The two shared protocol helpers (`negotiate`, `params`)
// live under `overview` and are imported here for the dispatcher glue below.
use ehrbase::service::EhrbaseService;

use crate::config::AppConfig;
use crate::extensions::access::authn::Authenticator;
use crate::extensions::access::authz::AuthzHandle;
use crate::extensions::access::authz::roles::default_role_claims;
use crate::extensions::management::Observability;
use crate::router::{management_router, router};
use crate::state::AppState;
use overview::{negotiate, params};

/// Errors raised while starting the server.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The authentication configuration was invalid.
    #[error("authentication configuration error: {0}")]
    Auth(String),
    /// Binding the listener or serving failed.
    #[error("server I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Build the application router backed by a concrete service (the `ehrbase`
/// application injects its DB-backed service here).
///
/// # Errors
/// [`ServeError::Auth`] if the OIDC key material/algorithms are invalid.
pub fn build_with(
    config: AppConfig,
    backend: std::sync::Arc<EhrbaseService>,
) -> Result<axum::Router, ServeError> {
    let authenticator = Authenticator::new(config.auth.clone()).map_err(ServeError::Auth)?;
    let state = AppState::with_backend(config, backend);
    Ok(router(state, authenticator))
}

/// Build and serve the application backed by a concrete service, with graceful
/// shutdown on `SIGINT`/`SIGTERM`. Client peer addresses are captured via
/// `ConnectInfo`.
///
/// ATNA auditing (when enabled) is emitted through the platform service's SM
/// `SystemLog` component; the binary boots the sender, injects it into the
/// service, and drains its `AuditHandle` on shutdown.
///
/// # Errors
/// [`ServeError::Auth`] on bad auth config; [`ServeError::Io`] on bind/serve failure.
pub async fn serve_with(
    config: AppConfig,
    backend: std::sync::Arc<EhrbaseService>,
) -> Result<(), ServeError> {
    let bind = config.server.bind.clone();
    let app = build_with(config, backend)?;
    run_server(app, &bind).await
}

/// Build the application router with a concrete backend and a full
/// [`Observability`] bundle (management surface + telemetry handles). The
/// management surface is merged into the returned router when it is enabled and
/// not bound to a separate port. ATNA auditing lives in the backend's SM
/// `SystemLog` component.
///
/// # Errors
/// [`ServeError::Auth`] if the OIDC key material/algorithms are invalid.
pub fn build_full(
    config: AppConfig,
    backend: std::sync::Arc<EhrbaseService>,
    authz: Option<std::sync::Arc<AuthzHandle>>,
    observability: Observability,
) -> Result<axum::Router, ServeError> {
    let authenticator = build_authenticator(&config, authz.as_deref())?;
    let state = AppState::with_parts(config, backend, authz, observability);
    Ok(router(state, authenticator))
}

/// Build the [`Authenticator`], threading the RBAC role-claim paths from the
/// authorization handle (default paths when none is wired) so Bearer role
/// extraction matches the gate's configuration (§5.1).
fn build_authenticator(
    config: &AppConfig,
    authz: Option<&AuthzHandle>,
) -> Result<std::sync::Arc<Authenticator>, ServeError> {
    let role_claims =
        authz.map_or_else(default_role_claims, AuthzHandle::role_claims);
    Authenticator::with_role_claims(config.auth.clone(), role_claims).map_err(ServeError::Auth)
}

/// Build and serve the application with full observability: the API + audit +
/// telemetry surface, and — when `management.port` is set — the management
/// surface on its own internal listener (otherwise merged into the main app).
/// Graceful shutdown on `SIGINT`/`SIGTERM` covers both listeners. ATNA auditing
/// lives in the backend's SM `SystemLog` component.
///
/// # Errors
/// [`ServeError::Auth`] on bad auth config; [`ServeError::Io`] on bind/serve failure.
pub async fn serve_full(
    config: AppConfig,
    backend: std::sync::Arc<EhrbaseService>,
    authz: Option<std::sync::Arc<AuthzHandle>>,
    observability: Observability,
) -> Result<(), ServeError> {
    let authenticator = build_authenticator(&config, authz.as_deref())?;
    let bind = config.server.bind.clone();
    let management_enabled = observability.management.enabled;
    let management_port = observability.management.port;
    let state = AppState::with_parts(config, backend, authz, observability);
    let main_app = router(state.clone(), authenticator.clone());

    // Separate-port management listener (§2): its own axum server task.
    let management_task = if management_enabled && let Some(port) = management_port {
        let management_app = management_router(&state, authenticator);
        let management_bind = format!("0.0.0.0:{port}");
        tracing::info!(bind = %management_bind, "ehrbase-rest management listening (separate port)");
        Some(tokio::spawn(async move {
            if let Err(e) = run_server(management_app, &management_bind).await {
                tracing::error!("management listener stopped: {e}");
            }
        }))
    } else {
        None
    };

    let result = run_server(main_app, &bind).await;
    if let Some(task) = management_task {
        task.abort();
    }
    result
}

/// Serve one router: wrap it in the path-normalization layer, bind, and serve
/// with graceful shutdown and per-connection peer info.
async fn run_server(app: axum::Router, bind: &str) -> Result<(), ServeError> {
    use std::net::SocketAddr;

    use axum::serve::ListenerExt as _;
    use tower::Layer;
    use tower_http::normalize_path::NormalizePathLayer;

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "ehrbase-rest listening");
    let make = axum::ServiceExt::<axum::extract::Request>::into_make_service_with_connect_info::<
        SocketAddr,
    >(app);
    // `TCP_NODELAY` on every accepted socket: small responses (the
    // `204`/minimal write acknowledgements the API is full of) must not sit
    // in Nagle's buffer waiting for an ACK — worth tens of milliseconds of
    // tail latency per small response on some stacks. A failed setsockopt is
    // not fatal — the connection is served regardless.
    let listener = listener.tap_io(|io: &mut tokio::net::TcpStream| {
        let _ = io.set_nodelay(true);
    });
    axum::serve(listener, make)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Resolve when the process receives `SIGINT` (Ctrl-C) or, on Unix, `SIGTERM`.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => tracing::warn!("failed to install SIGTERM handler: {e}"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
