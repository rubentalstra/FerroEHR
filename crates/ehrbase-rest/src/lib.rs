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
//! middleware over the API router ([`auth`]). Fine-grained RBAC is Stage 2.

pub mod auth;
pub mod backend;
pub mod config;
mod dispatch;
mod error;
mod negotiate;
mod openapi;
mod params;
pub mod response;
mod router;
mod state;
mod status;

pub use auth::{AuthMethod, Authenticator, Principal};
pub use backend::{Backend, EhrService, StubBackend};
pub use config::RestConfig;
pub use error::RestError;
pub use response::{ResourceMeta, ServiceResponse};
pub use router::router;
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
    let authenticator = Authenticator::new(config.auth.clone()).map_err(ServeError::Auth)?;
    let state = AppState::with_backend(config, backend);
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
    use tower::Layer;
    use tower_http::normalize_path::NormalizePathLayer;

    let bind = config.bind.clone();
    let app = build_with(config, backend)?;
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "ehrbase-rest listening");
    axum::serve(
        listener,
        axum::ServiceExt::<axum::extract::Request>::into_make_service(app),
    )
    .await?;
    Ok(())
}
