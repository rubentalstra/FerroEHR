//! The application service layer: the SM Platform Service Model realization,
//! one folder per SM chapter mirroring `app/ehrbase-sm/src/services/`
//! (vendored SM spec `docs/specs/openehr/SM/docs/openehr_platform/`).
//!
//! Each chapter folder carries the domain logic plus its
//! `impl <Interface>Service for EhrbaseService` blocks — the SM native traits
//! in `ehrbase-sm` are the fixed service seam the protocol adapter
//! (`ehrbase-rest`) calls through. Change-control semantics live in
//! [`crate::versioning`]; SQL row I/O lives in [`crate::storage`] (no openEHR
//! spec governs the SQL — our own design); this layer orchestrates.

pub mod admin;
pub mod definition;
pub mod demographic;
pub mod ehr;
pub mod ehr_index;
pub mod message;
pub mod query;
pub mod subject_proxy;
pub mod terminology;
pub mod validity;

pub mod committer;
pub mod status;

use status::SmError;
pub mod list;
pub mod platform_service;
pub mod response;
pub mod version_update;

pub use query::{PlanCache, PlanCacheStats, QueryConfig};
pub use subject_proxy::{SpFhirSystem, SubjectProxyConfig, SubjectProxyFhir};
pub use terminology::{
    ExternalTerminologyConfig, FhirOperation, FhirProviderConfig, FhirTerminologyProvider,
    ProviderKind, TerminologyConfig,
};

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::extensions::tenant_context::TenantContext;
use crate::system_log::AuditSender;
use crate::versioning::signature::Signer;
use crate::versioning::{CommitEnv, Kind, SigningCtx};
use openehr_flat::WebTemplate;
use openehr_flat::cache::WebTemplateCache;
use openehr_its::rest::runtime::ApiError;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// In-process cache of resolved tenants, keyed by the claim/header value (a
/// tenant name or uuid string) the middleware resolves per request. Shared
/// across service clones (single registry view); cleared wholesale on any
/// tenant CRUD write. Multi-tenancy is spec-silent — our own extension
/// ([`crate::extensions::tenancy`]).
type TenantCache = Arc<RwLock<HashMap<String, TenantContext>>>;

/// The default openEHR system identifier stamped into `OBJECT_VERSION_ID`s and
/// audit rows. Configurable per deployment (`main.rs` wires it from config).
pub const DEFAULT_SYSTEM_ID: &str = "ehrbase-rs.local";

