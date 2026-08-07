//! Authentication: HTTP Basic + OAuth2/OIDC bearer, plus the coarse **RBAC**
//! gate.
//!
//! # Spec grounding
//!
//! ITS-REST leaves authentication a `SHOULD` and mandates no scheme
//! (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
//! §Authentication and authorization). But **when a framework is present the
//! service MUST** use `WWW-Authenticate`/`Proxy-Authenticate` and return
//! `401`/`403`/`407` as applicable — the normative bar this middleware meets:
//! missing/invalid credentials → `401` with a `WWW-Authenticate` challenge
//! (`Authenticator::challenge`); authenticated-but-refused → `403`, no
//! challenge. (We serve no proxy, so `407`/`Proxy-Authenticate` do not apply.)
//! The CNF security suites
//! (`docs/specs/openehr/CNF/tests/platform/robot/SECURITY_TESTS/`) are the
//! conformance reference for the Basic + bearer 401/403 flows.
//!
//! # RBAC (spec-silent)
//!
//! No openEHR spec governs role-based authorization — the SM places it out of
//! band. The coarse RBAC gate is our own enterprise extension, kept clearly separate from the
//! spec-grounded authn above.
//!
//! Applied as one axum middleware over the API router (not per handler). A
//! successful authentication puts a [`Principal`] (with roles + retained JWT
//! claims) into the request extensions for downstream handlers/the service
//! layer. When an [`crate::extensions::access::authz::AuthzHandle`] is wired, the
//! middleware then runs the RBAC gate over the matched operation's class: a deny
//! is a `403` with the `Principal` attached to the response so the ATNA audit
//! layer records it.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 7): RFC 7519 leaves the claim set open; \
              decided-on claims lift into typed fields"
)]

mod basic;
mod jwt;

use ferroehr::config::auth::AuthConfig;

use std::sync::Arc;

use crate::extensions::access::authz::roles::RbacDecision;
use axum::extract::{FromRequestParts, MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http::request::Parts;
use http::{HeaderValue, StatusCode, header};

use openehr_its::rest::runtime::ApiError;

use crate::extensions::access::authz::AuthzHandle;
use crate::overview::error::RestError;
use jwt::JwtValidator;

/// The state the [`middleware`] runs on: the authenticator plus the optional
/// authorization handle (the RBAC gate; `None` restores authentication-only
/// behaviour). Cheap to clone.
#[derive(Clone)]
pub(crate) struct AuthLayer {
    pub(crate) authenticator: Arc<Authenticator>,
    pub(crate) authz: Option<Arc<AuthzHandle>>,
}

/// The authenticated caller.
#[derive(Debug, Clone)]
pub struct Principal {
    /// Token `sub` / Basic username.
    pub subject: String,
    /// `OAuth2` scopes granted to the caller (empty for Basic).
    pub scopes: Vec<String>,
    /// Roles granted to the caller, normalized to upper-case. For Bearer they
    /// come from the configured JWT claim paths (`authz.rbac.role_claims`,
    /// defaulting to the RFC 9068 §2.2.3.1 carriers); for Basic from the user's
    /// configured role list. Consumed by the RBAC gate. An OAuth2 scope is NOT
    /// a role — see [`Self::scopes`].
    pub roles: Vec<String>,
    /// The retained, validated JWT claim set (Bearer only; empty for Basic).
    /// Kept so the ABAC layer can resolve subject attributes (organization,
    /// patient) without re-parsing the token; unused by the RBAC gate.
    pub claims: serde_json::Map<String, serde_json::Value>,
    /// Which mechanism authenticated the caller.
    pub method: AuthMethod,
}

/// The mechanism that authenticated a [`Principal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// HTTP Basic: a username/password pair verified against the Argon2 store.
    Basic,
    /// OAuth2/OIDC Bearer: a JWT validated against the issuer's JWKS.
    Bearer,
}

/// A successful authentication: the [`Principal`] plus whether THIS request
/// performed a genuine credential verification (an actual authentication event,
/// not a continuation of an established one). It is `true` only for a Basic
/// verified-credential cache **miss** — the KDF actually ran, so the caller
/// authenticated here and now. A cache hit (the same credential re-presented
/// within the TTL) is a continuing session, and a Bearer request is federated
/// (the authentication event happened out of band at the OIDC provider), so
/// both are `false`. Used solely to decide whether to emit an IHE ATNA
/// login/"Application Activity" record, which marks authentication events, not
/// individual requests.
#[derive(Debug, Clone)]
pub(crate) struct Authenticated {
    pub(crate) principal: Principal,
    pub(crate) fresh: bool,
}

