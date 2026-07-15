//! The one server configuration tree — `ehrbase.toml` + `EHRBASE_*` env
//! overrides (`docs/design/configuration.md`).
//!
//! No openEHR spec governs configuration — this is entirely our own design.
//! [`EhrbaseConfig`] is the single serde root; each section is owned by the
//! crate that consumes it (§5.2) and referenced here. There is exactly one
//! loader ([`load`]/[`assemble`]) replacing the fourteen former per-subsystem
//! loaders — no figment, no per-subsystem `EHRBASE_*_CONFIG` file pointers.
//!
//! Precedence (lowest→highest): built-in `Default` impls, the config file, the
//! `EHRBASE_*` environment (`__` = nesting, `docs/design/configuration.md`
//! §P-4), then `--set key=value` overrides. Two conventional aliases sit below
//! their `EHRBASE_` forms within the env layer: `DATABASE_URL` → `db.url`,
//! `RUST_LOG` → `log.filter`.
//!
//! [`assemble`] is a **pure function** of `(file, env_map, overrides)` — no
//! process-global env — so the whole test plan runs on injected inputs.

mod alias;
mod loader;
mod strict;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use loader::{ConfigError, ConfigErrors};

/// The complete server configuration. Every section has a `Default`, so the
/// file may be empty or absent (zero-config boot, §3.16). `deny_unknown_fields`
/// makes a misspelled top-level table a boot error (§P-5).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EhrbaseConfig {
    /// `[server]` — HTTP listener + REST surface + System-Options identity.
    pub server: ehrbase_rest::config::ServerConfig,
    /// `[db]` — `PostgreSQL` connection.
    pub db: crate::db::DbConfig,
    /// `[log]` — logging.
    pub log: crate::telemetry::LogConfig,
    /// `[telemetry]` — OpenTelemetry export.
    pub telemetry: crate::telemetry::OtelConfig,
    /// `[auth]` — authentication.
    pub auth: ehrbase_rest::access::authn::AuthConfig,
    /// `[authz]` — RBAC + ABAC.
    pub authz: ehrbase_rest::access::authz::AuthzConfig,
    /// `[admin]` — the ADMIN API group.
    pub admin: ehrbase_rest::config::AdminConfig,
    /// `[tenancy]` — multi-tenancy.
    pub tenancy: ehrbase_rest::config::TenancyConfig,
    /// `[smart]` — SMART App Launch.
    pub smart: ehrbase_rest::smart::config::SmartConfig,
    /// `[management]` — the management/observability surface.
    pub management: ehrbase_rest::management::ManagementConfig,
    /// `[signing]` — VERSION signing.
    pub signing: crate::versioning::signature::SigningConfig,
    /// `[query]` — AQL execution knobs.
    pub query: crate::service::QueryConfig,
    /// `[events]` — contribution-outbox eventing (+ its admin API).
    pub events: crate::extensions::events::EventsConfig,
    /// `[fhir]` — the FHIR connector (inbound façade + outbound emitter).
    pub fhir: crate::extensions::fhir::FhirConfig,
    /// `[terminology]` — terminology API + external-server validation.
    pub terminology: crate::service::TerminologyConfig,
    /// `[multimedia]` — `DV_MULTIMEDIA` externalization.
    pub multimedia: crate::extensions::multimedia::MultimediaConfig,
    /// `[atna]` — IHE ATNA audit / System Log.
    pub atna: crate::system_log::AuditConfig,
    /// `[subject_proxy]` — Subject Proxy FHIR systems.
    pub subject_proxy: crate::service::SubjectProxyConfig,
}

/// The annotated default template `ehrbase config default` prints — a
/// hand-maintained asset kept in sync with the schema by the template tests.
pub const DEFAULT_TEMPLATE: &str = include_str!("../../assets/ehrbase.default.toml");

