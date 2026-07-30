//! `EHRbase` server binary — a thin shell over [`ehrbase_server::run`].
//!
//! All wiring logic lives in the crate's library half so it is reachable from
//! integration tests (the Book ch12.3 split: `main` only parses and delegates).

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ehrbase_server::run(ehrbase_server::Cli::parse()).await
}