/// The DB-backed application service — the concrete platform behind the SM
/// native traits.
#[derive(Debug, Clone)]
pub struct EhrbaseService {
    pool: PgPool,
    system_id: String,
    /// Cache of `WebTemplate`s built from stored OPTs, used by composition
    /// validation on create/update. Cheaply cloneable (moka-backed).
    web_templates: WebTemplateCache,
    /// Version signer (`VERSION.signature`, RM common master06 §Digital
    /// Signature). Defaults to server-side `digest` signing; `main.rs` wires
    /// the configured [`Signer`].
    signer: Arc<Signer>,
    /// The optional IHE ATNA audit sender realizing the SM `I_SYSTEM_LOG`
    /// component (`crate::system_log`). `None` = auditing off; the binary
    /// wires the configured [`AuditSender`] via [`Self::with_audit`].
    audit: Option<AuditSender>,
    /// The optional external terminology provider (FHIR R4), selected when a
    /// deployment opts in ([`ExternalTerminologyConfig`]). Used by AQL
    /// `TERMINOLOGY(…)` resolution; `None` keeps terminology on the
    /// in-process `openehr-term` bundle only.
    external_terminology: Option<Arc<FhirTerminologyProvider>>,
    /// The optional `DV_MULTIMEDIA` externalization engine (no openEHR spec
    /// governs media externalization — our own extension,
    /// [`crate::extensions::multimedia`]). `None` (default) = inline
    /// behaviour byte-identical.
    multimedia: Option<Arc<crate::extensions::multimedia::MultimediaEngine>>,
    /// The optional Subject Proxy FHIR-frame executor, selected when a
    /// deployment configures FHIR systems ([`SubjectProxyConfig`]). `None`
    /// (default) makes every FHIR frame a typed rejection (fail-closed).
    subject_proxy_fhir: Option<Arc<subject_proxy::SubjectProxyFhir>>,
    /// Multi-tenancy tenant registry cache (extension; empty and unconsulted
    /// in single-tenant mode).
    tenant_cache: TenantCache,
    /// Per-EHR cache of the current `EHR_ACCESS` scheme settings ("All access
    /// decisions to data in the EHR must be made in accordance with the
    /// policies and rules in this object" — RM ehr `ehr_access.adoc`).
    /// Invalidated on every `EHR_ACCESS` commit.
    ehr_access: ehr::EhrAccessCache,
    /// Bounded cache of lowered AQL plans keyed on the query text (P20). Shared
    /// across service clones (moka-backed). No openEHR spec governs it — our
    /// own performance design.
    plan_cache: PlanCache,
    /// Per-query DB execution budget (`[query].timeout_ms`); `None` disables it
    /// (the global request timeout is then the only guard). On overrun the query
    /// is reported as `408`. No openEHR spec governs a query timeout — our own
    /// extension.
    query_timeout: Option<std::time::Duration>,
    /// Whether the transactional event outbox is written on every commit. The
    /// outbox feeds the eventing extensions (AMQP publisher + FHIR outbound
    /// emitter) — no openEHR spec governs eventing (our own extension). When no
    /// consumer is configured the per-commit `event_outbox` INSERT (and its
    /// envelope serialization) is pure overhead, so the binary gates it on
    /// whether any consumer is configured on (`main.rs`:
    /// `events.enabled || fhir_outbound.enabled`). Defaults to `true` in
    /// [`Self::new`] so a bare service (tests, embeddings) never silently drops
    /// an event; the binary sets the real gate via [`Self::with_outbox_enabled`].
    outbox_enabled: bool,
    /// Short-lived stash of a just-created EHR's RM `EHR` wire body, built from
    /// the commit results at creation and popped by the `ehr_created_object`
    /// adapter seam so a `Prefer: return=representation` create response is
    /// served without re-reading the EHR (the header + status/access version +
    /// folder reads `ehr_summary` would repeat). Bounded + short TTL; an evicted
    /// or absent entry falls back to a full read, so it is invalidation-free by
    /// construction (an EHR's identity at creation is immutable). No openEHR spec
    /// governs it — our own performance design.
    created_ehr_repr: moka::future::Cache<Uuid, Value>,
}

