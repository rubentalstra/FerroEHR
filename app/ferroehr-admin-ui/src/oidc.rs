// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The console's OIDC authorization-code login (with PKCE): two plain axum
//! routes (`/auth/oidc/login`, `/auth/oidc/callback`) on the BFF.
//!
//! The bearer token is stored server-side in the session; the browser only
//! ever holds the console's session cookie.

use axum::response::{IntoResponse, Redirect, Response};
use http::StatusCode;
use openidconnect::{OAuth2TokenResponse, TokenResponse};
use serde::{Deserialize, Serialize};

use crate::config::OidcConfig;

/// Discovery output kept in [`crate::state::AppState`].
#[derive(Debug)]
pub struct OidcState {
    metadata: openidconnect::core::CoreProviderMetadata,
    http: openidconnect::reqwest::Client,
    config: OidcConfig,
}

/// The transient state parked in the session between login and callback.
#[derive(Debug, Serialize, Deserialize)]
struct FlowState {
    pkce_verifier: String,
    csrf: String,
    nonce: String,
}

const FLOW_KEY: &str = "oidc_flow";

/// Run OIDC discovery against the configured issuer.
///
/// # Errors
/// `anyhow::Error` on an unreachable/invalid issuer (boot-time failure —
/// the binary refuses to start with a broken OIDC config).
pub async fn discover(config: &OidcConfig) -> anyhow::Result<OidcState> {
    let mut builder = openidconnect::reqwest::ClientBuilder::new()
        .redirect(openidconnect::reqwest::redirect::Policy::none()); // SSRF hardening
    // Split-horizon issuer resolution (`host=ip:port`): the canonical
    // issuer hostname may only resolve inside a container network; the
    // override points this client at the mapped address while every URL
    // (and the token `iss`) keeps the canonical form.
    if !config.resolve.trim().is_empty() {
        let (host, addr) = config
            .resolve
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("auth.oidc.resolve must be `host=ip:port`"))?;
        builder = builder.resolve(host.trim(), addr.trim().parse()?);
    }
    let http = builder.build()?;
    let issuer = openidconnect::IssuerUrl::new(config.issuer.clone())?;
    let metadata = openidconnect::core::CoreProviderMetadata::discover_async(issuer, &http).await?;
    Ok(OidcState {
        metadata,
        http,
        config: config.clone(),
    })
}

impl OidcState {
    fn client(
        &self,
    ) -> anyhow::Result<
        openidconnect::core::CoreClient<
            openidconnect::EndpointSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointMaybeSet,
            openidconnect::EndpointMaybeSet,
        >,
    > {
        let redirect = openidconnect::RedirectUrl::new(format!(
            "{}/auth/oidc/callback",
            self.config.public_base_url.trim_end_matches('/')
        ))?;
        Ok(openidconnect::core::CoreClient::from_provider_metadata(
            self.metadata.clone(),
            openidconnect::ClientId::new(self.config.client_id.clone()),
            Some(openidconnect::ClientSecret::new(
                self.config.client_secret.clone(),
            )),
        )
        .set_redirect_uri(redirect))
    }
}

fn error_page(status: StatusCode, message: &str) -> Response {
    tracing::warn!(%status, message, "OIDC flow failed");
    (status, format!("OIDC login failed: {message}")).into_response()
}

/// `GET /auth/oidc/login` — park PKCE/CSRF/nonce in the session and
/// redirect to the authorization endpoint.
pub async fn login(
    axum::Extension(state): axum::Extension<crate::state::AppState>,
    session: tower_sessions::Session,
) -> Response {
    let Some(oidc) = state.oidc.as_ref() else {
        return error_page(StatusCode::NOT_FOUND, "OIDC is not enabled");
    };
    let client = match oidc.client() {
        Ok(client) => client,
        Err(e) => return error_page(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let (pkce_challenge, pkce_verifier) = openidconnect::PkceCodeChallenge::new_random_sha256();
    let mut request = client
        .authorize_url(
            openidconnect::core::CoreAuthenticationFlow::AuthorizationCode,
            openidconnect::CsrfToken::new_random,
            openidconnect::Nonce::new_random,
        )
        .set_pkce_challenge(pkce_challenge);
    for scope in &oidc.config.scopes {
        request = request.add_scope(openidconnect::Scope::new(scope.clone()));
    }
    let (auth_url, csrf, nonce) = request.url();

    let flow = FlowState {
        pkce_verifier: pkce_verifier.secret().clone(),
        csrf: csrf.secret().clone(),
        nonce: nonce.secret().clone(),
    };
    if let Err(e) = session.insert(FLOW_KEY, flow).await {
        return error_page(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    Redirect::to(auth_url.as_str()).into_response()
}

/// The callback's query parameters (only the success pair; a provider
/// error arrives as `error`/`error_description`).
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// `GET /auth/oidc/callback` — exchange the code (PKCE), validate the ID
/// token nonce, store the session, land on `/`.
pub async fn callback(
    axum::Extension(state): axum::Extension<crate::state::AppState>,
    session: tower_sessions::Session,
    axum::extract::Query(params): axum::extract::Query<CallbackParams>,
) -> Response {
    let Some(oidc) = state.oidc.as_ref() else {
        return error_page(StatusCode::NOT_FOUND, "OIDC is not enabled");
    };
    if let Some(error) = params.error {
        let detail = params.error_description.unwrap_or_default();
        return error_page(StatusCode::BAD_REQUEST, &format!("{error} {detail}"));
    }
    let (Some(code), Some(state_param)) = (params.code, params.state) else {
        return error_page(StatusCode::BAD_REQUEST, "missing code/state");
    };
    let flow: FlowState = match session.remove(FLOW_KEY).await {
        Ok(Some(flow)) => flow,
        Ok(None) => return error_page(StatusCode::BAD_REQUEST, "no login in progress"),
        Err(e) => return error_page(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    if flow.csrf != state_param {
        return error_page(StatusCode::BAD_REQUEST, "state mismatch");
    }

    let client = match oidc.client() {
        Ok(client) => client,
        Err(e) => return error_page(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let exchange = match client.exchange_code(openidconnect::AuthorizationCode::new(code)) {
        Ok(exchange) => {
            exchange.set_pkce_verifier(openidconnect::PkceCodeVerifier::new(flow.pkce_verifier))
        }
        Err(e) => return error_page(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let tokens = match exchange.request_async(&oidc.http).await {
        Ok(tokens) => tokens,
        Err(e) => return error_page(StatusCode::BAD_GATEWAY, &e.to_string()),
    };

    let Some(id_token) = tokens.id_token() else {
        return error_page(StatusCode::BAD_GATEWAY, "no ID token in response");
    };
    let nonce = openidconnect::Nonce::new(flow.nonce);
    let claims = match id_token.claims(&client.id_token_verifier(), &nonce) {
        Ok(claims) => claims,
        Err(e) => return error_page(StatusCode::BAD_REQUEST, &e.to_string()),
    };
    let identity = claims.preferred_username().map_or_else(
        || claims.subject().as_str().to_owned(),
        |u| u.as_str().to_owned(),
    );

    let scopes = tokens
        .scopes()
        .map(|s| s.iter().map(|scope| scope.to_string()).collect())
        .unwrap_or_default();
    let admin = crate::session::AdminSession {
        identity,
        credential: crate::session::Credential::Bearer {
            access_token: tokens.access_token().secret().clone(),
        },
        scopes,
    };
    if let Err(e) = session.insert(crate::session::SESSION_KEY, admin).await {
        return error_page(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
    }
    Redirect::to("/").into_response()
}
