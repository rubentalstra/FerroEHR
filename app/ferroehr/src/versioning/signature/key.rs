//! `OpenPGP` key loading + the rPGP detached-signature primitives.
//!
//! Spec: RM common `master06-change_control_package.adoc` §Digital Signature —
//! the signature "is generated according to the openPGP standard (IETF RFC
//! 4880)". NOTE (master06 §Digital Signature): an RFC 4880 signature
//! internally hashes the signed data, so a detached signature over the
//! `canonical_form` bytes is the standard-conformant realisation of the spec's
//! "digital signature … created from the hash".

use std::path::Path;

use pgp::composed::{
    ArmorOptions, Deserializable, DetachedSignature, SignedPublicKey, SignedSecretKey,
};
use pgp::crypto::hash::HashAlgorithm;
use pgp::types::Password;
use rand::rngs::OsRng;

/// The hash algorithm used for `OpenPGP` signatures (SHA-256, matching the digest
/// mode; RM common master06 §Digital Signature leaves the algorithm to the
/// format's self-description).
const HASH: HashAlgorithm = HashAlgorithm::Sha256;

/// A failure loading or exercising the configured `OpenPGP` signing key.
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// The armored key file could not be read.
    #[error("reading OpenPGP key file {path}")]
    Read {
        /// The path that failed.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The file is not a parseable armored RFC 4880 secret key.
    #[error("parsing armored OpenPGP secret key failed")]
    Parse(#[source] pgp::errors::Error),
    /// The key cannot produce a signature (wrong passphrase, or a non-signing
    /// key) — the fail-closed boot check.
    #[error("the configured OpenPGP key cannot sign (wrong passphrase or non-signing key)")]
    Unusable(#[from] PgpSignError),
}

/// A failure producing a detached `OpenPGP` signature at runtime.
#[derive(Debug, thiserror::Error)]
#[error("producing the detached OpenPGP signature failed")]
pub struct PgpSignError(#[from] pgp::errors::Error);

/// The structural outcome of verifying a PGP-armored signature against the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PgpVerdict {
    /// A valid detached signature by the configured key over the content.
    Valid,
    /// A well-formed detached signature that does not verify.
    Invalid,
    /// The armor could not be parsed as a detached signature.
    Malformed,
}

/// A loaded `OpenPGP` signing key: the armored secret key (which also carries the
/// public key) plus the passphrase that unlocks it. Never logged/serialised.
pub struct PgpKey {
    secret: SignedSecretKey,
    public: SignedPublicKey,
    password: Password,
}

impl std::fmt::Debug for PgpKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render key material or the passphrase.
        f.debug_struct("PgpKey").finish_non_exhaustive()
    }
}

impl PgpKey {
    /// Load an armored RFC 4880 secret key from `path`, unlocked by `passphrase`.
    /// Performs the fail-closed boot check (a test signature).
    ///
    /// # Errors
    /// [`KeyError::Read`] / [`KeyError::Parse`] / [`KeyError::Unusable`].
    pub fn load(
        path: &Path,
        passphrase: Option<&crate::config::secret::Secret>,
    ) -> Result<Self, KeyError> {
        let armored = std::fs::read_to_string(path).map_err(|source| KeyError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_armored(&armored, passphrase)
    }

    /// Load an armored RFC 4880 secret key from an in-memory string (used by
    /// tests).
    ///
    /// # Errors
    /// [`KeyError::Parse`] if the armor is not a secret key; [`KeyError::Unusable`]
    /// if the boot check signature fails.
    pub fn from_armored(
        armored: &str,
        passphrase: Option<&crate::config::secret::Secret>,
    ) -> Result<Self, KeyError> {
        let (secret, _headers) = SignedSecretKey::from_string(armored).map_err(KeyError::Parse)?;
        let public = secret.to_public_key();
        // The secret is exposed HERE and nowhere earlier: `pgp`'s `Password` is
        // the one consumer that must have the plaintext, so the value stays
        // inside its `secrecy`-backed wrapper — which zeroizes on drop — until
        // this line. Taking it as a `&str` at the boundary would leave an
        // ordinary copy in freed memory afterwards, which is the guarantee the
        // crate is pinned to provide.
        let password = match passphrase {
            Some(secret) => Password::from(secret.expose()),
            None => Password::empty(),
        };
        let key = Self {
            secret,
            public,
            password,
        };
        // Fail-closed: a real signature proves the passphrase unlocks the key
        // and it can actually sign.
        key.sign(b"ferroehr-signing boot check")?;
        Ok(key)
    }

    /// Produce an ASCII-armored RFC 4880 detached signature over `data`.
    ///
    /// # Errors
    /// [`PgpSignError`] if signing or armoring fails.
    pub fn sign(&self, data: &[u8]) -> Result<String, PgpSignError> {
        let sig = DetachedSignature::sign_binary_data(
            OsRng,
            &self.secret.primary_key,
            &self.password,
            HASH,
            data,
        )
        .map_err(PgpSignError)?;
        sig.to_armored_string(ArmorOptions::default())
            .map_err(PgpSignError)
    }

    /// Verify an armored detached signature over `data` against the public key.
    pub(crate) fn verify(&self, data: &[u8], armored_sig: &str) -> PgpVerdict {
        let Ok((sig, _headers)) = DetachedSignature::from_string(armored_sig) else {
            return PgpVerdict::Malformed;
        };
        if sig.verify(&self.public, data).is_ok() {
            PgpVerdict::Valid
        } else {
            PgpVerdict::Invalid
        }
    }
}
