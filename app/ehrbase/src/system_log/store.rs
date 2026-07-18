//! The local IHE ATNA **Audit Record Repository**: the `audit.audit_event`
//! table writer + retention reaper.
//!
//! NOTE: no openEHR spec governs audit storage — our own design/extension.
//! openEHR endorses in-system access logging and rules it out of the EHR
//! content ("read accesses by application users to EHR data should be logged
//! in the EHR system … currently openEHR does not support \[logs as part of
//! the EHR proper\]" — BASE `architecture_overview/master07-security.adoc`
//! §Access logging), so the store lives in its own `audit` schema, strictly
//! outside the EHR data (`migrations/audit/0001_baseline.sql`).
//!
//! The canonical stored form is the **FHIR R4 `AuditEvent`** (IHE BALP
//! shape, [`super::fhir`]) in the `fhir` jsonb column — the exact document
//! the RESTful-ATNA ITI-81 search serves; the promoted columns are derived
//! search keys, nothing more. Rows are append-only except the per-sink
//! delivery stamps (the forwarding outbox) and retention reaping.

use jiff_sqlx::Timestamp;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::system_log::AuditError;
use crate::system_log::codes::AtnaAction;
use crate::system_log::event::{AuditEvent, EventType, ObjectClass};
use crate::system_log::fhir::FhirAuditEvent;

/// The PG-backed Audit Record Repository (the `store` sink).
#[derive(Debug, Clone)]
pub struct AuditStore {
    pool: PgPool,
}

impl AuditStore {
    /// A store over the given pool. The pool should be the plain
    /// (non-tenant-scoped) pool: the audit schema is not RLS-scoped and the
    /// drain task runs outside any request's tenant session.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        AuditStore { pool }
    }

    /// Persist one audit record: the rendered FHIR `AuditEvent` as the
    /// canonical payload plus the promoted search columns derived from the
    /// resolved event. Returns the stored row id (for the per-sink delivery
    /// stamps).
    ///
    /// # Errors
    /// [`AuditError::Store`] when serialization of the FHIR document or the
    /// INSERT fails.
    pub async fn insert(
        &self,
        event: &AuditEvent,
        subject: Option<&str>,
        fhir: &FhirAuditEvent,
    ) -> Result<Uuid, AuditError> {
        let fhir_json = serde_json::to_value(fhir).map_err(|e| AuditError::Store(e.to_string()))?;
        let outcome = outcome_smallint(event);
        sqlx::query(
            "INSERT INTO audit.audit_event (recorded_at, action, outcome, event_code, \
             operation, principal, patient_id, resource_class, resource_id, client_ip, \
             token_id, tenant_id, fhir) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) \
             RETURNING id",
        )
        .bind(Timestamp::from(event.timestamp))
        .bind(action_str(event))
        .bind(outcome)
        .bind(event_code(event))
        .bind(operation(event))
        .bind(nonempty_opt(&event.user_id))
        .bind(subject)
        .bind(resource_class(event.object))
        .bind(event.object_id.as_deref())
        .bind(event.client_ip.as_deref())
        .bind(event.token_id.as_deref())
        .bind(event.tenant_id)
        .bind(fhir_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AuditError::Store(e.to_string()))?
        .try_get::<Uuid, _>("id")
        .map_err(|e| AuditError::Store(e.to_string()))
    }

    /// Stamp a row as delivered by the syslog sink. Delivery stamps are
    /// best-effort bookkeeping: a failure is logged, never propagated (the
    /// record itself is already durable).
    pub async fn mark_syslog_delivered(&self, id: Uuid) {
        let outcome =
            sqlx::query("UPDATE audit.audit_event SET delivered_syslog_at = now() WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await;
        if let Err(e) = outcome {
            tracing::warn!("audit store: stamping delivered_syslog_at failed: {e}");
        }
    }

    /// Stamp a row as delivered by the FHIR feed sink (see
    /// [`Self::mark_syslog_delivered`] for the best-effort semantics).
    pub async fn mark_fhir_feed_delivered(&self, id: Uuid) {
        let outcome = sqlx::query(
            "UPDATE audit.audit_event SET delivered_fhir_feed_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await;
        if let Err(e) = outcome {
            tracing::warn!("audit store: stamping delivered_fhir_feed_at failed: {e}");
        }
    }

    /// The oldest rows not yet delivered by the FHIR feed sink (the ITI-20
    /// ATX:FHIR Feed outbox): `(id, fhir document)`, oldest first.
    ///
    /// # Errors
    /// [`AuditError::Store`] when the SELECT fails.
    pub async fn pending_fhir_feed(
        &self,
        limit: i64,
    ) -> Result<Vec<(Uuid, serde_json::Value)>, AuditError> {
        let rows = sqlx::query(
            "SELECT id, fhir FROM audit.audit_event \
             WHERE delivered_fhir_feed_at IS NULL \
             ORDER BY stored_at ASC LIMIT $1",
        )
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuditError::Store(e.to_string()))?;
        rows.into_iter()
            .map(|row| {
                Ok((
                    row.try_get::<Uuid, _>("id")
                        .map_err(|e| AuditError::Store(e.to_string()))?,
                    row.try_get::<serde_json::Value, _>("fhir")
                        .map_err(|e| AuditError::Store(e.to_string()))?,
                ))
            })
            .collect()
    }

    /// Delete records older than `retention_days` (0 = keep forever).
    /// Returns the number of reaped rows.
    ///
    /// # Errors
    /// [`AuditError::Store`] when the DELETE fails.
    pub async fn reap(&self, retention_days: u32) -> Result<u64, AuditError> {
        if retention_days == 0 {
            return Ok(0);
        }
        let result = sqlx::query(
            "DELETE FROM audit.audit_event \
             WHERE recorded_at < now() - make_interval(days => $1)",
        )
        .bind(i32::try_from(retention_days).unwrap_or(i32::MAX))
        .execute(&self.pool)
        .await
        .map_err(|e| AuditError::Store(e.to_string()))?;
        Ok(result.rows_affected())
    }
}

fn action_str(event: &AuditEvent) -> String {
    event.action.as_char().to_string()
}

fn outcome_smallint(event: &AuditEvent) -> i16 {
    use crate::system_log::codes::AtnaOutcome;
    // The DICOM indicator values are 0/4/8/12 — always in smallint range.
    i16::try_from(event.outcome.as_i32()).unwrap_or(i16::MAX)
}

fn event_code(event: &AuditEvent) -> &'static str {
    use crate::system_log::codes::AtnaObject;
    event.object.event_id(event.action).0
}

/// The `operation` column: the ITS-REST operation id, or the DCM
/// login/logout `EventTypeCode` csd-code for authentication records.
fn operation(event: &AuditEvent) -> Option<&'static str> {
    match event.event_type {
        Some(EventType::RestOperation(op)) => Some(op),
        Some(EventType::Login) => Some("110122"),
        Some(EventType::Logout) => Some("110123"),
        None => None,
    }
}

const fn resource_class(object: ObjectClass) -> &'static str {
    match object {
        ObjectClass::Ehr => "ehr",
        ObjectClass::Composition => "composition",
        ObjectClass::Contribution => "contribution",
        ObjectClass::Directory => "directory",
        ObjectClass::Query => "query",
        ObjectClass::Template => "template",
        ObjectClass::Demographic => "demographic",
        ObjectClass::Extract => "extract",
        ObjectClass::ApplicationActivity => "application_activity",
        ObjectClass::Authentication => "authentication",
    }
}

fn nonempty_opt(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}