impl EhrbaseConfig {
    /// Aggregated semantic validation (§5.8): every cross-field rule reported at
    /// once, so an operator fixes the config in one iteration.
    ///
    /// # Errors
    /// [`ConfigErrors`] carrying every failing rule.
    pub fn validate(&self) -> Result<(), ConfigErrors> {
        let mut errors = Vec::new();

        // Authorization rules (moved verbatim from the old AuthzConfig::validate).
        if let Err(e) = self.authz.validate() {
            errors.push(ConfigError::semantic(format!("authz: {e}")));
        }
        // SMART deprecated-grant rule.
        if let Err(e) = self.smart.validate() {
            errors.push(ConfigError::semantic(format!("smart: {e}")));
        }
        // signing.mode = pgp ⇒ key_path set.
        if matches!(self.signing.mode, crate::versioning::signature::Mode::Pgp)
            && self.signing.key_path.is_none()
        {
            errors.push(ConfigError::semantic(
                "signing.mode = \"pgp\" requires signing.key_path".to_owned(),
            ));
        }
        // Secret / *_file mutual exclusion.
        if self.signing.key_passphrase.is_some() && self.signing.key_passphrase_file.is_some() {
            errors.push(ConfigError::semantic(
                "set only one of signing.key_passphrase / signing.key_passphrase_file".to_owned(),
            ));
        }
        if let Some(oidc) = &self.auth.oidc {
            if oidc.hmac_secret.is_some() && oidc.hmac_secret_file.is_some() {
                errors.push(ConfigError::semantic(
                    "set only one of auth.oidc.hmac_secret / auth.oidc.hmac_secret_file".to_owned(),
                ));
            }
            if oidc.jwks_json.is_some() && oidc.jwks_json_file.is_some() {
                errors.push(ConfigError::semantic(
                    "set only one of auth.oidc.jwks_json / auth.oidc.jwks_json_file".to_owned(),
                ));
            }
        }
        if self.multimedia.secret_access_key.is_some()
            && self.multimedia.secret_access_key_file.is_some()
        {
            errors.push(ConfigError::semantic(
                "set only one of multimedia.secret_access_key / \
                 multimedia.secret_access_key_file"
                    .to_owned(),
            ));
        }
        // management.port must differ from the server.bind port.
        if let Some(port) = self.management.port
            && server_bind_port(&self.server.bind) == Some(port)
        {
            errors.push(ConfigError::semantic(format!(
                "management.port ({port}) must differ from the server.bind port"
            )));
        }
        // External terminology enabled ⇒ at least one provider.
        if self.terminology.external.enabled && self.terminology.external.providers.is_empty() {
            errors.push(ConfigError::semantic(
                "terminology.external.enabled = true requires at least one \
                 [terminology.external.providers.<name>]"
                    .to_owned(),
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigErrors(errors))
        }
    }

    /// The redacted TOML rendering (secrets show `***`) for `/management/env`
    /// and `ehrbase config check`.
    ///
    /// # Errors
    /// [`ConfigError`] if the tree cannot be serialized to TOML.
    pub fn to_redacted_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self)
            .map_err(|e| ConfigError::semantic(format!("rendering config as TOML: {e}")))
    }
}

/// The port component of a `host:port` bind string, if parseable.
fn server_bind_port(bind: &str) -> Option<u16> {
    bind.rsplit_once(':').and_then(|(_, p)| p.parse().ok())
}

/// Assemble the configuration from explicit inputs — the pure seam every test
/// drives (§5.1/§6.6). Runs the alias sweep (warning once per set legacy var),
/// the strict env + file passes, the layered merge, and `*_file` secret
/// resolution.
///
/// # Errors
/// [`ConfigErrors`] aggregating unknown-key, type, and file-resolution errors.
// A boot-once seam over the process environment — a custom hasher has no
// call site; the generic would be pure noise.
#[allow(clippy::implicit_hasher)]
pub fn assemble(
    file: Option<&Path>,
    env: &HashMap<String, String>,
    overrides: &[(String, String)],
) -> Result<EhrbaseConfig, ConfigErrors> {
    loader::assemble(file, env, overrides)
}

