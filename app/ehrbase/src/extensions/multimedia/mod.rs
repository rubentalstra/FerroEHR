//! `DV_MULTIMEDIA` externalization to S3-compatible object storage.
//!
//! **No openEHR spec governs this — our own design/extension.** Server-side
//! blob storage is spec-silent (master13 is informative deployment guidance and
//! prescribes no blob-offload mechanism); this module fills it. Gate:
//! [`MultimediaConfig::enabled`] (`multimedia.enabled`, default off).
//!
//! On commit, an inline `DV_MULTIMEDIA.data` larger than a configured threshold
//! is written to a content-addressed blob store (keyed by its SHA-256) and the
//! canonical JSON is rewritten to reference it by `uri`, carrying the RM
//! integrity fields and the mandatory unencoded `size`. On read it can be
//! transparently re-inlined (`?expand_multimedia=true`), verifying the SHA-256
//! before serving.
//!
//! **Off by default** ([`MultimediaConfig::enabled`] = `false`): with the
//! feature disabled nothing here is constructed and the commit/read paths are
//! byte-identical to inline behaviour (the zero-drift gate).
//!
//! Spec basis for the *data shape* it rewrites: RM 1.2.0 `DV_MULTIMEDIA`
//! (`uri`/`data` alternatives under `is_inline or is_external`;
//! `integrity_check` ⇒ `integrity_check_algorithm` from the openEHR `Integrity
//! check algorithms` code set; mandatory unencoded `size`).
//!
//! ## Seams to versioning / storage
//! The engine is attached to the service via
//! `EhrbaseService::with_multimedia(...)` and consumed on the commit path
//! (offload, via the versioning `SigningCtx`) and the read path (expand). Those
//! call sites live in `crate::versioning` / `crate::service::ehr`; this module
//! only owns the engine + transforms.
//!
//! ## Module map
//! - `config` — the [`MultimediaConfig`] section struct.
//! - `store` — the content-addressed [`BlobStore`] over `object_store`.
//! - `offload` — the pure canonical-JSON transforms (externalize / expand).

// openEHR/product identifiers (DV_MULTIMEDIA, SeaweedFS, …) read as prose in docs.
#![allow(clippy::doc_markdown)]

use std::collections::HashMap;

use serde_json::Value;

pub mod config;
mod offload;
pub mod store;

use crate::extensions::multimedia::config::MultimediaConfig;
use crate::extensions::multimedia::store::BlobStore;

/// A failure in the multimedia externalization path.
#[derive(Debug, thiserror::Error)]
pub enum MultimediaError {
    /// The object store rejected an operation.
    #[error("blob store: {0}")]
    Store(#[from] object_store::Error),
    /// A stored blob's bytes do not hash to their content-addressed key — the
    /// integrity check failed.
    #[error("multimedia integrity check failed: expected sha-256 {expected}, got {actual}")]
    Integrity {
        /// The expected SHA-256 (the blob key).
        expected: String,
        /// The SHA-256 actually computed over the fetched bytes.
        actual: String,
    },
    /// A `DV_MULTIMEDIA` value was structurally malformed (e.g. `data` not
    /// base64, or size out of range).
    #[error("malformed multimedia value: {0}")]
    Malformed(String),
    /// The object-store client could not be constructed from configuration.
    #[error("multimedia store configuration: {0}")]
    Config(String),
}

/// The runtime engine bundling the blob store with its offload threshold. Held
/// by the service only when externalization is enabled.
#[derive(Debug, Clone)]
pub struct MultimediaEngine {
    store: BlobStore,
    threshold: usize,
}

impl MultimediaEngine {
    /// Build the engine from configuration, or `None` when the feature is
    /// disabled (`enabled = false`).
    ///
    /// # Errors
    /// Returns [`MultimediaError::Config`] if enabled but the object-store
    /// client cannot be built.
    pub fn from_config(cfg: &MultimediaConfig) -> Result<Option<Self>, MultimediaError> {
        if !cfg.enabled {
            return Ok(None);
        }
        Ok(Some(Self {
            store: BlobStore::from_config(cfg)?,
            threshold: cfg.threshold_bytes,
        }))
    }

    /// Construct directly from a store + threshold (test seam).
    #[must_use]
    pub fn from_parts(store: BlobStore, threshold: usize) -> Self {
        Self { store, threshold }
    }

    /// The underlying content-addressed blob store.
    #[must_use]
    pub fn store(&self) -> &BlobStore {
        &self.store
    }

    /// Externalize every qualifying inline `DV_MULTIMEDIA` in `canonical`,
    /// uploading the blobs. Rewrites `canonical` in place; a below-threshold or
    /// already-external value is left untouched.
    ///
    /// # Errors
    /// [`MultimediaError::Malformed`] when an inline `data` is not base64 or
    /// exceeds the i64 byte range, or [`MultimediaError::Store`] on an upload
    /// failure (the caller aborts the commit, so nothing is persisted on
    /// error).
    pub async fn offload(&self, canonical: &mut Value) -> Result<(), MultimediaError> {
        let pending = offload::plan_offload(canonical, self.threshold, &self.store)?;
        for (hex, bytes) in pending {
            self.store.put_if_absent(&hex, bytes).await?;
        }
        Ok(())
    }

    /// Re-inline every externalized `DV_MULTIMEDIA` (of *our* blobs) in
    /// `canonical`, verifying each blob's SHA-256 before use. A value already
    /// inline, or referencing a foreign URI, is left as-is.
    ///
    /// # Errors
    /// [`MultimediaError::Integrity`] on a hash mismatch, or
    /// [`MultimediaError::Store`] when a referenced blob is missing or the
    /// backend fails.
    pub async fn expand(&self, canonical: &mut Value) -> Result<(), MultimediaError> {
        let mut keys = offload::collect_expand_keys(canonical, &self.store);
        keys.sort_unstable();
        keys.dedup();
        if keys.is_empty() {
            return Ok(());
        }
        let mut fetched: HashMap<String, String> = HashMap::with_capacity(keys.len());
        for key in keys {
            let bytes = self.store.get(&key).await?;
            let b64_data = offload::verify_and_encode(&key, &bytes)?;
            fetched.insert(key, b64_data);
        }
        offload::apply_expand(canonical, &fetched, &self.store);
        Ok(())
    }

    /// The blob keys of *our* externalized media referenced anywhere in
    /// `canonical` (for GC and dump/load).
    #[must_use]
    pub fn referenced_keys(&self, canonical: &Value) -> Vec<String> {
        offload::referenced_keys(canonical, &self.store)
    }
}
