//! A per-EHR cache of parsed `EHR_ACCESS` scheme settings.
//!
//! The `EHR_ACCESS` gateway clause ("All access decisions to data in the EHR
//! must be made in accordance with the policies and rules in this object" — RM
//! `org.openehr.rm.ehr.ehr_access.adoc`) is consulted on **every** EHR-scoped
//! request, so the current `EHR_ACCESS` version's settings are cached rather
//! than re-read + re-decomposed per request. Keyed by `ehr_id`; a value of
//! `None` (absent settings / another scheme) is cached too. Invalidated
//! whenever an `EHR_ACCESS` version is committed (the settings are
//! change-controlled — RM ehr `master04-ehr_package.adoc` §EHR Access).
//!
//! No openEHR spec governs this cache — our own design/extension (storage
//! mechanics; `docs/design/ehr-access-scheme.md`). Mirrors the
//! `openehr_flat::cache::WebTemplateCache` `moka::future::Cache` usage.

use std::sync::Arc;

use ehrbase_sm::{EhrAccessSettings, SmError};
use moka::future::Cache;
use uuid::Uuid;

/// A shared, cloneable cache of the current `EHR_ACCESS` scheme settings per
/// EHR. `moka`'s `Cache` is `Arc`-backed, so every clone of the owning service
/// shares one cache (matching the `WebTemplate` cache).
#[derive(Debug, Clone)]
pub(super) struct EhrAccessCache {
    inner: Cache<Uuid, Arc<Option<EhrAccessSettings>>>,
}

impl EhrAccessCache {
    /// A cache holding up to `capacity` EHRs' settings.
    fn new(capacity: u64) -> Self {
        Self {
            inner: Cache::builder().max_capacity(capacity).build(),
        }
    }

    /// The cached settings for `ehr_id`, or load them via `init` (run at most
    /// once per key under contention) and cache the result.
    ///
    /// # Errors
    /// Propagates the `init` error (shared across concurrent callers as an
    /// `Arc<SmError>`).
    pub(super) async fn get_or_load<Fut>(
        &self,
        ehr_id: Uuid,
        init: Fut,
    ) -> Result<Arc<Option<EhrAccessSettings>>, Arc<SmError>>
    where
        Fut: std::future::Future<Output = Result<Option<EhrAccessSettings>, SmError>>,
    {
        self.inner
            .try_get_with(ehr_id, async move { init.await.map(Arc::new) })
            .await
    }

    /// Drop the cached settings for `ehr_id` — called on every `EHR_ACCESS`
    /// commit so the next read reflects the new version.
    pub(super) async fn invalidate(&self, ehr_id: Uuid) {
        self.inner.invalidate(&ehr_id).await;
    }
}

impl Default for EhrAccessCache {
    fn default() -> Self {
        Self::new(4096)
    }
}
