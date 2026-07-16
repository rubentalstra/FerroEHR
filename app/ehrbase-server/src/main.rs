//! `EHRbase` server binary.
//!
//! Boots the `ehrbase-rest` ITS-REST server backed by the DB-backed
//! [`EhrbaseService`](ehrbase::service::EhrbaseService): loads the one
//! configuration tree ([`ehrbase::config`]), initialises tracing, connects the
//! `PostgreSQL` pool, runs migrations, boots the ATNA audit sender, and serves.
//! On shutdown the audit queue is drained before exit.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use clap::{Parser, Subcommand};
use ehrbase::system_log::config::AuditConfig;
use ehrbase::system_log::sender::{AuditHandle, AuditSender, SubjectResolver};
use ehrbase::versioning::signature::signer::Signer;
use ehrbase_rest::config::AppConfig;
use ehrbase::telemetry::build_info::BuildInfo;
use ehrbase::telemetry::health::{HealthIndicator, HealthRegistry};
use ehrbase_rest::extensions::access::authz::AuthzHandle;
use ehrbase_rest::extensions::management::Observability;
use sqlx::PgPool;
use uuid::Uuid;

use ehrbase::db;
use ehrbase::service::EhrbaseService;
use ehrbase::telemetry::{self, indicators};
use ehrbase::telemetry::config::TelemetryConfig;

/// How long to wait for the audit queue to flush on shutdown.
const AUDIT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// The default endpoint the `healthcheck` subcommand probes.
const DEFAULT_HEALTHCHECK_URL: &str = "http://127.0.0.1:8080/ehrbase/rest/status";