/// Response-extension marker set by [`middleware`] when a request carried a
/// genuine authentication event (see [`Authenticated::fresh`]). The outermost
/// ATNA audit layer reads it to emit the login record only on real
/// authentications rather than on every authenticated request.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FreshAuthentication;

/// An authentication failure.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthError {
    /// The request carried no credential for any enabled mechanism.
    #[error("no credentials supplied")]
    MissingCredentials,
    /// An unknown user, or a password that does not match the stored Argon2
    /// hash. A credential that was PRESENTED and REJECTED — RFC 6750 §3.1's
    /// `invalid_token` case for bearer, an ordinary 401 for Basic.
    #[error("invalid credentials")]
    InvalidCredentials,
    /// The `Authorization` header is not a well-formed credential at all: an
    /// unparsable header, an unknown scheme, or a bearer credential outside the
    /// RFC 6750 §2.1 `b64token` grammar.
    ///
    /// RFC 6750 §3.1 gives this its own code — "The request is missing a
    /// required parameter, includes an unsupported parameter or parameter
    /// value, repeats the same parameter, uses more than one method for
    /// including an access token, or is otherwise malformed … 400 (Bad
    /// Request)" — because it is a request defect, not a credential decision.
    #[error("malformed authorization header: {0}")]
    MalformedRequest(String),
    /// A bearer token that failed structural, signature, or claim validation.
    #[error("invalid bearer token: {0}")]
    InvalidToken(String),
    /// The issuer's signing keys (JWKS) could not be fetched or parsed, so no
    /// bearer token can be validated.
    #[error("could not resolve signing keys: {0}")]
    KeyResolution(String),
    /// The caller authenticated but is not permitted the request — the 403
    /// branch of the ITS-REST 401-vs-403 split.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// The server could not RUN credential verification at all (e.g. the
    /// blocking Argon2 task panicked or was cancelled). A server fault, never
    /// a statement about the credentials — maps to 500, not 401, so an
    /// operator failure is never misreported as a client error.
    #[error("credential verification unavailable: {0}")]
    VerificationUnavailable(String),
}

impl AuthError {
    fn to_api_error(&self) -> ApiError {
        match self {
            AuthError::Forbidden(m) => ApiError::Forbidden(m.clone()),
            // A server fault: the reason names an internal task/KDF failure,
            // so it is traced and the body carries the curated message.
            AuthError::VerificationUnavailable(m) => {
                crate::overview::error::internal_fault("verify the supplied credentials", m)
            }
            AuthError::MalformedRequest(m) => ApiError::BadRequest(m.clone()),
            // The issuer's keys are unreachable, so NO token can be validated:
            // the server cannot decide, which RFC 9110 §15.6.4 makes a 503 —
            // "the server is currently unable to handle the request due to a
            // temporary overload or scheduled maintenance". Answering 401 would
            // tell a caller with a perfectly valid token that its credential was
            // rejected, and a client that trusts that stops retrying.
            AuthError::KeyResolution(m) => ApiError::ServiceUnavailable(m.clone()),
            other => ApiError::Unauthorized(other.to_string()),
        }
    }

    /// The RFC 6750 §3.1 `error` code this outcome carries on its
    /// `WWW-Authenticate` challenge, or `None` when the challenge must carry no
    /// code at all.
    ///
    /// §3.1: "If the request lacks any authentication information …  the
    /// resource server SHOULD NOT include an error code" — an unauthenticated
    /// caller has not made a mistake yet, so naming one invites the client to
    /// treat a first request as a failure.
    const fn bearer_error_code(&self) -> Option<&'static str> {
        match self {
            AuthError::InvalidCredentials | AuthError::InvalidToken(_) => Some("invalid_token"),
            AuthError::Forbidden(_) => Some("insufficient_scope"),
            AuthError::MalformedRequest(_) => Some("invalid_request"),
            // No credential presented, or a server-side fault: neither is a
            // statement the client can act on by changing its credential.
            AuthError::MissingCredentials
            | AuthError::KeyResolution(_)
            | AuthError::VerificationUnavailable(_) => None,
        }
    }
}

