//! Authentication (Stage 1): HTTP Basic + OAuth2/OIDC bearer.
//!
//! Applied as one axum middleware over the API router (not per handler). A
//! successful authentication puts a [`Principal`] into the request extensions
//! for downstream handlers/the service layer. Fine-grained RBAC is Stage 2; the
//! only authorization here is the optional coarse admin-scope gate.

mod basic;
pub mod config;
mod jwt;

use std::sync::Arc;

use axum::extract::{FromRequestParts, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::request::Parts;
use http::{HeaderValue, StatusCode, header};

use openehr_its::rest::runtime::ApiError;

use crate::error::RestError;
pub use config::AuthConfig;
use jwt::JwtValidator;

/// The authenticated caller.
#[derive(Debug, Clone)]
pub struct Principal {
    /// Token `sub` / Basic username.
    pub subject: String,
    /// `OAuth2` scopes granted to the caller (empty for Basic).
    pub scopes: Vec<String>,
    /// Roles granted to the caller, normalized to upper-case. Extracted from the
    /// configured JWT claim paths (default `realm_access.roles` + `scope`) for
    /// Bearer; from the Basic user's configured roles for Basic. Consumed by the
    /// RBAC gate (§5.2 of `docs/enterprise/access-control.md`).
    pub roles: Vec<String>,
    /// The retained, validated JWT claim set (Bearer only; empty for Basic).
    /// Kept so the Stage-2 ABAC layer can resolve attributes (organization /
    /// patient) without re-parsing the token; unused by the RBAC gate.
    pub claims: serde_json::Map<String, serde_json::Value>,
    /// Which mechanism authenticated the caller.
    pub method: AuthMethod,
}

/// The mechanism that authenticated a [`Principal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    Basic,
    Bearer,
}

/// An authentication failure.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthError {
    #[error("no credentials supplied")]
    MissingCredentials,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid bearer token: {0}")]
    InvalidToken(String),
    #[error("could not resolve signing keys: {0}")]
    KeyResolution(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
}

impl AuthError {
    fn to_api_error(&self) -> ApiError {
        match self {
            AuthError::Forbidden(m) => ApiError::Forbidden(m.clone()),
            other => ApiError::Unauthorized(other.to_string()),
        }
    }
}

/// The configured authenticator: the enabled mechanisms and the coarse admin
/// gate. Cheap to clone (shared internals).
pub struct Authenticator {
    config: AuthConfig,
    jwt: Option<JwtValidator>,
}

impl std::fmt::Debug for Authenticator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Authenticator")
            .field("enabled", &self.config.enabled)
            .field("basic", &self.config.basic.is_some())
            .field("oidc", &self.jwt.is_some())
            .finish()
    }
}

impl Authenticator {
    /// Build from configuration, constructing the bearer validator if OIDC is
    /// configured.
    ///
    /// # Errors
    /// Returns a message if the OIDC key material/algorithms are invalid.
    pub fn new(config: AuthConfig) -> Result<Arc<Self>, String> {
        let jwt = match &config.oidc {
            Some(oidc) => Some(JwtValidator::from_config(oidc)?),
            None => None,
        };
        Ok(Arc::new(Self { config, jwt }))
    }

    pub(crate) fn enabled(&self) -> bool {
        self.config.enabled
    }

    /// The configured admin scope, if the coarse admin gate is enabled.
    pub(crate) fn admin_scope(&self) -> Option<&str> {
        self.config.admin_scope.as_deref()
    }

