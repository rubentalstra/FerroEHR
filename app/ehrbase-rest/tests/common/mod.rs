//! One shared mock [`Platform`] for the `ehrbase-rest` router tests (ADR-011).
//!
//! The SM catalog has no default bodies on the EHR-core services or the two
//! ITS-REST adapters (`VersionMetaAdapter`, `ItemTagAdapter`) — a real
//! implementation is mandatory — so every router test needs a concrete
//! platform. Rather than each test re-implementing the whole surface, this
//! module provides a single [`Mock`] that implements the entire [`Platform`]
//! supertrait once, routing each overridable method through an optional
//! per-test closure in [`Hooks`]. An un-hooked EHR-core method returns
//! `CallStatusType::NotImplemented` → HTTP `501`, reproducing the old
//! `StubBackend` exactly (so a test that only wants "the op is mounted but has
//! no backend" sets no hook and still gets its `501`).
//!
//! The non-EHR-core SM services keep their trait default bodies (also `501`),
//! so a test that does not customise a group needs no hook there; the handful
//! of methods some test *does* customise are exposed as hooks too. The
//! generated `DefinitionApi` (templates + stored queries) stays on `ApiError`
//! (it is a generated wire trait); every SM-native service returns `SmError`.
//!
//! Construct with [`Mock::new`] (all-`501`) or [`Mock::with`] a populated
//! [`Hooks`] (`Hooks { create_ehr: Some(...), ..Default::default() }`). Hooks
//! are `Arc`-wrapped sync closures, so a test can share mutable state (e.g. a
//! stored composition) by capturing an `Arc<Mutex<_>>`.
#![allow(clippy::type_complexity, dead_code)]

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use openehr_base::prelude::ObjectVersionId;
use openehr_flat::WebTemplate;

use ehrbase_sm::services::{
    AdminArchive, AdminService, ContributionAdapter, DefinitionAdapter, DefinitionAdl2Service,
    DefinitionAdl14Service, DefinitionQueryService, DemographicService, EhrCompositionService,
    EhrContributionService, EhrDirectoryService, EhrIndexService, EhrService, EhrStatusService,
    ItemTagAdapter, PartyRelationshipService, QueryService, SystemLog, TerminologyService,
    VersionMetaAdapter, WebTemplateService,
};
use ehrbase_sm::types::{
    EhrSummary, PartyKind, ResourceMeta, ServiceResponse, SubjectRef, UpdateAudit, UpdateVersion,
};
use ehrbase_sm::{
    AuditEvent, CallStatusType, EmitOutcome, SmError, TerminologyDescription, TerminologyExtract,
};

/// A `501 Not Implemented` SM error — the un-hooked default (old `StubBackend`).
fn not_impl() -> SmError {
    SmError::new(CallStatusType::NotImplemented, "not mocked")
}

// ── hook type aliases ─────────────────────────────────────────────────────────

type CreateEhr = Arc<dyn Fn(Option<Value>) -> Result<Uuid, SmError> + Send + Sync>;
type CreateEhrWithId = Arc<dyn Fn(Uuid, Option<Value>) -> Result<Uuid, SmError> + Send + Sync>;
type EhrObject = Arc<dyn Fn(Uuid) -> Result<Value, SmError> + Send + Sync>;
type EhrObjectForSubject = Arc<dyn Fn(String, String) -> Result<Value, SmError> + Send + Sync>;
type StatusRead = Arc<dyn Fn(Uuid) -> Result<Value, SmError> + Send + Sync>;
type StatusAtTime = Arc<dyn Fn(Uuid, Option<String>) -> Result<Value, SmError> + Send + Sync>;
type StatusAtVersion = Arc<dyn Fn(Uuid, Uuid, i32) -> Result<Value, SmError> + Send + Sync>;
type ReplaceStatus = Arc<dyn Fn(Uuid, UpdateVersion) -> Result<String, SmError> + Send + Sync>;
type CompLatest = Arc<dyn Fn(Uuid, Uuid) -> Result<Value, SmError> + Send + Sync>;
type CompAtTime = Arc<dyn Fn(Uuid, Uuid, Option<String>) -> Result<Value, SmError> + Send + Sync>;
type CompAtVersion = Arc<dyn Fn(Uuid, ObjectVersionId) -> Result<Value, SmError> + Send + Sync>;
type CreateComp = Arc<dyn Fn(Uuid, UpdateVersion) -> Result<String, SmError> + Send + Sync>;
type UpdateComp = Arc<dyn Fn(Uuid, Uuid, UpdateVersion) -> Result<String, SmError> + Send + Sync>;
type DeleteComp = Arc<dyn Fn(Uuid, ObjectVersionId) -> Result<String, SmError> + Send + Sync>;
type CompMeta = Arc<dyn Fn(Uuid, Uuid) -> Result<Option<ResourceMeta>, SmError> + Send + Sync>;

