//! The EHR service (`service/ehr/`) — the openEHR **EHR component** of the
//! platform crate, implementing the six SM `I_EHR_*` interfaces.
//!
//! Layout mirrors the SM interface set one-file-per-interface (arch-overview
//! `master06-design_of_the_ehr.adoc` × the SM EHR component,
//! `docs/specs/openehr/SM/docs/openehr_platform/master05-ehr_service.adoc`):
//!
//! - [`service`]   — `I_EHR_SERVICE` (`i_ehr_service.adoc`): EHR create/get,
//!   `EHR_SUMMARY`, subject lookup, folder-hierarchy reads.
//! - [`status`]    — `I_EHR_STATUS` (`i_ehr_status.adoc`): the `EHR_STATUS`
//!   reads + the five discrete mutators + the `is_modifiable` write guard.
//! - [`directory`] — `I_EHR_DIRECTORY` (`i_ehr_directory.adoc`): the DIRECTORY
//!   FOLDER surface.
//! - [`composition`] — `I_EHR_COMPOSITION` (`i_ehr_composition.adoc`).
//! - [`contributions`] — `I_EHR_CONTRIBUTION` (`i_ehr_contribution.adoc`).
//! - [`access`]    — the `EHR_ACCESS` top-level structure (arch-overview
//!   master06 §`EHR_ACCESS`) + the spec-silent scheme cache.
//! - [`tags`]      — `ITEM_TAG` (ITS-REST experimental extension).
//! - [`uri`]       — `ehr:`-URI resolution (spec-silent extension).
//! - [`meta`]      — the shared version-metadata helpers (G-9).
//!
//! Every file carries its domain logic **and** its `impl <Interface>Service for
//! EhrbaseService`. The versioned-object mechanics are delegated to
//! [`crate::versioning`] (change control, RM common master06) and
//! [`crate::storage`] (row I/O — no openEHR spec governs the SQL).
//!
//! Method-name clashes between an SM trait method and an inherent internal
//! method of the same name (`create_ehr`, `create_composition`,
//! `update_composition`, `delete_composition`, `create_directory`,
//! `update_directory`, `has_directory_version`, …) resolve to the **inherent**
//! method by Rust's method-resolution priority; `self.<name>(…)` inside a trait
//! impl therefore calls the internal implementation, never recurses.
//!
//! # Integration seams
//!
//! [`crate::versioning::CommitEnv`] (the hooks the CONTRIBUTION commit engine
//! needs) is implemented for `EhrbaseService` in `service/mod.rs`; its EHR-owned
//! constituents are authored in this chapter: `default_committer` =
//! [`meta::committer`], `ensure_ehr_exists` / `ensure_content_writable` /
//! `current_vo` / `invalidate_ehr_access` are `EhrbaseService` methods here, and
//! the two in-transaction hooks delegate to
//! [`check_versioned_composition_invariants`] (COMPOSITION modify) and
//! [`EhrbaseService::sync_ehr_subject`] (`EHR_STATUS` commit) — the same fns the
//! direct create/update paths run inline. SQL row I/O is a storage seam
//! ([`crate::storage::ehr_repo`] / [`crate::storage::version_repo`]; no openEHR
//! spec governs the schema — our own design).

mod access;
mod composition;
mod composition_validate;
mod contributions;
mod directory;
mod meta;
mod service;
mod status;
mod status_validate;
mod tags;
mod uri;

pub mod access_types;
pub mod handle;

// The EHR-component surface other service modules and adapters consume.
pub(in crate::service) use access::{EhrAccessCache, default_ehr_access, validate_ehr_access};
pub(in crate::service) use composition_validate::check_versioned_composition_invariants;
pub(in crate::service) use directory::validate_folder;
pub(in crate::service) use meta::committer;
#[cfg(test)]
pub(in crate::service) use service::default_ehr_status;
pub(in crate::service) use status_validate::validate_ehr_status;

use crate::service::ehr_index::types::SubjectRef;
use crate::service::response::ResourceMeta;
use crate::service::status::SmError;
use serde_json::{Value, json};

use crate::versioning::TimeRange;

/// Extract the version-uid `String` a write produced from the internal
/// [`ServiceResponse`](crate::service::response::ServiceResponse)'s resource metadata — the
/// value the SM `create_*`/`update_*`/`delete_*` calls return.
fn version_uid(resp: crate::service::response::ServiceResponse) -> Result<String, SmError> {
    resp.meta
        .map(|m| m.uid)
        .ok_or_else(|| SmError::exception("write produced no version metadata"))
}

/// Enforce the full-`OBJECT_VERSION_ID` `If-Match` precondition (F-01-09 /
/// F-02-08): the client's `preceding_version_uid` MUST equal the resource's
/// current latest `version_uid` **in full** (`object_id` + creating-system id +
/// version), not merely the trunk number (ITS-REST `parameters/If-Match`). A
/// mismatch is a `412`. A `None` `latest` defers first-version/not-found
/// semantics to the versioning path the caller then invokes.
fn ensure_if_match(
    preceding: Option<&openehr_base::prelude::ObjectVersionId>,
    latest: Option<&ResourceMeta>,
) -> Result<(), SmError> {
    let Some(pre) = preceding else {
        return Ok(());
    };
    match latest {
        Some(meta) if meta.uid == pre.value => Ok(()),
        Some(meta) => Err(SmError::version_mismatch(format!(
            "If-Match {:?} does not match the current latest version {:?}",
            pre.value, meta.uid
        ))),
        None => Ok(()),
    }
}

/// Parse an ISO-8601 `Iso8601_date_time` argument for a time-travel read; a
/// malformed value is an argument-validity precondition failure (→ `400`).
fn parse_at_time(raw: &str) -> Result<jiff::Timestamp, SmError> {
    raw.parse::<jiff::Timestamp>()
        .map_err(|_| SmError::precondition(format!("invalid version_at_time: {raw}")))
}

/// Parse the optional SM `Interval<Iso8601_date_time>` bounds of a contribution
/// `time_range` into the internal [`crate::versioning::TimeRange`]; a malformed
/// bound is a `400`-equivalent precondition failure.
fn parse_time_range(raw: crate::service::ehr::handle::TimeRange) -> Result<TimeRange, SmError> {
    let parse = |b: Option<String>| -> Result<Option<jiff::Timestamp>, SmError> {
        b.map(|s| {
            s.parse::<jiff::Timestamp>()
                .map_err(|_| SmError::precondition(format!("invalid time_range bound: {s}")))
        })
        .transpose()
    };
    raw.map(|(lo, hi)| Ok((parse(lo)?, parse(hi)?))).transpose()
}

/// Build the `EHR_STATUS` for a subject-scoped EHR creation: the base status
/// with its `subject` set to a `PARTY_SELF` whose `external_ref` names the
/// subject (the promoted `ehr.subject_*` columns are kept in sync on commit by
/// [`EhrbaseService::sync_ehr_subject`](crate::service::EhrbaseService)).
fn status_for_subject(base: Value, subject: &SubjectRef) -> Value {
    let mut status = base;
    if let Value::Object(map) = &mut status {
        map.insert(
            "subject".to_owned(),
            json!({
                "_type": "PARTY_SELF",
                "external_ref": {
                    "_type": "PARTY_REF",
                    "namespace": subject.namespace,
                    "type": subject.r#type,
                    "id": {
                        "_type": "GENERIC_ID",
                        "value": subject.id,
                        "scheme": subject.namespace
                    }
                }
            }),
        );
    }
    status
}
