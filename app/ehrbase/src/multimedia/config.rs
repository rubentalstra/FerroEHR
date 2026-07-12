//! Configuration for `DV_MULTIMEDIA` externalization.
//!
//! Loaded independently of the rest of the service (the sibling
//! `fhir_outbound`/`events`/`signing` config pattern): a `figment` merge of
//! built-in defaults ← an optional TOML file (`EHRBASE_MULTIMEDIA_CONFIG`) ←
//! `EHRBASE_MULTIMEDIA_*` environment variables.
//!
//! **Off by default** (`enabled = false`): with externalization disabled the
//! commit/read paths are byte-identical to today's inline behaviour (the offload
//! §1, the zero-drift gate) and no object store is ever contacted.

// openEHR/product identifiers (SeaweedFS, object_store, …) read as prose in the
// config docs below.
#![allow(clippy::doc_markdown)]

use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

/// Default offload threshold: a decoded (unencoded) `DV_MULTIMEDIA.data`
/// larger than this is externalized. 256 KiB.
pub const DEFAULT_THRESHOLD_BYTES: usize = 256 * 1024;

/// The default bucket name when a deployment does not set one.
pub const DEFAULT_BUCKET: &str = "openehr-multimedia";

/// DV_MULTIMEDIA externalization settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimediaConfig {
    /// Master switch. `false` (default) = today's inline behaviour, byte for
    /// byte; no object store is built or contacted.
    #[serde(default)]
    pub enabled: bool,
    /// A decoded `DV_MULTIMEDIA.data` strictly larger than this many bytes is
    /// offloaded to the object store; at or below it stays inline.
    #[serde(default = "defaults::threshold_bytes")]
    pub threshold_bytes: usize,
    /// S3-compatible endpoint URL (e.g. a SeaweedFS S3 gateway
    /// `http://127.0.0.1:8333` in dev/test, or an AWS/MinIO endpoint in prod).
    /// `None` uses the object_store default AWS endpoint resolution.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Target bucket for content-addressed blobs.
    #[serde(default = "defaults::bucket")]
    pub bucket: String,
    /// AWS region (S3 requires one even for non-AWS endpoints).
    #[serde(default = "defaults::region")]
    pub region: String,
    /// Access key id. `None` (with `secret_access_key` also `None`) runs the
    /// client unsigned/anonymous — the mode a keyless dev SeaweedFS accepts.
    #[serde(default)]
    pub access_key_id: Option<String>,
    /// Secret access key (paired with `access_key_id`).
    #[serde(default)]
    pub secret_access_key: Option<String>,
    /// Allow plain-HTTP endpoints (dev/test only — a SeaweedFS container speaks
    /// HTTP). Production S3 is HTTPS, so this stays `false` there.
    #[serde(default)]
    pub allow_http: bool,
}

impl Default for MultimediaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_bytes: DEFAULT_THRESHOLD_BYTES,
            endpoint: None,
            bucket: DEFAULT_BUCKET.to_owned(),
            region: defaults::region(),
            access_key_id: None,
            secret_access_key: None,
            allow_http: false,
        }
    }
}

impl MultimediaConfig {
    /// Load from defaults ← optional TOML file ← `EHRBASE_MULTIMEDIA_*` env.
    ///
    /// # Errors
    /// Propagates a `figment` extraction error on malformed configuration.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(MultimediaConfig::default()));
        if let Ok(path) = std::env::var("EHRBASE_MULTIMEDIA_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_MULTIMEDIA_").split("__"))
            .extract()
    }

    /// Whether the client should run unsigned/anonymous (no credentials given).
    #[must_use]
    pub fn is_anonymous(&self) -> bool {
        self.access_key_id.is_none() && self.secret_access_key.is_none()
    }
}

mod defaults {
    use super::{DEFAULT_BUCKET, DEFAULT_THRESHOLD_BYTES};

    pub(super) fn threshold_bytes() -> usize {
        DEFAULT_THRESHOLD_BYTES
    }
    pub(super) fn bucket() -> String {
        DEFAULT_BUCKET.to_owned()
    }
    pub(super) fn region() -> String {
        "us-east-1".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_disabled_with_256kib_threshold() {
        let c = MultimediaConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.threshold_bytes, 256 * 1024);
        assert_eq!(c.bucket, DEFAULT_BUCKET);
        assert!(c.is_anonymous());
        assert!(!c.allow_http);
    }

    #[test]
    fn load_defaults_when_no_env() {
        // No EHRBASE_MULTIMEDIA_* set in the default test env.
        let c = MultimediaConfig::load().expect("load defaults");
        assert!(!c.enabled);
        assert_eq!(c.threshold_bytes, DEFAULT_THRESHOLD_BYTES);
    }
}