type WebTemplateHook = Arc<dyn Fn(String) -> Result<Arc<WebTemplate>, SmError> + Send + Sync>;
type AdminDelete = Arc<dyn Fn(String) -> Result<(), SmError> + Send + Sync>;
type AdminDeleteAll = Arc<dyn Fn(Vec<String>) -> Result<u64, SmError> + Send + Sync>;
type GetArtefact = Arc<dyn Fn(String) -> Result<String, SmError> + Send + Sync>;

type PartyCreate = Arc<dyn Fn(PartyKind, Value) -> Result<ServiceResponse, SmError> + Send + Sync>;
type PartyGet = Arc<
    dyn Fn(PartyKind, String, Option<String>) -> Result<ServiceResponse, SmError> + Send + Sync,
>;
type PartyUpdate =
    Arc<dyn Fn(PartyKind, String, String, Value) -> Result<ServiceResponse, SmError> + Send + Sync>;
type PartyDelete = Arc<
    dyn Fn(PartyKind, String, Option<String>) -> Result<ServiceResponse, SmError> + Send + Sync,
>;
type PartyMeta =
    Arc<dyn Fn(PartyKind, String) -> Result<Option<ResourceMeta>, SmError> + Send + Sync>;
type RelCreate = Arc<dyn Fn(Value) -> Result<ServiceResponse, SmError> + Send + Sync>;
type RelGet = Arc<dyn Fn(String, Option<String>) -> Result<ServiceResponse, SmError> + Send + Sync>;

// DEFINITION wire-op hooks — all native (SmError), matching the post-ADR-011
// dispatch: OPT retrieval via the SM `get_opt` seam; list/upload/example/query
// via the wire-shaped `DefinitionAdapter`.
type GetOpt = Arc<dyn Fn(String) -> Result<String, SmError> + Send + Sync>;
type ContributionCommitRaw =
    Arc<dyn Fn(Uuid, Value) -> Result<ServiceResponse, SmError> + Send + Sync>;
type TemplateExample =
    Arc<dyn Fn(String, Option<String>, Option<String>) -> Result<Value, SmError> + Send + Sync>;
type ValueListHook = Arc<dyn Fn() -> Result<Vec<Value>, SmError> + Send + Sync>;
type TemplateUploadHook = Arc<dyn Fn(String) -> Result<Value, SmError> + Send + Sync>;
type QueryListHook = Arc<dyn Fn(String) -> Result<Vec<Value>, SmError> + Send + Sync>;

// Terminology (SM I_TERMINOLOGY_SERVICE) wire-exposure hooks.
type TerminologyIds = Arc<dyn Fn() -> Result<Vec<String>, SmError> + Send + Sync>;
type TerminologyDescriptionHook =
    Arc<dyn Fn(String) -> Result<TerminologyDescription, SmError> + Send + Sync>;
// `Fn(terminology_id, code, at_date)` — the `attributes` allow-list is dropped
// on the wire (see the terminology dispatcher PORT NOTE), so the hook omits it.
type GetTerm = Arc<
    dyn Fn(String, String, Option<String>) -> Result<TerminologyExtract, SmError> + Send + Sync,
>;
type Subsumes = Arc<dyn Fn(String, String, String) -> Result<bool, SmError> + Send + Sync>;
type ValueSetValidate =
    Arc<dyn Fn(String, String, String, Option<String>) -> Result<bool, SmError> + Send + Sync>;
type GetValueSet = Arc<dyn Fn(String, String) -> Result<TerminologyExtract, SmError> + Send + Sync>;

