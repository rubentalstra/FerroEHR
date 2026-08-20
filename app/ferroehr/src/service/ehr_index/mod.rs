// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The EHR Index service (`service/ehr_index/`) — SM `I_EHR_INDEX`.
//!
//! Holds N:M subject↔EHR associations with duplicate-management metadata
//! (`docs/specs/openehr/SM/docs/openehr_platform/master07-ehr_index_service.adoc`
//! and the UML class `i_ehr_index.adoc`).
//!
//! Layout: `index` = the SM write operations (I1–I5) + the design-filled
//! reads, each public method parsing its `ehr_id` at the boundary;
//! `conflicts` = the design-filled advisory duplicate-detection read;
//! [`types`] = the SM information structures (`RESOURCE_STATUS`,
//! `RESOURCE_INSTANCE_TYPE`, `LOCATION_DESC`, the `OBJECT_REF` subject key).
//!
//! NOTE: index entries are **not** versioned objects — the SM defines no
//! versioning for the index — so these are plain SQL writes over the
//! `ehr_index` table, emitting no CONTRIBUTION/version. No openEHR spec governs
//! the storage mechanism (our own design); master07 governs the operation
//! semantics + error names.
//!
//! The index and the `ehr.subject_id` promotion (the Primary-instance fast path
//! for `ehr_get_by_subject`) are intentionally decoupled: an EHR created through
//! the normal API is not auto-indexed here, and the index models the full N:M
//! state. The `ehr_index` + `ehr`-existence SQL is this domain's own direct-SQL
//! design, so the table access lives here rather than behind a storage-owned
//! repository.
//!
//! No wire is mounted (EHR Index has no ITS-REST contract — native-API-only,
//! our own extension surface); the public methods exist for the SM native API
//! and future extension routes.

pub(crate) mod conflicts;
pub(crate) mod index;
pub mod types;

use sqlx::Row;

use crate::ids::EhrId;
use crate::service::ehr_index::types::{
    EhrIndexEntry, LocationDesc, ResourceInstanceType, ResourceStatus, SubjectRef,
};
use crate::service::error::ServiceError;
use crate::service::status::{CallStatusType, SmError};

/// The precise EHR-index failure kind.
///
/// `master07 §Errors` declares distinct `ehr_id_does_not_exist` and
/// `subject_id_does_not_exist` statuses, which must NOT collapse to the generic
/// `versioned_object_does_not_exist` (`i_ehr_index.adoc §Errors`).
/// [`From<IndexError> for SmError`] maps each variant onto its dedicated
/// [`CallStatusType`]; a generic [`ServiceError`] (a DB/codec fault) rides
/// through unchanged.
// Nominal `pub`: `FerroEhrService::index_conflicts` (a `pub` method — the
// design-filled detection read has no SM trait binding) carries this type in
// its public signature.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// `ehr_id_does_not_exist` — the addressed EHR is unknown.
    #[error("EHR {0} does not exist (ehr_id_does_not_exist)")]
    EhrDoesNotExist(EhrId),
    /// `subject_id_does_not_exist` — no such subject / association.
    #[error("subject {}@{} is not associated (subject_id_does_not_exist)", .0.id, .0.namespace)]
    SubjectDoesNotExist(SubjectRef),
    /// A generic service/DB fault (mapped by the shared table).
    #[error(transparent)]
    Service(#[from] ServiceError),
}

impl From<sqlx::Error> for IndexError {
    fn from(e: sqlx::Error) -> Self {
        IndexError::Service(ServiceError::from(e))
    }
}

impl From<IndexError> for SmError {
    /// Map the precise EHR-index errors onto their dedicated SM statuses
    /// (`master07 §Errors`), bypassing the generic
    /// `NotFound → versioned_object_does_not_exist` collapse the shared
    /// [`From<ServiceError> for SmError`] applies.
    fn from(e: IndexError) -> Self {
        match e {
            IndexError::EhrDoesNotExist(id) => SmError::new(
                CallStatusType::EhrIdDoesNotExist,
                format!("EHR {id} does not exist"),
            ),
            IndexError::SubjectDoesNotExist(subject) => SmError::new(
                CallStatusType::SubjectIdDoesNotExist,
                format!(
                    "subject {}@{} is not associated",
                    subject.id, subject.namespace
                ),
            ),
            IndexError::Service(e) => e.into(),
        }
    }
}

