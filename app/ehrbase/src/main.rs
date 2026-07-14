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
use ehrbase::system_log::{AuditConfig, AuditHandle, AuditSender, SubjectResolver};
use ehrbase::versioning::signature::{Signer, SigningConfig};
use ehrbase_rest::access::authz::AuthzConfig;
use ehrbase_rest::management::{BuildInfo, HealthIndicator, HealthRegistry, ManagementConfig};
use ehrbase_rest::{AuthzHandle, Observability};
use sqlx::PgPool;
use uuid::Uuid;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase::telemetry::{self, TelemetryConfig, indicators};

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

/// Boot the server: telemetry, config, pool, migrations, audit, health, then serve.
#[allow(clippy::too_many_lines)] // linear boot sequence; splitting it would obscure order
async fn serve() -> anyhow::Result<()> {
    // Load configuration first (telemetry init needs the log/otel config).
    let telemetry_config = TelemetryConfig::load().context("loading telemetry configuration")?;

    // Greet with the ASCII banner on stdout BEFORE telemetry/log init, so the
    // structured formatter never mangles the art. Skipped under `json` logging
    // (EHRBASE_LOG_FORMAT=json) — machine log consumers want one JSON object per
    // line, not decorative art; every other mode (auto/pretty) prints it.
    if telemetry_config.log.format != ehrbase::telemetry::LogFormat::Json {
        ehrbase::banner::print();
    }

    let rest_config = ehrbase_rest::RestConfig::load().context("loading REST configuration")?;
    let management_config = ManagementConfig::load().context("loading management configuration")?;
    let audit_config = AuditConfig::load().context("loading ATNA audit configuration")?;
    let authz_config = AuthzConfig::load().context("loading authorization configuration")?;
    authz_config
        .validate()
        .context("validating authorization configuration")?;
    let db_settings = DbSettings::from_env().context("loading database settings")?;

    // Telemetry: install the subscriber (logs + optional OTLP spans), the
    // Prometheus recorder, and (opt-in) the OTLP metrics push — before anything
    // else emits spans/metrics.
    let build_info = BuildInfo::current();
    let mut telemetry =
        telemetry::init(&telemetry_config, &build_info).context("initialising telemetry")?;

    let pool = db::connect(&db_settings)
        .await
        .context("connecting to PostgreSQL")?;
    db::run_migrations(&pool)
        .await
        .context("applying migrations")?;

    // Boot the ATNA audit sender (fail-open at boot: log and continue without
    // auditing if the transport cannot be established, so the CDR still serves).
    let (audit_sender, audit_handle) = start_audit(&audit_config, &pool).await;

    // Contribution-outbox eventing: off by default. When enabled, the
    // publisher drains the transactional outbox to the broker at-least-once; a
    // broker that is down is tolerated (the outbox buffers), so we spawn it
    // unconditionally-on-enabled and never fail boot on the broker.
    let events_config = ehrbase::extensions::events::EventsConfig::load()
        .context("loading eventing configuration")?;
    // The FHIR outbound emitter is the second event-outbox consumer; load its
    // config now (used again below to spawn it) so the transactional outbox
    // write can be gated on whether ANY consumer is configured on. No consumer
    // configured ⇒ the per-commit `event_outbox` INSERT is pure overhead and is
    // skipped (our own extension; no openEHR spec governs eventing).
    let fhir_outbound_config = ehrbase::extensions::fhir::FhirOutboundConfig::load()
        .context("loading FHIR outbound configuration")?;
    let outbox_enabled = events_config.enabled || fhir_outbound_config.enabled;
    let events_handle = if events_config.enabled {
        tracing::info!(exchange = %events_config.exchange, "contribution-outbox eventing enabled");
        Some(ehrbase::extensions::events::start(
            events_config,
            pool.clone(),
        ))
    } else {
        None
    };

    // Health indicators (DB ping + migrations-applied + audit-sender + events).
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

    // Start the background gauge sampler over the pool.
    telemetry.start_samplers(pool.clone());

    let env_snapshot = Arc::new(env_snapshot(
        &rest_config,
        &management_config,
        &telemetry_config,
        &db_settings,
    ));
    let observability = Observability {
        management: management_config,
        prometheus: Some(telemetry.prometheus_handle()),
        log_reload: Some(telemetry.log_reload()),
        health,
        build_info,
        env_snapshot,
    };

    // Version signing (RM common §"Digital Signature"; docs/design/version-signing.md).
    // Fail-closed at boot: `pgp` mode without a loadable, usable key refuses to
    // start (Signer::from_config performs the key load + a test signature).
    let signing_config = SigningConfig::load().context("loading signing configuration")?;
    let signer =
        Arc::new(Signer::from_config(&signing_config).context("initialising the version signer")?);
    tracing::info!(
        signing = signer.enabled(),
        verify_on_read = ?signer.verify_on_read(),
        "version signing configured"
    );

    // The audit sender (when enabled) is injected into the platform service:
    // it realizes the SM `SystemLog` component the REST audit layer emits
    // through (the system_log module).
    let audit_enabled = audit_sender.is_some();
    let mut service = EhrbaseService::new(pool.clone())
        .with_signer(signer)
        .with_outbox_enabled(outbox_enabled);
    if let Some(sender) = audit_sender {
        service = service.with_audit(sender);
    }

    // Opt-in external FHIR terminology provider (B4): when a deployment
    // configures one, wire it so AQL `TERMINOLOGY('expand', 'hl7.org/fhir/…',
    // …)` resolves against it; otherwise AQL terminology expansion routes only
    // to the in-process `openehr-term` bundle.
    match ehrbase::service::ExternalTerminologyConfig::load()
        .context("loading external-terminology configuration")?
        .default_provider()
    {
        Some(Ok(provider)) => {
            tracing::info!("external FHIR terminology provider configured");
            service = service.with_external_terminology(Arc::new(provider));
        }
        Some(Err(e)) => return Err(e).context("initialising the external terminology provider"),
        None => {}
    }

    // Opt-in Subject Proxy FHIR-frame executor: when a deployment configures FHIR
    // systems (EHRBASE_SUBJECT_PROXY__SYSTEMS__…), an `API_CALL`/`fhir_get`
    // DATA_FRAME retrieves from them (`I_DATA_BINDING`, `hl7_fhir_sample.adoc`);
    // otherwise every FHIR frame is a typed rejection (fail-closed).
    if let Some(fhir) = ehrbase::service::SubjectProxyConfig::load()
        .context("loading subject-proxy configuration")?
        .build()
        .context("initialising the subject-proxy FHIR executor")?
    {
        tracing::info!("subject-proxy FHIR-frame executor configured");
        service = service.with_subject_proxy(Arc::new(fhir));
    }

    // Opt-in DV_MULTIMEDIA externalization: off by default (inline
    // behaviour byte-identical). When enabled, large inline media is offloaded
    // to S3-compatible object storage on commit and re-inlined on demand.
    let multimedia_config = ehrbase::extensions::multimedia::MultimediaConfig::load()
        .context("loading multimedia configuration")?;
    if let Some(engine) =
        ehrbase::extensions::multimedia::MultimediaEngine::from_config(&multimedia_config)
            .context("initialising the multimedia object store")?
    {
        tracing::info!(
            bucket = %multimedia_config.bucket,
            threshold_bytes = multimedia_config.threshold_bytes,
            "DV_MULTIMEDIA externalization enabled (S3-compatible object storage)"
        );
        service = service.with_multimedia(Arc::new(engine));
    }

    // The service is now fully built; share it (the FHIR outbound emitter and the
    // REST server both hold it).
    let service = Arc::new(service);

    // FHIR outbound emitter: off by default. When enabled,
    // it walks committed outbox rows, reverse-maps matching COMPOSITIONs, and
    // publishes the FHIR resources to the broker — carrying PHI by design, hence
    // its own explicit gate (a separate switch from the REST FHIR connector).
    let fhir_outbound_handle = if fhir_outbound_config.enabled {
        tracing::info!(
            exchange = %fhir_outbound_config.exchange,
            "FHIR outbound emitter enabled (publishes clinical FHIR resources)"
        );
        Some(ehrbase::extensions::fhir::start(
            fhir_outbound_config,
            pool.clone(),
            service.clone(),
        ))
    } else {
        None
    };

    // Build the RBAC gate. Only wired when authentication is enabled (the gate
    // runs after authentication); `from_config` yields `None` when RBAC is
    // disabled, restoring authentication-only behaviour.
    let authz = if rest_config.auth.enabled {
        AuthzHandle::from_config(&authz_config, &rest_config.base_path).map(Arc::new)
    } else {
        None
    };

    tracing::info!(
        bind = %rest_config.bind,
        base_path = %rest_config.base_path,
        audit = audit_enabled,
        rbac = authz.is_some(),
        management = observability.management.enabled,
        "starting ehrbase"
    );
    ehrbase_rest::serve_full(rest_config, service, authz, observability)
        .await
        .context("serving ehrbase-rest")?;

    // The server has stopped and dropped its audit-sender clone; drain the queue.
    if let Some(handle) = audit_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    // Stop the eventing publisher (a final best-effort drain; unpublished rows
    // stay pending in the outbox and drain on next start — at-least-once).
    if let Some(handle) = events_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    // Stop the FHIR outbound emitter (rows past its cursor emit on next start —
    // at-least-once).
    if let Some(handle) = fhir_outbound_handle {
        handle.shutdown(AUDIT_DRAIN_TIMEOUT).await;
    }
    // Flush OTel exporters + stop samplers on the same shutdown path.
    telemetry.shutdown().await;
    Ok(())
}

/// Compose the effective-configuration snapshot for `/management/env`. Secrets
/// (auth hashes, HMAC/JWKS material) and the DB DSN credentials are masked by
/// the management endpoint's redactor at render time; this only assembles the
/// structured view.
fn env_snapshot(
    rest: &ehrbase_rest::RestConfig,
    management: &ManagementConfig,
    telemetry: &TelemetryConfig,
    db: &DbSettings,
) -> serde_json::Value {
    serde_json::json!({
        "rest": rest,
        "management": management,
        "telemetry": telemetry,
        "db": { "url": db.url },
    })
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
    match ehrbase::system_log::start(config.clone(), resolver).await {
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
