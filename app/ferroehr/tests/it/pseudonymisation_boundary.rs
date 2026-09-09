// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The pseudonymisation cutover, exercised the way production runs it.
//!
//! NOTE: no openEHR spec governs storage layout — our own design/extension
//! (GDPR Art. 4(5) and Art. 32(1)(a); the migrations carry the derivation).
//!
//! The testkit clones a template that is already fully migrated, so every
//! other DB test meets the demographic schema empty and the data move runs
//! against nothing. That is precisely the half production does not have: an
//! installation upgrading into this release carries parties in `ehr`, and the
//! move is the only thing that carries them across. These tests therefore
//! reconstruct the pre-move state with raw SQL inside the migrated clone and
//! re-run the cutover statements against it, so the move is proven on data
//! rather than on an empty schema.

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this module's \
              fixture helpers; a failing fixture must panic at the fixture (the \
              Rust Book ch11)"
)]

use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

/// The cutover statements of `demographic/0002_move_parties`, read from the
/// migration itself rather than restated here.
///
/// A copy would drift from the migration the moment either changed, and a test
/// asserting a copy proves nothing about what an installation actually runs.
fn cutover_sql() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/demographic/0002_move_parties.sql"
    ))
    .expect("read the cutover migration")
}

/// Apply the cutover the way the migrator does: the whole file as raw SQL
/// inside ONE transaction.
///
/// Both properties are load-bearing. The file is multi-statement, which a
/// prepared statement refuses; and its `ON COMMIT DROP` temporary tables carry
/// the move's working set between statements, so running the statements under
/// autocommit would drop them after the first one.
async fn run_cutover(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(cutover_sql()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

/// Write the audit and contribution rows every version row needs, leaving the
/// boundary constraints exactly as the migrated clone carries them.
///
/// Returns `(contribution_id, audit_id)`.
async fn change_control(pool: &PgPool, schema: &str, ehr_id: Option<Uuid>) -> (Uuid, Uuid) {
    let (audit_id, contribution_id) = (Uuid::now_v7(), Uuid::now_v7());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {schema}.audit (id, system_id, change_type, committer) \
         VALUES ($1, 'test.system', '249', \
                 '{{\"_type\":\"PARTY_IDENTIFIED\",\"name\":\"tester\"}}'::jsonb)"
    )))
    .bind(audit_id)
    .execute(pool)
    .await
    .expect("seed audit");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO {schema}.contribution (id, ehr_id, audit_id) VALUES ($1, $2, $3)"
    )))
    .bind(contribution_id)
    .bind(ehr_id)
    .bind(audit_id)
    .execute(pool)
    .await
    .expect("seed contribution");
    (contribution_id, audit_id)
}

/// Put one versioned object back where the pre-move release stored it: `ehr`,
/// with a NULL `ehr_id`.
///
/// The clone the testkit hands us has already run the cutover, so its four
/// boundary constraints come off first: re-running the migration must find the
/// schema as the previous release left it.
async fn seed_party_in_the_clinical_schema(pool: &PgPool, kind: &str) -> Uuid {
    for statement in [
        "ALTER TABLE ehr.vo_version DROP CONSTRAINT IF EXISTS ck_vo_version_ehr_scoped",
        "ALTER TABLE ehr.contribution DROP CONSTRAINT IF EXISTS ck_contribution_ehr_scoped",
        "ALTER TABLE demographic.vo_version DROP CONSTRAINT IF EXISTS ck_dem_vo_version_unscoped",
        "ALTER TABLE demographic.contribution DROP CONSTRAINT IF EXISTS ck_dem_contribution_unscoped",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("drop the boundary constraint");
    }
    let (contribution_id, audit_id) = change_control(pool, "ehr", None).await;
    let vo_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO ehr.vo_version \
           (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, \
            creating_system_id, contribution_id, audit_id, body) \
         VALUES ($1, $2, NULL, 1, 1, tstzrange(now(), NULL), 'test.system', $3, $4, $5)",
    )
    .bind(vo_id)
    .bind(kind)
    .bind(contribution_id)
    .bind(audit_id)
    .bind(format!("{{\"_type\":\"{kind}\"}}"))
    .execute(pool)
    .await
    .expect("seed party version");
    vo_id
}

async fn count(pool: &PgPool, sql: &'static str, vo_id: Uuid) -> i64 {
    sqlx::query(sql)
        .bind(vo_id)
        .fetch_one(pool)
        .await
        .expect("count")
        .try_get::<i64, _>(0)
        .expect("count column")
}