    /// The `WWW-Authenticate` challenge advertising the enabled mechanisms.
    pub(crate) fn challenge(&self) -> HeaderValue {
        let mut parts = Vec::new();
        if self.config.basic.is_some() {
            parts.push(r#"Basic realm="ehrbase""#);
        }
        if self.jwt.is_some() {
            parts.push("Bearer");
        }
        if parts.is_empty() {
            parts.push(r#"Basic realm="ehrbase""#);
        }
        HeaderValue::from_str(&parts.join(", "))
            .unwrap_or_else(|_| HeaderValue::from_static("Basic"))
    }

    pub(crate) async fn authenticate(
        &self,
        headers: &http::HeaderMap,
    ) -> Result<Principal, AuthError> {
        let auth = headers
            .get(header::AUTHORIZATION)
            .ok_or(AuthError::MissingCredentials)?;
        let scheme = auth
            .to_str()
            .map_err(|_| AuthError::InvalidCredentials)?
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        match scheme.as_str() {
            "basic" => {
                let cfg = self
                    .config
                    .basic
                    .as_ref()
                    .ok_or(AuthError::InvalidCredentials)?;
                basic::verify(auth, cfg)
            }
            "bearer" => {
                let validator = self.jwt.as_ref().ok_or(AuthError::InvalidCredentials)?;
                let token = auth
                    .to_str()
                    .map_err(|_| AuthError::InvalidCredentials)?
                    .trim_start_matches(|c: char| !c.is_whitespace())
                    .trim();
                validator.validate(token).await
            }
            _ => Err(AuthError::InvalidCredentials),
        }
    }

    /// The coarse admin-scope gate (Stage-1 seam). No-op unless configured.
    fn authorize_admin(&self, path: &str, principal: &Principal) -> Result<(), AuthError> {
        // TODO(port): Stage 2 RBAC — replace with real per-operation authorization.
        let Some(required) = &self.config.admin_scope else {
            return Ok(());
        };
        if is_admin_path(path) && !principal.scopes.iter().any(|s| s == required) {
            return Err(AuthError::Forbidden(format!(
                "admin operations require the '{required}' scope"
            )));
        }
        Ok(())
    }
}

fn is_admin_path(path: &str) -> bool {
    path.split('/').any(|seg| seg == "admin")
}

tokio::task_local! {
    /// The authenticated principal for the request currently being handled.
    static REQUEST_PRINCIPAL: Option<Principal>;
}

/// The authenticated principal for the current request, if any.
///
/// Set by [`middleware`] for the duration of request handling; downstream layers
/// (notably the service layer, when attributing a CONTRIBUTION's committer) read
/// it without the principal having to be threaded through the generated trait
/// signatures. Returns `None` when unauthenticated or called outside a request.
#[must_use]
pub fn current_principal() -> Option<Principal> {
    REQUEST_PRINCIPAL.try_with(Clone::clone).ok().flatten()
}

/// The authentication middleware. Attached to the API router; public endpoints
/// (status, health, Swagger) are mounted outside it.
pub async fn middleware(
    State(auth): State<Arc<Authenticator>>,
    mut req: Request,
    next: Next,
) -> Response {
    if !auth.enabled() {
        return REQUEST_PRINCIPAL.scope(None, next.run(req)).await;
    }

    let path = req.uri().path().to_owned();
    match auth.authenticate(req.headers()).await {
        Ok(principal) => {
            if let Err(e) = auth.authorize_admin(&path, &principal) {
                metrics::counter!(
                    crate::management::AUTH_FAILURES,
                    "mechanism" => mechanism_label(principal.method),
                    "status" => "403",
                )
                .increment(1);
                // Attribute the 403 to the authenticated caller for the audit layer.
                let mut resp = RestError(e.to_api_error()).into_response();
                resp.extensions_mut().insert(principal);
                return resp;
            }
            req.extensions_mut().insert(principal.clone());
            // Publish the principal for the service layer (committer attribution).
            let for_audit = principal.clone();
            let mut resp = REQUEST_PRINCIPAL
                .scope(Some(principal), next.run(req))
                .await;
            // Republish onto the response so the outer ATNA audit layer — which
            // cannot observe request-extension mutations — can attribute events.
            resp.extensions_mut().insert(for_audit);
            resp
        }
        Err(e) => {
            let api = e.to_api_error();
            let status = api.status();
            metrics::counter!(
                crate::management::AUTH_FAILURES,
                "mechanism" => scheme_label(req.headers()),
                "status" => if status == StatusCode::FORBIDDEN { "403" } else { "401" },
            )
            .increment(1);
            let needs_challenge = status == StatusCode::UNAUTHORIZED;
            let mut resp = RestError(api).into_response();
            if needs_challenge {
                resp.headers_mut()
                    .insert(header::WWW_AUTHENTICATE, auth.challenge());
            }
            resp
        }
    }
}

/// The `mechanism` metric label for a known authenticated principal.
fn mechanism_label(method: AuthMethod) -> &'static str {
    match method {
        AuthMethod::Basic => "basic",
        AuthMethod::Bearer => "bearer",
    }
}

/// The `mechanism` metric label derived from the request's `Authorization`
/// scheme (for failures where no principal was established).
fn scheme_label(headers: &http::HeaderMap) -> &'static str {
    match headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split_whitespace().next())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("basic") => "basic",
        Some("bearer") => "bearer",
        _ => "none",
    }
}

