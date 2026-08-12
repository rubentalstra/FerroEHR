// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Shared test fixture: the REST router over a **real** `FerroEhrService` on a
//! **real `PostgreSQL` 18** from the shared `testkit` harness (one server,
//! one migrated template database, one `CREATE DATABASE … TEMPLATE` clone
//! per call — see `tools/testkit`).
//!
//! These HTTP tests exercise the same concrete service the binary ships, so
//! every scenario is set up through the real API (upload a template, create
//! an EHR, …) and every assertion observes real behaviour. Hold the returned
//! [`testkit::TestDb`] guard for the test's lifetime — dropping it releases
//! the clone.

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use std::sync::Arc;

use ferroehr::service::FerroEhrService;
use ferroehr_rest::config::AppConfig;
use ferroehr_rest::extensions::access::authn::Authenticator;
use ferroehr_rest::state::AppState;
use sqlx::PgPool;

/// A fresh, fully migrated database from the shared harness.
pub(crate) async fn test_db() -> testkit::TestDb {
    testkit::db().await.expect("testkit database")
}

/// A fresh, fully-migrated database and its pool. Hold the guard with the
/// pool.
pub(crate) async fn migrated_pool() -> (testkit::TestDb, PgPool) {
    let db = test_db().await;
    let pool = db.pool();
    (db, pool)
}

/// The real platform service over a fresh database.
pub(crate) async fn test_service() -> (testkit::TestDb, Arc<FerroEhrService>) {
    let (db, pool) = migrated_pool().await;
    (db, Arc::new(FerroEhrService::new(pool)))
}

/// The assembled router over a real service with the given configuration —
/// the same wiring as [`ferroehr_rest::build_with`], split open so tests can
/// hand-tune `AppConfig` (auth modes, admin/extension toggles).
pub(crate) fn router_with(config: AppConfig, service: Arc<FerroEhrService>) -> axum::Router {
    let authenticator = Authenticator::new(config.auth.clone()).expect("test auth config is valid");
    let state = AppState::with_backend(config, service);
    ferroehr_rest::router::router(state, authenticator)
}

/// The assembled router over a real service with authentication disabled —
/// the baseline most HTTP tests want (auth-specific tests build their own
/// [`AppConfig`] via [`router_with`]).
pub(crate) async fn test_router() -> (testkit::TestDb, axum::Router) {
    let mut config = AppConfig::default();
    config.auth.enabled = false;
    let (db, service) = test_service().await;
    (db, router_with(config, service))
}
