// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The application service layer.
//!
//! The SM Platform Service Model realized as concrete [`FerroEhrService`]
//! methods, one folder per SM chapter (vendored SM spec
//! `docs/specs/openehr/SM/docs/openehr_platform/`).
//!
//! Each chapter folder owns its domain logic as inherent `FerroEhrService`
//! methods; the protocol adapter (`ferroehr-rest`) calls them directly.
//! Change-control semantics live in [`crate::versioning`]; SQL row I/O lives
//! in [`crate::storage`] (no openEHR spec governs the SQL — our own design);
//! this layer orchestrates.
//!
//! The root modules beside the chapters carry the cross-chapter service
//! vocabulary: the SM call-status model ([`status`]), the service error and
//! its status-mapping tables ([`error`]), the version-commit envelope
//! ([`version_update`]), the SM list cursor ([`list`]), the
//! `PLATFORM_SERVICE` enumeration ([`platform_service`]), the
//! protocol-adapter response envelope ([`response`]), the
//! authenticated-committer context ([`committer`]), the crate-internal
//! ITS-REST datetime-request-parameter decoder (`datetime`), and the SM
//! validity checker ([`validity`]).
//!
//! NOTE (adjudicated 2026-08-04, #1845 — no openEHR spec governs the internal
//! layering; the SM-chapter module map is our design):
//! the ~90 one-expression `Ok(self.inner(...).await?)` methods across the SM
//! modules are DELIBERATE, not dead weight. Each is the SM-named operation
//! of the platform service model (`docs/specs/openehr/SM/`) AND the error
//! boundary where a `ServiceError` becomes the SM `SmError` the callers
//! consume — collapsing them would delete the SM component map and scatter
//! the conversion across every call site. Keep the layer; a new SM operation
//! gets its SM-named method here even when the body is one expression.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): service-surface signatures over stored canonical \
              fragments and dynamic shapes"
)]

pub mod admin;
pub mod definition;
pub mod demographic;
pub mod ehr;
pub mod ehr_index;
pub mod message;
pub mod query;
pub mod subject_proxy;
pub mod terminology;

pub mod committer;
pub(crate) mod datetime;
pub mod error;
pub mod list;
pub mod platform_service;
pub mod response;
pub mod status;
pub mod validity;
pub mod version_update;

use crate::service::definition::lineage::{ArchetypeLineageCache, archetype_lineage_cache};
use crate::service::ehr::access::EhrAccessCache;
use crate::service::query::config::QueryConfig;
use crate::service::query::plan_cache::PlanCache;
use crate::service::subject_proxy::config::SubjectProxyFhir;
use crate::service::terminology::fhir::FhirTerminologyProvider;
use crate::service::terminology::router::TerminologyRouter;

mod commit_env;

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use sqlx::PgPool;

use crate::extensions::tenant_context::TenantContext;
use crate::ids::EhrId;
use crate::system_log::sender::AuditSender;
use crate::versioning::SigningCtx;
use crate::versioning::signature::signer::Signer;
use openehr_its::flat::cache::WebTemplateCache;
use openehr_its::flat::webtemplate::model::WebTemplate;
use status::SmError;

/// In-process cache of resolved tenants, keyed by the claim/header value (a
/// tenant name or uuid string) the middleware resolves per request. Shared
/// across service clones (single registry view); cleared wholesale on any
/// tenant CRUD write. Multi-tenancy is spec-silent — our own extension
/// ([`crate::extensions::tenancy`]).
///
/// Bounded + TTL'd, and it caches the NEGATIVE outcome too (`None` = the key
/// resolves to no tenant), so a request stream carrying a bogus tenant key
/// costs one registry read per TTL window, not one per request.
pub(crate) type TenantCache = moka::future::Cache<String, Option<TenantContext>>;

