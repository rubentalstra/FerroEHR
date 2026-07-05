//! openEHR **ITS-REST 1.0.3** server surface (`axum`) + authentication.
//!
//! `ehrbase-rest` implements the generated ITS-REST server traits from
//! [`openehr_its::rest::generated`] as a modern, idiomatic axum application
//! (ADR-005/006). The generated `ROUTES` tables drive an HTTP dispatcher
//! ([`dispatch`]) that rebuilds each operation's `*Params`, negotiates content
//! (canonical JSON / XML via `openehr-its`), and calls the trait method on
//! [`AppState`]. In Stage 1 (P11) the handlers return
//! `ApiError::NotImplemented`; P12 fills them with the service layer.
//!
//! Authentication (Stage 1) is HTTP Basic + OAuth2/OIDC bearer, applied as one
//! middleware over the API router ([`auth`]). Fine-grained RBAC is Stage 2.

mod api;
pub mod auth;
pub mod config;
mod dispatch;
mod error;
mod negotiate;
mod openapi;
mod params;
mod router;
mod state;
mod status;

pub use auth::{AuthMethod, Authenticator, Principal};
pub use config::RestConfig;
pub use error::RestError;
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

/// Build the application router from configuration (constructing the
/// authenticator and state).
///
/// # Errors
/// [`ServeError::Auth`] if the OIDC key material/algorithms are invalid.
pub fn build(config: RestConfig) -> Result<axum::Router, ServeError> {
    let authenticator = Authenticator::new(config.auth.clone()).map_err(ServeError::Auth)?;
    let state = AppState::new(config);
    Ok(router(state, authenticator))
}

/// Build and serve the application, binding the configured address. Applies the
/// path-normalization layer (which must wrap the router) at the outer edge.
///
/// # Errors
/// [`ServeError::Auth`] on bad auth config; [`ServeError::Io`] on bind/serve failure.
pub async fn serve(config: RestConfig) -> Result<(), ServeError> {
    use tower::Layer;
    use tower_http::normalize_path::NormalizePathLayer;

    let bind = config.bind.clone();
    let app = build(config)?;
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
