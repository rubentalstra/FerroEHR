//! The Admin service (`service/admin/`) — the openEHR **Admin component** of the
//! platform crate: SM `I_ADMIN_SERVICE` / `I_ADMIN_ARCHIVE` / `I_ADMIN_DUMP_LOAD`
//! (`docs/specs/openehr/SM/docs/openehr_platform/master15-admin_service.adoc`
//! and the UML classes `i_admin_service.adoc`, `i_admin_archive.adoc`,
//! `i_admin_dump_load.adoc`; `master02-overview.adoc` frames Admin as
//! "administrative facilities … such as back-up"). Design register:
//! `docs/design/platform/06-service-message-admin.md` §5.2.
//!
//! Layout mirrors the three SM admin interfaces:
//!
//! - [`delete`] — `I_ADMIN_SERVICE.physical_ehr_delete` / `physical_party_delete`
//!   (+ the `admin_ehr_delete_all` extension, G-A1): cascade + orphan-audit sweep.
//! - [`statistics`] — `I_ADMIN_SERVICE.list_contributions` / `contribution_count`
//!   / `versioned_composition_count` / `composition_version_count`.
//! - [`archive`] — `I_ADMIN_ARCHIVE.archive_ehrs` / `archive_parties`.
//! - [`dump_load`] — `I_ADMIN_DUMP_LOAD.export_ehrs` / `load_ehrs`.
//!
//! This `mod.rs` holds the three thin trait adapters — parse ids/time bounds,
//! delegate to the machinery — collapsing what used to be split between
//! `api/admin.rs` and `dump_load.rs` so all three admin traits impl in one
//! place. The `PLATFORM_SERVICE` statistics scope invalidity (`platform_service.adoc`)
//! is time-bound with a closed `[lo, hi]` interval:
//!
//! PORT NOTE (already-correct — `i_admin_service.adoc` types the range as
//! `Interval<Iso8601_date_time>` with no inclusivity stated; G-A7): a closed
//! `[lo, hi]` is SM-silent, so this is a documented realization of our own.
//!
//! # Cross-module wiring
//!
//! - **`crate::storage`** — [`dump_load`] reassembles/decomposes version bodies
//!   through the storage codec (`node_repo::read_version_canonical` /
//!   `decompose` + `write_nodes`).
//! - **[`archive`]** marks EHR/party versioned objects archived; the physical
//!   cold-tier storage movement is a spec-silent PERF(port) item (see the PORT
//!   NOTE there).

mod archive;
mod delete;
mod dump_load;
mod statistics;

pub mod types;

use std::path::Path;

use uuid::Uuid;

use crate::service::admin::types::StatTimeRange;
use crate::service::admin::types::{DumpLoadFailReport, ExportSpec};
use crate::service::platform_service::PlatformService;
use crate::service::status::SmError;

use crate::service::EhrbaseService;

/// Whether a `vo_version.kind` string names a demographic PARTY root (the five
/// concrete `ACTOR`/`PARTY` leaves — RM demographic) — as opposed to a
/// `PARTY_RELATIONSHIP` or a clinical versioned object. Shared by the physical
/// delete ([`delete`]) and archive ([`archive`]) party guards.
fn is_party_kind(kind: &str) -> bool {
    matches!(kind, "AGENT" | "GROUP" | "ORGANISATION" | "PERSON" | "ROLE")
}

impl EhrbaseService {
    pub async fn admin_ehr_delete(&self, ehr_id: String) -> Result<(), SmError> {
        Ok(self
            .physical_ehr_delete(parse_uuid(&ehr_id, "EHR")?)
            .await?)
    }

    pub async fn admin_ehr_delete_all(&self, ehr_ids: Vec<String>) -> Result<u64, SmError> {
        // Any malformed id in the list → 400 (the whole bulk request is rejected
        // before any deletion runs).
        let ids = ehr_ids
            .iter()
            .map(|s| parse_uuid(s, "EHR"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.physical_ehr_delete_all(&ids).await?)
    }

    pub async fn admin_list_contributions(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<Vec<String>, SmError> {
        let (lo, hi) = parse_range(time_range)?;
        Ok(self.stat_list_contributions(a_service, lo, hi).await?)
    }

    pub async fn admin_contribution_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = parse_range(time_range)?;
        Ok(self.stat_contribution_count(a_service, lo, hi).await?)
    }

    pub async fn versioned_composition_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = parse_range(time_range)?;
        Ok(self
            .stat_versioned_composition_count(a_service, lo, hi)
            .await?)
    }

    pub async fn composition_version_count(
        &self,
        a_service: PlatformService,
        time_range: StatTimeRange,
    ) -> Result<i64, SmError> {
        let (lo, hi) = parse_range(time_range)?;
        Ok(self
            .stat_composition_version_count(a_service, lo, hi)
            .await?)
    }

    pub async fn physical_party_delete(&self, a_party_id: String) -> Result<(), SmError> {
        Ok(self
            .party_physical_delete(parse_uuid(&a_party_id, "party")?)
            .await?)
    }
}

impl EhrbaseService {
    pub async fn archive_ehrs(&self, ehr_ids: Vec<String>) -> Result<(), SmError> {
        let ids = ehr_ids
            .iter()
            .map(|s| parse_uuid(s, "EHR"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.archive_ehr_vos(&ids).await?)
    }

    pub async fn archive_parties(&self, party_ids: Vec<String>) -> Result<(), SmError> {
        let ids = party_ids
            .iter()
            .map(|s| parse_uuid(s, "party"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.archive_party_vos(&ids).await?)
    }
}

impl EhrbaseService {
    pub async fn export_ehrs(
        &self,
        file_sys_loc: String,
        spec: ExportSpec,
    ) -> Result<Vec<DumpLoadFailReport>, SmError> {
        self.export_ehrs_to(Path::new(&file_sys_loc), &spec).await
    }

    pub async fn load_ehrs(
        &self,
        file_sys_loc: String,
    ) -> Result<Vec<DumpLoadFailReport>, SmError> {
        self.load_ehrs_from(Path::new(&file_sys_loc)).await
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
