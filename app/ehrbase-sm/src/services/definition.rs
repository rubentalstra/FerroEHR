//! The SM Definitions service interfaces for ADL 1.4 / ADL2 artefacts and
//! registered queries.
//!
//! Three Rust traits, one per SM interface:
//! - [`DefinitionAdl14Service`] realizes `I_DEFINITION_ADL14`
//!   (`docs/specs/openehr/SM/docs/UML/classes/i_definition_adl14.adoc`) — ADL
//!   1.4 source archetypes (keyed by `ARCHETYPE_ID`) and OPTs (keyed by `UUID`);
//! - [`DefinitionAdl2Service`] realizes `I_DEFINITION_ADL2`
//!   (`docs/specs/openehr/SM/docs/UML/classes/i_definition_adl2.adoc`) — ADL2
//!   artefacts (source archetypes, templates and OPTs), all archetype instances
//!   keyed by `ARCHETYPE_HRID` (`master04-definition_package.adoc`: "identified
//!   in the same way, via an Archetype human-readable identifier");
//! - [`DefinitionQueryService`] realizes `I_DEFINITION_QUERY`
//!   (`docs/specs/openehr/SM/docs/UML/classes/i_definition_query.adoc`) —
//!   registered queries addressed by qualified name (`master04-definition_package.adoc`).
//!
//! Every method defaults to `NotImplemented`, so [`StubBackend`] (and any
//! partial backend) inherits a `501` until the real service overrides them.
//!
//! PORT NOTE (interchange form): the SM signatures exchange AOM `ARCHETYPE`
//! objects (`upload_archetype(an_arch: ARCHETYPE)`, `get_archetype(): ARCHETYPE`,
//! …). openEHR has no BMM meta-model for AOM instances, so the native API
//! exchanges the **interchange serializations** the platform actually ingests
//! — ADL 1.4 *source text* for archetypes, OPT 1.4 *canonical XML* for OPTs —
//! and parsing happens inside the service, exactly as the ITS-REST wire does.
//!
//! [`StubBackend`]: crate::backend::StubBackend
//! [`Backend`]: crate::backend::Backend

use async_trait::async_trait;

use openehr_its::rest::runtime::ApiError;

use crate::types::{Page, QueryDescriptor};

