//! Physical deletion (SM `I_ADMIN_SERVICE.physical_ehr_delete` /
//! `physical_party_delete`).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc`
//! (`physical_ehr_delete`, precondition `has_ehr`, error `ehr_id_does_not_exist`;
//! `physical_party_delete`, "along with related Party relationships", error
//! `party_id_does_not_exist`). Behaviour is anchored by the CNF Robot prior art
//! (`I_ADMIN_SERVICE/001-EHR.robot` + `admin_keywords.robot`): a physical,
//! cascading delete after which every backing table returns to its pre-EHR
//! baseline count. No openEHR spec governs the cascade SQL / FK graph — our own
//! design over the greenfield schema (`0001_baseline.sql`).

use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};

impl EhrbaseService {
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
    /// PORT NOTE (already-correct — `i_admin_service.adoc` defines the failure
    /// only abstractly (`ehr_id_does_not_exist`) with no HTTP binding): we map it
    /// to `NotFound` → HTTP `404`, the natural REST reading of an operation on a
    /// non-existent resource.
    async fn physical_ehr_delete(&self, ehr_id: Uuid) -> Result<(), ServiceError> {
        // Our own extension (no openEHR spec governs multimedia offload): when
        // DV_MULTIMEDIA externalization is on, collect the blob keys this EHR's
        // nodes reference *before* deletion, so we can GC the ones no other node
        // still references once the delete commits.
        let candidate_blobs = self.collect_ehr_blob_keys(ehr_id).await?;

        let mut tx = self.pool.begin().await?;

        // Capture the audit ids the EHR's versions and contributions reference,
        // before the cascade deletes those referencing rows.
        let audit_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT audit_id FROM vo_version WHERE ehr_id = $1 \
             UNION \
             SELECT audit_id FROM contribution WHERE ehr_id = $1",
        )
        .bind(ehr_id)
        .fetch_all(&mut *tx)
        .await?;

        // Delete the EHR — cascades vo_version (→ node), contribution, item_tag.
        let deleted = sqlx::query("DELETE FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .execute(&mut *tx)
            .await?;
        if deleted.rows_affected() == 0 {
            // `has_ehr(ehr_id)` is false → `ehr_id_does_not_exist`. Rolls back
            // (nothing was written), so the audit capture above is discarded.
            return Err(ServiceError::NotFound(format!("EHR {ehr_id}")));
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

    /// Collect the distinct externalized-blob keys referenced by an EHR's stored
    /// nodes (empty when externalization is disabled). Read-only, on the pool.
    /// Our own extension — no openEHR spec governs multimedia offload.
    async fn collect_ehr_blob_keys(&self, ehr_id: Uuid) -> Result<Vec<String>, ServiceError> {
        let Some(engine) = &self.multimedia else {
            return Ok(Vec::new());
        };
        let datas: Vec<serde_json::Value> =
            sqlx::query_scalar("SELECT data FROM node WHERE ehr_id = $1")
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

    /// Delete each candidate blob no longer referenced by any surviving `node`.
    /// Our own extension — no openEHR spec governs multimedia offload. A
    /// conservative scan-based GC (a `blob_ref` count table is a P20 scale
    /// nicety); a blob-store failure is logged, not fatal (the delete has
    /// committed, an orphaned blob is harmless).
    async fn gc_unreferenced_blobs(&self, candidates: Vec<String>) {
        let Some(engine) = &self.multimedia else {
            return;
        };
        for hex in candidates {
            let uri = engine.store().uri_for(&hex);
            let still_referenced: Result<bool, _> = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM node WHERE position($1 in data::text) > 0)",
            )
            .bind(&uri)
            .fetch_one(&self.pool)
            .await;
            match still_referenced {
                Ok(true) => {}
                Ok(false) => {
                    if let Err(e) = engine.store().delete(&hex).await {
                        tracing::warn!(blob = %hex, error = %e, "multimedia blob GC delete failed");
                    }
                }
                Err(e) => {
                    tracing::warn!(blob = %hex, error = %e, "multimedia blob GC reference scan failed");
                }
            }
        }
    }

    /// Physically delete a set of EHRs, each with the full cascade of
    /// [`Self::physical_ehr_delete`] in its own transaction. Missing ids are
    /// skipped (idempotent bulk delete); the count of EHRs actually deleted is
    /// returned.
    ///
    /// An **empty** id list means "delete ALL EHRs": the ITS-REST admin
    /// `DELETE /admin/ehr` types its `ehr_id` as an **optional** subset selector,
    /// so an absent/empty list denotes the full set.
    ///
    /// PORT NOTE (keep — spec-silent extension; G-A1): `i_admin_service.adoc`
    /// has no bulk call, so the idempotent skip-missing semantics + returned
    /// count are our own design (no openEHR spec governs bulk-delete internals);
    /// a partial success is observable at the REST edge.
    async fn physical_ehr_delete_all(
        &self,
        ehr_ids: &[Uuid],
    ) -> Result<u64, ServiceError> {
        // An empty selector = the full EHR set (see doc above); a non-empty
        // selector deletes exactly the named EHRs.
        let targets: Vec<Uuid> = if ehr_ids.is_empty() {
            sqlx::query_scalar("SELECT id FROM ehr")
                .fetch_all(&self.pool)
                .await?
        } else {
            ehr_ids.to_vec()
        };
        let mut deleted = 0u64;
        for ehr_id in targets {
            match self.physical_ehr_delete(ehr_id).await {
                Ok(()) => deleted += 1,
                // A missing EHR is skipped, not an error (idempotent bulk).
                Err(ServiceError::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(deleted)
    }

    /// `physical_party_delete` (`i_admin_service.adoc`): physically delete a
    /// PARTY "along with related Party relationships", in one transaction —
    /// mirroring [`Self::physical_ehr_delete`]'s capture-then-cascade approach.
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
    async fn party_physical_delete(&self, party_id: Uuid) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;

        // The target must be a demographic PARTY (ehr-less; any version exists).
        let kind: Option<String> =
            sqlx::query_scalar("SELECT kind FROM vo_version WHERE vo_id = $1 AND ehr_id IS NULL")
                .bind(party_id)
                .fetch_optional(&mut *tx)
                .await?;
        if !kind.as_deref().is_some_and(super::is_party_kind) {
            // `party_id_does_not_exist` → NotFound (→ 404). Rolls back cleanly.
            return Err(ServiceError::NotFound(format!("party {party_id}")));
        }

        // Every PARTY_RELATIONSHIP VO referencing the party as source/target, in
        // ANY version. The relationship stores `source`/`target` PARTY_REFs
        // inline in its root node's canonical fragment (they are DATA attributes,
        // not LOCATABLE children), so their `id.value` — the party's
        // versioned-object id — is matched with a jsonb path extraction.
        let party_txt = party_id.to_string();
        let rel_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT n.vo_id FROM node n \
             JOIN vo_version v ON v.vo_id = n.vo_id AND v.sys_version = n.sys_version \
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
            "SELECT contribution_id FROM vo_version WHERE vo_id = ANY($1) \
             UNION \
             SELECT contribution_id FROM vo_attestation WHERE vo_id = ANY($1)",
        )
        .bind(&vo_ids)
        .fetch_all(&mut *tx)
        .await?;
        let audit_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT audit_id FROM vo_version WHERE vo_id = ANY($1) \
             UNION \
             SELECT audit_id FROM contribution WHERE id = ANY($2)",
        )
        .bind(&vo_ids)
        .bind(&contribution_ids)
        .fetch_all(&mut *tx)
        .await?;

        // Delete the versioned objects — cascades node + vo_attestation.
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