impl EhrbaseService {
    /// Construct the service over a connection pool with the default system id
    /// and the default (server-side `digest`) version signer.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            system_id: DEFAULT_SYSTEM_ID.to_owned(),
            web_templates: WebTemplateCache::default(),
            signer: Arc::new(Signer::digest_default()),
            audit: None,
            external_terminology: None,
            multimedia: None,
            subject_proxy_fhir: None,
            tenant_cache: TenantCache::default(),
            ehr_access: ehr::EhrAccessCache::default(),
            plan_cache: PlanCache::default(),
            query_timeout: None,
            outbox_enabled: true,
            created_ehr_repr: moka::future::Cache::builder()
                .max_capacity(4096)
                .time_to_live(std::time::Duration::from_secs(30))
                .build(),
        }
    }

    /// The openEHR `system_id` in effect for the current request: the resolved
    /// tenant's own `system_id` when tenancy is on, else the configured
    /// default (with tenancy off the task-local is never set and this is
    /// byte-identical to the configured `system_id`).
    fn effective_system_id(&self) -> String {
        crate::extensions::tenant_context::current()
            .map_or_else(|| self.system_id.clone(), |t| t.system_id)
    }

    /// The AQL plan cache (P20), for observability. No openEHR spec governs it.
    #[must_use]
    pub fn plan_cache(&self) -> &PlanCache {
        &self.plan_cache
    }

    /// Override the openEHR system id (identifies this CDR in versions/audit).
    #[must_use]
    pub fn with_system_id(mut self, system_id: impl Into<String>) -> Self {
        self.system_id = system_id.into();
        self
    }

    /// Install the configured version [`Signer`] (RM common master06 §Digital
    /// Signature).
    #[must_use]
    pub fn with_signer(mut self, signer: Arc<Signer>) -> Self {
        self.signer = signer;
        self
    }

    /// Install the IHE ATNA audit sender realizing the SM `I_SYSTEM_LOG`
    /// component; the binary boots it and wires it here.
    #[must_use]
    pub fn with_audit(mut self, sender: AuditSender) -> Self {
        self.audit = Some(sender);
        self
    }

    /// Install an external FHIR R4 terminology provider (opt-in), used by AQL
    /// `TERMINOLOGY(…)` resolution. Without it, terminology routes only to
    /// the in-process `openehr-term` bundle.
    #[must_use]
    pub fn with_external_terminology(mut self, provider: Arc<FhirTerminologyProvider>) -> Self {
        self.external_terminology = Some(provider);
        self
    }

    /// Install the `DV_MULTIMEDIA` externalization engine (opt-in extension).
    /// Without it, inline media is stored verbatim (byte-identical).
    #[must_use]
    pub fn with_multimedia(
        mut self,
        engine: Arc<crate::extensions::multimedia::MultimediaEngine>,
    ) -> Self {
        self.multimedia = Some(engine);
        self
    }

    /// Install the Subject Proxy FHIR-frame executor (opt-in via
    /// [`SubjectProxyConfig`]). Without it, an `API_CALL`/`fhir_get`
    /// `DATA_FRAME` is a typed rejection (fail-closed).
    #[must_use]
    pub fn with_subject_proxy(mut self, fhir: Arc<subject_proxy::SubjectProxyFhir>) -> Self {
        self.subject_proxy_fhir = Some(fhir);
        self
    }

    /// Set whether the transactional event outbox is written on commit. The
    /// binary calls this with `events.enabled || fhir_outbound.enabled` so the
    /// `event_outbox` INSERT is skipped when no consumer will ever read it. The
    /// gate reflects whether the eventing subsystem is *configured on* (a
    /// boot-time flag), not whether subscribers currently exist — so commits
    /// made while eventing is enabled are always recorded, even with zero bound
    /// subscribers (at-least-once replay). No openEHR spec governs eventing —
    /// our own extension.
    #[must_use]
    pub fn with_outbox_enabled(mut self, enabled: bool) -> Self {
        self.outbox_enabled = enabled;
        self
    }

    /// Whether the transactional event outbox is written on commit (see
    /// [`Self::with_outbox_enabled`]).
    fn outbox_enabled(&self) -> bool {
        self.outbox_enabled
    }

    /// Apply the `[query]` tuning knobs — the plan-cache capacity and the
    /// per-query execution budget (`crate::service::query::QueryConfig`). No
    /// openEHR spec governs these — our own operational extension.
    #[must_use]
    pub fn with_query_config(mut self, query: &crate::service::query::QueryConfig) -> Self {
        self.plan_cache = crate::service::query::PlanCache::new(query.plan_cache_capacity);
        self.query_timeout = query.timeout();
        self
    }

    /// The configured version [`Signer`] (used for read-time verification).
    fn signer(&self) -> &Signer {
        &self.signer
    }

    /// The write-time signing context (RM common master06 §Digital Signature)
    /// threaded into every versioned-object commit.
    fn signing_ctx(&self) -> SigningCtx<'_> {
        SigningCtx {
            system_id: self.effective_system_id(),
            signer: &self.signer,
            multimedia: self.multimedia.as_deref(),
            outbox_enabled: self.outbox_enabled,
        }
    }
}

/// The cross-area hooks the CONTRIBUTION commit orchestration needs
/// ([`crate::versioning::contribution::commit_version_set`]): content
/// validation, the EHR-existence + `is_modifiable` guards, the EHR-singleton
/// lookup, and `EHR_ACCESS` cache invalidation — each realized by its owning
/// service chapter.
#[async_trait::async_trait]
impl CommitEnv for EhrbaseService {
    fn pool(&self) -> &PgPool {
        &self.pool
    }

    fn effective_system_id(&self) -> String {
        EhrbaseService::effective_system_id(self)
    }