/// The SM `I_DEFINITION_ADL14` interface — ADL 1.4 archetypes + OPTs
/// (`i_definition_adl14.adoc`).
///
/// Archetypes are keyed by their `ARCHETYPE_ID` string; OPTs by a `UUID`
/// string (`master04-definition_package.adoc`: "In ADL 1.4, archetypes are
/// identified with the older `ARCHETYPE_ID`, while OPTs are identified with
/// UUIDs").
#[async_trait]
pub trait DefinitionAdl14Service: Send + Sync {
    /// `has_archetype` — True if an ADL 1.4 archetype with id `an_id` exists.
    async fn has_archetype(&self, _an_id: String) -> Result<bool, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `valid_archetype` — test validity of the supplied ADL 1.4 source.
    async fn valid_archetype(&self, _adl: String) -> Result<bool, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `upload_archetype` — upload a valid ADL 1.4 archetype, replacing any
    /// existing one with the same id (`Post_has_archetype`). The archetype must
    /// be valid to succeed; an invalid one is `invalid_archetype` (→ `422`).
    async fn upload_archetype(&self, _adl: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `get_archetype` — the ADL 1.4 source of the archetype with id `an_id`;
    /// `artefact_does_not_exist` (→ `404`) if absent.
    async fn get_archetype(&self, _an_id: String) -> Result<String, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_archetypes` — the ids of all known ADL 1.4 archetypes.
    async fn list_archetypes(&self, _page: Page) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_matching_archetypes` — archetype ids matching `id_pattern` (a
    /// regex); `invalid_id_pattern` (→ `400`) if the pattern will not compile.
    async fn list_matching_archetypes(
        &self,
        _id_pattern: String,
        _page: Page,
    ) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `delete_archetype` — delete a previously uploaded archetype
    /// (`Pre_artefact_exists`, `Post_archetype_removed`); absent → `404`.
    async fn delete_archetype(&self, _an_id: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `archetypes_count` — total archetypes count.
    async fn archetypes_count(&self) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `has_opt` — True if an ADL 1.4 OPT with id `an_opt_id` (a `UUID` string)
    /// exists.
    async fn has_opt(&self, _an_opt_id: String) -> Result<bool, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `valid_opt` — test validity of the supplied OPT 1.4 canonical XML.
    async fn valid_opt(&self, _opt_xml: String) -> Result<bool, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `upload_opt` — upload an ADL 1.4 OPT (`Pre_valid`: it must be valid, else
    /// `invalid_template` → `422`).
    ///
    /// PORT NOTE: the SM is silent on OPT *replacement*; the ITS-REST wire's
    /// CNF-tested conflict rule wins — re-uploading an existing `template_id` is
    /// a `409` (`409_template_already_exists.yaml`;
    /// `I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict`).
    async fn upload_opt(&self, _opt_xml: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `get_opt` — the OPT 1.4 canonical XML of the OPT with id `an_opt_id`;
    /// `artefact_does_not_exist` (→ `404`) if absent.
    async fn get_opt(&self, _an_opt_id: String) -> Result<String, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_opts` — the ids (`UUID`s) of all known ADL 1.4 OPTs.
    async fn list_opts(&self, _page: Page) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_matching_opts` — OPTs whose identifiers match `id_pattern` (a
    /// regex); `invalid_id_pattern` (→ `400`) if the pattern will not compile.
    ///
    /// PORT NOTE (spec defect): the SM types the return as `List<ARCHETYPE_ID>`
    /// even though OPTs are UUID-keyed. We return the OPTs' `template_id`
    /// strings (the meaningful human-readable identifier the pattern is useful
    /// against) rather than a nonsensical `ARCHETYPE_ID` cast.
    async fn list_matching_opts(
        &self,
        _id_pattern: String,
        _page: Page,
    ) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `delete_opt` — delete a previously uploaded OPT (`Pre_has_opt`,
    /// `Post_opt_removed`); absent → `404`.
    async fn delete_opt(&self, _an_opt_id: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `opts_count` — total OPTs count.
    async fn opts_count(&self) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }
}

/// The SM `I_DEFINITION_ADL2` interface — ADL2 artefacts (`i_definition_adl2.adoc`).
///
/// In ADL2, archetypes and 'templates' are all archetype instances: source
/// archetypes, templates, and Operational Templates (OPTs) can all be uploaded,
/// and are "identified in the same way, via an Archetype human-readable
/// identifier (`ARCHETYPE_HRID`) and a UUID" (`master04-definition_package.adoc`
/// §Archetypes and Templates). Artefacts are therefore keyed by their
/// `ARCHETYPE_HRID` (e.g. `openEHR-EHR-OBSERVATION.bp.v1.0.0`, optionally
/// namespace-qualified), with a `kind` (archetype / template / OPT) so the
/// per-concrete-type list/count calls can filter.
///
/// PORT NOTE (interchange form): the SM signatures exchange AOM2
/// `AUTHORED_ARCHETYPE` objects (`upload_artefact(an_artefact: AUTHORED_ARCHETYPE)`,
/// `get_artefact(): AUTHORED_ARCHETYPE`). openEHR has no BMM meta-model for AOM
/// instances and the tree has no ADL2/cADL *source* parser yet (`am24` is
/// generated AOM2 types only), so the native API exchanges **ADL2 source text**
/// — the interchange serialization the platform actually ingests. Validity is
/// therefore a lightweight structural check (a recognised artefact header +
/// well-formed `ARCHETYPE_HRID`); full AOM2 validation lands when the ADL2
/// source parser does.
#[async_trait]
pub trait DefinitionAdl2Service: Send + Sync {
    /// `has_artefact` — True if an AOM2 artefact with `ARCHETYPE_HRID` `an_id`
    /// exists in the service.
    async fn has_artefact(&self, _an_id: String) -> Result<bool, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `valid_artefact` — test validity of the supplied ADL2 source (structural,
    /// per the trait PORT NOTE).
    async fn valid_artefact(&self, _adl2: String) -> Result<bool, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `upload_artefact` — upload a valid ADL2 artefact (archetype, template or
    /// `operational_template`). "If an artefact with the same physical
    /// identifier and namespace exists, replace it." `Pre_valid`: the artefact must
    /// validate, else `invalid artefact` (→ `422`).
    async fn upload_artefact(&self, _adl2: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `get_artefact` — the ADL2 source of the artefact with `ARCHETYPE_HRID`
    /// `an_id` (`Pre_artefact_exists`); absent → `artefact_does_not_exist`
    /// (→ `404`).
    async fn get_artefact(&self, _an_id: String) -> Result<String, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_artefacts` — the `ARCHETYPE_HRID`s of all known ADL2 artefacts.
    async fn list_artefacts(&self, _page: Page) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_archetypes` — HRIDs of artefacts whose concrete type is
    /// `AUTHORED_ARCHETYPE` (`kind = archetype`).
    async fn list_archetypes(&self, _page: Page) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_templates` — HRIDs of artefacts whose concrete type is `TEMPLATE`
    /// (`kind = template`).
    async fn list_templates(&self, _page: Page) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_opts` — HRIDs of artefacts whose concrete type is
    /// `OPERATIONAL_TEMPLATE` (`kind = operational_template`).
    async fn list_opts(&self, _page: Page) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_matching_artefacts` — HRIDs matching `id_pattern` (a regex);
    /// `invalid_id_pattern` (→ `400`) if the pattern will not compile.
    async fn list_matching_artefacts(
        &self,
        _id_pattern: String,
        _page: Page,
    ) -> Result<Vec<String>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `delete_artefact` — delete the AOM2 artefact with `ARCHETYPE_HRID`
    /// `an_id`; absent → `artefact_does_not_exist` (→ `404`).
    async fn delete_artefact(&self, _an_id: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `artefacts_count` — total artefacts count.
    async fn artefacts_count(&self) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `archetypes_count` — total archetypes count.
    async fn archetypes_count(&self) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `templates_count` — total templates count.
    async fn templates_count(&self) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `opts_count` — total OPTs count.
    async fn opts_count(&self) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }
}

/// The SM `I_DEFINITION_QUERY` interface — registered queries and query sets
/// (`i_definition_query.adoc`).
///
/// Queries are identified by qualified names (`master04-definition_package.adoc`):
/// `<namespace>::<query-name>` or `<namespace>::<formalism>::<query-name>`; when
/// no namespace is supplied, the namespace `"misc"` is assumed. A formalism is
/// given via `a_type` — case-insensitive, with an optional `::version` (the
/// major version `"1"` when absent), so `"AQL"`, `"aql"`, and `"AQL::1"` are
/// equivalent.
#[async_trait]
pub trait DefinitionQueryService: Send + Sync {
    /// `has_query` — True if the query with qualified name `a_query_name` is
    /// registered (the `"misc"` namespace is assumed when none is supplied).
    async fn has_query(&self, _a_query_name: String) -> Result<bool, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `valid_query` — True if `a_query_text` is a valid instance of the
    /// formalism named by `a_type`. Only AQL major-version 1 is a known
    /// formalism here (any other → `false`, which the SM sanctions:
    /// "matching one of: aql; any other string value").
    async fn valid_query(&self, _a_query_text: String, _a_type: String) -> Result<bool, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `store_query` — register a query under a qualified name, returning its
    /// [`QueryDescriptor`]. If no name is supplied, one is generated.
    ///
    /// PORT NOTE (spec naming): the SM precondition is written
    /// `is_valid_query(a_query_text)` but the actual function is
    /// `valid_query(text, type)` (a spec inconsistency); we enforce
    /// `valid_query` and reject an invalid query as `invalid_query` (→ `422`).
    async fn store_query(
        &self,
        _a_query_text: String,
        _a_type: String,
        _a_query_name: Option<String>,
    ) -> Result<QueryDescriptor, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `store_query_set` — register a query set.
    ///
    /// PORT NOTE: the SM entry is an explicit TODO ("TODO: determine details",
    /// `i_definition_query.adoc`); with no defined semantics this stays
    /// `NotImplemented` (→ `501`) until the spec defines it.
    async fn store_query_set(&self, _a_query_set_name: Option<String>) -> Result<String, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_queries` — all registered queries.
    async fn list_queries(&self, _page: Page) -> Result<Vec<QueryDescriptor>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `list_matching_queries` — registered queries whose qualified name matches
    /// `id_pattern` (a regex) and whose referenced artefact identifiers match
    /// `artefact_id_pattern` (a regex; `None` = match any). `invalid_id_pattern`
    /// (→ `400`) if a pattern will not compile.
    async fn list_matching_queries(
        &self,
        _id_pattern: String,
        _artefact_id_pattern: Option<String>,
        _page: Page,
    ) -> Result<Vec<QueryDescriptor>, ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `delete_query` — delete the query with qualified name `a_query_name`
    /// (`Pre_has_query`, `Post_query_deleted`); absent → `404`.
    async fn delete_query(&self, _a_query_name: String) -> Result<(), ApiError> {
        Err(ApiError::NotImplemented)
    }

    /// `queries_count` — total count of queries.
    async fn queries_count(&self) -> Result<i64, ApiError> {
        Err(ApiError::NotImplemented)
    }
}
