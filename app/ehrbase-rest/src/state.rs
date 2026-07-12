//! Shared application state.
//!
//! [`AppState`] is the router state every group dispatcher in [`crate::api`]
//! receives. It is generic over the concrete platform service `S: Platform`
//! (no trait objects, no stub backend — the binary monomorphizes it
//! over the DB-backed `EhrbaseService`, the tests over a mock). It is cheap to
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
//! [`ehrbase_sm::services::WebTemplateService`] (W2-K / finding F-13-02).

use std::sync::Arc;

use ehrbase_sm::Platform;

use crate::config::RestConfig;
use crate::extensions::access::authz::AuthzHandle;
use crate::extensions::management::Observability;

/// Cheaply-cloneable application state, generic over the platform service `S`:
/// the configuration, the service the HTTP dispatcher calls into, and the
/// observability bundle.
#[derive(Debug)]
pub struct AppState<S: Platform> {
    inner: Arc<Inner<S>>,
}

// Hand-written so `Clone` does not spuriously require `S: Clone` — the state is
// always shared through the inner `Arc`.
impl<S: Platform> Clone for AppState<S> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Debug)]
struct Inner<S: Platform> {
    config: RestConfig,
    backend: Arc<S>,
    /// The authorization handle (the RBAC gate), when access control is wired in
    /// (the binary supplies it); `None` restores authentication-only behaviour.
    authz: Option<Arc<AuthzHandle>>,
    /// The observability bundle (management config + telemetry handles).
    observability: Observability,
}

impl<S: Platform> AppState<S> {
    /// Construct state with a concrete service (the `ehrbase` application injects
    /// its DB-backed service here).
    #[must_use]
    pub fn with_backend(config: RestConfig, backend: Arc<S>) -> Self {
        Self::with_parts(config, backend, None, Observability::default())
    }

    /// Construct state from all parts, including the authorization handle and the
    /// observability bundle.
    #[must_use]
    pub fn with_parts(
        config: RestConfig,
        backend: Arc<S>,
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
    pub fn config(&self) -> &RestConfig {
        &self.inner.config
    }

    /// The service the group dispatchers call into.
    pub(crate) fn backend(&self) -> &S {
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
