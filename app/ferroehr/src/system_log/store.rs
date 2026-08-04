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
//! The canonical stored form is the **FHIR R4B `AuditEvent`** (IHE BALP
//! shape, [`super::fhir`]) in the `fhir` jsonb column — the exact document
//! the RESTful-ATNA ITI-81 search serves; the promoted columns are derived
//! search keys, nothing more. Rows are append-only except the per-sink
//! delivery stamps (the forwarding outbox) and retention reaping.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

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

    /// Persist a whole drained batch in ONE multi-row `INSERT` (UNNEST over
    /// parallel column arrays) — the throughput path for the default
    /// store-only posture, where per-event round trips cannot keep up with a
    /// loaded write path. No row ids are returned: the batch path is used
    /// only when no per-row delivery stamping is needed (the syslog sink
    /// keeps the per-event [`Self::insert`]).
    ///
    /// # Errors
    /// [`AuditError::Store`] when serialization of a FHIR document or the
    /// INSERT fails (the whole batch fails together; the caller retries).
    pub async fn insert_batch(
        &self,
        records: &[(AuditEvent, Option<String>, FhirAuditEvent)],
    ) -> Result<(), AuditError> {
        if records.is_empty() {
            return Ok(());
        }
        let mut recorded_at: Vec<Timestamp> = Vec::with_capacity(records.len());
        let mut actions: Vec<String> = Vec::with_capacity(records.len());
        let mut outcomes: Vec<i16> = Vec::with_capacity(records.len());
        let mut event_codes: Vec<&str> = Vec::with_capacity(records.len());
        let mut operations: Vec<Option<&str>> = Vec::with_capacity(records.len());
        let mut principals: Vec<Option<&str>> = Vec::with_capacity(records.len());
        let mut patient_ids: Vec<Option<&str>> = Vec::with_capacity(records.len());
        let mut resource_classes: Vec<&str> = Vec::with_capacity(records.len());
        let mut resource_ids: Vec<Option<&str>> = Vec::with_capacity(records.len());
        let mut client_ips: Vec<Option<&str>> = Vec::with_capacity(records.len());
        let mut token_ids: Vec<Option<&str>> = Vec::with_capacity(records.len());
        let mut tenant_ids: Vec<Option<Uuid>> = Vec::with_capacity(records.len());
        let mut fhir_docs: Vec<serde_json::Value> = Vec::with_capacity(records.len());
        for (event, subject, fhir) in records {
            recorded_at.push(Timestamp::from(event.timestamp));
            actions.push(action_str(event));
            outcomes.push(outcome_smallint(event));
            event_codes.push(event_code(event));
            operations.push(operation(event));
            principals.push(nonempty_opt(&event.user_id));
            patient_ids.push(subject.as_deref());
            resource_classes.push(resource_class(event.object));
            resource_ids.push(event.object_id.as_deref());
            client_ips.push(event.client_ip.as_deref());
            token_ids.push(event.token_id.as_deref());
            tenant_ids.push(event.tenant_id);
            fhir_docs
                .push(serde_json::to_value(fhir).map_err(|e| AuditError::Store(e.to_string()))?);
        }
        sqlx::query(
            "INSERT INTO audit.audit_event (recorded_at, action, outcome, event_code, \
             operation, principal, patient_id, resource_class, resource_id, client_ip, \
             token_id, tenant_id, fhir) \
             SELECT * FROM UNNEST($1::timestamptz[], $2::text[], $3::smallint[], $4::text[], \
             $5::text[], $6::text[], $7::text[], $8::text[], $9::text[], $10::text[], \
             $11::text[], $12::uuid[], $13::jsonb[])",
        )
        .bind(recorded_at)
        .bind(actions)
        .bind(outcomes)
        .bind(event_codes)
        .bind(operations)
        .bind(principals)
        .bind(patient_ids)
        .bind(resource_classes)
        .bind(resource_ids)
        .bind(client_ips)
        .bind(token_ids)
        .bind(tenant_ids)
        .bind(fhir_docs)
        .execute(&self.pool)
        .await
        .map_err(|e| AuditError::Store(e.to_string()))?;
        Ok(())
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

    /// The RESTful-ATNA ITI-81 retrieval: filtered, newest-first stored FHIR
    /// `AuditEvent` documents plus the total match count. The filter is the
    /// supported ITI-81 search-parameter subset ([`AuditSearchFilter`]).
    ///
    /// # Errors
    /// [`AuditError::Store`] when either query fails.
    pub async fn search(
        &self,
        filter: &AuditSearchFilter,
    ) -> Result<(i64, Vec<serde_json::Value>), AuditError> {
        use sea_query::{Alias, Expr, ExprTrait, Order, PostgresQueryBuilder, Query};
        use sea_query_sqlx::SqlxBinder as _;

        let table = (Alias::new("audit"), Alias::new("audit_event"));
        let condition = {
            let mut cond = sea_query::Cond::all();
            // Timestamps bind as RFC 3339 text cast to timestamptz: the
            // sea-query binder's with-jiff is unimplemented upstream (see the
            // workspace manifest note), so the value crosses as text.
            if let Some(from) = filter.from {
                cond = cond.add(
                    Expr::col(Alias::new("recorded_at"))
                        .gte(Expr::val(from.to_string()).cast_as("timestamptz")),
                );
            }
            if let Some(to) = filter.to {
                cond = cond.add(
                    Expr::col(Alias::new("recorded_at"))
                        .lte(Expr::val(to.to_string()).cast_as("timestamptz")),
                );
            }
            if let Some(patient) = &filter.patient {
                cond = cond.add(Expr::col(Alias::new("patient_id")).eq(patient.clone()));
            }
            if let Some(agent) = &filter.agent {
                cond = cond.add(Expr::col(Alias::new("principal")).eq(agent.clone()));
            }
            if let Some(entity) = &filter.entity {
                cond = cond.add(Expr::col(Alias::new("resource_id")).eq(entity.clone()));
            }
            if let Some(outcome) = filter.outcome {
                cond = cond.add(Expr::col(Alias::new("outcome")).eq(outcome));
            }
            if let Some(action) = &filter.action {
                cond = cond.add(Expr::col(Alias::new("action")).eq(action.clone()));
            }
            cond
        };

        let (count_sql, count_values) = Query::select()
            .expr(Expr::col(sea_query::Asterisk).count())
            .from(table.clone())
            .cond_where(condition.clone())
            .build_sqlx(PostgresQueryBuilder);
        let total: i64 = sqlx::query_scalar_with(sqlx::AssertSqlSafe(count_sql), count_values)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AuditError::Store(e.to_string()))?;

        let (sql, values) = Query::select()
            .column(Alias::new("fhir"))
            .from(table)
            .cond_where(condition)
            .order_by(Alias::new("recorded_at"), Order::Desc)
            .order_by(Alias::new("stored_at"), Order::Desc)
            .limit(u64::try_from(filter.count.clamp(1, 1000)).unwrap_or(50))
            .offset(u64::try_from(Ord::max(filter.offset, 0)).unwrap_or(0))
            .build_sqlx(PostgresQueryBuilder);
        let rows = sqlx::query_with(sqlx::AssertSqlSafe(sql), values)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AuditError::Store(e.to_string()))?;
        let documents = rows
            .into_iter()
            .map(|row| {
                row.try_get::<serde_json::Value, _>("fhir")
                    .map_err(|e| AuditError::Store(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((total, documents))
    }
}

/// The supported ITI-81 search-parameter subset, resolved by the REST layer.
#[derive(Debug, Clone, Default)]
pub struct AuditSearchFilter {
    /// `date=ge…` — events at/after this instant.
    pub from: Option<jiff::Timestamp>,
    /// `date=le…` — events at/before this instant.
    pub to: Option<jiff::Timestamp>,
    /// `patient` — the recorded patient (EHR subject) id.
    pub patient: Option<String>,
    /// `agent` — the authenticated principal.
    pub agent: Option<String>,
    /// `entity` — the touched resource id.
    pub entity: Option<String>,
    /// `outcome` — the DICOM outcome indicator (0/4/8/12).
    pub outcome: Option<i16>,
    /// `action` — the DICOM action code (C/R/U/D/E).
    pub action: Option<String>,
    /// `_count` — page size (default 50, capped at 1000).
    pub count: i64,
    /// `_offset` — page offset.
    pub offset: i64,
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
