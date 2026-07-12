//! The [`EhrAccessAdapter`] native-API extension on [`EhrbaseService`].
//!
//! `EHR_ACCESS` is the openEHR access-decision authority ("All access decisions
//! to data in the EHR must be made in accordance with the policies and rules in
//! this object" — RM `org.openehr.rm.ehr.ehr_access.adoc`). This adapter reads
//! the EHR's current `EHR_ACCESS` version through the normal versioned-object
//! path (`current_vo` → `vobject::read_current`) and parses its `settings` as
//! the `ehrbase.access_control.v1` scheme; the protocol adapter (`ehrbase-rest`)
//! — the out-of-band decision point (SM `openehr_platform/master02-overview.adoc`)
//! — enforces them. The result is cached per EHR (the settings are consulted on
//! every EHR-scoped request) and invalidated on every `EHR_ACCESS` commit.
//!
//! The SM defines no `I_EHR_ACCESS` interface — no openEHR spec governs this
//! adapter, our own extension (`docs/design/ehr-access-scheme.md`).

use async_trait::async_trait;

use ehrbase_sm::{EhrAccessAdapter, EhrAccessSettings, SmError};
use uuid::Uuid;

use crate::service::EhrbaseService;
use crate::service::vobject::{self, Kind};

impl EhrbaseService {
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
        let Some(read) = vobject::read_current(&self.pool, vo_id).await? else {
            return Ok(None);
        };
        Ok(EhrAccessSettings::from_ehr_access(&read.canonical))
    }

    /// Drop the cached `EHR_ACCESS` settings for `ehr_id`. Called from the
    /// commit path whenever an `EHR_ACCESS` version is written so the next read
    /// reflects the new version (the settings are change-controlled — RM ehr
    /// `master04-ehr_package.adoc` §EHR Access).
    pub(crate) async fn invalidate_ehr_access(&self, ehr_id: Uuid) {
        self.ehr_access.invalidate(ehr_id).await;
    }
}

#[async_trait]
impl EhrAccessAdapter for EhrbaseService {
    async fn current_ehr_access_settings(
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
