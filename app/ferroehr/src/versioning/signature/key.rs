// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

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
    /// Index into `secret.secret_subkeys` of the signing-capable subkey, when
    /// the certificate carries one.
    ///
    /// Signing with a subkey is what makes rotation cheap: the operator issues
    /// a new signing subkey, the certificate retains the old one, and history
    /// keeps verifying against the same configured key with no retired-keyring
    /// entry. An index rather than a clone because the subkey is not `Clone`,
    /// and it stays valid for the lifetime of the owned certificate.
    signing_subkey: Option<usize>,
    public: SignedPublicKey,
    /// Certificates retired from signing and retained for verification only, so
    /// versions signed before a rotation keep verifying. Public keys by
    /// construction: a retired certificate can never sign again.
    retired: Vec<SignedPublicKey>,
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
        retired_paths: &[std::path::PathBuf],
    ) -> Result<Self, KeyError> {
        let armored = std::fs::read_to_string(path).map_err(|source| KeyError::Read {
            path: path.display().to_string(),
            source,
        })?;
        let mut retired = Vec::with_capacity(retired_paths.len());
        for retired_path in retired_paths {
            let armored =
                std::fs::read_to_string(retired_path).map_err(|source| KeyError::Read {
                    path: retired_path.display().to_string(),
                    source,
                })?;
            let (certificate, _headers) =
                SignedPublicKey::from_string(&armored).map_err(KeyError::Parse)?;
            retired.push(certificate);
        }
        Self::from_parts(&armored, passphrase, retired)
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
        Self::from_parts(armored, passphrase, Vec::new())
    }

    /// [`Self::from_armored`] plus the verify-only certificates retired from
    /// signing.
    ///
    /// # Errors
    /// As [`Self::from_armored`].
    pub fn from_parts(
        armored: &str,
        passphrase: Option<&crate::config::secret::Secret>,
        retired: Vec<SignedPublicKey>,
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
        // RFC 9580 §5.2.3.29 key flag 0x02 ("this key may be used to sign
        // data"): the subkey is chosen by CAPABILITY, never by position — an
        // encryption subkey signing would be a key-usage violation a strict
        // verifier should reject.
        let signing_subkey = secret
            .secret_subkeys
            .iter()
            .position(|subkey| subkey.signatures.iter().any(|sig| sig.key_flags().sign()));
        let key = Self {
            secret,
            signing_subkey,
            public,
            retired,
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
        // A certificate with no signing subkey signs with the primary key, as
        // before — an upgrade changes nothing for a single-key deployment.
        let sig = match self
            .signing_subkey
            .and_then(|index| self.secret.secret_subkeys.get(index))
        {
            Some(subkey) => {
                DetachedSignature::sign_binary_data(OsRng, &subkey.key, &self.password, HASH, data)
            }
            None => DetachedSignature::sign_binary_data(
                OsRng,
                &self.secret.primary_key,
                &self.password,
                HASH,
                data,
            ),
        }
        .map_err(PgpSignError)?;
        sig.to_armored_string(ArmorOptions::default())
            .map_err(PgpSignError)
    }

    /// Verify an armored detached signature over `data` against the public key.
    pub(crate) fn verify(&self, data: &[u8], armored_sig: &str) -> PgpVerdict {
        let Ok((sig, _headers)) = DetachedSignature::from_string(armored_sig) else {
            return PgpVerdict::Malformed;
        };
        for certificate in std::iter::once(&self.public).chain(&self.retired) {
            if verify_against_certificate(&sig, certificate, data) {
                return PgpVerdict::Valid;
            }
        }
        PgpVerdict::Invalid
    }
}

/// Verify `sig` against every component of one certificate — the primary key
/// and each of its subkeys.
///
/// The subkeys are not an optimisation: RFC 9580 §10.1 defines a transferable
/// public key as a primary key plus its subkeys, and rotating the *signing
/// subkey* is the mechanism `OpenPGP` provides for rotation without discarding
/// the primary key's accumulated trust. `rpgp`'s `VerifyingKey for
/// SignedPublicKey` consults the primary key alone, so a signature made by any
/// subkey would be rejected by a certificate that legitimately contains it.
fn verify_against_certificate(
    sig: &DetachedSignature,
    certificate: &SignedPublicKey,
    data: &[u8],
) -> bool {
    if sig.verify(certificate, data).is_ok() {
        return true;
    }
    certificate
        .public_subkeys
        .iter()
        .any(|subkey| sig.verify(subkey, data).is_ok())
}
