// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `I_EHR_INDEX` operations (I1–I5, `i_ehr_index.adoc`) + the two
//! design-filled reads. All direct SQL over the `ehr_index` table (no openEHR
//! spec governs the storage — our own design; master07 governs the operation
//! semantics + error names).
//!
//! Every domain failure is an [`IndexError`], whose `From<IndexError> for
//! SmError` maps `ehr_id_does_not_exist` / `subject_id_does_not_exist` onto
//! their dedicated `CallStatusType` variants — never the generic
//! `versioned_object_does_not_exist` (`i_ehr_index.adoc §Errors`).

use uuid::Uuid;

use crate::ids::EhrId;
use crate::service::FerroEhrService;
use crate::service::ehr_index::types::{EhrIndexEntry, LocationDesc, ResourceStatus, SubjectRef};
use crate::service::status::SmError;

use super::{IndexError, location_binding, parse_valid_time, require_association, row_to_entry};

/// Parse an `ehr_id` UUID. An unparseable id is a `400` precondition failure;
/// a well-formed-but-unknown id surfaces as `ehr_id_does_not_exist` at the DB
/// check (`i_ehr_index.adoc §Errors`).
#[expect(
    clippy::map_err_ignore,
    reason = "the mapped error already names the resource and echoes the \
              rejected token; the discarded `uuid::Error` adds only its own \
              wording, which is not part of the wire contract"
)]
fn parse_ehr_id(raw: &str) -> Result<EhrId, SmError> {
    Uuid::parse_str(raw)
        .map(EhrId)
        .map_err(|_| SmError::precondition(format!("invalid ehr id: {raw}")))
}

