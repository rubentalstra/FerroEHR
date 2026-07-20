//! Attestation: attaching an `ATTESTATION` to an `ORIGINAL_VERSION` at or after
//! committal.
//!
//! Spec: RM common `master06-change_control_package.adoc` §Attestation + RM
//! common `master04-generic_package.adoc` §Attestation. An `ATTESTATION` is an
//! `AUDIT_DETAILS` subtype (`items?`, `reason`, `proof`, `is_pending`); it "can
//! be added at any time after committal" and a `666|attestation|` member of a
//! CONTRIBUTION adds **no** new version. Attestations of an old version are not
//! valid for a new version (they are keyed to `(vo_id, sys_version)` and never
//! copied forward), and they are not part of the version's signed canonical
//! form (added after signing).

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::service::error::ServiceError;
use crate::versioning::Kind;
use crate::versioning::audit::{audit_details, change_type};
use crate::versioning::change::Committed;
use crate::versioning::object_version_id::TreeId;

/// A `666|attestation|` of an **existing** `ORIGINAL_VERSION` committed within a
/// CONTRIBUTION (master06 §Contributions — adds no new version). Carried
/// alongside the change set so it commits in the same transaction.
pub(crate) struct PendingAttest {
    pub(crate) vo_id: VoId,
    pub(crate) kind: Kind,
    /// The target version to attest (from `preceding_version_uid` — trunk or
    /// branch).
    pub(crate) expected: TreeId,
    /// The wire `UPDATE_ATTESTATION` partial, completed into a full RM
    /// `ATTESTATION` at commit time.
    pub(crate) partial: Value,
}

/// Attach an `ATTESTATION` to an **existing** `ORIGINAL_VERSION` (a
/// `666|attestation|` version item; master06 §Contributions — no new version,
/// `sys_period` untouched). Realizes `VERSIONED_OBJECT.commit_attestation`
/// precondition `has_version_id` (master06 §Versioned Objects). `attestation`
/// is the already-completed full RM `ATTESTATION`.
///
/// # Errors
/// [`ServiceError::NotFound`] when the target `(vo_id, tree, kind)` does not
/// exist or does not belong to `ehr_id`; the storage errors of the target
/// lookup / attestation insert.
#[allow(clippy::too_many_arguments)] // the parts of an attestation act + its commit instant
pub(crate) async fn attest(
    tx: &mut PgConnection,
    ehr_id: Option<EhrId>,
    vo_id: VoId,
    kind: Kind,
    expected: TreeId,
    attestation: &Value,
    contribution_id: Uuid,
    time_committed: jiff::Timestamp,
) -> Result<Committed, ServiceError> {
    // The target lookup (`version_repo::attestation::attestation_target`) yields the owning
    // EHR (compared against the caller's), the storage ordinal the attestation
    // keys to, and the target's `creating_system_id` (carried into the outbox).
    let target = crate::storage::version_repo::attestation::attestation_target(
        tx,
        vo_id,
        expected.columns(),
        kind.as_str(),
    )
    .await?;
    let Some(target) = target.filter(|t| t.ehr_id == ehr_id) else {
        return Err(ServiceError::NotFound(format!(
            "{} version {vo_id}::{expected}",
            kind.as_str()
        )));
    };
    crate::storage::version_repo::attestation::insert_attestation(
        tx,
        vo_id,
        target.sys_version,
        contribution_id,
        attestation,
    )
    .await?;
    Ok(Committed {
        vo_id,
        sys_version: target.sys_version,
        tree: expected,
        creating_system_id: target.creating_system_id,
        kind,
        // A 666 attestation adds no new version; it is announced in the
        // contribution's outbox envelope as a change to the existing version.
        change_type: change_type::ATTESTATION.to_owned(),
        template_id: None,
        // The contribution's commit-act time — a 666 attestation adds no new
        // version, so this is the instant the attestation itself committed.
        time_committed,
    })
}

/// Complete + persist the attestations committed together with a NEW version
/// (`UPDATE_VERSION.attestations`; master06 §Attestation "Signing content at
/// committal"). Each partial `UPDATE_ATTESTATION` is completed into a full RM
/// `ATTESTATION` and attached to the just-written version — same transaction.
///
/// # Errors
/// The [`complete_attestation`] `Unprocessable` rejections; the storage error
/// of the attestation insert.
#[allow(clippy::too_many_arguments)] // the parts of an ATTESTATION + its target version
pub(crate) async fn insert_accompanying_attestations(
    tx: &mut PgConnection,
    vo_id: VoId,
    sys_version: i32,
    contribution_id: Uuid,
    system_id: &str,
    committer_fallback: &Value,
    now: jiff::Timestamp,
    partials: &[Value],
) -> Result<(), ServiceError> {
    for partial in partials {
        let full = complete_attestation(partial, system_id, committer_fallback, now)?;
        crate::storage::version_repo::attestation::insert_attestation(
            tx,
            vo_id,
            sys_version,
            contribution_id,
            &full,
        )
        .await?;
    }
    Ok(())
}

