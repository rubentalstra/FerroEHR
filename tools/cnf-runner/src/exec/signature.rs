// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Version-signature verification for the SIG-VERSION cases — a portable,
//! deterministic capability (the reference interpreter of a language-agnostic
//! rule).
//!
//! Spec: RM common `master06-change_control_package.adoc` §Digital Signature —
//! a committed `VERSION` is "serialised into canonical form" over "all
//! attributes except signature", then hashed (digest mode) or signed (openPGP
//! mode). openEHR leaves the exact JSON serialisation "an agreed XML, ODIN or
//! other text format", so the framework PINS the agreed signed form:
//!
//!   **canonical form = RFC 8785 (JCS) of the `ORIGINAL_VERSION` ITS-JSON with
//!   the `signature` member removed, UTF-8 bytes.**
//!
//! Grounds: RM common `version.adoc` (`canonical_form`: "all attributes except
//! signature") + ITS-JSON (the canonical JSON representation) + RFC 8785 (byte
//! determinism so any language reproduces the bytes identically). This is a
//! framework-normative pin, not a spec silence.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use base64::Engine as _;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

/// The IXIT-declared signing posture of the SUT (the `signing` block, tagged
/// by `mode`).
///
/// `present`/`equals` are mode-agnostic; `verifiable` dispatches on this.
/// Deserialized directly from the IXIT so the framework tests whatever mode a
/// given deployment runs (RM common master06 §Digital Signature: a deployment
/// runs digest OR openPGP, one at a time).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum SigningMode {
    /// Plain digest (no PKI): the wire form is `<prefix><encoding(hash(bytes))>`,
    /// self-described by the SUT's declared algorithm/encoding/prefix.
    Digest {
        /// The hash algorithm the SUT applies to the canonical bytes.
        algorithm: String,
        /// How the digest is encoded on the wire (e.g. `hex`, `base64`).
        encoding: String,
        /// A fixed prefix the wire form carries before the encoded digest.
        #[serde(default)]
        prefix: String,
    },
    /// openPGP (RFC 4880): a detached armored signature over the canonical
    /// bytes, verified against the declared public key.
    Pgp {
        /// The armored public key the detached signature is verified against.
        public_key: String,
    },
}

/// Reconstruct the agreed signed canonical form from a read-back
/// `ORIGINAL_VERSION` envelope: drop `signature`, then RFC 8785 JCS.
///
/// # Errors
/// [`String`] when the object cannot be JCS-serialised.
pub fn canonical_form(envelope: &Value) -> Result<String, String> {
    let mut value = envelope.clone();
    if let Value::Object(map) = &mut value {
        map.remove("signature");
    }
    serde_jcs::to_string(&value).map_err(|e| format!("canonical-form (jcs): {e}"))
}

/// Whether `signature` verifies over the reconstructed canonical form of
/// `envelope` under `mode`.
///
/// Digest mode recomputes and compares; pgp mode verifies the RFC 4880
/// detached signature against the declared key.
///
/// # Errors
/// [`String`] on a malformed signature/key or an unknown digest algorithm or
/// encoding (an interpreter/artefact defect, never a conformance verdict).
pub fn verify(envelope: &Value, signature: &str, mode: &SigningMode) -> Result<bool, String> {
    let canonical = canonical_form(envelope)?;
    match mode {
        SigningMode::Digest {
            algorithm,
            encoding,
            prefix,
        } => {
            let body = signature.strip_prefix(prefix.as_str()).unwrap_or(signature);
            let recomputed = recompute_digest(canonical.as_bytes(), algorithm, encoding)?;
            Ok(recomputed == body)
        }
        SigningMode::Pgp { public_key } => verify_pgp(canonical.as_bytes(), signature, public_key),
    }
}

/// `<encoding>(<algorithm>(bytes))` — the digest body a digest-mode signature
/// carries (after its self-describing prefix).
fn recompute_digest(bytes: &[u8], algorithm: &str, encoding: &str) -> Result<String, String> {
    let raw: Vec<u8> = match algorithm {
        "sha256" => Sha256::digest(bytes).to_vec(),
        other => return Err(format!("unknown digest algorithm {other:?}")),
    };
    match encoding {
        "base64" => Ok(base64::engine::general_purpose::STANDARD.encode(raw)),
        "base64url" => Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)),
        other => Err(format!("unknown digest encoding {other:?}")),
    }
}