/// The configured authenticator: the enabled mechanisms and the coarse admin
/// gate. Cheap to clone (shared internals).
pub struct Authenticator {
    config: AuthConfig,
    jwt: Option<JwtValidator>,
    /// Verified Basic-credential cache: SHA-256 of the presented
    /// `Authorization` header → the verified [`Principal`]. An entry exists
    /// only after a successful Argon2 verification; the TTL
    /// (`config::AuthConfig::verified_cache_ttl_seconds`) bounds revocation
    /// lag. `None` when the TTL is `0` (cache disabled). Argon2 verification
    /// is tens of milliseconds of CPU per call by design — without this, a
    /// busy client's request rate is capped by the KDF's work factor.
    verified: Option<moka::future::Cache<[u8; 32], Principal>>,
    /// KDF verifications actually performed (cache misses) — a test seam and
    /// a cheap operational signal.
    kdf_verifications: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for Authenticator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Authenticator")
            .field("enabled", &self.config.enabled)
            .field("basic", &self.config.basic.is_some())
            .field("oidc", &self.jwt.is_some())
            .finish_non_exhaustive()
    }
}

impl Authenticator {
    /// Build from configuration, constructing the bearer validator if OIDC is
    /// configured.
    ///
    /// # Errors
    /// Returns a message if the OIDC key material/algorithms are invalid.
    pub fn new(config: AuthConfig) -> Result<Arc<Self>, String> {
        Self::with_role_claims(
            config,
            ferroehr::config::authz::RbacConfig::default().role_claims,
        )
    }

    /// Build from configuration with explicit RBAC role-claim paths (from
    /// `authz.rbac.role_claims`), used when an [`crate::extensions::access::authz::AuthzHandle`] is
    /// wired; [`Authenticator::new`] defaults them.
    ///
    /// # Errors
    /// Returns a message if the OIDC key material/algorithms are invalid.
    pub fn with_role_claims(
        config: AuthConfig,
        role_claims: Vec<String>,
    ) -> Result<Arc<Self>, String> {
        let jwt = match &config.oidc {
            Some(oidc) => Some(JwtValidator::with_role_claims(oidc, role_claims)?),
            None => None,
        };
        let verified = (config.verified_cache_ttl_seconds > 0).then(|| {
            moka::future::Cache::builder()
                .max_capacity(1024)
                .time_to_live(std::time::Duration::from_secs(
                    config.verified_cache_ttl_seconds,
                ))
                .build()
        });
        Ok(Arc::new(Self {
            config,
            jwt,
            verified,
            kdf_verifications: std::sync::atomic::AtomicU64::new(0),
        }))
    }

    pub(crate) fn enabled(&self) -> bool {
        self.config.enabled
    }