/// Build the tenant resolver cache: capacity-bounded (the registry is small —
/// the bound is a hostile-key guard) with a TTL that also serves as the
/// convergence window for renames across instances.
pub(crate) fn tenant_cache() -> TenantCache {
    moka::future::Cache::builder()
        .max_capacity(10_000)
        .time_to_live(Duration::from_mins(5))
        .build()
}

/// The default openEHR system identifier stamped into `EHR.system_id`,
/// `AUDIT_DETAILS.system_id`, and every `OBJECT_VERSION_ID.creating_system_id`.
///
/// This is the fallback only: a deployment sets its own identifier with the
/// `[server] system_id` config key (`FERROEHR__SERVER__SYSTEM_ID`), which the
/// binary wires in via [`FerroEhrService::with_system_id`]. An unset key leaves
/// this value in force.
pub const DEFAULT_SYSTEM_ID: &str = "ferroehr.local";

/// The `PARTY_IDENTIFIED.name` for a **system-generated** commit.
///
/// It is the `AUDIT_DETAILS.committer` (1..1) of a write with no authenticated
/// principal (auth disabled, or an internal write such as an import or a
/// synthesized composition).
///
/// RM common `master04-generic_package.adoc` §Audit Details makes
/// `AUDIT_DETAILS.committer` mandatory but says nothing about what a system
/// calls itself when it is the committer — no openEHR spec governs this value,
/// it is our own design. It is a single constant so the platform library and
/// the protocol adapter attribute such commits identically; the deployment's
/// machine-readable identity is [`DEFAULT_SYSTEM_ID`] / `[server] system_id`,
/// which is a different attribute (`AUDIT_DETAILS.system_id`) and stays
/// separately configurable.
pub const SYSTEM_COMMITTER_NAME: &str = "FerroEHR";

