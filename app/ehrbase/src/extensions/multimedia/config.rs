//! The `[multimedia]` section — `DV_MULTIMEDIA` externalization.
//!
//! **No openEHR spec governs this — our own design/extension.** A field of the
//! one config tree ([`crate::config::EhrbaseConfig`],
//! `docs/design/configuration.md` §3.15); no loader of its own.
//!
//! **Off by default** (`enabled = false`): with externalization disabled the
//! commit/read paths are byte-identical to today's inline behaviour and no
//! object store is ever contacted. The secret access key is a shared
//! [`crate::config::Secret`] (never rendered) with a `*_file` sibling.

// openEHR/product identifiers (SeaweedFS, object_store, …) read as prose below.
#![allow(clippy::doc_markdown)]

use std::path::PathBuf;

use crate::config::Secret;
use serde::{Deserialize, Serialize};

/// Default offload threshold: a decoded (unencoded) `DV_MULTIMEDIA.data`
/// larger than this is externalized. 256 KiB.
pub const DEFAULT_THRESHOLD_BYTES: usize = 256 * 1024;

/// The default bucket name when a deployment does not set one.
pub const DEFAULT_BUCKET: &str = "openehr-multimedia";

/// DV_MULTIMEDIA externalization settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MultimediaConfig {
    /// Master switch. `false` (default) = today's inline behaviour, byte for
    /// byte; no object store is built or contacted.
    pub enabled: bool,
    /// A decoded `DV_MULTIMEDIA.data` strictly larger than this many bytes is
    /// offloaded to the object store; at or below it stays inline.
    pub threshold_bytes: usize,
    /// S3-compatible endpoint URL (e.g. a SeaweedFS S3 gateway in dev/test, or
    /// an AWS/MinIO endpoint in prod). `None` uses the object_store default AWS
    /// endpoint resolution.
    pub endpoint: Option<String>,
    /// Target bucket for content-addressed blobs.
    pub bucket: String,
    /// AWS region (S3 requires one even for non-AWS endpoints).
    pub region: String,
    /// Access key id. `None` (with `secret_access_key` also `None`) runs the
    /// client unsigned/anonymous — the mode a keyless dev SeaweedFS accepts.
    pub access_key_id: Option<String>,
    /// Secret access key (paired with `access_key_id`); never rendered.
    pub secret_access_key: Option<Secret>,
    /// File-based indirection for [`Self::secret_access_key`] (K8s/Docker
    /// secrets). Exactly one of the pair may be set; the loader reads and trims
    /// the file.
    pub secret_access_key_file: Option<PathBuf>,
    /// Allow plain-HTTP endpoints (dev/test only — a SeaweedFS container speaks
    /// HTTP). Production S3 is HTTPS, so this stays `false` there.
    pub allow_http: bool,
}

impl Default for MultimediaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_bytes: DEFAULT_THRESHOLD_BYTES,
            endpoint: None,
            bucket: DEFAULT_BUCKET.to_owned(),
            region: "us-east-1".to_owned(),
            access_key_id: None,
            secret_access_key: None,
            secret_access_key_file: None,
            allow_http: false,
        }
    }
}

impl MultimediaConfig {
    /// Whether the client should run unsigned/anonymous (no credentials given).
    #[must_use]
    pub fn is_anonymous(&self) -> bool {
        self.access_key_id.is_none() && self.secret_access_key.is_none()
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
}