/// Complete a wire `UPDATE_ATTESTATION` partial into a full canonical RM
/// `ATTESTATION` (RM common master04 §Attestation; ITS-REST
/// `UpdateAttestation`). The server supplies the inherited `AUDIT_DETAILS`
/// fields it owns — `system_id`, `time_committed`, and the `666|attestation|`
/// `change_type` — exactly as `UPDATE_AUDIT` → `AUDIT_DETAILS` (master06
/// §Version Update Semantics), then adds the `ATTESTATION`-specific attributes.
/// `committer` comes from the partial when present, else the CONTRIBUTION's
/// committer (master06 §Committal).
///
/// # Errors
/// [`ServiceError::Unprocessable`] when an RM invariant fails:
/// - `reason` absent (mandatory, 1..1), or a coded `reason` whose
///   `defining_code` is not in the openEHR `attestation reason` group
///   (`ATTESTATION.Reason_valid`);
/// - `is_pending` absent or not a `Boolean` (mandatory, 1..1);
/// - `items` present but not a non-empty list (`ATTESTATION.Items_valid`).
pub(crate) fn complete_attestation(
    partial: &Value,
    system_id: &str,
    committer_fallback: &Value,
    now: jiff::Timestamp,
) -> Result<Value, ServiceError> {
    // reason (1..1)
    let reason = partial.get("reason").cloned().ok_or_else(|| {
        ServiceError::Unprocessable("ATTESTATION.reason is required (1..1)".to_owned())
    })?;
    // Reason_valid: a coded reason's defining_code must be a member of the
    // openEHR `attestation reason` group.
    if reason.get("_type").and_then(Value::as_str) == Some("DV_CODED_TEXT") {
        let code = reason
            .pointer("/defining_code/code_string")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !openehr_term::bundle::openehr().is_valid_attestation_reason(code) {
            return Err(ServiceError::Unprocessable(format!(
                "ATTESTATION.reason.defining_code {code:?} is not in the openEHR \
                 `attestation reason` group (ATTESTATION.Reason_valid)"
            )));
        }
    }
    // is_pending (1..1, Boolean)
    let is_pending = partial
        .get("is_pending")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            ServiceError::Unprocessable(
                "ATTESTATION.is_pending is required (1..1 Boolean)".to_owned(),
            )
        })?;
    // items (0..1); Items_valid: non-empty when present.
    let items = partial.get("items");
    if let Some(items) = items
        && items.as_array().is_none_or(Vec::is_empty)
    {
        return Err(ServiceError::Unprocessable(
            "ATTESTATION.items must be a non-empty list when present \
             (ATTESTATION.Items_valid)"
                .to_owned(),
        ));
    }
    // committer: from the partial if present, else the CONTRIBUTION committer.
    let committer = partial
        .get("committer")
        .cloned()
        .unwrap_or_else(|| committer_fallback.clone());
    // description: UPDATE_AUDIT.description is a plain string or DV_TEXT.
    let description = partial.get("description").and_then(|d| {
        d.as_str()
            .or_else(|| d.get("value").and_then(Value::as_str))
    });
    // The inherited AUDIT_DETAILS fields, built exactly like any audit, then
    // retyped to ATTESTATION with its own attributes appended.
    let mut att = audit_details(
        system_id,
        change_type::ATTESTATION,
        description,
        &committer,
        &now,
    );
    if let Value::Object(map) = &mut att {
        map.insert("_type".to_owned(), Value::String("ATTESTATION".to_owned()));
        map.insert("reason".to_owned(), reason);
        map.insert("is_pending".to_owned(), Value::Bool(is_pending));
        if let Some(v) = partial.get("attested_view") {
            map.insert("attested_view".to_owned(), v.clone());
        }
        if let Some(v) = partial.get("proof") {
            map.insert("proof".to_owned(), v.clone());
        }
        if let Some(v) = items {
            map.insert("items".to_owned(), v.clone());
        }
    }
    Ok(att)
}