/// Boot loader: a thin process-environment shim over [`assemble`] (§5.2).
/// Discovers the config file (§5.4), snapshots the environment, assembles, and
/// emits the dev-default-DB boot warning (§3.16 review condition).
///
/// # Errors
/// [`ConfigErrors`] on discovery failure or any assembly error.
pub fn load(
    cli_config: Option<&Path>,
    overrides: &[(String, String)],
) -> Result<EhrbaseConfig, ConfigErrors> {
    let env: HashMap<String, String> = std::env::vars().collect();
    let file = loader::discover_file(cli_config, &env)?;
    let config = assemble(file.as_deref(), &env, overrides)?;

    // Review condition 1: never a silent production trap — announce the dev
    // default DSN prominently at boot (§3.16).
    if config.db.is_dev_default() {
        tracing::warn!(
            url = crate::db::DEFAULT_URL,
            "[db].url is the built-in DEVELOPMENT DEFAULT ({}); no file/env/CLI value was \
             supplied. Set db.url (or EHRBASE__DB__URL / DATABASE_URL) for any non-dev \
             deployment — production MUST override it.",
            crate::db::DEFAULT_URL,
        );
    }
    Ok(config)
}

/// The file discovery order, exposed for `config check`.
///
/// # Errors
/// [`ConfigErrors`] if an explicitly-pointed-at file is missing/unreadable.
// Boot-once seam — see `assemble` on the hasher generic.
#[allow(clippy::implicit_hasher)]
pub fn discover_file(
    cli_config: Option<&Path>,
    env: &HashMap<String, String>,
) -> Result<Option<PathBuf>, ConfigErrors> {
    loader::discover_file(cli_config, env)
}

#[cfg(test)]
mod tests {
    use assert_fs::prelude::*;

    use super::*;

    /// Build an injected env map from `(key, value)` pairs.
    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    /// A temp file carrying `content`, for the file-source cases.
    fn toml_file(content: &str) -> assert_fs::NamedTempFile {
        let f = assert_fs::NamedTempFile::new("ehrbase.toml").expect("temp");
        f.write_str(content).expect("write");
        f
    }

    fn assemble_ok(
        file: Option<&Path>,
        env: &HashMap<String, String>,
        overrides: &[(String, String)],
    ) -> EhrbaseConfig {
        match assemble(file, env, overrides) {
            Ok(c) => c,
            Err(e) => panic!("assemble failed: {e}"),
        }
    }

    fn json(c: &EhrbaseConfig) -> serde_json::Value {
        serde_json::to_value(c).expect("serialize")
    }

    // ── 1. Layering ──────────────────────────────────────────────────────────

    #[test]
    fn defaults_only_boot_equals_default() {
        let c = assemble_ok(None, &env(&[]), &[]);
        assert_eq!(json(&c), json(&EhrbaseConfig::default()));
    }

    #[test]
    fn layering_file_env_set() {
        let file = toml_file("[db]\nmax_connections = 5\n");
        // file overrides default (20).
        let c = assemble_ok(Some(file.path()), &env(&[]), &[]);
        assert_eq!(c.db.max_connections, 5);
        // env overrides file.
        let c = assemble_ok(
            Some(file.path()),
            &env(&[("EHRBASE__DB__MAX_CONNECTIONS", "9")]),
            &[],
        );
        assert_eq!(c.db.max_connections, 9);
        // --set overrides env.
        let c = assemble_ok(
            Some(file.path()),
            &env(&[("EHRBASE__DB__MAX_CONNECTIONS", "9")]),
            &[("db.max_connections".to_owned(), "11".to_owned())],
        );
        assert_eq!(c.db.max_connections, 11);
    }