/// The DB-backed application service — the concrete platform behind the SM
/// chapter methods.
#[derive(Debug, Clone)]
pub struct FerroEhrService {
    pub(crate) pool: PgPool,
    system_id: String,
    /// The ACTIVE openEHR specification generation set (`spec_profile`).
    /// Boot-fixed; the AQL planner's profile gate and the ingress acceptance
    /// boundary read it.
    pub(crate) spec_profile: crate::config::profile::SpecProfile,
    /// Cache of `WebTemplate`s built from stored OPTs, used by composition
    /// validation on create/update. Cheaply cloneable (moka-backed).
    ///
    /// NOTE: cache health is telemetry only — no wire introspection endpoint
    /// exists or is planned (adjudicated 2026-07-25 on the tracker: an
    /// internal optimisation's state is a metric, not a REST surface; no
    /// openEHR spec governs the cache — our own design).
    pub(crate) web_templates: WebTemplateCache,
    /// Version signer (`VERSION.signature`, RM common master06 §Digital
    /// Signature). Defaults to server-side `digest` signing; the binary wires
    /// the configured [`Signer`].
    signer: Arc<Signer>,
    /// The optional IHE ATNA audit sender realizing the SM `I_SYSTEM_LOG`
    /// component (`crate::system_log`). `None` = auditing off; the binary
    /// wires the configured [`AuditSender`] via [`Self::with_audit`].
    pub(crate) audit: Option<AuditSender>,
    /// The optional local Audit Record Repository (read side — the ITI-81
    /// retrieval; `crate::system_log::store`). `None` = the local store is
    /// off; the binary wires it via [`Self::with_audit_store`].
    pub(crate) audit_store: Option<crate::system_log::store::AuditStore>,
    /// The optional external terminology servers (FHIR R4B) — **all** of the
    /// configured providers plus the terminology → provider routing, built
    /// when a deployment opts in
    /// ([`ExternalTerminologyConfig`](crate::service::terminology::config::ExternalTerminologyConfig)).
    /// Consulted by the SM `I_TERMINOLOGY_SERVICE` calls, composition
    /// constraint-binding validation, and AQL `TERMINOLOGY(…)` resolution;
    /// `None` keeps terminology on the in-process `openehr-term` bundle only.
    /// Several servers serve different terminologies at once — BASE
    /// `docs/architecture_overview/master12-terminology.adoc` §Overview.
    pub(crate) terminology: Option<Arc<TerminologyRouter>>,
    /// The optional `DV_MULTIMEDIA` externalization engine (no openEHR spec
    /// governs media externalization — our own extension,
    /// [`crate::extensions::multimedia`]). `None` (default) = inline
    /// behaviour byte-identical.
    #[cfg(feature = "multimedia")]
    pub(crate) multimedia: Option<Arc<ferroehr_ext::multimedia::MultimediaEngine>>,
    /// The optional Subject Proxy FHIR-frame executor, selected when a
    /// deployment configures FHIR systems ([`crate::service::subject_proxy::config::SubjectProxyConfig`]). `None`
    /// (default) makes every FHIR frame a typed rejection (fail-closed).
    pub(crate) subject_proxy_fhir: Option<Arc<SubjectProxyFhir>>,
    /// Multi-tenancy tenant registry cache (extension; empty and unconsulted
    /// in single-tenant mode).
    pub(crate) tenant_cache: TenantCache,
    /// Per-EHR cache of the current `EHR_ACCESS` scheme settings ("All access
    /// decisions to data in the EHR must be made in accordance with the
    /// policies and rules in this object" — RM ehr `ehr_access.adoc`).
    /// Invalidated on every `EHR_ACCESS` commit.
    pub(in crate::service) ehr_access: EhrAccessCache,
    /// Bounded cache of lowered AQL plans keyed on the query text. Shared
    /// across service clones (moka-backed). No openEHR spec governs it — our
    /// own performance design.
    pub(crate) plan_cache: PlanCache,
    /// Memo of the stored archetype specialisation graph
    /// (`adl2_artefact.parent_hrid` edges) the AQL archetype predicate widens a
    /// parent query through (AM `Identification` master07 §Supporting
    /// Archetype-based Querying). Invalidated on every local ADL2 artefact
    /// write; the memo itself is spec-silent — our own performance design.
    pub(crate) archetype_lineage: ArchetypeLineageCache,
    /// Per-query DB execution budget (`[query].timeout_ms`); `None` disables it
    /// (the global request timeout is then the only guard). On overrun the
    /// query is reported as `408`. No openEHR spec governs a query timeout —
    /// our own extension.
    query_timeout: Option<Duration>,
    /// The row ceiling applied to a query that neither the AQL nor the request
    /// bounds; `None` leaves such a query unbounded. See
    /// [`QueryConfig::max_result_rows`](crate::service::query::config::QueryConfig::max_result_rows).
    query_result_ceiling: Option<i64>,
    /// Whether the transactional event outbox is written on every commit. The
    /// outbox feeds the eventing extensions (AMQP publisher + FHIR outbound
    /// emitter) — no openEHR spec governs eventing (our own extension). When no
    /// consumer is configured the per-commit `event_outbox` INSERT (and its
    /// envelope serialization) is pure overhead, so the binary gates it on
    /// whether any consumer is configured on
    /// (`events.enabled || fhir_outbound.enabled`). Defaults to `true` in
    /// [`Self::new`] so a bare service (tests, embeddings) never silently drops
    /// an event; the binary sets the real gate via [`Self::with_outbox_enabled`].
    outbox_enabled: bool,
    /// Short-lived stash of a just-created EHR's RM `EHR` wire body, built from
    /// the commit results at creation and popped by the `ehr_created_object`
    /// adapter seam so a `Prefer: return=representation` create response is
    /// served without re-reading the EHR (the header + status/access version +
    /// folder reads `ehr_summary` would repeat). Bounded + short TTL; an evicted
    /// or absent entry falls back to a full read, so it is invalidation-free by
    /// construction (an EHR's identity at creation is immutable). No openEHR
    /// spec governs it — our own performance design.
    pub(in crate::service) created_ehr_repr: moka::future::Cache<EhrId, Value>,
}