impl FerroEhrService {
    /// SM `add_ehr_subject` (I1): associate `subject` with `ehr_id` with an
    /// optional status + location. The EHR must exist.
    ///
    /// NOTE: "Add" is realized as an idempotent upsert
    /// (`ON CONFLICT DO UPDATE`) — re-adding the same subject refreshes its
    /// status/location rather than erroring; the `0..1` cardinality of
    /// `add_ehr_subject` permits this. Status defaults to a `Primary` instance
    /// (`i_ehr_index.adoc`).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — `ehr_id` is not a well-formed UUID,
    ///   or a `start_valid_time`/`end_valid_time` is not an ISO date-time.
    /// - `ehr_id_does_not_exist` — no EHR with that id.
    /// - `exception` — a database fault while writing.
    pub async fn add_ehr_subject(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        status: Option<ResourceStatus>,
        loc: Option<LocationDesc>,
    ) -> Result<(), SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        self.index_ehr_exists(ehr_id).await?;
        let default_status = ResourceStatus::default();
        let status = status.as_ref().unwrap_or(&default_status);
        let start =
            parse_valid_time(status.start_valid_time.as_deref()).map_err(IndexError::Service)?;
        let end =
            parse_valid_time(status.end_valid_time.as_deref()).map_err(IndexError::Service)?;
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
        .bind(location_binding(loc.as_ref()))
        .execute(&self.pool)
        .await
        .map_err(IndexError::from)?;
        Ok(())
    }

    /// SM `update_ehr_subject_status` (I2): update the status of an existing
    /// (`ehr_id`, `subject`) association.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — `ehr_id` is not a well-formed UUID,
    ///   or a `start_valid_time`/`end_valid_time` is not an ISO date-time.
    /// - `ehr_id_does_not_exist` — no EHR with that id.
    /// - `subject_id_does_not_exist` — the subject is not associated with the
    ///   EHR (the update matched no row).
    /// - `exception` — a database fault while writing.
    pub async fn update_ehr_subject_status(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        status: ResourceStatus,
    ) -> Result<(), SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        self.index_ehr_exists(ehr_id).await?;
        let start =
            parse_valid_time(status.start_valid_time.as_deref()).map_err(IndexError::Service)?;
        let end =
            parse_valid_time(status.end_valid_time.as_deref()).map_err(IndexError::Service)?;
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
        .await
        .map_err(IndexError::from)?;
        Ok(require_association(updated.rows_affected(), &subject)?)
    }

    /// SM `update_ehr_subject_loc_desc` (I3): update (or clear, `loc = None`)
    /// the location descriptor of an existing association.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — `ehr_id` is not a well-formed UUID.
    /// - `ehr_id_does_not_exist` — no EHR with that id.
    /// - `subject_id_does_not_exist` — the subject is not associated with the
    ///   EHR (the update matched no row).
    /// - `exception` — a database fault while writing.
    pub async fn update_ehr_subject_loc_desc(
        &self,
        ehr_id: String,
        subject: SubjectRef,
        loc: Option<LocationDesc>,
    ) -> Result<(), SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        self.index_ehr_exists(ehr_id).await?;
        let updated = sqlx::query(
            "UPDATE ehr_index SET location = $4 \
             WHERE ehr_id = $1 AND subject_id = $2 AND subject_namespace = $3",
        )
        .bind(ehr_id)
        .bind(&subject.id)
        .bind(&subject.namespace)
        .bind(location_binding(loc.as_ref()))
        .execute(&self.pool)
        .await
        .map_err(IndexError::from)?;
        Ok(require_association(updated.rows_affected(), &subject)?)
    }

    /// SM `remove_ehr_subject` (I4): drop the `subject`↔`ehr_id` association
    /// (the subject may remain associated with other EHRs).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — `ehr_id` is not a well-formed UUID.
    /// - `ehr_id_does_not_exist` — no EHR with that id.
    /// - `subject_id_does_not_exist` — the subject is not associated with the
    ///   EHR (the delete matched no row).
    /// - `exception` — a database fault while writing.
    pub async fn remove_ehr_subject(
        &self,
        ehr_id: String,
        subject: SubjectRef,
    ) -> Result<(), SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        self.index_ehr_exists(ehr_id).await?;
        let deleted = sqlx::query(
            "DELETE FROM ehr_index \
             WHERE ehr_id = $1 AND subject_id = $2 AND subject_namespace = $3",
        )
        .bind(ehr_id)
        .bind(&subject.id)
        .bind(&subject.namespace)
        .execute(&self.pool)
        .await
        .map_err(IndexError::from)?;
        Ok(require_association(deleted.rows_affected(), &subject)?)
    }

    /// SM `remove_subject` (I5): drop all associations for `subject`.
    ///
    /// # Errors
    /// - `subject_id_does_not_exist` — the subject has no associations (the
    ///   delete matched no row).
    /// - `exception` — a database fault while writing.
    pub async fn remove_subject(&self, subject: SubjectRef) -> Result<(), SmError> {
        let deleted =
            sqlx::query("DELETE FROM ehr_index WHERE subject_id = $1 AND subject_namespace = $2")
                .bind(&subject.id)
                .bind(&subject.namespace)
                .execute(&self.pool)
                .await
                .map_err(IndexError::from)?;
        Ok(require_association(deleted.rows_affected(), &subject)?)
    }

    /// The subjects associated with an EHR (design-filled read; the SM defines
    /// no read operations — our own design). Empty for an unknown EHR.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — `ehr_id` is not a well-formed UUID.
    /// - `exception` — a database fault while reading.
    pub async fn ehr_subjects(&self, ehr_id: String) -> Result<Vec<EhrIndexEntry>, SmError> {
        let ehr_id = parse_ehr_id(&ehr_id)?;
        let rows = sqlx::query(
            "SELECT ehr_id, subject_id, subject_namespace, subject_type, instance_type, \
             start_valid_time, end_valid_time, notes, location FROM ehr_index \
             WHERE ehr_id = $1 ORDER BY subject_id, subject_namespace",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await
        .map_err(IndexError::from)?;
        Ok(rows
            .iter()
            .map(row_to_entry)
            .collect::<Result<_, _>>()
            .map_err(IndexError::from)?)
    }

    /// The EHRs associated with a subject (design-filled read; the SM defines
    /// no read operations — our own design). Empty for an unknown subject.
    ///
    /// # Errors
    /// - `exception` — a database fault while reading.
    pub async fn subject_ehrs(&self, subject: SubjectRef) -> Result<Vec<EhrIndexEntry>, SmError> {
        Ok(self.index_subject_ehrs(&subject).await?)
    }

    /// Confirm an EHR exists ([`IndexError::EhrDoesNotExist`] →
    /// `ehr_id_does_not_exist` otherwise). This distinguishes an unknown EHR
    /// from an unknown association to the caller (`master07 §Errors`).
    async fn index_ehr_exists(&self, ehr_id: EhrId) -> Result<(), IndexError> {
        let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .fetch_optional(&self.pool)
            .await?;
        found.map(|_| ()).ok_or(IndexError::EhrDoesNotExist(ehr_id))
    }

    /// The EHRs associated with a subject, as [`EhrIndexEntry`]s — shared by
    /// [`Self::subject_ehrs`] and the duplicate-detection scan
    /// ([`super::conflicts`]).
    pub(super) async fn index_subject_ehrs(
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