    #[test]
    fn conventional_aliases_lose_to_ehrbase_forms() {
        // DATABASE_URL alone binds db.url.
        let c = assemble_ok(None, &env(&[("DATABASE_URL", "postgres://a@h/x")]), &[]);
        assert_eq!(c.db.url.expose(), "postgres://a@h/x");
        // EHRBASE__DB__URL wins over DATABASE_URL.
        let c = assemble_ok(
            None,
            &env(&[
                ("DATABASE_URL", "postgres://a@h/x"),
                ("EHRBASE__DB__URL", "postgres://b@h/y"),
            ]),
            &[],
        );
        assert_eq!(c.db.url.expose(), "postgres://b@h/y");
    }

    // ── 2. Mapping (the test class whose absence let the dead env form ship) ──

    #[test]
    fn env_mapping_scalars_maps_and_lists() {
        let c = assemble_ok(
            None,
            &env(&[
                ("EHRBASE__SERVER__MAX_IN_FLIGHT", "64"),
                ("EHRBASE__AUTH__OIDC__ISSUER", "https://idp"),
                ("EHRBASE__AUTH__OIDC__AUDIENCES", "ehrbase,other"),
                (
                    "EHRBASE__SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL",
                    "https://pas/r4",
                ),
            ]),
            &[],
        );
        assert_eq!(c.server.max_in_flight, 64);
        let oidc = c.auth.oidc.expect("oidc table materialised from env");
        assert_eq!(oidc.issuer, "https://idp");
        assert_eq!(
            oidc.audiences,
            vec!["ehrbase".to_owned(), "other".to_owned()]
        );
        assert_eq!(
            c.subject_proxy.systems.get("pas").expect("pas").base_url,
            "https://pas/r4"
        );
    }

    // ── 3. Strictness ─────────────────────────────────────────────────────────

    #[test]
    fn unknown_env_var_is_a_boot_error_with_suggestion() {
        let err = assemble(None, &env(&[("EHRBASE__SIGNIN__ENABLED", "true")]), &[])
            .expect_err("unknown var");
        let msg = err.to_string();
        assert!(msg.contains("EHRBASE__SIGNIN__ENABLED"), "{msg}");
        assert!(msg.contains("signing"), "did-you-mean missing: {msg}");
    }

    #[test]
    fn near_miss_prefix_suggests_the_uniform_spelling() {
        // The old mixed form (single `_` after the prefix) no longer binds; the
        // sweep names the exact uniform spelling.
        let err = assemble(None, &env(&[("EHRBASE_DB__URL", "postgres://x")]), &[])
            .expect_err("near-miss");
        let msg = err.to_string();
        assert!(msg.contains("EHRBASE_DB__URL"), "{msg}");
        assert!(
            msg.contains("EHRBASE__DB__URL"),
            "suggestion missing: {msg}"
        );
    }

    #[test]
    fn ehrbase_config_pointer_stays_accepted() {
        // `EHRBASE_CONFIG` is a file pointer, not a config key — it keeps its
        // single-`_` spelling and must never be flagged by the strict sweep.
        let c = assemble_ok(
            None,
            &env(&[("EHRBASE_CONFIG", "/etc/ehrbase/ehrbase.toml")]),
            &[],
        );
        assert_eq!(json(&c), json(&EhrbaseConfig::default()));
    }

    #[test]
    fn retired_config_pointer_is_a_boot_error() {
        // The per-subsystem `*_CONFIG` file pointers are gone (greenfield —
        // no legacy special-casing): the spelling fails as an unknown
        // reserved-namespace variable.
        let err = assemble(None, &env(&[("EHRBASE_SIGNING_CONFIG", "/x.toml")]), &[])
            .expect_err("retired pointer must fail");
        let msg = err.to_string();
        assert!(msg.contains("EHRBASE_SIGNING_CONFIG"), "{msg}");
    }

    #[test]
    fn unknown_file_key_is_rejected() {
        let file = toml_file("[db]\nmax_conections = 5\n");
        let err = assemble(Some(file.path()), &env(&[]), &[]).expect_err("unknown key");
        assert!(err.to_string().contains("max_conections"), "{err}");
    }

