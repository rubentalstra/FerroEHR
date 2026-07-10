//! `PostgreSQL` persistence foundation (P09/P10).
//!
//! An `sqlx` connection pool over the **greenfield PG18-native schema**
//! (ADR-008) — the `ext` helper functions and the `ehr` schema (the unified
//! `node` table + temporal `vo_version` + supporting tables) under
//! `migrations/{ext,ehr}/`, applied via [`sqlx::migrate!`] — plus the
//! `sea-query` identifier definitions ([`iden`]) used by the AQL SQL generator.

mod error;
mod migrate;
mod pool;
mod settings;

pub mod iden;

pub use error::DbError;
pub use migrate::run_migrations;
pub use pool::{connect, connect_tenant_scoped};
pub use settings::DbSettings;