#[tokio::test]
async fn the_cutover_carries_a_party_out_of_the_clinical_schema() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let vo_id = seed_party_in_the_clinical_schema(&pool, "PERSON").await;

    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM ehr.vo_version WHERE vo_id = $1",
            vo_id
        )
        .await,
        1,
        "the fixture really put the party where the pre-move release stored it"
    );

    run_cutover(&pool)
        .await
        .expect("the cutover migration runs against real data");

    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM ehr.vo_version WHERE vo_id = $1",
            vo_id
        )
        .await,
        0,
        "the party has left the clinical schema"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM demographic.vo_version WHERE vo_id = $1",
            vo_id
        )
        .await,
        1,
        "and arrived in the demographic one"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM demographic.contribution c \
             JOIN demographic.vo_version v ON v.contribution_id = c.id WHERE v.vo_id = $1",
            vo_id
        )
        .await,
        1,
        "with its contribution, so the change-control chain is intact on the far side"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM demographic.audit a \
             JOIN demographic.vo_version v ON v.audit_id = a.id WHERE v.vo_id = $1",
            vo_id
        )
        .await,
        1,
        "and its audit"
    );
}

#[tokio::test]
async fn the_boundary_refuses_a_party_written_back_to_the_clinical_schema() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();

    // Change control first, then the write a code path that missed the split
    // would attempt. The boundary constraint is left in place: this asserts the
    // clone the testkit hands every other test already carries it.
    // The contribution needs a real EHR: its own half of the boundary already
    // refuses an EHR-less one, which is the constraint the sibling test covers.
    let ehr_id = Uuid::now_v7();
    sqlx::query("INSERT INTO ehr.ehr (id, system_id) VALUES ($1, 'test.system')")
        .bind(ehr_id)
        .execute(&pool)
        .await
        .expect("seed an EHR");
    let (contribution_id, audit_id) = change_control(&pool, "ehr", Some(ehr_id)).await;
    let refused = sqlx::query(
        "INSERT INTO ehr.vo_version \
           (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, \
            creating_system_id, contribution_id, audit_id, body) \
         VALUES ($1, 'PERSON', NULL, 1, 1, tstzrange(now(), NULL), 'test.system', $2, $3, '{}')",
    )
    .bind(Uuid::now_v7())
    .bind(contribution_id)
    .bind(audit_id)
    .execute(&pool)
    .await;

    let error = refused.expect_err("an EHR-less row must not enter the clinical schema");
    assert!(
        error.to_string().contains("ck_vo_version_ehr_scoped"),
        "the refusal names the boundary constraint rather than failing obscurely: {error}"
    );
}

#[tokio::test]
async fn the_boundary_refuses_an_ehr_scoped_object_in_the_demographic_schema() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();

    let (contribution_id, audit_id) = change_control(&pool, "demographic", None).await;
    let refused = sqlx::query(
        "INSERT INTO demographic.vo_version \
           (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, \
            creating_system_id, contribution_id, audit_id, body) \
         VALUES ($1, 'COMPOSITION', $2, 1, 1, tstzrange(now(), NULL), 'test.system', \
                 $3, $4, '{}')",
    )
    .bind(Uuid::now_v7())
    .bind(Uuid::now_v7())
    .bind(contribution_id)
    .bind(audit_id)
    .execute(&pool)
    .await;

    let error = refused.expect_err("a clinical object must not enter the demographic schema");
    assert!(
        error.to_string().contains("ck_dem_vo_version_unscoped"),
        "the refusal names the boundary constraint: {error}"
    );
}

#[tokio::test]
async fn the_cutover_refuses_an_ehr_less_row_it_does_not_classify() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    // A kind the move does not know about. The migration must stop rather than
    // guess which domain it belongs to, and rather than sweep it across on a
    // `ehr_id IS NULL` predicate.
    let vo_id = seed_party_in_the_clinical_schema(&pool, "COMPOSITION").await;

    let refused = run_cutover(&pool).await;

    let error = refused.expect_err("an unclassified EHR-less row must refuse the upgrade");
    let text = error.to_string();
    assert!(
        text.contains("does not") && text.contains("COMPOSITION"),
        "the refusal names the kind it found, so an operator can decide: {text}"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM demographic.vo_version WHERE vo_id = $1",
            vo_id
        )
        .await,
        0,
        "and nothing was moved"
    );
}
