//! HTTP dispatch for the `query` API group — ad-hoc + stored AQL execution
//! (ITS-REST 1.0.3 QUERY API). This module is the operation match: it runs the
//! ABAC pre-filter (`extensions::abac`), routes each `operationId` to the
//! ad-hoc ([`super::adhoc`]) or stored ([`super::stored`]) execution path, runs
//! the ABAC post-check, and renders the assembled `RESULT_SET` (with its
//! spec-mandated `ETag`) via [`super::response`].
//!
//! Spec: the six operations, their parameter lists, and their `RESULT_SET`
//! response are `query-codegen.openapi.yaml` + `docs/query/{Request,Response}.md`.
//! The request-normalization + response-rendering shared by both paths lives in
//! [`super::response`].

use axum::response::{IntoResponse, Response};

use openehr_its::rest::runtime::ApiError;

use crate::api::{BoxResponse, RequestParts};
use crate::overview::error::RestError;
use ehrbase_sm::Platform;

use super::response::{self, QueryScope};
use super::{adhoc, stored};
use crate::state::AppState;

pub(crate) fn dispatch<S: Platform>(
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
    // ABAC (§6.4): the patient subject-scope pre-filter + collection flag. A
    // missing configured patient claim is a ready 403.
    //
    // TODO(w3e-integrate): `extensions::abac::{query_pre,query_post}` are
    // `pub(super)` (visible only inside `extensions`) — after this dispatcher
    // moved into `api/query/`, they must be raised to `pub(crate)` for this
    // call site to compile. Visibility bump only; behaviour unchanged.
    let (subject_scope, collect) = match crate::extensions::abac::query_pre(&state, op) {
        Ok(prep) => prep,
        Err(deny) => return Ok(deny),
    };
    let scope = QueryScope {
        subject_scope,
        collect,
    };

    let outcome = match op {
        "query_execute_adhoc_query" | "query_execute_adhoc_query_body" => {
            adhoc::execute(&state, op, &parts, &scope).await?
        }
        "query_execute_stored_query"
        | "query_execute_stored_query_body"
        | "query_execute_stored_query_version"
        | "query_execute_stored_query_version_body" => {
            stored::execute(&state, op, &parts, &scope).await?
        }
        other => {
            return Err(RestError(ApiError::Internal(format!(
                "unrouted query operation: {other}"
            ))));
        }
    };

    // ABAC query post-check (§6.4): PDP fan-out over the touched template set.
    // TODO(w3e-integrate): same `pub(super)` → `pub(crate)` bump as `query_pre`.
    if let Err(deny) = crate::extensions::abac::query_post(&state, op, &outcome).await {
        return Ok(deny);
    }

    Ok(response::respond_result_set(
        &parts.headers,
        &outcome.result_set,
    ))
}
