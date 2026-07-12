//! `I_EHR_CONTRIBUTION` (`i_ehr_contribution.adoc`) — the explicit
//! CONTRIBUTION-level surface: has/get/commit/list/count, plus the ITS-REST
//! raw-wire commit seam (`create_ehr_contribution`).
//!
//! Spec: RM common `master06-change_control_package.adoc` §Contributions /
//! §Committal and Audits. The change-set engine itself (classify, atomic
//! multi-version commit, retrieval assembly) is versioning law and lives in
//! [`crate::versioning`] (`commit_version_set` / `get_contribution` /
//! `list_contributions` / `count_contributions`); this file keeps only the
//! I_EHR_CONTRIBUTION surface and the wire glue. The `Pre_has_ehr` guard (G-6)
//! is enforced inside `commit_version_set` via the
//! [`crate::versioning::CommitEnv`] `ensure_ehr_exists` hook.

use ehrbase_sm::{
    ContributionAdapter, EhrContributionService, Page, ResourceMeta, ServiceResponse, SmError,
    UpdateAudit, UpdateVersion,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
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
    /// `update_version.adoc`, both 1..1) and a committer, so it cannot represent
    /// an attestation-only (`666`) member, a delete (`523`) member, or a member
    /// inheriting `committer`/`system_id` from the CONTRIBUTION audit (RM common
    /// master06 §Committal m4). This raw-body seam carries the full-fidelity EHR
    /// CONTRIBUTION commit; all RM change_control semantics stay in
    /// `crate::versioning::commit_version_set`.
    ///
    /// TODO(w3f-integrate): `commit_version_set(self, …)` requires
    /// `impl CommitEnv for EhrbaseService` (wired at the fix pass).
    ///
    /// # Errors
    /// [`SmError`] if the CONTRIBUTION fails classification, content validation,
    /// the optimistic lock, or its storage commit.
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
    pub(in crate::service) async fn ehr_contribution(
        &self,
        ehr_id: Uuid,
        contribution_id: Uuid,
        resolve_refs: bool,
    ) -> Result<Value, ServiceError> {
        // TODO(w3f-integrate): retrieval seam (crate::versioning::get_contribution
        // over storage's contribution_audit/contribution_version_refs).
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

#[async_trait::async_trait]
impl EhrContributionService for EhrbaseService {
    async fn has_contribution(&self, an_ehr_id: Uuid, a_contrib_id: Uuid) -> Result<bool, SmError> {
        match self.ehr_contribution(an_ehr_id, a_contrib_id, false).await {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn get_contribution(
        &self,
        an_ehr_id: Uuid,
        a_contrib_id: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .ehr_contribution(an_ehr_id, a_contrib_id, false)
            .await?)
    }

    async fn get_contribution_resolved(
        &self,
        an_ehr_id: Uuid,
        a_contrib_id: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self.ehr_contribution(an_ehr_id, a_contrib_id, true).await?)
    }

    async fn commit_contribution(
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
        // PORT NOTE: this typed → wire-JSON → re-parse round-trip is a known glue
        // seam. The typed shapes differ from the raw wire in two ways
        // `commit_version_set` tolerates explicitly: `preceding_version_uid:
        // None` serializes to JSON `null` (not absent), and `change_type` is a
        // `Terminology_code` (`{terminology_id, code_string}`, SM
        // `update_audit.adoc`), not a `DV_CODED_TEXT` (see the PORT NOTEs in
        // `versioning/contribution.rs` `coded_value`/`classify`). A native typed
        // path skipping the JSON round-trip is a future cleanup — not W-3f.
        //
        // TODO(w3f-integrate): requires `impl CommitEnv for EhrbaseService`.
        let versions_json =
            serde_json::to_value(&versions).map_err(|e| SmError::exception(e.to_string()))?;
        let audit_json =
            serde_json::to_value(&an_audit).map_err(|e| SmError::exception(e.to_string()))?;
        let body = json!({ "versions": versions_json, "audit": audit_json });
        let id = commit_version_set(self, Some(an_ehr_id), &body, false).await?;
        Ok(id.to_string())
    }

    async fn list_contributions(
        &self,
        an_ehr_id: Uuid,
        time_range: ehrbase_sm::TimeRange,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        let time_range = parse_time_range(time_range)?;
        // `Pre_has_ehr` (`i_ehr_contribution.adoc` §list_contributions —
        // `ehr_does_not_exist`) is enforced inside the versioning read.
        // TODO(w3f-integrate): list seam (crate::versioning::list_contributions).
        let ids = list_contributions(&self.pool, an_ehr_id, time_range, page).await?;
        Ok(ids.iter().map(Uuid::to_string).collect())
    }

    async fn contribution_count(
        &self,
        an_ehr_id: Uuid,
        time_range: ehrbase_sm::TimeRange,
    ) -> Result<i64, SmError> {
        let time_range = parse_time_range(time_range)?;
        Ok(count_contributions(&self.pool, an_ehr_id, time_range).await?)
    }
}

#[async_trait::async_trait]
impl ContributionAdapter for EhrbaseService {
    async fn ehr_contribution_commit(
        &self,
        an_ehr_id: Uuid,
        a_contribution: Value,
    ) -> Result<ServiceResponse, SmError> {
        self.create_ehr_contribution(an_ehr_id, a_contribution)
            .await
    }
}
