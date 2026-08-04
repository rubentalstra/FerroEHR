//! `FerroEHR` server wiring — the binary's testable library half.
//!
//! Boots the `ferroehr-rest` ITS-REST server backed by the DB-backed
//! [`FerroEhrService`]: loads the one
//! configuration tree ([`ferroehr::config`]), initialises tracing, connects the
//! `PostgreSQL` pool, runs migrations, boots the ATNA audit sender, and serves.
//! On shutdown the audit queue is drained before exit.
//!
//! `main.rs` is a thin shell over [`run`] so the whole boot path is reachable
//! from tests — "you can't test the `main` function directly"
//! (<https://doc.rust-lang.org/book/ch12-03-improving-error-handling-and-modularity.html>).
//! `anyhow` is fine here: this lib target IS the binary's own logic half,
//! not a consumable library.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use clap::{Parser, Subcommand};
use ferroehr::system_log::config::AuditConfig;
use ferroehr::system_log::sender::{AuditHandle, AuditSender, SubjectResolver};
use ferroehr::telemetry::build_info::BuildInfo;
use ferroehr::telemetry::health::{HealthIndicator, HealthRegistry};
use ferroehr::versioning::signature::signer::Signer;
use ferroehr_rest::config::AppConfig;
use ferroehr_rest::extensions::access::authz::{
    AuthzHandle, AuthzResolvers, ResolveError, build_engine,
};
use ferroehr_rest::extensions::management::Observability;
use sqlx::PgPool;
use uuid::Uuid;

use ferroehr::db;
use ferroehr::service::FerroEhrService;
use ferroehr::telemetry::config::{LogFormat, TelemetryConfig};
use ferroehr::telemetry::{self, indicators};

/// How long to wait for the audit queue to flush on shutdown.
const AUDIT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// The default endpoint the `healthcheck` subcommand probes.
const DEFAULT_HEALTHCHECK_URL: &str = "http://127.0.0.1:8080/ferroehr/rest/status";

/// `FerroEHR` server command-line interface.
#[derive(Debug, Parser)]
#[command(name = "ferroehr", version, about = "openEHR-conformant CDR server")]
pub struct Cli {
    /// Path to the config file (overrides the search order: `FERROEHR_CONFIG`,
    /// `./ferroehr.toml`, `/etc/ferroehr/ferroehr.toml`).
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Repeatable dotted-path override, highest precedence (e.g.
    /// `--set db.max_connections=40`).
    #[arg(long = "set", global = true, value_parser = parse_override)]
    set: Vec<(String, String)>,
    #[command(subcommand)]
    command: Option<Command>,
}

/// Top-level subcommands of the `ferroehr` binary.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Probe the running server's status endpoint; exit 0 on 2xx, 1 otherwise.
    Healthcheck {
        /// The status URL to probe.
        #[arg(long, env = "FERROEHR_HEALTHCHECK_URL", default_value = DEFAULT_HEALTHCHECK_URL)]
        url: String,
    },
    /// Configuration utilities (validate / print the annotated default).
    Config {
        /// Which configuration utility to run.
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}

/// `ferroehr config …` subcommands.
#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
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

/// Dispatch a parsed [`Cli`] — the whole binary behind one testable seam.
///
/// # Errors
///
/// Returns any boot, configuration, probe, or serve failure of the selected
/// subcommand; the process exit code follows from `main` returning it.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Some(Command::Healthcheck { url }) => healthcheck(&url).await,
        Some(Command::Config { cmd }) => run_config(&cmd, cli.config.as_deref(), &cli.set),
        None => serve(cli.config.as_deref(), &cli.set).await,
    }
}

