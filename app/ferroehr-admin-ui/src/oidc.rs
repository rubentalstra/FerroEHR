//! The console's OIDC authorization-code login (with PKCE): two plain axum
//! routes (`/auth/oidc/login`, `/auth/oidc/callback`) on the BFF.
//!
//! The bearer token is stored server-side in the session; the browser only
//! ever holds the console's session cookie.
//!
//! Discovery is lazy and cached ([`OidcState`]): the console boots with a
//! configured-but-unreachable issuer, serves Basic login, and retries
//! discovery on each OIDC sign-in until the provider answers.

use axum::response::{IntoResponse, Redirect, Response};
use http::StatusCode;
use openidconnect::{OAuth2TokenResponse, TokenResponse};
use serde::{Deserialize, Serialize};

use crate::config::OidcConfig;
use crate::error::AdminUiError;

/// The console's OIDC client and its lazily-discovered provider metadata.
///
/// Construction is inert: nothing here touches the network, so a configured
/// but unreachable identity provider cannot stop the console from booting and
/// serving Basic login. Discovery happens on the first OIDC sign-in (or the
/// boot-time warm-up, [`prime_in_background`]) and is cached; a failure leaves
/// the cache empty, so the next attempt retries without a restart.
#[derive(Debug)]
pub struct OidcState {
    /// The parsed issuer URL — a local config defect, checked at boot.
    issuer: openidconnect::IssuerUrl,
    http: openidconnect::reqwest::Client,
    config: OidcConfig,
    /// The discovery document once fetched. A `tokio` mutex, so concurrent
    /// first sign-ins collapse into one discovery instead of a thundering herd.
    discovered: tokio::sync::Mutex<Option<openidconnect::core::CoreProviderMetadata>>,
}

/// The transient state parked in the session between login and callback.
#[derive(Debug, Serialize, Deserialize)]
struct FlowState {
    pkce_verifier: String,
    csrf: String,
    nonce: String,
}

const FLOW_KEY: &str = "oidc_flow";

impl OidcState {
    /// Build the console's OIDC client without contacting the issuer.
    ///
    /// Everything checked here is LOCAL configuration — the `resolve`
    /// override's grammar, the socket address it names, and the issuer URL —
    /// so a failure is a config defect the operator must fix and stays fatal
    /// at boot. Reaching the identity provider is deferred to
    /// [`Self::metadata`].
    ///
    /// # Errors
    /// [`AdminUiError::Internal`] on a malformed `auth.oidc.resolve`, an
    /// unparseable resolve address, an HTTP client that cannot be built, or a
    /// malformed `auth.oidc.issuer`.
    pub fn new(config: &OidcConfig) -> Result<Self, AdminUiError> {
        let mut builder = openidconnect::reqwest::ClientBuilder::new()
            .redirect(openidconnect::reqwest::redirect::Policy::none()); // SSRF hardening
        // Split-horizon issuer resolution (`host=ip:port`): the canonical
        // issuer hostname may only resolve inside a container network; the
        // override points this client at the mapped address while every URL
        // (and the token `iss`) keeps the canonical form.
        if !config.resolve.trim().is_empty() {
            let (host, addr) = config.resolve.split_once('=').ok_or_else(|| {
                AdminUiError::Internal("auth.oidc.resolve must be `host=ip:port`".to_owned())
            })?;
            let addr: std::net::SocketAddr = addr.trim().parse().map_err(|e| {
                AdminUiError::Internal(format!("auth.oidc.resolve address `{addr}`: {e}"))
            })?;
            builder = builder.resolve(host.trim(), addr);
        }
        let http = builder
            .build()
            .map_err(|e| AdminUiError::Internal(format!("OIDC HTTP client: {e}")))?;
        let issuer = openidconnect::IssuerUrl::new(config.issuer.clone()).map_err(|e| {
            AdminUiError::Internal(format!("auth.oidc.issuer `{}`: {e}", config.issuer))
        })?;
        Ok(Self {
            issuer,
            http,
            config: config.clone(),
            discovered: tokio::sync::Mutex::new(None),
        })
    }

