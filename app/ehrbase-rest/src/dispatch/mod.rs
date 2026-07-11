//! The HTTP adapter over the generated ITS-REST server traits.
//!
//! The generated contract combines each operation's path/query/header
//! parameters into one `*Params` struct and exposes a `ROUTES` table of
//! `(method, path, operation_id)`. This module turns that table into an axum
//! router: for every route it mounts a handler that captures the operation id,
//! collects the request's raw parts, and calls the group's dispatcher. Each
//! implemented group's dispatcher ([`ehr`], [`definition`]) rebuilds the exact
//! `*Params` with [`crate::params`], calls the trait method on [`AppState`],
//! and renders a negotiated response.
//!
//! Every API group has its own dispatcher module ([`ehr`], [`definition`],
//! [`query`], [`demographic`], [`admin`], [`terminology`]); operations whose
//! backend seam method is not overridden surface as `501` + the standard error
//! body.

mod abac;
mod admin;
mod definition;
mod demographic;
mod ehr;
mod event_subscription;
mod fhir;
mod flat;
mod query;
mod tenant;
mod terminology;

use std::future::Future;
use std::pin::Pin;

use axum::Router;
use axum::extract::{RawPathParams, RawQuery, State};
use axum::response::Response;
use axum::routing::{MethodFilter, MethodRouter};
use bytes::Bytes;
use ehrbase_sm::Platform;
use http::HeaderMap;
use indexmap::IndexMap;

use crate::state::AppState;

/// A boxed response future — the uniform return type of a group dispatcher.
type BoxResponse = Pin<Box<dyn Future<Output = Response> + Send>>;

