// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

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

use std::io::IsTerminal as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use clap::{Parser, Subcommand};
use ferroehr::config::deployment::{ClusterIdentities, DeploymentPosture, DeploymentProfile};
use ferroehr::config::management::EndpointLevels;
use ferroehr::config::management::ManagementConfig;
use ferroehr::system_log::config::AuditConfig;
use ferroehr::system_log::config::AuditPosture;
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
use ferroehr::telemetry::config::{LogFormat, ResolvedLogFormat, TelemetryConfig};
use ferroehr::telemetry::{self, indicators};

/// How long to wait for the audit queue to flush on shutdown.
const AUDIT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

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
        /// The status URL to probe. Unset, it is derived from the effective
        /// configuration: `http://127.0.0.1:<server.bind port><REST root>/status`,
        /// so a shortened `server.base_path` moves the probe with it.
        #[arg(long, env = "FERROEHR_HEALTHCHECK_URL")]
        url: Option<String>,
    },
    /// Configuration utilities (validate / print the annotated default).
    Config {
        /// Which configuration utility to run.
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// Database schema utilities (apply / verify the migrations, then exit).
    Db {
        /// Which schema utility to run.
        #[command(subcommand)]
        cmd: DbCmd,
    },
}

/// `ferroehr db …` subcommands.
///
/// These exist so a least-privilege deployment can separate the two database
/// identities: `migrate` runs under the migrator DSN as a one-shot step
/// (a Kubernetes Job, an init container, a CI/CD stage), and the server then
/// boots with `[db].migrate = "verify"` under a DSN with no DDL rights at all.
#[derive(Debug, Subcommand)]
pub enum DbCmd {
    /// Apply the embedded migrations and exit; the DSN must hold DDL rights.
    Migrate,
    /// Verify, without issuing any DDL, that the database carries exactly this
    /// build's migrations; exit 0 when it does, 1 otherwise.
    Verify,
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
        Some(Command::Healthcheck { url }) => {
            let url = if let Some(url) = url {
                url
            } else {
                let config = ferroehr::config::load(cli.config.as_deref(), &cli.set)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                healthcheck_url(&config)?
            };
            healthcheck(&url).await
        }
        Some(Command::Config { cmd }) => run_config(&cmd, cli.config.as_deref(), &cli.set),
        Some(Command::Db { cmd }) => run_db(&cmd, cli.config.as_deref(), &cli.set).await,
        None => serve(cli.config.as_deref(), &cli.set).await,
    }
}

/// `ferroehr db migrate` / `ferroehr db verify` — the out-of-band schema step.
async fn run_db(
    cmd: &DbCmd,
    config_path: Option<&Path>,
    overrides: &[(String, String)],
) -> anyhow::Result<()> {
    let config =
        ferroehr::config::load(config_path, overrides).map_err(|e| anyhow::anyhow!("{e}"))?;
    config.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
    let telemetry_config = TelemetryConfig {
        log: config.log.clone(),
        otel: config.telemetry.clone(),
    };
    let build_info = BuildInfo::for_profile(config.spec_profile);
    let telemetry =
        telemetry::init(&telemetry_config, &build_info).context("initialising telemetry")?;

    // Both subcommands read the schema on the migration DSN
    // (`[db].migrate_url`, falling back to `[db].url`), which is the
    // credential that can reach every schema when the runtime ones each hold
    // one pseudonymisation domain.
    let outcome = match cmd {
        DbCmd::Migrate => db::apply_schema(&config.db)
            .await
            .context("applying migrations"),
        DbCmd::Verify => verify_schema_and_isolation(&config.db).await,
    };
    telemetry.shutdown().await;
    outcome
}

