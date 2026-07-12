//! `I_DEFINITION_ADL14` (`i_definition_adl14.adoc`) — "Interface to ADL 1.4
//! definitions (i.e. archetypes and OPTs) in an EHR 'system'."

use async_trait::async_trait;

use crate::common::{Page, SmError};

/// `I_DEFINITION_ADL14` — ADL 1.4 archetypes + OPTs, one Rust method per SM
/// call.
///
/// "For ADL 1.4, 'templates' are distinct artefacts, and the service enables
/// the upload of source archetypes and ADL 1.4-based OPTs, which are XML
/// artefacts. In ADL 1.4, archetypes are identified with the older
/// `ARCHETYPE_ID`, while OPTs are identified with UUIDs"
/// (`master04-definition_package.adoc` §Archetypes and Templates).
///
/// No default method bodies (compile-time completeness): a backend that does
/// not implement a call is a build error, not a silent runtime stub.
#[async_trait]
pub trait DefinitionAdl14Service: Send + Sync {
    /// `has_archetype (an_id: ARCHETYPE_ID): Boolean` — "True if an ADL 1.4
    /// archetype with id `an_id` exists in the service."
    async fn has_archetype(&self, an_id: String) -> Result<bool, SmError>;

    /// `valid_archetype (an_arch: ARCHETYPE): Boolean` — "Test validity of
    /// archetype `an_arch`" (supplied as ADL 1.4 source text — module PORT
    /// NOTE).
    async fn valid_archetype(&self, adl: String) -> Result<bool, SmError>;

    /// `upload_archetype (an_arch: ARCHETYPE)` with `__Post_has_archetype__:
    /// has_archetype (an_arch.identifier)` — "Upload a valid ADL 1.4
    /// archetype. If an archetype with the same id already exists, replace
    /// it. The archetype must be valid to succeed." Error `invalid_archetype`
    /// (→ `422`).
    async fn upload_archetype(&self, adl: String) -> Result<(), SmError>;

    /// `get_archetype (an_id: ARCHETYPE_ID): ARCHETYPE` — the ADL 1.4 source
    /// of the archetype with id `an_id`. Error `artefact_does_not_exist`
    /// (→ `404`).
    async fn get_archetype(&self, an_id: String) -> Result<String, SmError>;

    /// `list_archetypes (item_offset [0..1], items_to_fetch [0..1]):
    /// List<ARCHETYPE_ID>` — "List all ADL 1.4 archetypes known in the
    /// service."
    async fn list_archetypes(&self, page: Page) -> Result<Vec<String>, SmError>;

    /// `list_matching_archetypes (id_pattern: String, item_offset [0..1],
    /// items_to_fetch [0..1]): List<ARCHETYPE_ID>` — "List all archetypes
    /// whose identifiers match a regex pattern." Error `invalid_id_pattern`
    /// (→ `400`).
    async fn list_matching_archetypes(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError>;

    /// `delete_archetype (an_id: ARCHETYPE_ID)` with `__Pre_artefact_exists__:
    /// has_artefact (an_id)` + `__Post_archetype_removed__: not has_archetype
    /// (an_id)` — "Delete a previously uploaded archetype"; absent → `404`.
    async fn delete_archetype(&self, an_id: String) -> Result<(), SmError>;

    /// `archetypes_count (): Integer` — "Return total archetypes count."
    async fn archetypes_count(&self) -> Result<i64, SmError>;

    /// `has_opt (an_opt_id: UUID): Boolean` — "True if ADL 1.4 OPT with id
    /// `an_opt_id` exists in the service."
    async fn has_opt(&self, an_opt_id: String) -> Result<bool, SmError>;

    /// `valid_opt (an_opt: ARCHETYPE): Boolean` — "Test validity of OPT
    /// `an_opt`" (supplied as OPT 1.4 canonical XML — module PORT NOTE).
    async fn valid_opt(&self, opt_xml: String) -> Result<bool, SmError>;

    /// `upload_opt (an_opt: ARCHETYPE)` with `__Pre_valid__: valid_opt(an_opt)`
    /// — "Upload an ADL 1.4 Operational Template (OPT)." Error
    /// `invalid_template` (→ `422`).
    ///
    /// PORT NOTE: the SM is silent on OPT *replacement*; the ITS-REST wire's
    /// CNF-tested conflict rule wins — re-uploading an existing `template_id`
    /// is a `409` (`409_template_already_exists.yaml`).
    async fn upload_opt(&self, opt_xml: String) -> Result<(), SmError>;

    /// `get_opt (an_opt_id: UUID): ARCHETYPE` — the OPT 1.4 canonical XML of
    /// the OPT with id `an_opt_id`. Error `artefact_does_not_exist` (→ `404`).
    async fn get_opt(&self, an_opt_id: String) -> Result<String, SmError>;

    /// `list_opts (item_offset [0..1], items_to_fetch [0..1]): List<UUID>` —
    /// "List all ADL 1.4 OPTs known in the service."
    async fn list_opts(&self, page: Page) -> Result<Vec<String>, SmError>;

    /// `list_matching_opts (id_pattern: String, item_offset [0..1],
    /// items_to_fetch [0..1])` — "List all OPTs whose identifiers match a
    /// regex pattern." Error `invalid_id_pattern` (→ `400`).
    ///
    /// PORT NOTE (spec defect): the SM types the return `List<ARCHETYPE_ID>`
    /// even though OPTs are UUID-keyed. We return the OPTs' `template_id`
    /// strings (the meaningful human-readable identifier the pattern is
    /// useful against), not a nonsensical `ARCHETYPE_ID` cast.
    async fn list_matching_opts(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError>;

    /// `delete_opt (an_id: UUID)` with `__Pre_has_opt__` +
    /// `__Post_opt_removed__` — "Delete a previously uploaded ADL 1.4 OPT";
    /// absent → `404`.
    async fn delete_opt(&self, an_opt_id: String) -> Result<(), SmError>;

    /// `opts_count (): Integer` — "Return total OPTs count."
    async fn opts_count(&self) -> Result<i64, SmError>;
}