/// `ferroehr config check` / `ferroehr config default`.
#[expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the config default|check subcommands' PURPOSE is console output \
              (.claude/rules/reliability.md §tools)"
)]
fn run_config(
    cmd: &ConfigCmd,
    config: Option<&Path>,
    set: &[(String, String)],
) -> anyhow::Result<()> {
    match cmd {
        ConfigCmd::Default => {
            print!("{}", ferroehr::config::DEFAULT_TEMPLATE);
            Ok(())
        }
        ConfigCmd::Check => {
            let cfg = ferroehr::config::load(config, set).map_err(|e| anyhow::anyhow!("{e}"))?;
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
#[expect(
    clippy::too_many_lines,
    reason = "linear boot sequence; splitting it would obscure order"
)]
async fn serve(config_path: Option<&Path>, overrides: &[(String, String)]) -> anyhow::Result<()> {
    // One load + one aggregated validate (all errors at once), then distribute.
    let config =
        ferroehr::config::load(config_path, overrides).map_err(|e| anyhow::anyhow!("{e}"))?;
    config.validate().map_err(|e| anyhow::anyhow!("{e}"))?;

    let telemetry_config = TelemetryConfig {
        log: config.log.clone(),
        otel: config.telemetry.clone(),
    };

    // ASCII banner before telemetry/log init (skipped under `json` logging).
    if telemetry_config.log.format != LogFormat::Json {
        ferroehr::banner::print();
    }

    let build_info = BuildInfo::current();
    let mut telemetry =
        telemetry::init(&telemetry_config, &build_info).context("initialising telemetry")?;

    // Review condition 1: announce the dev-default DSN prominently (never a
    // silent production trap) — now that logging is up.
    if config.db.is_dev_default() {
        tracing::warn!(
            url = db::DEFAULT_URL,
            "[db].url is the built-in DEVELOPMENT DEFAULT ({}); no file/env/CLI value was \
             supplied. Set db.url (FERROEHR__DB__URL / DATABASE_URL) for any non-dev deployment — \
             production MUST override it.",
            db::DEFAULT_URL,
        );
    }

    // Multi-tenant mode swaps in the tenant-scoped pool: every checked-out
    // connection is stamped with the request's `ferroehr.tenant_id` session
    // GUC, which the RLS `tenant_isolation` policy reads (no openEHR spec
    // governs multi-tenancy — our own deployment extension). Single-tenant
    // deployments keep the plain pool and pay no per-acquire cost.
    let pool = if config.tenancy.enabled {
        db::connect_tenant_scoped(&config.db)
            .await
            .context("connecting to PostgreSQL (tenant-scoped)")?
    } else {
        db::connect(&config.db)
            .await
            .context("connecting to PostgreSQL")?
    };
    db::run_migrations(&pool)
        .await
        .context("applying migrations")?;

    // ATNA audit (fail-open at boot).
    let audit_config: AuditConfig = config.audit.clone();
    let (audit_sender, audit_handle) = start_audit(&audit_config, &pool).await;

    // Contribution-outbox eventing + FHIR outbound emitter (both off by default).
    let outbox_enabled = config.events.enabled || config.fhir.outbound.enabled;
    let events_handle = if config.events.enabled {
        tracing::info!(exchange = %config.events.exchange, "contribution-outbox eventing enabled");
        Some(ferroehr::extensions::events::publisher::start(
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

    // The data-authoring identity every commit stamps into `EHR.system_id`,
    // `AUDIT_DETAILS.system_id`, and `OBJECT_VERSION_ID.creating_system_id`
    // (`[server] system_id`) — logged so an operator can see the key took.
    tracing::info!(system_id = %config.server.system_id, "openEHR system identifier");

    let audit_enabled = audit_sender.is_some();
    let mut service = FerroEhrService::new(pool.clone())
        .with_system_id(config.server.system_id.clone())
        .with_signer(signer)
        .with_outbox_enabled(outbox_enabled)
        .with_query_config(&config.query);
    if let Some(sender) = audit_sender {
        service = service.with_audit(sender);
    }
    // The local Audit Record Repository read side (the ITI-81 retrieval):
    // wired whenever auditing + the store are on.
    if audit_enabled && config.audit.store.enabled {
        service =
            service.with_audit_store(ferroehr::system_log::store::AuditStore::new(pool.clone()));
    }

    // Opt-in external FHIR terminology servers — ALL configured providers,
    // with the terminology→provider routing (a deployment binds several
    // terminologies at once; BASE `architecture_overview/
    // master12-terminology.adoc` §Overview).
    if let Some(router) = ferroehr::service::terminology::router::TerminologyRouter::build(
        &config.terminology.external,
    )
    .context("initialising the external terminology providers")?
    {
        tracing::info!(
            providers = %router.provider_names().collect::<Vec<_>>().join(", "),
            fail_on_error = router.fail_on_error(),
            "external FHIR terminology providers configured"
        );
        service = service.with_terminology_router(Arc::new(router));
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
        ferroehr::extensions::multimedia::MultimediaEngine::from_config(&config.multimedia)
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
        Some(ferroehr::extensions::fhir::outbound::start(
            config.fhir.outbound.clone(),
            pool.clone(),
            Arc::clone(&service),
        ))
    } else {
        None
    };

    // Authorization — only wired when authentication is enabled: the RBAC gate
    // plus the ABAC engine + DB-backed attribute resolvers. A misconfigured
    // ABAC block (enabled but unbuildable) aborts BOOT — configuration that
    // promises fine-grained authorization must never degrade to authz-off.
    let authz = if config.auth.enabled {
        build_authz(
            &config.authz,
            &config.server.base_path,
            authz_resolvers(pool.clone(), Arc::clone(&service)),
        )?
    } else {
        None
    };
    if let Some(handle) = &authz {
        tracing::info!(
            rbac = handle.rbac_active(),
            abac = handle.abac_active(),
            "authorization enabled"
        );
    }

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
        "starting ferroehr"
    );
    ferroehr_rest::serve_full(app_config, service, authz, observability)
        .await
        .context("serving ferroehr-rest")?;

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
    match ferroehr::system_log::sender::start(config.clone(), resolver, Some(pool.clone())).await {
        Ok((sender, handle)) => (Some(sender), Some(handle)),
        Err(e) => {
            tracing::error!("ATNA audit failed to start ({e}); continuing without auditing");
            (None, None)
        }
    }
}

/// Build the full authorization handle the binary serves with: the RBAC gate
/// (when `rbac.enabled`) plus the ABAC gate over the boot-built policy engine
/// (when `abac.enabled`).
///
/// `None` when neither layer is active (auth-only behaviour). Fine-grained
/// authorization is our own extension — no openEHR spec governs it (ITS-REST
/// places authorization out of band).
///
/// # Errors
/// An ABAC block that is enabled but unbuildable (missing/invalid Cedar
/// policies, an unbuildable remote-PDP client) — startup must abort rather
/// than silently run without the promised gate.
pub fn build_authz(
    config: &ferroehr::config::authz::AuthzConfig,
    base_path: &str,
    resolvers: AuthzResolvers,
) -> anyhow::Result<Option<Arc<AuthzHandle>>> {
    let engine = build_engine(&config.abac).context("building the ABAC policy engine")?;
    Ok(AuthzHandle::build(config, base_path, engine, resolvers).map(Arc::new))
}

/// The DB-backed ABAC attribute resolvers.
///
/// Builds `ferroehr_rest::extensions::access::authz::AuthzResolvers`: the EHR
/// subject external-ref id (the promoted `ehr.subject_id` column — the same
/// query the audit [`SubjectResolver`] runs) and the committed template of a
/// COMPOSITION version (`vo_version.template_id` via the service read-back).
/// Failures are typed [`ResolveError`]s — the PEP fails closed on them, never
/// silently permits.
#[must_use]
pub fn authz_resolvers(pool: PgPool, service: Arc<FerroEhrService>) -> AuthzResolvers {
    AuthzResolvers {
        subject: Arc::new(move |ehr_id: String| {
            let pool = pool.clone();
            Box::pin(async move {
                let id = Uuid::parse_str(&ehr_id)
                    .map_err(|e| ResolveError(format!("ehr id {ehr_id}: {e}")))?;
                sqlx::query_scalar::<_, Option<String>>("SELECT subject_id FROM ehr WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&pool)
                    .await
                    .map(Option::flatten)
                    .map_err(|e| ResolveError(format!("ehr subject lookup: {e}")))
            })
        }),
        template_of_version: Arc::new(move |vo: String, version: Option<String>| {
            let service = Arc::clone(&service);
            Box::pin(async move {
                let vo_id = vo
                    .parse::<ferroehr::ids::VoId>()
                    .map_err(|e| ResolveError(format!("vo id {vo}: {e}")))?;
                service
                    .template_of_version(vo_id, version.as_deref())
                    .await
                    .map_err(|e| ResolveError(format!("template lookup: {e}")))
            })
        }),
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
