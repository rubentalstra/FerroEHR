//! Shared test fixture: the REST router over a **real** `EhrbaseService` on a
//! **real PostgreSQL 18** (testcontainers).
//!
//! The former `Mock`/`Hooks` scripted backend died with the trait seam (W-14
//! B+C): these HTTP tests now exercise the same concrete service the binary
//! ships, so every scenario is set up through the real API (upload a template,
//! create an EHR, …) and every assertion observes real behaviour.
//!
//! One PostgreSQL container serves the whole test binary; each test takes its
//! own database (`test_service("my_test")`), so tests stay independent and
//! parallel.

#![allow(dead_code)]

use std::sync::Arc;

use sqlx::{Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;

use ehrbase::db;
use ehrbase::db::settings::DbConfig;
use ehrbase::service::EhrbaseService;
use ehrbase_rest::access::authn::Authenticator;
use ehrbase_rest::{AppConfig, AppState};

/// The shared PostgreSQL 18 container (one per test binary).
struct Pg {
    _container: ContainerAsync<Postgres>,
    host: String,
    port: u16,
}

static PG: OnceCell<Pg> = OnceCell::const_new();

async fn pg() -> &'static Pg {
    PG.get_or_init(|| async {
        let container = Postgres::default()
            .with_tag("18")
            .start()
            .await
            .expect("start postgres:18 (is Docker running?)");
        let host = container.get_host().await.expect("host").to_string();
        let port = container.get_host_port_ipv4(5432).await.expect("port");
        Pg {
            _container: container,
            host,
            port,
        }
    })
    .await
}

/// A fresh, fully-migrated database named `name` on the shared container.
pub async fn migrated_pool(name: &str) -> PgPool {
    let pg = pg().await;
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
    pool
}

/// The real platform service over a fresh database.
///
/// `name` must be a unique, SQL-identifier-safe string per test (it becomes
/// the database name).
pub async fn test_service(name: &str) -> Arc<EhrbaseService> {
    Arc::new(EhrbaseService::new(migrated_pool(name).await))
}

/// The assembled router over a real service with the given configuration —
/// the same wiring as [`ehrbase_rest::build_with`], split open so tests can
/// hand-tune `AppConfig` (auth modes, admin/extension toggles).
pub fn router_with(config: AppConfig, service: Arc<EhrbaseService>) -> axum::Router {
    let authenticator = Authenticator::new(config.auth.clone()).expect("test auth config is valid");
    let state = AppState::with_backend(config, service);
    ehrbase_rest::router(state, authenticator)
}

/// The assembled router over a real service with authentication disabled —
/// the baseline most HTTP tests want (auth-specific tests build their own
/// [`AppConfig`] via [`router_with`]).
pub async fn test_router(name: &str) -> axum::Router {
    let mut config = AppConfig::default();
    config.auth.enabled = false;
    router_with(config, test_service(name).await)
}
