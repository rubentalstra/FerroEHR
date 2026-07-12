//! HTTP dispatch for the `query` API group — ad-hoc + stored AQL execution
//! (ITS-REST 1.0.3 QUERY API). Each arm normalizes the request (the `q`/name,
//! the `ehr_id` scope, the `offset`/`fetch` paging window, and the
//! `query_parameters` binds) from the query string or the request body
//! (`AdhocQueryExecute` / `Query`), calls the [`QueryService`] seam, and renders
//! the assembled `RESULT_SET` as JSON.
//!
//! Spec: the paging + scope parameters are `parameters/query/{ehr_id,offset,
//! fetch}` and the `query_parameters` map (`docs/query/Request.md`); `ehr_id` may
//! arrive as the `ehr_id` query parameter or the `openEHR-EHR-id` header.

use axum::response::{IntoResponse, Response};
use http::{HeaderMap, StatusCode};

use openehr_its::rest::generated::query::{
    AdhocQueryExecute, Query as StoredQueryBody, QueryExecuteAdhocQueryParams,
    QueryExecuteStoredQueryParams, QueryExecuteStoredQueryVersionParams,
};
use openehr_its::rest::runtime::ApiError;

use super::{BoxResponse, RequestParts};
use crate::error::RestError;
use ehrbase_sm::Platform;
use ehrbase_sm::AqlQueryRequest;

use crate::state::AppState;
use crate::{negotiate, params};

pub(super) fn dispatch<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();

    // ABAC (§6.4): the patient subject-scope pre-filter + collection flag. A
    // missing configured patient claim is a ready 403.
    let (subject_scope, collect) = match super::abac::query_pre(&state, op) {
        Ok(prep) => prep,
        Err(deny) => return Ok(deny),
    };
    let scope = |mut request: AqlQueryRequest| {
        request.subject_scope.clone_from(&subject_scope);
        request.collect_attributes = collect;
        request
    };

    let outcome = match op {
        // ── ad-hoc ────────────────────────────────────────────────────────────
        "query_execute_adhoc_query" => {
            let p = params::build::<QueryExecuteAdhocQueryParams>(&parts.path, q, h)?;
            let request = scope(AqlQueryRequest {
                ehr_ids: p.ehr_id.into_iter().collect(),
                offset: p.offset,
                fetch: p.fetch,
                parameters: p.query_parameters.unwrap_or_default(),
                ..Default::default()
            });
            state.backend().execute_ad_hoc_query(p.q, request).await?
        }
        "query_execute_adhoc_query_body" => {
            let body: AdhocQueryExecute = decode_body(h, &parts.body)?;
            let request = scope(AqlQueryRequest {
                ehr_ids: ehr_id_from_request(q, h).into_iter().collect(),
                offset: body.offset,
                fetch: body.fetch,
                parameters: body.query_parameters.unwrap_or_default(),
                ..Default::default()
            });
            state.backend().execute_ad_hoc_query(body.q, request).await?
        }
        // ── stored (latest) ─────────────────────────────────────────────────────
        "query_execute_stored_query" => {
            let p = params::build::<QueryExecuteStoredQueryParams>(&parts.path, q, h)?;
            let request = scope(AqlQueryRequest {
                ehr_ids: p.ehr_id.into_iter().collect(),
                offset: p.offset,
                fetch: p.fetch,
                parameters: p.query_parameters.unwrap_or_default(),
                ..Default::default()
            });
            state
                .backend()
                .execute_stored_query(p.qualified_query_name, None, request)
                .await?
        }
        "query_execute_stored_query_body" => {
            let name = path_segment(&parts, "qualified_query_name")?;
            let request = scope(stored_body_request(q, h, &parts.body)?);
            state
                .backend()
                .execute_stored_query(name, None, request)
                .await?
        }
        // ── stored (explicit version) ─────────────────────────────────────────
        "query_execute_stored_query_version" => {
            let p = params::build::<QueryExecuteStoredQueryVersionParams>(&parts.path, q, h)?;
            let request = scope(AqlQueryRequest {
                ehr_ids: p.ehr_id.into_iter().collect(),
                offset: p.offset,
                fetch: p.fetch,
                parameters: p.query_parameters.unwrap_or_default(),
                ..Default::default()
            });
            state
                .backend()
                .execute_stored_query(p.qualified_query_name, Some(p.version), request)
                .await?
        }
        "query_execute_stored_query_version_body" => {
            let name = path_segment(&parts, "qualified_query_name")?;
            let version = path_segment(&parts, "version")?;
            let request = scope(stored_body_request(q, h, &parts.body)?);
            state
                .backend()
                .execute_stored_query(name, Some(version), request)
                .await?
        }
        other => {
            return Err(RestError(ApiError::Internal(format!(
                "unrouted query operation: {other}"
            ))));
        }
    };

    // ABAC query post-check (§6.4): PDP fan-out over the touched template set.
    if let Err(deny) = super::abac::query_post(&state, op, &outcome).await {
        return Ok(deny);
    }

    Ok(negotiate::respond(h, StatusCode::OK, &outcome.result_set))
}

/// Build the request for a stored-query `POST` body (`Query` schema: `offset`,
/// `fetch`, `query_parameters`); `ehr_id` comes from the query string or header.
fn stored_body_request(
    q: Option<&str>,
    h: &HeaderMap,
    body: &bytes::Bytes,
) -> Result<AqlQueryRequest, RestError> {
    let parsed: StoredQueryBody = decode_body(h, body)?;
    Ok(AqlQueryRequest {
        ehr_ids: ehr_id_from_request(q, h).into_iter().collect(),
        offset: Some(parsed.offset),
        fetch: Some(parsed.fetch),
        parameters: parsed.query_parameters,
        ..Default::default()
    })
}

/// Decode a JSON request body into `T`.
fn decode_body<T: serde::de::DeserializeOwned>(
    h: &HeaderMap,
    body: &bytes::Bytes,
) -> Result<T, RestError> {
    let value = negotiate::json_value(h, body).map_err(RestError)?;
    serde_json::from_value(value).map_err(|e| {
        RestError(ApiError::BadRequest(format!(
            "invalid query request body: {e}"
        )))
    })
}

/// The `ehr_id` scope from the `ehr_id` query parameter or the `openEHR-EHR-id`
/// request header (spec `docs/query/Request.md`: either form is accepted).
fn ehr_id_from_request(q: Option<&str>, h: &HeaderMap) -> Option<String> {
    params::query_param(q, "ehr_id").or_else(|| {
        h.get("openEHR-EHR-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    })
}

/// A required path segment (the generated `*BodyParams` for POST carry only the
/// name/version path parts; read them directly from the matched path).
fn path_segment(parts: &RequestParts, key: &str) -> Result<String, RestError> {
    parts.path.get(key).cloned().ok_or_else(|| {
        RestError(ApiError::BadRequest(format!(
            "missing path parameter `{key}`"
        )))
    })
}
