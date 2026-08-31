// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Physical deletion (SM `I_ADMIN_SERVICE.physical_ehr_delete` /
//! `physical_party_delete` + the `admin_ehr_delete_all` bulk extension).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc`
//! (`physical_ehr_delete`, precondition `has_ehr`, error `ehr_id_does_not_exist`;
//! `physical_party_delete`, "along with related Party relationships", error
//! `party_id_does_not_exist`). Behaviour is anchored by the CNF Robot prior art
//! (`I_ADMIN_SERVICE/001-EHR.robot` + `admin_keywords.robot`): a physical,
//! cascading delete after which every backing table returns to its pre-EHR
//! baseline count. No openEHR spec governs the cascade SQL / FK graph — our own
//! design over the greenfield schema (`0001_baseline.sql`).

#![cfg_attr(
    feature = "multimedia",
    expect(
        clippy::disallowed_types,
        reason = "owner-approved 2026-08-03 (#1694 family 3): EHR-Extract/TDD/dump-load compose \
                  over verbatim stored content (RM common master06 §Copying); the Value sites \
                  are multimedia-gated, so the expectation exists only where it is fulfilled"
    )
)]

use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::service::FerroEhrService;
use crate::service::error::ServiceError;
use crate::service::status::{CallStatusType, SmError};

impl FerroEhrService {
    /// SM `physical_ehr_delete`: physically delete one EHR and every trace of
    /// it.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — `ehr_id` is not a well-formed UUID.
    /// - `versioned_object_does_not_exist` (`404`) — no EHR with that id
    ///   (`has_ehr` false → `ehr_id_does_not_exist`).
    /// - `exception` — a database fault mid-transaction (rolled back).
    pub async fn admin_ehr_delete(&self, ehr_id: String) -> Result<(), SmError> {
        Ok(self
            .delete_ehr(EhrId(super::parse_uuid(&ehr_id, "EHR")?))
            .await?)
    }

    /// The `admin_ehr_delete_all` extension: physically delete a set of EHRs
    /// (each with the full [`Self::admin_ehr_delete`] cascade), returning the
    /// count actually deleted. An **empty** list means "delete ALL EHRs" (the
    /// ITS-REST admin `DELETE /admin/ehr` types `ehr_id` as an **optional**
    /// subset selector, so an absent/empty list denotes the full set).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — any id in the list is malformed
    ///   (the whole bulk request is rejected before any deletion runs).
    /// - `exception` — a database fault while deleting.
    pub async fn admin_ehr_delete_all(&self, ehr_ids: Vec<String>) -> Result<u64, SmError> {
        let ids: Vec<EhrId> = super::parse_uuid_list(&ehr_ids, "EHR")?
            .into_iter()
            .map(EhrId)
            .collect();
        Ok(self.delete_ehr_set(&ids).await?)
    }

    /// Admin extension `DELETE /admin/template/{template_id}`: physically delete
    /// one operational template by its wire `template_id` (case-insensitive,
    /// §Composite Identifiers and Case), evicting its `WebTemplate` cache entry.
    ///
    /// NOTE: no openEHR spec governs this operation — the ITS-REST Admin API
    /// defines only EHR deletes (`admin.openapi.yaml`). Our own design/extension,
    /// mirroring the SM `I_DEFINITION_ADL14` UUID-keyed delete
    /// ([`Self::delete_opt`]) but addressed by the wire id and guarded against
    /// orphaning committed clinical data.
    ///
    /// # Errors
    /// - `versioned_object_does_not_exist` (`404`) — no template with that id.
    /// - `409` (`ServiceError::Conflict`) — a `vo_version` row still references
    ///   the template (`vo_version.template_id` FK, `0001_baseline.sql`); a
    ///   physical delete must never orphan the compositions built on it.
    /// - `exception` — a database fault.
    pub async fn admin_template_delete(&self, template_id: String) -> Result<(), SmError> {
        Ok(self.delete_template_by_id(&template_id).await?)
    }

