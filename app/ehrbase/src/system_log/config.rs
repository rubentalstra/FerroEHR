//! Audit configuration ([`AuditConfig`]) — a `figment`-loaded serde struct.
//!
//! No openEHR spec governs configuration — our own design. This is the `[atna]`
//! section of the one config tree ([`crate::config::EhrbaseConfig`],
//! `docs/design/configuration.md` §3.15); no loader of its own.

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
#[serde(default, deny_unknown_fields)]
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

mod defaults {
    fn host() -> String {
        "localhost".to_owned()
    }
    const fn port() -> u16 {
        514
    }
    fn source_id() -> String {
        "ehrbase".to_owned()
    }
    fn value_if_missing() -> String {
        "UNKNOWN".to_owned()
    }
    const fn yes() -> bool {
        true
    }
    const fn queue_capacity() -> usize {
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
}
