//! `I_EHR_SERVICE` (`i_ehr_service.adoc`) + `EHR_SUMMARY` (`ehr_summary.adoc`):
//! EHR create (4 variants), `has_ehr`(`_for_subject`), `get_ehr(s)`, and the
//! folder-hierarchy reads the `EHR` wire body needs.
//!
//! Spec: arch-overview `master06-design_of_the_ehr.adoc` §The EHR (EHR root,
//! `system_id`, EHR_ACCESS, EHR_STATUS, directory, folders, time_created) and
//! RM ehr `master04-ehr_package.adoc` §EHR Creation / §Folders. The EHR-table
//! and folder-membership SQL is a storage seam (G-10; no openEHR spec governs
//! the schema — our own design).

use ehrbase_sm::{EhrService, EhrSummary, ResourceMeta, ServiceResponse, SmError, SubjectRef};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{Change, Kind, change_type, commit_contribution};

use super::{status_for_subject, version_uid};

impl EhrbaseService {
    /// Create an EHR (with the given id), its initial `EHR_STATUS`, and its
    /// `EHR_ACCESS`, all committed under **one** CONTRIBUTION — RM ehr master04
    /// §EHR Creation: "the result should be a root EHR object, an EHR Status
    /// object, and an EHR Access object … created and committed in a
    /// Contribution". Shared by `POST /ehr` and `PUT /ehr/{ehr_id}`.
    ///
    /// A duplicate subject conflicts at the database (`ehr_subject_uq`, kept in
    /// sync by [`Self::sync_ehr_subject`]) → 409 (ITS-REST `409_EHR.yaml`; CNF
    /// `create_ehr-two_ehrs_same_patient`).
    pub(in crate::service) async fn create_ehr(
        &self,
        ehr_id: Uuid,
        status: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        // The supplied EHR_STATUS must be a structurally valid RM instance before
        // the EHR is created (CNF master06 §Test Data Sets INVALID class 2).
        super::validate_ehr_status(&status)?;

        let mut tx = self.pool.begin().await?;

        // EHR.system_id is recorded at creation, immutable thereafter (arch
        // master06 §System Identity — a stored value, not the live config).
        // TODO(w3f-integrate): storage seam (G-10) — the `ehr` row insert.
        let inserted =
            sqlx::query("INSERT INTO ehr (id, system_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(ehr_id)
                .bind(self.effective_system_id())
                .execute(&mut *tx)
                .await?;
        if inserted.rows_affected() == 0 {
            return Err(ServiceError::Conflict(format!(
                "EHR {ehr_id} already exists"
            )));
        }

        let audit = self.audit(change_type::CREATION, "EHR creation");
        // TODO(w3f-integrate): commit seam (crate::versioning::commit_contribution)
        // + signing_ctx (becomes crate::versioning::SigningCtx at the fix pass).
        commit_contribution(
            &mut tx,
            Some(ehr_id),
            None,
            &audit,
            vec![
                (
                    audit.clone(),
                    Change::Create {
                        kind: Kind::EhrStatus,
                        canonical: status.clone(),
                        template_id: None,
                        signature: None,
                        lifecycle_state: None,
                        attestations: Vec::new(),
                    },
                ),
                (
                    audit.clone(),
                    Change::Create {
                        kind: Kind::EhrAccess,
                        canonical: super::default_ehr_access(),
                        template_id: None,
                        signature: None,
                        lifecycle_state: None,
                        attestations: Vec::new(),
                    },
                ),
            ],
            Vec::new(),
            &self.signing_ctx(),
        )
        .await?;
        // Keep the promoted subject columns in sync with the committed EHR_STATUS
        // (one EHR per subject — RM ehr master04 §EHR Status; the sync hook).
        self.sync_ehr_subject(&mut tx, ehr_id, &status).await?;
        tx.commit().await?;

        self.ehr_summary(ehr_id).await
    }

    /// Find an EHR by the subject its current `EHR_STATUS` names (external ref
    /// `id.value` + `namespace`). Served from the promoted `ehr.subject_*`
    /// columns (unique per subject — `ehr_subject_uq`).
    ///
    /// PORT NOTE (G-4, `i_ehr_service.adoc` §get_ehrs_for_subject): the DB
    /// constraint narrows the SM `List<EHR_SUMMARY>` to ≤1. CNF
    /// `create_ehr-two_ehrs_same_patient` expects **409** on a second EHR for the
    /// same subject, which supports the one-EHR-per-subject rule (RM ehr master04
    /// §EHR Status: the subject is 0..1 and identifies the EHR); kept, cited.
    pub(in crate::service) async fn ehr_by_subject(
        &self,
        subject_id: &str,
        namespace: &str,
    ) -> Result<ServiceResponse, ServiceError> {
        // TODO(w3f-integrate): storage seam (G-10) — the promoted subject-column
        // lookup.
        let ehr_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM ehr WHERE subject_id = $1 AND subject_namespace = $2",
        )
        .bind(subject_id)
        .bind(namespace)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ServiceError::NotFound(format!("EHR for subject {subject_id}@{namespace}"))
        })?;
        self.ehr_summary(ehr_id).await
    }

    /// Build the canonical RM `EHR` object for an existing EHR, with its `ehr_id`
    /// metadata (the `ETag`/`Location` for `POST /ehr`). ITS-REST extension: the
    /// wire `GET /ehr/{id}` returns the RM `EHR`, not the SM `EHR_SUMMARY`.
    pub(in crate::service) async fn ehr_summary(
        &self,
        ehr_id: Uuid,
    ) -> Result<ServiceResponse, ServiceError> {
        // TODO(w3f-integrate): storage seam (G-10) — the `ehr` row read.
        let row = sqlx::query("SELECT system_id, time_created FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR {ehr_id}")))?;
        // EHR.system_id is IMMUTABLE after creation (arch master06 §System
        // Identity) — the stored per-EHR value, never the live config.
        let stored_system_id: String = row.try_get("system_id")?;
        let time_created: jiff::Timestamp = row
            .try_get::<jiff_sqlx::Timestamp, _>("time_created")?
            .to_jiff();

        let (status_vo, status_version) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        // The uid uses the stored per-version creating_system_id (master06
        // §Distributed Versioning), stable across a `with_system_id` change.
        let (t, b, v) = status_version.columns();
        // TODO(w3f-integrate): storage seam (G-10) — per-version creating_system_id.
        let status_csid: String = sqlx::query_scalar(
            "SELECT creating_system_id FROM vo_version WHERE vo_id = $1 \
             AND trunk_version = $2 AND branch_number = $3 AND branch_version = $4",
        )
        .bind(status_vo)
        .bind(t)
        .bind(b)
        .bind(v)
        .fetch_one(&self.pool)
        .await?;
        let status_ovid = self.object_version_id(status_vo, &status_csid, status_version);

        let mut body = json!({
            "_type": "EHR",
            "system_id": { "_type": "HIER_OBJECT_ID", "value": stored_system_id },
            "ehr_id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() },
            // Ehr_status_valid: ehr_status.type.is_equal("VERSIONED_EHR_STATUS")
            // (RM ehr `ehr.adoc` invariants — normative). PORT NOTE (spec
            // defect): the non-normative ITS-REST example shows `type:
            // EHR_STATUS`; the RM invariant wins for `type`, the id keeps the
            // OBJECT_VERSION_ID form clients use to address the current version.
            "ehr_status": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_EHR_STATUS",
                "id": { "_type": "OBJECT_VERSION_ID", "value": status_ovid }
            },
            "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
        });
        // EHR.ehr_access (1..1): a reference to the VERSIONED_EHR_ACCESS container
        // (invariant Ehr_access_valid — RM ehr, EHR class). Every EHR this
        // service creates has one; tolerate absence only for raw fixtures.
        if let Some((access_vo, _)) = self.current_vo(ehr_id, Kind::EhrAccess).await? {
            body["ehr_access"] = json!({
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_EHR_ACCESS",
                "id": { "_type": "HIER_OBJECT_ID", "value": access_vo.to_string() }
            });
        }
        // EHR.folders (0..1) + EHR.directory (0..1): the LIVE hierarchies in rank
        // order, each an OBJECT_REF to a VERSIONED_FOLDER (invariant Folders_valid;
        // directory = folders.item(1), Directory_in_folders — RM ehr, EHR class).
        let folders = self.live_folder_hierarchies(ehr_id).await?;
        let refs: Vec<Value> = folders
            .iter()
            .map(|vo| {
                json!({
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": "VERSIONED_FOLDER",
                    "id": { "_type": "HIER_OBJECT_ID", "value": vo.to_string() }
                })
            })
            .collect();
        if let Some(first) = refs.first() {
            body["directory"] = first.clone();
            body["folders"] = Value::Array(refs.clone());
        }
        let meta = ResourceMeta::new(ehr_id.to_string(), ehr_id.to_string())
            .with_last_modified(time_created);
        Ok(ServiceResponse::new(body, meta))
    }

    /// SM `EHR_SUMMARY` (`ehr_summary.adoc`) — all six mandatory attributes.
    /// `system_id` is the stored `EHR.system_id`; `ehr_status` is the current
    /// bare `EHR_STATUS`; `composition_count` is the number of "(versioned)
    /// Compositions" — distinct versioned objects (`vo_id`), not versions.
    pub(in crate::service) async fn summarize_ehr(
        &self,
        ehr_id: Uuid,
    ) -> Result<EhrSummary, ServiceError> {
        // TODO(w3f-integrate): storage seam (G-10) — the `ehr`/`contribution`/
        // `vo_version` counts below.
        let row = sqlx::query("SELECT system_id, time_created FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR {ehr_id}")))?;
        let stored_system_id: String = row.try_get("system_id")?;
        let time_created: jiff::Timestamp = row
            .try_get::<jiff_sqlx::Timestamp, _>("time_created")?
            .to_jiff();

        // Copy of EHR.ehr_status: the current EHR_STATUS (bare, with its uid).
        let ehr_status = self.status_at(ehr_id, None).await?.body;

        let contribution_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM contribution WHERE ehr_id = $1")
                .bind(ehr_id)
                .fetch_one(&self.pool)
                .await?;
        let composition_count: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT vo_id) FROM vo_version WHERE ehr_id = $1 AND kind = 'COMPOSITION'",
        )
        .bind(ehr_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(EhrSummary {
            ehr_id: ehr_id.to_string(),
            system_id: stored_system_id,
            ehr_status,
            time_created: time_created.to_string(),
            contribution_count,
            composition_count,
        })
    }

    /// The LIVE folder hierarchies of an EHR in `rank` order — the members of
    /// `EHR.folders` (RM ehr, EHR class `Folders_valid`; RM ehr master04
    /// §Folders). "Live" = the current trunk version exists and is not logically
    /// deleted (lifecycle `523`). Empty when the EHR indexes no live hierarchy.
    pub(in crate::service) async fn live_folder_hierarchies(
        &self,
        ehr_id: Uuid,
    ) -> Result<Vec<Uuid>, ServiceError> {
        // TODO(w3f-integrate): storage seam (G-10) — the `ehr_folder` ⋈
        // `vo_version` membership read; no openEHR spec governs the SQL.
        let rows = sqlx::query(
            "SELECT f.vo_id FROM ehr_folder f \
             JOIN vo_version v ON v.vo_id = f.vo_id \
             AND upper_inf(v.sys_period) AND v.branch_number = 0 \
             WHERE f.ehr_id = $1 AND v.lifecycle_state <> '523' \
             ORDER BY f.rank",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(|r| Ok(r.try_get("vo_id")?)).collect()
    }
}

