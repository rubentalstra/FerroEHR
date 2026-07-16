//! `EHR_ACCESS` — the EHR-wide access-control top-level structure (arch-overview
//! `master06-design_of_the_ehr.adoc` §`EHR_ACCESS`; RM ehr
//! `org.openehr.rm.ehr.ehr_access.adoc`). This file owns the object's default +
//! validation (created under the EHR-creation CONTRIBUTION — there is no direct
//! ITS-REST `EHR_ACCESS` write) and carries the per-EHR scheme cache.
//!
//! The `EhrAccessCache` mechanics are spec-silent — no openEHR spec governs a
//! per-request cache of `EHR_ACCESS` settings (our own design/extension). The
//! cache rides along here (cross-register ruling) but the access-control *gate*
//! that consults it (RBAC/ABAC) is a Stage-2 enterprise concern (CLAUDE.md), not
//! designed in this chapter.

use std::sync::Arc;

use crate::service::ehr::access_types::EhrAccessSettings;
use crate::service::status::SmError;
use moka::future::Cache;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{Kind, read_current};

impl EhrbaseService {
    /// Drop the cached `EHR_ACCESS` settings for `ehr_id` — the
    /// [`crate::versioning::CommitEnv`] `invalidate_ehr_access` hook, called after
    /// any `EHR_ACCESS` commit so the next access decision reflects the new
    /// version (the settings are change-controlled — RM ehr master04 §EHR Access).
    pub(in crate::service) async fn invalidate_ehr_access(&self, ehr_id: Uuid) {
        self.ehr_access.invalidate(ehr_id).await;
    }

    /// Pre-warm the `EHR_ACCESS` cache for a just-created EHR as default-open
    /// (`None`). Every EHR is created with the settings-less
    /// [`super::default_ehr_access`] (there is no direct `EHR_ACCESS` write —
    /// RM ehr master04 §EHR Access), so a fresh EHR is unconditionally
    /// default-open; seeding that entry saves the first-access DB miss. A later
    /// `EHR_ACCESS` commit evicts it through [`Self::invalidate_ehr_access`].
    pub(in crate::service) async fn prewarm_ehr_access_open(&self, ehr_id: Uuid) {
        self.ehr_access.insert(ehr_id, None).await;
    }

    /// Read + parse the EHR's current `EHR_ACCESS` scheme settings from storage
    /// (the cache-miss path). `None` when the EHR has no `EHR_ACCESS`, its
    /// settings are absent, or they belong to another scheme — all default-open.
    async fn load_ehr_access_settings(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<EhrAccessSettings>, SmError> {
        let Some((vo_id, _)) = self.current_vo(ehr_id, Kind::EhrAccess).await? else {
            return Ok(None);
        };
        let Some(read) = read_current(&self.pool, vo_id).await? else {
            return Ok(None);
        };
        Ok(EhrAccessSettings::from_ehr_access(&read.canonical))
    }
}

/// The `EhrAccessAdapter` native-API extension: the protocol adapter
/// (`ehrbase-rest`) — the out-of-band access-decision point (SM
/// `openehr_platform/master02-overview.adoc`) — reads the EHR's current
/// `EHR_ACCESS` settings through this seam. The SM defines no `I_EHR_ACCESS`
/// interface — no openEHR spec governs this adapter, our own extension.
impl EhrbaseService {
    pub async fn current_ehr_access_settings(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<EhrAccessSettings>, SmError> {
        // Clone the (cheap, Arc-backed) service into an owned, `'static` load
        // future so `moka`'s single-flight `try_get_with` can drive it.
        let svc = self.clone();
        let cached = self
            .ehr_access
            .get_or_load(
                ehr_id,
                async move { svc.load_ehr_access_settings(ehr_id).await },
            )
            .await
            .map_err(|e| (*e).clone())?;
        Ok((*cached).clone())
    }
}

/// The default `EHR_ACCESS` created with every EHR (RM ehr master04 §EHR
/// Creation; finding F-06-07). `EHR_ACCESS` is a LOCATABLE with only the optional
/// `settings`; with no access-control scheme configured (Stage 1 has no RBAC),
/// it is committed with none.
pub(in crate::service) fn default_ehr_access() -> Value {
    json!({
        "_type": "EHR_ACCESS",
        "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Access" }
    })
}

/// Validate a client-supplied `EHR_ACCESS` before it is committed (via a
/// CONTRIBUTION — there is no direct ITS-REST `EHR_ACCESS` write). RM ehr
/// `ehr_access.adoc`:
///
/// - a LOCATABLE: `name` (1..1) and a non-empty `archetype_node_id`
///   (`Archetype_node_id_valid`);
/// - a foreign `_type` in this slot is invalid (the container holds `EHR_ACCESS`
///   only);
/// - `settings` (0..1) is a subtype of the ABSTRACT `ACCESS_CONTROL_SETTINGS` —
///   the RM defines no concrete scheme, so a present `settings` must carry a
///   non-empty concrete `_type`, which `scheme()` names (`Scheme_valid`).
pub(in crate::service) fn validate_ehr_access(access: &Value) -> Result<(), ServiceError> {
    let unproc = |m: String| ServiceError::Unprocessable(m);
    let obj = access
        .as_object()
        .ok_or_else(|| unproc("EHR_ACCESS must be a JSON object".to_owned()))?;
    match obj.get("_type").and_then(Value::as_str) {
        None | Some("EHR_ACCESS") => {}
        Some(other) => {
            return Err(unproc(format!(
                "expected an EHR_ACCESS, got _type {other:?}"
            )));
        }
    }
    if obj.get("name").is_none_or(Value::is_null) {
        return Err(unproc(
            "EHR_ACCESS.name is mandatory (LOCATABLE.name 1..1)".to_owned(),
        ));
    }
    if obj
        .get("archetype_node_id")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(unproc(
            "EHR_ACCESS.archetype_node_id is mandatory and non-empty \
             (LOCATABLE.Archetype_node_id_valid)"
                .to_owned(),
        ));
    }
    if let Some(settings) = obj.get("settings").filter(|v| !v.is_null())
        && settings
            .get("_type")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        return Err(unproc(
            "EHR_ACCESS.settings must be a concrete ACCESS_CONTROL_SETTINGS subtype \
             carrying its _type — the scheme name (EHR_ACCESS.Scheme_valid)"
                .to_owned(),
        ));
    }
    Ok(())
}

