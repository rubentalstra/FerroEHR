//! The SM `I_QUERY_SERVICE` interface — AQL query execution.

use async_trait::async_trait;

use openehr_its::rest::runtime::ApiError;

use crate::types::{AqlQueryRequest, QueryOutcome};

/// The AQL query execution seam (P16) — the QUERY API group's application seam,
/// re-joined to [`Backend`] now that the engine lands (the W3-B slimming removed
/// `QueryApi` with the note "query rejoins at P16"). It returns the assembled
/// ITS-REST 1.0.3 `RESULT_SET` as canonical JSON; the HTTP edge renders it.
///
/// Realizes the SM `I_QUERY_SERVICE` interface
/// (`docs/specs/openehr/SM/docs/UML/classes/i_query_service.adoc`).
///
/// Both methods default to `NotImplemented`, so [`StubBackend`] (and any partial
/// backend) inherits a `501` until the real service overrides them.
///
/// [`Backend`]: crate::backend::Backend
/// [`StubBackend`]: crate::backend::StubBackend
#[async_trait]
pub trait QueryService: Send + Sync {
    /// `POST/GET /query/aql` — execute an ad-hoc AQL query, returning its
    /// `RESULT_SET`.
    async fn query_execute_adhoc(
        &self,
        _aql: String,
        _request: AqlQueryRequest,
    ) -> Result<QueryOutcome, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `POST/GET /query/{qualified_query_name}[/{version}]` — execute a stored
    /// query, returning its `RESULT_SET`. `version` is a full/partial SEMVER or
    /// `None` for the latest.
    async fn query_execute_stored(
        &self,
        _qualified_query_name: String,
        _version: Option<String>,
        _request: AqlQueryRequest,
    ) -> Result<QueryOutcome, ApiError> {
        Err(ApiError::NotImplemented)
    }
}