/// The per-test override closures. Every field defaults to `None` (→ the
/// `501`/trait-default behaviour); a test populates only what it exercises.
#[derive(Default, Clone)]
pub struct Hooks {
    // EHR
    pub create_ehr: Option<CreateEhr>,
    pub create_ehr_with_id: Option<CreateEhrWithId>,
    pub ehr_object: Option<EhrObject>,
    pub ehr_object_for_subject: Option<EhrObjectForSubject>,
    // EHR_STATUS
    pub get_ehr_status: Option<StatusRead>,
    pub get_ehr_status_at_time: Option<StatusAtTime>,
    pub get_ehr_status_at_version: Option<StatusAtVersion>,
    pub ehr_status_version_at_time: Option<StatusAtTime>,
    pub replace_ehr_status: Option<ReplaceStatus>,
    // COMPOSITION
    pub get_composition_latest: Option<CompLatest>,
    pub get_composition_at_time: Option<CompAtTime>,
    pub get_composition_at_version: Option<CompAtVersion>,
    pub create_composition: Option<CreateComp>,
    pub update_composition: Option<UpdateComp>,
    pub delete_composition: Option<DeleteComp>,
    pub composition_latest_meta: Option<CompMeta>,
    // non-EHR SM-native
    pub web_template: Option<WebTemplateHook>,
    pub admin_ehr_delete: Option<AdminDelete>,
    pub admin_ehr_delete_all: Option<AdminDeleteAll>,
    pub get_artefact: Option<GetArtefact>,
    pub party_create: Option<PartyCreate>,
    pub party_get: Option<PartyGet>,
    pub party_update: Option<PartyUpdate>,
    pub party_delete: Option<PartyDelete>,
    pub demographic_latest_meta: Option<PartyMeta>,
    pub party_relationship_create: Option<RelCreate>,
    pub party_relationship_get: Option<RelGet>,
    // raw wire CONTRIBUTION commit (`ContributionAdapter::ehr_contribution_commit`)
    pub ehr_contribution_commit: Option<ContributionCommitRaw>,
    // DEFINITION wire ops (SM I_DEFINITION_* + DefinitionAdapter)
    pub get_opt: Option<GetOpt>,
    // adl1.4 wire GET: template_id-keyed (`DefinitionAdapter::template_adl14_get`;
    // the SM `get_opt` stays UUID-keyed — i_definition_adl14.adoc).
    pub template_adl14_get: Option<GetOpt>,
    pub template_adl14_list: Option<ValueListHook>,
    pub template_adl14_upload: Option<TemplateUploadHook>,
    pub template_adl14_example: Option<TemplateExample>,
    // adl2 upload: `Fn(source) -> stored HRID` (same shape as `GetOpt`).
    pub template_adl2_upload: Option<GetOpt>,
    pub template_adl2_list: Option<ValueListHook>,
    pub query_list: Option<QueryListHook>,
    // Terminology (SM I_TERMINOLOGY_SERVICE) — the wire-exposed calls.
    pub get_terminology_ids: Option<TerminologyIds>,
    pub get_terminology_description: Option<TerminologyDescriptionHook>,
    pub get_term: Option<GetTerm>,
    pub subsumes: Option<Subsumes>,
    pub value_set_validate: Option<ValueSetValidate>,
    pub get_value_set: Option<GetValueSet>,
    // SM System Log: an in-memory audit recorder. When set, the mock's
    // `SystemLog::emit` records every event (so a test can assert the ATNA
    // event a request produced); `audit_enabled()` is then true. Replaces the
    // old router-state `AuditSender` — the real syslog transport is tested in
    // `ehrbase::system_log` (the emitter now lives in the backend, ADR-011).
    pub audit: Option<AuditSink>,
}

/// An in-memory audit sink for the mock's [`SystemLog`] — records emitted
/// [`AuditEvent`]s so router tests can assert what the audit middleware sent,
/// and drives the [`EmitOutcome`] the middleware sees (for the fail-open /
/// fail-closed paths that the real `AuditSender`'s queue used to exercise).
#[derive(Clone)]
pub struct AuditSink {
    /// The events the middleware emitted, in order.
    pub events: Arc<std::sync::Mutex<Vec<AuditEvent>>>,
    /// Whether successful-login "Application Activity" records are suppressed.
    pub suppress_login: bool,
    /// The outcome `emit` reports (default `Enqueued`; `Dropped` = fail-open,
    /// `Rejected` = fail-closed → the middleware returns `503`).
    pub emit_outcome: EmitOutcome,
}

