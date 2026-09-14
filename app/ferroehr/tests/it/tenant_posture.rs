// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The tenant reader fails closed under the multi-tenant posture (#3341).
//!
//! `ext.current_tenant_id()` resolved an unset `ferroehr.tenant_id` to the
//! reserved default tenant, so a connection that declared no tenant read and
//! wrote the default tenant's rows under every row policy. The server now
//! stamps the tenancy posture at boot, declares the default tenant on every
//! connection it opens, and the reader refuses an undeclared tenant only under
//! `multi`. Proven here against a real database.

#![expect(
    clippy::expect_used,
    reason = "integration tests fail loudly on harness errors"
)]

use ferroehr::db::{self, DbConfig};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

/// A connection on its own credential, the way a repair session or a foreign
/// client reaches the database: no pool hook, no tenant declared.
async fn raw(url: &str) -> PgConnection {
    let mut conn = PgConnection::connect(url).await.expect("raw connect");
    sqlx::query("SET search_path TO ehr, ext, public")
        .execute(&mut conn)
        .await
        .expect("search_path");
    conn
}

async fn tenant_of(conn: &mut PgConnection) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("SELECT ext.current_tenant_id()")
        .fetch_one(conn)
        .await
}

#[tokio::test]
async fn an_undeclared_tenant_is_refused_under_the_multi_posture() {
    let testdb = testkit::db().await.expect("testkit database");
    let pool = testdb.pool();
    db::stamp_tenancy_posture(&pool, true)
        .await
        .expect("stamp multi");

    let mut conn = raw(testdb.url()).await;
    let err = tenant_of(&mut conn)
        .await
        .expect_err("an undeclared tenant must be refused under multi");
    let text = err.to_string();
    assert!(
        text.contains("multi-tenant"),
        "the refusal names the posture: {text}"
    );

    // An empty setting is unset, not a tenant.
    sqlx::query("SELECT set_config('ferroehr.tenant_id', '', false)")
        .execute(&mut conn)
        .await
        .expect("set empty");
    tenant_of(&mut conn)
        .await
        .expect_err("an empty setting is still undeclared");

    // A declared tenant reads back as itself.
    let tenant = Uuid::now_v7();
    sqlx::query("SELECT set_config('ferroehr.tenant_id', $1, false)")
        .bind(tenant.to_string())
        .execute(&mut conn)
        .await
        .expect("set tenant");
    assert_eq!(tenant_of(&mut conn).await.expect("declared tenant"), tenant);
}

#[tokio::test]
async fn the_single_posture_keeps_the_reserved_default() {
    let testdb = testkit::db().await.expect("testkit database");
    let pool = testdb.pool();

    // No stamp at all (a database the server never booted against).
    let mut conn = raw(testdb.url()).await;
    assert_eq!(tenant_of(&mut conn).await.expect("default"), Uuid::nil());

    db::stamp_tenancy_posture(&pool, false)
        .await
        .expect("stamp single");
    let mut conn = raw(testdb.url()).await;
    assert_eq!(tenant_of(&mut conn).await.expect("default"), Uuid::nil());

    // Multi, then back to single: the stamp is re-read, never cached.
    db::stamp_tenancy_posture(&pool, true)
        .await
        .expect("stamp multi");
    db::stamp_tenancy_posture(&pool, false)
        .await
        .expect("stamp single again");
    let mut conn = raw(testdb.url()).await;
    assert_eq!(tenant_of(&mut conn).await.expect("default"), Uuid::nil());
}

#[tokio::test]
async fn the_servers_own_connections_declare_the_default_tenant() {
    let testdb = testkit::db().await.expect("testkit database");
    db::stamp_tenancy_posture(&testdb.pool(), true)
        .await
        .expect("stamp multi");

    // The plain pool: every physical connection declares the default tenant
    // at open, so nothing the server runs unscoped is refused.
    let settings = DbConfig::new(testdb.url().to_owned());
    let plain = db::connect(&settings).await.expect("plain pool");
    let declared: String = sqlx::query_scalar("SELECT current_setting('ferroehr.tenant_id', true)")
        .fetch_one(&plain)
        .await
        .expect("read the GUC");
    assert_eq!(declared, Uuid::nil().to_string());
    let tenant: Uuid = sqlx::query_scalar("SELECT ext.current_tenant_id()")
        .fetch_one(&plain)
        .await
        .expect("the reader answers the declared default");
    assert_eq!(tenant, Uuid::nil());

    // The tenant-scoped pool outside any request scope (a background worker)
    // declares the default explicitly too.
    let scoped = db::connect_tenant_scoped(&settings)
        .await
        .expect("scoped pool");
    let tenant: Uuid = sqlx::query_scalar("SELECT ext.current_tenant_id()")
        .fetch_one(&scoped)
        .await
        .expect("a worker outside a request scope reads the default tenant");
    assert_eq!(tenant, Uuid::nil());

    // The migrator declares it as well: re-applying the sets under multi is
    // idempotent and refused nowhere.
    db::run_migrations(&plain)
        .await
        .expect("migrations re-apply under the multi posture");
}
