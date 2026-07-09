//! EHR Index domain logic (SM `I_EHR_INDEX`, `master07-ehr_index_service.adoc`):
//! N:M subject↔EHR associations with duplicate-management metadata.
//!
//! PORT NOTE: index entries are **not** versioned objects — the SM defines no
//! versioning for the index (design 08 §4.1) — so these are plain SQL writes
//! over the `ehr_index` table, emitting no CONTRIBUTION/version. This does not
//! touch the `ehr.subject_id` promotion (the Primary-instance fast path for
//! `ehr_get_by_subject` stays as-is); the index models the full N:M state.

use ehrbase_rest::backend::{
    EhrIndexEntry, LocationDesc, ResourceInstanceType, ResourceStatus, SubjectRef,
};
use jiff_sqlx::ToSqlx;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::{EhrbaseService, ServiceError};

/// Parse an ISO-8601 date-time string into a Postgres timestamptz binding, or
/// `None`. An unparseable value is a `400`. (`RESOURCE_STATUS` validity times are
/// typed `@@` in the SM — implemented as ISO date-time, PORT NOTE.)
fn parse_valid_time(raw: Option<&str>) -> Result<Option<jiff_sqlx::Timestamp>, ServiceError> {
    match raw {
        None => Ok(None),
        Some(s) => s
            .parse::<jiff::Timestamp>()
            .map(|t| Some(t.to_sqlx()))
            .map_err(|_| ServiceError::BadRequest(format!("invalid valid_time: {s}"))),
    }
}

/// Render a [`LocationDesc`] as the stored canonical JSON, or SQL NULL.
fn location_json(loc: Option<&LocationDesc>) -> Option<Value> {
    loc.map(|l| {
        json!({
            "system_id": l.system_id,
            "uri": l.uri,
            "description": l.description,
        })
    })
}

impl EhrbaseService {
    /// Confirm an EHR exists (`ehr_id_does_not_exist` → `404` otherwise).
    async fn ehr_exists(&self, ehr_id: Uuid) -> Result<(), ServiceError> {
        let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .fetch_optional(&self.pool)
            .await?;
        found
            .map(|_| ())
            .ok_or_else(|| ServiceError::NotFound(format!("EHR {ehr_id} does not exist")))
    }

