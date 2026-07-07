//! The service backend abstraction (dependency inversion).
//!
//! `ehrbase-rest` owns the HTTP surface but not the storage/service logic —
//! that lives in the `ehrbase` application crate, which depends on this one. To
//! avoid a dependency cycle, the server depends on the **`Backend`** trait
//! rather than on a concrete service; [`AppState`](crate::AppState) holds an
//! `Arc<dyn Backend>`. `ehrbase` implements the traits on its DB-backed service
//! and injects it via [`AppState::with_backend`](crate::AppState::with_backend);
//! until then the default [`StubBackend`] answers every operation with
//! `NotImplemented`.
//!
//! `Backend` is the union of the seams the server actually dispatches to:
//! the generated `DefinitionApi` (templates + stored queries), [`EhrService`]
//! — the EHR group's application seam (W2-A) — and [`WebTemplateService`],
//! the single service-owned `WebTemplate` resolution the FLAT/STRUCTURED and
//! `wt+json` surfaces consume (W2-K/F-13-02). The generated `EhrApi` trait
//! exchanges a bare `serde_json::Value` and so cannot carry the spec-mandated
//! `ETag`/`Location` headers or drive `Prefer`; [`EhrService`] supersedes it
//! for the whole EHR group, returning a [`ServiceResponse`] (RM payload +
//! typed [`ResourceMeta`]) from which the HTTP edge derives those headers.
//!
//! The generated `DemographicApi` / `QueryApi` / `AdminApi` groups have **no
//! implemented operations yet** (demographic: a future RM phase; query/AQL:
//! P16; admin: later), so they are not part of the seam — their routes answer
//! through the generic not-implemented dispatcher (F-13-03). Each trait joins
//! `Backend` in the phase that first implements it.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use openehr_flat::WebTemplate;
use openehr_its::rest::generated::definition::DefinitionApi;
use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams, ContributionCreateParams, ContributionGetParams,
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams, EhrCreateParams, EhrCreateWithIdParams,
    EhrGetByIdParams, EhrGetBySubjectParams, EhrStatusGetAtTimeParams,
    EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams, EhrStatusTagsGetParams,
    EhrStatusTagsUpdateParams, EhrStatusUpdateParams, EhrTagsGetParams,
    VersionedCompositionGetParams, VersionedCompositionRevisionHistoryParams,
    VersionedCompositionVersionGetAtTimeParams, VersionedCompositionVersionGetByIdParams,
    VersionedEhrStatusGetParams, VersionedEhrStatusRevisionHistoryParams,
    VersionedEhrStatusVersionGetAtTimeParams, VersionedEhrStatusVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::response::{ResourceMeta, ServiceResponse};

/// The single `WebTemplate` resolution seam (W2-K / finding F-13-02).
///
/// A stored OPT 1.4 template has exactly **one** built [`WebTemplate`]
/// representation, owned and cached by the service (one `moka` cache keyed by
/// template id). Every consumer — composition validation, the FLAT/STRUCTURED
/// (simSDT/structSDT) converters, and the Better `wt+json` template GET — goes
/// through this method, so the `WebTemplate` a composition is validated against
/// is byte-identical to the one its FLAT round-trip uses. The REST layer holds
/// no cache of its own and never re-fetches/re-parses OPT XML.
///
/// An unknown template id resolves as `Unprocessable` (→ ITS-REST `422`): on a
/// composition commit an unknown referenced template is a *semantic* error
/// (`422_COMPOSITION.yaml` — "the underlying template is not known"; CNF
/// `create_composition-event_bad_opt`).
#[async_trait]
pub trait WebTemplateService: Send + Sync {
    /// Resolve the (service-cached) [`WebTemplate`] for a stored operational
    /// template.
    async fn web_template(&self, _template_id: &str) -> Result<Arc<WebTemplate>, ApiError> {
        Err(ApiError::NotImplemented)
    }
}