/// A group dispatcher: `(state, operation_id, request parts) → response`.
type Dispatcher<S> = fn(AppState<S>, &'static str, RequestParts) -> BoxResponse;

/// The raw request sources a dispatcher needs to rebuild a `*Params` struct and
/// decode a body.
pub(crate) struct RequestParts {
    pub(crate) path: IndexMap<String, String>,
    pub(crate) query: Option<String>,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
}

impl RequestParts {
    fn new(
        raw_path: &RawPathParams,
        query: Option<String>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Self {
        let path = raw_path
            .iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        Self {
            path,
            query,
            headers,
            body,
        }
    }
}

/// Build the API router covering every ITS-REST route (group-relative paths;
/// nest under the configured base path). State is applied by the caller.
pub(crate) fn api_router<S: Platform>() -> Router<AppState<S>> {
    use openehr_its::rest::generated as g;

    Router::new()
        .merge(mount(g::ehr::ROUTES, ehr::dispatch::<S>))
        .merge(mount(g::demographic::ROUTES, demographic::dispatch::<S>))
        // Our-own-design PARTY_RELATIONSHIP extension routes (no ITS-REST
        // contract; SM-3), served by the same demographic dispatcher.
        .merge(mount(
            demographic::RELATIONSHIP_ROUTES,
            demographic::dispatch::<S>,
        ))
        .merge(mount(g::definition::ROUTES, definition::dispatch::<S>))
        .merge(mount(g::query::ROUTES, query::dispatch::<S>))
        .merge(mount(g::admin::ROUTES, admin::dispatch::<S>))
        // Our-own-design terminology extension routes (no ITS-REST contract;
        // SM `I_TERMINOLOGY_SERVICE`, design doc 08 §7), config-gated.
        .merge(mount(
            terminology::TERMINOLOGY_ROUTES,
            terminology::dispatch::<S>,
        ))
        // Our-own-design event-subscription admin extension routes (no ITS-REST
        // contract; ADR-014 §5 "Event Trigger" parity), config-gated.
        .merge(mount(
            event_subscription::EVENT_SUBSCRIPTION_ROUTES,
            event_subscription::dispatch::<S>,
        ))
        // Our-own-design tenant admin extension routes (no ITS-REST contract;
        // ADR-015 §5 multi-tenancy), config-gated.
        .merge(mount(tenant::TENANT_ROUTES, tenant::dispatch::<S>))
        // Our-own-design FHIR R4 connector routes (no ITS-REST contract;
        // ADR-016 §Decision 3/5 — inbound ingest + mapping CRUD), config-gated.
        .merge(mount(fhir::FHIR_ROUTES, fhir::dispatch::<S>))
}

/// Mount one API group's routes onto a router, grouping methods that share a
/// path into a single `MethodRouter`.
fn mount<S: Platform>(
    routes: &'static [(&'static str, &'static str, &'static str)],
    dispatch: Dispatcher<S>,
) -> Router<AppState<S>> {
    let mut by_path: IndexMap<String, Vec<(MethodFilter, &'static str)>> = IndexMap::new();
    for (method, path, op) in routes {
        let filter = method_filter(method)
            .unwrap_or_else(|| panic!("ITS-REST route uses unsupported method {method}"));
        by_path
            .entry(normalize_path(path))
            .or_default()
            .push((filter, op));
    }

    let mut group_router = Router::new();
    for (path, ops) in by_path {
        let mut method_router: MethodRouter<AppState<S>> = MethodRouter::new();
        for (filter, op) in ops {
            method_router = method_router.on(
                filter,
                move |raw_path: RawPathParams,
                      RawQuery(query): RawQuery,
                      headers: HeaderMap,
                      State(state): State<AppState<S>>,
                      body: Bytes| async move {
                    let parts = RequestParts::new(&raw_path, query, headers, body);
                    // ABAC (§7): pre-check short-circuits before the backend on a
                    // deny/engine-failure; the post-check may replace a success
                    // with 403/500. Both inert unless ABAC is wired.
                    let mut resp = match abac::pre_check(&state, op, &parts).await {
                        Ok(()) => {
                            let resp = dispatch(state.clone(), op, parts).await;
                            abac::post_check(&state, op, resp).await
                        }
                        Err(deny) => deny,
                    };
                    // The single, generic ATNA hook: tag the response with the
                    // matched operation id for the audit layer (§8.2 step 1).
                    resp.extensions_mut().insert(crate::audit::AuditOpId(op));
                    resp
                },
            );
        }
        group_router = group_router.route(&path, method_router);
    }
    group_router
}

fn method_filter(method: &str) -> Option<MethodFilter> {
    Some(match method {
        "GET" => MethodFilter::GET,
        "POST" => MethodFilter::POST,
        "PUT" => MethodFilter::PUT,
        "DELETE" => MethodFilter::DELETE,
        "PATCH" => MethodFilter::PATCH,
        "HEAD" => MethodFilter::HEAD,
        "OPTIONS" => MethodFilter::OPTIONS,
        _ => return None,
    })
}

/// Strip RFC 6570 query/expansion suffixes (e.g. `all{?ehr_id*}` → `all`) that
/// the `OpenAPI` paths carry but axum path templates do not use; `{name}` capture
/// segments are already axum-0.8 syntax and pass through unchanged.
///
/// Shared with the RBAC gate ([`crate::authz`]), which keys its route→class map
/// by the same normalized templates so an axum `MatchedPath` resolves exactly.
pub(crate) fn normalize_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = String::with_capacity(path.len());
    let mut i = 0;
    while i < bytes.len() {
        // A `{` immediately followed by an RFC 6570 operator opens a
        // query/path-style expansion that is not part of the resource path.
        if bytes[i] == b'{'
            && i + 1 < bytes.len()
            && matches!(bytes[i + 1], b'?' | b'&' | b'#' | b'.' | b';' | b'/' | b'+')
        {
            // Skip to the matching `}`.
            while i < bytes.len() && bytes[i] != b'}' {
                i += 1;
            }
            i += 1; // consume `}`
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_query_expansion() {
        assert_eq!(normalize_path("/admin/ehr/all{?ehr_id*}"), "/admin/ehr/all");
        assert_eq!(normalize_path("/ehr/{ehr_id}"), "/ehr/{ehr_id}");
        assert_eq!(
            normalize_path("/ehr/{ehr_id}/composition/{uid_based_id}"),
            "/ehr/{ehr_id}/composition/{uid_based_id}"
        );
        assert_eq!(
            normalize_path("/definition/template/adl1.4"),
            "/definition/template/adl1.4"
        );
    }
}
