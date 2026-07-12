//! `I_DEFINITION_QUERY` (`i_definition_query.adoc`) — "Interface for storying
//! queries and query sets" (spec's own typo) — plus `QUERY_DESCRIPTOR`
//! (`query_descriptor.adoc`).

use async_trait::async_trait;

use crate::common::{CallStatusType, Page, SmError};

/// `QUERY_DESCRIPTOR` — "Object describing a query in terms of its unique
/// identifier, name under which it is currently registered and registration
/// time under the current name" (`query_descriptor.adoc`).
///
/// PORT NOTE (wire): `registration_time` is typed `Iso8601_date_time` in the
/// SM; carried as an ISO-8601 `String` (the stored `created_at` rendered),
/// consistent with the rest of the native API's date handling.
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

/// `I_DEFINITION_QUERY` — registered queries and query sets, one Rust method
/// per SM call.
///
/// Naming (`master04-definition_package.adoc` §Registered Queries): queries
/// are identified by qualified names — `<namespace>::<query-name>` or
/// `<namespace>::<formalism>::<query-name>`; "If no namespace is supplied,
/// the namespace `misc` is assumed."
///
/// Formalism (§Query Formalism): the `a_type` parameter is "a string value,
/// treated case-insensitively carrying the name of the formalism of the
/// query text, with an optional version identifier separated by the `::`
/// delimiter … If no version identifier part is supplied, the major version
/// '1' is assumed" — so `"AQL"`, `"aql"` and `"AQL::1"` are equivalent.
///
/// No default method bodies except [`store_query_set`], a spec TODO.
///
/// [`store_query_set`]: DefinitionQueryService::store_query_set
#[async_trait]
pub trait DefinitionQueryService: Send + Sync {
    /// `has_query (a_query_name: String): Boolean` — "True if the query
    /// identified by `a_query_name` is registered" (the `misc` namespace is
    /// assumed when none is supplied).
    async fn has_query(&self, a_query_name: String) -> Result<bool, SmError>;

    /// `valid_query (a_query_text: String, a_type: String): Boolean` — "True
    /// if the provided query text is a valid instance of the formalism."
    async fn valid_query(&self, a_query_text: String, a_type: String) -> Result<bool, SmError>;

    /// `store_query (a_query_text: String, a_type: String, a_query_name
    /// [0..1]): QUERY_DESCRIPTOR` with `__Pre_valid_query__` — "Register a
    /// query under a qualified name. If no name is provided, one is created
    /// in the service."
    ///
    /// PORT NOTE (spec naming): the precondition is written
    /// `is_valid_query(a_query_text)` but the actual function is
    /// `valid_query(text, type)` (a spec inconsistency); `valid_query` is
    /// enforced and an invalid query rejects as `invalid_query` (→ `422`).
    async fn store_query(
        &self,
        a_query_text: String,
        a_type: String,
        a_query_name: Option<String>,
    ) -> Result<QueryDescriptor, SmError>;

    /// `store_query_set (a_query_set_name: String [0..1]): UUID` — "Register
    /// a query set. TODO: determine details."
    ///
    /// PORT NOTE: an explicit spec TODO with no defined semantics — the one
    /// sanctioned default body in this interface, `NotImplemented` (→ `501`)
    /// until the spec defines it.
    async fn store_query_set(&self, _a_query_set_name: Option<String>) -> Result<String, SmError> {
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "store_query_set is an SM TODO (i_definition_query.adoc): not implemented",
        ))
    }

    /// `list_queries (item_offset [0..1], items_to_fetch [0..1]):
    /// List<QUERY_DESCRIPTOR>` — "List all registered queries."
    async fn list_queries(&self, page: Page) -> Result<Vec<QueryDescriptor>, SmError>;

    /// `list_matching_queries (id_pattern: String, artefact_id_pattern:
    /// String, …): List<QUERY_DESCRIPTOR>` — "List all registered queries
    /// matching an identifier pattern (regex) and an artefact identifier
    /// pattern (regex) for artefacts referenced in the query. Either pattern
    /// may be the regex for 'match any'." Error `invalid_id_pattern`.
    async fn list_matching_queries(
        &self,
        id_pattern: String,
        artefact_id_pattern: Option<String>,
        page: Page,
    ) -> Result<Vec<QueryDescriptor>, SmError>;

    /// `delete_query (a_query_name: String)` with `__Pre_has_query__` +
    /// `__Post_query_deleted__` — "Delete query with name `a_query_name`";
    /// absent → `404`.
    async fn delete_query(&self, a_query_name: String) -> Result<(), SmError>;

    /// `queries_count (): Integer` — "Return total count of queries."
    async fn queries_count(&self) -> Result<i64, SmError>;
}
