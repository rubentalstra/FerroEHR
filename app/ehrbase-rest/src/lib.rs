//! openEHR **ITS-REST 1.0.3** server surface (`axum`) + authentication.
//!
//! `ehrbase-rest` implements the openEHR SM native API (`ehrbase-sm`) as a
//! modern, idiomatic axum application. The generated `ROUTES`
//! tables drive an HTTP dispatcher ([`dispatch`]) that rebuilds each operation's
//! `*Params`, negotiates content (canonical JSON / XML via `openehr-its`), and
//! calls the configured platform service `S: Platform`. The adapter is generic
//! over `S` (no trait objects, no stub backend by design) — the `ehrbase` crate
//! monomorphizes it over its DB-backed `EhrbaseService` via
//! [`AppState::with_backend`], and the tests over a mock.
//!
//! Authentication (Stage 1) is HTTP Basic + OAuth2/OIDC bearer, applied as one
//! middleware over the API router ([`auth`]); the same middleware runs the
//! coarse **RBAC** gate ([`authz`]) when an [`AuthzHandle`] is wired.
//! Fine-grained ABAC is the follow-up (`docs/enterprise/access-control.md`).

pub mod access;
mod audit;
mod audit_table;
pub mod config;
mod dispatch;
pub mod management;
mod openapi;
pub mod overview;
mod router;
mod state;

use overview::{committal, error, negotiate, params, status, version_id};

pub use access::authn::{AuthMethod, Authenticator, Principal};
pub use access::authz::{AuthzHandle, AuthzResolvers, ResolveError, build_engine};
// The native API lives in `ehrbase-sm`; re-exported here for the
// server's public surface (test mocks, the binary) — no local shim module.
pub use config::{
    AdminConfig, EventSubscriptionConfig, FhirConfig, RestConfig, TenancyConfig, TerminologyConfig,
};
pub use ehrbase_sm::Platform;
pub use ehrbase_sm::{
    AdminArchive, AdminService, DefinitionAdl2Service, DefinitionAdl14Service,
    DefinitionQueryService, DemographicService, EhrCompositionService, EhrContributionService,
    EhrDirectoryService, EhrIndexService, EhrService, EhrStatusService, ItemTagAdapter,
    PartyRelationshipService, QueryService, StatTimeRange, SystemLog, TerminologyService,
    ValidityChecker, VersionMetaAdapter, WebTemplateService,
};
pub use ehrbase_sm::{
    AqlQueryRequest, EhrIndexEntry, EhrSummary, LocationDesc, Page, PartyKind, PlatformService,
    QueryDescriptor, QueryOutcome, ResourceInstanceType, ResourceMeta, ResourceStatus,
    ServiceResponse, SubjectRef,
};
pub use error::RestError;
pub use management::{ManagementConfig, Observability};
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

/// Build the application router backed by a concrete service (the `ehrbase`
/// application injects its DB-backed service here).
///
/// # Errors
/// [`ServeError::Auth`] if the OIDC key material/algorithms are invalid.
pub fn build_with<S: Platform>(
    config: RestConfig,
    backend: std::sync::Arc<S>,
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
pub async fn serve_with<S: Platform>(
    config: RestConfig,
    backend: std::sync::Arc<S>,
) -> Result<(), ServeError> {
    let bind = config.bind.clone();
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
pub fn build_full<S: Platform>(
    config: RestConfig,
    backend: std::sync::Arc<S>,
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
    config: &RestConfig,
    authz: Option<&AuthzHandle>,
) -> Result<std::sync::Arc<Authenticator>, ServeError> {
    let role_claims =
        authz.map_or_else(access::authz::default_role_claims, AuthzHandle::role_claims);
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
pub async fn serve_full<S: Platform>(
    config: RestConfig,
    backend: std::sync::Arc<S>,
    authz: Option<std::sync::Arc<AuthzHandle>>,
    observability: Observability,
) -> Result<(), ServeError> {
    let authenticator = build_authenticator(&config, authz.as_deref())?;
    let bind = config.bind.clone();
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
