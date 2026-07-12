//! Version reads + the wire builders: VERSIONED_OBJECT / ORIGINAL_VERSION /
//! REVISION_HISTORY (S-08..S-10, S-12, S-13, S-46, S-47).
//!
//! Spec: RM common `master06-change_control_package.adoc` §Versioned Objects /
//! §Version and its Subtypes, RM common `master04-generic_package.adoc`
//! §Revision History, BASE base_types `master05-identification_package.adoc`
//! §References. These builders surface the full provenance a versioned object
//! carries: its OBJECT_VERSION_ID, the CONTRIBUTION that produced it, the
//! mandatory commit audit, and its data. All SQL is delegated to
//! `crate::storage::version_repo`; the canonical body comes from
//! `crate::storage::node_repo`.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::ServiceError;
use crate::versioning::audit::{AuditInput, OPENEHR, audit_details};
use crate::versioning::lifecycle::{self, lifecycle_rubric};
use crate::versioning::object_version_id::{TreeId, object_version_id};
use crate::versioning::signature::Signer;
use crate::versioning::{Kind, integrity};

/// A stored version's row (the `vo_version` ⋈ `audit` join), returned by the
/// storage `version_repo`. The value contract at the read seam: versioning
/// composes a [`VersionRead`] from this plus the canonical body and the
/// attestations.
///
/// TODO(w3f-integrate): `crate::storage::version_repo` produces this value
/// (register 02 owns the SQL + `PgRow` → `StoredVersion` mapping).
#[derive(Debug, Clone)]
pub(crate) struct StoredVersion {
    pub(crate) vo_id: Uuid,
    /// The owning EHR, or `None` for a demographic party (no EHR scope).
    pub(crate) ehr_id: Option<Uuid>,
    /// The per-vo storage commit ordinal (the node / attestation key), NOT the
    /// wire version number.
    pub(crate) sys_version: i32,
    /// The version's `VERSION_TREE_ID` (the wire version identity).
    pub(crate) tree: TreeId,
    pub(crate) preceding_version_uid: Option<String>,
    pub(crate) other_input_version_uids: Vec<String>,
    pub(crate) lifecycle_state: String,
    pub(crate) creating_system_id: String,
    pub(crate) contribution_id: Uuid,
    pub(crate) audit: AuditInput,
    pub(crate) time_committed: jiff::Timestamp,
    pub(crate) template_id: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) kind: Kind,
}

/// A loaded version: its full provenance metadata and reassembled canonical
/// JSON (with attestations attached).
#[derive(Debug, Clone)]
pub(crate) struct VersionRead {
    pub(crate) vo_id: Uuid,
    pub(crate) ehr_id: Option<Uuid>,
    pub(crate) tree: TreeId,
    pub(crate) preceding_version_uid: Option<String>,
    pub(crate) other_input_version_uids: Vec<String>,
    pub(crate) lifecycle_state: String,
    /// The immutable identity of the system that created this version (RM common
    /// master06 §Distributed Versioning), the middle part of its
    /// OBJECT_VERSION_ID.
    pub(crate) creating_system_id: String,
    pub(crate) contribution_id: Uuid,
    /// The mandatory `VERSION.commit_audit` (1..1).
    pub(crate) audit: AuditInput,
    pub(crate) time_committed: jiff::Timestamp,
    pub(crate) template_id: Option<String>,
    /// The stored `VERSION.signature` (0..1; RM common master06 §Digital
    /// Signature), or `None` for versions committed before signing was enabled.
    pub(crate) signature: Option<String>,
    /// The reassembled canonical JSON, or `Value::Null` for a deleted version
    /// (a logical delete stores no node rows — master06 §Logical Deletion).
    pub(crate) canonical: Value,
    /// The `ATTESTATION`s attached to this version, in commit order (RM common
    /// master06 §Attestation). Surfaced as `ORIGINAL_VERSION.attestations`,
    /// appended **after** signature verification (attestations arrive after
    /// committal and are not part of the signed canonical form).
    pub(crate) attestations: Vec<Value>,
}

