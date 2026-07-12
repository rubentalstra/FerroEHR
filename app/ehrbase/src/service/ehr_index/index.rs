//! `I_EHR_INDEX` write operations (I1–I5, `i_ehr_index.adoc`) + the two
//! design-filled reads. All direct SQL over the `ehr_index` table (the ehr_index
//! domain's own design — register §5; no openEHR spec governs the storage,
//! master07 governs the operation semantics + error names).

use ehrbase_sm::{EhrIndexEntry, LocationDesc, ResourceStatus, SubjectRef};
use uuid::Uuid;

use crate::service::EhrbaseService;

use super::{IndexError, location_json, parse_valid_time, require_association, row_to_entry};

impl EhrbaseService {
    /// Confirm an EHR exists ([`IndexError::EhrDoesNotExist`] →
    /// `ehr_id_does_not_exist` otherwise). G-8/G-9: this distinguishes an
    /// unknown EHR from an unknown association to the caller.
    async fn index_ehr_exists(&self, ehr_id: Uuid) -> Result<(), IndexError> {
        let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .fetch_optional(&self.pool)
            .await?;
        found.map(|_| ()).ok_or(IndexError::EhrDoesNotExist(ehr_id))
    }

    /// `add_ehr_subject` (I1): associate `subject` with `ehr_id` with an
    /// optional status + location. The EHR must exist.
    ///
    /// PORT NOTE (G-14): "Add" is realized as an idempotent upsert
    /// (`ON CONFLICT DO UPDATE`) — re-adding the same subject refreshes its
    /// status/location rather than erroring; the `0..1` cardinality of
    /// `add_ehr_subject` permits this. Status defaults to a `Primary` instance
    /// (`i_ehr_index.adoc`).
    pub(crate) async fn index_add_subject(
        &self,
        ehr_id: Uuid,
        subject: &SubjectRef,
        status: Option<&ResourceStatus>,
        loc: Option<&LocationDesc>,
    ) -> Result<(), IndexError> {
        self.index_ehr_exists(ehr_id).await?;
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

    /// `update_ehr_subject_status` (I2): update the status of an existing
    /// (`ehr_id`, `subject`) association. Errors `ehr_id_does_not_exist` /
    /// `subject_id_does_not_exist`.
    pub(crate) async fn index_update_status(
        &self,
        ehr_id: Uuid,
        subject: &SubjectRef,
        status: &ResourceStatus,
    ) -> Result<(), IndexError> {
        self.index_ehr_exists(ehr_id).await?;
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
        require_association(updated.rows_affected(), subject)
    }

    /// `update_ehr_subject_loc_desc` (I3): update (or clear, `loc = None`) the
    /// location descriptor of an existing association. Errors
    /// `ehr_id_does_not_exist` / `subject_id_does_not_exist`.
    pub(crate) async fn index_update_loc_desc(
        &self,
        ehr_id: Uuid,
        subject: &SubjectRef,
        loc: Option<&LocationDesc>,
    ) -> Result<(), IndexError> {
        self.index_ehr_exists(ehr_id).await?;
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
        require_association(updated.rows_affected(), subject)
    }

    /// `remove_ehr_subject` (I4): drop the `subject`↔`ehr_id` association (the
    /// subject may remain associated with other EHRs). Errors
    /// `ehr_id_does_not_exist` / `subject_id_does_not_exist`.
    pub(crate) async fn index_remove_ehr_subject(
        &self,
        ehr_id: Uuid,
        subject: &SubjectRef,
    ) -> Result<(), IndexError> {
        self.index_ehr_exists(ehr_id).await?;
        let deleted = sqlx::query(
            "DELETE FROM ehr_index \
             WHERE ehr_id = $1 AND subject_id = $2 AND subject_namespace = $3",
        )
        .bind(ehr_id)
        .bind(&subject.id)
        .bind(&subject.namespace)
        .execute(&self.pool)
        .await?;
        require_association(deleted.rows_affected(), subject)
    }

    /// `remove_subject` (I5): drop all associations for `subject`. Error
    /// `subject_id_does_not_exist`.
    pub(crate) async fn index_remove_subject(
        &self,
        subject: &SubjectRef,
    ) -> Result<(), IndexError> {
        let deleted =
            sqlx::query("DELETE FROM ehr_index WHERE subject_id = $1 AND subject_namespace = $2")
                .bind(&subject.id)
                .bind(&subject.namespace)
                .execute(&self.pool)
                .await?;
        require_association(deleted.rows_affected(), subject)
    }

    /// The subjects associated with an EHR (design-filled read; the SM defines
    /// no read operations — our own design). Empty for an unknown EHR.
    pub(crate) async fn index_ehr_subjects(
        &self,
        ehr_id: Uuid,
    ) -> Result<Vec<EhrIndexEntry>, IndexError> {
        let rows = sqlx::query(
            "SELECT ehr_id, subject_id, subject_namespace, subject_type, instance_type, \
             start_valid_time, end_valid_time, notes, location FROM ehr_index \
             WHERE ehr_id = $1 ORDER BY subject_id, subject_namespace",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(row_to_entry).collect::<Result<_, _>>()?)
    }

    /// The EHRs associated with a subject (design-filled read). Empty for an
    /// unknown subject.
    pub(crate) async fn index_subject_ehrs(
        &self,
        subject: &SubjectRef,
    ) -> Result<Vec<EhrIndexEntry>, IndexError> {
        let rows = sqlx::query(
            "SELECT ehr_id, subject_id, subject_namespace, subject_type, instance_type, \
             start_valid_time, end_valid_time, notes, location FROM ehr_index \
             WHERE subject_id = $1 AND subject_namespace = $2 ORDER BY ehr_id",
        )
        .bind(&subject.id)
        .bind(&subject.namespace)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(row_to_entry).collect::<Result<_, _>>()?)
    }
}
