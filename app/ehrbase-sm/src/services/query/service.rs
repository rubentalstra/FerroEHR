//! `I_QUERY_SERVICE` (`i_query_service.adoc`) — "Query execution service
//! interface."

use async_trait::async_trait;

use crate::common::SmError;

use super::request::{AqlQueryRequest, QueryOutcome};

/// `I_QUERY_SERVICE` — one Rust method per SM call. The execute-spec
/// classes' content (query text / qualified name + version, parameters,
/// paging, EHR scope) rides in [`AqlQueryRequest`]; the `RESULT_SET` is
/// returned inside [`QueryOutcome`].
///
/// No default method bodies (compile-time completeness).
#[async_trait]
pub trait QueryService: Send + Sync {
    /// `execute_ad_hoc_query (exec_spec: ADHOC_QUERY_EXECUTE_SPEC,
    /// row_offset [0..1], rows_to_fetch [0..1], ehr_ids: List<UUID> [0..1]):
    /// RESULT_SET` — "Execute an ad hoc query, supplying the query text."
    /// Error `ehr_id_does_not_exist` (a listed EHR does not exist).
    async fn execute_ad_hoc_query(
        &self,
        aql: String,
        request: AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError>;

    /// `execute_stored_query (exec_spec: STORED_QUERY_EXECUTE_SPEC,
    /// row_offset [0..1], rows_to_fetch [0..1], ehr_ids: List<UUID> [0..1]):
    /// RESULT_SET` — "Execute a query stored in the definition service, using
    /// its qualified query name" (`reverse_domain::name`; `version` a
    /// semver.org string, latest when absent). Error `ehr_id_does_not_exist`.
    async fn execute_stored_query(
        &self,
        qualified_query_name: String,
        version: Option<String>,
        request: AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError>;
}
