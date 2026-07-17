//! The `[signing]` section — VERSION signing configuration.
//!
//! No openEHR spec governs the configuration surface — our own design. This is
//! a field of the one config tree ([`crate::config::EhrbaseConfig`],
//! `docs/design/configuration.md` §3.11); no loader of its own. The modes it
//! selects are the spec-blessed ones (RM common
//! `master06-change_control_package.adoc` §Digital Signature: `OpenPGP`
//! signature or digest-only integrity check).
//!
//! The passphrase is a shared [`crate::config::secret::Secret`] so it can never leak
//! through a config snapshot (`/management/env`) or a `Debug` log; it has a
//! `*_file` sibling for file-based indirection, resolved by the loader.

use std::path::PathBuf;

use crate::config::secret::Secret;
use serde::{Deserialize, Serialize};

/// The signing mode: a data-integrity digest, or an `OpenPGP` (RFC 4880) digital
/// signature (RM common master06 §Digital Signature).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SigningConfig {
    /// Server-side signing of committed versions. Defaults **on** so the
    /// STANDARD "Signing" capability is demonstrably met.
    pub enabled: bool,
    /// Signing mode: `digest` | `pgp`.
    pub mode: Mode,
    /// Armored RFC 4880 secret key; **required for `pgp` mode** (validated at
    /// boot, fail-closed).
    pub key_path: Option<PathBuf>,
    /// Key passphrase, kept in [`crate::config::secret::Secret`] and redacted from any
    /// config snapshot.
    pub key_passphrase: Option<Secret>,
    /// File-based indirection for [`Self::key_passphrase`] (K8s/Docker secrets).
    /// Exactly one of the pair may be set; the loader reads and trims the file.
    pub key_passphrase_file: Option<PathBuf>,
    /// Read-time verification policy: `off` | `warn` | `strict`.
    pub verify_on_read: VerifyOnRead,
}

impl Default for SigningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: Mode::default(),
            key_path: None,
            key_passphrase: None,
            key_passphrase_file: None,
            verify_on_read: VerifyOnRead::default(),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn defaults_sign_on_digest_verify_off() {
        let c = SigningConfig::default();
        assert!(c.enabled);
        assert_eq!(c.mode, Mode::Digest);
        assert_eq!(c.verify_on_read, VerifyOnRead::Off);
        assert!(c.key_path.is_none());
    }

    #[test]
    fn debug_does_not_leak_passphrase() {
        let c = SigningConfig {
            key_passphrase: Some(Secret::new("top-secret-value")),
            ..SigningConfig::default()
        };
        let dbg = format!("{c:?}");
        assert!(
            !dbg.contains("top-secret-value"),
            "passphrase leaked: {dbg}"
        );
    }
}