    // ── 4. No legacy (greenfield, owner ruling 2026-07-15) ──────────────────

    #[test]
    fn legacy_spellings_are_boot_errors_with_the_uniform_suggestion() {
        // A pre-redesign spelling is never silently honoured or remapped —
        // it fails at boot naming the exact uniform replacement.
        let err = assemble(None, &env(&[("EHRBASE_DB_MAX_CONNECTIONS", "7")]), &[])
            .expect_err("legacy spelling must fail");
        let msg = err.to_string();
        assert!(msg.contains("EHRBASE_DB_MAX_CONNECTIONS"), "{msg}");
        assert!(msg.contains("EHRBASE__DB__MAX_CONNECTIONS"), "{msg}");
    }

    // ── 5. Secrets ──────────────────────────────────────────────────────────

    #[test]
    fn secrets_never_render_and_file_resolves() {
        let file = toml_file("[auth.oidc]\nissuer = \"x\"\nhmac_secret = \"topsecret\"\n");
        let c = assemble_ok(Some(file.path()), &env(&[]), &[]);
        let rendered = c.to_redacted_toml().expect("toml");
        assert!(!rendered.contains("topsecret"), "secret leaked: {rendered}");
        assert!(!serde_json::to_string(&c).unwrap().contains("topsecret"));
        // *_file resolution.
        let secret = assert_fs::NamedTempFile::new("pass").expect("temp");
        secret.write_str("s3cret\n").expect("write");
        let file = toml_file(&format!(
            "[signing]\nkey_passphrase_file = \"{}\"\n",
            secret.path().display()
        ));
        let c = assemble_ok(Some(file.path()), &env(&[]), &[]);
        assert_eq!(
            c.signing
                .key_passphrase
                .as_ref()
                .expect("resolved")
                .expose(),
            "s3cret"
        );
    }

    #[test]
    fn secret_and_file_both_set_is_rejected() {
        let file =
            toml_file("[signing]\nkey_passphrase = \"a\"\nkey_passphrase_file = \"/dev/null\"\n");
        assert!(assemble(Some(file.path()), &env(&[]), &[]).is_err());
    }

    // ── 6. Template sync (§5.5) ───────────────────────────────────────────────

    #[test]
    fn template_parses_to_default() {
        let file = toml_file(DEFAULT_TEMPLATE);
        let c = assemble_ok(Some(file.path()), &env(&[]), &[]);
        assert_eq!(json(&c), json(&EhrbaseConfig::default()));
    }

    #[test]
    fn template_mentions_every_section() {
        for section in alias::SECTIONS {
            let header = format!("[{section}]");
            let dotted = format!("[{section}."); // sub-tables count too
            assert!(
                DEFAULT_TEMPLATE.contains(&header) || DEFAULT_TEMPLATE.contains(&dotted),
                "template missing section {section}"
            );
        }
    }

    // ── 7. Semantic validation (§5.8) ─────────────────────────────────────────

    #[test]
    fn validate_pgp_requires_key_path() {
        let mut c = EhrbaseConfig::default();
        c.signing.mode = crate::versioning::signature::Mode::Pgp;
        assert!(c.validate().is_err());
        c.signing.key_path = Some(std::path::PathBuf::from("/k.asc"));
        assert!(c.validate().is_ok());
    }

    #[test]
    fn validate_management_port_must_differ_from_bind() {
        let mut c = EhrbaseConfig::default();
        c.server.bind = "0.0.0.0:8080".to_owned();
        c.management.port = Some(8080);
        assert!(c.validate().is_err());
        c.management.port = Some(9100);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn validate_external_terminology_needs_a_provider() {
        let mut c = EhrbaseConfig::default();
        c.terminology.external.enabled = true;
        assert!(c.validate().is_err());
    }
}
