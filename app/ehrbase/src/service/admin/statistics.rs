//! Admin statistics (SM `I_ADMIN_SERVICE.list_contributions` /
//! `contribution_count` / `versioned_composition_count` /
//! `composition_version_count`).
//!
//! Spec: `docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc` — each
//! call takes a `PLATFORM_SERVICE` (`a_service`, "Name of a versioned content
//! service") and an optional `Interval<Iso8601_date_time>` matched against the
//! CONTRIBUTION / version audit `time_committed`. The enumeration members are
//! `platform_service.adoc`. No openEHR spec governs the SQL that answers these —
//! our own design over the greenfield `contribution` / `vo_version` / `audit`
//! tables (`0001_baseline.sql`).

use crate::service::admin::types::StatTimeRange;
use crate::service::platform_service::PlatformService;
use crate::service::status::SmError;
use crate::service::EhrbaseService;
use crate::service::error::ServiceError;

/// Whether a [`PlatformService`]'s CONTRIBUTIONs are EHR-scoped (`Some(true)` →
/// `ehr_id IS NOT NULL`), ehr-less (`Some(false)` → `ehr_id IS NULL`), or the
/// service is not a versioned-content service (`None` → statistics are trivially
/// empty/0).
///
/// PORT NOTE (already-correct — `platform_service.adoc`): `a_service` is
/// "Name of a versioned content service". Only `Ehr` (EHR-scoped) and
/// `Demographic` (ehr-less) hold contributions in this CDR; the remaining
/// members (`Admin`/`Definitions`/`Ehr_index`/`Message`/`Query`/`System_log`)
/// are not versioned-content services and yield nothing — a defensible reading,
/// not a gap. Returned as a bool so the SQL stays static (parameterized), never
/// string-built.
fn contribution_ehr_scoped(service: PlatformService) -> Option<bool> {
    match service {
        PlatformService::Ehr => Some(true),
        PlatformService::Demographic => Some(false),
        _ => None,
    }
}

impl EhrbaseService {
    /// SM `list_contributions`: the ids of all CONTRIBUTIONs of the named
    /// versioned-content service within the (optional) time range, ordered by
    /// commit time. A non-content service yields the empty list.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — a bound of `time_range` is not a
    ///   valid ISO 8601 date-time.
    /// - `exception` — a database fault while querying.
    pub async fn admin_list_contributions(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<Vec<String>, SmError> {
        let (lo, hi) = super::parse_range(time_range)?;
        let Some(ehr_scoped) = contribution_ehr_scoped(a_service) else {
            return Ok(Vec::new());
        };
        // Static SQL; `$3` selects EHR-scoped vs ehr-less contributions.
        let ids: Vec<String> = sqlx::query_scalar(
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
        .await
        .map_err(ServiceError::from)?;
        Ok(ids)
    }

    /// SM `contribution_count`: the count of all CONTRIBUTIONs of the named
    /// service within the (optional) time range. A non-content service → 0.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — a bound of `time_range` is not a
    ///   valid ISO 8601 date-time.
    /// - `exception` — a database fault while querying.
    pub async fn admin_contribution_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = super::parse_range(time_range)?;
        let Some(ehr_scoped) = contribution_ehr_scoped(a_service) else {
            return Ok(0);
        };
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE (($3 AND c.ehr_id IS NOT NULL) OR (NOT $3 AND c.ehr_id IS NULL)) \
               AND ($1::timestamptz IS NULL OR a.time_committed >= $1::timestamptz) \
               AND ($2::timestamptz IS NULL OR a.time_committed <= $2::timestamptz)",
        )
        .bind(lo)
        .bind(hi)
        .bind(ehr_scoped)
        .fetch_one(&self.pool)
        .await
        .map_err(ServiceError::from)?;
        Ok(count)
    }

    /// SM `versioned_composition_count`: the count of distinct COMPOSITION
    /// versioned objects with a version committed within the (optional) range.
    ///
    /// PORT NOTE (already-correct — `platform_service.adoc`): COMPOSITIONs are
    /// EHR-scoped, so only `a_service = Ehr` yields a non-zero count; every
    /// other member → 0 (COMPOSITIONs are not in its scope).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — a bound of `time_range` is not a
    ///   valid ISO 8601 date-time.
    /// - `exception` — a database fault while querying.
    pub async fn versioned_composition_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = super::parse_range(time_range)?;
        if a_service != PlatformService::Ehr {
            return Ok(0);
        }
        let count: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT v.vo_id) FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.kind = 'COMPOSITION' \
               AND ($1::timestamptz IS NULL OR a.time_committed >= $1::timestamptz) \
               AND ($2::timestamptz IS NULL OR a.time_committed <= $2::timestamptz)",
        )
        .bind(lo)
        .bind(hi)
        .fetch_one(&self.pool)
        .await
        .map_err(ServiceError::from)?;
        Ok(count)
    }

    /// SM `composition_version_count`: the count of individual COMPOSITION
    /// version rows committed within the (optional) range. Scope gate as
    /// [`Self::versioned_composition_count`].
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — a bound of `time_range` is not a
    ///   valid ISO 8601 date-time.
    /// - `exception` — a database fault while querying.
    pub async fn composition_version_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = super::parse_range(time_range)?;
        if a_service != PlatformService::Ehr {
            return Ok(0);
        }
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.kind = 'COMPOSITION' \
               AND ($1::timestamptz IS NULL OR a.time_committed >= $1::timestamptz) \
               AND ($2::timestamptz IS NULL OR a.time_committed <= $2::timestamptz)",
        )
        .bind(lo)
        .bind(hi)
        .fetch_one(&self.pool)
        .await
        .map_err(ServiceError::from)?;
        Ok(count)
    }
}