impl FerroEhrService {
    /// Construct the service over a connection pool with the default system id
    /// and the default (server-side `digest`) version signer.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            system_id: DEFAULT_SYSTEM_ID.to_owned(),
            spec_profile: crate::config::profile::SpecProfile::default(),
            web_templates: WebTemplateCache::default(),
            signer: Arc::new(Signer::digest_default()),
            audit: None,
            audit_store: None,
            terminology: None,
            #[cfg(feature = "multimedia")]
            multimedia: None,
            subject_proxy_fhir: None,
            tenant_cache: tenant_cache(),
            ehr_access: EhrAccessCache::default(),
            plan_cache: PlanCache::default(),
            archetype_lineage: archetype_lineage_cache(),
            query_timeout: None,
            query_result_ceiling: None,
            outbox_enabled: true,
            created_ehr_repr: moka::future::Cache::builder()
                .max_capacity(4096)
                .time_to_live(Duration::from_secs(30))
                .build(),
        }
    }

    // ── Builders (the binary wires each configured subsystem) ────────────────

    /// Selects the openEHR specification generation set this service runs
    /// (`spec_profile`; default `development`).
    #[must_use]
    pub fn with_spec_profile(mut self, profile: crate::config::profile::SpecProfile) -> Self {
        self.spec_profile = profile;
        self
    }

    /// Set the openEHR system id (identifies this CDR in `EHR.system_id`,
    /// audit rows, and every minted `OBJECT_VERSION_ID`). The binary wires
    /// `[server] system_id`; without a call, [`DEFAULT_SYSTEM_ID`] stands.
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

    /// Install the local Audit Record Repository (the read side serving the
    /// RESTful-ATNA ITI-81 retrieval); the binary wires it when
    /// `[audit.store]` is enabled.
    #[must_use]
    pub fn with_audit_store(mut self, store: crate::system_log::store::AuditStore) -> Self {
        self.audit_store = Some(store);
        self
    }

    /// Install the materialised terminology servers + their routing (opt-in).
    /// Without it, terminology routes only to the in-process `openehr-term`
    /// bundle.
    #[must_use]
    pub fn with_terminology_router(mut self, router: Arc<TerminologyRouter>) -> Self {
        self.terminology = Some(router);
        self
    }

    /// Install a single external FHIR R4B terminology provider as the default
    /// route — the one-server shorthand for
    /// [`Self::with_terminology_router`].
    #[must_use]
    pub fn with_external_terminology(self, provider: Arc<FhirTerminologyProvider>) -> Self {
        self.with_terminology_router(Arc::new(TerminologyRouter::single(provider)))
    }

    /// The terminology server `key` is explicitly routed to, or `None` (no
    /// default fallback — [`TerminologyRouter::route`]). A caller with several
    /// candidate keys chains this and ends with
    /// [`Self::terminology_default_provider`].
    pub(crate) fn terminology_route(&self, key: &str) -> Option<Arc<FhirTerminologyProvider>> {
        self.terminology.as_deref().and_then(|r| r.route(key))
    }

    /// The terminology server answering an unrouted call, or `None` when no
    /// external terminology is configured
    /// ([`TerminologyRouter::default_provider`]).
    pub(crate) fn terminology_default_provider(&self) -> Option<Arc<FhirTerminologyProvider>> {
        self.terminology
            .as_deref()
            .and_then(TerminologyRouter::default_provider)
    }

    /// The terminology server serving `terminology` — its explicit route, else
    /// the default provider.
    pub(crate) fn terminology_provider(
        &self,
        terminology: &str,
    ) -> Option<Arc<FhirTerminologyProvider>> {
        self.terminology_route(terminology)
            .or_else(|| self.terminology_default_provider())
    }

    /// Install the `DV_MULTIMEDIA` externalization engine (opt-in extension).
    /// Without it, inline media is stored verbatim (byte-identical).
    #[must_use]
    #[cfg(feature = "multimedia")]
    pub fn with_multimedia(
        mut self,
        engine: Arc<ferroehr_ext::multimedia::MultimediaEngine>,
    ) -> Self {
        self.multimedia = Some(engine);
        self
    }

    /// Install the Subject Proxy FHIR-frame executor (opt-in via
    /// [`crate::service::subject_proxy::config::SubjectProxyConfig`]). Without it, an `API_CALL`/`fhir_get`
    /// `DATA_FRAME` is a typed rejection (fail-closed).
    #[must_use]
    pub fn with_subject_proxy(mut self, fhir: Arc<SubjectProxyFhir>) -> Self {
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

    /// Apply the `[query]` tuning knobs — the plan-cache capacity and the
    /// per-query execution budget ([`QueryConfig`]). No openEHR spec governs
    /// these — our own operational extension.
    #[must_use]
    pub fn with_query_config(mut self, query: &QueryConfig) -> Self {
        self.plan_cache = PlanCache::new(query.plan_cache_capacity);
        self.query_timeout = query.timeout();
        self.query_result_ceiling = query.result_ceiling();
        self
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// The `WebTemplate` derived from a stored OPT — the SM `I_DEFINITION`
    /// `WebTemplate` exposure: one resolution serves validation,
    /// FLAT/STRUCTURED conversion, and `wt+json` (the derived runtime artefact;
    /// the `WebTemplate` format itself is spec-silent, `crate::templates`).
    ///
    /// # Errors
    ///
    /// `content_invalid` when no OPT with `template_id` is stored ("operational
    /// template not known" — the commit-path `422`,
    /// `422_COMPOSITION.yaml`: "the underlying template is not known") or when
    /// the stored OPT fails to parse/build into a `WebTemplate`; a
    /// storage-classified status (`exception`, `conflict`, or
    /// `service_overloaded`) on a database failure while loading it.
    pub async fn web_template(&self, template_id: &str) -> Result<Arc<WebTemplate>, SmError> {
        Ok(self.web_template_for(template_id).await?)
    }

    /// The AQL plan cache, for observability. No openEHR spec governs it.
    #[must_use]
    pub fn plan_cache(&self) -> &PlanCache {
        &self.plan_cache
    }

    /// The openEHR `system_id` in effect for the current request: the resolved
    /// tenant's own `system_id` when tenancy is on, else the configured
    /// default (with tenancy off the task-local is never set and this is
    /// byte-identical to the configured `system_id`).
    #[expect(
        clippy::same_name_method,
        reason = "the `CommitEnv` seam (service/commit_env.rs) deliberately \
                  mirrors these chapter method names so the versioning layer \
                  calls them by their own vocabulary; that impl disambiguates \
                  explicitly with `FerroEhrService::<name>(self, …)`"
    )]
    pub(crate) fn effective_system_id(&self) -> String {
        crate::extensions::tenant_context::current()
            .map_or_else(|| self.system_id.clone(), |t| t.system_id)
    }

    /// The configured version [`Signer`] (used for read-time verification).
    pub(crate) fn signer(&self) -> &Signer {
        &self.signer
    }

    /// The write-time signing context (RM common master06 §Digital Signature)
    /// threaded into every versioned-object commit.
    #[expect(
        clippy::same_name_method,
        reason = "the `CommitEnv` seam (service/commit_env.rs) deliberately \
                  mirrors these chapter method names so the versioning layer \
                  calls them by their own vocabulary; that impl disambiguates \
                  explicitly with `FerroEhrService::<name>(self, …)`"
    )]
    pub(crate) fn signing_ctx(&self) -> SigningCtx<'_> {
        SigningCtx {
            system_id: self.effective_system_id(),
            signer: &self.signer,
            spec_profile: self.spec_profile,
            #[cfg(feature = "multimedia")]
            multimedia: self.multimedia.as_deref(),
            outbox_enabled: self.outbox_enabled,
        }
    }
}