    /// Admin extension `DELETE /admin/query/{qualified_name}/{version}`:
    /// physically delete exactly one stored query (a single version row),
    /// case-insensitive on the qualified name (matching the PUT store path),
    /// exact on the version.
    ///
    /// NOTE: no openEHR spec governs this operation — the ITS-REST Admin API
    /// defines only EHR deletes. Our own design/extension; the SM
    /// `I_DEFINITION_QUERY.delete_query` deletes *every* version by name, whereas
    /// this admin surface targets a single `(name, version)` row.
    ///
    /// # Errors
    /// - `versioned_object_does_not_exist` (`404`) — no stored query at that
    ///   name + version.
    /// - `exception` — a database fault.
    pub async fn admin_query_delete(
        &self,
        qualified_name: String,
        version: String,
    ) -> Result<(), SmError> {
        Ok(self
            .delete_stored_query_version(&qualified_name, &version)
            .await?)
    }

    /// Delete one template by its wire id, refusing (409) while any committed
    /// version still references it. The reference count and the delete run in
    /// one transaction so the friendly 409 is consistent with the delete; the
    /// `vo_version.template_id` → `template_ref` foreign key
    /// (`0001_baseline.sql`, NO ACTION) is the underlying integrity guard that
    /// makes orphaning impossible even under a concurrent commit.
    async fn delete_template_by_id(&self, template_id: &str) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        // Resolve the stored (case-preserved) id; absent → 404 (§Composite
        // Identifiers and Case: compare case-insensitively).
        let stored: Option<String> = sqlx::query_scalar(
            "SELECT template_id FROM template_store WHERE lower(template_id) = lower($1)",
        )
        .bind(template_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(stored) = stored else {
            return Err(ServiceError::sm(
                CallStatusType::TemplateDoesNotExist,
                format!("template {template_id}"),
            ));
        };
        // Counted over BOTH storage tiers: the cold archival mirror is
        // foreign-key-free, so an archived composition's reference is invisible
        // to the `template_ref` FK and would be orphaned silently.
        let refs: i64 =
            sqlx::query_scalar("SELECT count(*) FROM vo_version_all WHERE template_id = $1")
                .bind(&stored)
                .fetch_one(&mut *tx)
                .await?;
        if refs > 0 {
            return Err(ServiceError::conflict(format!(
                "template '{stored}' is still referenced by {refs} committed version(s); \
                 delete those compositions before deleting the template"
            )));
        }
        sqlx::query("DELETE FROM template_store WHERE template_id = $1")
            .bind(&stored)
            .execute(&mut *tx)
            .await?;
        // Deregister the wire address unless a template-kind ADL2 artefact
        // also claims it (`template_ref` is the union of both dialects'
        // addresses; the FK blocks the deregistration if a concurrent commit
        // referenced it after the count above).
        sqlx::query(
            "DELETE FROM template_ref WHERE template_id = $1 AND NOT EXISTS \
             (SELECT 1 FROM adl2_artefact WHERE lower(hrid) = lower($1) \
              AND kind IN ('template', 'operational_template'))",
        )
        .bind(&stored)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        // Evict the derived-runtime cache for the deleted id (case-canonical key,
        // matching the SM delete path). No openEHR spec governs the cache.
        self.web_templates
            .invalidate(&crate::templates::identity::canonical_key(&stored))
            .await;
        Ok(())
    }

    /// SM `physical_party_delete`: physically delete a demographic PARTY "along
    /// with related Party relationships".
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — `a_party_id` is not a well-formed
    ///   UUID.
    /// - `versioned_object_does_not_exist` (`404`) — the id names no
    ///   demographic PARTY root (`party_id_does_not_exist`; a
    ///   `PARTY_RELATIONSHIP` or unknown id is also this failure).
    /// - `exception` — a database fault mid-transaction (rolled back).
    pub async fn physical_party_delete(&self, a_party_id: String) -> Result<(), SmError> {
        Ok(self
            .physical_delete_party(VoId(super::parse_uuid(&a_party_id, "party")?))
            .await?)
    }

