//! Physical EHR deletion (SM `I_ADMIN_SERVICE.physical_ehr_delete`).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc`
//! (`physical_ehr_delete`, precondition `has_ehr`, error
//! `ehr_id_does_not_exist`, requirement level 0..1). Behaviour is anchored by
//! the CNF Robot prior art (`I_ADMIN_SERVICE/001-EHR.robot` and its
//! `admin_keywords.robot`): a physical, cascading delete after which every
//! backing table returns to its pre-EHR baseline count.

use uuid::Uuid;

use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Physically delete one EHR and every trace of it, in a single transaction.
    ///
    /// The FK graph (`migrations/ehr/0001_schema.sql` + `0004_vo_attestation.sql`)
    /// makes `DELETE FROM ehr` cascade to `vo_version` (→ `node`,
    /// → `vo_attestation`), `contribution`, and `item_tag` (all `ON DELETE
    /// CASCADE`; `vo_attestation` cascades via its `(vo_id, sys_version)` FK to
    /// `vo_version`, and it carries no `audit` row of its own). The `audit` rows
    /// have **no** FK
    /// from `ehr` — `vo_version.audit_id`/`contribution.audit_id` reference
    /// `audit` (NO ACTION) — so the cascade cannot reach them and they would be
    /// orphaned. We therefore capture the referenced audit ids first, let the
    /// EHR delete cascade remove everything referencing `audit`, then delete the
    /// captured audit rows.
    ///
    /// Row-count 0 on the EHR delete means the EHR did not exist (`has_ehr`
    /// false) → [`ServiceError::NotFound`].
    ///
    /// PORT NOTE: SM `i_admin_service.adoc` defines the failure only abstractly
    /// (`ehr_id_does_not_exist`) with no HTTP binding (the ADMIN API is
    /// dev-branch only). We map it to `NotFound` → HTTP `404`, the natural REST
    /// reading of an operation on a non-existent resource.
    pub(super) async fn physical_ehr_delete(&self, ehr_id: Uuid) -> Result<(), ServiceError> {
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
        // vo_version.contribution_id → contribution(id) is NO ACTION (checked at
        // end of statement), and both sides cascade from `ehr`, so the single
        // statement resolves without an intra-cascade FK violation.
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
        Ok(())
    }

    /// Physically delete a set of EHRs, each with the full cascade of
    /// [`physical_ehr_delete`] in its own transaction. Missing ids are skipped
    /// (idempotent bulk delete); the count of EHRs actually deleted is returned.
    ///
    /// PORT NOTE: bulk delete has no spec (not in the SM, not in any OAS). The
    /// idempotent skip-missing semantics + returned count are our own design so
    /// a partial success is observable at the REST edge.
    pub(super) async fn physical_ehr_delete_all(
        &self,
        ehr_ids: &[Uuid],
    ) -> Result<u64, ServiceError> {
        let mut deleted = 0u64;
        for &ehr_id in ehr_ids {
            match self.physical_ehr_delete(ehr_id).await {
                Ok(()) => deleted += 1,
                // A missing EHR is skipped, not an error (idempotent bulk).
                Err(ServiceError::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(deleted)
    }
}
