// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Server-side session state and the auth guard every `#[server]` fn calls
//! (the Leptos book's `server/25` security rule.
//!
//! Server functions are a public HTTP API — "only my UI calls this" is never
//! assumed). CDR credentials live ONLY here, server-side; they never reach a
//! signal, prop, or serialized resource.

use serde::{Deserialize, Serialize};

use crate::error::AdminUiError;

/// The session-store key for the one [`AdminSession`] record.
pub const SESSION_KEY: &str = "admin_session";

/// The credential the BFF attaches to outbound CDR calls.
///
/// `Debug` is manual and redacting: secrets must never appear in logs
/// (reliability rule — log identifiers and shapes, not bodies).
#[derive(Clone, Serialize, Deserialize)]
pub enum Credential {
    /// Basic auth, validated against the CDR at login.
    Basic {
        /// The CDR username.
        username: String,
        /// The CDR password (server-side only).
        password: String,
    },
    /// A bearer token from the console's OIDC login.
    Bearer {
        /// The access token (server-side only).
        access_token: String,
    },
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic { username, .. } => f
                .debug_struct("Basic")
                .field("username", username)
                .field("password", &"<redacted>")
                .finish(),
            Self::Bearer { .. } => f
                .debug_struct("Bearer")
                .field("access_token", &"<redacted>")
                .finish(),
        }
    }
}

/// One authenticated console session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSession {
    /// Display identity (username or OIDC `preferred_username`/`sub`).
    pub identity: String,
    /// How the session authenticates to the CDR.
    pub credential: Credential,
    /// Scopes granted to the OIDC token (empty for Basic) — the "what can
    /// I do right now" panel reads these.
    pub scopes: Vec<String>,
}

/// Extract the current `tower_sessions` session from the request context.
///
/// # Errors
/// [`AdminUiError::Internal`] when called outside a request (a bug).
pub async fn http_session() -> Result<tower_sessions::Session, AdminUiError> {
    leptos_axum::extract::<tower_sessions::Session>()
        .await
        .map_err(|e| AdminUiError::Internal(format!("session extraction: {e}")))
}

/// The guard: return the authenticated session or `Unauthenticated`.
///
/// Every `#[server]` fn that touches the CDR or session state calls this
/// first.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a live session;
/// [`AdminUiError::Internal`] on a session-store failure.
pub async fn require_session() -> Result<AdminSession, AdminUiError> {
    let session = http_session().await?;
    session
        .get::<AdminSession>(SESSION_KEY)
        .await
        .map_err(|e| AdminUiError::Internal(format!("session store: {e}")))?
        .ok_or(AdminUiError::Unauthenticated)
}
