// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The admin storage-parity sweep against a real `PostgreSQL` 18 (shared
//! testkit harness).
//!
//! NOTE: no openEHR spec governs storage mechanics — our own design/extension;
//! the sweep re-derives every stored version from its `node` rows and compares
//! the result with the materialized `vo_version.body`.
//!
//! Every tamper case reaches PAST the service into the stored rows with raw
//! SQL, which is the point: an attacker or a corrupt page does not go through
//! the write path, so neither does the fixture. A case that tampers must prove
//! the `UPDATE`/`DELETE` matched something, or it would pass while testing
//! nothing.

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this module's \
              fixture helpers; a failing fixture must panic at the fixture (the \
              Rust Book ch11)"
)]

use sqlx::PgPool;
use uuid::Uuid;

use ferroehr::ids::EhrId;
use ferroehr::service::FerroEhrService;
use ferroehr::service::admin::integrity::{StorageParityDefect, StorageParityScope};

use crate::fixtures::{composition, folder, uv};

/// A minimal *valid* root FOLDER (a folder-hierarchy root).
/// An EHR carrying one committed COMPOSITION — three content versions in all
/// (`EHR_STATUS`, `EHR_ACCESS`, the COMPOSITION), every one of which the sweep
/// reads.
async fn seed_ehr_with_composition(svc: &FerroEhrService) -> EhrId {
    let ehr_id = svc.create_ehr(None).await.expect("ehr create");
    svc.create_composition(ehr_id, uv(&composition("Encounter"), "249", None))
        .await
        .expect("composition commit");
    ehr_id
}

/// The `(vo_id, sys_version)` of the one stored version of `kind` in `ehr_id`.
async fn one_version(pool: &PgPool, ehr_id: EhrId, kind: &str) -> (Uuid, i32) {
    sqlx::query_as(
        "SELECT vo_id, sys_version FROM vo_version WHERE ehr_id = $1 AND kind = $2 \
         ORDER BY sys_version DESC LIMIT 1",
    )
    .bind(ehr_id.0)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("a stored version of the requested kind")
}

/// Run a statement that must touch at least one row — a tamper fixture that
/// matched nothing would let its test pass while proving nothing.
async fn tamper(pool: &PgPool, sql: &'static str, vo_id: Uuid, sys_version: i32) {
    let affected = sqlx::query(sql)
        .bind(vo_id)
        .bind(sys_version)
        .execute(pool)
        .await
        .expect("the tamper statement runs")
        .rows_affected();
    assert!(
        affected > 0,
        "the tamper statement matched no rows, so nothing was actually tampered"
    );
}

#[tokio::test]
async fn a_committed_composition_sweeps_clean() {
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());
    seed_ehr_with_composition(&svc).await;

    let report = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("sweep");

    assert!(
        report.is_clean(),
        "a repository written through the commit path must be parity-clean: {report:?}"
    );
    assert!(report.mismatches.is_empty());
    assert!(
        report.versions_checked >= 3,
        "the EHR create plus one composition commit stores at least three versions, saw {}",
        report.versions_checked
    );
    assert_eq!(
        report.versions_checked,
        report.versions_with_body + report.versions_without_body
    );
}

#[tokio::test]
async fn a_tampered_node_row_is_reported_as_content_differs() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = FerroEhrService::new(pool.clone());
    let ehr_id = seed_ehr_with_composition(&svc).await;
    let (vo_id, sys_version) = one_version(&pool, ehr_id, "COMPOSITION").await;

    // The AQL index copy, which read-time signature verification never sees.
    tamper(
        &pool,
        "UPDATE node SET data = jsonb_set(data, '{archetype_node_id}', '\"tampered\"') \
         WHERE vo_id = $1 AND sys_version = $2 AND num = 0",
        vo_id,
        sys_version,
    )
    .await;

    let report = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("sweep");

    assert_eq!(
        report.mismatch_count, 1,
        "exactly the tampered version must be reported: {report:?}"
    );
    let found = &report.mismatches[0];
    assert_eq!(found.vo_id, vo_id);
    assert_eq!(found.sys_version, sys_version);
    assert_eq!(found.kind, "COMPOSITION");
    assert_eq!(found.defect, StorageParityDefect::ContentDiffers);
    assert!(!report.truncated);
}

