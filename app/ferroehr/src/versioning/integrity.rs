//! Digital-signature integrity at commit and read.
//!
//! Spec: RM common `master06-change_control_package.adoc` §Digital Signature +
//! BASE arch-overview `master07-security.adoc` §Integrity. The signature is
//! computed over the version's canonical serialized form with the signature
//! attribute Void, and is a stored fact carried with the data (for Extracts).
//! This module holds the *policy* — sign the assembled `ORIGINAL_VERSION` at
//! commit, and (optionally) recompute-and-compare at read; the primitives live
//! in [`signature`](super::signature).

use serde_json::Value;
use uuid::Uuid;

use crate::ids::VoId;
use crate::service::error::ServiceError;
use crate::versioning::SigningCtx;
use crate::versioning::audit::AuditInput;
use crate::versioning::object_version_id::TreeId;
use crate::versioning::signature::config::VerifyOnRead;
use crate::versioning::signature::signer::Signer;
use crate::versioning::wire::build_original_version;

/// Compute the server-generated `VERSION.signature` for a version about to be
/// persisted (RM common master06 §Digital Signature). The caller
/// ([`crate::versioning::change`]) decides client-vs-server first: a
/// client-supplied signature is stored verbatim and never reaches here.
///
/// When signing is enabled, the fully-assembled `ORIGINAL_VERSION` — the
/// *exact* value that will later be served (built by the shared
/// [`build_original_version`] so commit-time and read-time bytes match,
/// **including `other_input_version_uids` merge provenance**, which is part of
/// the committed version) — is signed over its `canonical_form()` (the signature
/// attribute Void during serialization). Attestations are NOT signed: they
/// arrive after committal (master06 §Attestation) and are appended after
/// verification, so they are excluded here exactly as at read time.
///
/// # Errors
/// [`ServiceError::Signing`] when the canonical form cannot be produced or the
/// `OpenPGP` signer fails (digest signing is infallible); the
/// [`build_original_version`] rejection of a commit audit whose committer is
/// not a canonical `PARTY_PROXY`.
#[expect(
    clippy::too_many_arguments,
    reason = "the parts of an ORIGINAL_VERSION plus the signing context; a \
              parameter struct would not read clearer at the call sites"
)]
pub(crate) fn sign_version(
    ctx: &SigningCtx<'_>,
    audit: &AuditInput,
    time_committed: jiff::Timestamp,
    vo_id: VoId,
    tree: TreeId,
    preceding_uid: Option<&str>,
    other_input_version_uids: &[String],
    contribution_id: Uuid,
    lifecycle_state: &str,
    data: &Value,
) -> Result<Option<String>, ServiceError> {
    if !ctx.signer.enabled() {
        return Ok(None);
    }
    let ov = build_original_version(
        &ctx.system_id,
        vo_id,
        tree,
        preceding_uid,
        other_input_version_uids,
        contribution_id,
        audit,
        &time_committed,
        lifecycle_state,
        data,
        None,
    )?;
    let canonical = openehr_rm::common::change_control::version_impl::canonical_form_of_json(&ov)
        .map_err(|e| ServiceError::Signing(e.to_string()))?;
    let signature = ctx
        .signer
        .sign(&canonical)
        .map_err(|e| ServiceError::Signing(e.to_string()))?;
    Ok(Some(signature))
}

/// Read-time signature verification (RM common master06 §Digital Signature —
/// the signature verifies the served version against its recomputed
/// `canonical_form`). Applies ONLY to signatures **this server generated**: a
/// `client_supplied` signature is stored verbatim and never re-verified (the
/// author may have signed another agreed serialization we cannot recompute —
/// master06 §Digital Signature / §Copying). Otherwise a no-op when
/// `verify_on_read = off` or the version carries no signature. A `warn`
/// mismatch logs + meters (`version_signature_invalid_total`); a `strict`
/// mismatch is a 5xx integrity failure.
///
/// # Errors
/// [`ServiceError::Signing`] when `verify_on_read = strict` and the stored
/// server signature fails verification, or (in any non-`off` mode) when the
/// served version's canonical form cannot be recomputed.
pub(crate) fn verify_on_read(
    signer: &Signer,
    ov: &Value,
    signature: Option<&str>,
    client_supplied: bool,
) -> Result<(), ServiceError> {
    // Client-supplied signatures are foreign facts stored verbatim — never our
    // canonical form to recompute (master06 §Digital Signature).
    if client_supplied || signer.verify_on_read() == VerifyOnRead::Off {
        return Ok(());
    }
    let Some(signature) = signature else {
        return Ok(());
    };
    let canonical = openehr_rm::common::change_control::version_impl::canonical_form_of_json(ov)
        .map_err(|e| ServiceError::Signing(e.to_string()))?;
    let verdict = signer.verify(&canonical, signature);
    if verdict.is_failure() {
        metrics::counter!(
            crate::telemetry::prometheus::VERSION_SIGNATURE_INVALID,
            "verdict" => verdict.label(),
        )
        .increment(1);
        tracing::error!(
            verdict = verdict.label(),
            "version signature failed verification (verify_on_read)"
        );
        if signer.verify_on_read() == VerifyOnRead::Strict {
            return Err(ServiceError::Signing(format!(
                "stored version signature does not verify ({})",
                verdict.label()
            )));
        }
    }
    Ok(())
}
