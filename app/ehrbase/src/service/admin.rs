//! Physical EHR deletion (SM `I_ADMIN_SERVICE.physical_ehr_delete`).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc`
//! (`physical_ehr_delete`, precondition `has_ehr`, error
//! `ehr_id_does_not_exist`, requirement level 0..1). Behaviour is anchored by
//! the CNF Robot prior art (`I_ADMIN_SERVICE/001-EHR.robot` and its
//! `admin_keywords.robot`): a physical, cascading delete after which every
//! backing table returns to its pre-EHR baseline count.

use ehrbase_sm::PlatformService;
use uuid::Uuid;

use super::{EhrbaseService, ServiceError};

/// Whether a `vo_version.kind` string names a demographic PARTY root (the five
/// concrete `ACTOR`/`PARTY` leaves) — as opposed to a `PARTY_RELATIONSHIP` or a
/// clinical versioned object.
fn is_party_kind(kind: &str) -> bool {
    matches!(kind, "AGENT" | "GROUP" | "ORGANISATION" | "PERSON" | "ROLE")
}

/// Whether a [`PlatformService`]'s CONTRIBUTIONs are EHR-scoped (`Some(true)` →
/// `ehr_id IS NOT NULL`), ehr-less (`Some(false)` → `ehr_id IS NULL`), or the
/// service is not a versioned-content service (`None` → statistics are trivially
/// empty/0 — `i_admin_service.adoc` `a_service` "Name of a versioned content
/// service"; design fixed-decision SM-4).
///
/// PORT NOTE: only `Ehr` (EHR-scoped) and `Demographic` (ehr-less) hold
/// contributions in this CDR; the remaining `PLATFORM_SERVICE` members
/// (`Admin`/`Definitions`/`Ehr_index`/`Message`/`Query`/`System_log`) are not
/// versioned-content services and yield nothing. Returned as a bool so the SQL
/// stays static (parameterized), never string-built.
fn contribution_ehr_scoped(service: PlatformService) -> Option<bool> {
    match service {
        PlatformService::Ehr => Some(true),
        PlatformService::Demographic => Some(false),
        _ => None,
    }
}

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
        // ADR-017: when DV_MULTIMEDIA externalization is on, collect the blob
        // keys this EHR's nodes reference *before* deletion, so we can GC the
        // ones no other node still references once the delete commits.
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

        // The EHR and its nodes are gone; GC any blob this EHR referenced that
        // no *surviving* node still references (content-addressed dedup means a
        // blob shared with another EHR/version must be kept).
        self.gc_unreferenced_blobs(candidate_blobs).await;
        Ok(())
    }

    /// Collect the distinct externalized-blob keys referenced by an EHR's stored
    /// nodes (empty when externalization is disabled). Read-only, on the pool.
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
    ///
    /// PORT NOTE (ADR-017 §5): this is a conservative scan-based GC — for each
    /// candidate we check whether its `s3://…` URI still appears in any node's
    /// `data`, deleting only the orphans. A `blob_ref` count table (O(1)
    /// bookkeeping) is deliberately deferred to P20 scale. A blob-store failure
    /// is logged, not fatal: the delete has already committed, and an orphaned
    /// blob is harmless (re-collected next time) — never fail the delete over a
    /// GC hiccup.
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

    // ── statistics (SM `I_ADMIN_SERVICE`) ───────────────────────────────────
    //
    // Each call takes a `PLATFORM_SERVICE` + an optional closed
    // `Interval<Iso8601_date_time>` (`i_admin_service.adoc`). The interval is
    // matched against the CONTRIBUTION/version audit `time_committed`; `lo`/`hi`
    // are pre-validated ISO strings (or `None` for an open bound) bound as
    // `::timestamptz` — the invalid-ISO `400` is enforced at the adapter.

    /// `list_contributions`: the ids of all CONTRIBUTIONs of the named
    /// versioned-content service within the (optional) time range, ordered by
    /// commit time. A non-content service yields the empty list.
    pub(super) async fn stat_list_contributions(
        &self,
        service: PlatformService,
        lo: Option<String>,
        hi: Option<String>,
    ) -> Result<Vec<String>, ServiceError> {
        let Some(ehr_scoped) = contribution_ehr_scoped(service) else {
            return Ok(Vec::new());
        };
        // Static SQL; `$3` selects EHR-scoped vs ehr-less contributions.
        Ok(sqlx::query_scalar(
            "SELECT c.id::text FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE (($3 AND c.ehr_id IS NOT NULL) OR (NOT $3 AND c.ehr_id IS NULL)) \
               AND ($1::timestamptz IS NULL OR a.time_committed >= $1::timestamptz) \
               AND ($2::timestamptz IS NULL OR a.time_committed <= $2::timestamptz) \
             ORDER BY a.time_committed, c.id",
        )
        .bind(lo)
        .bind(hi)
        .bind(ehr_scoped)
        .fetch_all(&self.pool)
        .await?)
    }

    /// `contribution_count`: the count of all CONTRIBUTIONs of the named service
    /// within the (optional) time range. A non-content service → 0.
    pub(super) async fn stat_contribution_count(
        &self,
        service: PlatformService,
        lo: Option<String>,
        hi: Option<String>,
    ) -> Result<i64, ServiceError> {
        let Some(ehr_scoped) = contribution_ehr_scoped(service) else {
            return Ok(0);
        };
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE (($3 AND c.ehr_id IS NOT NULL) OR (NOT $3 AND c.ehr_id IS NULL)) \
               AND ($1::timestamptz IS NULL OR a.time_committed >= $1::timestamptz) \
               AND ($2::timestamptz IS NULL OR a.time_committed <= $2::timestamptz)",
        )
        .bind(lo)
        .bind(hi)
        .bind(ehr_scoped)
        .fetch_one(&self.pool)
        .await?)
    }

    /// `versioned_composition_count`: the count of distinct COMPOSITION versioned
    /// objects with a version committed within the (optional) range.
    ///
    /// PORT NOTE: COMPOSITIONs are EHR-scoped, so only `a_service = Ehr` yields a
    /// non-zero count; every other member → 0 (COMPOSITIONs are not in its scope).
    pub(super) async fn stat_versioned_composition_count(
        &self,
        service: PlatformService,
        lo: Option<String>,
        hi: Option<String>,
    ) -> Result<i64, ServiceError> {
        if service != PlatformService::Ehr {
            return Ok(0);
        }
        Ok(sqlx::query_scalar(
            "SELECT count(DISTINCT v.vo_id) FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.kind = 'COMPOSITION' \
               AND ($1::timestamptz IS NULL OR a.time_committed >= $1::timestamptz) \
               AND ($2::timestamptz IS NULL OR a.time_committed <= $2::timestamptz)",
        )
        .bind(lo)
        .bind(hi)
        .fetch_one(&self.pool)
        .await?)
    }

    /// `composition_version_count`: the count of individual COMPOSITION version
    /// rows committed within the (optional) range. Scope gate as
    /// [`Self::stat_versioned_composition_count`].
    pub(super) async fn stat_composition_version_count(
        &self,
        service: PlatformService,
        lo: Option<String>,
        hi: Option<String>,
    ) -> Result<i64, ServiceError> {
        if service != PlatformService::Ehr {
            return Ok(0);
        }
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.kind = 'COMPOSITION' \
               AND ($1::timestamptz IS NULL OR a.time_committed >= $1::timestamptz) \
               AND ($2::timestamptz IS NULL OR a.time_committed <= $2::timestamptz)",
        )
        .bind(lo)
        .bind(hi)
        .fetch_one(&self.pool)
        .await?)
    }

    /// `physical_party_delete` (`i_admin_service.adoc`): physically delete a
    /// PARTY "along with related Party relationships", in one transaction —
    /// mirroring [`Self::physical_ehr_delete`]'s capture-then-cascade approach.
    ///
    /// The target must be a demographic PARTY root (any version, ehr-less); a
    /// `PARTY_RELATIONSHIP` or a non-party/unknown id is `party_id_does_not_exist`
    /// → [`ServiceError::NotFound`] (→ HTTP `404`). Deleted physically: the party
    /// VO + every `PARTY_RELATIONSHIP` VO whose stored canonical `source`/`target`
    /// `PARTY_REF` references the party (see `service/relationship.rs`), with
    /// their `vo_version` rows (which cascade `node` + `vo_attestation` via the
    /// `(vo_id, sys_version)` FKs), the CONTRIBUTIONs/audit rows they orphan
    /// (guarded — a row shared with a survivor is kept), and any `vo_archive`
    /// markers. `audit` has no FK from `vo_version` (NO ACTION), so those rows are
    /// swept explicitly, as in the EHR delete.
    pub(super) async fn party_physical_delete(&self, party_id: Uuid) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;

        // The target must be a demographic PARTY (ehr-less; any version exists).
        let kind: Option<String> =
            sqlx::query_scalar("SELECT kind FROM vo_version WHERE vo_id = $1 AND ehr_id IS NULL")
                .bind(party_id)
                .fetch_optional(&mut *tx)
                .await?;
        if !kind.as_deref().is_some_and(is_party_kind) {
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

    // ── archive (SM `I_ADMIN_ARCHIVE`) ──────────────────────────────────────
    //
    // Marker-only this phase (design fixed-decision SM-4): populate `vo_archive`;
    // serving reads are unchanged (zero wire drift). All-or-nothing — an unknown
    // id aborts the transaction before any marker is written.

    /// `archive_ehrs`: mark every versioned object of each EHR as archived
    /// (idempotent). Any unknown EHR → `ehr_id_does_not_exist`
    /// ([`ServiceError::NotFound`], → `404`) and nothing is archived.
    pub(super) async fn archive_ehr_vos(&self, ehr_ids: &[Uuid]) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        for &ehr_id in ehr_ids {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
                .bind(ehr_id)
                .fetch_one(&mut *tx)
                .await?;
            if !exists {
                return Err(ServiceError::NotFound(format!("EHR {ehr_id}")));
            }
        }
        for &ehr_id in ehr_ids {
            sqlx::query(
                "INSERT INTO vo_archive (vo_id, reason) \
                 SELECT DISTINCT vo_id, 'archive_ehrs' FROM vo_version WHERE ehr_id = $1 \
                 ON CONFLICT (vo_id) DO NOTHING",
            )
            .bind(ehr_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// `archive_parties`: mark each party's versioned object as archived
    /// (idempotent). Any unknown/non-party id → `party_id_does_not_exist`
    /// ([`ServiceError::NotFound`], → `404`) and nothing is archived.
    ///
    /// PORT NOTE: `i_admin_archive.adoc` says "Parties **and relationships**";
    /// only the party VO is marked this phase (design fixed-decision SM-4).
    /// Archival is a read-neutral marker, so not marking the related
    /// relationships has no observable effect; the storage-tier work (P20) would
    /// extend the marker set.
    pub(super) async fn archive_party_vos(&self, party_ids: &[Uuid]) -> Result<(), ServiceError> {
        let mut tx = self.pool.begin().await?;
        for &party_id in party_ids {
            let kind: Option<String> = sqlx::query_scalar(
                "SELECT kind FROM vo_version WHERE vo_id = $1 AND ehr_id IS NULL",
            )
            .bind(party_id)
            .fetch_optional(&mut *tx)
            .await?;
            if !kind.as_deref().is_some_and(is_party_kind) {
                return Err(ServiceError::NotFound(format!("party {party_id}")));
            }
        }
        for &party_id in party_ids {
            sqlx::query(
                "INSERT INTO vo_archive (vo_id, reason) VALUES ($1, 'archive_parties') \
                 ON CONFLICT (vo_id) DO NOTHING",
            )
            .bind(party_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