    fn default_committer(&self) -> Value {
        ehr::committer()
    }

    fn signing_ctx(&self) -> SigningCtx<'_> {
        EhrbaseService::signing_ctx(self)
    }

    async fn validate_for_commit(
        &self,
        kind: Kind,
        data: &Value,
        incomplete: bool,
    ) -> Result<(), ServiceError> {
        EhrbaseService::validate_for_commit(self, kind, data, incomplete).await
    }

    async fn ensure_ehr_exists(&self, ehr_id: Uuid) -> Result<(), ServiceError> {
        EhrbaseService::ensure_ehr_exists(self, ehr_id).await
    }

    async fn ensure_content_writable(&self, ehr_id: Uuid) -> Result<(), ServiceError> {
        EhrbaseService::ensure_content_writable(self, ehr_id).await
    }

    async fn current_vo(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<Option<(Uuid, i32)>, ServiceError> {
        Ok(EhrbaseService::current_vo(self, ehr_id, kind)
            .await?
            .map(|(vo_id, tree)| (vo_id, tree.trunk)))
    }

    async fn invalidate_ehr_access(&self, ehr_id: Uuid) {
        EhrbaseService::invalidate_ehr_access(self, ehr_id).await;
    }

    async fn folder_root_exists(
        &self,
        ehr_id: Uuid,
        archetype_node_id: &str,
        name: &str,
    ) -> Result<bool, ServiceError> {
        Ok(crate::storage::ehr_repo::live_folder_root_exists(
            &self.pool,
            ehr_id,
            archetype_node_id,
            name,
        )
        .await?)
    }

    async fn pre_composition_modify(
        &self,
        tx: &mut sqlx::PgConnection,
        vo_id: Uuid,
        canonical: &Value,
    ) -> Result<(), ServiceError> {
        ehr::check_versioned_composition_invariants(tx, vo_id, canonical).await
    }

    async fn post_status_commit(
        &self,
        tx: &mut sqlx::PgConnection,
        ehr_id: Uuid,
        status: &Value,
    ) -> Result<(), ServiceError> {
        self.sync_ehr_subject(tx, ehr_id, status).await
    }
}

/// SM `I_DEFINITION` `WebTemplate` exposure: one resolution serves validation,
/// FLAT/STRUCTURED conversion, and `wt+json` (the derived runtime artefact —
/// the `WebTemplate` format itself is spec-silent, `crate::templates`).
impl EhrbaseService {
    pub async fn web_template(&self, template_id: &str) -> Result<Arc<WebTemplate>, SmError> {
        Ok(self.web_template_for(template_id).await?)
    }
}

/// Service-layer error, mapped to the ITS-REST [`ApiError`] at the trait
/// boundary so the REST layer stays free of persistence concerns.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// The requested resource does not exist.
    #[error("{0} not found")]
    NotFound(String),
    /// The request is malformed at the semantic level (e.g. a stale/invalid
    /// `preceding_version_uid`, or an operation on an already-deleted object) —
    /// ITS-REST `400 Bad Request` (`400_already_deleted.yaml`).
    #[error("bad request: {0}")]
    BadRequest(String),
    /// The request conflicts with current state (e.g. EHR already exists).
    #[error("conflict: {0}")]
    Conflict(String),
    /// Optimistic-concurrency precondition (`If-Match`) failed.
    #[error("version conflict: {0}")]
    VersionConflict(String),
    /// The submitted payload is malformed or fails a structural rule.
    #[error("unprocessable: {0}")]
    Unprocessable(String),
    /// A well-formed payload that fails semantic (template/RM/terminology)
    /// validation — carries the per-path violations for the ITS-REST 422 body.
    #[error("{} validation error(s)", .0.len())]
    ValidationFailed(Vec<openehr_its::rest::runtime::ValidationError>),
    /// A storage/codec failure.
    #[error("storage: {0}")]
    Storage(#[from] crate::storage::StorageError),
    /// A database failure.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    /// A JSON (de)serialization failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// A version-signing or read-time integrity failure (RM common master06
    /// §Digital Signature) — either signing at commit failed, or
    /// `verify_on_read = strict` found a stored signature that does not match
    /// the served version.
    #[error("signing: {0}")]
    Signing(String),
    /// A server-side fault with no more specific variant (SM
    /// `CALL_STATUS_TYPE.exception` / `file_not_writable` — → HTTP 500).
    #[error("internal: {0}")]
    Internal(String),
}