    /// Physically delete one EHR and every trace of it, in a single transaction.
    ///
    /// The FK graph (`0001_baseline.sql`) makes `DELETE FROM ehr` cascade to
    /// `vo_version` (→ `node`, → `vo_attestation`), `contribution`, and
    /// `item_tag` (all `ON DELETE CASCADE`; `vo_attestation` cascades via its
    /// `(vo_id, sys_version)` FK to `vo_version`, and it carries no `audit` row
    /// of its own). The `audit` rows have **no** FK from `ehr` —
    /// `vo_version.audit_id` / `contribution.audit_id` reference `audit`
    /// (NO ACTION) — so the cascade cannot reach them and they would be
    /// orphaned. We therefore capture the referenced audit ids first, let the
    /// EHR delete cascade remove everything referencing `audit`, then delete the
    /// captured audit rows.
    ///
    /// Row-count 0 on the EHR delete means the EHR did not exist (`has_ehr`
    /// false) → [`ServiceError::NotFound`].
    ///
    /// NOTE (already-correct — `i_admin_service.adoc` defines the failure
    /// only abstractly (`ehr_id_does_not_exist`) with no HTTP binding): we map it
    /// to `NotFound` → HTTP `404`, the natural REST reading of an operation on a
    /// non-existent resource.
    async fn delete_ehr(&self, ehr_id: EhrId) -> Result<(), ServiceError> {
        // Our own extension (no openEHR spec governs multimedia offload): when
        // DV_MULTIMEDIA externalization is on, collect the blob keys this EHR's
        // nodes reference *before* deletion, so we can GC the ones no other node
        // still references once the delete commits.
        let candidate_blobs = self.collect_ehr_blob_keys(ehr_id).await?;

        let mut tx = self.pool.begin().await?;

        // Capture the audit ids the EHR's versions and contributions reference,
        // before the cascade deletes those referencing rows. Read over BOTH
        // storage tiers: an archived version still holds its audit row.
        let audit_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT audit_id FROM vo_version_all WHERE ehr_id = $1 \
             UNION \
             SELECT audit_id FROM contribution WHERE ehr_id = $1",
        )
        .bind(ehr_id)
        .fetch_all(&mut *tx)
        .await?;

        // The cold archival tier is foreign-key-free by design, so no cascade
        // reaches it — the EHR's archived rows and their markers are removed
        // explicitly (`crate::storage::version_repo::tier`).
        crate::storage::version_repo::tier::purge_ehrs(&mut tx, &[ehr_id]).await?;

