// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Console authentication: the login/logout/session server functions.
//!
//! Server functions are a public HTTP API — each one enforces auth itself
//! (Leptos book `server/25`); the CDR credential never leaves the server.

use leptos::server;
use serde::{Deserialize, Serialize};

use crate::error::AdminUiError;

/// What the UI may know about the current session (never the credential).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Display identity.
    pub identity: String,
    /// `"basic"` or `"oidc"`.
    pub method: String,
    /// OIDC token scopes (empty for Basic).
    pub scopes: Vec<String>,
}

/// Basic login: validate the credentials against the CDR (an authenticated
/// ITS-REST call), then store them in the server-side session.
///
/// # Errors
/// [`AdminUiError::Invalid`] on wrong credentials, [`AdminUiError::Cdr`] /
/// [`AdminUiError::CdrUnreachable`] when the CDR misbehaves.
#[server]
pub async fn login_basic(
    /// The CDR username the console authenticates as.
    username: String,
    /// The matching CDR password; it never leaves the server.
    password: String,
    /// Where to land after a successful login; the dashboard when absent.
    next: Option<String>,
) -> Result<(), AdminUiError> {
    let state: crate::state::AppState = leptos::prelude::expect_context();
    if !state.config.auth.basic_enabled {
        return Err(AdminUiError::Invalid("Basic login is disabled".to_owned()));
    }
    let credential = crate::session::Credential::Basic {
        username: username.clone(),
        password,
    };
    // An authenticated, cheap ITS-REST read: the ADL 1.4 template list
    // (Definition API). 401 = wrong credentials; with CDR auth disabled the
    // probe trivially passes, which is the dev posture.
    let url = state.cdr.rest_v1("definition/template/adl1.4");
    let probe = state.cdr.get(&credential, &url, "application/json").await?;
    if probe.is(http::StatusCode::UNAUTHORIZED) {
        return Err(AdminUiError::Invalid(
            "wrong username or password".to_owned(),
        ));
    }
    if probe.status >= http::StatusCode::INTERNAL_SERVER_ERROR.as_u16() {
        return Err(AdminUiError::Cdr {
            status: probe.status,
            message: "CDR error while validating credentials".to_owned(),
        });
    }

    let session = crate::session::http_session().await?;
    session
        .insert(
            crate::session::SESSION_KEY,
            crate::session::AdminSession {
                identity: username,
                credential,
                scopes: Vec::new(),
            },
        )
        .await
        .map_err(|e| AdminUiError::Internal(format!("session store: {e}")))?;

    leptos_axum::redirect(next.as_deref().unwrap_or("/"));
    Ok(())
}

/// Destroy the session and land on `/login`.
///
/// NOTE: deliberately not guarded by `require_session` — flushing the
/// caller's own cookie-scoped session is idempotent and leaks nothing; an
/// unauthenticated call is a harmless no-op redirect.
///
/// # Errors
/// [`AdminUiError::Internal`] on a session-store failure.
#[server]
pub async fn logout() -> Result<(), AdminUiError> {
    let session = crate::session::http_session().await?;
    session
        .flush()
        .await
        .map_err(|e| AdminUiError::Internal(format!("session store: {e}")))?;
    leptos_axum::redirect("/login");
    Ok(())
}

/// The current session, if any — drives the shell's user menu and the
/// scope panel. Deliberately NOT guarded: "no session" is a valid answer.
///
/// # Errors
/// [`AdminUiError::Internal`] on a session-store failure.
#[server]
pub async fn current_session() -> Result<Option<SessionInfo>, AdminUiError> {
    let session = crate::session::http_session().await?;
    let admin = session
        .get::<crate::session::AdminSession>(crate::session::SESSION_KEY)
        .await
        .map_err(|e| AdminUiError::Internal(format!("session store: {e}")))?;
    Ok(admin.map(|s| SessionInfo {
        identity: s.identity,
        method: match s.credential {
            crate::session::Credential::Basic { .. } => "basic".to_owned(),
            crate::session::Credential::Bearer { .. } => "oidc".to_owned(),
        },
        scopes: s.scopes,
    }))
}

/// Which login modes the login screen should offer.
///
/// # Errors
/// None in practice; the signature is fallible per the server-fn contract.
#[server]
pub async fn login_modes() -> Result<(bool, bool), AdminUiError> {
    let state: crate::state::AppState = leptos::prelude::expect_context();
    // The console offers only what BOTH sides support: its own configured
    // modes intersected with the schemes the CDR advertises in its
    // `WWW-Authenticate` challenge (a Basic form against a bearer-only CDR
    // can never succeed). An unreachable CDR falls back to the console's
    // config alone so the login page still renders — the login attempt
    // itself then surfaces the outage.
    let (cdr_basic, cdr_bearer) = state.cdr.advertised_schemes().await.unwrap_or((true, true));
    Ok((
        state.config.auth.basic_enabled && cdr_basic,
        state.config.auth.oidc.enabled && cdr_bearer,
    ))
}

/// The CDR `/ferroehr/rest/status` document (public endpoint), raw JSON — the
/// shell's health pill and the system panel both read it.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session (the shell only
/// polls when logged in); CDR transport errors pass through.
#[server]
pub async fn fetch_status() -> Result<String, AdminUiError> {
    crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.origin_url("ferroehr/rest/status");
    let response = state.cdr.get_public(&url, "application/json").await?;
    Ok(crate::cdr::CdrClient::expect_success(response)?.body)
}
