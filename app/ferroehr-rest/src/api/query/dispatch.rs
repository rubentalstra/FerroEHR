// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! HTTP dispatch for the `query` API group — ad-hoc + stored AQL execution
//! (ITS-REST 1.1.0 QUERY API). This module is the operation match: it runs the
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

use super::response::{self, QueryScope};
use super::{adhoc, stored};
use crate::state::AppState;

pub(crate) fn dispatch(state: AppState, op: &'static str, parts: RequestParts) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    // ABAC: the patient subject-scope pre-filter + collection flag. A
    // missing configured patient claim is a ready 403. The PEP entry points
    // (`extensions::access::pep::{query_pre,query_post}`) are `pub(crate)`, so
    // this cross-module dispatcher calls them directly.
    let (subject_scope, collect) = match crate::extensions::access::pep::query_pre(&state, op) {
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

    // ABAC query post-check: PDP fan-out over the touched template set.
    if let Err(deny) = crate::extensions::access::pep::query_post(&state, op, &outcome).await {
        return Ok(deny);
    }

    Ok(response::respond_result_set(
        &parts.headers,
        &outcome.result_set,
    ))
}