impl VersionRead {
    /// Whether this version is logically deleted (`lifecycle_state` `523`).
    pub(crate) fn deleted(&self) -> bool {
        self.lifecycle_state == lifecycle::state::DELETED
    }
}

/// Compose a [`VersionRead`] from a stored row, resolving the canonical body: a
/// deleted version (lifecycle `523`) carries no node rows, so its body is
/// `Value::Null` and reassembly is skipped entirely (this is what stops a
/// deleted read from erroring on an empty node set).
async fn version_read(
    pool: &sqlx::PgPool,
    stored: StoredVersion,
) -> Result<VersionRead, ServiceError> {
    let canonical = if stored.lifecycle_state == lifecycle::state::DELETED {
        Value::Null
    } else {
        // TODO(w3f-integrate): storage node reassembly seam.
        crate::storage::node_repo::read_version_canonical(pool, stored.vo_id, stored.sys_version)
            .await?
    };
    // TODO(w3f-integrate): version_repo::read_attestations.
    let attestations =
        crate::storage::version_repo::read_attestations(pool, stored.vo_id, stored.sys_version)
            .await?;
    Ok(VersionRead {
        vo_id: stored.vo_id,
        ehr_id: stored.ehr_id,
        tree: stored.tree,
        preceding_version_uid: stored.preceding_version_uid,
        other_input_version_uids: stored.other_input_version_uids,
        lifecycle_state: stored.lifecycle_state,
        creating_system_id: stored.creating_system_id,
        contribution_id: stored.contribution_id,
        audit: stored.audit,
        time_committed: stored.time_committed,
        template_id: stored.template_id,
        signature: stored.signature,
        canonical,
        attestations,
    })
}

/// Read the current version of an object by id (any kind). `None` if it never
/// existed; a deleted current version is returned with `canonical = Null` and a
/// `523` lifecycle so callers can distinguish 404 (never existed) from a
/// deleted read (RM common master06 §Logical Deletion).
pub(crate) async fn read_current(
    pool: &sqlx::PgPool,
    vo_id: Uuid,
) -> Result<Option<VersionRead>, ServiceError> {
    // TODO(w3f-integrate): version_repo::read_current.
    match crate::storage::version_repo::read_current(pool, vo_id).await? {
        Some(stored) => Ok(Some(version_read(pool, stored).await?)),
        None => Ok(None),
    }
}

/// Read a specific version of an object by its STORAGE ORDINAL (`sys_version`)
/// — for internal callers that key rows by ordinal (the FHIR mapping table,
/// extract export iteration), never for wire version ids.
pub(crate) async fn read_version_by_ordinal(
    pool: &sqlx::PgPool,
    vo_id: Uuid,
    ordinal: i32,
) -> Result<Option<VersionRead>, ServiceError> {
    // TODO(w3f-integrate): version_repo::read_version_by_ordinal.
    match crate::storage::version_repo::read_version_by_ordinal(pool, vo_id, ordinal).await? {
        Some(stored) => Ok(Some(version_read(pool, stored).await?)),
        None => Ok(None),
    }
}

/// Read a specific version of an object by its `VERSION_TREE_ID`
/// (`.../version/{version_uid}` — trunk or branch).
pub(crate) async fn read_version(
    pool: &sqlx::PgPool,
    vo_id: Uuid,
    tree: TreeId,
) -> Result<Option<VersionRead>, ServiceError> {
    // TODO(w3f-integrate): version_repo::read_version.
    match crate::storage::version_repo::read_version(pool, vo_id, tree).await? {
        Some(stored) => Ok(Some(version_read(pool, stored).await?)),
        None => Ok(None),
    }
}

/// Read the version of an object that was current at a given instant
/// (time-travel; RM common master08 §Change Management — any previous state is
/// reconstructable): the row whose `sys_period` contains `at`.
pub(crate) async fn version_at(
    pool: &sqlx::PgPool,
    vo_id: Uuid,
    at: jiff::Timestamp,
) -> Result<Option<VersionRead>, ServiceError> {
    // TODO(w3f-integrate): version_repo::read_version_at.
    match crate::storage::version_repo::read_version_at(pool, vo_id, at).await? {
        Some(stored) => Ok(Some(version_read(pool, stored).await?)),
        None => Ok(None),
    }
}