        // Delete the EHR — cascades vo_version (→ node), contribution, item_tag.
        let deleted = sqlx::query("DELETE FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .execute(&mut *tx)
            .await?;
        if deleted.rows_affected() == 0 {
            // `has_ehr(ehr_id)` is false → `ehr_id_does_not_exist`. Rolls back
            // (nothing was written), so the audit capture above is discarded.
            return Err(ServiceError::sm(
                CallStatusType::EhrIdDoesNotExist,
                format!("EHR {ehr_id}"),
            ));
        }

        // The referencing vo_version/contribution rows are gone, so the audit
        // rows are now unreferenced and can be removed.
        if !audit_ids.is_empty() {
            sqlx::query("DELETE FROM audit WHERE id = ANY($1)")
                .bind(&audit_ids)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        // The EHR and its nodes are gone; GC any blob this EHR referenced that
        // no *surviving* node still references (content-addressed dedup means a
        // blob shared with another EHR/version must be kept).
        self.gc_unreferenced_blobs(candidate_blobs).await;
        Ok(())
    }

    /// Physically delete a set of EHRs, each with the full cascade of
    /// [`Self::delete_ehr`] in its own transaction. Missing ids are skipped
    /// (idempotent bulk delete); the count of EHRs actually deleted is
    /// returned.
    ///
    /// NOTE (keep — spec-silent extension): `i_admin_service.adoc` has no
    /// bulk call, so the idempotent skip-missing semantics + returned count are
    /// our own design (no openEHR spec governs bulk-delete internals); a
    /// partial success is observable at the REST edge.
    async fn delete_ehr_set(&self, ehr_ids: &[EhrId]) -> Result<u64, ServiceError> {
        // Chunking bounds each transaction's lock/WAL footprint on a
        // full-store wipe.
        const CHUNK: usize = 128;
        // An empty selector = the full EHR set (see `admin_ehr_delete_all`); a
        // non-empty selector deletes exactly the named EHRs.
        let targets: Vec<EhrId> = if ehr_ids.is_empty() {
            sqlx::query_scalar("SELECT id FROM ehr")
                .fetch_all(&self.pool)
                .await?
        } else {
            ehr_ids.to_vec()
        };
        // Three set statements per chunk, not a per-EHR transaction loop; a
        // missing id deletes zero rows, and `DELETE … RETURNING id` counts the
        // EHRs actually removed.
        let mut deleted = 0u64;
        for chunk in targets.chunks(CHUNK) {
            let candidate_blobs = self.collect_blob_keys_for(chunk).await?;
            let mut tx = self.pool.begin().await?;
            let audit_ids: Vec<Uuid> = sqlx::query_scalar(
                "SELECT audit_id FROM vo_version_all WHERE ehr_id = ANY($1) \
                 UNION \
                 SELECT audit_id FROM contribution WHERE ehr_id = ANY($1)",
            )
            .bind(chunk)
            .fetch_all(&mut *tx)
            .await?;
            crate::storage::version_repo::tier::purge_ehrs(&mut tx, chunk).await?;
            let removed: Vec<Uuid> =
                sqlx::query_scalar("DELETE FROM ehr WHERE id = ANY($1) RETURNING id")
                    .bind(chunk)
                    .fetch_all(&mut *tx)
                    .await?;
            if !audit_ids.is_empty() {
                sqlx::query("DELETE FROM audit WHERE id = ANY($1)")
                    .bind(&audit_ids)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            #[expect(
                clippy::as_conversions,
                reason = "the removed-row count widens exactly: usize is at most 64 bits \
                          on every supported target"
            )]
            let removed_rows = removed.len() as u64;
            deleted += removed_rows;
            self.gc_unreferenced_blobs(candidate_blobs).await;
        }
        Ok(deleted)
    }

    /// The distinct externalized-blob keys referenced by a SET of EHRs' nodes
    /// (empty when no object store is reachable) — one read for the whole
    /// chunk. Our own extension — no openEHR spec governs multimedia offload.
    ///
    /// NOTE: reachability, not `multimedia.enabled` — a deployment that stopped
    /// externalizing still has blobs to collect, and skipping them here would
    /// orphan every one of them in the bucket.
    #[cfg(feature = "multimedia")]
    async fn collect_blob_keys_for(&self, ehr_ids: &[EhrId]) -> Result<Vec<String>, ServiceError> {
        let Some(engine) = &self.multimedia else {
            return Ok(Vec::new());
        };
        let datas: Vec<serde_json::Value> =
            sqlx::query_scalar("SELECT data FROM node_all WHERE ehr_id = ANY($1)")
                .bind(ehr_ids)
                .fetch_all(&self.pool)
                .await?;
        let mut keys: Vec<String> = datas
            .iter()
            .flat_map(|d| engine.referenced_keys(d))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        Ok(keys)
    }