/// Verify an RFC 4880 detached signature over `bytes` against an armored
/// public key.
///
/// # Errors
/// [`String`] on a malformed key/signature or a failed verification.
fn verify_pgp(
    bytes: &[u8],
    signature_armored: &str,
    public_key_armored: &str,
) -> Result<bool, String> {
    use pgp::composed::{Deserializable as _, DetachedSignature, SignedPublicKey};

    let (key, _) = SignedPublicKey::from_string(public_key_armored)
        .map_err(|e| format!("pgp public key: {e}"))?;
    let (sig, _) = DetachedSignature::from_string(signature_armored)
        .map_err(|e| format!("pgp signature: {e}"))?;

    // A signature by a signing-flagged SUBKEY is a signature by the
    // certificate: RFC 9580 §10.1 defines a transferable public key as a
    // primary key plus its subkeys, and §5.2.3.29 key flag 0x02 marks a key as
    // usable to sign data. `rpgp`'s `VerifyingKey for SignedPublicKey` consults
    // `primary_key` alone, so verifying only against it refuses a signature the
    // declared certificate legitimately covers — which is a verifier defect,
    // never a finding about the system under test.
    if sig.verify(&key, bytes).is_ok() {
        return Ok(true);
    }
    Ok(key
        .public_subkeys
        .iter()
        .any(|subkey| sig.verify(subkey, bytes).is_ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn digest_mode() -> SigningMode {
        SigningMode::Digest {
            algorithm: "sha256".to_owned(),
            encoding: "base64".to_owned(),
            prefix: "sha256:".to_owned(),
        }
    }

    #[test]
    fn canonical_form_drops_signature_and_is_jcs() {
        // JCS sorts object keys lexicographically; `signature` is removed.
        let env = json!({ "signature": "sha256:x", "b": 2, "a": 1 });
        assert_eq!(canonical_form(&env).unwrap(), r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn digest_verifies_and_detects_tamper() {
        // Sign-side: compute the digest of the canonical form the way a
        // conformant server must, then confirm verify() accepts it.
        let env = json!({ "_type": "ORIGINAL_VERSION", "data": { "b": 2, "a": 1 } });
        let canonical = canonical_form(&env).unwrap();
        let good = format!(
            "sha256:{}",
            base64::engine::general_purpose::STANDARD.encode(Sha256::digest(canonical.as_bytes()))
        );
        assert!(verify(&env, &good, &digest_mode()).unwrap());
        // A tampered signature body must fail.
        assert!(!verify(&env, "sha256:AAAA", &digest_mode()).unwrap());
        // A tampered envelope (different data) must fail against the old digest.
        let env2 = json!({ "_type": "ORIGINAL_VERSION", "data": { "b": 3, "a": 1 } });
        assert!(!verify(&env2, &good, &digest_mode()).unwrap());
    }

    // A signature made by the certificate's signing SUBKEY must verify.
    //
    // The corpus certificate carries one, the server signs with it by
    // capability, and `rpgp` verifies against the primary key alone — so a
    // verifier written the obvious way reports a conformant server as
    // unverifiable. Three CNF rows failed exactly that way, unnoticed because
    // nothing pinned which key component signs.
    #[test]
    #[expect(
        clippy::panic_in_result_fn,
        reason = "test assertions in the Book's Result-returning test shape"
    )]
    fn subkey_signature_verifies_against_the_certificate() -> Result<(), Box<dyn std::error::Error>>
    {
        use pgp::composed::{
            ArmorOptions, Deserializable as _, DetachedSignature, SignedSecretKey,
        };
        use pgp::crypto::hash::HashAlgorithm;
        use pgp::types::Password;
        use rand::rngs::OsRng;

        let keys = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts/corpus/keys");
        let (secret, _) = SignedSecretKey::from_string(&std::fs::read_to_string(
            keys.join("cnf-signing.sec.asc"),
        )?)?;
        let public_armored = std::fs::read_to_string(keys.join("cnf-signing.pub.asc"))?;

        // Pick the subkey by CAPABILITY, the way the server does — an
        // encryption subkey signing would be a key-usage violation.
        let subkey = secret
            .secret_subkeys
            .iter()
            .find(|sub| sub.signatures.iter().any(|sig| sig.key_flags().sign()))
            .ok_or("the corpus certificate carries no signing subkey")?;

        let data = b"ferroehr-cnf subkey verification";
        let sig = DetachedSignature::sign_binary_data(
            OsRng,
            &subkey.key,
            &Password::empty(),
            HashAlgorithm::Sha256,
            &data[..],
        )?
        .to_armored_string(ArmorOptions::default())?;

        assert!(
            verify_pgp(data, &sig, &public_armored)?,
            "a signature by the certificate's signing subkey must verify"
        );
        // The negative direction: a different payload must still fail, so the
        // subkey fallback cannot pass by accepting everything.
        assert!(!verify_pgp(b"tampered", &sig, &public_armored)?);
        Ok(())
    }

    #[test]
    fn present_is_signature_nonempty() {
        // `present` is evaluated by the driver as a non-empty signature field;
        // this pins the canonical-form contract the driver relies on.
        let env = json!({ "signature": "sha256:abc", "data": {} });
        assert!(
            env.get("signature")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
        );
        assert!(canonical_form(&env).unwrap().contains("data"));
    }
}