impl Default for AuditSink {
    fn default() -> Self {
        Self {
            events: Arc::default(),
            suppress_login: false,
            emit_outcome: EmitOutcome::Enqueued,
        }
    }
}

impl AuditSink {
    /// A fresh recording sink (login events not suppressed, `emit` enqueues).
    #[must_use]
    pub fn recording() -> Self {
        Self::default()
    }

    /// Suppress successful-login records.
    #[must_use]
    pub fn with_suppress_login(mut self, suppress: bool) -> Self {
        self.suppress_login = suppress;
        self
    }

    /// Set the [`EmitOutcome`] `emit` reports (`Dropped` fail-open / `Rejected`
    /// fail-closed).
    #[must_use]
    pub fn with_emit_outcome(mut self, outcome: EmitOutcome) -> Self {
        self.emit_outcome = outcome;
        self
    }

    /// The events recorded so far (cloned out).
    #[must_use]
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().expect("audit sink poisoned").clone()
    }
}

/// The shared mock platform. Cheap to clone (the hooks live behind an `Arc`).
#[derive(Clone)]
pub struct Mock {
    h: Arc<Hooks>,
}

impl std::fmt::Debug for Mock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Mock")
    }
}

impl Default for Mock {
    fn default() -> Self {
        Self::new()
    }
}

impl Mock {
    /// An all-`501` platform (the old `StubBackend`).
    pub fn new() -> Self {
        Self {
            h: Arc::new(Hooks::default()),
        }
    }

    /// A platform with the given per-test overrides.
    pub fn with(hooks: Hooks) -> Self {
        Self { h: Arc::new(hooks) }
    }
}

// ── EHR ─────────────────────────────────────────────────────────────────────

#[async_trait]
impl EhrService for Mock {
    async fn has_ehr(&self, _ehr_id: Uuid) -> Result<bool, SmError> {
        Err(not_impl())
    }
    async fn has_ehr_for_subject(&self, _s: SubjectRef) -> Result<bool, SmError> {
        Err(not_impl())
    }
    async fn create_ehr(&self, status: Option<Value>) -> Result<Uuid, SmError> {
        match &self.h.create_ehr {
            Some(f) => f(status),
            None => Err(not_impl()),
        }
    }
    async fn create_ehr_with_id(
        &self,
        ehr_id: Uuid,
        status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        match &self.h.create_ehr_with_id {
            Some(f) => f(ehr_id, status),
            None => Err(not_impl()),
        }
    }
    async fn create_ehr_for_subject(
        &self,
        _s: SubjectRef,
        _status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        Err(not_impl())
    }
    async fn create_ehr_for_subject_with_id(
        &self,
        _ehr_id: Uuid,
        _s: SubjectRef,
        _status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        Err(not_impl())
    }
    async fn get_ehr(&self, _ehr_id: Uuid) -> Result<EhrSummary, SmError> {
        Err(not_impl())
    }
    async fn get_ehrs_for_subject(&self, _s: SubjectRef) -> Result<Vec<EhrSummary>, SmError> {
        Err(not_impl())
    }
    async fn ehr_object(&self, ehr_id: Uuid) -> Result<Value, SmError> {
        match &self.h.ehr_object {
            Some(f) => f(ehr_id),
            None => Err(not_impl()),
        }
    }
    async fn ehr_object_for_subject(
        &self,
        subject_id: &str,
        subject_namespace: &str,
    ) -> Result<Value, SmError> {
        match &self.h.ehr_object_for_subject {
            Some(f) => f(subject_id.to_owned(), subject_namespace.to_owned()),
            None => Err(not_impl()),
        }
    }
}

// ── EHR_STATUS ────────────────────────────────────────────────────────────────