    /// The `WWW-Authenticate` challenge advertising the enabled mechanisms.
    pub(crate) fn challenge(&self, outcome: Option<&AuthError>) -> HeaderValue {
        let error = outcome.and_then(AuthError::bearer_error_code);
        let mut parts: Vec<String> = Vec::new();
        if self.config.basic.is_some() {
            // RFC 7617 §2.1: `charset="UTF-8"` tells the client which encoding
            // to use for the credential; the RFC defines `UTF-8` as its only
            // legal value.
            parts.push(r#"Basic realm="ferroehr", charset="UTF-8""#.to_owned());
        }
        if self.jwt.is_some() {
            // RFC 6750 §3: the `Bearer` scheme's challenge carries `realm` and,
            // when the request failed for a reason the client can act on, an
            // `error` code from §3.1.
            let bearer = match error {
                Some(code) => format!(r#"Bearer realm="ferroehr", error="{code}""#),
                None => r#"Bearer realm="ferroehr""#.to_owned(),
            };
            parts.push(bearer);
        }
        if parts.is_empty() {
            // Unreachable in a booted server (`auth.enabled` without a mechanism
            // is a boot error), but a challenge must never be empty.
            parts.push(r#"Basic realm="ferroehr", charset="UTF-8""#.to_owned());
        }
        HeaderValue::from_str(&parts.join(", "))
            .unwrap_or_else(|_| HeaderValue::from_static(r#"Basic realm="ferroehr""#))
    }

    #[expect(
        clippy::map_err_ignore,
        reason = "every failure on the credential path collapses to one opaque \
                  outcome deliberately: a caller must not learn from the 401 which \
                  part of the credential was rejected"
    )]
    pub(crate) async fn authenticate(
        &self,
        headers: &http::HeaderMap,
    ) -> Result<Authenticated, AuthError> {
        let auth = headers
            .get(header::AUTHORIZATION)
            .ok_or(AuthError::MissingCredentials)?;
        let raw = auth.to_str().map_err(|_| {
            AuthError::MalformedRequest(
                "the Authorization header is not valid ISO-8859-1 text".to_owned(),
            )
        })?;
        let scheme = raw
            .split_whitespace()
            .next()
            .ok_or_else(|| {
                AuthError::MalformedRequest("the Authorization header is empty".to_owned())
            })?
            .to_ascii_lowercase();

        match scheme.as_str() {
            "basic" => {
                let cfg = self
                    .config
                    .basic
                    .as_ref()
                    .ok_or(AuthError::InvalidCredentials)?;
                // Verified-credential cache: key = SHA-256 of the presented
                // header (never the plaintext). A hit skips the KDF entirely;
                // a miss runs Argon2 on the blocking pool so the async workers
                // are never parked on CPU-bound hashing.
                let key: [u8; 32] = {
                    use sha2::Digest as _;
                    sha2::Sha256::digest(auth.as_bytes()).into()
                };
                if let Some(cache) = &self.verified
                    && let Some(principal) = cache.get(&key).await
                {
                    // A cache hit is a continuing session, not a new
                    // authentication event.
                    return Ok(Authenticated {
                        principal,
                        fresh: false,
                    });
                }
                self.kdf_verifications
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let principal = {
                    let auth = auth.clone();
                    let cfg = cfg.clone();
                    tokio::task::spawn_blocking(move || basic::verify(&auth, &cfg))
                        .await
                        .map_err(|join_error| {
                            AuthError::VerificationUnavailable(format!(
                                "password-verification task failed: {join_error}"
                            ))
                        })??
                };
                if let Some(cache) = &self.verified {
                    cache.insert(key, principal.clone()).await;
                }
                // The KDF actually ran: this request is a genuine authentication.
                Ok(Authenticated {
                    principal,
                    fresh: true,
                })
            }
            "bearer" => {
                let validator = self.jwt.as_ref().ok_or(AuthError::InvalidCredentials)?;
                let token = bearer_credential(raw)?;
                // Federated: the authentication event happened at the OIDC
                // provider, so a per-request login record is never minted here.
                validator
                    .validate(token)
                    .await
                    .map(|principal| Authenticated {
                        principal,
                        fresh: false,
                    })
            }
            other => Err(AuthError::MalformedRequest(format!(
                "unsupported authentication scheme `{other}`"
            ))),
        }
    }
}

/// Extract the bearer credential from an `Authorization` header value, enforcing
/// the RFC 6750 §2.1 grammar.
///
/// The RFC is exact: `credentials = "Bearer" 1*SP b64token` with
/// `b64token = 1*( ALPHA / DIGIT / "-" / "." / "_" / "~" / "+" / "/" ) *"="`.
/// A value outside it is a malformed REQUEST, not a rejected credential, so it
/// answers 400 rather than 401 (§3.1 `invalid_request`).
///
/// # Errors
/// [`AuthError::MalformedRequest`] when the scheme is not followed by at least
/// one space and a non-empty `b64token`, or when the token carries a character
/// the grammar does not admit.
fn bearer_credential(raw: &str) -> Result<&str, AuthError> {
    let rest = raw
        .strip_prefix("Bearer")
        .or_else(|| raw.strip_prefix("bearer"))
        .ok_or_else(|| AuthError::MalformedRequest("expected the Bearer scheme".to_owned()))?;
    // `1*SP`: at least one space, and the RFC names SP specifically — a tab or a
    // newline is not a legal separator.
    if !rest.starts_with(' ') {
        return Err(AuthError::MalformedRequest(
            "the Bearer scheme must be followed by a single space and the token".to_owned(),
        ));
    }
    let token = rest.trim_start_matches(' ');
    if token.is_empty() {
        return Err(AuthError::MalformedRequest(
            "the Bearer credential carries no token".to_owned(),
        ));
    }
    // `*"="` is a TRAILING run only, so padding may not appear mid-token.
    let body = token.trim_end_matches('=');
    if !body
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'+' | b'/'))
    {
        return Err(AuthError::MalformedRequest(
            "the Bearer token is outside the RFC 6750 b64token grammar".to_owned(),
        ));
    }
    Ok(token)
}

tokio::task_local! {
    /// The authenticated principal for the request currently being handled.
    static REQUEST_PRINCIPAL: Option<Principal>;
}

/// The authenticated principal for the current request, if any.
///
/// Set by `middleware` for the duration of request handling; downstream layers
/// (notably the service layer, when attributing a CONTRIBUTION's committer) read
/// it without the principal having to be threaded through the generated trait
/// signatures. Returns `None` when unauthenticated or called outside a request.
#[must_use]
pub fn current_principal() -> Option<Principal> {
    REQUEST_PRINCIPAL.try_with(Clone::clone).ok().flatten()
}

/// The authentication + RBAC middleware. Attached to the API router; public
/// endpoints (status, health, Swagger) are mounted outside it.
pub(crate) async fn middleware(
    State(layer): State<AuthLayer>,
    mut req: Request,
    next: Next,
) -> Response {
    let auth = &layer.authenticator;
    if !auth.enabled() {
        return REQUEST_PRINCIPAL.scope(None, next.run(req)).await;
    }

    match auth.authenticate(req.headers()).await {
        Ok(Authenticated { principal, fresh }) => {
            // RBAC gate: resolve the matched operation's class and gate it
            // against the caller's roles. `None` authz handle = auth-only.
            if let Some(rbac) = layer.authz.as_deref().and_then(AuthzHandle::rbac) {
                let matched = req
                    .extensions()
                    .get::<MatchedPath>()
                    .map(|m| m.as_str().to_owned());
                let class = rbac.class_for(req.method(), matched.as_deref());
                // The coarse class gate, then the read-only restriction: a
                // principal carrying the configured read-only role is refused on
                // every write operation, overriding any grant. Both denials share
                // the one 403 path below (no openEHR spec governs role semantics
                // — our own design/extension).
                let decision = match rbac.decide(class, &principal.roles) {
                    RbacDecision::Deny(reason) => RbacDecision::Deny(reason),
                    RbacDecision::Allow => {
                        let is_write = rbac.is_write_for(req.method(), matched.as_deref());
                        rbac.decide_readonly(is_write, &principal.roles)
                    }
                };
                if let RbacDecision::Deny(reason) = decision {
                    metrics::counter!(
                        ferroehr::telemetry::prometheus::AUTH_FAILURES,
                        "mechanism" => mechanism_label(principal.method),
                        "status" => "403",
                    )
                    .increment(1);
                    // Attribute the 403 to the authenticated caller so the outer
                    // ATNA audit layer records the denied access.
                    let mut resp = RestError(ApiError::Forbidden(reason)).into_response();
                    resp.extensions_mut().insert(principal);
                    return resp;
                }
            }
            req.extensions_mut().insert(principal.clone());
            // Publish the principal for the service layer (committer attribution).
            let for_audit = principal.clone();
            // Also publish the platform committer identity: a default-committer
            // audit (a write whose request carried no committal headers) is
            // attributed to the authenticated principal instead of the system
            // identity (`AUDIT_DETAILS.committer` 1..1 — RM common master04
            // §Audit Details).
            let committer = ferroehr::service::committer::CommitterIdentity {
                subject: for_audit.subject.clone(),
                id_type: match for_audit.method {
                    AuthMethod::Basic => "basic",
                    AuthMethod::Bearer => "oauth2",
                },
                // A Bearer principal's subject was minted by the identity
                // provider, so the issuing authority the audit records is the
                // validated token issuer (`iss`, checked against the configured
                // issuer in `jwt.rs` before the claim set is retained), never
                // this server. Basic credentials are held locally and carry no
                // external issuer, so the platform stamps its own product name.
                issuer: match for_audit.method {
                    AuthMethod::Basic => None,
                    AuthMethod::Bearer => for_audit
                        .claims
                        .get("iss")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                },
            };
            let mut resp = REQUEST_PRINCIPAL
                .scope(
                    Some(principal),
                    ferroehr::service::committer::with_committer(Some(committer), next.run(req)),
                )
                .await;
            // Republish onto the response so the outer ATNA audit layer — which
            // cannot observe request-extension mutations — can attribute events.
            resp.extensions_mut().insert(for_audit);
            // Mark a genuine authentication event so the ATNA layer emits the
            // login record only then, not on every authenticated request.
            if fresh {
                resp.extensions_mut().insert(FreshAuthentication);
            }
            resp
        }
        Err(e) => {
            let api = e.to_api_error();
            let status = api.status();
            metrics::counter!(
                ferroehr::telemetry::prometheus::AUTH_FAILURES,
                "mechanism" => scheme_label(req.headers()),
                "status" => if status == StatusCode::FORBIDDEN { "403" } else { "401" },
            )
            .increment(1);
            // ITS-REST §Authentication and authorization: a `401` MUST carry a
            // `WWW-Authenticate` challenge. RFC 6750 §3 additionally carries one
            // on a bearer `403` — the `insufficient_scope` case — because there
            // the challenge tells the client WHAT it lacks rather than that it is
            // unauthenticated.
            let needs_challenge =
                matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN);
            let challenge = auth.challenge(Some(&e));
            let mut resp = RestError(api).into_response();
            if needs_challenge {
                resp.headers_mut()
                    .insert(header::WWW_AUTHENTICATE, challenge);
            }
            resp
        }
    }
}

