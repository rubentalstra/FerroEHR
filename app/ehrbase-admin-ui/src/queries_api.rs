//! The query surfaces' shared server API: ad-hoc AQL execution, BFF-local
//! validation, and stored-query CRUD over the ITS-REST Definition/Query APIs.
//!
//! The console persists nothing of its own here (nor anywhere): a stored
//! query's grouping is derived from the namespace in its qualified name
//! (`crate::query_namespace`), so it lives in the CDR and reads identically
//! for every API client.

use leptos::server;
use serde::{Deserialize, Serialize};

use crate::error::AdminUiError;
use crate::pages::ehrs::ResultPage;

/// A stored-query listing row (ITS-REST Definition API).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredQueryRow {
    /// Qualified name (`namespace::name`).
    pub name: String,
    /// Version (semver text).
    pub version: String,
    /// The stored AQL text (empty in list responses that omit it).
    pub query: String,
    /// Saved timestamp as reported.
    pub saved: String,
}

/// Validate AQL text BFF-locally through the real grammar — no CDR round
/// trip. Returns the parser diagnostic on failure.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session;
/// [`AdminUiError::Invalid`] carrying the parse diagnostic.
#[server]
pub async fn validate_aql(aql: String) -> Result<(), AdminUiError> {
    crate::session::require_session().await?;
    openehr_query::parser::parse_str(&aql)
        .map(|_| ())
        .map_err(AdminUiError::Invalid)
}

/// Run ad-hoc AQL via `POST query/aql` with parameter bindings (a JSON
/// object as text), one page at `offset`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session;
/// [`AdminUiError::Invalid`] when `parameters_json` is not a JSON object;
/// CDR errors normalized via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn run_aql(
    aql: String,
    parameters_json: String,
    offset: u32,
) -> Result<ResultPage, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let parameters: serde_json::Value = if parameters_json.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&parameters_json)
            .map_err(|e| AdminUiError::Invalid(format!("parameters must be a JSON object: {e}")))?
    };
    if !parameters.is_object() {
        return Err(AdminUiError::Invalid(
            "parameters must be a JSON object".to_owned(),
        ));
    }
    let url = state.cdr.rest_v1("query/aql");
    // ITS-REST forbids combining the `fetch`/`offset` request parameters
    // with an AQL LIMIT/TOP clause (QUERY §Query structure/LIMIT) — when
    // the query carries its own window, send it bare; otherwise page with
    // fetch/offset as usual.
    let has_own_window = openehr_query::parser::parse_str(&aql)
        .is_ok_and(|q| q.limit.is_some() || q.select.top.is_some());
    let body = if has_own_window {
        serde_json::json!({ "q": aql, "query_parameters": parameters }).to_string()
    } else {
        crate::pages::ehrs::aql_request_body(&aql, &parameters, offset)
    };
    let response = state
        .cdr
        .post(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[],
            body,
        )
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    crate::pages::ehrs::parse_result_set(&response.body, offset)
}

/// List the CDR's stored queries (`GET definition/query`).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session; CDR errors
/// normalized; [`AdminUiError::Internal`] on an unparseable body.
#[server]
pub async fn list_stored_queries() -> Result<Vec<StoredQueryRow>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1("definition/query");
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    if response.status == 404 {
        // A CDR with no stored queries may 404 the listing; treat as empty.
        return Ok(Vec::new());
    }
    let body = crate::cdr::CdrClient::expect_success(response)?.body;
    let doc: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AdminUiError::Internal(format!("stored-query list JSON: {e}")))?;
    // The list shape nests under "versions" per the Definition API; parse
    // defensively (array at root or under "versions").
    let items = doc
        .get("versions")
        .and_then(serde_json::Value::as_array)
        .or_else(|| doc.as_array())
        .cloned()
        .unwrap_or_default();
    let text = |item: &serde_json::Value, keys: &[&str]| {
        keys.iter()
            .find_map(|k| item.get(*k).and_then(serde_json::Value::as_str))
            .unwrap_or_default()
            .to_owned()
    };
    Ok(items
        .iter()
        .map(|item| StoredQueryRow {
            name: text(item, &["name", "qualified_query_name"]),
            version: text(item, &["version"]),
            query: text(item, &["query", "q"]),
            saved: text(item, &["saved", "timestamp"]),
        })
        .collect())
}

/// Fetch one stored query's AQL text
/// (`GET definition/query/{name}/{version}`).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session; CDR errors
/// normalized (a `404` for an unknown name included).
#[server]
pub async fn fetch_stored_query(name: String, version: String) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1(&format!(
        "definition/query/{}/{}",
        urlencoding::encode(&name),
        urlencoding::encode(&version)
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    let body = crate::cdr::CdrClient::expect_success(response)?.body;
    let doc: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AdminUiError::Internal(format!("stored query JSON: {e}")))?;
    Ok(doc
        .get("query")
        .or_else(|| doc.get("q"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&body)
        .to_owned())
}

/// Store (create or version-bump) a query
/// (`PUT definition/query/{name}` with the AQL as `text/plain`).
///
/// `name` is the qualified query name — `[{namespace}::]{query-name}`, the
/// namespace optional (ITS-REST
/// `specifications/docs/query/Qualified_query_name.md` §Qualified query name).
/// The save screens compose it from their namespace + name fields with
/// [`qualify`](crate::query_namespace::qualify); the whole value is one path
/// segment here, percent-encoded via `urlencoding`. The AQL is validated
/// BFF-locally first so an invalid query never reaches the CDR.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session;
/// [`AdminUiError::Invalid`] for a bad name or unparseable AQL; CDR errors
/// normalized.
#[server]
pub async fn store_query(name: String, aql: String) -> Result<(), AdminUiError> {
    let session = crate::session::require_session().await?;
    if name.trim().is_empty() {
        return Err(AdminUiError::Invalid(
            "the stored query needs a name".to_owned(),
        ));
    }
    openehr_query::parser::parse_str(&aql).map_err(AdminUiError::Invalid)?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state
        .cdr
        .rest_v1(&format!("definition/query/{}", urlencoding::encode(&name)));
    let response = state
        .cdr
        .put_text(&session.credential, &url, "text/plain", aql)
        .await?;
    crate::cdr::CdrClient::expect_success(response)?;
    Ok(())
}

/// Run one stored query (`GET query/{name}/{version}`) and return its match
/// count — the dashboard namespace-tile primitive. The count is the RESULT_SET
/// row count of a `fetch`-limited run when the query has no aggregate;
/// tiles built from `SELECT COUNT(*)` queries read the single cell.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session; CDR errors
/// normalized; [`AdminUiError::Internal`] on an unparseable result.
#[server]
pub async fn run_stored_count(name: String, version: String) -> Result<i64, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1(&format!(
        "query/{}/{}",
        urlencoding::encode(&name),
        urlencoding::encode(&version)
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    let body = crate::cdr::CdrClient::expect_success(response)?.body;
    let doc: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AdminUiError::Internal(format!("stored-query result JSON: {e}")))?;
    let rows = doc
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    // A single 1x1 numeric cell (COUNT-shaped query) IS the count.
    if rows.len() == 1
        && let Some(cell) = rows[0]
            .as_array()
            .and_then(|r| if r.len() == 1 { r[0].as_i64() } else { None })
    {
        return Ok(cell);
    }
    Ok(i64::try_from(rows.len()).unwrap_or(i64::MAX))
}
