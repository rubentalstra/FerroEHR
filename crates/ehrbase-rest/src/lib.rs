//! openEHR **ITS-REST 1.0.3** server surface (`axum`) + authentication.
//!
//! `ehrbase-rest` implements the generated ITS-REST server traits from
//! [`openehr_its::rest::generated`] as a modern, idiomatic axum application
//! (ADR-005/006). The generated `ROUTES` tables drive an HTTP dispatcher
//! ([`dispatch`]) that rebuilds each operation's `*Params`, negotiates content
//! (canonical JSON / XML via `openehr-its`), and calls the configured service
//! [`Backend`] (dependency inversion — the DB-backed service lives in the
//! `ehrbase` crate and is injected via [`AppState::with_backend`]). Operations a
//! backend has not implemented return `ApiError::NotImplemented` (the generated
//! traits' default); the default [`StubBackend`] implements none.
//!
//! Authentication (Stage 1) is HTTP Basic + OAuth2/OIDC bearer, applied as one
//! middleware over the API router ([`auth`]); the same middleware runs the
//! coarse **RBAC** gate ([`authz`]) when an [`AuthzHandle`] is wired.
//! Fine-grained ABAC is the follow-up (`docs/enterprise/access-control.md`).

mod audit;
pub mod auth;
pub mod authz;
pub mod backend;
pub mod config;
mod dispatch;
mod error;
pub mod management;
mod negotiate;
mod openapi;
mod params;
pub mod response;
mod router;
mod state;
mod status;

pub use auth::{AuthMethod, Authenticator, Principal};
pub use authz::{AuthzHandle, AuthzResolvers, ResolveError, build_engine};
pub use backend::{
    AqlQueryRequest, Backend, EhrService, QueryService, StubBackend, WebTemplateService,
};
pub use config::RestConfig;
pub use error::RestError;
pub use management::{ManagementConfig, Observability};
pub use response::{ResourceMeta, ServiceResponse};
pub use router::{management_router, router};
pub use state::AppState;

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

/// Build the application router with the default [`StubBackend`] (every
/// operation → `NotImplemented`).
///
/// # Errors
/// [`ServeError::Auth`] if the OIDC key material/algorithms are invalid.
pub fn build(config: RestConfig) -> Result<axum::Router, ServeError> {
    build_with(config, std::sync::Arc::new(StubBackend))
}

/// Build the application router backed by a concrete service (the `ehrbase`
/// application injects its DB-backed service here).
///
/// # Errors
/// [`ServeError::Auth`] if the OIDC key material/algorithms are invalid.
pub fn build_with(
    config: RestConfig,
    backend: std::sync::Arc<dyn Backend>,
) -> Result<axum::Router, ServeError> {
    build_with_audit(config, backend, None)
}

/// Build the application router backed by a concrete service and, optionally, an
/// ATNA audit sender (the `ehrbase` binary boots the sender and injects it here).
///
/// # Errors
/// [`ServeError::Auth`] if the OIDC key material/algorithms are invalid.
pub fn build_with_audit(
    config: RestConfig,
    backend: std::sync::Arc<dyn Backend>,
    audit: Option<ehrbase_audit::AuditSender>,
) -> Result<axum::Router, ServeError> {
    let authenticator = Authenticator::new(config.auth.clone()).map_err(ServeError::Auth)?;
    let state = AppState::with_backend_and_audit(config, backend, audit);
    Ok(router(state, authenticator))
}

/// Build and serve the application, binding the configured address. Applies the
/// path-normalization layer (which must wrap the router) at the outer edge.
///
/// # Errors
/// [`ServeError::Auth`] on bad auth config; [`ServeError::Io`] on bind/serve failure.
pub async fn serve(config: RestConfig) -> Result<(), ServeError> {
    serve_with(config, std::sync::Arc::new(StubBackend)).await
}

/// Build and serve the application backed by a concrete service.
///
/// # Errors
/// [`ServeError::Auth`] on bad auth config; [`ServeError::Io`] on bind/serve failure.
pub async fn serve_with(
    config: RestConfig,
    backend: std::sync::Arc<dyn Backend>,
) -> Result<(), ServeError> {
    serve_with_audit(config, backend, None).await
}

/// Build and serve the application backed by a concrete service and an optional
/// ATNA audit sender, with graceful shutdown on `SIGINT`/`SIGTERM`.
///
/// The audit sender lives in the router state; when the graceful shutdown
/// completes and this future returns, the router (and its sender clone) is
/// dropped — the caller then drains the [`ehrbase_audit::AuditHandle`] to flush
/// buffered records. Client peer addresses are captured via `ConnectInfo`.
///
/// # Errors
/// [`ServeError::Auth`] on bad auth config; [`ServeError::Io`] on bind/serve failure.
pub async fn serve_with_audit(
    config: RestConfig,
    backend: std::sync::Arc<dyn Backend>,
    audit: Option<ehrbase_audit::AuditSender>,
) -> Result<(), ServeError> {
    let bind = config.bind.clone();
    let app = build_with_audit(config, backend, audit)?;
    run_server(app, &bind).await
}

/// Build the application router with a concrete backend, an optional ATNA audit
/// sender, and a full [`Observability`] bundle (management surface + telemetry
/// handles). The management surface is merged into the returned router when it
/// is enabled and not bound to a separate port.
///
/// # Errors
/// [`ServeError::Auth`] if the OIDC key material/algorithms are invalid.
pub fn build_full(
    config: RestConfig,
    backend: std::sync::Arc<dyn Backend>,
    audit: Option<ehrbase_audit::AuditSender>,
    authz: Option<std::sync::Arc<AuthzHandle>>,
    observability: Observability,
) -> Result<axum::Router, ServeError> {
    let authenticator = build_authenticator(&config, authz.as_deref())?;
    let state = AppState::with_parts(config, backend, audit, authz, observability);
    Ok(router(state, authenticator))
}

/// Build the [`Authenticator`], threading the RBAC role-claim paths from the
/// authorization handle (default paths when none is wired) so Bearer role
/// extraction matches the gate's configuration (§5.1).
fn build_authenticator(
    config: &RestConfig,
    authz: Option<&AuthzHandle>,
) -> Result<std::sync::Arc<Authenticator>, ServeError> {
    let role_claims = authz.map_or_else(authz::default_role_claims, AuthzHandle::role_claims);
    Authenticator::with_role_claims(config.auth.clone(), role_claims).map_err(ServeError::Auth)
}

/// Build and serve the application with full observability: the API + audit +
/// telemetry surface, and — when `management.port` is set — the management
/// surface on its own internal listener (otherwise merged into the main app).
/// Graceful shutdown on `SIGINT`/`SIGTERM` covers both listeners.
///
/// # Errors
/// [`ServeError::Auth`] on bad auth config; [`ServeError::Io`] on bind/serve failure.
pub async fn serve_full(
    config: RestConfig,
    backend: std::sync::Arc<dyn Backend>,
    audit: Option<ehrbase_audit::AuditSender>,
    authz: Option<std::sync::Arc<AuthzHandle>>,
    observability: Observability,
) -> Result<(), ServeError> {
    let authenticator = build_authenticator(&config, authz.as_deref())?;
    let bind = config.bind.clone();
    let management_enabled = observability.management.enabled;
    let management_port = observability.management.port;
    let state = AppState::with_parts(config, backend, audit, authz, observability);
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

    use tower::Layer;
    use tower_http::normalize_path::NormalizePathLayer;

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "ehrbase-rest listening");
    let make = axum::ServiceExt::<axum::extract::Request>::into_make_service_with_connect_info::<
        SocketAddr,
    >(app);
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
