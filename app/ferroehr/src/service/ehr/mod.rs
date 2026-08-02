//! The EHR service (`service/ehr/`) — the openEHR **EHR component** of the
//! platform crate, implementing the SM `I_EHR_*` interfaces as concrete
//! `FerroEhrService` methods.
//!
//! Layout mirrors the SM interface set one-file-per-interface (arch-overview
//! `master06-design_of_the_ehr.adoc` × the SM EHR component,
//! `docs/specs/openehr/SM/docs/openehr_platform/master05-ehr_service.adoc`):
//!
//! - `service`      — `I_EHR_SERVICE` (`i_ehr_service.adoc`): EHR create/get,
//!   `EHR_SUMMARY`, subject lookup, folder-hierarchy reads.
//! - `status`       — `I_EHR_STATUS` (`i_ehr_status.adoc`): the `EHR_STATUS`
//!   reads + the five discrete mutators + the `is_modifiable` write guard.
//! - `directory`    — `I_EHR_DIRECTORY` (`i_ehr_directory.adoc`): the
//!   DIRECTORY FOLDER surface.
//! - `composition`  — `I_EHR_COMPOSITION` (`i_ehr_composition.adoc`).
//! - `contributions` — `I_EHR_CONTRIBUTION` (`i_ehr_contribution.adoc`).
//! - `access`       — the `EHR_ACCESS` top-level structure (arch-overview
//!   master06 §`EHR_ACCESS`) + the spec-silent scheme cache.
//! - `validation`   — the commit-validation choke point for every EHR-owned
//!   kind (`EHR_STATUS` / `EHR_ACCESS` / FOLDER / COMPOSITION) + the
//!   `VERSIONED_COMPOSITION` cross-version invariants.
//! - `meta` — the shared version-metadata helpers.
//! - `tags`         — `ITEM_TAG` (ITS-REST experimental extension).
//! - `uri`          — `ehr:`-URI resolution (spec-silent extension).
//!
//! The versioned-object mechanics are delegated to [`crate::versioning`]
//! (change control, RM common master06) and [`crate::storage`] (row I/O — no
//! openEHR spec governs the SQL).
//!
//! # Integration seams
//!
//! `crate::versioning::CommitEnv` (the hooks the CONTRIBUTION commit engine
//! needs) is implemented for `FerroEhrService` in `service/mod.rs`; its
//! EHR-owned constituents are authored in this chapter: `default_committer` =
//! `meta::committer`, `ensure_ehr_exists` / `ensure_content_writable` /
//! `current_vo` / `invalidate_ehr_access` are `FerroEhrService` methods here,
//! and the two in-transaction hooks delegate to
//! `check_versioned_composition_invariants` (COMPOSITION modify) and
//! `FerroEhrService::sync_ehr_subject` (`EHR_STATUS` commit) — the same fns the
//! direct create/update paths run inline. SQL row I/O is a storage seam
//! ([`crate::storage::ehr_repo`] /
//! [`crate::storage::version_repo`]; no openEHR spec governs the schema — our
//! own design).

pub(in crate::service) mod access;
pub(in crate::service) mod category;
mod composition;
mod contributions;
mod directory;
pub(in crate::service) mod meta;
pub(in crate::service) mod service;
mod status;
pub(in crate::service) mod tags;
mod uri;
pub(crate) mod validation;

pub mod access_types;
pub mod handle;

use openehr_base::base_types::identification::lexical::composite_ids_equal;
use openehr_base::prelude::{GenericId, ObjectId, PartyRef};
use openehr_rm::prelude::{PartyProxy, PartySelf};
use serde_json::Value;

use crate::service::ehr_index::types::SubjectRef;
use crate::service::response::ResourceMeta;
use crate::service::status::SmError;
use crate::versioning::contribution::TimeRange;

/// The committed version's full [`ResourceMeta`] (version uid + commit
/// time), for write paths whose wire response carries `ETag` **and**
/// `Last-Modified` (ITS-REST overview §"`ETag` and Last-Modified": both
/// SHOULD accompany versioned resources).
fn committed_meta(
    resp: crate::service::response::ServiceResponse,
) -> Result<ResourceMeta, SmError> {
    resp.meta
        .ok_or_else(|| SmError::exception("write produced no version metadata"))
}

/// Verify a version-addressed READ returned the VERSION the caller named:
/// the addressed `OBJECT_VERSION_ID` must equal the served version's full
/// three-part identity — `object_id :: creating_system_id ::
/// version_tree_id` (ITS-REST overview `Resources.md` §Identifier types:
/// the `version_uid` "uniquely identifies a VERSION") — compared
/// case-insensitively — the shared composite-identifier rule
/// ([`composite_ids_equal`], BASE `base_types` master05 §"Composite Identifiers
/// and Case"). A tree-id-only fetch would satisfy a fabricated
/// `creating_system_id`; that names no VERSION in this repository → 404.
fn ensure_addressed_version(
    addressed: &openehr_base::prelude::ObjectVersionId,
    served_uid: &str,
) -> Result<(), crate::service::error::ServiceError> {
    if composite_ids_equal(served_uid, addressed.value()) {
        Ok(())
    } else {
        Err(crate::service::error::ServiceError::sm(
            crate::service::status::CallStatusType::ObjectVersionDoesNotExist,
            format!("version {}", addressed.value()),
        ))
    }
}

