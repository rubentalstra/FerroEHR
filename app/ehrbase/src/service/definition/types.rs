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

/// The template-list filter carried by the ITS-REST `definition_template_*_list`
/// operations. All three are optional query parameters the wire decodes but the
/// SM `I_DEFINITION_*` list interfaces (which return plain `List<UUID>`) do not
/// express — so they ride on the wire-shaped [`DefinitionAdapter`] list methods
/// alongside the SM cursor [`Page`].
///
/// - `template_id`: glob pattern matching `template_id`, `*` wildcard
///   (`parameters/query/filter_template_id.yaml`, "supports wildcards `*`").
/// - `concept`: glob pattern matching `concept`, `*` wildcard
///   (`parameters/query/concept.yaml`).
/// - `version`: version filter taken from `template_id` (e.g. `1.2.*`, or `*`
///   for all); absent → latest version only
///   (`parameters/query/filter_version.yaml`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplateListFilter {
    /// Glob pattern for `template_id` (`*` wildcard); `None` = match all.
    pub template_id: Option<String>,
    /// Glob pattern for `concept` (`*` wildcard); `None` = match all.
    pub concept: Option<String>,
    /// Version filter (e.g. `1.2.*`, `*`); `None` = latest version only.
    pub version: Option<String>,
}

// --- stored-query descriptor (SM I_DEFINITION_QUERY, definition/query) ---
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryDescriptor {
    /// `qualified_query_name [1]` — "Unique qualified name of query.
    /// Qualified names follow patterns such as `<namespace>::<query_name>`,
    /// e.g. `ehr::all_over_50_women`."
    pub qualified_query_name: String,
    /// `version [0..1]` — "Query semver.org version number."
    pub version: Option<String>,
    /// `registration_time [1]` — "Time query was registered in the service"
    /// (ISO-8601).
    pub registration_time: String,
    /// `formalism [1]` — "Formalism of the query, matching one of: 'aql';
    /// any other string value."
    pub formalism: String,
    /// `source [0..1]` — "Source query text to be executed (prior to
    /// parameter substitution)."
    pub source: Option<String>,
}
