//! The service backend abstraction (dependency inversion).
//!
//! `ehrbase-rest` owns the HTTP surface but not the storage/service logic —
//! that lives in the `ehrbase` application crate, which depends on this one. To
//! avoid a dependency cycle, the server depends on the **`Backend`** trait
//! (the union of the five generated ITS-REST server traits) rather than on a
//! concrete service; [`AppState`](crate::AppState) holds an `Arc<dyn Backend>`.
//! `ehrbase` implements the five traits on its DB-backed service and injects it
//! via [`AppState::with_backend`](crate::AppState::with_backend); until then the
//! default [`StubBackend`] answers every operation with `NotImplemented`.

use openehr_its::rest::generated::admin::AdminApi;
use openehr_its::rest::generated::definition::DefinitionApi;
use openehr_its::rest::generated::demographic::DemographicApi;
use openehr_its::rest::generated::ehr::EhrApi;
use openehr_its::rest::generated::query::QueryApi;

/// The full server backend: everything the ITS-REST surface can dispatch to.
/// Implemented once, on the application's service (or on [`StubBackend`]).
pub trait Backend:
    EhrApi
    + DemographicApi
    + DefinitionApi
    + QueryApi
    + AdminApi
    + Send
    + Sync
    + std::fmt::Debug
    + 'static
{
}

impl<T> Backend for T where
    T: EhrApi
        + DemographicApi
        + DefinitionApi
        + QueryApi
        + AdminApi
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
/// Each `impl` is empty — the generated traits' default method bodies already
/// return `NotImplemented`, so no per-operation stubs are needed.
#[derive(Debug, Clone, Copy, Default)]
pub struct StubBackend;

impl EhrApi for StubBackend {}
impl DemographicApi for StubBackend {}
impl DefinitionApi for StubBackend {}
impl QueryApi for StubBackend {}
impl AdminApi for StubBackend {}