/// Enforce the full-`OBJECT_VERSION_ID` `If-Match` precondition: the client's `preceding_version_uid` MUST equal the resource's
/// current latest `version_uid` **in full** (`object_id` + creating-system id +
/// version), not merely the trunk number (ITS-REST `parameters/If-Match`). A
/// mismatch is a `412`. A `None` `latest` defers first-version/not-found
/// semantics to the versioning path the caller then invokes.
fn ensure_if_match(
    preceding: Option<&openehr_base::prelude::ObjectVersionId>,
    latest: Option<&ResourceMeta>,
) -> Result<(), crate::service::error::ServiceError> {
    let Some(pre) = preceding else {
        return Ok(());
    };
    match latest {
        // Composite identifiers compare case-INsensitively (BASE
        // base_types master05 §"Composite Identifiers and Case": two
        // identifiers identical apart from case identify the same thing).
        Some(meta) if composite_ids_equal(&meta.uid, pre.value()) => Ok(()),
        Some(meta) => Err(crate::service::error::ServiceError::VersionConflict(
            format!(
                "If-Match {:?} does not match the current latest version {:?}",
                pre.value(), meta.uid
            ),
        )),
        None => Ok(()),
    }
}

/// Parse the optional SM `Interval<Iso8601_date_time>` bounds of a contribution
/// `time_range` into the internal [`crate::versioning::contribution::TimeRange`]; a malformed
/// bound is a `400`-equivalent precondition failure. Bound decoding is the
/// shared ITS-REST datetime-parameter decoder
/// (`crate::service::datetime`) — an offset is optional on the wire.
fn parse_time_range(raw: handle::TimeRange) -> Result<TimeRange, SmError> {
    let parse = |b: Option<String>| -> Result<Option<jiff::Timestamp>, SmError> {
        b.as_deref()
            .map(crate::service::datetime::parse_time_range_bound)
            .transpose()
    };
    raw.map(|(lo, hi)| Ok((parse(lo)?, parse(hi)?))).transpose()
}

/// The caller's `UPDATE_VERSION` envelope resolved for a direct commit: the
/// commit audit (caller attributes merged with the server rules — ITS-REST
/// overview §"openehr-version and openehr-audit-details" MUST, including a
/// caller-supplied `change_type` after group + operation validation) and the
/// write envelope (lifecycle / verbatim signature / attestations). Shared by
/// the COMPOSITION, `EHR_STATUS`, and DIRECTORY direct-write paths.
///
/// # Errors
/// [`ServiceError::Unprocessable`] / [`ServiceError::BadRequest`] — the
/// caller's `change_type` is out-of-group / contradicts the operation
/// (`crate::versioning::audit::AuditInput::from_update`).
fn resolve_envelope(
    version: &crate::service::version_update::UpdateVersion,
    operation_change_type: &str,
    default_description: &str,
    system_id: &str,
) -> Result<
    (
        crate::versioning::audit::AuditInput,
        crate::versioning::change::WriteEnvelope,
    ),
    crate::service::error::ServiceError,
> {
    let audit = crate::versioning::audit::AuditInput::from_update(
        &version.audit,
        operation_change_type,
        default_description,
        system_id,
    )?;
    let attestations = version
        .attestations
        .as_ref()
        .map(|a| {
            a.iter()
                .filter_map(|x| serde_json::to_value(x).ok())
                .collect()
        })
        .unwrap_or_default();
    let envelope = crate::versioning::change::WriteEnvelope {
        lifecycle_state: Some(version.lifecycle_state.code_string.clone()),
        signature: version.signature.clone(),
        attestations,
    };
    Ok((audit, envelope))
}

/// Build the `EHR_STATUS` for a subject-scoped EHR creation: the base status
/// with its `subject` set to a `PARTY_SELF` whose `external_ref` names the
/// subject (the promoted `ehr.subject_*` columns are kept in sync on commit by
/// [`FerroEhrService::sync_ehr_subject`](crate::service::FerroEhrService)).
fn status_for_subject(base: Value, subject: &SubjectRef) -> Value {
    let mut status = base;
    if let Value::Object(map) = &mut status {
        map.insert(
            "subject".to_owned(),
            openehr_its::json::to_canonical_value(&PartyProxy::PartySelf(PartySelf {
                external_ref: Some(PartyRef {
                    namespace: subject.namespace.clone(),
                    r#type: subject.r#type.clone(),
                    id: ObjectId::GenericId(GenericId {
                        value: subject.id.clone(),
                        scheme: subject.namespace.clone(),
                    }),
                }),
            })),
        );
    }
    status
}
