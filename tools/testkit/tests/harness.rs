#![allow(clippy::expect_used, clippy::panic)] // test assertions
//! Live gate for the harness itself: server acquisition, template build,
//! and per-test cloning against a real `PostgreSQL` 18.

/// Two clones from one process: unique databases, both fully migrated
/// (the `ehr.node` table exists and is queryable), independent state.
#[tokio::test]
async fn clones_are_unique_and_fully_migrated() {
    let first = testkit::db().await.expect("first clone");
    let second = testkit::db().await.expect("second clone");
    assert_ne!(first.name(), second.name(), "clones must be distinct");

    for db in [&first, &second] {
        let migrated: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
             WHERE table_schema = 'ehr' AND table_name = 'node')",
        )
        .fetch_one(&db.pool())
        .await
        .expect("schema probe");
        assert!(migrated, "clone {} is not migrated", db.name());
    }

    // Writes stay isolated per clone: an EHR row in one never shows in the
    // other (both tables start empty — the template carries schema only).
    sqlx::query(
        "INSERT INTO ehr.ehr (id, system_id, time_created) \
         VALUES (gen_random_uuid(), 'testkit', now())",
    )
    .execute(&first.pool())
    .await
    .expect("insert into first");
    let counts: (i64, i64) = (
        sqlx::query_scalar("SELECT count(*) FROM ehr.ehr")
            .fetch_one(&first.pool())
            .await
            .expect("count first"),
        sqlx::query_scalar("SELECT count(*) FROM ehr.ehr")
            .fetch_one(&second.pool())
            .await
            .expect("count second"),
    );
    assert_eq!(counts, (1, 0), "clone state leaked between databases");
}

/// The migration fingerprint is stable within a build — the template cache
/// key must not wobble between calls.
#[test]
fn fingerprint_is_stable() {
    assert_eq!(
        ehrbase::db::migration_fingerprint(),
        ehrbase::db::migration_fingerprint()
    );
}
