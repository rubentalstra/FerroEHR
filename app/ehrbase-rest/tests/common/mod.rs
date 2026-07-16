//! Shared test fixture: the REST router over a **real** `EhrbaseService` on a
//! **real `PostgreSQL` 18** (testcontainers).
//!
//! The former `Mock`/`Hooks` scripted backend died with the trait seam (W-14
//! B+C): these HTTP tests now exercise the same concrete service the binary
//! ships, so every scenario is set up through the real API (upload a template,
//! create an EHR, …) and every assertion observes real behaviour.
//!
//! Each test starts its own `PostgreSQL` container and database
//! (`test_service("my_test")`), dropped with the test — independent,
//! parallel, and reaped (a process-static container is never dropped and
//! leaks; nextest runs one process per test).

#![allow(dead_code)]

use std::sync::Arc;

use ehrbase::db;
use ehrbase::db::DbConfig;
use ehrbase::service::EhrbaseService;
use ehrbase_rest::config::AppConfig;
use ehrbase_rest::extensions::access::authn::Authenticator;
use ehrbase_rest::state::AppState;
use sqlx::{Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

/// A per-test `PostgreSQL` 18 container. Keep it alive for the pool's
/// lifetime — dropping it stops the container (the reaping).
pub struct Pg {
    _container: ContainerAsync<Postgres>,
    host: String,
    port: u16,
}

impl Pg {
    pub async fn start() -> Self {
        let container = Postgres::default()
            .with_tag("18")
            .start()
            .await
            .expect("start postgres:18 (is Docker running?)");
        let host = container.get_host().await.expect("host").to_string();
        let port = container.get_host_port_ipv4(5432).await.expect("port");
        Self {
            _container: container,
            host,
            port,
        }
    }
}

/// A fresh, fully-migrated database named `name` on a fresh container.
/// Returns the container guard with the pool — hold both.
pub async fn migrated_pool(name: &str) -> (Pg, PgPool) {
    let pg = Pg::start().await;
    let admin = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        pg.host, pg.port
    );
    let mut conn = PgConnection::connect(&admin).await.expect("admin connect");
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&mut conn)
        .await
        .expect("create db");
    let settings = DbConfig::new(format!(
        "postgres://postgres:postgres@{}:{}/{name}",
        pg.host, pg.port
    ));
    let pool = db::connect(&settings).await.expect("pool");
    db::run_migrations(&pool).await.expect("migrate");
    (pg, pool)
}

/// The real platform service over a fresh database.
///
/// `name` must be a unique, SQL-identifier-safe string per test (it becomes
/// the database name).
pub async fn test_service(name: &str) -> (Pg, Arc<EhrbaseService>) {
    let (pg, pool) = migrated_pool(name).await;
    (pg, Arc::new(EhrbaseService::new(pool)))
}

/// The assembled router over a real service with the given configuration —
/// the same wiring as [`ehrbase_rest::build_with`], split open so tests can
/// hand-tune `AppConfig` (auth modes, admin/extension toggles).
pub fn router_with(config: AppConfig, service: Arc<EhrbaseService>) -> axum::Router {
    let authenticator = Authenticator::new(config.auth.clone()).expect("test auth config is valid");
    let state = AppState::with_backend(config, service);
    ehrbase_rest::router::router(state, authenticator)
}

/// The assembled router over a real service with authentication disabled —
/// the baseline most HTTP tests want (auth-specific tests build their own
/// [`AppConfig`] via [`router_with`]).
pub async fn test_router(name: &str) -> (Pg, axum::Router) {
    let mut config = AppConfig::default();
    config.auth.enabled = false;
    let (pg, service) = test_service(name).await;
    (pg, router_with(config, service))
}