/// A shared, cloneable per-EHR cache of the current `EHR_ACCESS` scheme settings.
///
/// The `EHR_ACCESS` gateway clause ("All access decisions to data in the EHR
/// must be made in accordance with the policies and rules in this object" — RM
/// ehr `ehr_access.adoc`) is consulted on **every** EHR-scoped request, so the
/// current settings are cached rather than re-read + re-decomposed per request.
/// Invalidated on every `EHR_ACCESS` commit (the settings are change-controlled
/// — RM ehr master04 §EHR Access).
///
/// No openEHR spec governs this cache — our own design/extension. `moka`'s
/// `Cache` is `Arc`-backed, so every clone of the owning service shares one
/// cache (mirroring `openehr_flat::cache::WebTemplateCache`).
#[derive(Debug, Clone)]
pub(in crate::service) struct EhrAccessCache {
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
    pub(in crate::service) async fn get_or_load<Fut>(
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

    /// Seed the cache with a value for `ehr_id` without a DB read — the
    /// pre-warm path. `try_get_with` already negative-caches a default-open
    /// (`None`) result on the first miss, but the *first* access to a
    /// freshly-created EHR would still pay one DB round trip (the
    /// `current_vo` + `read_current` lookup) to discover it is default-open.
    /// A workload that creates EHRs constantly (a hospital day) pays that miss
    /// per new EHR; seeding the known-default-open entry at creation turns the
    /// first access into a hit. Any later `EHR_ACCESS` commit evicts this entry
    /// via [`Self::invalidate`], so a subsequently-restricted EHR is re-read.
    pub(in crate::service) async fn insert(
        &self,
        ehr_id: Uuid,
        settings: Option<EhrAccessSettings>,
    ) {
        self.inner.insert(ehr_id, Arc::new(settings)).await;
    }

    /// Drop the cached settings for `ehr_id` — called on every `EHR_ACCESS`
    /// commit so the next read reflects the new version (evicts a positive OR a
    /// pre-warmed default-open negative entry alike).
    pub(in crate::service) async fn invalidate(&self, ehr_id: Uuid) {
        self.inner.invalidate(&ehr_id).await;
    }
}

impl Default for EhrAccessCache {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{default_ehr_access, validate_ehr_access};

    /// `EHR_ACCESS` commit validation (RM ehr `ehr_access.adoc`): LOCATABLE
    /// structure enforced, a present `settings` must be a concrete
    /// `ACCESS_CONTROL_SETTINGS` subtype (its `_type` is the scheme name —
    /// `Scheme_valid`).
    #[test]
    fn ehr_access_commit_validation() {
        validate_ehr_access(&default_ehr_access()).expect("the default EHR_ACCESS is valid");
        let err = validate_ehr_access(&json!({ "_type": "EHR_STATUS" }))
            .expect_err("foreign _type rejected");
        assert!(err.to_string().contains("EHR_ACCESS"), "got {err}");
        let err = validate_ehr_access(&json!({
            "_type": "EHR_ACCESS", "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1"
        }))
        .expect_err("missing name rejected");
        assert!(err.to_string().contains("name"), "got {err}");
        let err = validate_ehr_access(&json!({
            "_type": "EHR_ACCESS",
            "name": { "_type": "DV_TEXT", "value": "EHR Access" },
            "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
            "settings": { "scheme": "acme" }
        }))
        .expect_err("settings without a concrete _type rejected (Scheme_valid)");
        assert!(err.to_string().contains("Scheme_valid"), "got {err}");
    }
}