    /// The provider metadata: the cached discovery document, or a fresh
    /// discovery against the issuer.
    ///
    /// A failure caches nothing, so the next OIDC sign-in retries — an
    /// identity provider that comes up late recovers the console's OIDC path
    /// without a restart.
    ///
    /// # Errors
    /// [`AdminUiError::IdpUnavailable`] when discovery fails (the issuer is
    /// unreachable, times out, or does not serve a valid discovery document).
    pub async fn metadata(
        &self,
    ) -> Result<openidconnect::core::CoreProviderMetadata, AdminUiError> {
        let mut cached = self.discovered.lock().await;
        if let Some(metadata) = cached.as_ref() {
            return Ok(metadata.clone());
        }
        let metadata = openidconnect::core::CoreProviderMetadata::discover_async(
            self.issuer.clone(),
            &self.http,
        )
        .await
        .map_err(|e| AdminUiError::IdpUnavailable {
            issuer: self.config.issuer.clone(),
            message: e.to_string(),
        })?;
        *cached = Some(metadata.clone());
        Ok(metadata)
    }

    /// The OAuth2 client for a discovery document.
    ///
    /// Synchronous and independent of discovery, so a provider outage and a
    /// malformed `auth.oidc.public_base_url` stay distinct failure classes.
    ///
    /// # Errors
    /// [`AdminUiError::Internal`] when `auth.oidc.public_base_url` does not
    /// yield a valid redirect URI.
    fn client(
        &self,
        metadata: openidconnect::core::CoreProviderMetadata,
    ) -> Result<
        openidconnect::core::CoreClient<
            openidconnect::EndpointSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointMaybeSet,
            openidconnect::EndpointMaybeSet,
        >,
        AdminUiError,
    > {
        let redirect = openidconnect::RedirectUrl::new(format!(
            "{}/auth/oidc/callback",
            self.config.public_base_url.trim_end_matches('/')
        ))
        .map_err(|e| {
            AdminUiError::Internal(format!(
                "auth.oidc.public_base_url `{}`: {e}",
                self.config.public_base_url
            ))
        })?;
        Ok(openidconnect::core::CoreClient::from_provider_metadata(
            metadata,
            openidconnect::ClientId::new(self.config.client_id.clone()),
            Some(openidconnect::ClientSecret::new(
                self.config.client_secret.clone(),
            )),
        )
        .set_redirect_uri(redirect))
    }
}

/// Warm the discovery cache once, in the background, without blocking boot.
///
/// The console serves requests immediately either way; this exists so an
/// identity provider that is down at startup is named loudly in the boot log
/// instead of only surfacing on someone's first sign-in attempt.
pub fn prime_in_background(oidc: std::sync::Arc<OidcState>) {
    tokio::spawn(async move {
        match oidc.metadata().await {
            Ok(_) => tracing::info!(
                issuer = %oidc.config.issuer,
                "OIDC discovery succeeded; OIDC sign-in is available"
            ),
            Err(e) => tracing::warn!(
                issuer = %oidc.config.issuer,
                error = %e,
                "OIDC discovery failed at startup; the console serves Basic login and retries discovery on the next OIDC sign-in"
            ),
        }
    });
}

/// Report an identity-provider outage on the login screen.
///
/// The raw diagnostic goes to the log (operators need it, users cannot act on
/// it); the browser gets actionable copy and the still-working Basic form.
fn idp_unavailable(issuer: &str, error: &AdminUiError) -> Response {
    tracing::warn!(issuer, error = %error, "OIDC discovery failed");
    let message = format!(
        "OIDC sign-in is unavailable: the identity provider at {issuer} could not be reached. \
         Try again shortly, or sign in with a username and password."
    );
    Redirect::to(&format!("/login?error={}", urlencoding::encode(&message))).into_response()
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
    let metadata = match oidc.metadata().await {
        Ok(metadata) => metadata,
        Err(e) => return idp_unavailable(&oidc.config.issuer, &e),
    };
    let client = match oidc.client(metadata) {
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

    let metadata = match oidc.metadata().await {
        Ok(metadata) => metadata,
        Err(e) => return idp_unavailable(&oidc.config.issuer, &e),
    };
    let client = match oidc.client(metadata) {
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
