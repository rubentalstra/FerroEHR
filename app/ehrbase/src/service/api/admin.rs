//! [`AdminService`] + [`AdminArchive`] on [`EhrbaseService`] — physical
//! deletion (SM `I_ADMIN_SERVICE`) + archive markers (SM `I_ADMIN_ARCHIVE`).
//!
//! Thin trait adapters: parse the id(s)/time bounds and delegate to the
//! physical-delete, statistics, and archive machinery in
//! [`crate::service::admin`]. The config gate (whether the admin surface is
//! reachable at all) lives at the REST edge (`dispatch::admin`).

use async_trait::async_trait;
use uuid::Uuid;

use ehrbase_rest::{AdminArchive, AdminService, PlatformService, StatTimeRange};
use ehrbase_sm::SmError;

use crate::service::EhrbaseService;

#[async_trait]
impl AdminService for EhrbaseService {
    async fn admin_ehr_delete(&self, ehr_id: String) -> Result<(), SmError> {
        Ok(self
            .physical_ehr_delete(parse_uuid(&ehr_id, "EHR")?)
            .await?)
    }

    async fn admin_ehr_delete_all(&self, ehr_ids: Vec<String>) -> Result<u64, SmError> {
        // Any malformed id in the list → 400 (the whole bulk request is
        // rejected before any deletion runs).
        let ids = ehr_ids
            .iter()
            .map(|s| parse_uuid(s, "EHR"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.physical_ehr_delete_all(&ids).await?)
    }

    async fn admin_list_contributions(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<Vec<String>, SmError> {
        let (lo, hi) = parse_range(time_range)?;
        Ok(self.stat_list_contributions(a_service, lo, hi).await?)
    }

    async fn admin_contribution_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = parse_range(time_range)?;
        Ok(self.stat_contribution_count(a_service, lo, hi).await?)
    }

    async fn versioned_composition_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = parse_range(time_range)?;
        Ok(self
            .stat_versioned_composition_count(a_service, lo, hi)
            .await?)
    }

    async fn composition_version_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = parse_range(time_range)?;
        Ok(self
            .stat_composition_version_count(a_service, lo, hi)
            .await?)
    }

    async fn physical_party_delete(&self, a_party_id: String) -> Result<(), SmError> {
        Ok(self
            .party_physical_delete(parse_uuid(&a_party_id, "party")?)
            .await?)
    }
}

#[async_trait]
impl AdminArchive for EhrbaseService {
    async fn archive_ehrs(&self, ehr_ids: Vec<String>) -> Result<(), SmError> {
        let ids = ehr_ids
            .iter()
            .map(|s| parse_uuid(s, "EHR"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.archive_ehr_vos(&ids).await?)
    }

    async fn archive_parties(&self, party_ids: Vec<String>) -> Result<(), SmError> {
        let ids = party_ids
            .iter()
            .map(|s| parse_uuid(s, "party"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.archive_party_vos(&ids).await?)
    }
}

/// Parse a `UUID` id, mapping a malformed value to `400`. `label` names the
/// resource for the error text (`EHR` / `party`).
fn parse_uuid(raw: &str, label: &str) -> Result<Uuid, SmError> {
    Uuid::parse_str(raw).map_err(|_| SmError::precondition(format!("invalid {label} id: {raw}")))
}

/// Parse the optional `(lower, upper)` ISO 8601 date-time bounds of a statistics
/// call into validated `::timestamptz` bind strings; each bound is independently
/// optional (open bounds → `None`). An invalid ISO bound → `400` (SM
/// `Interval<Iso8601_date_time>`; the invalid-date failure is the adapter's).
fn parse_range(range: StatTimeRange) -> Result<(Option<String>, Option<String>), SmError> {
    let Some((lo, hi)) = range else {
        return Ok((None, None));
    };
    Ok((parse_bound(lo)?, parse_bound(hi)?))
}

/// Validate one optional ISO 8601 date-time bound, returning its canonical
/// string form for binding (or `None` for an open bound). Invalid → `400`.
fn parse_bound(bound: Option<String>) -> Result<Option<String>, SmError> {
    match bound {
        None => Ok(None),
        Some(raw) => {
            let ts: jiff::Timestamp = raw
                .parse()
                .map_err(|_| SmError::precondition(format!("invalid ISO 8601 date-time: {raw}")))?;
            Ok(Some(ts.to_string()))
        }
    }
}