/// Extractor for handlers/the service layer to read the authenticated caller.
/// Yields 401 if no principal is present (auth disabled or middleware bypassed).
#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub Principal);

impl<S: Sync> FromRequestParts<S> for AuthenticatedUser {
    type Rejection = RestError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Principal>()
            .cloned()
            .map(AuthenticatedUser)
            .ok_or_else(|| RestError(ApiError::Unauthorized("authentication required".to_owned())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::config::{BasicConfig, BasicUser, OidcConfig, Redacted};
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{Argon2, password_hash::PasswordHash};

    fn hash(pw: &str) -> String {
        let salt = SaltString::from_b64("MTIzNDU2Nzg5MDEyMzQ1Ng").unwrap();
        Argon2::default()
            .hash_password(pw.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }

    fn basic_only() -> Arc<Authenticator> {
        Authenticator::new(AuthConfig {
            enabled: true,
            basic: Some(BasicConfig {
                users: vec![BasicUser {
                    username: "alice".to_owned(),
                    password_hash: Redacted(hash("pw")),
                    roles: vec!["USER".to_owned()],
                }],
            }),
            oidc: None,
            admin_scope: Some("ehrbase:admin".to_owned()),
        })
        .unwrap()
    }

    fn hmac_oidc() -> Arc<Authenticator> {
        Authenticator::new(AuthConfig {
            enabled: true,
            basic: None,
            oidc: Some(OidcConfig {
                issuer: "https://issuer.example".to_owned(),
                audiences: vec![],
                algorithms: vec!["HS256".to_owned()],
                hmac_secret: Some(Redacted("secret".to_owned())),
                jwks_json: None,
            }),
            admin_scope: None,
        })
        .unwrap()
    }

    fn headers(auth: &str) -> http::HeaderMap {
        let mut h = http::HeaderMap::new();
        h.insert(header::AUTHORIZATION, HeaderValue::from_str(auth).unwrap());
        h
    }

    #[tokio::test]
    async fn missing_credentials() {
        let err = basic_only()
            .authenticate(&http::HeaderMap::new())
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::MissingCredentials));
        assert_eq!(err.to_api_error().status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn valid_basic() {
        // "alice:pw" base64 = YWxpY2U6cHc=
        let p = basic_only()
            .authenticate(&headers("Basic YWxpY2U6cHc="))
            .await
            .expect("ok");
        assert_eq!(p.subject, "alice");
    }

    #[test]
    fn admin_gate_forbids_without_scope() {
        let auth = basic_only();
        let principal = Principal {
            subject: "alice".to_owned(),
            scopes: vec![],
            roles: vec![],
            claims: serde_json::Map::new(),
            method: AuthMethod::Basic,
        };
        let err = auth
            .authorize_admin("/ehrbase/rest/openehr/v1/admin/ehr/x", &principal)
            .expect_err("forbidden");
        assert!(matches!(err, AuthError::Forbidden(_)));
        assert_eq!(err.to_api_error().status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn admin_gate_allows_with_scope() {
        let auth = basic_only();
        let principal = Principal {
            subject: "alice".to_owned(),
            scopes: vec!["ehrbase:admin".to_owned()],
            roles: vec![],
            claims: serde_json::Map::new(),
            method: AuthMethod::Basic,
        };
        assert!(auth.authorize_admin("/x/admin/ehr/y", &principal).is_ok());
    }

    #[test]
    fn non_admin_path_never_gated() {
        let auth = basic_only();
        let principal = Principal {
            subject: "alice".to_owned(),
            scopes: vec![],
            roles: vec![],
            claims: serde_json::Map::new(),
            method: AuthMethod::Basic,
        };
        assert!(auth.authorize_admin("/x/ehr/y", &principal).is_ok());
    }

    #[tokio::test]
    async fn bearer_without_oidc_is_rejected() {
        let err = basic_only()
            .authenticate(&headers("Bearer sometoken"))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[test]
    fn phc_hash_is_parseable() {
        // Guards the fixed-salt test helper against argon2 format drift.
        assert!(PasswordHash::new(&hash("pw")).is_ok());
    }

    #[tokio::test]
    async fn bearer_smoke_with_hmac() {
        // A malformed token is rejected as an invalid token (not missing creds).
        let err = hmac_oidc()
            .authenticate(&headers("Bearer not-a-jwt"))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidToken(_)), "got {err:?}");
    }
}
