//! EHR Index service module (SM `I_EHR_INDEX`,
//! `master07-ehr_index_service.adoc`): N:M subject↔EHR associations with
//! duplicate-management metadata. Register:
//! `docs/design/platform/04-service-demographic-ehr-index.md`.
//!
//! Internal split: [`index`] = the SM write ops (I1–I5) + the design-filled
//! reads; [`conflicts`] = the design-filled advisory duplicate-detection read
//! (G-10); [`api`] = the [`EhrIndexService`](crate::service::EhrIndexService) trait
//! impl.
//!
//! PORT NOTE: index entries are **not** versioned objects — the SM defines no
//! versioning for the index — so these are plain SQL writes over the
//! `ehr_index` table, emitting no CONTRIBUTION/version. No openEHR spec governs
//! the storage mechanism (our own design); master07 governs the operation
//! semantics + error names. This does not touch the `ehr.subject_id` promotion
//! (the Primary-instance fast path for `ehr_get_by_subject`); the index models
//! the full N:M state (G-15: the index and `ehr.subject_id` are intentionally
//! decoupled — an EHR created via the normal API is not auto-indexed here).
//!
//! The `ehr_index` + `ehr`-existence SQL is this domain's own direct-SQL design
//! — no openEHR spec governs the storage mechanism (master07 governs only the
//! operation semantics + error names), so the table access lives here rather
//! than behind a storage-owned repository.

use crate::service::status::{CallStatusType, SmError};
use crate::service::ehr_index::types::{EhrIndexEntry, LocationDesc, ResourceInstanceType, ResourceStatus, SubjectRef};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::service::ServiceError;

pub(crate) mod api;
pub(crate) mod conflicts;
pub(crate) mod index;

/// The precise EHR-index failure kind (G-8/G-9): `master07 §Errors` declares
/// distinct `ehr_id_does_not_exist` and `subject_id_does_not_exist` statuses,
/// which must NOT collapse to the generic `versioned_object_does_not_exist`
/// (`i_ehr_index.adoc §Errors`). The adapter ([`api`]) maps each variant onto
/// its dedicated [`CallStatusType`]; a generic [`ServiceError`] (a DB/codec
/// fault) rides through unchanged.
// Nominal `pub`: [`crate::service::EhrbaseService::index_conflicts`] (a `pub`
// method — the design-filled detection read has no SM trait binding) carries
// this type in its public signature.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// `ehr_id_does_not_exist` — the addressed EHR is unknown.
    #[error("EHR {0} does not exist (ehr_id_does_not_exist)")]
    EhrDoesNotExist(Uuid),
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
    /// G-8/G-9: map the precise EHR-index errors onto their dedicated SM
    /// statuses (`master07 §Errors`), bypassing the generic
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
/// PORT NOTE (G-16): `RESOURCE_STATUS.start_valid_time`/`end_valid_time` are
/// typed `@@` (an unresolved placeholder) in the SM — a recorded spec defect
/// (`resource_status.adoc:20,24`); implemented as ISO date-time strings.
fn parse_valid_time(raw: Option<&str>) -> Result<Option<jiff_sqlx::Timestamp>, ServiceError> {
    use jiff_sqlx::ToSqlx;
    match raw {
        None => Ok(None),
        Some(s) => s
            .parse::<jiff::Timestamp>()
            .map(|t| Some(t.to_sqlx()))
            .map_err(|_| ServiceError::BadRequest(format!("invalid valid_time: {s}"))),
    }
}

/// Render a [`LocationDesc`] as the stored canonical JSON, or SQL NULL.
///
/// PORT NOTE (G-12): `LOCATION_DESC` is an attribute-less stub in the SM
/// (`location_desc.adoc`) — a recorded spec defect; the designed contract
/// `{system_id, uri?, description?}` is our own design.
fn location_json(loc: Option<&LocationDesc>) -> Option<Value> {
    loc.map(|l| {
        json!({
            "system_id": l.system_id,
            "uri": l.uri,
            "description": l.description,
        })
    })
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
    let ehr_id: Uuid = row.try_get("ehr_id")?;
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
    let location = row
        .try_get::<Option<Value>, _>("location")?
        .map(|v| LocationDesc {
            system_id: v
                .get("system_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            uri: v.get("uri").and_then(Value::as_str).map(str::to_owned),
            description: v
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_owned),
        });
    Ok(EhrIndexEntry {
        ehr_id: ehr_id.to_string(),
        subject,
        status,
        location,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// G-8/G-9: the two declared EHR-index errors map to their dedicated SM
    /// statuses, never the generic `versioned_object_does_not_exist`.
    #[test]
    fn index_errors_map_to_dedicated_statuses() {
        let ehr = Uuid::now_v7();
        let ehr_sm: SmError = IndexError::EhrDoesNotExist(ehr).into();
        assert_eq!(ehr_sm.status, CallStatusType::EhrIdDoesNotExist);

        let subject = SubjectRef::person("p1", "demographic");
        let subj_sm: SmError = IndexError::SubjectDoesNotExist(subject).into();
        assert_eq!(subj_sm.status, CallStatusType::SubjectIdDoesNotExist);

        // A generic service fault still routes through the shared table (404).
        let svc: SmError = IndexError::Service(ServiceError::NotFound("x".into())).into();
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

pub mod types;
pub use types::*;
