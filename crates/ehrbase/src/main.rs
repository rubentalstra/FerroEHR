//! `EHRbase` server binary.
//!
//! Boots the `ehrbase-rest` ITS-REST server backed by the DB-backed
//! [`EhrbaseService`](ehrbase::service::EhrbaseService): initialises tracing,
//! loads configuration (`figment`), connects the `PostgreSQL` pool, runs
//! migrations, and serves.

use std::sync::Arc;

use anyhow::Context as _;
use tracing_subscriber::EnvFilter;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,ehrbase=debug,ehrbase_rest=debug")),
        )
        .init();

    let rest_config = ehrbase_rest::RestConfig::load().context("loading REST configuration")?;
    let db_settings = DbSettings::from_env().context("loading database settings")?;

    let pool = db::connect(&db_settings)
        .await
        .context("connecting to PostgreSQL")?;
    db::run_migrations(&pool)
        .await
        .context("applying migrations")?;
    let service = EhrbaseService::new(pool);

    tracing::info!(bind = %rest_config.bind, base_path = %rest_config.base_path, "starting ehrbase");
    ehrbase_rest::serve_with(rest_config, Arc::new(service))
        .await
        .context("serving ehrbase-rest")?;
    Ok(())
}
