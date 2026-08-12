// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Audit configuration ([`AuditConfig`]) — the `[audit]` section of the one
//! config tree ([`crate::config::FerroEhrConfig`]); no loader of its own.
//!
//! No openEHR spec governs configuration — our own design. The tree is
//! sink-structured: the shared event/queue settings at the root, one
//! sub-table per sink — `[audit.store]` (the local Audit Record Repository,
//! the durability anchor, **on by default**), `[audit.syslog]` (the classic
//! IHE ITI-20 DICOM-over-syslog feed, opt-in), `[audit.fhir_feed]` (the
//! RESTful-ATNA ITI-20 ATX:FHIR Feed, opt-in). Auditing itself is **on by
//! default** with only the local store active: compliance out of the box,
//! nothing leaves the node.

use serde::{Deserialize, Serialize};

use crate::config::secret::SecretUrl;

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
    /// Reject auditable operations with `503` when auditing cannot be
    /// delivered — a full queue, or (with the store on) a store that stopped
    /// accepting writes. No un-audited PHI access.
    Closed,
}

/// The local Audit Record Repository (`[audit.store]`) — the PG-backed store
/// ([`crate::system_log::store`]), the durability anchor of the subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StoreConfig {
    /// Persist every record locally (`FERROEHR__AUDIT__STORE__ENABLED`).
    /// **On by default** (owner posture: compliance out of the box).
    pub enabled: bool,
    /// Days to keep records; `0` = keep forever
    /// (`FERROEHR__AUDIT__STORE__RETENTION_DAYS`). Applied hourly by the
    /// retention reaper.
    pub retention_days: u32,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: 0,
        }
    }
}

/// The classic ATNA feed (`[audit.syslog]`): the DICOM PS3.15 §A.5 XML
/// record over syslog (IHE ITI TF-2 ITI-20; RFC 5424 message, RFC 5426 UDP
/// or RFC 5425 TLS transport).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SyslogConfig {
    /// Ship records to an external ARR over syslog
    /// (`FERROEHR__AUDIT__SYSLOG__ENABLED`).
    pub enabled: bool,
    /// ARR host (`FERROEHR__AUDIT__SYSLOG__HOST`).
    pub host: String,
    /// ARR port (`FERROEHR__AUDIT__SYSLOG__PORT`).
    pub port: u16,
    /// Transport (`FERROEHR__AUDIT__SYSLOG__TRANSPORT`): `udp` | `tls`.
    pub transport: Transport,
    /// PEM file with the ARR CA to trust for TLS
    /// (`FERROEHR__AUDIT__SYSLOG__TLS_CA_FILE`).
    pub tls_ca_file: Option<String>,
    /// Client-certificate PEM file for mutual TLS
    /// (`FERROEHR__AUDIT__SYSLOG__TLS_IDENTITY_CERT_FILE`).
    pub tls_identity_cert_file: Option<String>,
    /// Client-key PEM file for mutual TLS
    /// (`FERROEHR__AUDIT__SYSLOG__TLS_IDENTITY_KEY_FILE`).
    pub tls_identity_key_file: Option<String>,
}

impl Default for SyslogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "localhost".to_owned(),
            port: 514,
            transport: Transport::default(),
            tls_ca_file: None,
            tls_identity_cert_file: None,
            tls_identity_key_file: None,
        }
    }
}

/// The RESTful-ATNA feed (`[audit.fhir_feed]`): ITI-20 **ATX: FHIR Feed** —
/// HTTP `POST {url}/AuditEvent` of the FHIR R4B `AuditEvent` (IHE BALP shape)
/// to an external Audit Record Repository.
///
/// When the local store is on, the feed drains the store's outbox
/// (`delivered_fhir_feed_at IS NULL`), so a down ARR loses nothing; with the
/// store off it ships in-drain with bounded retries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FhirFeedConfig {
    /// Ship records to an external FHIR ARR
    /// (`FERROEHR__AUDIT__FHIR_FEED__ENABLED`).
    pub enabled: bool,
    /// The ARR's FHIR base URL (`FERROEHR__AUDIT__FHIR_FEED__URL`); the
    /// `AuditEvent` endpoint is `{url}/AuditEvent`. Credentials in the URL
    /// (basic auth) are redacted from every rendering.
    pub url: SecretUrl,
    /// Outbox rows shipped per poll (`FERROEHR__AUDIT__FHIR_FEED__BATCH_SIZE`).
    pub batch_size: i64,
    /// Outbox poll interval when idle, in milliseconds
    /// (`FERROEHR__AUDIT__FHIR_FEED__POLL_INTERVAL_MS`).
    pub poll_interval_ms: u64,
    /// Per-record POST retries before the record is left pending (store on)
    /// or dropped + metered (store off)
    /// (`FERROEHR__AUDIT__FHIR_FEED__MAX_RETRIES`).
    pub max_retries: usize,
}

