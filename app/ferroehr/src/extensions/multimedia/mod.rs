//! The `[multimedia]` config section + the feature-gated glue onto the
//! extension crate's externalization engine.
//!
//! **No openEHR spec governs this — our own design/extension.** The engine,
//! blob store, and canonical-JSON transforms live in
//! `ferroehr_ext::multimedia` (the `multimedia` cargo feature); this module
//! keeps the serde config section in the platform's one config tree and maps
//! it onto the extension's runtime parameters.

pub mod config;

use crate::extensions::multimedia::config::MultimediaConfig;

/// Build the externalization engine from configuration: `None` when
/// `multimedia.enabled` is `false`.
///
/// # Errors
/// Returns `MultimediaError::Config` when enabled but the object-store
/// client cannot be built.
#[cfg(feature = "multimedia")]
pub fn engine_from_config(
    cfg: &MultimediaConfig,
) -> Result<Option<ferroehr_ext::multimedia::MultimediaEngine>, ferroehr_ext::multimedia::MultimediaError>
{
    if !cfg.enabled {
        return Ok(None);
    }
    let params = ferroehr_ext::multimedia::store::BlobStoreParams {
        endpoint: cfg.endpoint.clone(),
        bucket: cfg.bucket.clone(),
        region: cfg.region.clone(),
        access_key_id: cfg.access_key_id.clone(),
        secret_access_key: cfg
            .secret_access_key
            .as_ref()
            .map(|s| secrecy::SecretString::from(s.expose().to_owned())),
        allow_http: cfg.allow_http,
    };
    ferroehr_ext::multimedia::MultimediaEngine::from_params(params, cfg.threshold_bytes).map(Some)
}

/// The loud slim-build refusal: a configuration that enables externalization
/// on a binary compiled without the `multimedia` feature is a boot error,
/// never a silent ignore.
#[cfg(not(feature = "multimedia"))]
pub fn require_disabled(cfg: &MultimediaConfig) -> Result<(), String> {
    if cfg.enabled {
        return Err(
            "multimedia.enabled = true, but this binary was built without the \
             `multimedia` cargo feature"
                .to_owned(),
        );
    }
    Ok(())
}
