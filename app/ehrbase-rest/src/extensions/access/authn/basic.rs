//! HTTP Basic authentication against an Argon2 PHC password store.
//!
//! Basic is one of the two Stage-1 mechanisms; the CNF security suites exercise
//! the Basic flow's 401/403 behaviour
//! (`docs/specs/openehr/CNF/tests/platform/robot/SECURITY_TESTS/`), which the
//! authn middleware ([`super`]) enforces per ITS-REST §Authentication and
//! authorization. Password verification uses `argon2` (never a hand-rolled
//! comparison); only the PHC hash is ever stored.

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine as _;
use base64::alphabet;
use base64::engine::general_purpose::GeneralPurposeConfig;
use base64::engine::{DecodePaddingMode, GeneralPurpose};
use http::HeaderValue;

use super::{AuthError, AuthMethod, Principal};
use ehrbase::config::auth::BasicConfig;

/// RFC 4648 standard-alphabet decoder for the RFC 7617 `Basic` credential,
/// padding-indifferent: canonical padded output is the RFC form, but clients
/// that omit padding are accepted (the previous decoder did, and RFC 7617
/// gives no reason to reject them). Non-alphabet bytes, interior padding,
/// and non-canonical trailing bits are rejected.
static BASIC_B64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// Verify a `Basic <base64>` credential against the configured user store.
///
/// # Errors
/// [`AuthError::InvalidCredentials`] on a malformed header, unknown user, or a
/// password that does not match the stored Argon2 hash.
#[expect(
    clippy::map_err_ignore,
    reason = "every failure on the credential path collapses to one opaque outcome \
              deliberately: a caller must not learn from the 401 whether the header \
              was malformed, the user unknown, or the password wrong"
)]
pub(super) fn verify(header: &HeaderValue, cfg: &BasicConfig) -> Result<Principal, AuthError> {
    let raw = header.to_str().map_err(|_| AuthError::InvalidCredentials)?;
    let b64 = raw
        .strip_prefix("Basic ")
        .or_else(|| raw.strip_prefix("basic "))
        .ok_or(AuthError::InvalidCredentials)?;
    let decoded = BASIC_B64
        .decode(b64.trim())
        .map_err(|_| AuthError::InvalidCredentials)?;
    let decoded = String::from_utf8(decoded).map_err(|_| AuthError::InvalidCredentials)?;
    let (username, password) = decoded
        .split_once(':')
        .ok_or(AuthError::InvalidCredentials)?;

    let user = cfg
        .users
        .iter()
        .find(|u| u.username == username)
        .ok_or(AuthError::InvalidCredentials)?;

    let parsed = PasswordHash::new(user.password_hash.expose())
        .map_err(|_| AuthError::InvalidCredentials)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AuthError::InvalidCredentials)?;

    // Basic-user roles are configured (default `["USER"]`); normalize to
    // upper-case so they match the RBAC role model regardless of config casing.
    let roles = user
        .roles
        .iter()
        .map(|r| r.trim().to_ascii_uppercase())
        .filter(|r| !r.is_empty())
        .collect();

    Ok(Principal {
        subject: username.to_owned(),
        scopes: Vec::new(),
        roles,
        // Basic auth carries no JWT claims (ABAC under Basic is rejected).
        claims: serde_json::Map::new(),
        method: AuthMethod::Basic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::password_hash::{PasswordHasher, SaltString};
    use ehrbase::config::auth::BasicUser;

    fn hash(pw: &str) -> String {
        // Fixed salt keeps the test hermetic without the argon2 `rand` feature.
        let salt = SaltString::from_b64("MTIzNDU2Nzg5MDEyMzQ1Ng").expect("salt");
        Argon2::default()
            .hash_password(pw.as_bytes(), &salt)
            .expect("hash")
            .to_string()
    }

    fn store() -> BasicConfig {
        BasicConfig {
            users: vec![BasicUser {
                username: "alice".to_owned(),
                password_hash: ehrbase::config::secret::Secret::new(hash("s3cret")),
                roles: vec!["user".to_owned()],
            }],
        }
    }

    fn header(user: &str, pw: &str) -> HeaderValue {
        let token = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pw}"));
        HeaderValue::from_str(&format!("Basic {token}")).unwrap()
    }

    fn raw_header(token: &str) -> HeaderValue {
        HeaderValue::from_str(&format!("Basic {token}")).unwrap()
    }

    #[test]
    fn valid_credentials_authenticate() {
        let p = verify(&header("alice", "s3cret"), &store()).expect("ok");
        assert_eq!(p.subject, "alice");
        assert_eq!(p.method, AuthMethod::Basic);
    }

    #[test]
    fn configured_roles_are_upper_cased() {
        // The configured lower-case `user` role surfaces normalized on the
        // Principal (§5.1 — Basic role config, upper-casing).
        let p = verify(&header("alice", "s3cret"), &store()).expect("ok");
        assert_eq!(p.roles, vec!["USER".to_owned()]);
        assert!(p.claims.is_empty(), "Basic carries no JWT claims");
    }

    #[test]
    fn admin_user_role_config() {
        let cfg = BasicConfig {
            users: vec![BasicUser {
                username: "root".to_owned(),
                password_hash: ehrbase::config::secret::Secret::new(hash("s3cret")),
                roles: vec!["ADMIN".to_owned()],
            }],
        };
        let p = verify(&header("root", "s3cret"), &cfg).expect("ok");
        assert_eq!(p.roles, vec!["ADMIN".to_owned()]);
    }

    #[test]
    fn wrong_password_rejected() {
        let err = verify(&header("alice", "nope"), &store()).expect_err("reject");
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[test]
    fn unknown_user_rejected() {
        let err = verify(&header("bob", "s3cret"), &store()).expect_err("reject");
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    // Decode-boundary pins for the RFC 7617 credential (401 in every reject
    // case — the opaque-outcome discipline of `verify`).

    #[test]
    fn unpadded_credential_accepted() {
        // "alice:s3cret" canonical is "YWxpY2U6czNjcmV0" (len % 4 == 0, no
        // padding involved); "alice:s3cret1" pads to "…MQ==" — strip it.
        let padded = base64::engine::general_purpose::STANDARD.encode("alice:s3cret");
        let unpadded = padded.trim_end_matches('=').to_owned();
        let p = verify(&raw_header(&unpadded), &store()).expect("unpadded canonical accepted");
        assert_eq!(p.subject, "alice");
    }

    #[test]
    fn non_alphabet_byte_rejected() {
        let err = verify(&raw_header("YWxpY2U6*zNjcmV0"), &store()).expect_err("reject");
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[test]
    fn excess_padding_rejected() {
        // Deliberate strictness increase over the retired hand-rolled decoder,
        // which trimmed ANY number of trailing '='. Canonical encoders never
        // emit this, and garbage credentials were a 401 anyway — the wire
        // outcome is unchanged.
        let err = verify(&raw_header("QQ==="), &store()).expect_err("reject");
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[test]
    fn interior_padding_rejected() {
        let err = verify(&raw_header("YWx=pY2U6czNjcmV0"), &store()).expect_err("reject");
        assert!(matches!(err, AuthError::InvalidCredentials));
    }
}
