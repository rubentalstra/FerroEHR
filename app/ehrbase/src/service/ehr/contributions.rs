//! `I_EHR_CONTRIBUTION` (`i_ehr_contribution.adoc`) — the explicit
//! CONTRIBUTION-level surface: has/get/commit/list/count, plus the ITS-REST
//! raw-wire commit seam (`create_ehr_contribution`).
//!
//! Spec: RM common `master06-change_control_package.adoc` §Contributions /
//! §Committal and Audits. The change-set engine itself (classify, atomic
//! multi-version commit, retrieval assembly) is versioning law and lives in
//! [`crate::versioning`] (`commit_version_set` / `get_contribution` /
//! `list_contributions` / `count_contributions`); this file keeps only the
//! `I_EHR_CONTRIBUTION` surface and the wire glue. The `Pre_has_ehr` guard
//! (G-6) is enforced inside `commit_version_set` via the
//! [`crate::versioning::CommitEnv`] `ensure_ehr_exists` hook.

use crate::service::list::Page;
use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::status::SmError;
use crate::service::version_update::{UpdateAudit, UpdateVersion};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::versioning::contribution::{
    commit_version_set, count_contributions, get_contribution, list_contributions,
};

use super::parse_time_range;

impl EhrbaseService {
    /// `POST /ehr/{ehr_id}/contribution` — commit a raw wire CONTRIBUTION body
    /// atomically and return the stored `CONTRIBUTION` with its resource
    /// metadata (the `contribution_uid` for the `201` `ETag`/`Location`).
    ///
    /// PORT NOTE: the SM native `commit_contribution(Vec<UpdateVersion>,
    /// UpdateAudit)` is a *typed subset* of the wire CONTRIBUTION —
    /// `UPDATE_VERSION` mandates `data` + `lifecycle_state` (SM
    /// `update_version.adoc`, both 1..1) and a committer, so it cannot
    /// represent an attestation-only (`666`) member, a delete (`523`) member,
    /// or a member inheriting `committer`/`system_id` from the CONTRIBUTION
    /// audit (RM common master06 §Committal m4). This raw-body seam carries
    /// the full-fidelity EHR CONTRIBUTION commit; all RM `change_control`
    /// semantics stay in `crate::versioning::contribution::commit_version_set` (over the
    /// [`crate::versioning::CommitEnv`] impl `EhrbaseService` provides).
    ///
    /// # Errors
    /// [`SmError`] if the CONTRIBUTION fails classification, content
    /// validation, the optimistic lock, or its storage commit.
    pub async fn create_ehr_contribution(
        &self,
        ehr_id: Uuid,
        body: Value,
    ) -> Result<ServiceResponse, SmError> {
        let contribution_id = commit_version_set(self, Some(ehr_id), &body, false).await?;
        let body = self
            .ehr_contribution(ehr_id, contribution_id, false)
            .await?;
        let meta = ResourceMeta::new(ehr_id.to_string(), contribution_id.to_string());
        Ok(ServiceResponse::new(body, meta))
    }

    /// Retrieve a CONTRIBUTION by id (scoped to the EHR), its `versions` as
    /// `OBJECT_REF`s, or — with `resolve_refs` — as the resolved
    /// `ORIGINAL_VERSION` objects (ITS-REST `Prefer: resolve_refs`,
    /// `Requests_and_responses` §Representation details negotiation).
    ///
    /// # Errors
    /// [`ServiceError::NotFound`] when the CONTRIBUTION does not exist in this
    /// EHR; [`ServiceError::Database`] on a storage failure.
    pub(in crate::service) async fn ehr_contribution(
        &self,
        ehr_id: Uuid,
        contribution_id: Uuid,
        resolve_refs: bool,
    ) -> Result<Value, ServiceError> {
        get_contribution(
            &self.pool,
            self.signer(),
            ehr_id,
            contribution_id,
            resolve_refs,
        )
        .await
    }
}

// ── The SM I_EHR_CONTRIBUTION call surface ────────────────────────────────────

impl EhrbaseService {
    /// SM `I_EHR_CONTRIBUTION.has_contribution` — whether the CONTRIBUTION
    /// exists in the EHR.
    ///
    /// # Errors
    /// [`SmError`] if the retrieval fails (a missing CONTRIBUTION is
    /// `Ok(false)`).
    pub async fn has_contribution(
        &self,
        an_ehr_id: Uuid,
        a_contrib_id: Uuid,
    ) -> Result<bool, SmError> {
        match self.ehr_contribution(an_ehr_id, a_contrib_id, false).await {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// SM `I_EHR_CONTRIBUTION.get_contribution` — the stored `CONTRIBUTION`
    /// (its `versions` as `OBJECT_REF`s).
    ///
    /// # Errors
    /// [`SmError`] when the CONTRIBUTION does not exist in this EHR
    /// (404-equivalent) or a read fails.
    pub async fn get_contribution(
        &self,
        an_ehr_id: Uuid,
        a_contrib_id: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .ehr_contribution(an_ehr_id, a_contrib_id, false)
            .await?)
    }

    /// The stored `CONTRIBUTION` with its `versions` resolved to the full
    /// `ORIGINAL_VERSION` objects (ITS-REST `Prefer: resolve_refs`).
    ///
    /// # Errors
    /// [`SmError`] when the CONTRIBUTION does not exist in this EHR
    /// (404-equivalent) or a read fails.
    pub async fn get_contribution_resolved(
        &self,
        an_ehr_id: Uuid,
        a_contrib_id: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self.ehr_contribution(an_ehr_id, a_contrib_id, true).await?)
    }

