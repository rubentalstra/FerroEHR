// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Integration tests for the local IHE ATNA Audit Record Repository
//! (`ferroehr::system_log::store`) against a real `PostgreSQL` 18
//! (shared testkit harness): the `audit` schema migrates cleanly alongside
//! `ext`/`ehr`, inserted records land with the promoted search columns and
//! the FHIR R4B `AuditEvent` payload (IHE BALP shape), and the retention
//! reaper deletes only rows older than the horizon (0 = keep forever).
//! No openEHR spec governs audit storage — our own design/extension (the
//! schema-separation rationale is BASE
//! `architecture_overview/master07-security.adoc` §Access logging).

use jiff::Timestamp;
use sqlx::Row;
use uuid::Uuid;

use ferroehr::system_log::event::{
    AuditEvent, EventActionCode, EventOutcome, EventType, ObjectClass,
};
use ferroehr::system_log::fhir;
use ferroehr::system_log::message::AuditContext;
use ferroehr::system_log::store::AuditStore;

fn ctx() -> AuditContext {
    AuditContext {
        source_id: "ferroehr".to_owned(),
        enterprise_site_id: "site-1".to_owned(),
        server_ip: "10.42.23.77".to_owned(),
        value_if_missing: "UNKNOWN".to_owned(),
    }
}

fn read_event(at: Timestamp) -> AuditEvent {
    let mut e = AuditEvent::new(
        EventActionCode::Read,
        ObjectClass::Composition,
        EventOutcome::Success,
    );
    "alice".clone_into(&mut e.user_id);
    e.client_ip = Some("10.0.0.9".to_owned());
    e.object_id = Some("8fa1::ferroehr::1".to_owned());
    e.event_type = Some(EventType::RestOperation("composition_get"));
    e.token_id = Some("jti-1".to_owned());
    e.tenant_id = Some(Uuid::nil());
    e.timestamp = at;
    e
}

#[tokio::test]
async fn insert_persists_promoted_columns_and_fhir_payload() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let store = AuditStore::new(pool.clone());

    let event = read_event("2026-07-10T08:30:00Z".parse().unwrap());
    let rendered = fhir::to_fhir(&event, &ctx(), Some("patient-42")).expect("render");
    store
        .insert(&event, Some("patient-42"), &rendered)
        .await
        .expect("insert");

    let row = sqlx::query(
        "SELECT action, outcome, event_code, operation, principal, patient_id, \
         resource_class, resource_id, client_ip, token_id, tenant_id, fhir, \
         delivered_syslog_at, delivered_fhir_feed_at \
         FROM audit.audit_event",
    )
    .fetch_one(&pool)
    .await
    .expect("one row");

    assert_eq!(row.get::<String, _>("action"), "R");
    assert_eq!(row.get::<i16, _>("outcome"), 0);
    assert_eq!(row.get::<String, _>("event_code"), "110110");
    assert_eq!(
        row.get::<Option<String>, _>("operation").as_deref(),
        Some("composition_get")
    );
    assert_eq!(
        row.get::<Option<String>, _>("principal").as_deref(),
        Some("alice")
    );
    assert_eq!(
        row.get::<Option<String>, _>("patient_id").as_deref(),
        Some("patient-42")
    );
    assert_eq!(row.get::<String, _>("resource_class"), "composition");
    assert_eq!(
        row.get::<Option<String>, _>("resource_id").as_deref(),
        Some("8fa1::ferroehr::1")
    );
    assert_eq!(
        row.get::<Option<String>, _>("client_ip").as_deref(),
        Some("10.0.0.9")
    );
    assert_eq!(
        row.get::<Option<String>, _>("token_id").as_deref(),
        Some("jti-1")
    );
    assert_eq!(row.get::<Option<Uuid>, _>("tenant_id"), Some(Uuid::nil()));
    // Both forwarding outbox stamps start pending.
    assert_eq!(
        row.get::<Option<jiff_sqlx::Timestamp>, _>("delivered_syslog_at")
            .map(jiff_sqlx::Timestamp::to_jiff),
        None
    );

    // The stored payload is the exact rendered BALP AuditEvent.
    let stored: serde_json::Value = row.get("fhir");
    assert_eq!(stored, serde_json::to_value(&rendered).expect("value"));
    assert_eq!(stored["resourceType"], "AuditEvent");
    assert_eq!(
        stored["meta"]["profile"][0],
        "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientRead"
    );
}

#[tokio::test]
async fn insert_batch_persists_every_record_with_identical_shape() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let store = AuditStore::new(pool.clone());

    // A drained batch: distinct events, one with a subject, one without.
    let first = read_event("2026-07-10T09:00:00Z".parse().unwrap());
    let mut second = read_event("2026-07-10T09:00:01Z".parse().unwrap());
    "bob".clone_into(&mut second.user_id);
    second.object_id = Some("9bb2::ferroehr::1".to_owned());
    let records = vec![
        (
            first.clone(),
            Some("patient-42".to_owned()),
            Some(fhir::to_fhir(&first, &ctx(), Some("patient-42")).expect("render")),
        ),
        (
            second.clone(),
            None,
            Some(fhir::to_fhir(&second, &ctx(), None).expect("render")),
        ),
    ];
    store.insert_batch(&records).await.expect("batch insert");

    let rows = sqlx::query(
        "SELECT principal, patient_id, resource_id, fhir \
         FROM audit.audit_event ORDER BY recorded_at",
    )
    .fetch_all(&pool)
    .await
    .expect("rows");
    assert_eq!(rows.len(), 2, "the whole batch landed");
    assert_eq!(
        rows[0].get::<Option<String>, _>("principal").as_deref(),
        Some("alice")
    );
    assert_eq!(
        rows[0].get::<Option<String>, _>("patient_id").as_deref(),
        Some("patient-42")
    );
    assert_eq!(
        rows[1].get::<Option<String>, _>("principal").as_deref(),
        Some("bob")
    );
    assert_eq!(rows[1].get::<Option<String>, _>("patient_id"), None);
    assert_eq!(
        rows[1].get::<Option<String>, _>("resource_id").as_deref(),
        Some("9bb2::ferroehr::1")
    );
    // The batch path stores the exact same canonical payload as the
    // per-event path.
    let stored: serde_json::Value = rows[0].get("fhir");
    assert_eq!(
        stored,
        serde_json::to_value(&records[0].2).expect("value"),
        "batch-stored FHIR document differs from the rendered one"
    );

    // An empty batch is a no-op, never an error.
    store.insert_batch(&[]).await.expect("empty batch");
}

#[tokio::test]
async fn reap_deletes_only_rows_past_the_horizon() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let store = AuditStore::new(pool.clone());

    // One fresh record and one 40 days old.
    let now = Timestamp::now();
    let old = now - jiff::SignedDuration::from_hours(40 * 24);
    for at in [now, old] {
        let event = read_event(at);
        let rendered = fhir::to_fhir(&event, &ctx(), None).expect("render");
        store.insert(&event, None, &rendered).await.expect("insert");
    }

    // retention 0 = keep forever.
    assert_eq!(store.reap(0).await.expect("reap 0"), 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.audit_event")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count, 2);

    // A 30-day horizon reaps exactly the row older than the horizon.
    assert_eq!(store.reap(30).await.expect("reap 30"), 1);
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.audit_event")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(remaining, 1);
}
