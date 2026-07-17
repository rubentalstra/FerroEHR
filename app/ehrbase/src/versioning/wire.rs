//! The served wire builders: `ORIGINAL_VERSION` / `VERSIONED_OBJECT` /
//! `REVISION_HISTORY` canonical-JSON value construction.
//!
//! Spec: RM common `master06-change_control_package.adoc` §Versioned Objects /
//! §Version and its Subtypes, RM common `master04-generic_package.adoc`
//! §Revision History, BASE `base_types` `master05-identification_package.adoc`
//! §References. These builders surface the full provenance a versioned object
//! carries: its `OBJECT_VERSION_ID`, the CONTRIBUTION that produced it, the
//! mandatory commit audit, and its data. The loaded-version input comes from
//! [`super::read`]; the residual SQL (first-version time, all-version metadata)
//! is delegated to `crate::storage::version_repo`.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::service::error::ServiceError;
use crate::versioning::audit::{AuditInput, OPENEHR, audit_details};
use crate::versioning::integrity;
use crate::versioning::lifecycle::lifecycle_rubric;
use crate::versioning::object_version_id::{TreeId, object_version_id};
use crate::versioning::read::VersionRead;
use crate::versioning::signature::signer::Signer;

/// The `REVISION_HISTORY` of a versioned object: one item per version, each with
/// its version id and the `AUDIT_DETAILS` of the change that produced it plus
/// any attestations of that revision (RM common master04 §Revision History:
/// "there will always be at least one commit audit … there may also be further
/// attestations").
///
/// PORT NOTE (G-12, master04 §Revision History): the `REVISION_HISTORY` is
/// assembled directly as canonical JSON rather than through a typed
/// `openehr-rm` builder — a spec-silent serialization choice; the wire shape is
/// spec-correct.
///
/// # Errors
/// [`ServiceError::NotFound`] when the object has no stored version or is not
/// owned by `ehr_id`; the storage read errors of the metadata / attestation
/// queries.
pub(crate) async fn revision_history(
    pool: &sqlx::PgPool,
    ehr_id: EhrId,
    vo_id: VoId,
) -> Result<Value, ServiceError> {
    let rows = crate::storage::version_repo::meta::all_version_meta(pool, vo_id).await?;
    let first = rows
        .first()
        .ok_or_else(|| ServiceError::NotFound(format!("versioned object {vo_id}")))?;
    if first.ehr_id != Some(ehr_id) {
        return Err(ServiceError::NotFound(format!(
            "versioned object {vo_id} in EHR {ehr_id}"
        )));
    }

    // Attestations for the object, keyed by version, in commit order.
    let att_rows =
        crate::storage::version_repo::attestation::read_attestations_all(pool, vo_id).await?;
    let mut attestations: std::collections::HashMap<i32, Vec<Value>> =
        std::collections::HashMap::new();
    for (sys_version, data) in att_rows {
        attestations.entry(sys_version).or_default().push(data);
    }

    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut audits = vec![audit_details(
            &row.audit_system_id,
            &row.audit_change_type,
            row.audit_description.as_deref(),
            &row.audit_committer,
            &row.time_committed,
        )];
        if let Some(atts) = attestations.remove(&row.sys_version) {
            audits.extend(atts);
        }
        items.push(json!({
            "_type": "REVISION_HISTORY_ITEM",
            "version_id": {
                "_type": "OBJECT_VERSION_ID",
                "value": object_version_id(
                    vo_id,
                    &row.creating_system_id,
                    TreeId::from_columns(row.trunk_version, row.branch_number, row.branch_version)
                )
            },
            "audits": audits
        }));
    }
    Ok(json!({ "_type": "REVISION_HISTORY", "items": items }))
}

/// A versioned-object wire body for `vo_id` owned by `ehr_id`, carrying the
/// **concrete** RM `_type` (`rm_type`: `VERSIONED_COMPOSITION` /
/// `VERSIONED_EHR_STATUS` / `VERSIONED_FOLDER` — RM ehr master04 defines the
/// concrete bindings of `VERSIONED_OBJECT<T>`; the ITS-REST
/// `200_VERSIONED_COMPOSITION` schema pins the discriminator to the concrete
/// class, never the generic `VERSIONED_OBJECT`), including the mandatory
/// `time_created` (1..1) — the commit time of the object's first version
/// (`VERSIONED_OBJECT.time_created`, RM common master06 §Versioned Objects).
///
/// PORT NOTE (EHR-Extract import, master06 §Copying): the earliest **held**
/// version is used, not a hardcoded `sys_version = 1`. A latest-only clone
/// (`import_ehr` over an `export_ehrs` extract) legitimately holds a partial
/// trunk history whose lowest version is `> 1`; `time_created` is then the
/// earliest version this repository received.
///
/// # Errors
/// [`ServiceError::NotFound`] when the object has no stored version; the
/// storage read error of `version_repo::meta::time_created`.
pub(crate) async fn versioned_object(
    pool: &sqlx::PgPool,
    vo_id: VoId,
    ehr_id: EhrId,
    rm_type: &str,
) -> Result<Value, ServiceError> {
    let time_created = crate::storage::version_repo::meta::time_created(pool, vo_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("versioned object {vo_id}")))?;
    Ok(json!({
        "_type": rm_type,
        "uid": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() },
        "owner_id": {
            "_type": "OBJECT_REF",
            "namespace": "local",
            "type": "EHR",
            "id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() }
        },
        "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
    }))
}

