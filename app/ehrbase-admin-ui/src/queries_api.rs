//! The query surfaces' shared server API: ad-hoc AQL execution, BFF-local
//! validation, stored-query CRUD (the ITS-REST Definition/Query APIs), and
//! the console-local query groups (no ITS-REST resource exists for groups —
//! no openEHR spec governs them; our own design/extension, persisted to a
//! small JSON file next to the console).

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

/// One console-local query group: a named set of stored queries whose match
/// counts the dashboard tiles show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryGroup {
    /// Display name (also the group key).
    pub name: String,
    /// Member stored queries as `name@version` references.
    pub members: Vec<String>,
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
    let body = crate::pages::ehrs::aql_request_body(&aql, &parameters, offset);
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
/// (`PUT definition/query/{name}` with the AQL as `text/plain`). The name
/// must be a qualified query name (`namespace::name`); the AQL is validated
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

/// List the console-local query groups.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session;
/// [`AdminUiError::Internal`] on an unreadable groups file.
#[server]
pub async fn list_groups() -> Result<Vec<QueryGroup>, AdminUiError> {
    crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let path = state.config.groups_file();
    // File I/O off the async runtime (reliability rule: no sync I/O on it).
    tokio::task::spawn_blocking(move || crate::groups::read_groups(&path))
        .await
        .map_err(|e| AdminUiError::Internal(format!("groups task: {e}")))?
}

/// Create/replace a console-local query group (empty `members` deletes it).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a session;
/// [`AdminUiError::Invalid`] for an empty name;
/// [`AdminUiError::Internal`] on an unwritable groups file.
#[server]
pub async fn save_group(name: String, members: Vec<String>) -> Result<(), AdminUiError> {
    crate::session::require_session().await?;
    if name.trim().is_empty() {
        return Err(AdminUiError::Invalid("the group needs a name".to_owned()));
    }
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let path = state.config.groups_file();
    tokio::task::spawn_blocking(move || crate::groups::write_group(&path, &name, members))
        .await
        .map_err(|e| AdminUiError::Internal(format!("groups task: {e}")))?
}

/// Run one stored query (`GET query/{name}/{version}`) and return its match
/// count — the dashboard/group tile primitive. The count is the RESULT_SET
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