impl ServiceError {
    /// Construct the [`ServiceError`] variant for an SM call status — the
    /// service-side entry into the single SM ↔ `ServiceError` ↔ HTTP table
    /// (statuses per `CALL_STATUS_TYPE` + descendants, `ehrbase-sm::error`).
    ///
    /// Consistency with the wire is test-enforced: for every status,
    /// `ApiError::from(ServiceError::sm(s, m))` and
    /// [`CallStatusType::api_error`] produce the same HTTP status.
    #[must_use]
    pub fn sm(status: crate::service::status::CallStatusType, message: impl Into<String>) -> Self {
        use crate::service::status::CallStatusType as S;
        let m = message.into();
        match status {
            // `success` is not an error; constructing it is a server bug.
            // Auth is decided at the adapter (401/403 before dispatch), so a
            // service-side auth failure is likewise a server fault.
            S::Success | S::Exception | S::FileNotWritable | S::AuthFailure => {
                ServiceError::Internal(m)
            }
            S::PreconditionViolation | S::InvalidIdPattern => ServiceError::BadRequest(m),
            S::ObjectVersionDoesNotExist
            | S::VersionedObjectDoesNotExist
            | S::EhrIdDoesNotExist
            | S::PartyIdDoesNotExist
            | S::CompositionDoesNotExist
            | S::ContributionDoesNotExist
            | S::ArtefactDoesNotExist
            | S::TemplateDoesNotExist
            | S::VersionDoesNotExist
            | S::SubjectIdDoesNotExist
            | S::VersionedCompositionDoesNotExist => ServiceError::NotFound(m),
            S::VersionMismatch => ServiceError::VersionConflict(m),
            S::EhrCreateFailDuplicateId
            | S::CompositionAlreadyExists
            | S::EhrForSubjectAlreadyExists
            // A storage-classified generic conflict (W-14 F-13) is also a `409`.
            | S::Conflict => ServiceError::Conflict(m),
            S::CompositionArchetypeInvalid
            | S::InvalidArchetype
            | S::InvalidTemplate
            | S::InvalidArtefact
            | S::InvalidQuery
            | S::DefinitionUnknown
            | S::ContentInvalid => ServiceError::Unprocessable(m),
            // No service-side `ServiceError::NotImplemented`; a not-implemented
            // status surfaces as a server fault (the service implements every
            // catalog call, so this row is unreachable in practice).
            //
            // `ServiceOverloaded` (W-14 F-13) originates only at the storage
            // bridge and flows *up* to the wire as an `SmError` (→ `503`); it
            // never round-trips back into a `ServiceError`. `ServiceError` has
            // no overload variant, so this defensive (unreachable) reverse
            // mapping degrades to a server fault.
            S::NotImplemented | S::ServiceOverloaded => ServiceError::Internal(m),
        }
    }
}