/// The kind of the current version of an object, or `None` if it does not
/// exist.
pub(crate) async fn object_kind(
    pool: &sqlx::PgPool,
    vo_id: Uuid,
) -> Result<Option<Kind>, ServiceError> {
    // TODO(w3f-integrate): version_repo::object_kind.
    crate::storage::version_repo::object_kind(pool, vo_id).await
}

/// The `REVISION_HISTORY` of a versioned object: one item per version, each with
/// its version id and the `AUDIT_DETAILS` of the change that produced it plus
/// any attestations of that revision (RM common master04 §Revision History:
/// "there will always be at least one commit audit … there may also be further
/// attestations").
///
/// PORT NOTE (G-12, master04 §Revision History): the REVISION_HISTORY is
/// assembled directly as canonical JSON rather than through a typed
/// `openehr-rm` builder — a spec-silent serialization choice; the wire shape is
/// spec-correct.
pub(crate) async fn revision_history(
    pool: &sqlx::PgPool,
    ehr_id: Uuid,
    vo_id: Uuid,
) -> Result<Value, ServiceError> {
    // TODO(w3f-integrate): version_repo::all_versions (ordered by sys_version).
    let rows = crate::storage::version_repo::all_versions(pool, vo_id).await?;
    let first = rows
        .first()
        .ok_or_else(|| ServiceError::NotFound(format!("versioned object {vo_id}")))?;
    if first.ehr_id != Some(ehr_id) {
        return Err(ServiceError::NotFound(format!(
            "versioned object {vo_id} in EHR {ehr_id}"
        )));
    }

    // Attestations for the object, keyed by version, in commit order.
    // TODO(w3f-integrate): version_repo::read_attestations_all.
    let att_rows = crate::storage::version_repo::read_attestations_all(pool, vo_id).await?;
    let mut attestations: std::collections::HashMap<i32, Vec<Value>> =
        std::collections::HashMap::new();
    for (sys_version, data) in att_rows {
        attestations.entry(sys_version).or_default().push(data);
    }

    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut audits = vec![audit_details(
            &row.audit.system_id,
            &row.audit.change_type,
            row.audit.description.as_deref(),
            &row.audit.committer,
            &row.time_committed,
        )];
        if let Some(atts) = attestations.remove(&row.sys_version) {
            audits.extend(atts);
        }
        items.push(json!({
            "_type": "REVISION_HISTORY_ITEM",
            "version_id": {
                "_type": "OBJECT_VERSION_ID",
                "value": object_version_id(vo_id, &row.creating_system_id, row.tree)
            },
            "audits": audits
        }));
    }
    Ok(json!({ "_type": "REVISION_HISTORY", "items": items }))
}

/// A `VERSIONED_OBJECT` for `vo_id` owned by `ehr_id`, including the mandatory
/// `time_created` (1..1) — the commit time of the object's first version
/// (`VERSIONED_OBJECT.time_created`, RM common master06 §Versioned Objects).
///
/// PORT NOTE (EHR-Extract import, master06 §Copying): the earliest **held**
/// version is used, not a hardcoded `sys_version = 1`. A latest-only clone
/// (`import_ehr` over an `export_ehrs` extract) legitimately holds a partial
/// trunk history whose lowest version is `> 1`; `time_created` is then the
/// earliest version this repository received.
pub(crate) async fn versioned_object(
    pool: &sqlx::PgPool,
    vo_id: Uuid,
    ehr_id: Uuid,
) -> Result<Value, ServiceError> {
    // TODO(w3f-integrate): version_repo::time_created (earliest held version).
    let time_created = crate::storage::version_repo::time_created(pool, vo_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("versioned object {vo_id}")))?;
    Ok(json!({
        "_type": "VERSIONED_OBJECT",
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
/// [`ServiceError::Signing`] only when `verify_on_read = strict` and the stored
/// signature fails verification.
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
    vo_id: Uuid,
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
