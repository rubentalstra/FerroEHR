//! Digital-signature integrity at commit and read (S-15, S-40, S-44).
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

use crate::service::ServiceError;
use crate::versioning::audit::AuditInput;
use crate::versioning::object_version_id::TreeId;
use crate::versioning::revision_history::build_original_version;
use crate::versioning::signature::{Signer, VerifyOnRead};
use crate::versioning::{Committed, SigningCtx};

/// Compute the `VERSION.signature` for a version about to be persisted (RM
/// common master06 §Digital Signature).
///
/// A **client-supplied** signature (from the CONTRIBUTION `UPDATE_VERSION` path)
/// wins and is stored verbatim — never re-signed, never validated against our
/// canonical form (the author may use another agreed serialization, master06
/// §Digital Signature; S-44). Otherwise, when signing is enabled, the
/// fully-assembled `ORIGINAL_VERSION` — the *exact* value that will later be
/// served (built by the shared [`build_original_version`] so commit-time and
/// read-time bytes match) — is signed over its `canonical_form()` (the
/// signature attribute Void during serialization, S-40).
#[allow(clippy::too_many_arguments)] // the parts of an ORIGINAL_VERSION + signing context
fn sign_version(
    ctx: &SigningCtx<'_>,
    audit: &AuditInput,
    time_committed: jiff::Timestamp,
    vo_id: Uuid,
    tree: TreeId,
    preceding_uid: Option<&str>,
    contribution_id: Uuid,
    lifecycle_state: &str,
    data: &Value,
    client_signature: Option<String>,
) -> Result<Option<String>, ServiceError> {
    if let Some(sig) = client_signature {
        return Ok(Some(sig));
    }
    if !ctx.signer.enabled() {
        return Ok(None);
    }
    let ov = build_original_version(
        &ctx.system_id,
        vo_id,
        tree,
        preceding_uid,
        &[],
        contribution_id,
        audit,
        &time_committed,
        lifecycle_state,
        data,
        None,
    );
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
/// `canonical_form`). No-op when `verify_on_read = off` or the version carries
/// no signature. A `warn` mismatch logs + meters; a `strict` mismatch is a 5xx
/// integrity failure.
///
/// # Errors
/// [`ServiceError::Signing`] only when `verify_on_read = strict` and the stored
/// signature fails verification.
fn verify_on_read(
    signer: &Signer,
    ov: &Value,
    signature: Option<&str>,
) -> Result<(), ServiceError> {
    if signer.verify_on_read() == VerifyOnRead::Off {
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

impl Committed {
    /// The per-version entry for the PHI-free event-outbox envelope: identity +
    /// provenance metadata only, never clinical content.
    ///
    /// PORT NOTE: no openEHR spec governs eventing — our own extension. The
    /// outbox row is written inside the commit transaction by storage; this
    /// only builds the payload (README cross-ruling: extensions build payloads).
    fn envelope_entry(&self) -> Value {
        serde_json::json!({
            "vo_id": self.vo_id,
            "kind": self.kind.as_str(),
            "sys_version": self.sys_version,
            "version_tree_id": self.tree.to_string(),
            "change_type": self.change_type,
            "template_id": self.template_id,
        })
    }
}