#[async_trait]
impl EhrStatusService for Mock {
    async fn has_ehr_status_version(&self, _e: Uuid, _v: Uuid) -> Result<bool, SmError> {
        Err(not_impl())
    }
    async fn get_ehr_status(&self, ehr_id: Uuid) -> Result<Value, SmError> {
        match &self.h.get_ehr_status {
            Some(f) => f(ehr_id),
            None => Err(not_impl()),
        }
    }
    async fn get_ehr_status_at_time(
        &self,
        ehr_id: Uuid,
        t: Option<String>,
    ) -> Result<Value, SmError> {
        match &self.h.get_ehr_status_at_time {
            Some(f) => f(ehr_id, t),
            None => Err(not_impl()),
        }
    }
    async fn get_ehr_status_at_version(
        &self,
        ehr_id: Uuid,
        v: Uuid,
        ver: i32,
    ) -> Result<Value, SmError> {
        match &self.h.get_ehr_status_at_version {
            Some(f) => f(ehr_id, v, ver),
            None => Err(not_impl()),
        }
    }
    async fn get_versioned_ehr_status(&self, _ehr_id: Uuid) -> Result<Value, SmError> {
        Err(not_impl())
    }
    async fn replace_ehr_status(
        &self,
        ehr_id: Uuid,
        status: UpdateVersion,
    ) -> Result<String, SmError> {
        match &self.h.replace_ehr_status {
            Some(f) => f(ehr_id, status),
            None => Err(not_impl()),
        }
    }
    async fn ehr_status_revision_history(&self, _ehr_id: Uuid) -> Result<Value, SmError> {
        Err(not_impl())
    }
    async fn ehr_status_version_at_time(
        &self,
        ehr_id: Uuid,
        t: Option<String>,
    ) -> Result<Value, SmError> {
        match &self.h.ehr_status_version_at_time {
            Some(f) => f(ehr_id, t),
            None => Err(not_impl()),
        }
    }
    async fn ehr_status_original_version(
        &self,
        _ehr_id: Uuid,
        _v: Uuid,
        _ver: i32,
    ) -> Result<Value, SmError> {
        Err(not_impl())
    }
}

// ── COMPOSITION ───────────────────────────────────────────────────────────────

#[async_trait]
impl EhrCompositionService for Mock {
    async fn has_composition(&self, _e: Uuid, _c: ObjectVersionId) -> Result<bool, SmError> {
        Err(not_impl())
    }
    async fn get_composition_latest(&self, ehr_id: Uuid, vo: Uuid) -> Result<Value, SmError> {
        match &self.h.get_composition_latest {
            Some(f) => f(ehr_id, vo),
            None => Err(not_impl()),
        }
    }
    async fn get_composition_at_time(
        &self,
        ehr_id: Uuid,
        vo: Uuid,
        t: Option<String>,
    ) -> Result<Value, SmError> {
        match &self.h.get_composition_at_time {
            Some(f) => f(ehr_id, vo, t),
            None => Err(not_impl()),
        }
    }
    async fn get_composition_at_version(
        &self,
        ehr_id: Uuid,
        ovid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        match &self.h.get_composition_at_version {
            Some(f) => f(ehr_id, ovid),
            None => Err(not_impl()),
        }
    }
    async fn get_versioned_composition(&self, _e: Uuid, _vo: Uuid) -> Result<Value, SmError> {
        Err(not_impl())
    }
    async fn create_composition(&self, ehr_id: Uuid, v: UpdateVersion) -> Result<String, SmError> {
        match &self.h.create_composition {
            Some(f) => f(ehr_id, v),
            None => Err(not_impl()),
        }
    }
    async fn update_composition(
        &self,
        ehr_id: Uuid,
        vo: Uuid,
        v: UpdateVersion,
    ) -> Result<String, SmError> {
        match &self.h.update_composition {
            Some(f) => f(ehr_id, vo, v),
            None => Err(not_impl()),
        }
    }
    async fn delete_composition(
        &self,
        ehr_id: Uuid,
        ovid: ObjectVersionId,
    ) -> Result<String, SmError> {
        match &self.h.delete_composition {
            Some(f) => f(ehr_id, ovid),
            None => Err(not_impl()),
        }
    }
    async fn composition_revision_history(&self, _e: Uuid, _vo: Uuid) -> Result<Value, SmError> {
        Err(not_impl())
    }
    async fn composition_version_at_time(
        &self,
        _e: Uuid,
        _vo: Uuid,
        _t: Option<String>,
    ) -> Result<Value, SmError> {
        Err(not_impl())
    }
    async fn composition_original_version(
        &self,
        _e: Uuid,
        _ovid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        Err(not_impl())
    }
}

// ── DIRECTORY ─────────────────────────────────────────────────────────────────