/// `EHRbase` server command-line interface.
#[derive(Debug, Parser)]
#[command(name = "ehrbase", version, about = "openEHR-conformant CDR server")]
struct Cli {
    /// Path to the config file (overrides the search order: `EHRBASE_CONFIG`,
    /// `./ehrbase.toml`, `/etc/ehrbase/ehrbase.toml`).
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Repeatable dotted-path override, highest precedence (e.g.
    /// `--set db.max_connections=40`).
    #[arg(long = "set", global = true, value_parser = parse_override)]
    set: Vec<(String, String)>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Probe the running server's status endpoint; exit 0 on 2xx, 1 otherwise.
    Healthcheck {
        /// The status URL to probe.
        #[arg(long, env = "EHRBASE_HEALTHCHECK_URL", default_value = DEFAULT_HEALTHCHECK_URL)]
        url: String,
    },
    /// Configuration utilities (validate / print the annotated default).
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCmd {
    /// Validate the effective configuration (3-pass strict + semantic) and print
    /// it, redacted; exit 0 when valid, 1 otherwise.
    Check,
    /// Emit the annotated default configuration template to stdout.
    Default,
}

/// Parse a `--set key=value` pair.
fn parse_override(raw: &str) -> Result<(String, String), String> {
    raw.split_once('=')
        .map(|(k, v)| (k.trim().to_owned(), v.to_owned()))
        .ok_or_else(|| format!("expected key=value, got `{raw}`"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Healthcheck { url }) => healthcheck(&url).await,
        Some(Command::Config { cmd }) => run_config(&cmd, cli.config.as_deref(), &cli.set),
        None => serve(cli.config.as_deref(), &cli.set).await,
    }
}

/// `ehrbase config check` / `ehrbase config default`.
fn run_config(
    cmd: &ConfigCmd,
    config: Option<&Path>,
    set: &[(String, String)],
) -> anyhow::Result<()> {
    match cmd {
        ConfigCmd::Default => {
            print!("{}", ehrbase::config::DEFAULT_TEMPLATE);
            Ok(())
        }
        ConfigCmd::Check => {
            let cfg = ehrbase::config::load(config, set).map_err(|e| anyhow::anyhow!("{e}"))?;
            cfg.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
            let rendered = cfg.to_redacted_toml().map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("{rendered}");
            if cfg.db.is_dev_default() {
                eprintln!(
                    "note: [db].url is the built-in DEVELOPMENT DEFAULT; set it for any \
                     non-dev deployment."
                );
            }
            eprintln!("configuration OK");
            Ok(())
        }
    }
}

/// Probe `url`; `Ok(())` iff the response status is 2xx.
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

/// Boot the server: config, telemetry, pool, migrations, audit, health, serve.
#[allow(clippy::too_many_lines)] // linear boot sequence; splitting it would obscure order
async fn serve(config_path: Option<&Path>, overrides: &[(String, String)]) -> anyhow::Result<()> {
    // One load + one aggregated validate (all errors at once), then distribute.
    let config =
        ehrbase::config::load(config_path, overrides).map_err(|e| anyhow::anyhow!("{e}"))?;
    config.validate().map_err(|e| anyhow::anyhow!("{e}"))?;

    let telemetry_config = TelemetryConfig {
        log: config.log.clone(),
        otel: config.telemetry.clone(),
    };

    // ASCII banner before telemetry/log init (skipped under `json` logging).
    if telemetry_config.log.format != ehrbase::telemetry::config::LogFormat::Json {
        ehrbase::banner::print();
    }

    let build_info = BuildInfo::current();
    let mut telemetry =
        telemetry::init(&telemetry_config, &build_info).context("initialising telemetry")?;

    // Review condition 1: announce the dev-default DSN prominently (never a
    // silent production trap) — now that logging is up.
    if config.db.is_dev_default() {
        tracing::warn!(
            url = ehrbase::db::DEFAULT_URL,
            "[db].url is the built-in DEVELOPMENT DEFAULT ({}); no file/env/CLI value was \
             supplied. Set db.url (EHRBASE__DB__URL / DATABASE_URL) for any non-dev deployment — \
             production MUST override it.",
            ehrbase::db::DEFAULT_URL,
        );
    }

    let pool = db::connect(&config.db)
        .await
        .context("connecting to PostgreSQL")?;
    db::run_migrations(&pool)
        .await
        .context("applying migrations")?;

    // ATNA audit (fail-open at boot).
    let audit_config: AuditConfig = config.atna.clone();
    let (audit_sender, audit_handle) = start_audit(&audit_config, &pool).await;

    // Contribution-outbox eventing + FHIR outbound emitter (both off by default).
    let outbox_enabled = config.events.enabled || config.fhir.outbound.enabled;
    let events_handle = if config.events.enabled {
        tracing::info!(exchange = %config.events.exchange, "contribution-outbox eventing enabled");
        Some(ehrbase::extensions::events::publisher::start(
            config.events.clone(),
            pool.clone(),
        ))
    } else {
        None
    };

    // Health indicators.
    let mut indicators: Vec<Arc<dyn HealthIndicator>> = vec![
        Arc::new(indicators::DbHealth::new(pool.clone())),
        Arc::new(indicators::MigrationsHealth::new(pool.clone())),
    ];
    if let Some(sender) = &audit_sender {
        indicators.push(Arc::new(indicators::AuditHealth::new(sender.clone())));
    }
    if let Some(handle) = &events_handle {
        indicators.push(Arc::new(indicators::EventsHealth::new(handle.healthy())));
    }
    let health = HealthRegistry::new(indicators);

    telemetry.start_samplers(pool.clone());

    // `/management/env` reports the whole redacted config tree (secrets rendered
    // `***` by construction — P-6), replacing the old ad-hoc snapshot.
    let env_snapshot = Arc::new(serde_json::to_value(&config).unwrap_or(serde_json::Value::Null));
    let observability = Observability {
        management: config.management.clone(),
        prometheus: Some(telemetry.prometheus_handle()),
        log_reload: Some(telemetry.log_reload()),
        health,
        build_info,
        env_snapshot,
    };

    // Version signing (fail-closed at boot for `pgp` without a usable key).
    let signer =
        Arc::new(Signer::from_config(&config.signing).context("initialising the version signer")?);
    tracing::info!(
        signing = signer.enabled(),
        verify_on_read = ?signer.verify_on_read(),
        "version signing configured"
    );

    let audit_enabled = audit_sender.is_some();
    let mut service = EhrbaseService::new(pool.clone())
        .with_signer(signer)
        .with_outbox_enabled(outbox_enabled)
        .with_query_config(&config.query);
    if let Some(sender) = audit_sender {
        service = service.with_audit(sender);
    }

    // Opt-in external FHIR terminology provider.
    match config.terminology.external.default_provider() {
        Some(Ok(provider)) => {
            tracing::info!("external FHIR terminology provider configured");
            service = service.with_external_terminology(Arc::new(provider));
        }
        Some(Err(e)) => return Err(e).context("initialising the external terminology provider"),
        None => {}
    }

    // Opt-in Subject Proxy FHIR-frame executor (fail-closed).
    if let Some(fhir) = config
        .subject_proxy
        .build()
        .context("initialising the subject-proxy FHIR executor")?
    {
        tracing::info!("subject-proxy FHIR-frame executor configured");
        service = service.with_subject_proxy(Arc::new(fhir));
    }

    // Opt-in DV_MULTIMEDIA externalization.
    if let Some(engine) =
        ehrbase::extensions::multimedia::MultimediaEngine::from_config(&config.multimedia)
            .context("initialising the multimedia object store")?
    {
        tracing::info!(
            bucket = %config.multimedia.bucket,
            threshold_bytes = config.multimedia.threshold_bytes,
            "DV_MULTIMEDIA externalization enabled"
        );
        service = service.with_multimedia(Arc::new(engine));
    }

    let service = Arc::new(service);

    // FHIR outbound emitter (off by default; carries PHI).
    let fhir_outbound_handle = if config.fhir.outbound.enabled {
        tracing::info!(
            exchange = %config.fhir.outbound.exchange,
            "FHIR outbound emitter enabled (publishes clinical FHIR resources)"
        );
        Some(ehrbase::extensions::fhir::outbound::start(
            config.fhir.outbound.clone(),
            pool.clone(),
            service.clone(),
        ))
    } else {
        None
    };

    // The RBAC gate — only wired when authentication is enabled.
    let authz = if config.auth.enabled {
        AuthzHandle::from_config(&config.authz, &config.server.base_path).map(Arc::new)
    } else {
        None
    };

    // Assemble the REST adapter's runtime config view from the tree.
    let app_config = AppConfig {
        server: config.server.clone(),
        auth: config.auth.clone(),
        admin: config.admin.clone(),
        tenancy: config.tenancy.clone(),
        smart: config.smart.clone(),
        fhir_api_enabled: config.fhir.api_enabled,
        terminology_api_enabled: config.terminology.api_enabled,
        events_admin_api: config.events.admin_api,
    };

    tracing::info!(
        bind = %app_config.server.bind,
        base_path = %app_config.server.base_path,
        audit = audit_enabled,
        rbac = authz.is_some(),
        management = observability.management.enabled,
        "starting ehrbase"
    );
    ehrbase_rest::serve_full(app_config, service, authz, observability)
        .await
        .context("serving ehrbase-rest")?;

    if let Some(handle) = audit_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    if let Some(handle) = events_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    if let Some(handle) = fhir_outbound_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    telemetry.shutdown().await;
    Ok(())
}

/// Start the audit subsystem from config. `(None, None)` when disabled or on a
/// boot failure (fail-open).
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
    match ehrbase::system_log::sender::start(config.clone(), resolver).await {
        Ok((sender, handle)) => (Some(sender), Some(handle)),
        Err(e) => {
            tracing::error!("ATNA audit failed to start ({e}); continuing without auditing");
            (None, None)
        }
    }
}

/// A background-only indexed lookup of `ehr.subject_id` for the Patient-Number
/// participant object.
fn subject_resolver(pool: PgPool) -> SubjectResolver {
    Arc::new(move |ehr_id: String| {
        let pool = pool.clone();
        Box::pin(async move {
            let id = Uuid::parse_str(&ehr_id).ok()?;
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
