//! Shared application state.
//!
//! [`AppState`] is the router state every group dispatcher in [`crate::api`]
//! receives. It holds the concrete platform service (`EhrbaseService`)
//! (no trait objects, no stub backend — the binary monomorphizes it
//! over the DB-backed `EhrbaseService`). It is cheap to
//! clone (an `Arc` inside) and carries the configuration, the service `S` the
//! dispatchers call into, the optional authorization handle, and the
//! [`Observability`] bundle (management surface + telemetry handles + health
//! registry) — which defaults to fully off, so a server without observability
//! is the clean default. ATNA auditing is no longer state-held: it lives in the
//! platform service `S` (the SM `SystemLog` component), reached through
//! [`AppState::backend`].
//!
//! The REST layer holds **no caches of its own** — in particular, `WebTemplate`
//! resolution is a single service-owned concern reached through
//! [`ehrbase::service::WebTemplateService`].

use std::sync::Arc;

use ehrbase::service::EhrbaseService;

use crate::config::AppConfig;
use crate::extensions::access::authz::AuthzHandle;
use crate::extensions::management::Observability;

/// Cheaply-cloneable application state, generic over the platform service `S`:
/// the configuration, the service the HTTP dispatcher calls into, and the
/// observability bundle.
#[derive(Debug)]
pub struct AppState {
    inner: Arc<Inner>,
}

// Hand-written so `Clone` does not spuriously require `S: Clone` — the state is
// always shared through the inner `Arc`.
impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Debug)]
struct Inner {
    config: AppConfig,
    backend: Arc<EhrbaseService>,
    /// The authorization handle (the RBAC gate), when access control is wired in
    /// (the binary supplies it); `None` restores authentication-only behaviour.
    authz: Option<Arc<AuthzHandle>>,
    /// The observability bundle (management config + telemetry handles).
    observability: Observability,
}

impl AppState {
    /// Construct state with a concrete service (the `ehrbase` application injects
    /// its DB-backed service here).
    #[must_use]
    pub fn with_backend(config: AppConfig, backend: Arc<EhrbaseService>) -> Self {
        Self::with_parts(config, backend, None, Observability::default())
    }

    /// Construct state from all parts, including the authorization handle and the
    /// observability bundle.
    #[must_use]
    pub fn with_parts(
        config: AppConfig,
        backend: Arc<EhrbaseService>,
        authz: Option<Arc<AuthzHandle>>,
        observability: Observability,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                config,
                backend,
                authz,
                observability,
            }),
        }
    }

    /// The server configuration.
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }

    /// The service the group dispatchers call into.
    pub(crate) fn backend(&self) -> &EhrbaseService {
        &self.inner.backend
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
