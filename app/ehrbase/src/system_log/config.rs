//! Audit configuration ([`AuditConfig`]) — a `figment`-loaded serde struct.
//!
//! No openEHR spec governs configuration — the `EHRBASE_ATNA_`-prefixed env key
//! set is our own design (the non-normative design record
//! `docs/enterprise/atna-audit.md` tabulates it). Loading mirrors the
//! `ehrbase-rest` `RestConfig` pattern: defaults ← optional TOML file
//! (`EHRBASE_ATNA_CONFIG`) ← `EHRBASE_ATNA_`-prefixed environment.

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};

/// The syslog transport to the Audit Record Repository (ARR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// RFC 5426 UDP (the reference Elastic/Logstash stack default, port 514).
    #[default]
    Udp,
    /// RFC 5425 TLS (the IHE-recommended secure transport).
    Tls,
}

/// Behaviour when an audit record cannot be enqueued/delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FailMode {
    /// Log + meter the drop and let the request succeed (common ATNA default).
    #[default]
    Open,
    /// Reject auditable operations with `503` when auditing cannot be delivered.
    Closed,
}

/// ATNA audit configuration. Every field has a default, so an all-defaults
/// [`AuditConfig`] is valid (auditing is off unless `enabled`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Master switch (`EHRBASE_ATNA_ENABLED`).
    #[serde(default)]
    pub enabled: bool,
    /// Enterprise/tenant id → `AuditEnterpriseSiteID` (`EHRBASE_ATNA_ENTERPRISE_SITE_ID`).
    #[serde(default)]
    pub enterprise_site_id: Option<String>,
    /// ARR host (`EHRBASE_ATNA_REPOSITORY_HOST`).
    #[serde(default = "defaults::host")]
    pub repository_host: String,
    /// ARR port (`EHRBASE_ATNA_REPOSITORY_PORT`).
    #[serde(default = "defaults::port")]
    pub repository_port: u16,
    /// Transport (`EHRBASE_ATNA_TRANSPORT`): `udp` | `tls`.
    #[serde(default)]
    pub transport: Transport,
    /// Audit source id → `AuditSourceID` and the destination `UserID`
    /// (`EHRBASE_ATNA_SOURCE_ID`).
    #[serde(default = "defaults::source_id")]
    pub source_id: String,
    /// Fill value for empty mandatory fields (`EHRBASE_ATNA_VALUE_IF_MISSING`).
    #[serde(default = "defaults::value_if_missing")]
    pub value_if_missing: String,
    /// Skip auth/login "Application Activity" events (`EHRBASE_ATNA_SUPPRESS_LOGIN_EVENTS`).
    #[serde(default = "defaults::yes")]
    pub suppress_login_events: bool,
    /// Failure mode (`EHRBASE_ATNA_FAIL_MODE`): `open` | `closed`.
    #[serde(default)]
    pub fail_mode: FailMode,
    /// Enrich the Patient-Number participant object via a background indexed
    /// lookup of `ehr.subject_id` (`EHRBASE_ATNA_RESOLVE_SUBJECT`). Off by
    /// default; the binary supplies the resolver.
    #[serde(default)]
    pub resolve_subject: bool,
    /// Bounded audit queue capacity (`EHRBASE_ATNA_QUEUE_CAPACITY`).
    #[serde(default = "defaults::queue_capacity")]
    pub queue_capacity: usize,
    /// This node's advertised network address → the destination
    /// `NetworkAccessPointID` (`EHRBASE_ATNA_SERVER_HOST`). Defaults to the
    /// `repository_host`-facing best guess is left to the binary; when unset the
    /// `value_if_missing` fill is used.
    #[serde(default)]
    pub server_host: Option<String>,
    /// PEM file with the ARR CA to trust for TLS (`EHRBASE_ATNA_TLS_CA_PATH`).
    #[serde(default)]
    pub tls_ca_path: Option<String>,
    /// Client-certificate PEM for mutual TLS (`EHRBASE_ATNA_TLS_IDENTITY_CERT_PATH`).
    #[serde(default)]
    pub tls_identity_cert_path: Option<String>,
    /// Client-key PEM for mutual TLS (`EHRBASE_ATNA_TLS_IDENTITY_KEY_PATH`).
    #[serde(default)]
    pub tls_identity_key_path: Option<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            enterprise_site_id: None,
            repository_host: defaults::host(),
            repository_port: defaults::port(),
            transport: Transport::default(),
            source_id: defaults::source_id(),
            value_if_missing: defaults::value_if_missing(),
            suppress_login_events: defaults::yes(),
            fail_mode: FailMode::default(),
            resolve_subject: false,
            queue_capacity: defaults::queue_capacity(),
            server_host: None,
            tls_ca_path: None,
            tls_identity_cert_path: None,
            tls_identity_key_path: None,
        }
    }
}

impl AuditConfig {
    /// Load configuration: defaults, then an optional TOML file (path in
    /// `EHRBASE_ATNA_CONFIG`), then `EHRBASE_ATNA_`-prefixed environment
    /// variables (nested keys use `__`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(AuditConfig::default()));
        if let Ok(path) = std::env::var("EHRBASE_ATNA_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_ATNA_").split("__"))
            .extract()
    }
}

mod defaults {
    pub(super) fn host() -> String {
        "localhost".to_owned()
    }
    pub(super) const fn port() -> u16 {
        514
    }
    pub(super) fn source_id() -> String {
        "ehrbase".to_owned()
    }
    pub(super) fn value_if_missing() -> String {
        "UNKNOWN".to_owned()
    }
    pub(super) const fn yes() -> bool {
        true
    }
    pub(super) const fn queue_capacity() -> usize {
        1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = AuditConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.repository_port, 514);
        assert_eq!(c.transport, Transport::Udp);
        assert_eq!(c.value_if_missing, "UNKNOWN");
        assert!(c.suppress_login_events);
        assert_eq!(c.fail_mode, FailMode::Open);
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn env_overrides_apply() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_ATNA_ENABLED", "true");
            jail.set_env("EHRBASE_ATNA_TRANSPORT", "tls");
            jail.set_env("EHRBASE_ATNA_REPOSITORY_PORT", "6514");
            jail.set_env("EHRBASE_ATNA_FAIL_MODE", "closed");
            jail.set_env("EHRBASE_ATNA_SUPPRESS_LOGIN_EVENTS", "false");
            let c = AuditConfig::load().expect("load");
            assert!(c.enabled);
            assert_eq!(c.transport, Transport::Tls);
            assert_eq!(c.repository_port, 6514);
            assert_eq!(c.fail_mode, FailMode::Closed);
            assert!(!c.suppress_login_events);
            Ok(())
        });
    }
}
