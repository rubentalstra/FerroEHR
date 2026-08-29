// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Design-filled advisory duplicate-detection read.
//!
//! `master07 §Overview` names two error states the index metadata exists "to
//! detect and rectify": multiple EHRs recorded for one subject, and multiple
//! subjects recorded for one EHR. The SM defines no detection *operation* —
//! this read is our own design (advisory, never a hard reject: the N:M states
//! are legal-but-flagged per `resource_instance_type.adoc` `Duplicate`).

use sqlx::Row;

use crate::ids::EhrId;
use crate::service::FerroEhrService;
use crate::service::ehr_index::types::{EhrIndexEntry, SubjectRef};

use super::{IndexError, row_to_entry};

/// One detected index error state (master07 §Overview) — advisory only.
///
/// `pub`: the SM `I_EHR_INDEX` defines no detection operation (this read fills
/// that silence — our own design), so it has no SM trait binding and no
/// ITS-REST wire binding. It is exposed on the public [`FerroEhrService`]
/// surface as a native-API-only diagnostic; a route or admin-CLI binding would
/// be a spec-silent extension of our own, not a conformance requirement.
#[derive(Debug, Clone)]
pub enum IndexConflict {
    /// One subject is associated with more than one EHR (the
    /// "multiple EHRs … created in different locations" case). Carries every
    /// association of that subject so the operator can pick the `Primary`.
    SubjectWithMultipleEhrs {
        /// The subject the associations share, with its stored type.
        subject: SubjectRef,
        /// Every association of that subject, ordered by EHR id.
        entries: Vec<EhrIndexEntry>,
    },
    /// One EHR is associated with more than one subject (the
    /// "records merged … multiple subject ids" case).
    EhrWithMultipleSubjects {
        /// The EHR the associations share.
        ehr_id: EhrId,
        /// Every association of that EHR, ordered by subject key.
        entries: Vec<EhrIndexEntry>,
    },
}

impl FerroEhrService {
    /// Scan the index for the two master07 error states, returning every
    /// conflicting association group (empty = clean). Advisory: detection only,
    /// no mutation — rectification is the operator's `update_ehr_subject_status`
    /// / `remove_ehr_subject` call (I2/I4).
    ///
    /// # Errors
    /// [`IndexError::Service`] on a storage/database fault.
    pub async fn index_conflicts(&self) -> Result<Vec<IndexConflict>, IndexError> {
        let mut conflicts = Vec::new();

        // Subjects associated with more than one EHR.
        let subject_rows = sqlx::query(
            "SELECT subject_id, subject_namespace FROM ehr_index \
             GROUP BY subject_id, subject_namespace HAVING count(DISTINCT ehr_id) > 1 \
             ORDER BY subject_id, subject_namespace",
        )
        .fetch_all(&self.pool)
        .await?;
        for row in &subject_rows {
            let subject_id: String = row.try_get("subject_id")?;
            let namespace: String = row.try_get("subject_namespace")?;
            let subject = SubjectRef {
                id: subject_id,
                namespace,
                r#type: "PERSON".to_owned(),
            };
            let entries = self.index_subject_ehrs(&subject).await?;
            // Preserve the stored subject type (all associations share the key).
            let subject = entries.first().map_or(subject, |e| e.subject.clone());
            conflicts.push(IndexConflict::SubjectWithMultipleEhrs { subject, entries });
        }

        // EHRs associated with more than one subject.
        let ehr_rows = sqlx::query(
            "SELECT ehr_id FROM ehr_index \
             GROUP BY ehr_id HAVING count(*) > 1 ORDER BY ehr_id",
        )
        .fetch_all(&self.pool)
        .await?;
        for row in &ehr_rows {
            let ehr_id: EhrId = row.try_get("ehr_id")?;
            let rows = sqlx::query(
                "SELECT ehr_id, subject_id, subject_namespace, subject_type, instance_type, \
                 start_valid_time, end_valid_time, notes, location FROM ehr_index \
                 WHERE ehr_id = $1 ORDER BY subject_id, subject_namespace",
            )
            .bind(ehr_id)
            .fetch_all(&self.pool)
            .await?;
            let entries = rows
                .iter()
                .map(row_to_entry)
                .collect::<Result<Vec<_>, _>>()?;
            conflicts.push(IndexConflict::EhrWithMultipleSubjects { ehr_id, entries });
        }

        Ok(conflicts)
    }
}
