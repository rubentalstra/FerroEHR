//! Shared application state.
//!
//! [`AppState`] is the type the generated ITS-REST server traits are
//! implemented on (see [`crate::api`]). It is cheap to clone (an `Arc` inside)
//! and is threaded through axum as router state. It carries the configuration
//! and the service [`Backend`](crate::Backend) the dispatcher calls into (the
//! DB-backed service is injected by the `ehrbase` crate; default `StubBackend`).
//!
//! The REST layer holds **no caches of its own** — in particular, `WebTemplate`
//! resolution is a single service-owned concern reached through
//! [`crate::backend::WebTemplateService`] (W2-K / finding F-13-02).

use std::sync::Arc;

use crate::backend::{Backend, StubBackend};
use crate::config::RestConfig;

/// Cheaply-cloneable application state: the configuration plus the service
/// backend the HTTP dispatcher calls into.
#[derive(Clone, Debug)]
pub struct AppState {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    config: RestConfig,
    backend: Arc<dyn Backend>,
}

impl AppState {
    /// Construct state with the default [`StubBackend`] (every operation →
    /// `NotImplemented`); the server still boots, routes, and authenticates.
    #[must_use]
    pub fn new(config: RestConfig) -> Self {
        Self::with_backend(config, Arc::new(StubBackend))
    }

    /// Construct state with a concrete service backend (the `ehrbase`
    /// application injects its DB-backed service here).
    #[must_use]
    pub fn with_backend(config: RestConfig, backend: Arc<dyn Backend>) -> Self {
        Self {
            inner: Arc::new(Inner { config, backend }),
        }
    }

    /// The server configuration.
    #[must_use]
    pub fn config(&self) -> &RestConfig {
        &self.inner.config
    }

    /// The service backend the HTTP dispatcher calls into.
    pub(crate) fn backend(&self) -> &dyn Backend {
        &*self.inner.backend
    }
}