    /// `add_ehr_subject`: associate `subject` with `ehr_id` (idempotent upsert of
    /// the association's status + location). The EHR must exist.
    pub(super) async fn index_add_subject(
        &self,
        ehr_id: Uuid,
        subject: &SubjectRef,
        status: Option<&ResourceStatus>,
        loc: Option<&LocationDesc>,
    ) -> Result<(), ServiceError> {
        self.ehr_exists(ehr_id).await?;
        let default_status = ResourceStatus::default();
        let status = status.unwrap_or(&default_status);
        let start = parse_valid_time(status.start_valid_time.as_deref())?;
        let end = parse_valid_time(status.end_valid_time.as_deref())?;
        sqlx::query(
            "INSERT INTO ehr_index \
             (ehr_id, subject_id, subject_namespace, subject_type, instance_type, \
              start_valid_time, end_valid_time, notes, location) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (ehr_id, subject_id, subject_namespace) DO UPDATE SET \
              subject_type = EXCLUDED.subject_type, instance_type = EXCLUDED.instance_type, \
              start_valid_time = EXCLUDED.start_valid_time, end_valid_time = EXCLUDED.end_valid_time, \
              notes = EXCLUDED.notes, location = EXCLUDED.location",
        )
        .bind(ehr_id)
        .bind(&subject.id)
        .bind(&subject.namespace)
        .bind(&subject.r#type)
        .bind(status.instance_type.as_str())
        .bind(start)
        .bind(end)
        .bind(status.notes.as_deref())
        .bind(location_json(loc))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// `update_ehr_subject_status`: update the status of an existing
    /// (`ehr_id`, `subject`) association.
    pub(super) async fn index_update_status(
        &self,
        ehr_id: Uuid,
        subject: &SubjectRef,
        status: &ResourceStatus,
    ) -> Result<(), ServiceError> {
        self.ehr_exists(ehr_id).await?;
        let start = parse_valid_time(status.start_valid_time.as_deref())?;
        let end = parse_valid_time(status.end_valid_time.as_deref())?;
        let updated = sqlx::query(
            "UPDATE ehr_index SET instance_type = $4, start_valid_time = $5, \
             end_valid_time = $6, notes = $7 \
             WHERE ehr_id = $1 AND subject_id = $2 AND subject_namespace = $3",
        )
        .bind(ehr_id)
        .bind(&subject.id)
        .bind(&subject.namespace)
        .bind(status.instance_type.as_str())
        .bind(start)
        .bind(end)
        .bind(status.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Self::require_association(updated.rows_affected(), subject)
    }

    /// `update_ehr_subject_loc_desc`: update (or clear, `loc = None`) the
    /// location descriptor of an existing (`ehr_id`, `subject`) association.
    pub(super) async fn index_update_loc_desc(
        &self,
        ehr_id: Uuid,
        subject: &SubjectRef,
        loc: Option<&LocationDesc>,
    ) -> Result<(), ServiceError> {
        self.ehr_exists(ehr_id).await?;
        let updated = sqlx::query(
            "UPDATE ehr_index SET location = $4 \
             WHERE ehr_id = $1 AND subject_id = $2 AND subject_namespace = $3",
        )
        .bind(ehr_id)
        .bind(&subject.id)
        .bind(&subject.namespace)
        .bind(location_json(loc))
        .execute(&self.pool)
        .await?;
        Self::require_association(updated.rows_affected(), subject)
    }

    /// `remove_ehr_subject`: drop the `subject`↔`ehr_id` association (the subject
    /// may remain associated with other EHRs).
    pub(super) async fn index_remove_ehr_subject(
        &self,
        ehr_id: Uuid,
        subject: &SubjectRef,
    ) -> Result<(), ServiceError> {
        self.ehr_exists(ehr_id).await?;
        let deleted = sqlx::query(
            "DELETE FROM ehr_index \
             WHERE ehr_id = $1 AND subject_id = $2 AND subject_namespace = $3",
        )
        .bind(ehr_id)
        .bind(&subject.id)
        .bind(&subject.namespace)
        .execute(&self.pool)
        .await?;
        Self::require_association(deleted.rows_affected(), subject)
    }

    /// `remove_subject`: drop all associations for `subject`.
    pub(super) async fn index_remove_subject(
        &self,
        subject: &SubjectRef,
    ) -> Result<(), ServiceError> {
        let deleted =
            sqlx::query("DELETE FROM ehr_index WHERE subject_id = $1 AND subject_namespace = $2")
                .bind(&subject.id)
                .bind(&subject.namespace)
                .execute(&self.pool)
                .await?;
        Self::require_association(deleted.rows_affected(), subject)
    }

    /// The subjects associated with an EHR (design-filled read).
    pub(super) async fn index_ehr_subjects(
        &self,
        ehr_id: Uuid,
    ) -> Result<Vec<EhrIndexEntry>, ServiceError> {
        let rows = sqlx::query(
            "SELECT ehr_id, subject_id, subject_namespace, subject_type, instance_type, \
             start_valid_time, end_valid_time, notes, location FROM ehr_index \
             WHERE ehr_id = $1 ORDER BY subject_id, subject_namespace",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(row_to_entry).collect()
    }

    /// The EHRs associated with a subject (design-filled read).
    pub(super) async fn index_subject_ehrs(
        &self,
        subject: &SubjectRef,
    ) -> Result<Vec<EhrIndexEntry>, ServiceError> {
        let rows = sqlx::query(
            "SELECT ehr_id, subject_id, subject_namespace, subject_type, instance_type, \
             start_valid_time, end_valid_time, notes, location FROM ehr_index \
             WHERE subject_id = $1 AND subject_namespace = $2 ORDER BY ehr_id",
        )
        .bind(&subject.id)
        .bind(&subject.namespace)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(row_to_entry).collect()
    }

    /// Map a zero-rows-affected write to `subject_id_does_not_exist` (`404`).
    fn require_association(affected: u64, subject: &SubjectRef) -> Result<(), ServiceError> {
        if affected == 0 {
            return Err(ServiceError::NotFound(format!(
                "subject {}@{} is not associated (subject_id_does_not_exist)",
                subject.id, subject.namespace
            )));
        }
        Ok(())
    }
}

/// Reassemble one [`EhrIndexEntry`] from an `ehr_index` row.
fn row_to_entry(row: &sqlx::postgres::PgRow) -> Result<EhrIndexEntry, ServiceError> {
    let ehr_id: Uuid = row.try_get("ehr_id")?;
    let subject = SubjectRef {
        id: row.try_get("subject_id")?,
        namespace: row.try_get("subject_namespace")?,
        r#type: row.try_get("subject_type")?,
    };
    let start: Option<jiff_sqlx::Timestamp> = row.try_get("start_valid_time")?;
    let end: Option<jiff_sqlx::Timestamp> = row.try_get("end_valid_time")?;
    let status = ResourceStatus {
        instance_type: ResourceInstanceType::from_str_or_primary(
            &row.try_get::<String, _>("instance_type")?,
        ),
        start_valid_time: start.map(|t| t.to_jiff().to_string()),
        end_valid_time: end.map(|t| t.to_jiff().to_string()),
        notes: row.try_get("notes")?,
    };
    let location = row
        .try_get::<Option<Value>, _>("location")?
        .map(|v| LocationDesc {
            system_id: v
                .get("system_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            uri: v.get("uri").and_then(Value::as_str).map(str::to_owned),
            description: v
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_owned),
        });
    Ok(EhrIndexEntry {
        ehr_id: ehr_id.to_string(),
        subject,
        status,
        location,
    })
}
