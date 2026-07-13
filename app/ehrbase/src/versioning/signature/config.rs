//! Signing configuration ([`SigningConfig`]) — a `figment`-loaded serde struct.
//!
//! No openEHR spec governs the configuration surface — our own design. Env keys
//! are the `EHRBASE_SIGNING_`-prefixed set. Loading mirrors the other configs:
//! serde defaults ← optional TOML file (`EHRBASE_SIGNING_CONFIG`) ←
//! `EHRBASE_SIGNING_`-prefixed environment (nested keys via `__`).
//!
//! The modes it selects are the spec-blessed ones (RM common
//! `master06-change_control_package.adoc` §Digital Signature: `OpenPGP` signature
//! or digest-only integrity check).
//!
//! `SigningConfig` intentionally does **not** derive `Serialize`: the
//! passphrase is a [`secrecy::SecretString`] so it can never leak through a
//! config snapshot (`/management/env`) or a `Debug` log.

use std::path::PathBuf;

use figment::Figment;
use figment::providers::{Env, Format, Toml};
use secrecy::SecretString;
use serde::Deserialize;

/// The signing mode: a data-integrity digest, or an `OpenPGP` (RFC 4880) digital
/// signature (RM common master06 §Digital Signature).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// `radix-64(SHA-256(canonical_form))` — a data-integrity check, no key
    /// management (master07 §Digital Signature: "the encryption step might be
    /// omitted, resulting in a digest only").
    #[default]
    Digest,
    /// An RFC 4880 detached signature over the canonical form with a
    /// server-held `OpenPGP` private key — authentication + non-repudiation.
    Pgp,
}

/// Recompute-and-compare policy at read/reassembly time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VerifyOnRead {
    /// Serve the stored signature untouched (RM common master06 §Digital
    /// Signature models it as a stored fact — S-44).
    #[default]
    Off,
    /// Log + meter a mismatch (`version_signature_invalid_total`); still serve.
    Warn,
    /// A mismatch is a 5xx integrity failure (the record is provably corrupt).
    Strict,
}

/// Version-signing configuration. Every field has a default, so an all-defaults
/// value is valid: signing **on**, `digest` mode, verification off.
#[derive(Debug, Clone, Deserialize)]
pub struct SigningConfig {
    /// Server-side signing of committed versions (`EHRBASE_SIGNING_ENABLED`).
    /// Defaults **on** so the STANDARD "Signing" capability is demonstrably met.
    #[serde(default = "defaults::enabled")]
    pub enabled: bool,
    /// Signing mode (`EHRBASE_SIGNING_MODE`): `digest` | `pgp`.
    #[serde(default)]
    pub mode: Mode,
    /// Armored RFC 4880 secret key (`EHRBASE_SIGNING_KEY_PATH`); required for
    /// `pgp` mode.
    #[serde(default)]
    pub key_path: Option<PathBuf>,
    /// Key passphrase (`EHRBASE_SIGNING_KEY_PASSPHRASE`), kept in `secrecy` and
    /// redacted from any config snapshot.
    #[serde(default)]
    pub key_passphrase: Option<SecretString>,
    /// Read-time verification policy (`EHRBASE_SIGNING_VERIFY_ON_READ`):
    /// `off` | `warn` | `strict`.
    #[serde(default)]
    pub verify_on_read: VerifyOnRead,
}

impl Default for SigningConfig {
    fn default() -> Self {
        Self {
            enabled: defaults::enabled(),
            mode: Mode::default(),
            key_path: None,
            key_passphrase: None,
            verify_on_read: VerifyOnRead::default(),
        }
    }
}

impl SigningConfig {
    /// Load configuration: serde defaults, then an optional TOML file (path in
    /// `EHRBASE_SIGNING_CONFIG`), then `EHRBASE_SIGNING_`-prefixed environment
    /// variables (nested keys use `__`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::new();
        if let Ok(path) = std::env::var("EHRBASE_SIGNING_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_SIGNING_").split("__"))
            .extract()
    }
}

mod defaults {
    pub(super) const fn enabled() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::result_large_err)]

    use super::*;
    use figment::Jail;
    use secrecy::ExposeSecret as _;

    #[test]
    fn defaults_sign_on_digest_verify_off() {
        let c = SigningConfig::default();
        assert!(c.enabled);
        assert_eq!(c.mode, Mode::Digest);
        assert_eq!(c.verify_on_read, VerifyOnRead::Off);
        assert!(c.key_path.is_none());
    }

    #[test]
    fn env_overrides_apply() {
        Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_SIGNING_ENABLED", "false");
            jail.set_env("EHRBASE_SIGNING_MODE", "pgp");
            jail.set_env("EHRBASE_SIGNING_KEY_PATH", "/etc/ehrbase/signing.asc");
            jail.set_env("EHRBASE_SIGNING_KEY_PASSPHRASE", "s3cret");
            jail.set_env("EHRBASE_SIGNING_VERIFY_ON_READ", "strict");
            let c = SigningConfig::load().unwrap();
            assert!(!c.enabled);
            assert_eq!(c.mode, Mode::Pgp);
            assert_eq!(
                c.key_path.as_deref().unwrap().to_str().unwrap(),
                "/etc/ehrbase/signing.asc"
            );
            assert_eq!(c.key_passphrase.unwrap().expose_secret(), "s3cret");
            assert_eq!(c.verify_on_read, VerifyOnRead::Strict);
            Ok(())
        });
    }

    #[test]
    fn empty_env_yields_defaults() {
        Jail::expect_with(|_jail| {
            let c = SigningConfig::load().unwrap();
            assert!(c.enabled);
            assert_eq!(c.mode, Mode::Digest);
            Ok(())
        });
    }

    #[test]
    fn debug_does_not_leak_passphrase() {
        let c = SigningConfig {
            key_passphrase: Some(SecretString::from("top-secret-value")),
            ..SigningConfig::default()
        };
        let dbg = format!("{c:?}");
        assert!(
            !dbg.contains("top-secret-value"),
            "passphrase leaked: {dbg}"
        );
    }
}
