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
use crate::types::ResourceMeta;

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
