//! ITS-REST **adapter-support** extension traits — calls the openEHR SM does
//! **not** define, segregated here so the SM catalog traits stay pure.
//!
//! PORT NOTE: none of these are SM interface calls. They exist because the
//! ITS-REST 1.0.3 wire needs them: the `*_latest_meta` seams decorate a
//! `409`/`412` response with the current `version_uid` in `ETag`/`Location`
//! (`409_COMPOSITION_with_uid_based_id.yaml` / `412_*.yaml`), and the item-tag
//! CRUD is `EHRbase`'s experimental tag extension — neither has an SM call. The
//! platform component implements them beside the SM catalog; the adapter
//! dispatches to them for the wire routes that need them.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::error::SmError;
use crate::tenant::TenantContext;
use crate::types::{ResourceMeta, ServiceResponse};

/// Current-version metadata for the `409`/`412` `ETag`/`Location` decoration.
#[async_trait]
pub trait VersionMetaAdapter: Send + Sync {
    /// The current COMPOSITION version metadata (latest `version_uid`), for a
    /// `409`/`412` on `update`/`delete`. `None` if unknown.
    async fn composition_latest_meta(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Option<ResourceMeta>, SmError>;

    /// The current `EHR_STATUS` version metadata, for a `412` on
    /// `PUT /ehr_status`.
    async fn ehr_status_latest_meta(
        &self,
        an_ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, SmError>;

    /// The current directory FOLDER version metadata, for a `412` on
    /// `PUT`/`DELETE /directory`.
    async fn directory_latest_meta(&self, an_ehr_id: Uuid)
    -> Result<Option<ResourceMeta>, SmError>;
}

/// ITS-REST **Definitions** adapter-support extension — the wire-shaped
/// template + stored-query operations the ITS-REST `DEFINITION` group needs
/// that the SM `I_DEFINITION_*` interfaces do not express directly.
///
/// PORT NOTE (ADR-011): the SM Definitions interfaces
/// ([`DefinitionAdl14Service`](super::DefinitionAdl14Service) /
/// [`DefinitionAdl2Service`](super::DefinitionAdl2Service) /
/// [`DefinitionQueryService`](super::DefinitionQueryService)) exchange plain
/// identifiers and counts (`list_opts(): List<UUID>`, `get_opt(): String`,
/// …). The ITS-REST wire, by contrast, returns *rich* metadata objects
/// (`TEMPLATE` summaries, `StoredQuery` descriptors) and a generated example
/// `COMPOSITION`, none of which the SM catalog defines. Those wire-only shapes
/// live here as `serde_json::Value`, so the generated ITS-REST `DefinitionApi`
/// no longer needs to be part of [`Platform`](crate::Platform) and
/// `ehrbase-sm` stays protocol-free. The `get_opt`/`get_artefact` retrievals
/// (which *do* match the SM shape) are still served through the SM traits.
#[async_trait]
pub trait DefinitionAdapter: Send + Sync {
    /// `POST …/definition/template/adl1.4` — ingest an OPT 1.4 canonical-XML
    /// template, returning the wire template summary (`201` body).
    async fn template_adl14_upload(&self, opt_xml: String) -> Result<Value, SmError>;

    /// `GET …/definition/template/adl1.4` — the list of stored OPT 1.4
    /// templates as wire summary objects.
    async fn template_adl14_list(&self) -> Result<Vec<Value>, SmError>;

    /// `GET …/definition/template/adl1.4/{template_id}` — the stored OPT 1.4
    /// canonical XML addressed by its **`template_id` string** (the wire's
    /// address; the SM `get_opt` is `UUID`-keyed, `i_definition_adl14.adoc`
    /// `get_opt(an_opt_id: UUID)` — the two addressings cannot be conflated).
    /// Unknown template → `artefact_does_not_exist` (`404`).
    async fn template_adl14_get(&self, template_id: String) -> Result<String, SmError>;

    /// `GET …/definition/template/adl1.4/{template_id}/example` — a generated
    /// example `COMPOSITION` for the template. `detail_level`/`kind` are the
    /// raw dev-OAS query values (`example_detail_level`/`example_type`); an
    /// out-of-enum value is `precondition_violation` (→ `400`).
    async fn template_adl14_example(
        &self,
        template_id: String,
        detail_level: Option<String>,
        kind: Option<String>,
    ) -> Result<Value, SmError>;

    /// `POST …/definition/template/adl2` — ingest an ADL2 operational-template
    /// source, returning the stored `ARCHETYPE_HRID` (for the `Location`
    /// header + `Prefer` body).
    async fn template_adl2_upload(&self, source: String) -> Result<String, SmError>;

    /// `GET …/definition/template/adl2` — the list of stored ADL2 templates as
    /// wire summary objects.
    async fn template_adl2_list(&self) -> Result<Vec<Value>, SmError>;

    /// `GET …/definition/query/{qualified_query_name}` — the registered
    /// queries under this qualified name, as wire `StoredQuery` descriptors.
    async fn query_list(&self, qualified_query_name: String) -> Result<Vec<Value>, SmError>;

    /// `GET …/definition/query/{qualified_query_name}/{version}` — the
    /// registered query at the given SEMVER, as a wire `StoredQuery`
    /// descriptor; `versioned_object_does_not_exist` (→ `404`) if absent.
    async fn query_version_get(
        &self,
        qualified_query_name: String,
        version: String,
    ) -> Result<Value, SmError>;

    /// `PUT …/definition/query/{qualified_query_name}[/{version}]` — register
    /// the AQL text `body` under the qualified name (and optional SEMVER).
    async fn query_store(
        &self,
        qualified_query_name: String,
        version: Option<String>,
        body: String,
    ) -> Result<(), SmError>;
}

/// The experimental item-tag CRUD extension (`EHRbase`; no SM call).
#[async_trait]
pub trait ItemTagAdapter: Send + Sync {
    /// `GET /ehr/{ehr_id}/tags` — all item tags in the EHR, filtered by the
    /// optional `key`/`value`/`target_path`.
    async fn ehr_tags_get(
        &self,
        an_ehr_id: Uuid,
        key: Option<String>,
        value: Option<String>,
        target_path: Option<String>,
    ) -> Result<Vec<Value>, SmError>;

    /// `GET …/{target}/{uid_based_id}/tags` — the tags on a versioned target
    /// (COMPOSITION or `EHR_STATUS`).
    async fn target_tags_get(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
    ) -> Result<Vec<Value>, SmError>;

    /// `PUT …/{target}/{uid_based_id}/tags` — replace the tags on a versioned
    /// target; `target_type` is the RM type name (`COMPOSITION`/`EHR_STATUS`).
    async fn target_tags_replace(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, SmError>;

    /// `DELETE …/{target}/{uid_based_id}/tags/{key}` — delete one tag by key.
    async fn target_tag_delete(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
        key: String,
    ) -> Result<(), SmError>;
}

/// **Event-subscription** admin extension — CRUD over the event-filter
/// subscription store (ADR-014 §5, "Event Trigger" parity).
///
/// PORT NOTE: not an SM interface call. Event/subscription semantics are
/// spec-silent (ADR-014 §5 is the design record filling that seam), so there is
/// no `I_*` interface to transcribe. The subscriptions are a server-side filter
/// model the broker fans out on (a durable per-subscription queue bound with a
/// topic key built from the predicates); their CRUD is exposed through a
/// config-gated admin extension surface in `ehrbase-rest` (like the terminology
/// group), dispatching to this adapter. Bodies are `serde_json::Value` (the
/// subscription is a small predicate record, not an RM type). No default bodies
/// — the platform component implements every method (ADR-011).
#[async_trait]
pub trait EventSubscriptionAdapter: Send + Sync {
    /// `GET …/admin/event_subscription` — every stored subscription as a JSON
    /// record (`{id, name, kind, change_type, template_id, archetype, enabled,
    /// created_at}`; NULL predicate = wildcard).
    async fn event_subscription_list(&self) -> Result<Vec<Value>, SmError>;

    /// `POST …/admin/event_subscription` — create a subscription from the JSON
    /// body (`name` required; the four predicates optional/NULL = wildcard).
    /// Returns the stored record (with its generated `id`) for the `201` body.
    /// A malformed body / duplicate name is a `precondition_violation` (`400`).
    async fn event_subscription_create(&self, a_subscription: Value) -> Result<Value, SmError>;

    /// `GET …/admin/event_subscription/{id}` — the stored subscription;
    /// `versioned_object_does_not_exist` (`404`) if unknown.
    async fn event_subscription_get(&self, a_subscription_id: Uuid) -> Result<Value, SmError>;

    /// `PUT …/admin/event_subscription/{id}` — replace the subscription's
    /// predicates/enabled from the JSON body, returning the updated record;
    /// `versioned_object_does_not_exist` (`404`) if unknown.
    async fn event_subscription_update(
        &self,
        a_subscription_id: Uuid,
        a_subscription: Value,
    ) -> Result<Value, SmError>;

    /// `DELETE …/admin/event_subscription/{id}` — remove the subscription;
    /// `versioned_object_does_not_exist` (`404`) if unknown.
    async fn event_subscription_delete(&self, a_subscription_id: Uuid) -> Result<(), SmError>;
}

/// **Tenant** admin extension — CRUD over the tenant registry, plus the
/// claim/header → context resolution the middleware needs (ADR-015 §5).
///
/// PORT NOTE: not an SM interface call. The tenancy model is spec-silent
/// (ADR-015 fills it), so — like [`EventSubscriptionAdapter`] — there is no
/// `I_*` interface to transcribe; the CRUD is a config-gated `/admin/tenant`
/// extension in `ehrbase-rest`, bodies are `serde_json::Value` (a tenant is a
/// small `{name, system_id}` record). [`Self::tenant_resolve`] is the
/// middleware seam, not a wire route: it maps a JWT-claim / header value (a
/// tenant name or uuid) to the [`TenantContext`] that scopes the request. No
/// default bodies — the platform component implements every method (ADR-011).
#[async_trait]
pub trait TenantAdapter: Send + Sync {
    /// `GET …/admin/tenant` — every tenant as a JSON record
    /// (`{id, name, system_id, created_at}`).
    async fn tenant_list(&self) -> Result<Vec<Value>, SmError>;

    /// `POST …/admin/tenant` — create a tenant from the JSON body (`name` +
    /// `system_id` required). Returns the stored record (with its generated
    /// `id`) for the `201` body. A malformed body is a `precondition_violation`
    /// (`400`); a duplicate name is a `409`.
    async fn tenant_create(&self, a_tenant: Value) -> Result<Value, SmError>;

    /// `GET …/admin/tenant/{id}` — the stored tenant; `404` if unknown.
    async fn tenant_get(&self, a_tenant_id: Uuid) -> Result<Value, SmError>;

    /// `PUT …/admin/tenant/{id}` — update the tenant's `name`/`system_id` from
    /// the JSON body, returning the updated record; `404` if unknown.
    async fn tenant_update(&self, a_tenant_id: Uuid, a_tenant: Value) -> Result<Value, SmError>;

    /// `DELETE …/admin/tenant/{id}` — remove the tenant; `404` if unknown.
    /// Deletion is refused (`409`) unless the tenant owns no rows and is not the
    /// reserved default tenant (ADR-015 §5 — physical purge of a non-empty
    /// tenant goes through the per-EHR admin delete machinery first).
    async fn tenant_delete(&self, a_tenant_id: Uuid) -> Result<(), SmError>;

    /// Resolve a claim/header value (a tenant name or uuid string) to its
    /// [`TenantContext`], or `None` if unknown. The tenant-resolution middleware
    /// calls this once per request; it is not exposed as a wire route.
    async fn tenant_resolve(&self, key: &str) -> Result<Option<TenantContext>, SmError>;
}

/// **FHIR-connector** extension — mapping-store CRUD plus the inbound
/// `POST /fhir/r4/{resourceType}` ingest (ADR-016).
///
/// PORT NOTE (ADR-016): not an SM interface call. FHIR↔openEHR mapping is
/// spec-silent (ADR-016 is the design record), so — like
/// [`EventSubscriptionAdapter`] / [`TenantAdapter`] — there is no `I_*`
/// interface to transcribe; the surface is a config-gated `/fhir/r4/*` +
/// `/admin/fhir_mapping` extension in `ehrbase-rest` (off by default). Mapping
/// bodies are `serde_json::Value` (a mapping is a small deployable data record,
/// ADR-016 §Decision 2). [`Self::fhir_ingest`] resolves a mapping by resource
/// type + profile, builds a COMPOSITION, and commits it through the platform's
/// NORMAL validated create path (never a bypass, ADR-016 §Decision 3) with
/// `FEEDER_AUDIT` provenance; it returns the committed [`ServiceResponse`]
/// (the composition body + its resource metadata for the wire's `Location`/
/// `ETag`). No default bodies — the platform component implements every method
/// (ADR-011).
#[async_trait]
pub trait FhirConnectorAdapter: Send + Sync {
    /// `GET …/admin/fhir_mapping` — every stored mapping as a JSON record
    /// (`{id, name, resource_type, profile_url, template_id, definition,
    /// enabled, created_at}`).
    async fn fhir_mapping_list(&self) -> Result<Vec<Value>, SmError>;

    /// `POST …/admin/fhir_mapping` — create a mapping from the JSON body
    /// (`name`, `resource_type`, `template_id`, `definition` required; the
    /// `definition` is validated against the connector's schema on upload).
    /// Returns the stored record (with its generated `id`) for the `201` body.
    /// A malformed body is a `precondition_violation` (`400`); a duplicate name
    /// is a `409`; an unknown `template_id` is a `precondition_violation`.
    async fn fhir_mapping_create(&self, a_mapping: Value) -> Result<Value, SmError>;

    /// `GET …/admin/fhir_mapping/{id}` — the stored mapping;
    /// `versioned_object_does_not_exist` (`404`) if unknown.
    async fn fhir_mapping_get(&self, a_mapping_id: Uuid) -> Result<Value, SmError>;

    /// `PUT …/admin/fhir_mapping/{id}` — replace the mapping's fields from the
    /// JSON body, returning the updated record; `404` if unknown.
    async fn fhir_mapping_update(
        &self,
        a_mapping_id: Uuid,
        a_mapping: Value,
    ) -> Result<Value, SmError>;

    /// `DELETE …/admin/fhir_mapping/{id}` — remove the mapping; `404` if
    /// unknown.
    async fn fhir_mapping_delete(&self, a_mapping_id: Uuid) -> Result<(), SmError>;

    /// `POST …/fhir/r4/{resource_type}` — ingest a FHIR R4 resource: resolve
    /// the mapping by `resource_type` (+ optional `profile` from the resource's
    /// `meta.profile`), build a COMPOSITION from the mapping definition, and
    /// commit it through the NORMAL validated create path with `FEEDER_AUDIT`
    /// provenance (ADR-016 §Decision 3). Returns the committed composition +
    /// its resource metadata. No enabled mapping for the type/profile →
    /// `versioned_object_does_not_exist` (`404`); a resource that maps to an
    /// invalid COMPOSITION is rejected by the validator (`content_invalid` →
    /// `422`), never partially stored (ADR-016 §Decision 6).
    async fn fhir_ingest(
        &self,
        resource_type: String,
        profile: Option<String>,
        a_resource: Value,
    ) -> Result<ServiceResponse, SmError>;

    /// `GET …/fhir/r4/{resource_type}?patient=<ehr-subject-or-id>[&_count=N]` —
    /// the **read façade** (ADR-016 §Decision 4b): resolve every enabled mapping
    /// for the type, run the mapped template's COMPOSITION query (scoped to the
    /// `patient`'s EHR) through the platform's query seam, reverse-map each hit
    /// to a FHIR resource, and return a FHIR `searchset` **Bundle** (`total`,
    /// `entry[].fullUrl`, `entry[].resource`). Read-only and stateless — no FHIR
    /// persistence, no generic FHIR Search (only this explicit `patient` scope,
    /// ADR-016 §Decision 5). `patient` is mandatory (the protocol edge rejects a
    /// missing one `400`); a type with no enabled mapping yields an empty
    /// (`total: 0`) Bundle, not an error.
    async fn fhir_search(
        &self,
        resource_type: String,
        patient: String,
        count: Option<i64>,
    ) -> Result<Value, SmError>;
}

/// ITS-REST **CONTRIBUTION** adapter-support extension — the raw-wire
/// EHR-scoped CONTRIBUTION commit.
///
/// PORT NOTE (ADR-011): the SM `I_EHR_CONTRIBUTION.commit_contribution`
/// (`Vec<UpdateVersion>, UpdateAudit`) is a *typed subset* of the ITS-REST
/// wire CONTRIBUTION: `UPDATE_VERSION` mandates `data` + `lifecycle_state`
/// (SM `update_version.adoc`, both `1..1`) and a committer, so it cannot
/// represent an attestation-only (`666`) member, a delete (`523`) member
/// (`"data": null`), or a member inheriting `committer`/`system_id` from the
/// CONTRIBUTION audit (RM common `master06-change_control_package.adoc`
/// §Committal m4). The wire fields are also RM-shaped (`lifecycle_state`/
/// `change_type` are `DV_CODED_TEXT`, ITS-REST `UpdateVersion.yaml`), not the
/// SM's `Terminology_code`. `POST …/contribution` therefore commits the raw
/// wire body through this seam; all RM `change_control` semantics live in the
/// platform's shared commit path.
#[async_trait]
pub trait ContributionAdapter: Send + Sync {
    /// `POST …/ehr/{ehr_id}/contribution` — commit a wire CONTRIBUTION
    /// atomically. Returns the stored `CONTRIBUTION` body with its resource
    /// metadata (the contribution uid for the `201` `ETag`/`Location`).
    async fn ehr_contribution_commit(
        &self,
        an_ehr_id: Uuid,
        a_contribution: Value,
    ) -> Result<ServiceResponse, SmError>;
}

/// ITS-REST **multimedia expansion** adapter-support extension (ADR-017).
///
/// PORT NOTE: not an SM interface call. When `DV_MULTIMEDIA` externalization is
/// enabled (ADR-017), a stored COMPOSITION serves its large media by reference
/// (`uri` + integrity fields) by default; the `?expand_multimedia=true` query
/// parameter on a composition GET asks the server to re-inline the bytes,
/// verifying each blob's SHA-256 before serving (a mismatch is a `500`, never
/// silent corruption). The default implementation is a no-op passthrough — a
/// platform without externalization returns the body unchanged.
#[async_trait]
pub trait MultimediaAdapter: Send + Sync {
    /// Re-inline externalized `DV_MULTIMEDIA` blobs in a canonical composition
    /// `body`, verifying integrity. A no-op when externalization is disabled or
    /// the body carries no externalized media.
    async fn expand_multimedia(&self, body: Value) -> Result<Value, SmError> {
        Ok(body)
    }
}