/// `ferroehr db verify`: the recorded schema state, then the pseudonymisation
/// boundary.
///
/// The two checks deliberately authenticate as different credentials, exactly
/// as [`ferroehr::db::prepare`] does at boot. The schema state is read on the
/// migration DSN, which spans all five migration sets; the isolation check
/// runs on the RUNTIME pool, because what it measures is what the serving
/// credential can reach — asked of the migration credential it would report
/// on a role that holds every domain by design.
///
/// # Errors
/// A schema divergence, an unreadable migration set, a breached domain
/// boundary, or a connection failure.
async fn verify_schema_and_isolation(settings: &db::DbConfig) -> anyhow::Result<()> {
    db::verify_schema(settings)
        .await
        .map_err(|error| anyhow::Error::new(error).context("verifying the schema"))?;
    let pool = db::connect(settings)
        .await
        .context("connecting to PostgreSQL")?;
    let outcome = db::verify_domain_isolation(&pool)
        .await
        .context("verifying the pseudonymisation domain isolation");
    pool.close().await;
    outcome
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
            cfg.auth
                .require_mechanism()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
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

/// The default healthcheck URL, derived from the effective configuration.
///
/// The probe runs inside the container beside the server, so the caller hands
/// it the configuration the server booted with: the port of `server.bind` and
/// the REST root `server.base_path` derives. Loopback is fixed; the bind
/// address is whatever the listener accepts from, which is not necessarily
/// dialable.
///
/// # Errors
/// `server.bind` carries no port.
pub fn healthcheck_url(config: &ferroehr::config::FerroEhrConfig) -> anyhow::Result<String> {
    let port = config
        .server
        .bind
        .rsplit_once(':')
        .map(|(_, port)| port)
        .with_context(|| format!("server.bind `{}` carries no port", config.server.bind))?;
    Ok(format!(
        "http://127.0.0.1:{port}{}/status",
        config.server.rest_root()
    ))
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

/// Whether the ASCII boot banner prints, given the configured `[log] format`
/// and whether stdout is a terminal.
///
/// Keyed on the RESOLVED rendering rather than the configured value: `format =
/// "auto"` renders JSON whenever stdout is not a terminal, and a banner ahead of
/// it would leave a log collector with unparseable first bytes. No openEHR spec
/// governs logging — our own design.
fn prints_banner(format: LogFormat, stdout_is_terminal: bool) -> bool {
    format.resolve(stdout_is_terminal) == ResolvedLogFormat::Pretty
}

/// Whether a bind address is loopback-only, so a plaintext listener there is
/// not reachable off the host.
///
/// A host part that does not parse as an IP address (a DNS name, or the empty
/// host of `:8080`) is treated as routable: assuming otherwise would suppress
/// the warning in exactly the ambiguous case that deserves it.
fn binds_loopback(bind: &str) -> bool {
    bind.parse::<std::net::SocketAddr>()
        .is_ok_and(|address| address.ip().is_loopback())
}

/// Summarizes which management endpoints are mounted, and at which access
/// level, as `info=admin_only, prometheus=public`.
///
/// Worth a boot line of its own: every endpoint defaults to `off`, so what an
/// operator needs to see is exactly which ones a configuration turned on and how
/// exposed each one is — `env` renders the effective configuration and
/// `flamegraph` starts a profiler on request, so neither should ever be a
/// surprise. `none` when the surface is mounted with nothing enabled.
fn mounted_management_endpoints(levels: EndpointLevels) -> String {
    let described = [
        ("info", levels.info),
        ("metrics", levels.metrics),
        ("prometheus", levels.prometheus),
        ("env", levels.env),
        ("loggers", levels.loggers),
        ("flamegraph", levels.flamegraph),
    ];
    let mounted: Vec<String> = described
        .iter()
        .filter(|(_, level)| level.is_mounted())
        .map(|(name, level)| format!("{name}={}", level.as_str()))
        .collect();
    if mounted.is_empty() {
        "none".to_owned()
    } else {
        mounted.join(", ")
    }
}

/// Announces the deployment postures that are legal but easy to leave on by
/// accident, once logging is up.
///
/// The dev-default DSN and permissive CORS are announced rather than silently
/// accepted (OWASP REST Security Cheat Sheet). The HTTPS posture is stated
/// rather than enforced: this server cannot tell plaintext-because-misconfigured
/// from plaintext-behind-a-TLS-terminating-ingress, so authentication over
/// plaintext on a routable bind warns loudly and proceeds.
/// Assembles the platform service from the resolved config tree.
///
/// Every optional collaborator is attached here: the audit sender, the local
/// Audit Record Repository read side (wired only when auditing and the store are
/// both on), terminology, the subject proxy, and the multimedia store.
///
/// # Errors
/// Any collaborator whose configuration is enabled but unbuildable — a slim
/// build missing its cargo feature, or an unusable external dependency.
fn assemble_service(
    config: &ferroehr::config::FerroEhrConfig,
    pools: &Pools,
    audit_sender: Option<AuditSender>,
    outbox_enabled: bool,
    signer: Arc<Signer>,
    deployment: DeploymentPosture,
) -> anyhow::Result<FerroEhrService> {
    let pool = &pools.clinical;
    let audit_enabled = audit_sender.is_some();
    // The MRN patterns compiled here; `FerroEhrConfig::validate` already
    // refused an uncompilable one at boot, so this cannot be the first place a
    // bad pattern is noticed.
    let privacy = ferroehr::privacy::PrivacyPolicy::compile(&config.privacy)
        .context("compiling the [privacy] policy")?;
    // The identifier-protection engine, when the deployment configured one. A
    // key that cannot be parsed is a boot error: a server that believes it
    // seals national identifiers and does not is the failure this refuses to
    // start into.
    let identifiers =
        ferroehr::service::demographic::identifier::engine::IdentifierProtection::from_config(
            &config.demographic.identifier_protection,
            config.demographic.identifier_protection.key.as_ref(),
            pools.demographic.clone(),
        )
        .context("building the [demographic.identifier_protection] engine")?;

    // The licence in force: the configured token, else the one the build
    // embeds. Never a boot failure; the outcome is logged once, served on
    // GET /rest/status, and selects the identifier stamp key.
    let anchors = ferroehr::licence::anchors().context("parsing the embedded licence anchors")?;
    let licence = ferroehr::licence::state::LicenceState::load_now(
        &config.licence,
        ferroehr::licence::EMBEDDED_TOKEN,
        &anchors,
    );
    tracing::info!(licence = %licence, "licence");

    let mut service = FerroEhrService::new(pool.clone())
        .with_demographic_pool(pools.demographic.clone())
        .with_linkage_pool(pools.linkage.clone())
        .with_spec_profile(config.spec_profile)
        .with_system_id(config.server.system_id.clone())
        .with_signer(signer)
        .with_licence(licence)
        .with_deployment(deployment)
        .with_outbox_enabled(outbox_enabled)
        .with_privacy(Arc::new(privacy))
        .with_identifier_protection_opt(identifiers.map(Arc::new))
        .with_query_config(&config.query);
    if let Some(sender) = audit_sender {
        service = service.with_audit(sender);
    }
    if audit_enabled && config.audit.store.enabled {
        service =
            service.with_audit_store(ferroehr::system_log::store::AuditStore::new(pool.clone()));
    }

    service = attach_terminology(service, config)?;
    service = attach_subject_proxy(service, config)?;
    attach_multimedia(service, config)
}

/// Wires authorization, active only when authentication is enabled: the RBAC
/// gate plus the ABAC engine over the DB-backed attribute resolvers.
///
/// `None` when authentication is off, or when neither layer is configured.
///
/// # Errors
/// An ABAC block that is enabled but unbuildable aborts boot: configuration
/// promising fine-grained authorization must never degrade to authz-off.
fn wire_authz(
    config: &ferroehr::config::FerroEhrConfig,
    pool: &PgPool,
    service: &Arc<FerroEhrService>,
) -> anyhow::Result<Option<Arc<AuthzHandle>>> {
    if !config.auth.enabled {
        return Ok(None);
    }
    let authz = build_authz(
        &config.authz,
        &config.server.base_path,
        authz_resolvers(pool.clone(), Arc::clone(service)),
    )?;
    if let Some(handle) = &authz {
        tracing::info!(
            rbac = handle.rbac_active(),
            abac = handle.abac_active(),
            "authorization enabled"
        );
    }
    Ok(authz)
}

/// Logs the resolved runtime posture, one line per subsystem, immediately
/// before the listener starts.
///
/// Each line carries that subsystem's own facts, so the listener line stays
/// about the listener: authorization already logged its own line when it was
/// built, and auditing and the management surface get theirs here.
fn log_resolved_posture(
    app_config: &AppConfig,
    audit_enabled: bool,
    audit: &AuditConfig,
    management: &ManagementConfig,
) {
    tracing::info!(
        mechanisms = %app_config.auth.advertised_mechanisms(),
        enabled = app_config.auth.enabled,
        "authentication configured"
    );
    if audit_enabled {
        tracing::info!(
            local_repository = audit.store.enabled,
            syslog = audit.syslog.enabled,
            fhir_feed = audit.fhir_feed.enabled,
            queue_capacity = audit.queue_capacity,
            fail_mode = ?audit.fail_mode,
            resolve_subject = audit.resolve_subject,
            "IHE ATNA audit enabled"
        );
    }
    // A legitimate posture that is wrong to run silently is said at `warn`
    // with a structured field, the same sentence the readiness indicator and
    // `/management/info` carry, so a collector alerts on it without parsing
    // (#3238).
    for caution in AuditPosture::of(audit).cautions() {
        tracing::warn!(posture = "audit", caution, "audit posture");
    }
    if management.enabled {
        tracing::info!(
            base_path = %management.base_path,
            listener = match management.port {
                Some(port) => format!("own port {port}"),
                None => "shared with the API".to_owned(),
            },
            mounted = %mounted_management_endpoints(management.endpoints),
            "management surface enabled"
        );
    }
    tracing::info!(
        enabled = app_config.server.rate_limit.enabled,
        principal_per_second = app_config.server.rate_limit.principal_per_second,
        address_per_second = app_config.server.rate_limit.address_per_second,
        "request-rate limiting configured"
    );
    tracing::info!(
        bind = %app_config.server.bind,
        base_path = %app_config.server.base_path,
        tls = app_config.server.tls.enabled,
        "starting ferroehr"
    );
}

/// Evaluate the declared deployment posture over the clusters the pools
/// reached, refuse to start under `production` while a separation is open and
/// not accepted, and say every open gap at `warn` otherwise (#3226).
///
/// # Errors
/// The refusal, listing each open gap with its finding and remedy; or a
/// failure reading a pool's cluster identity.
async fn evaluate_deployment(
    config: &ferroehr::config::FerroEhrConfig,
    pools: &Pools,
) -> anyhow::Result<DeploymentPosture> {
    let clusters = ClusterIdentities {
        clinical: Some(
            db::cluster_identity(&pools.clinical)
                .await
                .context("reading the clinical pool's cluster identity")?,
        ),
        demographic: Some(
            db::cluster_identity(&pools.demographic)
                .await
                .context("reading the demographic pool's cluster identity")?,
        ),
        linkage: Some(
            db::cluster_identity(&pools.linkage)
                .await
                .context("reading the linkage pool's cluster identity")?,
        ),
    };
    let posture = DeploymentPosture::evaluate(config, &clusters);
    if !posture.permits_boot() {
        anyhow::bail!("{}", posture.refusal_message());
    }
    for gap in &posture.gaps {
        let accepted = posture.accepted.contains(gap);
        tracing::warn!(
            posture = "deployment",
            profile = %posture.profile,
            gap = %gap,
            accepted,
            "{}",
            gap.describe()
        );
    }
    if posture.profile == DeploymentProfile::Sandbox {
        tracing::warn!(
            posture = "deployment",
            profile = "sandbox",
            open_gaps = posture.gaps.len(),
            "deployment_profile is sandbox: this deployment has not made the production \
             separations and must not hold real patient data"
        );
    } else {
        tracing::info!(
            posture = "deployment",
            profile = "production",
            accepted_gaps = posture.accepted.len(),
            "deployment_profile is production: every separation holds or is accepted by name"
        );
    }
    Ok(posture)
}

/// Stamp the subject-shape posture the database guard reads (#3241) and report
/// the rows a newly declared namespace finds already stored.
///
/// # Errors
/// A failure writing the stamp or counting the rows.
async fn stamp_subject_posture(
    config: &ferroehr::config::FerroEhrConfig,
    pool: &PgPool,
) -> anyhow::Result<()> {
    let required = !config.privacy.subject_namespaces.is_empty();
    db::stamp_subject_posture(pool, required)
        .await
        .context("stamping the subject pseudonym posture")?;
    if required {
        let stored = db::non_pseudonym_subjects(pool)
            .await
            .context("counting stored subject references that are not pseudonyms")?;
        if stored > 0 {
            tracing::warn!(
                posture = "subject_pseudonyms",
                ehrs = stored,
                "privacy.subject_namespaces is declared, but {stored} stored EHR(s) carry a \
                 subject reference that is not a UUID; the database now refuses new ones, and \
                 these need re-pseudonymising"
            );
        }
    }
    Ok(())
}

fn warn_boot_postures(config: &ferroehr::config::FerroEhrConfig) {
    if config.db.is_dev_default() {
        tracing::warn!(
            url = db::DEFAULT_URL,
            "[db].url is the built-in DEVELOPMENT DEFAULT ({}); no file/env/CLI value was \
             supplied. Set db.url (FERROEHR__DB__URL / DATABASE_URL) for any non-dev deployment — \
             production MUST override it.",
            db::DEFAULT_URL,
        );
    }
    if config.server.cors_permissive {
        tracing::warn!(
            "[server].cors_permissive is ON: any origin may read API responses. This is a \
             DEVELOPMENT setting — configure explicit origins for any deployment reachable by \
             a browser."
        );
    }
    if config.privacy.allow_identified_parties_in_ehr {
        tracing::warn!(
            "[privacy].allow_identified_parties_in_ehr is ON: beyond the provider names a \
             PARTY_IDENTIFIED may carry by default, clinical content may now also carry \
             formal identifiers on either party class and a PARTY_RELATED name (composer, \
             participations, health_care_facility, feeder audit). Formal identifiers are \
             the national-identifier slot, and a party named in relation to the subject \
             re-identifies the subject — GDPR Art. 25(2) data minimisation is on this \
             deployment to justify."
        );
    }
    if config.privacy.identifier_scan.mode == ferroehr::privacy::config::ScanMode::Warn {
        tracing::warn!(
            "[privacy.identifier_scan].mode is `warn`: a clinical write carrying a value one \
             of the active rules claims is ACCEPTED and recorded, not refused. Set it to \
             `strict` once the recorded findings are down to none."
        );
    }
    // Which jurisdictions are actually covered is the one fact an operator
    // cannot infer, and believing a jurisdiction is scanned when it is not is
    // the failure this line exists to prevent.
    if config.privacy.identifier_scan.rules.is_empty() {
        tracing::warn!(
            "[privacy.identifier_scan].rules is empty: NO national identifier rule is active, \
             so the scanner checks only the configured patterns. Name the rules this \
             deployment's content could carry."
        );
    } else {
        tracing::info!(
            rules = %config.privacy.identifier_scan.rules.join(", "),
            patterns = config.privacy.identifier_scan.patterns.len(),
            mode = config.privacy.identifier_scan.mode.as_str(),
            "clinical writes are scanned for national identifiers"
        );
    }
    if config.privacy.subject_namespaces.is_empty() {
        tracing::info!(
            "[privacy].subject_namespaces is empty: an EHR_STATUS subject reference is \
             accepted in any namespace with any identifier. Declare this deployment's \
             pseudonym namespaces to bind the subject to an opaque UUID."
        );
    } else {
        tracing::info!(
            namespaces = %config.privacy.subject_namespaces.join(", "),
            "EHR_STATUS subject references are bound to a UUID in a configured pseudonym \
             namespace"
        );
    }
    if config.auth.enabled && !config.server.tls.enabled && !binds_loopback(&config.server.bind) {
        tracing::warn!(
            bind = %config.server.bind,
            "authentication is enabled but this listener is PLAINTEXT on a routable address. \
             Credentials and bearer tokens will cross the wire unencrypted unless a \
             TLS-terminating proxy fronts this port. Enable [server.tls] or ensure the ingress \
             terminates TLS."
        );
    }
}

/// The two domain pools the server runs on: the clinical one and the
/// demographic (pseudonymisation-domain) one.
///
/// They differ in `search_path` always, and in credential when the deployment
/// sets `[db].demographic_url`. No openEHR spec governs storage layout or
/// database roles — our own design/extension.
#[derive(Debug)]
struct Pools {
    /// Serves the `ehr` schema: EHRs, compositions, folders, templates,
    /// eventing and every supporting relation.
    clinical: PgPool,
    /// Serves the `demographic` schema: parties and their change control.
    demographic: PgPool,
    /// Serves the `linkage` schema: which party is the subject of which EHR.
    linkage: PgPool,
}

/// Connects both domain pools the deployment's tenancy mode calls for and
/// prepares the schema.
///
/// Multi-tenant mode swaps in the tenant-scoped pools, stamping every
/// checked-out connection with the request's `ferroehr.tenant_id` session GUC
/// that the RLS `tenant_isolation` policy reads; single-tenant deployments keep
/// the plain ones. The demographic pool carries the `demographic` search path
/// and, when `[db].demographic_url` is set, its own credential — which is what
/// makes the pseudonymisation boundary a role boundary rather than only a
/// schema one. Neither multi-tenancy nor the domain split is governed by an
/// openEHR spec; both are our own deployment extensions.
///
/// Schema preparation is [`ferroehr::db::prepare`], which spans every schema
/// on the migration DSN (`[db].migrate_url`, falling back to `[db].url`) and
/// then measures the CLINICAL pool's own reach with
/// [`ferroehr::db::verify_domain_isolation`], refusing to boot a database
/// whose grants let one runtime role read another domain.
///
/// # Errors
/// A connection, migration or domain-isolation failure, contextualized for the
/// operator.
async fn connect_pool(config: &ferroehr::config::FerroEhrConfig) -> anyhow::Result<Pools> {
    let (clinical, demographic, linkage) = if config.tenancy.enabled {
        (
            db::connect_tenant_scoped(&config.db)
                .await
                .context("connecting to PostgreSQL (tenant-scoped)")?,
            db::connect_tenant_scoped_demographic(&config.db)
                .await
                .context("connecting to PostgreSQL (demographic, tenant-scoped)")?,
            db::connect_tenant_scoped_linkage(&config.db)
                .await
                .context("connecting to PostgreSQL (linkage, tenant-scoped)")?,
        )
    } else {
        (
            db::connect(&config.db)
                .await
                .context("connecting to PostgreSQL")?,
            db::connect_demographic(&config.db)
                .await
                .context("connecting to PostgreSQL (demographic)")?,
            db::connect_linkage(&config.db)
                .await
                .context("connecting to PostgreSQL (linkage)")?,
        )
    };
    db::prepare(&config.db, &clinical)
        .await
        .context("preparing the database schema")?;
    if config.db.roles_are_separated() {
        tracing::info!(
            "the demographic domain connects on its own DSN: the clinical and demographic \
             credentials are separate database roles"
        );
    }
    if config.db.linkage_role_is_separated() {
        tracing::info!(
            "the linkage domain connects on its own DSN: the map from a party to its EHR is \
             held on a credential that has neither the clinical nor the demographic grants"
        );
    }
    if config.tenancy.enabled {
        warn_on_occupied_default_tenant(&clinical).await?;
    }
    Ok(Pools {
        clinical,
        demographic,
        linkage,
    })
}

/// Says at boot how much content the reserved default tenant holds, because a
/// request the tenancy middleware cannot scope reads exactly that.
///
/// The default tenant owns every row written while tenancy was off, so on a
/// deployment that enables tenancy over an existing store it IS the legacy
/// repository. Two request shapes still land there: one carrying no tenant key
/// at all, and, under `tenancy.unknown_tenant = "default_tenant"`, one whose
/// key names no registered tenant.
///
/// A warning rather than a refusal: an operator enabling tenancy on a live
/// store has no in-product way to reassign those rows yet, so refusing would
/// strand the deployment instead of protecting it.
///
/// # Errors
/// A database failure reading the count.
async fn warn_on_occupied_default_tenant(pool: &PgPool) -> anyhow::Result<()> {
    let versions = db::default_tenant_versions(pool)
        .await
        .context("counting the reserved default tenant's stored versions")?;
    if versions > 0 {
        tracing::warn!(
            default_tenant_versions = versions,
            "tenancy is enabled and the reserved default tenant owns stored versions: a request \
             with no tenant key reads them, and so does an unresolvable key unless \
             tenancy.unknown_tenant is \"refuse\". Move this content into a named tenant, or \
             treat the deployment as single-tenant."
        );
    }
    Ok(())
}

/// Wires the opt-in external FHIR terminology servers — ALL configured
/// providers, with the terminology→provider routing (a deployment binds
/// several terminologies at once; BASE
/// `architecture_overview/master12-terminology.adoc` §Overview).
///
/// # Errors
/// A provider that cannot be built (a boot error: configuration promising a
/// terminology binding must never degrade to no binding).
fn attach_terminology(
    service: FerroEhrService,
    config: &ferroehr::config::FerroEhrConfig,
) -> anyhow::Result<FerroEhrService> {
    let Some(router) = ferroehr::service::terminology::router::TerminologyRouter::build(
        &config.terminology.external,
    )
    .context("initialising the external terminology providers")?
    else {
        return Ok(service);
    };
    tracing::info!(
        providers = %router.provider_names().collect::<Vec<_>>().join(", "),
        fail_on_error = router.fail_on_error(),
        "external FHIR terminology providers configured"
    );
    Ok(service.with_terminology_router(Arc::new(router)))
}

/// Wires the opt-in Subject Proxy FHIR-frame executor (fail-closed).
///
/// # Errors
/// An executor that cannot be built.
fn attach_subject_proxy(
    service: FerroEhrService,
    config: &ferroehr::config::FerroEhrConfig,
) -> anyhow::Result<FerroEhrService> {
    let Some(fhir) = config
        .subject_proxy
        .build()
        .context("initialising the subject-proxy FHIR executor")?
    else {
        return Ok(service);
    };
    tracing::info!("subject-proxy FHIR-frame executor configured");
    Ok(service.with_subject_proxy(Arc::new(fhir)))
}

/// Wires the opt-in `DV_MULTIMEDIA` externalization behind the `multimedia`
/// cargo feature; a slim build refuses an enabled config loudly.
///
/// A store kept only to read back already-offloaded blobs must never stop the
/// server starting, so an unbuildable store is fatal only when the integration
/// is enabled.
///
/// # Errors
/// An unbuildable object store while the integration is enabled, or an enabled
/// configuration in a build without the feature.
fn attach_multimedia(
    service: FerroEhrService,
    config: &ferroehr::config::FerroEhrConfig,
) -> anyhow::Result<FerroEhrService> {
    #[cfg(feature = "multimedia")]
    {
        let engine = match ferroehr::extensions::multimedia::engine_from_config(&config.multimedia)
        {
            Ok(engine) => engine,
            Err(e) if !config.multimedia.enabled => {
                tracing::warn!(
                    error = %e,
                    "multimedia is disabled and its object store could not be built: \
                     already-externalized content cannot be re-inlined, and a read that \
                     asks for it will be refused rather than answered with the reference"
                );
                None
            }
            Err(e) => {
                return Err(
                    anyhow::Error::new(e).context("initialising the multimedia object store")
                );
            }
        };
        let Some(engine) = engine else {
            return Ok(service);
        };
        if engine.offload_enabled() {
            tracing::info!(
                bucket = %config.multimedia.bucket,
                threshold_bytes = config.multimedia.threshold_bytes,
                "DV_MULTIMEDIA externalization enabled"
            );
        } else {
            tracing::info!(
                bucket = %config.multimedia.bucket,
                "DV_MULTIMEDIA externalization disabled; the configured store stays \
                 readable so already-externalized content can still be served"
            );
        }
        Ok(service.with_multimedia(Arc::new(engine)))
    }
    #[cfg(not(feature = "multimedia"))]
    {
        ferroehr::extensions::multimedia::require_disabled(&config.multimedia)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(service)
    }
}

/// Boot the server: config, telemetry, pool, migrations, audit, health, serve.
#[expect(
    clippy::too_many_lines,
    reason = "the remaining body is one linear boot sequence of subsystem \
              startups, each closed over by the next; splitting it further \
              would obscure the order that makes it correct"
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

    // ASCII banner before telemetry/log init, on the same resolved rendering the
    // log layer installs (so JSON output stays pure from the first byte).
    if prints_banner(telemetry_config.log.format, std::io::stdout().is_terminal()) {
        // The posture the configuration alone shows; the cluster check needs
        // the pools and follows in the boot log.
        let declared = DeploymentPosture::evaluate(&config, &ClusterIdentities::default());
        ferroehr::banner::print(config.spec_profile, &declared, true);
    }

    let build_info =
        BuildInfo::for_profile(config.spec_profile).with_audit(AuditPosture::of(&config.audit));
    let mut telemetry =
        telemetry::init(&telemetry_config, &build_info).context("initialising telemetry")?;

    warn_boot_postures(&config);
    let pools = connect_pool(&config).await?;
    let pool = pools.clinical.clone();

    // The declared deployment posture, over the clusters the pools actually
    // reached (#3226): `production` refuses here, `sandbox` says what is open.
    let deployment = evaluate_deployment(&config, &pools).await?;

    // The database's own subject-shape guard follows the declared namespaces
    // (#3241): stamped by the runtime role on every boot, read by the trigger.
    stamp_subject_posture(&config, &pool).await?;

    // Fail-open at boot, except in a slim build, which cannot render the FHIR
    // `AuditEvent` the store and the ATX:FHIR Feed carry.
    #[cfg(not(feature = "fhir"))]
    ferroehr::system_log::require_fhir_disabled(&config.audit).map_err(|e| anyhow::anyhow!(e))?;
    let audit_config: AuditConfig = config.audit.clone();
    let (audit_sender, audit_handle) = start_audit(&audit_config, &pool).await;

    // Contribution-outbox eventing + FHIR outbound emitter (both off by default).
    let outbox_enabled = config.events.enabled || config.fhir.outbound.enabled;
    #[cfg(feature = "events")]
    let events_handle = if config.events.enabled {
        tracing::info!(exchange = %config.events.exchange, "contribution-outbox eventing enabled");
        Some(ferroehr::extensions::events::publisher::start(
            config.events.clone(),
            pool.clone(),
            pools.demographic.clone(),
        ))
    } else {
        None
    };
    #[cfg(not(feature = "events"))]
    ferroehr::extensions::events::require_disabled(&config.events)
        .map_err(|e| anyhow::anyhow!(e))?;

    // Health indicators.
    let mut indicators: Vec<Arc<dyn HealthIndicator>> = vec![
        Arc::new(indicators::DbHealth::new(pool.clone())),
        Arc::new(indicators::MigrationsHealth::new(pool.clone())),
    ];
    // Always registered: a deployment that writes no access log reads DEGRADED
    // with the consequence stated rather than showing no row (#3238).
    let audit_posture = AuditPosture::of(&config.audit);
    indicators.push(Arc::new(match &audit_sender {
        Some(sender) => indicators::AuditHealth::new(sender.clone(), audit_posture),
        None => indicators::AuditHealth::disabled(audit_posture),
    }));
    #[cfg(feature = "events")]
    if let Some(handle) = &events_handle {
        indicators.push(Arc::new(indicators::EventsHealth::new(handle.healthy())));
    }

    telemetry.start_samplers(pool.clone());

    // `/management/env` reports the whole config tree; secrets render `***` by
    // construction of the `Secret` type.
    let env_snapshot = Arc::new(serde_json::to_value(&config).unwrap_or(serde_json::Value::Null));

    // Version signing (fail-closed at boot for `pgp` without a usable key).
    let signer =
        Arc::new(Signer::from_config(&config.signing).context("initialising the version signer")?);
    tracing::info!(
        signing = signer.enabled(),
        verify_on_read = ?signer.verify_on_read(),
        "version signing configured"
    );

    // `[server] system_id` is stamped into `EHR.system_id`,
    // `AUDIT_DETAILS.system_id` and `OBJECT_VERSION_ID.creating_system_id`;
    // logged so an operator can see the key took.
    tracing::info!(system_id = %config.server.system_id, "openEHR system identifier");

    let audit_enabled = audit_sender.is_some();
    let service = Arc::new(assemble_service(
        &config,
        &pools,
        audit_sender,
        outbox_enabled,
        signer,
        deployment,
    )?);

    // Off by default (it carries PHI) and gated on the `fhir` feature, which
    // itself implies `events` for the broker transport.
    #[cfg(feature = "fhir")]
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
    #[cfg(not(feature = "fhir"))]
    if config.fhir.outbound.enabled {
        return Err(anyhow::anyhow!(
            "fhir.outbound.enabled = true, but this binary was built without the `fhir` cargo feature"
        ));
    }
    #[cfg(feature = "fhir")]
    if let Some(handle) = &fhir_outbound_handle {
        indicators.push(Arc::new(indicators::FhirOutboundHealth::new(
            handle.healthy(),
        )));
    }

    // The readiness registry closes over every started subsystem, so it is
    // assembled after the last optional one (the FHIR outbound emitter).
    let observability = Observability {
        management: config.management.clone(),
        prometheus: Some(telemetry.registry()),
        log_reload: Some(telemetry.log_reload()),
        health: HealthRegistry::new(indicators),
        build_info,
        env_snapshot,
    };

    let authz = wire_authz(&config, &pool, &service)?;

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
        audit_organization_claim: config.authz.abac.organization_claim().map(str::to_owned),
        spec_profile: config.spec_profile,
    };

    // RFC 9110 §11.6.1: a 401 challenge must name a scheme applicable to the
    // target resource, and a server with no mechanism has none to name.
    app_config
        .auth
        .require_mechanism()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    log_resolved_posture(
        &app_config,
        audit_enabled,
        &config.audit,
        &observability.management,
    );
    ferroehr_rest::serve_full(app_config, service, authz, observability)
        .await
        .context("serving ferroehr-rest")?;

    if let Some(handle) = audit_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    #[cfg(feature = "events")]
    if let Some(handle) = events_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    #[cfg(feature = "fhir")]
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

/// Builds the full authorization handle the binary serves with: the RBAC gate
/// plus the ABAC gate over the boot-built policy engine.
///
/// `None` when neither layer is active. Fine-grained authorization is our own
/// extension — no openEHR spec governs it (ITS-REST places authorization out of
/// band).
///
/// # Errors
/// An ABAC block that is enabled but unbuildable: startup must abort rather than
/// silently run without the promised gate.
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
                    .map_err(|e| ResolveError::new(format!("ehr id {ehr_id}"), e))?;
                sqlx::query_scalar::<_, Option<String>>("SELECT subject_id FROM ehr WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&pool)
                    .await
                    .map(Option::flatten)
                    .map_err(|e| ResolveError::new("ehr subject lookup", e))
            })
        }),
        template_of_version: Arc::new(move |vo: String, version: Option<String>| {
            let service = Arc::clone(&service);
            Box::pin(async move {
                let vo_id = vo
                    .parse::<ferroehr::ids::VoId>()
                    .map_err(|e| ResolveError::new(format!("vo id {vo}"), e))?;
                service
                    .template_of_version(vo_id, version.as_deref())
                    .await
                    .map_err(|e| ResolveError::new("template lookup", e))
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

#[cfg(test)]
mod tests {
    use super::{LogFormat, binds_loopback, prints_banner};

    /// The banner follows the RESOLVED rendering: `auto` with stdout piped into
    /// a log collector renders JSON, so no banner may precede it. The terminal
    /// state is injected — never probed from the process — so both directions
    /// are pinned deterministically.
    #[test]
    fn the_banner_is_keyed_on_the_resolved_log_format() {
        assert!(!prints_banner(LogFormat::Auto, false));
        assert!(prints_banner(LogFormat::Auto, true));
        for is_terminal in [false, true] {
            assert!(
                !prints_banner(LogFormat::Json, is_terminal),
                "explicit json never prints the banner"
            );
            assert!(
                prints_banner(LogFormat::Pretty, is_terminal),
                "explicit pretty always prints the banner"
            );
        }
    }

    #[test]
    fn loopback_binds_are_recognized() {
        assert!(binds_loopback("127.0.0.1:8080"));
        assert!(binds_loopback("127.0.0.53:8080"));
        assert!(binds_loopback("[::1]:8080"));
    }

    /// The plaintext-authentication warning must fire for anything reachable off
    /// the host, and an unparseable host counts as reachable.
    #[test]
    fn routable_and_ambiguous_binds_are_not_loopback() {
        for bind in [
            "0.0.0.0:8080",
            "10.0.0.4:8080",
            "[::]:8080",
            "ferroehr.internal:8080",
            ":8080",
        ] {
            assert!(!binds_loopback(bind), "{bind} must count as routable");
        }
    }
}