/// The RFC 6750 §2.1 grammar and the challenge shapes, asserted per clause.
#[cfg(test)]
mod bearer_grammar_tests {
    use super::{AuthError, bearer_credential};

    #[test]
    fn bearer_credentials_follow_rfc6750_abnf() {
        // `credentials = "Bearer" 1*SP b64token`
        assert_eq!(
            bearer_credential("Bearer abc.def.ghi").expect("ok"),
            "abc.def.ghi"
        );
        assert_eq!(bearer_credential("bearer abc").expect("ok"), "abc");
        // `1*SP` — more than one space is legal.
        assert_eq!(bearer_credential("Bearer   abc").expect("ok"), "abc");
        // Trailing `*"="` padding is part of the grammar.
        assert_eq!(bearer_credential("Bearer abc==").expect("ok"), "abc==");

        for malformed in [
            "Bearer",         // no separator, no token
            "Bearer ",        // separator, no token
            "Bearer\tabc",    // a tab is not SP
            "Bearer abc def", // a space inside the token
            "Bearer abc?def", // `?` is outside b64token
            "Bearer a=bc",    // interior padding
            "Basic abc",      // wrong scheme for this parser
        ] {
            assert!(
                matches!(
                    bearer_credential(malformed),
                    Err(AuthError::MalformedRequest(_))
                ),
                "`{malformed}` must be a malformed REQUEST, not a rejected credential",
            );
        }
    }

