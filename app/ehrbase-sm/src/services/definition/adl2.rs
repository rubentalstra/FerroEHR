//! `I_DEFINITION_ADL2` (`i_definition_adl2.adoc`) — "Interface to ADL2
//! definitions (i.e. models) in an EHR 'system'."

use async_trait::async_trait;

use crate::common::{Page, SmError};

/// `I_DEFINITION_ADL2` — ADL2 artefacts, one Rust method per SM call.
///
/// "In the ADL2 case, archetypes and 'templates' are all instances of
/// archetypes, formally speaking, which means that both source artefacts and
/// Operational Templates (OPTs) can be uploaded. All such artefacts are
/// identified in the same way, via an Archetype human-readable identifier
/// (`ARCHETYPE_HRID`) and a UUID" (`master04-definition_package.adoc`
/// §Archetypes and Templates). Artefacts are keyed by `ARCHETYPE_HRID`
/// (e.g. `openEHR-EHR-OBSERVATION.bp.v1.0.0`), with a concrete kind
/// (archetype / template / `template_overlay` / `operational_template`) so
/// the per-kind list/count calls can filter.
///
/// The artefact payload is ADL2 source text (module PORT NOTE on interchange
/// forms). Full AOM2 semantic validation is the W-4 mandate
/// (`docs/plans/WORKLIST.md`); until it lands, `valid_artefact` is the
/// documented structural subset.
///
/// No default method bodies (compile-time completeness).
#[async_trait]
pub trait DefinitionAdl2Service: Send + Sync {
    /// `has_artefact (an_id: ARCHETYPE_HRID): Boolean` — "True if AOM2
    /// artefact with id `an_id` exists in the service."
    async fn has_artefact(&self, an_id: String) -> Result<bool, SmError>;

    /// `valid_artefact (an_artefact: AUTHORED_ARCHETYPE): Boolean` — "Test
    /// validity of artefact" (ADL2 source text).
    async fn valid_artefact(&self, adl2: String) -> Result<bool, SmError>;

    /// `upload_artefact (an_artefact: AUTHORED_ARCHETYPE)` with
    /// `__Pre_valid__: valid_artefact (an_arch)` + `__Post_has_artefact__:
    /// has_artefact (an_arch.identifier)` — "Upload an ADL2 artefact, i.e.
    /// archetype, template or `operational_template`. If an artefact with the
    /// same physical identifier and namespace exists, replace it. The
    /// artefact must validate." Error `invalid_artefact` (+ specific
    /// messages; → `422`).
    ///
    /// PORT NOTE: the wire conflict rule (409 on re-upload) diverges from the
    /// SM's "replace it" — a deliberate wire/SM split recorded at the adapter;
    /// the native call follows the SM and replaces.
    async fn upload_artefact(&self, adl2: String) -> Result<(), SmError>;

    /// `get_artefact (an_id: ARCHETYPE_HRID): AUTHORED_ARCHETYPE` with
    /// `__Pre_artefact_exists__` — the ADL2 source of the artefact. Error
    /// `artefact_does_not_exist` (→ `404`).
    async fn get_artefact(&self, an_id: String) -> Result<String, SmError>;

    /// `list_artefacts (item_offset [0..1], items_to_fetch [0..1]):
    /// List<ARCHETYPE_HRID>` — "List all AOM2 artefacts known in the service."
    async fn list_artefacts(&self, page: Page) -> Result<Vec<String>, SmError>;

    /// `list_archetypes` — "artefacts whose concrete type is
    /// `AUTHORED_ARCHETYPE`."
    async fn list_archetypes(&self, page: Page) -> Result<Vec<String>, SmError>;

    /// `list_templates` — "artefacts whose concrete type is `TEMPLATE`."
    async fn list_templates(&self, page: Page) -> Result<Vec<String>, SmError>;

    /// `list_opts` — "artefacts whose concrete type is
    /// `OPERATIONAL_TEMPLATE`."
    async fn list_opts(&self, page: Page) -> Result<Vec<String>, SmError>;

    /// `list_matching_artefacts (id_pattern: String, …):
    /// List<ARCHETYPE_HRID>` — "List all artefacts whose identifiers match a
    /// regex pattern." Error `invalid_id_pattern` (→ `400`).
    async fn list_matching_artefacts(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError>;

    /// `delete_artefact (an_id: ARCHETYPE_HRID)` — "Delete the AOM2 artefact
    /// with id `an_id`." Error `artefact_does_not_exist` (→ `404`).
    async fn delete_artefact(&self, an_id: String) -> Result<(), SmError>;

    /// `artefacts_count (): Integer` — "Return total artefacts count."
    async fn artefacts_count(&self) -> Result<i64, SmError>;

    /// `archetypes_count (): Integer` — "Return total archetypes count."
    async fn archetypes_count(&self) -> Result<i64, SmError>;

    /// `templates_count (): Integer` — "Return total templates count."
    async fn templates_count(&self) -> Result<i64, SmError>;

    /// `opts_count (): Integer` — "Return total OPTs count."
    async fn opts_count(&self) -> Result<i64, SmError>;
}
