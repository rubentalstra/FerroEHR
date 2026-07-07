//! Shared application state.
//!
//! [`AppState`] is the type the generated ITS-REST server traits are
//! implemented on (see [`crate::api`]). It is cheap to clone (an `Arc` inside)
//! and is threaded through axum as router state. It carries the configuration,
//! the service [`Backend`](crate::Backend) the dispatcher calls into, the
//! optional ATNA audit sender, and the [`Observability`] bundle (management
//! surface + telemetry handles + health registry) — which defaults to fully
//! off, so a server without observability is the clean default.
//!
//! The REST layer holds **no caches of its own** — in particular, `WebTemplate`
//! resolution is a single service-owned concern reached through
//! [`crate::backend::WebTemplateService`] (W2-K / finding F-13-02).

use std::sync::Arc;

use ehrbase_audit::AuditSender;

use crate::authz::AuthzHandle;
use crate::backend::{Backend, StubBackend};
use crate::config::RestConfig;
use crate::management::Observability;

/// Cheaply-cloneable application state: the configuration, the service backend
/// the HTTP dispatcher calls into, the optional audit sender, and the
/// observability bundle.
#[derive(Clone, Debug)]
pub struct AppState {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    config: RestConfig,
    backend: Arc<dyn Backend>,
    /// The ATNA audit sender, when auditing is wired in (the binary supplies it).
    audit: Option<AuditSender>,
    /// The authorization handle (the RBAC gate), when access control is wired in
    /// (the binary supplies it); `None` restores authentication-only behaviour.
    authz: Option<Arc<AuthzHandle>>,
    /// The observability bundle (management config + telemetry handles).
    observability: Observability,
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
        Self::with_backend_and_audit(config, backend, None)
    }

    /// Construct state with a concrete backend and an optional ATNA audit sender
    /// (observability off, no authorization handle).
    #[must_use]
    pub fn with_backend_and_audit(
        config: RestConfig,
        backend: Arc<dyn Backend>,
        audit: Option<AuditSender>,
    ) -> Self {
        Self::with_parts(config, backend, audit, None, Observability::default())
    }

    /// Construct state from all parts, including the authorization handle and the
    /// observability bundle.
    #[must_use]
    pub fn with_parts(
        config: RestConfig,
        backend: Arc<dyn Backend>,
        audit: Option<AuditSender>,
        authz: Option<Arc<AuthzHandle>>,
        observability: Observability,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                config,
                backend,
                audit,
                authz,
                observability,
            }),
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

    /// The ATNA audit sender, if auditing is enabled/wired.
    pub(crate) fn audit(&self) -> Option<AuditSender> {
        self.inner.audit.clone()
    }

    /// The authorization handle (RBAC gate), if access control is wired.
    pub(crate) fn authz(&self) -> Option<Arc<AuthzHandle>> {
        self.inner.authz.clone()
    }

    /// The observability bundle (management + telemetry handles).
    pub(crate) fn observability(&self) -> &Observability {
        &self.inner.observability
    }
}