    /// RFC 6750 §3.1 assigns each outcome its own code, and deliberately assigns
    /// NONE when no credential was presented: an unauthenticated first request
    /// has not made a mistake yet.
    #[test]
    fn challenge_error_codes_follow_rfc6750() {
        assert_eq!(AuthError::MissingCredentials.bearer_error_code(), None);
        assert_eq!(
            AuthError::InvalidCredentials.bearer_error_code(),
            Some("invalid_token")
        );
        assert_eq!(
            AuthError::InvalidToken("x".to_owned()).bearer_error_code(),
            Some("invalid_token")
        );
        assert_eq!(
            AuthError::Forbidden("x".to_owned()).bearer_error_code(),
            Some("insufficient_scope")
        );
        assert_eq!(
            AuthError::MalformedRequest("x".to_owned()).bearer_error_code(),
            Some("invalid_request")
        );
        // A server-side fault is not a statement about the credential.
        assert_eq!(
            AuthError::KeyResolution("x".to_owned()).bearer_error_code(),
            None
        );
    }

    /// RFC 9110 §15.6.4: an unreachable issuer means the server cannot decide,
    /// which is a 503 — never a 401 telling a caller its valid token was
    /// rejected.
    #[test]
    fn unreachable_issuer_is_503_not_401() {
        let api = AuthError::KeyResolution("jwks unreachable".to_owned()).to_api_error();
        assert_eq!(api.status(), http::StatusCode::SERVICE_UNAVAILABLE);
    }

