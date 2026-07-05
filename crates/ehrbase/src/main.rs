//! `EHRbase` server binary.
//!
//! Boots the `ehrbase-rest` ITS-REST server: initialises tracing, loads the
//! configuration (`figment`), and serves. The storage/service/AQL layers this
//! crate provides are wired into the request handlers as later phases land
//! (P12+); until then the REST surface answers with typed `NotImplemented`.

use anyhow::Context as _;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,ehrbase=debug,ehrbase_rest=debug")),
        )
        .init();

    let config = ehrbase_rest::RestConfig::load().context("loading REST configuration")?;
    tracing::info!(bind = %config.bind, base_path = %config.base_path, "starting ehrbase");

    ehrbase_rest::serve(config)
        .await
        .context("serving ehrbase-rest")?;
    Ok(())
}
