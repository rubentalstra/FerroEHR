//! The SM `I_EHR` per-EHR accessor, realized as a generic handle.
//!
//! `I_EHR` (`i_ehr.adoc`: "Interface for single patient EHR-level
//! operations") is not an operation interface —
//! it is a per-EHR *accessor* with four mandatory attributes
//! (`ehr_status`, `directory`, `compositions`, `contributions`), obtained via
//! `I_EHR_SERVICE.i_ehr(ehr_id)`. It is realized here as a zero-cost generic
//! handle over the platform service `S`; the sub-handles delegate to the flat
//! trait calls (which remain the implementation surface) — the literal SM
//! shape as ergonomic sugar.

use uuid::Uuid;

use super::{EhrCompositionService, EhrContributionService, EhrDirectoryService, EhrStatusService};

/// The `I_EHR` accessor: a borrow of the platform service `S` bound to one
/// `ehr_id`. Built via [`EhrService::i_ehr`](crate::services::EhrService::i_ehr).
#[derive(Debug, Clone, Copy)]
pub struct IEhr<'a, S: ?Sized> {
    svc: &'a S,
    ehr_id: Uuid,
}

impl<'a, S: ?Sized> IEhr<'a, S> {
    /// Bind the accessor to a service borrow and an EHR id.
    #[must_use]
    pub fn new(svc: &'a S, ehr_id: Uuid) -> Self {
        Self { svc, ehr_id }
    }

    /// The EHR id this accessor is bound to (`I_EHR` is per-EHR).
    #[must_use]
    pub fn ehr_id(&self) -> Uuid {
        self.ehr_id
    }
}

impl<'a, S: EhrStatusService + ?Sized> IEhr<'a, S> {
    /// `I_EHR.ehr_status: I_EHR_STATUS` — the `EHR_STATUS` sub-interface.
    #[must_use]
    pub fn ehr_status(&self) -> EhrStatusHandle<'a, S> {
        EhrStatusHandle {
            svc: self.svc,
            ehr_id: self.ehr_id,
        }
    }
}

impl<'a, S: EhrDirectoryService + ?Sized> IEhr<'a, S> {
    /// `I_EHR.directory: I_EHR_DIRECTORY` — the DIRECTORY sub-interface.
    #[must_use]
    pub fn directory(&self) -> EhrDirectoryHandle<'a, S> {
        EhrDirectoryHandle {
            svc: self.svc,
            ehr_id: self.ehr_id,
        }
    }
}

impl<'a, S: EhrCompositionService + ?Sized> IEhr<'a, S> {
    /// `I_EHR.compositions: I_EHR_COMPOSITION` — the COMPOSITION sub-interface.
    #[must_use]
    pub fn compositions(&self) -> EhrCompositionHandle<'a, S> {
        EhrCompositionHandle {
            svc: self.svc,
            ehr_id: self.ehr_id,
        }
    }
}

impl<'a, S: EhrContributionService + ?Sized> IEhr<'a, S> {
    /// `I_EHR.contributions: I_EHR_CONTRIBUTION` — the CONTRIBUTION
    /// sub-interface.
    #[must_use]
    pub fn contributions(&self) -> EhrContributionHandle<'a, S> {
        EhrContributionHandle {
            svc: self.svc,
            ehr_id: self.ehr_id,
        }
    }
}

/// The `I_EHR_STATUS` sub-handle bound to one EHR (delegates to the flat trait).
#[derive(Debug, Clone, Copy)]
pub struct EhrStatusHandle<'a, S: ?Sized> {
    svc: &'a S,
    ehr_id: Uuid,
}

impl<'a, S: EhrStatusService + ?Sized> EhrStatusHandle<'a, S> {
    /// The underlying service and the bound EHR id (for direct flat calls).
    #[must_use]
    pub fn parts(&self) -> (&'a S, Uuid) {
        (self.svc, self.ehr_id)
    }
}

/// The `I_EHR_DIRECTORY` sub-handle bound to one EHR.
#[derive(Debug, Clone, Copy)]
pub struct EhrDirectoryHandle<'a, S: ?Sized> {
    svc: &'a S,
    ehr_id: Uuid,
}

impl<'a, S: EhrDirectoryService + ?Sized> EhrDirectoryHandle<'a, S> {
    /// The underlying service and the bound EHR id.
    #[must_use]
    pub fn parts(&self) -> (&'a S, Uuid) {
        (self.svc, self.ehr_id)
    }
}

/// The `I_EHR_COMPOSITION` sub-handle bound to one EHR.
#[derive(Debug, Clone, Copy)]
pub struct EhrCompositionHandle<'a, S: ?Sized> {
    svc: &'a S,
    ehr_id: Uuid,
}

impl<'a, S: EhrCompositionService + ?Sized> EhrCompositionHandle<'a, S> {
    /// The underlying service and the bound EHR id.
    #[must_use]
    pub fn parts(&self) -> (&'a S, Uuid) {
        (self.svc, self.ehr_id)
    }
}

/// The `I_EHR_CONTRIBUTION` sub-handle bound to one EHR.
#[derive(Debug, Clone, Copy)]
pub struct EhrContributionHandle<'a, S: ?Sized> {
    svc: &'a S,
    ehr_id: Uuid,
}

impl<'a, S: EhrContributionService + ?Sized> EhrContributionHandle<'a, S> {
    /// The underlying service and the bound EHR id.
    #[must_use]
    pub fn parts(&self) -> (&'a S, Uuid) {
        (self.svc, self.ehr_id)
    }
}
