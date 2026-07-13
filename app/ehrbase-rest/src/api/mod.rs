//! The ITS-REST **API hub** — the assembly over the generated `ROUTES` tables.
//!
//! The crate is laid out per ITS-REST specification (the development-edition
//! register, `docs/design/its-rest/README.md`): one module per API group under
//! `api/`, each split along the spec's own resource boundaries and citing its
//! governing section. This hub is the wiring layer that turns the generated
//! contract into one axum router.
//!
//! ## The generated contract
//!
//! `emit-rest` produces, per API group, a `ROUTES` table of
//! `(method, path, operation_id)` and one `*Params` struct per operation
//! (`openehr_its::rest::generated::{ehr,query,definition,admin,demographic}`).
//! The System API is not part of that contract (no `system` group is emitted),
//! so its single `OPTIONS /` operation is hand-written in [`system`] and mounted
//! by [`crate::router`], not here.
//!
//! ## The per-group modules
//!
//! Each group owns a dispatcher — an operation-id `match` that rebuilds the
//! exact `*Params` from the raw request parts, calls the SM native-API method on
//! [`AppState`], and renders a negotiated response:
//!
//! - [`ehr`] — the EHR API (EHR, `EHR_STATUS`, `VERSIONED_EHR_STATUS`, COMPOSITION,
//!   `VERSIONED_COMPOSITION`, DIRECTORY, CONTRIBUTION), re-exporting
//!   `ehr::dispatch::dispatch`.
//! - [`query`] — the Query API (adhoc + stored), re-exporting `query::dispatch`.
//! - [`definition`] — the Definition API (ADL 1.4 / ADL 2 templates + stored
//!   queries), re-exporting `definition::dispatch`.
//! - [`demographic`] — the Demographic API (spec-governed, DEVELOPMENT) plus the
//!   own-design `PARTY_RELATIONSHIP` extension routes, re-exporting
//!   `demographic::dispatch` + `demographic::RELATIONSHIP_ROUTES`.
//! - [`admin`] — the Admin API (physical EHR delete), whose dispatcher is reached
//!   as `admin::dispatch::dispatch` (the group publishes no re-export).
//! - [`system`] — the System API manifest ([`system::SystemManifest`],
//!   [`system::SystemOptionsConfig`]), assembled and mounted by [`crate::router`].
//!
//! Own-design extension surfaces with no ITS-REST contract
//! ([`crate::extensions`]: terminology, event subscription, multi-tenancy admin,
//! FHIR connector) carry their own `*_ROUTES` tables + dispatcher and are merged
//! here behind their config flags, so a stock server exposes only the
//! standardised ITS-REST surface.

pub mod admin;
pub mod definition;
pub mod demographic;
pub mod ehr;
pub mod query;
pub mod system;

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

use crate::extensions::access::{ehr_access, pep};
use crate::extensions::{event_subscription, fhir, tenant_routes, terminology};
use crate::state::AppState;

/// A boxed response future — the uniform return type of a group dispatcher.
pub(crate) type BoxResponse = Pin<Box<dyn Future<Output = Response> + Send>>;

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
///
/// Each generated group's dispatcher is mounted through [`mount`]; the private
/// per-group `dispatch` modules (`query`/`definition`/`demographic`) are reached
/// through their `pub(crate) use dispatch::dispatch` re-export, `ehr` through its
/// public `dispatch` module, and `admin` — which publishes no re-export —
/// through `admin::dispatch::dispatch`.
pub(crate) fn api_router<S: Platform>() -> Router<AppState<S>> {
    use openehr_its::rest::generated as g;

    Router::new()
        .merge(mount(g::ehr::ROUTES, ehr::dispatch::dispatch::<S>))
        .merge(mount(g::demographic::ROUTES, demographic::dispatch::<S>))
        // Our-own-design PARTY_RELATIONSHIP extension routes (no ITS-REST
        // contract; SM-3), served by the same demographic dispatcher.
        .merge(mount(
            demographic::RELATIONSHIP_ROUTES,
            demographic::dispatch::<S>,
        ))
        .merge(mount(g::definition::ROUTES, definition::dispatch::<S>))
        .merge(mount(g::query::ROUTES, query::dispatch::<S>))
        .merge(mount(g::admin::ROUTES, admin::dispatch::dispatch::<S>))
        // Our-own-design terminology extension routes (no ITS-REST contract;
        // SM `I_TERMINOLOGY_SERVICE`, design doc 08 §7), config-gated.
        .merge(mount(
            terminology::TERMINOLOGY_ROUTES,
            terminology::dispatch::<S>,
        ))
        // Our-own-design event-subscription admin extension routes (no ITS-REST
        // contract; an "Event Trigger"-style extension — no openEHR spec governs
        // eventing), config-gated.
        .merge(mount(
            event_subscription::EVENT_SUBSCRIPTION_ROUTES,
            event_subscription::dispatch::<S>,
        ))
        // Our-own-design tenant admin extension routes (no ITS-REST contract;
        // multi-tenancy — no openEHR spec governs it), config-gated.
        .merge(mount(
            tenant_routes::TENANT_ROUTES,
            tenant_routes::dispatch::<S>,
        ))
        // Our-own-design FHIR R4 connector routes (no ITS-REST contract;
        // inbound ingest + mapping CRUD — no openEHR spec governs FHIR interop),
        // config-gated.
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
                    // The EHR_ACCESS gate is the spec-grounded access-decision
                    // authority (RM `org.openehr.rm.ehr.ehr_access.adoc` — "All
                    // access decisions to data in the EHR must be made in
                    // accordance with the policies and rules in this object"), so
                    // it runs FIRST, before the enterprise RBAC/ABAC layers; the
                    // latter compose on top as additive restrictions. Always-on
                    // (default-open keeps existing flows working); a deny is 403.
                    // Then ABAC (§7): the PEP pre-check short-circuits before the
                    // backend on a deny/engine-failure; the post-check may replace
                    // a success with 403/500. ABAC is inert unless wired.
                    let mut resp = match ehr_access::enforce(&state, op, &parts).await {
                        Ok(()) => match pep::pre_check(&state, op, &parts).await {
                            Ok(()) => {
                                let resp = dispatch(state.clone(), op, parts).await;
                                pep::post_check(&state, op, resp).await
                            }
                            Err(deny) => deny,
                        },
                        Err(deny) => deny,
                    };
                    // The single, generic ATNA hook: tag the response with the
                    // matched operation id for the audit layer (§8.2 step 1).
                    resp.extensions_mut()
                        .insert(crate::system_log::middleware::AuditOpId(op));
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
/// Shared with the access-control classifier
/// ([`crate::extensions::access::authz`]), which keys its route→class map by the
/// same normalized templates so an axum `MatchedPath` resolves exactly.
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