/// Parse an ISO-8601 date-time string into a Postgres `timestamptz` binding, or
/// `None`. An unparseable value is a `400`.
///
/// NOTE: `RESOURCE_STATUS.start_valid_time`/`end_valid_time` are typed
/// `@@` (an unresolved placeholder) in the SM — a recorded spec defect
/// (`resource_status.adoc:20,24`); implemented as ISO date-time strings.
#[expect(
    clippy::map_err_ignore,
    reason = "the mapped error already echoes the rejected token; the discarded \
              parse error adds only its own wording, which is not part of the \
              wire contract"
)]
fn parse_valid_time(raw: Option<&str>) -> Result<Option<jiff_sqlx::Timestamp>, ServiceError> {
    use jiff_sqlx::ToSqlx;
    match raw {
        None => Ok(None),
        Some(s) => s
            .parse::<jiff::Timestamp>()
            .map(|t| Some(t.to_sqlx()))
            .map_err(|_| ServiceError::precondition(format!("invalid valid_time: {s}"))),
    }
}

/// Wrap a [`LocationDesc`] for its typed `jsonb` binding, or SQL NULL.
///
/// NOTE: `LOCATION_DESC` is an attribute-less stub in the SM
/// (`location_desc.adoc`) — a recorded spec defect; the designed contract
/// `{system_id, uri?, description?}` is our own design.
fn location_binding(loc: Option<&LocationDesc>) -> Option<sqlx::types::Json<&LocationDesc>> {
    loc.map(sqlx::types::Json)
}

/// Map a zero-rows-affected write to [`IndexError::SubjectDoesNotExist`]
/// (`subject_id_does_not_exist`).
fn require_association(affected: u64, subject: &SubjectRef) -> Result<(), IndexError> {
    if affected == 0 {
        return Err(IndexError::SubjectDoesNotExist(subject.clone()));
    }
    Ok(())
}

/// Reassemble one [`EhrIndexEntry`] from an `ehr_index` row.
fn row_to_entry(row: &sqlx::postgres::PgRow) -> Result<EhrIndexEntry, sqlx::Error> {
    let ehr_id: EhrId = row.try_get("ehr_id")?;
    let subject = SubjectRef {
        id: row.try_get("subject_id")?,
        namespace: row.try_get("subject_namespace")?,
        r#type: row.try_get("subject_type")?,
    };
    let start: Option<jiff_sqlx::Timestamp> = row.try_get("start_valid_time")?;
    let end: Option<jiff_sqlx::Timestamp> = row.try_get("end_valid_time")?;
    let status = ResourceStatus {
        instance_type: ResourceInstanceType::from_str_or_primary(
            &row.try_get::<String, _>("instance_type")?,
        ),
        start_valid_time: start.map(|t| t.to_jiff().to_string()),
        end_valid_time: end.map(|t| t.to_jiff().to_string()),
        notes: row.try_get("notes")?,
    };
    // A stored row that no longer decodes as the designed contract is a
    // server fault: surface it (`?` → the DB error path), never blank fields.
    let location = row
        .try_get::<Option<sqlx::types::Json<LocationDesc>>, _>("location")?
        .map(|j| j.0);
    Ok(EhrIndexEntry {
        ehr_id: ehr_id.to_string(),
        subject,
        status,
        location,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two declared EHR-index errors map to their dedicated SM statuses
    /// (`master07 §Errors`), never the generic
    /// `versioned_object_does_not_exist`.
    #[test]
    fn index_errors_map_to_dedicated_statuses() {
        let ehr = EhrId::new();
        let ehr_sm: SmError = IndexError::EhrDoesNotExist(ehr).into();
        assert_eq!(ehr_sm.status, CallStatusType::EhrIdDoesNotExist);

        let subject = SubjectRef::person("p1", "demographic");
        let subj_sm: SmError = IndexError::SubjectDoesNotExist(subject).into();
        assert_eq!(subj_sm.status, CallStatusType::SubjectIdDoesNotExist);

        // A generic service miss still routes through the shared table (404),
        // carrying the status it was constructed with.
        let svc: SmError = IndexError::Service(ServiceError::sm(
            CallStatusType::VersionedObjectDoesNotExist,
            "x",
        ))
        .into();
        assert_eq!(svc.status, CallStatusType::VersionedObjectDoesNotExist);
    }

    #[test]
    fn valid_time_parsing() {
        assert!(parse_valid_time(None).unwrap().is_none());
        assert!(
            parse_valid_time(Some("2021-01-01T00:00:00Z"))
                .unwrap()
                .is_some()
        );
        assert!(parse_valid_time(Some("not-a-time")).is_err());
    }
}