/// An `ORIGINAL_VERSION` wrapping a loaded version, with read-time signature
/// verification (RM common master06 §Version subtypes / §Digital Signature).
///
/// When `verify_on_read` is not `off`, the served version's signature is checked
/// against its recomputed `canonical_form`: a `warn` mismatch logs + meters, a
/// `strict` mismatch is a 5xx integrity failure.
///
/// # Errors
/// [`ServiceError::Signing`] when `verify_on_read = strict` and the stored
/// signature fails verification, or (in any non-`off` mode) when the served
/// version's canonical form cannot be recomputed.
pub(crate) fn original_version(read: &VersionRead, signer: &Signer) -> Result<Value, ServiceError> {
    let mut ov = build_original_version(
        &read.creating_system_id,
        read.vo_id,
        read.tree,
        read.preceding_version_uid.as_deref(),
        &read.other_input_version_uids,
        read.contribution_id,
        &read.audit,
        &read.time_committed,
        &read.lifecycle_state,
        &read.canonical,
        read.signature.as_deref(),
    );
    integrity::verify_on_read(signer, &ov, read.signature.as_deref())?;
    // ORIGINAL_VERSION.attestations (RM common master06 §Attestation). Appended
    // AFTER verify_on_read: attestations "can be added at any time after
    // committal", so they are not part of the signed/verified canonical form
    // (the signature was computed over the attestation-free version at commit).
    if !read.attestations.is_empty()
        && let Value::Object(map) = &mut ov
    {
        map.insert(
            "attestations".to_owned(),
            Value::Array(read.attestations.clone()),
        );
    }
    Ok(ov)
}

/// Build the `ORIGINAL_VERSION` JSON for a version from its parts — the single
/// shared builder used by both the read path ([`original_version`]) and the
/// commit path ([`super::integrity::sign_version`]), so the bytes signed at
/// commit and served at read are identical. `canonical_data` is `Value::Null`
/// for a deleted version (no `data`); `signature` is set iff known.
///
/// Spec: `VERSION.commit_audit` 1..1; `VERSION.Preceding_version_uid_validity`;
/// `ORIGINAL_VERSION.lifecycle_state` coded from `version_lifecycle_state`
/// (RM common master06 §Version subtypes).
#[allow(clippy::too_many_arguments)] // the ORIGINAL_VERSION's attributes; a struct would not read clearer
pub(crate) fn build_original_version(
    creating_system_id: &str,
    vo_id: VoId,
    tree: TreeId,
    preceding_version_uid: Option<&str>,
    other_input_version_uids: &[String],
    contribution_id: Uuid,
    audit: &AuditInput,
    time_committed: &jiff::Timestamp,
    lifecycle_state: &str,
    canonical_data: &Value,
    signature: Option<&str>,
) -> Value {
    let mut ov = json!({
        "_type": "ORIGINAL_VERSION",
        "uid": {
            "_type": "OBJECT_VERSION_ID",
            "value": object_version_id(vo_id, creating_system_id, tree)
        },
        "contribution": {
            "_type": "OBJECT_REF",
            "namespace": "local",
            "type": "CONTRIBUTION",
            "id": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() }
        },
        "commit_audit": audit_details(
            &audit.system_id,
            &audit.change_type,
            audit.description.as_deref(),
            &audit.committer,
            time_committed,
        ),
        "lifecycle_state": {
            "_type": "DV_CODED_TEXT",
            "value": lifecycle_rubric(lifecycle_state),
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": OPENEHR },
                "code_string": lifecycle_state
            }
        }
    });
    if let Value::Object(map) = &mut ov {
        // preceding_version_uid: the STORED prior OBJECT_VERSION_ID (absent for
        // a first version). Stored — not synthesized — because under branching
        // and import the preceding version may carry a different
        // creating_system_id (RM common master06 §Distributed Versioning).
        if let Some(preceding) = preceding_version_uid {
            map.insert(
                "preceding_version_uid".to_owned(),
                json!({ "_type": "OBJECT_VERSION_ID", "value": preceding }),
            );
        }
        // other_input_version_uids: merge provenance (master06 §Version
        // Merging); `is_merged` is its derived boolean
        // (`VERSION.Is_merged_validity`: is_merged = not …is_empty).
        if !other_input_version_uids.is_empty() {
            map.insert(
                "other_input_version_uids".to_owned(),
                json!(
                    other_input_version_uids
                        .iter()
                        .map(|uid| json!({ "_type": "OBJECT_VERSION_ID", "value": uid }))
                        .collect::<Vec<_>>()
                ),
            );
        }
        // A deleted version carries no data (canonical is Null).
        if !canonical_data.is_null() {
            map.insert("data".to_owned(), canonical_data.clone());
        }
        if let Some(sig) = signature {
            map.insert("signature".to_owned(), Value::String(sig.to_owned()));
        }
    }
    ov
}