/// The default `EHR_STATUS` for a new EHR (queryable, modifiable, `PARTY_SELF`).
pub(in crate::service) fn default_ehr_status() -> Value {
    json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": { "_type": "PARTY_SELF" },
        "is_queryable": true,
        "is_modifiable": true
    })
}

#[async_trait::async_trait]
impl EhrService for EhrbaseService {
    async fn has_ehr(&self, ehr_id: Uuid) -> Result<bool, SmError> {
        match self.ensure_ehr_exists(ehr_id).await {
            Ok(()) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn has_ehr_for_subject(&self, a_subject_id: SubjectRef) -> Result<bool, SmError> {
        match self
            .ehr_by_subject(&a_subject_id.id, &a_subject_id.namespace)
            .await
        {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn create_ehr(&self, an_ehr_status: Option<Value>) -> Result<Uuid, SmError> {
        // PORT NOTE (G-5, `i_ehr_service.adoc` §create_ehr `Pre_no_subject`): the
        // SM precondition `an_ehr_status.subject = Void` is NOT enforced on the
        // id-only create paths. `POST /ehr` intentionally accepts a
        // subject-bearing status (the ITS-REST `ehr` schema carries an optional
        // `ehr_status`, and the sync hook records the subject as the EHR's
        // patient), an accepted SM-vs-ITS-REST divergence: the subject slot is a
        // 0..1 `PARTY_SELF` (RM ehr master04 §EHR Status), and the CDR treats a
        // supplied anonymous-or-identified subject as the EHR's subject rather
        // than rejecting it. Recorded, not silently guessed.
        let ehr_id = Uuid::now_v7();
        let status = an_ehr_status.unwrap_or_else(default_ehr_status);
        self.create_ehr(ehr_id, status).await?;
        Ok(ehr_id)
    }

    async fn create_ehr_with_id(
        &self,
        an_ehr_id: Uuid,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        // G-5: see `create_ehr` — `Pre_no_subject` deliberately not enforced.
        let status = an_ehr_status.unwrap_or_else(default_ehr_status);
        self.create_ehr(an_ehr_id, status).await?;
        Ok(an_ehr_id)
    }

    async fn create_ehr_for_subject(
        &self,
        a_subject_id: SubjectRef,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        let ehr_id = Uuid::now_v7();
        let status = status_for_subject(
            an_ehr_status.unwrap_or_else(default_ehr_status),
            &a_subject_id,
        );
        self.create_ehr(ehr_id, status).await?;
        Ok(ehr_id)
    }

    async fn create_ehr_for_subject_with_id(
        &self,
        an_ehr_id: Uuid,
        a_subject_id: SubjectRef,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        let status = status_for_subject(
            an_ehr_status.unwrap_or_else(default_ehr_status),
            &a_subject_id,
        );
        self.create_ehr(an_ehr_id, status).await?;
        Ok(an_ehr_id)
    }

    async fn get_ehr(&self, an_ehr_id: Uuid) -> Result<EhrSummary, SmError> {
        Ok(self.summarize_ehr(an_ehr_id).await?)
    }

    async fn get_ehrs_for_subject(
        &self,
        a_subject_id: SubjectRef,
    ) -> Result<Vec<EhrSummary>, SmError> {
        // G-4: one EHR per subject narrows the List to ≤1 (see `ehr_by_subject`).
        match self
            .ehr_by_subject(&a_subject_id.id, &a_subject_id.namespace)
            .await
        {
            Ok(resp) => {
                let ehr_id = resp
                    .body
                    .pointer("/ehr_id/value")
                    .and_then(Value::as_str)
                    .and_then(|v| Uuid::parse_str(v).ok())
                    .ok_or_else(|| SmError::exception("EHR body carries no ehr_id"))?;
                Ok(vec![self.summarize_ehr(ehr_id).await?])
            }
            Err(ServiceError::NotFound(_)) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    async fn ehr_object(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.ehr_summary(an_ehr_id).await?.body)
    }

    async fn ehr_object_for_subject(
        &self,
        subject_id: &str,
        subject_namespace: &str,
    ) -> Result<Value, SmError> {
        Ok(self
            .ehr_by_subject(subject_id, subject_namespace)
            .await?
            .body)
    }
}

#[cfg(test)]
mod tests {
    use super::default_ehr_status;

    /// The default `EHR_STATUS` must be a valid structure root for the storage
    /// codec (one root node — the decomposition granularity of
    /// `crate::storage::decompose`).
    #[test]
    fn default_status_decomposes() {
        let rows = crate::storage::decompose(default_ehr_status()).expect("decompose");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rm_type, "EHR_STATUS");
    }
}
