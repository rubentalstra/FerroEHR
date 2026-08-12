// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Shared server state, provided to every server function via Leptos
//! context (`leptos_routes_with_context`) and to the plain axum handlers
//! via `Extension`.

use std::sync::Arc;

/// Everything the BFF side needs per request.
#[derive(Debug, Clone)]
pub struct AppState {
    /// The loaded console configuration.
    pub config: Arc<crate::config::AdminUiConfig>,
    /// The one CDR client.
    pub cdr: crate::cdr::CdrClient,
    /// OIDC discovery result (present when `auth.oidc.enabled`).
    pub oidc: Option<Arc<crate::oidc::OidcState>>,
}