impl Default for FhirFeedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: SecretUrl::new("http://localhost:8080/fhir"),
            batch_size: 64,
            poll_interval_ms: 2000,
            max_retries: 3,
        }
    }
}

/// ATNA audit configuration (`[audit]`). Every field has a default; the
/// all-defaults tree is **auditing on with only the local store active**.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuditConfig {
    /// Master switch (`FERROEHR__AUDIT__ENABLED`). **On by default** — every
    /// deployment gets a queryable audit trail with zero external
    /// dependencies (the sinks decide where records go).
    pub enabled: bool,
    /// Enterprise/site id → `AuditEnterpriseSiteID`
    /// (`FERROEHR__AUDIT__ENTERPRISE_SITE_ID`).
    pub enterprise_site_id: Option<String>,
    /// Audit source id → `AuditSourceID` and the destination participant
    /// (`FERROEHR__AUDIT__SOURCE_ID`).
    pub source_id: String,
    /// Fill value for empty mandatory fields
    /// (`FERROEHR__AUDIT__VALUE_IF_MISSING`).
    pub value_if_missing: String,
    /// Skip successful-login records (`FERROEHR__AUDIT__SUPPRESS_LOGIN_EVENTS`).
    /// Rejected accesses (401/403) are always recorded.
    pub suppress_login_events: bool,
    /// Failure mode (`FERROEHR__AUDIT__FAIL_MODE`): `open` | `closed`.
    pub fail_mode: FailMode,
    /// Enrich the patient participant via a background indexed lookup of
    /// `ehr.subject_id` (`FERROEHR__AUDIT__RESOLVE_SUBJECT`). On by default —
    /// the IHE BALP `Patient*` patterns and the patient-centric audit search
    /// need the subject; the lookup runs only on the background drain.
    pub resolve_subject: bool,
    /// Bounded audit queue capacity (`FERROEHR__AUDIT__QUEUE_CAPACITY`).
    /// Sized for write-path bursts: the drain persists in multi-row batches,
    /// so the queue only needs to ride out sink latency spikes, but a loaded
    /// write path can enqueue thousands per second.
    pub queue_capacity: usize,
    /// This node's advertised network address → the destination
    /// network-access-point (`FERROEHR__AUDIT__SERVER_HOST`); the
    /// `value_if_missing` fill when unset.
    pub server_host: Option<String>,
    /// `[audit.store]` — the local Audit Record Repository.
    pub store: StoreConfig,
    /// `[audit.syslog]` — the classic DICOM-over-syslog feed.
    pub syslog: SyslogConfig,
    /// `[audit.fhir_feed]` — the RESTful-ATNA FHIR `AuditEvent` feed.
    pub fhir_feed: FhirFeedConfig,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enterprise_site_id: None,
            source_id: "ferroehr".to_owned(),
            value_if_missing: "UNKNOWN".to_owned(),
            suppress_login_events: true,
            fail_mode: FailMode::default(),
            resolve_subject: true,
            queue_capacity: 8192,
            server_host: None,
            store: StoreConfig::default(),
            syslog: SyslogConfig::default(),
            fhir_feed: FhirFeedConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_on_with_store_only() {
        let c = AuditConfig::default();
        assert!(c.enabled, "auditing is on by default (owner posture)");
        assert!(c.store.enabled, "the local store is the default sink");
        assert_eq!(c.store.retention_days, 0, "keep forever by default");
        assert!(!c.syslog.enabled, "forwarding is opt-in");
        assert!(!c.fhir_feed.enabled, "forwarding is opt-in");
        assert_eq!(c.syslog.port, 514);
        assert_eq!(c.syslog.transport, Transport::Udp);
        assert_eq!(c.value_if_missing, "UNKNOWN");
        assert!(c.suppress_login_events);
        assert!(c.resolve_subject);
        assert_eq!(c.fail_mode, FailMode::Open);
        assert_eq!(c.fhir_feed.batch_size, 64);
    }
}