#[async_trait]
impl EhrDirectoryService for Mock {
    async fn has_directory(&self, _e: Uuid) -> Result<bool, SmError> {
        Err(not_impl())
    }
    async fn has_path(&self, _e: Uuid, _p: String) -> Result<bool, SmError> {
        Err(not_impl())
    }
    async fn create_directory(&self, _e: Uuid, _v: UpdateVersion) -> Result<String, SmError> {
        Err(not_impl())
    }
    async fn get_directory_at_time(
        &self,
        _e: Uuid,
        _t: Option<String>,
        _p: Option<String>,
    ) -> Result<Value, SmError> {
        Err(not_impl())
    }
    async fn update_directory(&self, _e: Uuid, _v: UpdateVersion) -> Result<String, SmError> {
        Err(not_impl())
    }
    async fn delete_directory(
        &self,
        _e: Uuid,
        _pre: Option<ObjectVersionId>,
    ) -> Result<(), SmError> {
        Err(not_impl())
    }
    async fn get_directory_at_version(
        &self,
        _e: Uuid,
        _ovid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        Err(not_impl())
    }
}

// ── CONTRIBUTION ──────────────────────────────────────────────────────────────

#[async_trait]
impl EhrContributionService for Mock {
    async fn has_contribution(&self, _e: Uuid, _c: Uuid) -> Result<bool, SmError> {
        Err(not_impl())
    }
    async fn get_contribution(&self, _e: Uuid, _c: Uuid) -> Result<Value, SmError> {
        Err(not_impl())
    }
    async fn commit_contribution(
        &self,
        _e: Uuid,
        _versions: Vec<UpdateVersion>,
        _audit: UpdateAudit,
    ) -> Result<String, SmError> {
        Err(not_impl())
    }
    async fn list_contributions(
        &self,
        _e: Uuid,
        _tr: ehrbase_sm::TimeRange,
        _page: ehrbase_sm::types::Page,
    ) -> Result<Vec<String>, SmError> {
        Err(not_impl())
    }
    async fn contribution_count(
        &self,
        _e: Uuid,
        _tr: ehrbase_sm::TimeRange,
    ) -> Result<i64, SmError> {
        Err(not_impl())
    }
}

// ── adapters (mandatory, no defaults) ─────────────────────────────────────────

#[async_trait]
impl ContributionAdapter for Mock {
    async fn ehr_contribution_commit(
        &self,
        an_ehr_id: Uuid,
        a_contribution: Value,
    ) -> Result<ServiceResponse, SmError> {
        match &self.h.ehr_contribution_commit {
            Some(f) => f(an_ehr_id, a_contribution),
            None => Err(not_impl()),
        }
    }
}

#[async_trait]
impl VersionMetaAdapter for Mock {
    async fn composition_latest_meta(
        &self,
        ehr_id: Uuid,
        vo: Uuid,
    ) -> Result<Option<ResourceMeta>, SmError> {
        match &self.h.composition_latest_meta {
            Some(f) => f(ehr_id, vo),
            None => Ok(None),
        }
    }
    async fn ehr_status_latest_meta(&self, _e: Uuid) -> Result<Option<ResourceMeta>, SmError> {
        Ok(None)
    }
    async fn directory_latest_meta(&self, _e: Uuid) -> Result<Option<ResourceMeta>, SmError> {
        Ok(None)
    }
}

#[async_trait]
impl ItemTagAdapter for Mock {
    async fn ehr_tags_get(
        &self,
        _e: Uuid,
        _key: Option<String>,
        _value: Option<String>,
        _target_path: Option<String>,
    ) -> Result<Vec<Value>, SmError> {
        Err(not_impl())
    }
    async fn target_tags_get(&self, _e: Uuid, _uid: String) -> Result<Vec<Value>, SmError> {
        Err(not_impl())
    }
    async fn target_tags_replace(
        &self,
        _e: Uuid,
        _uid: String,
        _kind: &str,
        _tags: Vec<Value>,
    ) -> Result<Vec<Value>, SmError> {
        Err(not_impl())
    }
    async fn target_tag_delete(&self, _e: Uuid, _uid: String, _key: String) -> Result<(), SmError> {
        Err(not_impl())
    }
}

// ── demographic (SM-native, defaults kept; hooks for the overridden methods) ──

