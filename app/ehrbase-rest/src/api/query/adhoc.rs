//! Ad-hoc AQL execution — `GET`/`POST /query/aql` (`query_execute_adhoc_query`
//! + `_body`).
//!
//! An ad-hoc query supplies the AQL text directly (`Query_types.md` §Ad-hoc
//! (non-stored) queries; `schemas/query/AdhocQueryExecute.yaml`: `q` required,
//! plus `offset`/`fetch`/`query_parameters`). The `q` and the paging/scope
//! parameters arrive either in the query string (`GET`) or the JSON body
//! (`POST`, `AdhocQueryExecute`); this module normalizes both into an
//! [`AqlQueryRequest`] and calls the [`QueryService`] ad-hoc seam.
//!
//! [`QueryService`]: ehrbase::service::QueryService

use openehr_its::rest::generated::query::{AdhocQueryExecute, QueryExecuteAdhocQueryParams};
use openehr_its::rest::runtime::ApiError;

use ehrbase::service::{AqlQueryRequest, QueryOutcome};

use super::response::{self, QueryScope};
use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::params;
use crate::state::AppState;

/// Execute the two ad-hoc operations. The single wire `ehr_id` (query parameter
/// or `openEHR-EHR-id` header) is collected into the one-element
/// [`AqlQueryRequest::ehr_ids`] scope (the SM `List<UUID>` is realized as an
/// extension; the conformant wire binds one `ehr_id`).
pub(super) async fn execute(
    state: &AppState,
    op: &str,
    parts: &RequestParts,
    scope: &QueryScope,
) -> Result<QueryOutcome, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();

    match op {
        // GET /query/aql — `q` + paging/scope in the query string.
        "query_execute_adhoc_query" => {
            let p = params::build::<QueryExecuteAdhocQueryParams>(&parts.path, q, h)?;
            let request = scope.apply(AqlQueryRequest {
                ehr_ids: p.ehr_id.into_iter().collect(),
                offset: p.offset,
                fetch: p.fetch,
                parameters: p.query_parameters.unwrap_or_default(),
                ..Default::default()
            });
            Ok(state.backend().execute_ad_hoc_query(p.q, request).await?)
        }
        // POST /query/aql — `AdhocQueryExecute` body (`q` + paging/params);
        // `ehr_id` still from the query string / header.
        "query_execute_adhoc_query_body" => {
            let body: AdhocQueryExecute = response::decode_body(h, &parts.body)?;
            let request = scope.apply(AqlQueryRequest {
                ehr_ids: response::ehr_id_from_request(q, h).into_iter().collect(),
                offset: body.offset,
                fetch: body.fetch,
                parameters: body.query_parameters.unwrap_or_default(),
                ..Default::default()
            });
            Ok(state
                .backend()
                .execute_ad_hoc_query(body.q, request)
                .await?)
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted ad-hoc query operation: {other}"
        )))),
    }
}