impl From<ServiceError> for crate::service::status::SmError {
    /// Map a service failure onto the SM native `CALL_STATUS_TYPE` error the
    /// catalog traits return. This is the mirror of the
    /// [`From<ServiceError> for ApiError`] table above, expressed in SM status
    /// terms — the protocol adapter (`ehrbase-rest`) then maps the status back
    /// to the ITS-REST status code via [`crate::service::status::CallStatusType::api_error`],
    /// so the wire outcome is identical row-for-row:
    ///
    /// | `ServiceError`            | `CallStatusType`             | HTTP |
    /// |---------------------------|------------------------------|------|
    /// | `NotFound`                | `VersionedObjectDoesNotExist`| 404  |
    /// | `VersionConflict`         | `VersionMismatch`            | 412  |
    /// | `Conflict`                | `CompositionAlreadyExists`   | 409  |
    /// | `Unprocessable`           | `ContentInvalid`             | 422  |
    /// | `ValidationFailed`        | `ContentInvalid`             | 422  |
    /// | `BadRequest`              | `PreconditionViolation`      | 400  |
    /// | `Storage`/`Database`/`Json`/`Signing`/`Internal` | `Exception` | 500 |
    ///
    /// `NotFound` cannot recover the concrete resource kind, so it maps to the
    /// generic `versioned_object_does_not_exist` (all 404s); a chapter that
    /// knows the precise kind constructs its own `SmError` instead (e.g. the
    /// EHR-index chapter's `IndexError`). `Conflict` maps to a representative
    /// already-exists status (all 409s).
    ///
    /// PORT NOTE (wire): the structured per-path violations of `ValidationFailed`
    /// (the ITS-REST `Error.validationErrors[]` array) do **not** survive the SM
    /// boundary — `SmError` carries only a status + message (the SM `I_STATUS`
    /// shape). The violations are joined into the message so the detail is not
    /// wholly lost; the `422` body renders as `{ error, message }` rather than
    /// `{ message, validationErrors[] }`. This is spec-permitted:
    /// `422_COMPOSITION.yaml` declares no `content`/`schema` (the `422` body is
    /// spec-silent; the `Error` object is formally bound only to `400`).
    fn from(e: ServiceError) -> Self {
        use crate::service::status::CallStatusType as S;
        use crate::service::status::SmError;
        match e {
            ServiceError::NotFound(m) => SmError::new(S::VersionedObjectDoesNotExist, m),
            ServiceError::VersionConflict(m) => SmError::new(S::VersionMismatch, m),
            ServiceError::Conflict(m) => SmError::new(S::CompositionAlreadyExists, m),
            ServiceError::Unprocessable(m) => SmError::new(S::ContentInvalid, m),
            ServiceError::ValidationFailed(v) => {
                let joined = v
                    .into_iter()
                    .map(|e| format!("{}: {}", e.path, e.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                SmError::new(S::ContentInvalid, joined)
            }
            ServiceError::BadRequest(m) => SmError::new(S::PreconditionViolation, m),
            ServiceError::Storage(e) => SmError::from(e),
            // A raw `sqlx` error carries SQLSTATE/constraint detail: classify it
            // (integrity/serialization conflict → 409, pool exhaustion → 503)
            // instead of collapsing every database error to a blanket 500
            // (W-14 F-13). The classifier emits the structured trace.
            ServiceError::Database(e) => crate::storage::classify_sqlx(&e),
            ServiceError::Json(e) => SmError::new(S::Exception, e.to_string()),
            ServiceError::Signing(m) | ServiceError::Internal(m) => SmError::new(S::Exception, m),
        }
    }
}

impl From<ServiceError> for ApiError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::NotFound(m) => ApiError::NotFound(m),
            ServiceError::BadRequest(m) => ApiError::BadRequest(m),
            ServiceError::Conflict(m) => ApiError::Conflict(m),
            ServiceError::VersionConflict(m) => ApiError::PreconditionFailed(m),
            ServiceError::Unprocessable(m) => ApiError::Unprocessable(m),
            ServiceError::ValidationFailed(v) => ApiError::ValidationFailed(v),
            // Storage/DB failures carry SQLSTATE/constraint detail: classify
            // them (integrity/serialization conflict → 409, pool exhaustion →
            // 503) rather than blanket-500 (W-14 F-13). A genuine fault stays
            // 500. This path is secondary to the SM `SmError` bridge, but must
            // stay consistent with it.
            ServiceError::Storage(e) => {
                sqlx_conflict_api_error(crate::service::status::SmError::from(e))
            }
            ServiceError::Database(e) => sqlx_conflict_api_error(crate::storage::classify_sqlx(&e)),
            // A JSON (de)serialization failure at the service boundary is a
            // malformed client payload → 400.
            ServiceError::Json(e) => ApiError::BadRequest(e.to_string()),
            // Signing/integrity failures and generic faults are server-side
            // (5xx).
            ServiceError::Signing(m) | ServiceError::Internal(m) => ApiError::Internal(m),
        }
    }
}

