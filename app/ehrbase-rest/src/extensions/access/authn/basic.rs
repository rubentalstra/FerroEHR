//! HTTP Basic authentication against an Argon2 PHC password store.
//!
//! Basic is one of the two Stage-1 mechanisms; the CNF security suites exercise
//! the Basic flow's 401/403 behaviour
//! (`docs/specs/openehr/CNF/tests/platform/robot/SECURITY_TESTS/`), which the
//! authn middleware ([`super`]) enforces per ITS-REST §Authentication and
//! authorization. Password verification uses `argon2` (never a hand-rolled
//! comparison); only the PHC hash is ever stored.

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use http::HeaderValue;

use ehrbase::config::auth::BasicConfig;
use super::{AuthError, AuthMethod, Principal};

/// Verify a `Basic <base64>` credential against the configured user store.
///
/// # Errors
/// [`AuthError::InvalidCredentials`] on a malformed header, unknown user, or a
/// password that does not match the stored Argon2 hash.
pub(super) fn verify(header: &HeaderValue, cfg: &BasicConfig) -> Result<Principal, AuthError> {
    let raw = header.to_str().map_err(|_| AuthError::InvalidCredentials)?;
    let b64 = raw
        .strip_prefix("Basic ")
        .or_else(|| raw.strip_prefix("basic "))
        .ok_or(AuthError::InvalidCredentials)?;
    let decoded = base64_decode(b64.trim()).ok_or(AuthError::InvalidCredentials)?;
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

/// Standard base64 decode (RFC 4648, with padding) without pulling a dep in for
/// one call. Returns `None` on any invalid input.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let s = s.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &c in s {
        let v = u32::from(val(c)?);
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::access::authn::config::BasicUser;
    use argon2::password_hash::{PasswordHasher, SaltString};

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
        let token = base64_encode(format!("{user}:{pw}").as_bytes());
        HeaderValue::from_str(&format!("Basic {token}")).unwrap()
    }

    fn base64_encode(bytes: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
            out.push(T[(n >> 18 & 63) as usize] as char);
            out.push(T[(n >> 12 & 63) as usize] as char);
            out.push(if chunk.len() > 1 {
                T[(n >> 6 & 63) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                T[(n & 63) as usize] as char
            } else {
                '='
            });
        }
        out
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

    #[test]
    fn base64_roundtrip() {
        let enc = base64_encode(b"alice:s3cret");
        assert_eq!(base64_decode(&enc).unwrap(), b"alice:s3cret");
    }
}
