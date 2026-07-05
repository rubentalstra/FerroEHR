//! CONTRIBUTION retrieval — the change-set envelope with its `AUDIT_DETAILS` and
//! the versions it produced.
//!
//! `contribution_create` (applying a set of VERSIONs atomically under one
//! contribution) needs a shared-contribution write path across `vobject` and is
//! the next endpoint; the retrieval + provenance model is complete here.

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Retrieve a CONTRIBUTION by id (scoped to the EHR), with its audit and the
    /// `OBJECT_REFs` of the versions it committed.
    pub(super) async fn get_contribution(
        &self,
        ehr_id: Uuid,
        contribution_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let meta = sqlx::query(
            "SELECT a.system_id, a.change_type, a.description, a.committer, a.time_committed \
             FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE c.id = $1 AND c.ehr_id = $2",
        )
        .bind(contribution_id)
        .bind(ehr_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("CONTRIBUTION {contribution_id}")))?;

        let system_id: String = meta.try_get("system_id")?;
        let change_type: String = meta.try_get("change_type")?;
        let description: Option<String> = meta.try_get("description")?;
        let committer: Value = meta.try_get("committer")?;
        let time_committed: jiff::Timestamp = meta
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff();

        let version_rows = sqlx::query(
            "SELECT vo_id, sys_version, kind FROM vo_version WHERE contribution_id = $1 \
             ORDER BY vo_id",
        )
        .bind(contribution_id)
        .fetch_all(&self.pool)
        .await?;

        let versions: Vec<Value> = version_rows
            .iter()
            .map(|row| -> Result<Value, ServiceError> {
                let vo_id: Uuid = row.try_get("vo_id")?;
                let sys_version: i32 = row.try_get("sys_version")?;
                let kind: String = row.try_get("kind")?;
                Ok(json!({
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": kind,
                    "id": {
                        "_type": "OBJECT_VERSION_ID",
                        "value": self.object_version_id(vo_id, sys_version)
                    }
                }))
            })
            .collect::<Result<_, _>>()?;

        Ok(json!({
            "_type": "CONTRIBUTION",
            "uid": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() },
            "audit": Self::audit_details(&system_id, &change_type, description.as_deref(), &committer, &time_committed),
            "versions": versions
        }))
    }

    /// Build an `AUDIT_DETAILS` from stored audit columns.
    fn audit_details(
        system_id: &str,
        change_type: &str,
        description: Option<&str>,
        committer: &Value,
        time_committed: &jiff::Timestamp,
    ) -> Value {
        let mut audit = json!({
            "_type": "AUDIT_DETAILS",
            "system_id": system_id,
            "time_committed": { "_type": "DV_DATE_TIME", "value": time_committed.to_string() },
            "change_type": {
                "_type": "DV_CODED_TEXT",
                "value": change_type,
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": change_type
                }
            },
            "committer": committer
        });
        if let (Some(desc), Value::Object(map)) = (description, &mut audit) {
            map.insert(
                "description".to_owned(),
                json!({ "_type": "DV_TEXT", "value": desc }),
            );
        }
        audit
    }
}
