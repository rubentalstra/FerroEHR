//! The service backend abstraction (dependency inversion).
//!
//! A protocol adapter (`ehrbase-rest`) owns the wire surface but not the
//! storage/service logic — that lives in the `ehrbase` application crate, which
//! depends on this one. To avoid a dependency cycle, the adapter depends on the
//! **`Backend`** trait rather than on a concrete service; the adapter's app
//! state holds an `Arc<dyn Backend>`. `ehrbase` implements the SM service
//! traits on its DB-backed service and injects it; until then the default
//! [`StubBackend`] answers every operation with `NotImplemented`.
//!
//! `Backend` is the union of the seams the server actually dispatches to: the
//! generated `DefinitionApi` (templates + stored queries) plus the SM
//! Definitions native seams ([`DefinitionAdl14Service`] /
//! [`DefinitionAdl2Service`] / [`DefinitionQueryService`], SM-2), the five SM EHR-core
//! interfaces ([`EhrService`] / [`EhrStatusService`] / [`EhrCompositionService`]
//! / [`EhrDirectoryService`] / [`EhrContributionService`]), the
//! [`DemographicService`] and [`AdminService`] seams, the [`QueryService`]
//! (AQL), and [`WebTemplateService`] — the single service-owned `WebTemplate`
//! resolution the FLAT/STRUCTURED and `wt+json` surfaces consume (W2-K/F-13-02).
//! The EHR seams return a [`ServiceResponse`](crate::types::ServiceResponse) (RM
//! payload + typed [`ResourceMeta`](crate::types::ResourceMeta)) from which the
//! HTTP edge derives the spec-mandated `ETag`/`Location` headers and drives
//! `Prefer` — none of which the generated bare-`Value` traits can express.

use openehr_its::rest::generated::definition::DefinitionApi;

use crate::services::{
    AdminService, DefinitionAdl2Service, DefinitionAdl14Service, DefinitionQueryService,
    DemographicService, EhrCompositionService, EhrContributionService, EhrDirectoryService,
    EhrIndexService, EhrService, EhrStatusService, PartyRelationshipService, QueryService,
    WebTemplateService,
};

/// The full server backend: everything the ITS-REST surface dispatches to.
/// Implemented once, on the application's service (or on [`StubBackend`]).
/// Groups with no implemented operations are answered `501` without touching
/// the backend (F-13-03); each seam joins this union in the phase that first
/// implements it.
pub trait Backend:
    EhrService
    + EhrStatusService
    + EhrCompositionService
    + EhrDirectoryService
    + EhrContributionService
    + DemographicService
    + PartyRelationshipService
    + EhrIndexService
    + DefinitionApi
    + DefinitionAdl14Service
    + DefinitionAdl2Service
    + DefinitionQueryService
    + WebTemplateService
    + QueryService
    + AdminService
    + Send
    + Sync
    + std::fmt::Debug
    + 'static
{
}

impl<T> Backend for T where
    T: EhrService
        + EhrStatusService
        + EhrCompositionService
        + EhrDirectoryService
        + EhrContributionService
        + DemographicService
        + PartyRelationshipService
        + EhrIndexService
        + DefinitionApi
        + DefinitionAdl14Service
        + DefinitionAdl2Service
        + DefinitionQueryService
        + WebTemplateService
        + QueryService
        + AdminService
        + Send
        + Sync
        + std::fmt::Debug
        + 'static
{
}

/// The default backend: every operation returns
/// [`ApiError::NotImplemented`](openehr_its::rest::runtime::ApiError::NotImplemented).
/// Lets the server boot and route before the `ehrbase` service is wired in.
///
/// Each `impl` is empty — the traits' default method bodies already return
/// `NotImplemented`, so no per-operation stubs are needed.
#[derive(Debug, Clone, Copy, Default)]
pub struct StubBackend;

impl EhrService for StubBackend {}
impl EhrStatusService for StubBackend {}
impl EhrCompositionService for StubBackend {}
impl EhrDirectoryService for StubBackend {}
impl EhrContributionService for StubBackend {}
impl DefinitionApi for StubBackend {}
impl DefinitionAdl14Service for StubBackend {}
impl DefinitionAdl2Service for StubBackend {}
impl DefinitionQueryService for StubBackend {}
impl WebTemplateService for StubBackend {}
impl QueryService for StubBackend {}
impl DemographicService for StubBackend {}
impl PartyRelationshipService for StubBackend {}
impl EhrIndexService for StubBackend {}
impl AdminService for StubBackend {}