#[async_trait]
impl DemographicService for Mock {
    async fn party_create(&self, kind: PartyKind, body: Value) -> Result<ServiceResponse, SmError> {
        match &self.h.party_create {
            Some(f) => f(kind, body),
            None => Err(not_impl()),
        }
    }
    async fn party_get(
        &self,
        kind: PartyKind,
        uid: String,
        at: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        match &self.h.party_get {
            Some(f) => f(kind, uid, at),
            None => Err(not_impl()),
        }
    }
    async fn party_update(
        &self,
        kind: PartyKind,
        uid: String,
        if_match: String,
        body: Value,
    ) -> Result<ServiceResponse, SmError> {
        match &self.h.party_update {
            Some(f) => f(kind, uid, if_match, body),
            None => Err(not_impl()),
        }
    }
    async fn party_delete(
        &self,
        kind: PartyKind,
        uid: String,
        if_match: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        match &self.h.party_delete {
            Some(f) => f(kind, uid, if_match),
            None => Err(not_impl()),
        }
    }
    async fn demographic_latest_meta(
        &self,
        kind: PartyKind,
        uid: String,
    ) -> Result<Option<ResourceMeta>, SmError> {
        match &self.h.demographic_latest_meta {
            Some(f) => f(kind, uid),
            None => Ok(None),
        }
    }
}

#[async_trait]
impl PartyRelationshipService for Mock {
    async fn party_relationship_create(&self, body: Value) -> Result<ServiceResponse, SmError> {
        match &self.h.party_relationship_create {
            Some(f) => f(body),
            None => Err(not_impl()),
        }
    }
    async fn party_relationship_get(
        &self,
        uid: String,
        at: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        match &self.h.party_relationship_get {
            Some(f) => f(uid, at),
            None => Err(not_impl()),
        }
    }
}

impl EhrIndexService for Mock {}

// ── definition (SM I_DEFINITION_* + wire-shaped DefinitionAdapter; ADR-011) ───

// OPT 1.4 retrieval is the SM `get_opt` seam (the dispatcher's adl1.4 GET);
// other SM `I_DEFINITION_ADL14` calls keep the trait defaults (501).
#[async_trait]
impl DefinitionAdl14Service for Mock {
    async fn get_opt(&self, an_opt_id: String) -> Result<String, SmError> {
        match &self.h.get_opt {
            Some(f) => f(an_opt_id),
            None => Err(not_impl()),
        }
    }
}

#[async_trait]
impl DefinitionAdl2Service for Mock {
    async fn get_artefact(&self, an_id: String) -> Result<String, SmError> {
        match &self.h.get_artefact {
            Some(f) => f(an_id),
            None => Err(not_impl()),
        }
    }
}

impl DefinitionQueryService for Mock {}

// The wire-shaped adapter (no trait defaults — every method is implemented).
#[async_trait]
impl DefinitionAdapter for Mock {
    async fn template_adl14_upload(&self, opt_xml: String) -> Result<Value, SmError> {
        match &self.h.template_adl14_upload {
            Some(f) => f(opt_xml),
            None => Err(not_impl()),
        }
    }
    async fn template_adl14_get(&self, template_id: String) -> Result<String, SmError> {
        match &self.h.template_adl14_get {
            Some(f) => f(template_id),
            None => Err(not_impl()),
        }
    }
    async fn template_adl14_list(&self) -> Result<Vec<Value>, SmError> {
        match &self.h.template_adl14_list {
            Some(f) => f(),
            None => Err(not_impl()),
        }
    }
    async fn template_adl14_example(
        &self,
        template_id: String,
        detail_level: Option<String>,
        kind: Option<String>,
    ) -> Result<Value, SmError> {
        match &self.h.template_adl14_example {
            Some(f) => f(template_id, detail_level, kind),
            None => Err(not_impl()),
        }
    }
    async fn template_adl2_upload(&self, source: String) -> Result<String, SmError> {
        match &self.h.template_adl2_upload {
            Some(f) => f(source),
            None => Err(not_impl()),
        }
    }
    async fn template_adl2_list(&self) -> Result<Vec<Value>, SmError> {
        match &self.h.template_adl2_list {
            Some(f) => f(),
            None => Err(not_impl()),
        }
    }
    async fn query_list(&self, qualified_query_name: String) -> Result<Vec<Value>, SmError> {
        match &self.h.query_list {
            Some(f) => f(qualified_query_name),
            None => Err(not_impl()),
        }
    }
    async fn query_version_get(
        &self,
        _qualified_query_name: String,
        _version: String,
    ) -> Result<Value, SmError> {
        Err(not_impl())
    }
    async fn query_store(
        &self,
        _qualified_query_name: String,
        _version: Option<String>,
        _body: String,
    ) -> Result<(), SmError> {
        Err(not_impl())
    }
}