/// The EHR group's application seam (W2-A) — the whole EHR / `EHR_STATUS` /
/// COMPOSITION / DIRECTORY / CONTRIBUTION surface, returning a typed
/// [`ServiceResponse`] (the canonical-JSON RM payload plus optional
/// [`ResourceMeta`]) instead of the generated `EhrApi`'s bare `Value`.
///
/// The envelope lets the HTTP edge set the `ETag`/`Location` headers ITS-REST
/// 1.0.3 mandates (`headers/ETag_*.yaml`, `headers/Location_*.yaml`) and honour
/// `Prefer` (`return=minimal` default vs `return=representation`) — none of
/// which a bare `Value` can express. Reads whose spec response declares no
/// headers simply return [`ServiceResponse::plain`]; the dispatch layer decides
/// per operation which headers to emit.
///
/// Every method defaults to `NotImplemented`, so the [`StubBackend`] (and any
/// partial backend) inherits a `501` until the real service overrides it.
#[async_trait]
pub trait EhrService: Send + Sync {
    // ── EHR ──────────────────────────────────────────────────────────────────

    /// `GET /ehr` — find an EHR by subject. `200_EHR` (no `ETag`/`Location`).
    async fn ehr_get_by_subject(
        &self,
        _params: EhrGetBySubjectParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `POST /ehr` — create an EHR. `201` with `ETag`(`ehr_id`)/`Location`; body
    /// only on `Prefer: return=representation` (`201_EHR.yaml`).
    async fn ehr_create(
        &self,
        _params: EhrCreateParams,
        _body: Option<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}` — retrieve an EHR. `200_EHR` (no `ETag`/`Location`).
    async fn ehr_get_by_id(&self, _params: EhrGetByIdParams) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}` — create an EHR with a client id. As `ehr_create`.
    async fn ehr_create_with_id(
        &self,
        _params: EhrCreateWithIdParams,
        _body: Option<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    // ── EHR_STATUS ───────────────────────────────────────────────────────────

    /// `GET /ehr/{ehr_id}/ehr_status/{version_uid}` — the **bare** `EHR_STATUS` at
    /// a specific version (not the `ORIGINAL_VERSION` wrapper — F-01-03). `200`
    /// with `ETag`(`version_uid`)/`Location` (`200_EHR_STATUS_retrieved.yaml`).
    async fn ehr_status_get_by_version_id(
        &self,
        _params: EhrStatusGetByVersionIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/ehr_status` — the `EHR_STATUS` (current or at time).
    /// `200` with `ETag`/`Location` (`200_EHR_STATUS_retrieved.yaml`).
    async fn ehr_status_get_at_time(
        &self,
        _params: EhrStatusGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/ehr_status` — update `EHR_STATUS`. Default `204` (no
    /// body); `200` + body on `return=representation`; `ETag`/`Location` on both
    /// (`204_EHR_STATUS.yaml` / `200_EHR_STATUS_updated.yaml`).
    async fn ehr_status_update(
        &self,
        _params: EhrStatusUpdateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_ehr_status` — the `VERSIONED_EHR_STATUS`.
    /// `200_VERSIONED_EHR_STATUS` (no `ETag`/`Location`).
    async fn versioned_ehr_status_get(
        &self,
        _params: VersionedEhrStatusGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/revision_history`. `200` (plain).
    async fn versioned_ehr_status_revision_history(
        &self,
        _params: VersionedEhrStatusRevisionHistoryParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/version` — the VERSION extant at a
    /// time. `200_VERSION_at_time`: `ETag`(`version_uid`)/`Location`.
    async fn versioned_ehr_status_version_get_at_time(
        &self,
        _params: VersionedEhrStatusVersionGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_ehr_status/version/{version_uid}` — the
    /// `ORIGINAL_VERSION`. `200_VERSION` (no `ETag`/`Location`).
    async fn versioned_ehr_status_version_get_by_id(
        &self,
        _params: VersionedEhrStatusVersionGetByIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    // ── COMPOSITION ──────────────────────────────────────────────────────────

    /// `POST /ehr/{ehr_id}/composition` — create. `201` + `ETag`/`Location`;
    /// body per `Prefer` (`201_COMPOSITION.yaml`).
    async fn composition_create(
        &self,
        _params: CompositionCreateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/composition/{uid_based_id}` — retrieve. `200` +
    /// `ETag`/`Location`, or a deleted read → empty body (→ `204`).
    async fn composition_get(
        &self,
        _params: CompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/composition/{uid_based_id}` — update. `200` +
    /// `ETag`/`Location`; body per `Prefer` (`200_COMPOSITION_updated.yaml`).
    async fn composition_update(
        &self,
        _params: CompositionUpdateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /ehr/{ehr_id}/composition/{uid_based_id}` — logical delete. `204`
    /// + `ETag`/`Location` of the deleted version (`204_COMPOSITION_deleted.yaml`).
    async fn composition_delete(
        &self,
        _params: CompositionDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}`.
    /// `200_VERSIONED_COMPOSITION` (no `ETag`/`Location`).
    async fn versioned_composition_get(
        &self,
        _params: VersionedCompositionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history`.
    async fn versioned_composition_revision_history(
        &self,
        _params: VersionedCompositionRevisionHistoryParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version` —
    /// the VERSION extant at a time. `200_VERSION_of_COMPOSITION_at_time`:
    /// `ETag`/`Location`.
    async fn versioned_composition_version_get_at_time(
        &self,
        _params: VersionedCompositionVersionGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}`
    /// — the `ORIGINAL_VERSION`. `200_VERSION` (no `ETag`/`Location`).
    async fn versioned_composition_version_get_by_id(
        &self,
        _params: VersionedCompositionVersionGetByIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    // ── DIRECTORY (FOLDER) ─────────────────────────────────────────────────────

    /// `GET /ehr/{ehr_id}/directory` — the directory FOLDER (current or at time),
    /// or a deleted read → empty body (→ `204`). `200_FOLDER_retrieved` (no
    /// `ETag`/`Location`).
    async fn directory_get_at_time(
        &self,
        _params: DirectoryGetAtTimeParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/directory` — update. Default `204`; `200` + body on
    /// `return=representation`; `ETag`/`Location` on both
    /// (`204_directory_updated.yaml` / `200_directory_updated.yaml`).
    async fn directory_update(
        &self,
        _params: DirectoryUpdateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `POST /ehr/{ehr_id}/directory` — create the directory FOLDER. `201` +
    /// `ETag`/`Location`; body per `Prefer` (`201_directory.yaml`).
    async fn directory_create(
        &self,
        _params: DirectoryCreateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /ehr/{ehr_id}/directory` — logical delete. `204_because_deleted`
    /// (no headers).
    async fn directory_delete(
        &self,
        _params: DirectoryDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/directory/{version_uid}` — a specific version, or a
    /// deleted read → empty body (→ `204`). `200_FOLDER_retrieved` (no headers).
    async fn directory_get_by_version_id(
        &self,
        _params: DirectoryGetByVersionIdParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    // ── CONTRIBUTION ───────────────────────────────────────────────────────────

    /// `POST /ehr/{ehr_id}/contribution` — commit a CONTRIBUTION. `201` +
    /// `ETag`(`contribution_uid`)/`Location`; body per `Prefer`
    /// (`201_CONTRIBUTION.yaml`).
    async fn contribution_create(
        &self,
        _params: ContributionCreateParams,
        _body: Value,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/contribution/{contribution_uid}`. `200_CONTRIBUTION`
    /// (no `ETag`/`Location`).
    async fn contribution_get(
        &self,
        _params: ContributionGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    // ── item tags (a JSON array body; no headers) ──────────────────────────────

    /// `GET /ehr/{ehr_id}/tags` — all item tags in the EHR.
    async fn ehr_tags_get(&self, _params: EhrTagsGetParams) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/composition/{uid_based_id}/tags`.
    async fn composition_tags_get(
        &self,
        _params: CompositionTagsGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/composition/{uid_based_id}/tags`.
    async fn composition_tags_update(
        &self,
        _params: CompositionTagsUpdateParams,
        _body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}`.
    async fn composition_tags_delete(
        &self,
        _params: CompositionTagsDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `GET /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags`.
    async fn ehr_status_tags_get(
        &self,
        _params: EhrStatusTagsGetParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `PUT /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags`.
    async fn ehr_status_tags_update(
        &self,
        _params: EhrStatusTagsUpdateParams,
        _body: Vec<Value>,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `DELETE /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}`.
    async fn ehr_status_tags_delete(
        &self,
        _params: EhrStatusTagsDeleteParams,
    ) -> Result<ServiceResponse, ApiError> {
        Err(ApiError::NotImplemented)
    }

    // ── conflict-decoration helpers (latest version for 409/412 headers) ────────

    /// The current `EHR_STATUS` version metadata, for the latest `version_uid` the
    /// spec requires in the `ETag`/`Location` of a `412` precondition failure
    /// (`412_EHR_STATUS.yaml`). `None` if the EHR/status is unknown.
    async fn ehr_status_latest_meta(
        &self,
        _ehr_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(None)
    }

    /// The current COMPOSITION version metadata, for the latest `version_uid` in
    /// the `ETag`/`Location` of a `409`/`412`
    /// (`409_COMPOSITION_with_uid_based_id.yaml` / `412_COMPOSITION.yaml`).
    async fn composition_latest_meta(
        &self,
        _ehr_id: String,
        _uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(None)
    }

    /// The current directory FOLDER version metadata, for the latest
    /// `version_uid` in the `ETag`/`Location` of a `412` (`412_directory.yaml`).
    async fn directory_latest_meta(
        &self,
        _ehr_id: String,
    ) -> Result<Option<ResourceMeta>, ApiError> {
        Ok(None)
    }
}

/// A normalized AQL query request: the paging window, the single-EHR scope, and
/// the `$parameter` bindings, gathered from the query string or the request body
/// (`AdhocQueryExecute` / `Query`) by the dispatch layer (ITS-REST query
/// `parameters/query/{ehr_id,offset,fetch}` + `query_parameters`).
#[derive(Debug, Clone, Default)]
pub struct AqlQueryRequest {
    /// The `ehr_id` scope (query param or `openEHR-EHR-id` header), if any.
    pub ehr_id: Option<String>,
    /// The `offset` paging parameter (0-based row to start from).
    pub offset: Option<i64>,
    /// The `fetch` paging parameter (max rows to return).
    pub fetch: Option<i64>,
    /// The `query_parameters` (`$name` binds, no `$` prefix).
    pub parameters: std::collections::BTreeMap<String, Value>,
    /// The ABAC patient-scope subject id (`docs/enterprise/access-control.md`
    /// §6.4): when set, the engine pre-filters every VO root to EHRs whose
    /// subject equals it. `None` = no patient scope.
    pub subject_scope: Option<String>,
    /// Whether the executor should collect the touched EHR-id / template-id sets
    /// for the ABAC query post-check (set by the dispatcher when ABAC is on).
    pub collect_attributes: bool,
}

/// The outcome of an AQL execution: the assembled `RESULT_SET` plus — when the
/// caller asked for them (`AqlQueryRequest::collect_attributes`) — the distinct
/// EHR ids and template ids the query touched, for the ABAC post-check (§6.4).
#[derive(Debug, Clone, Default)]
pub struct QueryOutcome {
    /// The ITS-REST 1.0.3 `RESULT_SET` (canonical JSON) the HTTP edge renders.
    pub result_set: Value,
    /// The distinct EHR ids the query touched (empty unless collected).
    pub ehr_ids: Vec<String>,
    /// The distinct template ids the query touched (empty unless collected).
    pub template_ids: Vec<String>,
}

impl QueryOutcome {
    /// An outcome with no collected attributes (the pre-ABAC shape).
    #[must_use]
    pub fn plain(result_set: Value) -> Self {
        Self {
            result_set,
            ehr_ids: Vec::new(),
            template_ids: Vec::new(),
        }
    }
}

/// The AQL query execution seam (P16) — the QUERY API group's application seam,
/// re-joined to [`Backend`] now that the engine lands (the W3-B slimming removed
/// `QueryApi` with the note "query rejoins at P16"). It returns the assembled
/// ITS-REST 1.0.3 `RESULT_SET` as canonical JSON; the HTTP edge renders it.
///
/// Both methods default to `NotImplemented`, so [`StubBackend`] (and any partial
/// backend) inherits a `501` until the real service overrides them.
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

/// The full server backend: everything the ITS-REST surface dispatches to.
/// Implemented once, on the application's service (or on [`StubBackend`]).
/// Groups with no implemented operations (demographic / admin) are deliberately
/// absent — their routes answer 501 without touching the backend (F-13-03); each
/// generated trait joins this union in the phase that first implements it.
pub trait Backend:
    EhrService
    + DefinitionApi
    + WebTemplateService
    + QueryService
    + Send
    + Sync
    + std::fmt::Debug
    + 'static
{
}

impl<T> Backend for T where
    T: EhrService
        + DefinitionApi
        + WebTemplateService
        + QueryService
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
impl DefinitionApi for StubBackend {}
impl WebTemplateService for StubBackend {}
impl QueryService for StubBackend {}