#[tokio::test]
async fn a_tampered_version_body_is_reported_as_content_differs() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = FerroEhrService::new(pool.clone());
    let ehr_id = seed_ehr_with_composition(&svc).await;
    let (vo_id, sys_version) = one_version(&pool, ehr_id, "COMPOSITION").await;

    // The other copy: the materialized projection every point read serves.
    tamper(
        &pool,
        "UPDATE vo_version SET body = (jsonb_set((body)::jsonb, '{archetype_node_id}', '\"tampered\"'))::text \
         WHERE vo_id = $1 AND sys_version = $2",
        vo_id,
        sys_version,
    )
    .await;

    let report = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("sweep");

    assert_eq!(report.mismatch_count, 1, "{report:?}");
    let found = &report.mismatches[0];
    assert_eq!(found.vo_id, vo_id);
    assert_eq!(found.sys_version, sys_version);
    assert_eq!(found.defect, StorageParityDefect::ContentDiffers);
}

#[tokio::test]
async fn a_version_whose_node_rows_are_gone_is_reported_as_nodes_missing() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = FerroEhrService::new(pool.clone());
    let ehr_id = seed_ehr_with_composition(&svc).await;
    let (vo_id, sys_version) = one_version(&pool, ehr_id, "COMPOSITION").await;

    tamper(
        &pool,
        "DELETE FROM node WHERE vo_id = $1 AND sys_version = $2",
        vo_id,
        sys_version,
    )
    .await;

    let report = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("sweep");

    assert_eq!(report.mismatch_count, 1, "{report:?}");
    let found = &report.mismatches[0];
    assert_eq!(found.vo_id, vo_id);
    assert_eq!(found.sys_version, sys_version);
    assert_eq!(found.defect, StorageParityDefect::NodesMissing);
}

#[tokio::test]
async fn a_logically_deleted_version_sweeps_clean() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = FerroEhrService::new(pool.clone());
    let ehr_id = svc.create_ehr(None).await.expect("ehr create");
    let dir = svc
        .create_directory(ehr_id, uv(&folder("root"), "249", None))
        .await
        .expect("directory create");
    svc.delete_directory(ehr_id, Some(dir.uid.parse().expect("ovid")), None)
        .await
        .expect("directory delete");

    // The delete committed a version with data Void (RM common master06
    // §Logical Deletion): no body, and no node rows to disagree with it.
    let bodiless: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM vo_version WHERE ehr_id = $1 AND kind = 'FOLDER' AND body IS NULL",
    )
    .bind(ehr_id.0)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(bodiless, 1, "the delete must store one bodiless version");

    let report = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("sweep");

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(report.versions_without_body, 1);
}

#[tokio::test]
async fn node_rows_under_a_bodiless_version_are_reported_as_unexpected_nodes() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = FerroEhrService::new(pool.clone());
    let ehr_id = svc.create_ehr(None).await.expect("ehr create");
    let dir = svc
        .create_directory(ehr_id, uv(&folder("root"), "249", None))
        .await
        .expect("directory create");
    svc.delete_directory(ehr_id, Some(dir.uid.parse().expect("ovid")), None)
        .await
        .expect("directory delete");
    let (vo_id, sys_version) = one_version(&pool, ehr_id, "FOLDER").await;

    // Give the bodiless version the previous version's rows: content in the
    // AQL index that no served version accounts for.
    let inserted = sqlx::query(
        "INSERT INTO node (vo_id, sys_version, ehr_id, num, num_cap, parent_num, citem_num, \
         rm_type, archetype, arch_entity, arch_concept, arch_major, name, path, data) \
         SELECT vo_id, $2, ehr_id, num, num_cap, parent_num, citem_num, rm_type, archetype, \
         arch_entity, arch_concept, arch_major, name, path, data \
         FROM node WHERE vo_id = $1 AND sys_version = $2 - 1",
    )
    .bind(vo_id)
    .bind(sys_version)
    .execute(&pool)
    .await
    .expect("insert the stray rows")
    .rows_affected();
    assert!(inserted > 0, "the fixture must actually insert stray rows");

    let report = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("sweep");

    assert_eq!(report.mismatch_count, 1, "{report:?}");
    let found = &report.mismatches[0];
    assert_eq!(found.vo_id, vo_id);
    assert_eq!(found.sys_version, sys_version);
    assert_eq!(found.defect, StorageParityDefect::UnexpectedNodes);
}