    /// SM `I_EHR_CONTRIBUTION.commit_contribution` — commit a typed change
    /// set atomically, returning the new CONTRIBUTION uid.
    ///
    /// # Errors
    /// [`SmError`] when the typed shapes fail to serialize, or the commit
    /// fails classification, content validation, the optimistic lock, or its
    /// storage write.
    pub async fn commit_contribution(
        &self,
        an_ehr_id: Uuid,
        versions: Vec<UpdateVersion>,
        an_audit: UpdateAudit,
    ) -> Result<String, SmError> {
        // Reassemble the wire CONTRIBUTION body `commit_version_set` parses:
        // `{ versions: [ { commit_audit, data, preceding_version_uid,
        // lifecycle_state, attestations, signature } … ], audit }`. The typed
        // shapes serialize to exactly those field names.
        //
        // PORT NOTE: this typed → wire-JSON → re-parse round-trip is a known
        // glue seam. The typed shapes differ from the raw wire in two ways
        // `commit_version_set` tolerates explicitly: `preceding_version_uid:
        // None` serializes to JSON `null` (not absent), and `change_type` is a
        // `Terminology_code` (`{terminology_id, code_string}`, SM
        // `update_audit.adoc`), not a `DV_CODED_TEXT` (see the PORT NOTEs in
        // `versioning/contribution.rs` `coded_value`/`classify`). A native
        // typed path skipping the JSON round-trip is a future cleanup.
        let versions_json =
            serde_json::to_value(&versions).map_err(|e| SmError::exception(e.to_string()))?;
        let audit_json =
            serde_json::to_value(&an_audit).map_err(|e| SmError::exception(e.to_string()))?;
        let body = json!({ "versions": versions_json, "audit": audit_json });
        let id = commit_version_set(self, Some(an_ehr_id), &body, false).await?;
        Ok(id.to_string())
    }

    /// SM `I_EHR_CONTRIBUTION.list_contributions` — the EHR's CONTRIBUTION
    /// uids, optionally bounded by `time_range`, paged.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `time_range` bound (400-equivalent), a
    /// missing EHR (`Pre_has_ehr`, enforced inside the versioning read), or a
    /// read failure.
    pub async fn list_contributions(
        &self,
        an_ehr_id: Uuid,
        time_range: crate::service::ehr::handle::TimeRange,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        let time_range = parse_time_range(time_range)?;
        // `Pre_has_ehr` (`i_ehr_contribution.adoc` §list_contributions —
        // `ehr_does_not_exist`) is enforced inside the versioning read.
        let ids = list_contributions(&self.pool, an_ehr_id, time_range, page).await?;
        Ok(ids.iter().map(Uuid::to_string).collect())
    }

    /// SM `I_EHR_CONTRIBUTION.contribution_count` — the number of
    /// CONTRIBUTIONs in the EHR, optionally bounded by `time_range`.
    ///
    /// # Errors
    /// [`SmError`] for a malformed `time_range` bound (400-equivalent) or a
    /// read failure.
    pub async fn contribution_count(
        &self,
        an_ehr_id: Uuid,
        time_range: crate::service::ehr::handle::TimeRange,
    ) -> Result<i64, SmError> {
        let time_range = parse_time_range(time_range)?;
        Ok(count_contributions(&self.pool, an_ehr_id, time_range).await?)
    }

    /// The `POST /ehr/{ehr_id}/contribution` commit with the wire `Prefer`
    /// split: `return=representation` assembles the stored CONTRIBUTION body;
    /// `return=minimal` commits and returns just the metadata.
    ///
    /// # Errors
    /// [`SmError`] if the CONTRIBUTION fails classification, content
    /// validation, the optimistic lock, or its storage commit.
    pub async fn ehr_contribution_commit(
        &self,
        an_ehr_id: Uuid,
        a_contribution: Value,
        representation: bool,
    ) -> Result<ServiceResponse, SmError> {
        if representation {
            // `return=representation`: assemble the stored CONTRIBUTION body
            // (audit + version OBJECT_REFs) for the response.
            self.create_ehr_contribution(an_ehr_id, a_contribution)
                .await
        } else {
            // `return=minimal`: the response is headers-only, so commit and
            // return just the contribution uid (the `201` `ETag`/`Location`) —
            // the post-commit composite re-read the representation path pays is
            // pure waste here (RM common master06 §Committal — the commit
            // itself yields the new CONTRIBUTION id).
            let contribution_id =
                commit_version_set(self, Some(an_ehr_id), &a_contribution, false).await?;
            let meta = ResourceMeta::new(an_ehr_id.to_string(), contribution_id.to_string());
            Ok(ServiceResponse::new(Value::Null, meta))
        }
    }
}