    /// The slim twin: externalization is compiled out, so no stored node
    /// references a blob and the key set is empty by construction.
    #[cfg(not(feature = "multimedia"))]
    #[expect(
        clippy::unused_async,
        reason = "the multimedia twin awaits; callers await unconditionally"
    )]
    async fn collect_blob_keys_for(&self, _ehr_ids: &[EhrId]) -> Result<Vec<String>, ServiceError> {
        Ok(Vec::new())
    }

    /// Collect the distinct externalized-blob keys referenced by an EHR's stored
    /// nodes (empty when externalization is disabled). Read-only, on the pool.
    /// Our own extension — no openEHR spec governs multimedia offload.
    #[cfg(feature = "multimedia")]
    async fn collect_ehr_blob_keys(&self, ehr_id: EhrId) -> Result<Vec<String>, ServiceError> {
        let Some(engine) = &self.multimedia else {
            return Ok(Vec::new());
        };
        let datas: Vec<serde_json::Value> =
            sqlx::query_scalar("SELECT data FROM node_all WHERE ehr_id = $1")
                .bind(ehr_id)
                .fetch_all(&self.pool)
                .await?;
        let mut keys: Vec<String> = datas
            .iter()
            .flat_map(|d| engine.referenced_keys(d))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        Ok(keys)
    }

    /// The slim twin: externalization is compiled out, so the key set is empty
    /// by construction.
    #[cfg(not(feature = "multimedia"))]
    #[expect(
        clippy::unused_async,
        reason = "the multimedia twin awaits; callers await unconditionally"
    )]
    async fn collect_ehr_blob_keys(&self, _ehr_id: EhrId) -> Result<Vec<String>, ServiceError> {
        Ok(Vec::new())
    }

    /// Delete each candidate blob no longer referenced by any surviving `node`.
    /// Our own extension — no openEHR spec governs multimedia offload. A
    /// conservative scan-based GC (a `blob_ref` count table is a scale
    /// nicety); a blob-store failure is logged, not fatal (the delete has
    /// committed, an orphaned blob is harmless).
    ///
    /// The reference check is ONE pass over `node` for the whole candidate
    /// set (the still-referenced keys fall out of a single scan joined
    /// against the candidate array), never a scan per blob.
    #[cfg(feature = "multimedia")]
    async fn gc_unreferenced_blobs(&self, candidates: Vec<String>) {
        let Some(engine) = &self.multimedia else {
            return;
        };
        if candidates.is_empty() {
            return;
        }
        let uris: Vec<String> = candidates
            .iter()
            .map(|hex| engine.store().uri_for(hex))
            .collect();
        let still_referenced: Vec<String> = match sqlx::query_scalar(
            "SELECT DISTINCT k.uri FROM node_all n \
             JOIN unnest($1::text[]) AS k(uri) ON position(k.uri in n.data::text) > 0",
        )
        .bind(&uris)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "multimedia blob GC reference scan failed; keeping all candidates");
                return;
            }
        };
        for (hex, uri) in candidates.iter().zip(&uris) {
            if still_referenced.contains(uri) {
                continue;
            }
            if let Err(e) = engine.store().delete(hex).await {
                tracing::warn!(blob = %hex, error = %e, "multimedia blob GC delete failed");
            }
        }
    }

    /// The slim twin: externalization is compiled out, so no blob can exist to
    /// collect.
    #[cfg(not(feature = "multimedia"))]
    #[expect(
        clippy::unused_async,
        reason = "the multimedia twin awaits; callers await unconditionally"
    )]
    async fn gc_unreferenced_blobs(&self, _candidates: Vec<String>) {}

    /// `physical_party_delete` (`i_admin_service.adoc`): physically delete a
    /// PARTY "along with related Party relationships", in one transaction —
    /// mirroring [`Self::delete_ehr`]'s capture-then-cascade approach.
    ///
    /// The target must be a demographic PARTY root (any version, ehr-less); a
    /// `PARTY_RELATIONSHIP` or a non-party/unknown id is `party_id_does_not_exist`
    /// → [`ServiceError::NotFound`] (→ HTTP `404`). Deleted physically: the party
    /// VO + every `PARTY_RELATIONSHIP` VO whose stored canonical `source`/`target`
    /// `PARTY_REF` references the party (see `service/demographic/`), with their
    /// `vo_version` rows (which cascade `node` + `vo_attestation` via the
    /// `(vo_id, sys_version)` FKs), the CONTRIBUTIONs/audit rows they orphan
    /// (guarded — a row shared with a survivor is kept), and any `vo_archive`
    /// markers. `audit` has no FK from `vo_version` (NO ACTION), so those rows
    /// are swept explicitly, as in the EHR delete.
    async fn physical_delete_party(&self, party_id: VoId) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;

        // The target must be a demographic PARTY (ehr-less; any version exists),
        // in either storage tier — an archived party is still deletable.
        let kind: Option<String> = sqlx::query_scalar(
            "SELECT kind FROM vo_version_all WHERE vo_id = $1 AND ehr_id IS NULL LIMIT 1",
        )
        .bind(party_id)
        .fetch_optional(&mut *tx)
        .await?;
        if !kind.as_deref().is_some_and(super::is_party_kind) {
            // `party_id_does_not_exist` → NotFound (→ 404). Rolls back cleanly.
            return Err(ServiceError::sm(
                CallStatusType::PartyIdDoesNotExist,
                format!("party {party_id}"),
            ));
        }

        // Every PARTY_RELATIONSHIP VO referencing the party as source/target, in
        // ANY version. The relationship stores `source`/`target` PARTY_REFs
        // inline in its root node's canonical fragment (they are DATA attributes,
        // not LOCATABLE children), so their `id.value` — the party's
        // versioned-object id — is matched with a jsonb path extraction.
        let party_txt = party_id.to_string();
        let rel_ids: Vec<VoId> = sqlx::query_scalar(
            "SELECT DISTINCT n.vo_id FROM node_all n \
             JOIN vo_version_all v ON v.vo_id = n.vo_id AND v.sys_version = n.sys_version \
             WHERE v.kind = 'PARTY_RELATIONSHIP' \
               AND (n.data #>> '{source,id,value}' = $1 OR n.data #>> '{target,id,value}' = $1)",
        )
        .bind(&party_txt)
        .fetch_all(&mut *tx)
        .await?;

        let mut vo_ids = rel_ids;
        vo_ids.push(party_id);

        // Capture the CONTRIBUTION + audit ids these VOs reference before the
        // vo_version delete cascades their node/attestation rows away.
        let contribution_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT contribution_id FROM vo_version_all WHERE vo_id = ANY($1) \
             UNION \
             SELECT contribution_id FROM vo_attestation_all WHERE vo_id = ANY($1)",
        )
        .bind(&vo_ids)
        .fetch_all(&mut *tx)
        .await?;
        let audit_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT audit_id FROM vo_version_all WHERE vo_id = ANY($1) \
             UNION \
             SELECT audit_id FROM contribution WHERE id = ANY($2)",
        )
        .bind(&vo_ids)
        .bind(&contribution_ids)
        .fetch_all(&mut *tx)
        .await?;

        // Delete the versioned objects — cascades node + vo_attestation. The
        // cold archival tier is foreign-key-free by design, so its rows and the
        // archive markers go explicitly (`crate::storage::version_repo::tier`).
        crate::storage::version_repo::tier::purge_vos(&mut tx, &vo_ids).await?;
        sqlx::query("DELETE FROM vo_version WHERE vo_id = ANY($1)")
            .bind(&vo_ids)
            .execute(&mut *tx)
            .await?;

        // Orphaned CONTRIBUTIONs (guarded: keep any still referenced by a
        // surviving version/attestation).
        sqlx::query(
            "DELETE FROM contribution c WHERE c.id = ANY($1) \
               AND NOT EXISTS (SELECT 1 FROM vo_version v WHERE v.contribution_id = c.id) \
               AND NOT EXISTS (SELECT 1 FROM vo_attestation a WHERE a.contribution_id = c.id)",
        )
        .bind(&contribution_ids)
        .execute(&mut *tx)
        .await?;

        // Orphaned audit rows (guarded the same way).
        sqlx::query(
            "DELETE FROM audit a WHERE a.id = ANY($1) \
               AND NOT EXISTS (SELECT 1 FROM vo_version v WHERE v.audit_id = a.id) \
               AND NOT EXISTS (SELECT 1 FROM contribution c WHERE c.audit_id = a.id)",
        )
        .bind(&audit_ids)
        .execute(&mut *tx)
        .await?;

        // Any archive markers for the deleted VOs.
        sqlx::query("DELETE FROM vo_archive WHERE vo_id = ANY($1)")
            .bind(&vo_ids)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}
