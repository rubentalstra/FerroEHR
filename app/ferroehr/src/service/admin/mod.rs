// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The Admin service (`service/admin/`) — the openEHR **Admin component**.
//!
//! Realizes SM `I_ADMIN_SERVICE` / `I_ADMIN_ARCHIVE` / `I_ADMIN_DUMP_LOAD`
//! (`docs/specs/openehr/SM/docs/openehr_platform/master15-admin_service.adoc`
//! and the UML classes `i_admin_service.adoc`, `i_admin_archive.adoc`,
//! `i_admin_dump_load.adoc`; `master02-overview.adoc` frames Admin as
//! "administrative facilities … such as back-up").
//!
//! One file per SM admin interface concern; each file carries both the public
//! `FerroEhrService` methods (parse ids/time bounds at the boundary) and the
//! machinery behind them:
//!
//! - `delete` — `I_ADMIN_SERVICE.physical_ehr_delete` / `physical_party_delete`
//!   (+ the `admin_ehr_delete_all` extension): cascade + orphan-audit sweep.
//! - `statistics` — `I_ADMIN_SERVICE.list_contributions` / `contribution_count`
//!   / `versioned_composition_count` / `composition_version_count`.
//! - `archive` — `I_ADMIN_ARCHIVE.archive_ehrs` / `archive_parties`.
//! - `dump_load` — `I_ADMIN_DUMP_LOAD.export_ehrs` / `load_ehrs`.
//! - [`types`] — the SM information classes of the ADMIN group
//!   (`EXPORT_SPEC`, `DUMP_LOAD_FAIL_REPORT`, the format enumerations).
//!
//! # Cross-module wiring
//!
//! - **`crate::storage`** — `dump_load` reassembles/decomposes version bodies
//!   through the storage codec (`node_repo::read_version_canonical` /
//!   `decompose` + `write_nodes`).
//! - **`archive`** marks EHR/party versioned objects archived and physically
//!   moves their rows into the spec-silent cold tier
//!   ([`crate::storage::version_repo::tier`]), reversibly; `delete` and
//!   `dump_load` reach both tiers because a physical delete and a repository
//!   export are whole-repository operations by definition.

mod archive;
mod delete;
mod dump_load;
mod statistics;

pub mod types;

use uuid::Uuid;

use crate::service::status::SmError;

/// Whether a `vo_version.kind` string names a demographic PARTY root (the five
/// concrete `ACTOR`/`PARTY` leaves — RM demographic) — as opposed to a
/// `PARTY_RELATIONSHIP` or a clinical versioned object. Shared by the physical
/// delete ([`delete`]) and archive ([`archive`]) party guards.
pub(super) fn is_party_kind(kind: &str) -> bool {
    matches!(kind, "AGENT" | "GROUP" | "ORGANISATION" | "PERSON" | "ROLE")
}

/// Parse a `UUID` id, mapping a malformed value to a `precondition_violation`
/// (`400`). `label` names the resource for the error text (`EHR` / `party`).
#[expect(
    clippy::map_err_ignore,
    reason = "the mapped error already names the resource and echoes the \
              rejected token; the discarded `uuid::Error` adds only its own \
              wording, which is not part of the wire contract"
)]
fn parse_uuid(raw: &str, label: &str) -> Result<Uuid, SmError> {
    Uuid::parse_str(raw).map_err(|_| SmError::precondition(format!("invalid {label} id: {raw}")))
}

/// Parse a whole id list, rejecting the entire request on the first malformed
/// id (a bulk call is validated before any work runs).
fn parse_uuid_list(raw: &[String], label: &str) -> Result<Vec<Uuid>, SmError> {
    raw.iter().map(|s| parse_uuid(s, label)).collect()
}

/// Parse the optional `(lower, upper)` ISO 8601 date-time bounds of a
/// statistics call into validated `::timestamptz` bind strings; each bound is
/// independently optional (open bounds → `None`). An invalid ISO bound → `400`
/// (SM `Interval<Iso8601_date_time>`; the invalid-date failure is the
/// boundary's).
///
/// NOTE (`i_admin_service.adoc` types the range as
/// `Interval<Iso8601_date_time>` with no inclusivity stated): the interval is
/// treated as **closed** `[lo, hi]` — the default openEHR `Interval` bound
/// inclusivity — an SM-silent, documented realization of our own.
///
/// NOTE (BASE `org.openehr.base.foundation_types.interval.adoc` §Invariants,
/// `Limits_consistent`: `(not upper_unbounded and not lower_unbounded) implies
/// lower <= upper`): a bounded pair whose lower bound is AFTER its upper bound
/// is not an `Interval` at all, so the parameter value violates its own type
/// and the call is refused (`precondition_violation`) rather than silently
/// answered with the empty result an inverted range would select.
fn parse_range(range: types::StatTimeRange) -> Result<(Option<String>, Option<String>), SmError> {
    let Some((lo, hi)) = range else {
        return Ok((None, None));
    };
    let lower = parse_bound(lo)?;
    let upper = parse_bound(hi)?;
    if let (Some(lower), Some(upper)) = (lower, upper)
        && lower > upper
    {
        return Err(SmError::precondition(format!(
            "time_interval lower bound {lower} is after its upper bound {upper} — an \
             Interval requires lower <= upper (BASE Interval invariant Limits_consistent)"
        )));
    }
    Ok((
        lower.map(|ts| ts.to_string()),
        upper.map(|ts| ts.to_string()),
    ))
}

/// Validate one optional ISO 8601 date-time bound (or `None` for an open
/// bound). Invalid → `400`.
#[expect(
    clippy::map_err_ignore,
    reason = "the mapped error already echoes the rejected token; the discarded \
              parse error adds only its own wording, which is not part of the \
              wire contract"
)]
fn parse_bound(bound: Option<String>) -> Result<Option<jiff::Timestamp>, SmError> {
    match bound {
        None => Ok(None),
        Some(raw) => raw
            .parse::<jiff::Timestamp>()
            .map(Some)
            .map_err(|_| SmError::precondition(format!("invalid ISO 8601 date-time: {raw}"))),
    }
}