// SM System Log — records to the optional in-memory sink. With no sink,
// auditing is disabled (the middleware early-returns) and `emit` is a no-op
// drop, reproducing a server booted without an audit trail.
impl SystemLog for Mock {
    fn emit(&self, event: AuditEvent) -> EmitOutcome {
        match &self.h.audit {
            Some(sink) => {
                sink.events.lock().expect("audit sink poisoned").push(event);
                sink.emit_outcome
            }
            None => EmitOutcome::Dropped,
        }
    }
    fn audit_enabled(&self) -> bool {
        self.h.audit.is_some()
    }
    fn suppress_login_events(&self) -> bool {
        self.h.audit.as_ref().is_some_and(|s| s.suppress_login)
    }
}

#[async_trait]
impl WebTemplateService for Mock {
    async fn web_template(&self, template_id: &str) -> Result<Arc<WebTemplate>, SmError> {
        match &self.h.web_template {
            Some(f) => f(template_id.to_owned()),
            None => Err(not_impl()),
        }
    }
}

impl QueryService for Mock {}

// ── admin (SM-native, defaults kept; hooks for the overridden methods) ────────

#[async_trait]
impl AdminService for Mock {
    async fn admin_ehr_delete(&self, ehr_id: String) -> Result<(), SmError> {
        match &self.h.admin_ehr_delete {
            Some(f) => f(ehr_id),
            None => Err(not_impl()),
        }
    }
    async fn admin_ehr_delete_all(&self, ehr_ids: Vec<String>) -> Result<u64, SmError> {
        match &self.h.admin_ehr_delete_all {
            Some(f) => f(ehr_ids),
            None => Err(not_impl()),
        }
    }
}

impl AdminArchive for Mock {}

// ── terminology (SM I_TERMINOLOGY_SERVICE; wire exposure per design 08 §7) ────
//
// Defaults kept (→ 501) except the six wire-exposed calls, which route through
// the optional per-test hooks.
#[async_trait]
impl TerminologyService for Mock {
    async fn get_terminology_ids(&self) -> Result<Vec<String>, SmError> {
        match &self.h.get_terminology_ids {
            Some(f) => f(),
            None => Err(not_impl()),
        }
    }
    async fn get_terminology_description(
        &self,
        terminology_id: &str,
    ) -> Result<TerminologyDescription, SmError> {
        match &self.h.get_terminology_description {
            Some(f) => f(terminology_id.to_owned()),
            None => Err(not_impl()),
        }
    }
    async fn get_term(
        &self,
        terminology_id: &str,
        code: &str,
        _attributes: Option<BTreeMap<String, String>>,
        at_date: Option<String>,
    ) -> Result<TerminologyExtract, SmError> {
        match &self.h.get_term {
            Some(f) => f(terminology_id.to_owned(), code.to_owned(), at_date),
            None => Err(not_impl()),
        }
    }
    async fn subsumes(
        &self,
        terminology_id: &str,
        ref_code: &str,
        candidate_child_code: &str,
    ) -> Result<bool, SmError> {
        match &self.h.subsumes {
            Some(f) => f(
                terminology_id.to_owned(),
                ref_code.to_owned(),
                candidate_child_code.to_owned(),
            ),
            None => Err(not_impl()),
        }
    }
    async fn value_set_validate(
        &self,
        terminology_id: &str,
        value_set_id: &str,
        candidate_code: &str,
        at_date: Option<String>,
    ) -> Result<bool, SmError> {
        match &self.h.value_set_validate {
            Some(f) => f(
                terminology_id.to_owned(),
                value_set_id.to_owned(),
                candidate_code.to_owned(),
                at_date,
            ),
            None => Err(not_impl()),
        }
    }
    async fn get_value_set(
        &self,
        terminology_id: &str,
        value_set_code: &str,
    ) -> Result<TerminologyExtract, SmError> {
        match &self.h.get_value_set {
            Some(f) => f(terminology_id.to_owned(), value_set_code.to_owned()),
            None => Err(not_impl()),
        }
    }
}
