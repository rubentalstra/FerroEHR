//! `PostgreSQL` persistence foundation (P09).
//!
//! An `sqlx` connection pool over the real `EHRbase` v2 schema — the vendored
//! Flyway SQL under `migrations/{ext,ehr}/`, renamed to sqlx's
//! `<version>_<description>.sql` scheme and applied via [`sqlx::migrate!`]
//! (see ADR-007) — plus the `sea-query` identifier definitions ([`iden`])
//! that replace jOOQ's generated table metadata.

mod error;
mod migrate;
mod pool;
mod settings;

pub mod iden;

pub use error::DbError;
pub use migrate::run_migrations;
pub use pool::connect;
pub use settings::DbSettings;
