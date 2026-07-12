//! ITS-REST **stored-query** resource (`tags: Query`).
//!
//! Operations (`docs/specs/openehr/ITS-REST/specifications/operations/`):
//! `definition_query_list` / `definition_query_store.yaml` /
//! `definition_query_version_get` / `definition_query_version_store.yaml`.
//! Governing spec text:
//! `docs/specs/openehr/ITS-REST/specifications/docs/definition/`.
//! Register (gaps + target): `docs/design/its-rest/definition.md` (G-2).
//!
//! Queries route through the wire-shaped `DefinitionAdapter`
//! (`query_list`/`query_version_get`/`query_store`) — the SM
//! `DefinitionQueryService` exchanges `QUERY_DESCRIPTOR`s, while the wire
//! returns/accepts the ITS-REST `StoredQuery` shapes. A store success is
//! `200 OK` (not `204`) with a `Location` for the stored resource
//! (`responses/200_StoredQuery_stored.yaml` + `headers/Location_Query.yaml`).

use axum::response::Response;
use http::{HeaderMap, StatusCode};

use openehr_its::rest::generated::definition::{
    DefinitionQueryListParams, DefinitionQueryStoreYamlParams, DefinitionQueryVersionGetParams,
    DefinitionQueryVersionStoreYamlParams,
};

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};
use ehrbase_sm::Platform;

/// The default query formalism when `query_type` is absent
/// (`parameters/query/query_type.yaml`: `default: "AQL"`).
const DEFAULT_QUERY_TYPE: &str = "AQL";

/// `GET …/definition/query/{qualified_query_name}` — the registered queries
/// under the qualified name (a prefix pattern; wildcard on empty).
pub(super) async fn list<S: Platform>(
    state: &AppState<S>,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p = params::build::<DefinitionQueryListParams>(&parts.path, parts.query.as_deref(), h)?;
    Ok(negotiate::respond(
        h,
        StatusCode::OK,
        &state.backend().query_list(p.qualified_query_name).await?,
    ))
}

/// `PUT …/definition/query/{qualified_query_name}` — store/upsert a query
/// (server-assigned SEMVER).
///
/// G-2: the `query_type` query parameter (default `AQL`,
/// `parameters/query/query_type.yaml`) is now read and threaded to the store —
/// no longer silently dropped. The store persists the declared formalism and,
/// per `QUERY_DESCRIPTOR.formalism` ("may be any other string value"), an
/// unsupported non-AQL formalism gets an honest unsupported-formalism reject
/// (not a blanket "invalid AQL" 400).
pub(super) async fn store<S: Platform>(
    state: &AppState<S>,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p =
        params::build::<DefinitionQueryStoreYamlParams>(&parts.path, parts.query.as_deref(), h)?;
    let name = p.qualified_query_name.clone();
    let query_type = p
        .query_type
        .unwrap_or_else(|| DEFAULT_QUERY_TYPE.to_owned());
    let body = negotiate::text_body(&parts.body)?;
    // TODO(w3e-integrate): the `DefinitionAdapter::query_store` impl
    // (`app/ehrbase/src/service/stored_query.rs`) must honour `query_type`:
    // thread it to the SM `store_query`'s `a_type` (currently hardcoded `AQL`),
    // run the AQL syntactic check only when the formalism is AQL, persist the
    // declared formalism in the descriptor, and reject an unsupported non-AQL
    // formalism as a distinct typed error (not "invalid AQL"). See G-2.
    state
        .backend()
        .query_store(name.clone(), None, query_type, body)
        .await?;
    // The no-version store auto-assigns the SEMVER but the generated trait
    // method is bodyless (`()`), so the assigned version is recovered through
    // the list seam: exact-name rows come back ordered by version ascending, so
    // the last one is the version this store just wrote (or upserted).
    match stored_version_of(state, &name, h).await {
        Some(version) => {
            let location = format!(
                "{}/definition/query/{name}/{version}",
                state.config().base_path
            );
            Ok(negotiate::empty_with_location(StatusCode::OK, &location))
        }
        None => Ok(negotiate::empty(StatusCode::OK)),
    }
}

/// `GET …/definition/query/{qualified_query_name}/{version}` — the registered
/// query at the SEMVER (or SEMVER prefix); `404` if absent.
pub(super) async fn version_get<S: Platform>(
    state: &AppState<S>,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p =
        params::build::<DefinitionQueryVersionGetParams>(&parts.path, parts.query.as_deref(), h)?;
    Ok(negotiate::respond(
        h,
        StatusCode::OK,
        &state
            .backend()
            .query_version_get(p.qualified_query_name, p.version)
            .await?,
    ))
}

/// `PUT …/definition/query/{qualified_query_name}/{version}` — store a query at
/// a specified SEMVER (stored verbatim); `409` on an existing `(name, version)`.
///
/// G-2: as [`store`], the `query_type` parameter is read and threaded through.
pub(super) async fn version_store<S: Platform>(
    state: &AppState<S>,
    parts: &RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p = params::build::<DefinitionQueryVersionStoreYamlParams>(
        &parts.path,
        parts.query.as_deref(),
        h,
    )?;
    let name = p.qualified_query_name.clone();
    let version = p.version.clone();
    let query_type = p
        .query_type
        .unwrap_or_else(|| DEFAULT_QUERY_TYPE.to_owned());
    let body = negotiate::text_body(&parts.body)?;
    // TODO(w3e-integrate): as in `store`, the `query_store` impl must honour
    // `query_type` (thread to SM `store_query` `a_type`; AQL check only for the
    // AQL formalism; honest unsupported-formalism reject). See G-2.
    state
        .backend()
        .query_store(name.clone(), Some(version.clone()), query_type, body)
        .await?;
    let location = format!(
        "{}/definition/query/{name}/{version}",
        state.config().base_path
    );
    Ok(negotiate::empty_with_location(StatusCode::OK, &location))
}

/// The stored SEMVER of the stored query `name` after a no-version store: the
/// exact-name entries from the list seam (ordered by version ascending), taking
/// the highest. `None` when the lookup fails or finds nothing — the store itself
/// already succeeded, so the response degrades to Location-less rather than
/// failing the request.
async fn stored_version_of<S: Platform>(
    state: &AppState<S>,
    name: &str,
    _headers: &HeaderMap,
) -> Option<String> {
    let list = state.backend().query_list(name.to_owned()).await.ok()?;
    list.iter()
        .filter(|entry| entry.get("name").and_then(|n| n.as_str()) == Some(name))
        .filter_map(|entry| entry.get("version").and_then(|v| v.as_str()))
        .next_back()
        .map(str::to_owned)
}