    /// A malformed header is a request defect (400), not a credential decision
    /// (401) — RFC 6750 §3.1 `invalid_request`.
    #[test]
    fn malformed_authorization_header_is_400_invalid_request() {
        let api = AuthError::MalformedRequest("bad".to_owned()).to_api_error();
        assert_eq!(api.status(), http::StatusCode::BAD_REQUEST);
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
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{Argon2, password_hash::PasswordHash};
    use ferroehr::config::auth::{BasicConfig, BasicUser, OidcConfig};

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
                    password_hash: ferroehr::config::secret::Secret::new(hash("pw")),
                    password_hash_file: None,
                    roles: vec!["USER".to_owned()],
                }],
            }),
            oidc: None,
            ..AuthConfig::default()
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
                hmac_secret: Some(ferroehr::config::secret::Secret::new("secret".to_owned())),
                jwks_json: None,
                ..OidcConfig::default()
            }),
            ..AuthConfig::default()
        })
        .unwrap()
    }

    fn headers(auth: &str) -> http::HeaderMap {
        let mut h = http::HeaderMap::new();
        h.insert(header::AUTHORIZATION, HeaderValue::from_str(auth).unwrap());
        h
    }

    #[tokio::test]
    async fn verified_credential_cache_skips_the_kdf_on_a_hit() {
        let auth = basic_only();
        let h = headers("Basic YWxpY2U6cHc="); // alice:pw
        let first = auth.authenticate(&h).await.expect("first verifies");
        let second = auth.authenticate(&h).await.expect("second hits the cache");
        assert!(first.fresh, "the KDF ran → a genuine authentication event");
        assert!(
            !second.fresh,
            "a cache hit is a continuing session, not a new authentication"
        );
        assert_eq!(
            auth.kdf_verifications
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            "one KDF verification for two requests with the same credential"
        );

        // A different credential is a different key — it must NOT hit the
        // cached entry, and the wrong password still fails.
        let bad = headers("Basic YWxpY2U6d3Jvbmc="); // alice:wrong
        auth.authenticate(&bad).await.expect_err("wrong password");
        assert_eq!(
            auth.kdf_verifications
                .load(std::sync::atomic::Ordering::Relaxed),
            2,
            "the wrong credential paid its own (failed) verification"
        );
    }

    #[tokio::test]
    async fn verified_cache_ttl_zero_disables_caching() {
        let mut cfg = AuthConfig {
            enabled: true,
            basic: Some(BasicConfig {
                users: vec![BasicUser {
                    username: "alice".to_owned(),
                    password_hash: ferroehr::config::secret::Secret::new(hash("pw")),
                    password_hash_file: None,
                    roles: vec!["USER".to_owned()],
                }],
            }),
            oidc: None,
            ..AuthConfig::default()
        };
        cfg.verified_cache_ttl_seconds = 0;
        let auth = Authenticator::new(cfg).unwrap();
        let h = headers("Basic YWxpY2U6cHc=");
        auth.authenticate(&h).await.expect("ok");
        auth.authenticate(&h).await.expect("ok");
        assert_eq!(
            auth.kdf_verifications
                .load(std::sync::atomic::Ordering::Relaxed),
            2,
            "TTL 0 verifies every request"
        );
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
        assert_eq!(p.principal.subject, "alice");
        assert!(
            p.fresh,
            "a first Basic verification is a genuine authentication"
        );
    }

    #[tokio::test]
    async fn bearer_without_oidc_is_rejected() {
        let err = basic_only()
            .authenticate(&headers("Bearer sometoken"))
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[tokio::test]
    async fn wrong_password_is_401_not_500() {
        let err = basic_only()
            .authenticate(&headers("Basic YWxpY2U6d3Jvbmc=")) // "alice:wrong"
            .await
            .expect_err("reject");
        assert!(matches!(err, AuthError::InvalidCredentials));
        assert_eq!(err.to_api_error().status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn verification_unavailable_is_500_not_401() {
        // A JoinError from the blocking Argon2 task is a SERVER fault: it must
        // surface as 500 Internal, never as 401 InvalidCredentials — a server
        // failure misreported as a client error is the silent-wrong-answer
        // class this crate's error mapping exists to prevent.
        let err = AuthError::VerificationUnavailable("task panicked".to_owned());
        assert_eq!(
            err.to_api_error().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
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