/// A scoped sweep reads only what its bounds cover, which is how an operator
/// verifies one record without reading the whole repository (#3110).
///
/// The tamper lives in one EHR; a sweep scoped to the other reports clean and
/// never even counts the damaged version, while the unscoped sweep finds it.
#[tokio::test]
async fn a_scoped_sweep_covers_only_its_own_ehr() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = FerroEhrService::new(pool.clone());

    let damaged = seed_ehr_with_composition(&svc).await;
    let intact = seed_ehr_with_composition(&svc).await;
    let (vo_id, sys_version) = one_version(&pool, damaged, "COMPOSITION").await;
    tamper(
        &pool,
        "UPDATE vo_version SET body = '{\"_type\":\"COMPOSITION\"}' \
         WHERE vo_id = $1 AND sys_version = $2",
        vo_id,
        sys_version,
    )
    .await;

    let clean = svc
        .verify_storage_parity(StorageParityScope {
            ehr_id: Some(intact.0),
            committed_since: None,
        })
        .await
        .expect("scoped sweep");
    assert!(clean.is_clean(), "the intact EHR sweeps clean: {clean:?}");
    assert!(
        clean.versions_checked > 0,
        "the scoped sweep still read that EHR's own versions: {clean:?}"
    );

    let scoped = svc
        .verify_storage_parity(StorageParityScope {
            ehr_id: Some(damaged.0),
            committed_since: None,
        })
        .await
        .expect("scoped sweep");
    assert_eq!(scoped.mismatch_count, 1, "the tamper is in this EHR");

    let whole = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("unscoped sweep");
    assert_eq!(whole.mismatch_count, 1);
    assert!(
        whole.versions_checked > scoped.versions_checked,
        "the unscoped sweep reads more than one EHR's versions: {whole:?} vs {scoped:?}"
    );
}

/// A `committed_since` bound covers only versions written at or after it, so
/// an operator can verify what changed since an incident (#3110).
#[tokio::test]
async fn a_committed_since_bound_excludes_earlier_versions() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = FerroEhrService::new(pool.clone());

    seed_ehr_with_composition(&svc).await;
    let before = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("unscoped sweep");
    assert!(before.versions_checked > 0);

    // The cutoff is read from the database, not the process: `sys_period` is
    // stamped by PostgreSQL's own clock, and a container's clock need not agree
    // with this process's to the millisecond.
    let cutoff: jiff_sqlx::Timestamp = sqlx::query_scalar("SELECT now()")
        .fetch_one(&pool)
        .await
        .expect("the database's own clock");
    let cutoff = cutoff.to_jiff();

    // Everything seeded so far is older than the cutoff, so it covers nothing;
    // the sweep succeeds and reports an empty count rather than failing or
    // silently falling back to the whole store.
    let after = svc
        .verify_storage_parity(StorageParityScope {
            ehr_id: None,
            committed_since: Some(cutoff),
        })
        .await
        .expect("bounded sweep");
    assert_eq!(after.versions_checked, 0, "{after:?}");

    // A version committed after the cutoff is covered.
    seed_ehr_with_composition(&svc).await;
    let later = svc
        .verify_storage_parity(StorageParityScope {
            ehr_id: None,
            committed_since: Some(cutoff),
        })
        .await
        .expect("bounded sweep");
    assert!(later.versions_checked > 0, "{later:?}");

    let whole = svc
        .verify_storage_parity(StorageParityScope::default())
        .await
        .expect("unscoped sweep");
    assert!(
        later.versions_checked < whole.versions_checked,
        "the bound still excludes the versions written before it: {later:?} vs {whole:?}"
    );
}
