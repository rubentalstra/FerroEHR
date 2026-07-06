//! Builders for the ITS-REST version wrappers (`VERSIONED_OBJECT`,
//! `ORIGINAL_VERSION`) from a loaded [`VersionRead`](super::vobject::VersionRead).
//! These surface the full provenance a versioned object carries: its
//! `OBJECT_VERSION_ID`, the CONTRIBUTION that produced it, and its data.

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::codes;
use super::vobject::VersionRead;
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// The `REVISION_HISTORY` of a versioned object: one item per version, each
    /// with its version id and the `AUDIT_DETAILS` of the change that produced it.
    pub(super) async fn revision_history(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let rows = sqlx::query(
            "SELECT v.sys_version, v.ehr_id, a.system_id, a.change_type, a.description, \
             a.committer, a.time_committed \
             FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.vo_id = $1 ORDER BY v.sys_version",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;

        let first = rows
            .first()
            .ok_or_else(|| ServiceError::NotFound(format!("versioned object {vo_id}")))?;
        if first.try_get::<Uuid, _>("ehr_id")? != ehr_id {
            return Err(ServiceError::NotFound(format!(
                "versioned object {vo_id} in EHR {ehr_id}"
            )));
        }

        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            let sys_version: i32 = row.try_get("sys_version")?;
            let system_id: String = row.try_get("system_id")?;
            let change_type: String = row.try_get("change_type")?;
            let description: Option<String> = row.try_get("description")?;
            let committer: Value = row.try_get("committer")?;
            let time_committed: jiff::Timestamp = row
                .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
                .to_jiff();
            items.push(json!({
                "_type": "REVISION_HISTORY_ITEM",
                "version_id": {
                    "_type": "OBJECT_VERSION_ID",
                    "value": self.object_version_id(vo_id, sys_version)
                },
                "audits": [Self::audit_details(
                    &system_id, &change_type, description.as_deref(), &committer, &time_committed,
                )]
            }));
        }
        Ok(json!({ "_type": "REVISION_HISTORY", "items": items }))
    }

    /// A `VERSIONED_OBJECT` for `vo_id` owned by `ehr_id`, including the mandatory
    /// `time_created` (1..1) — the commit time of the object's first version
    /// (`VERSIONED_OBJECT.time_created`, RM `change_control`; F-06-05/F-01-08).
    pub(super) async fn versioned_object(
        &self,
        vo_id: Uuid,
        ehr_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let time_created: jiff_sqlx::Timestamp = sqlx::query_scalar(
            "SELECT a.time_committed FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.vo_id = $1 AND v.sys_version = 1",
        )
        .bind(vo_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("versioned object {vo_id}")))?;
        let time_created = time_created.to_jiff();
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

    /// An `ORIGINAL_VERSION` wrapping a loaded version: its `OBJECT_VERSION_ID`,
    /// the CONTRIBUTION reference, the mandatory `commit_audit` (1..1), the
    /// `preceding_version_uid` (present iff not the first version), the coded
    /// `lifecycle_state`, and the version data (absent for a deleted version).
    ///
    /// Spec: `VERSION.commit_audit` 1..1 (F-06-01/F-01-07);
    /// `VERSION.Preceding_version_uid_validity` (F-06-03);
    /// `ORIGINAL_VERSION.lifecycle_state` coded from `version_lifecycle_state` —
    /// `523|deleted|` for a deleted version (F-06-04/F-02-07).
    pub(super) fn original_version(&self, read: &VersionRead) -> Value {
        let mut ov = json!({
            "_type": "ORIGINAL_VERSION",
            "uid": {
                "_type": "OBJECT_VERSION_ID",
                "value": self.object_version_id(read.vo_id, read.sys_version)
            },
            "contribution": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "CONTRIBUTION",
                "id": { "_type": "HIER_OBJECT_ID", "value": read.contribution_id.to_string() }
            },
            "commit_audit": Self::audit_details(
                &read.audit.system_id,
                &read.audit.change_type,
                read.audit.description.as_deref(),
                &read.audit.committer,
                &read.time_committed,
            ),
            "lifecycle_state": {
                "_type": "DV_CODED_TEXT",
                "value": codes::lifecycle_rubric(&read.lifecycle_state),
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": codes::OPENEHR },
                    "code_string": read.lifecycle_state
                }
            }
        });
        if let Value::Object(map) = &mut ov {
            // preceding_version_uid: the prior version's OBJECT_VERSION_ID —
            // present for every non-first version, absent for the first.
            if read.sys_version > 1 {
                map.insert(
                    "preceding_version_uid".to_owned(),
                    json!({
                        "_type": "OBJECT_VERSION_ID",
                        "value": self.object_version_id(read.vo_id, read.sys_version - 1)
                    }),
                );
            }
            // A deleted version carries no data (canonical is Null).
            if !read.canonical.is_null() {
                map.insert("data".to_owned(), read.canonical.clone());
            }
        }
        ov
    }
}
