//! `EHRbase` server binary.
//!
//! Boots the `ehrbase-rest` ITS-REST server backed by the DB-backed
//! [`EhrbaseService`](ehrbase::service::EhrbaseService): initialises tracing,
//! loads configuration (`figment`), connects the `PostgreSQL` pool, runs
//! migrations, boots the ATNA audit sender, and serves. On shutdown the audit
//! queue is drained before exit.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use clap::{Parser, Subcommand};
use ehrbase_audit::{AuditConfig, AuditHandle, AuditSender, SubjectResolver};
use sqlx::PgPool;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;

/// How long to wait for the audit queue to flush on shutdown.
const AUDIT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// The default endpoint the `healthcheck` subcommand probes: the public,
/// auth-exempt `/rest/status` route the server serves on its default bind.
const DEFAULT_HEALTHCHECK_URL: &str = "http://127.0.0.1:8080/ehrbase/rest/status";

/// `EHRbase` server command-line interface.
#[derive(Debug, Parser)]
#[command(name = "ehrbase", version, about = "openEHR-conformant CDR server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Probe the running server's status endpoint; exit 0 on a 2xx response,
    /// 1 otherwise. Used as the container `HEALTHCHECK` (works in a shell-less
    /// distroless image) and by compose/Kubernetes probes.
    Healthcheck {
        /// The status URL to probe.
        #[arg(long, env = "EHRBASE_HEALTHCHECK_URL", default_value = DEFAULT_HEALTHCHECK_URL)]
        url: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Some(Command::Healthcheck { url }) => healthcheck(&url).await,
        None => serve().await,
    }
}

/// Probe `url` and return `Ok(())` iff the response status is 2xx. An `Err`
/// makes the process exit non-zero (via `anyhow`'s `Termination`), which is
/// exactly the 0/1 contract a Docker `HEALTHCHECK` expects.
async fn healthcheck(url: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("building healthcheck HTTP client")?;
    let status = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("probing {url}"))?
        .status();
    anyhow::ensure!(status.is_success(), "healthcheck: {url} returned {status}");
    Ok(())
}

/// Boot the server: tracing, config, pool, migrations, audit, then serve.
async fn serve() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("info,ehrbase=debug,ehrbase_rest=debug,ehrbase_audit=info")
        }))
        .init();

    let rest_config = ehrbase_rest::RestConfig::load().context("loading REST configuration")?;
    let audit_config = AuditConfig::load().context("loading ATNA audit configuration")?;
    let db_settings = DbSettings::from_env().context("loading database settings")?;

    let pool = db::connect(&db_settings)
        .await
        .context("connecting to PostgreSQL")?;
    db::run_migrations(&pool)
        .await
        .context("applying migrations")?;

    // Boot the ATNA audit sender (fail-open at boot: log and continue without
    // auditing if the transport cannot be established, so the CDR still serves).
    let (audit_sender, audit_handle) = start_audit(&audit_config, &pool).await;

    let service = EhrbaseService::new(pool);

    tracing::info!(
        bind = %rest_config.bind,
        base_path = %rest_config.base_path,
        audit = audit_sender.is_some(),
        "starting ehrbase"
    );
    ehrbase_rest::serve_with_audit(rest_config, Arc::new(service), audit_sender)
        .await
        .context("serving ehrbase-rest")?;

    // The server has stopped and dropped its audit-sender clone; drain the queue.
    if let Some(handle) = audit_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    Ok(())
}

/// Start the audit subsystem from config, wiring the DB-backed subject resolver
/// when `resolve_subject` is enabled. Returns `(None, None)` when auditing is
/// disabled or fails to start (fail-open at boot).
async fn start_audit(
    config: &AuditConfig,
    pool: &PgPool,
) -> (Option<AuditSender>, Option<AuditHandle>) {
    if !config.enabled {
        return (None, None);
    }
    let resolver = config
        .resolve_subject
        .then(|| subject_resolver(pool.clone()));
    match ehrbase_audit::start(config.clone(), resolver).await {
        Ok((sender, handle)) => (Some(sender), Some(handle)),
        Err(e) => {
            tracing::error!("ATNA audit failed to start ({e}); continuing without auditing");
            (None, None)
        }
    }
}

/// A background-only, indexed lookup of `ehr.subject_id` for the Patient-Number
/// participant object. Keeps `ehrbase-audit` free of any DB dependency (the
/// binary owns the pool and hands the sender this closure).
fn subject_resolver(pool: PgPool) -> SubjectResolver {
    Arc::new(move |ehr_id: String| {
        let pool = pool.clone();
        Box::pin(async move {
            let id = Uuid::parse_str(&ehr_id).ok()?;
            // `subject_id` is the promoted EHR_STATUS subject.external_ref id
            // (migrations/ehr/0001_schema.sql), indexed via the PK on `id`.
            sqlx::query_scalar::<_, Option<String>>("SELECT subject_id FROM ehr WHERE id = $1")
                .bind(id)
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten()
                .flatten()
        })
    })
}