/// Map a storage-classified [`SmError`](crate::service::status::SmError) (from
/// [`crate::storage::classify_sqlx`]) to the ITS-REST [`ApiError`] on the
/// direct `ServiceError → ApiError` path. Only the storage-classified statuses
/// occur here — a database conflict (`409`), pool exhaustion (`503`), or a
/// genuine fault (`500`) — mirroring the `sm_api_error` rows the SM bridge uses
/// (`ehrbase-rest::overview::error`). The `503` is our own overload contract
/// (no openEHR spec governs overload; RFC 9110 §15.6.4 is the HTTP authority).
fn sqlx_conflict_api_error(sm: crate::service::status::SmError) -> ApiError {
    use crate::service::status::CallStatusType as S;
    match sm.status {
        S::Conflict | S::EhrForSubjectAlreadyExists => ApiError::Conflict(sm.message),
        S::ServiceOverloaded => ApiError::ServiceUnavailable(sm.message),
        _ => ApiError::Internal(sm.message),
    }
}

#[cfg(test)]
mod sm_error_table_tests {
    use crate::service::status::CallStatusType as S;
    use openehr_its::rest::runtime::ApiError;

    use super::ServiceError;

    /// `ServiceError::sm(status)` routed to the ITS-REST [`ApiError`] must land
    /// on the HTTP status the SM row prescribes. The SM → `ApiError` half of
    /// the table lives in the protocol adapter
    /// (`ehrbase-rest::error::sm_api_error`) and is tested there end-to-end;
    /// here we verify the service-side `ServiceError::sm` +
    /// `From<ServiceError> for ApiError` composition against the expected code
    /// per status.
    #[test]
    fn service_error_routes_to_the_expected_http_status() {
        use http::StatusCode as C;
        let rows = [
            (S::PreconditionViolation, C::BAD_REQUEST),
            (S::InvalidIdPattern, C::BAD_REQUEST),
            (S::ObjectVersionDoesNotExist, C::NOT_FOUND),
            (S::VersionedObjectDoesNotExist, C::NOT_FOUND),
            (S::EhrIdDoesNotExist, C::NOT_FOUND),
            (S::PartyIdDoesNotExist, C::NOT_FOUND),
            (S::CompositionDoesNotExist, C::NOT_FOUND),
            (S::ContributionDoesNotExist, C::NOT_FOUND),
            (S::ArtefactDoesNotExist, C::NOT_FOUND),
            (S::TemplateDoesNotExist, C::NOT_FOUND),
            (S::VersionDoesNotExist, C::NOT_FOUND),
            (S::SubjectIdDoesNotExist, C::NOT_FOUND),
            (S::VersionedCompositionDoesNotExist, C::NOT_FOUND),
            (S::VersionMismatch, C::PRECONDITION_FAILED),
            (S::EhrCreateFailDuplicateId, C::CONFLICT),
            (S::CompositionAlreadyExists, C::CONFLICT),
            (S::EhrForSubjectAlreadyExists, C::CONFLICT),
            (S::CompositionArchetypeInvalid, C::UNPROCESSABLE_ENTITY),
            (S::InvalidArchetype, C::UNPROCESSABLE_ENTITY),
            (S::InvalidTemplate, C::UNPROCESSABLE_ENTITY),
            (S::InvalidArtefact, C::UNPROCESSABLE_ENTITY),
            (S::InvalidQuery, C::UNPROCESSABLE_ENTITY),
            (S::DefinitionUnknown, C::UNPROCESSABLE_ENTITY),
            (S::ContentInvalid, C::UNPROCESSABLE_ENTITY),
            // Service-side auth/exception faults surface as 500 (auth is the
            // adapter's job before dispatch — see `ServiceError::sm`).
            (S::Exception, C::INTERNAL_SERVER_ERROR),
            (S::FileNotWritable, C::INTERNAL_SERVER_ERROR),
            (S::AuthFailure, C::INTERNAL_SERVER_ERROR),
        ];
        for (status, expected) in rows {
            let got = ApiError::from(ServiceError::sm(status, "m")).status();
            assert_eq!(got, expected, "row {} diverged", status.sm_name());
        }
    }
}
