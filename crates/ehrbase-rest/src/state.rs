//! Shared application state.
//!
//! [`AppState`] is the type the generated ITS-REST server traits are
//! implemented on (see [`crate::api`]). It is cheap to clone (an `Arc` inside)
//! and is threaded through axum as router state. In Stage 1 it carries only
//! configuration; P12 adds the storage pool and service dependencies.

use std::sync::Arc;

use crate::config::RestConfig;

/// Cheaply-cloneable application state.
#[derive(Clone, Debug)]
pub struct AppState {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    config: RestConfig,
}

impl AppState {
    /// Construct state from the loaded configuration.
    #[must_use]
    pub fn new(config: RestConfig) -> Self {
        Self {
            inner: Arc::new(Inner { config }),
        }
    }

    /// The server configuration.
    #[must_use]
    pub fn config(&self) -> &RestConfig {
        &self.inner.config
    }
}
