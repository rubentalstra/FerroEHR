//! The `/ehrs` screen — the EHR finder.
//!
//! A lookup form (jump straight to an EHR by id) over a recent-EHRs table
//! sourced from an ad-hoc AQL query — ITS-REST has no unpaged EHR-list
//! endpoint, so listing via AQL is the spec-honest route.
//! Paging is URL-driven (`?offset=`, rules §9): shareable, refresh-safe, and
//! WASM-optional.
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The wire it reads IS spec-bound: the AQL runs against `POST query/aql`
//! (`docs/specs/openehr/ITS-REST/docs/query/`). User input NEVER concatenates
//! into the AQL text — the fixed query is a validated const and the caller's
//! value travels as an AQL `query_parameters` binding.
//!
//! Every co-located `#[server]` fn guards with
//! [`require_session`](crate::session::require_session) first — server
//! functions are a public HTTP API (rules §0) — and the CDR credential never
//! reaches client-visible state.

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AdminUiError;

#[cfg(feature = "ssr")]
/// The fixed AQL that lists EHRs newest-first for the recent-EHRs table.
/// Validated by [`tests::fixed_aql_parses`]; never concatenated with user
/// input.
const LIST_EHRS_AQL: &str =
    "SELECT e/ehr_id/value, e/time_created/value FROM EHR e ORDER BY e/time_created/value DESC";

/// Rows fetched per page across every AQL-backed table in the console.
pub(crate) const PAGE_SIZE: u32 = 25;

/// One page of an AQL `RESULT_SET`, flattened for rendering: the column
/// headers, the raw row cells, and the offset that produced it (so the view
/// can build prev/next links). Shared across the EHR browse surfaces; carries
/// only fixed-size ints so it is WASM-safe over the server-fn boundary
/// (rules §1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultPage {
    /// The result-set column names (falling back to the column path).
    pub columns: Vec<String>,
    /// The result rows, each a vector of raw JSON cell values.
    pub rows: Vec<Vec<Value>>,
    /// The offset this page was fetched at (for prev/next paging).
    pub offset: u32,
}

#[cfg(feature = "ssr")]
/// Build the `POST query/aql` request body: the fixed AQL text, the caller's
/// parameter bindings (never string-interpolated into the query), and the
/// `fetch`/`offset` window.
pub(crate) fn aql_request_body(aql: &str, parameters: &Value, offset: u32) -> String {
    serde_json::json!({
        "q": aql,
        "query_parameters": parameters,
        "fetch": PAGE_SIZE,
        "offset": offset,
    })
    .to_string()
}

#[cfg(feature = "ssr")]
/// Parse an AQL `RESULT_SET` JSON body into a [`ResultPage`]. The result-set
/// shape (`columns: [{name, path}]`, `rows: [[…]]`) is the ITS-REST Query API
/// contract.
///
/// # Errors
/// [`AdminUiError::Internal`] when the body is not valid JSON.
pub(crate) fn parse_result_set(body: &str, offset: u32) -> Result<ResultPage, AdminUiError> {
    let doc: Value = serde_json::from_str(body)
        .map_err(|e| AdminUiError::Internal(format!("AQL result JSON: {e}")))?;
    let columns = doc
        .get("columns")
        .and_then(Value::as_array)
        .map(|cols| {
            cols.iter()
                .enumerate()
                .map(|(i, col)| {
                    col.get("name")
                        .and_then(Value::as_str)
                        .or_else(|| col.get("path").and_then(Value::as_str))
                        .map_or_else(|| format!("#{i}"), str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default();
    let rows = doc
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| row.as_array().cloned().unwrap_or_default())
                .collect()
        })
        .unwrap_or_default();
    Ok(ResultPage {
        columns,
        rows,
        offset,
    })
}

/// List EHRs newest-first via [`LIST_EHRS_AQL`], one page at `offset`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] when the result set is not valid JSON.
#[server]
pub async fn list_ehrs(offset: u32) -> Result<ResultPage, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1("query/aql");
    let body = aql_request_body(LIST_EHRS_AQL, &serde_json::json!({}), offset);
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
    parse_result_set(&response.body, offset)
}

/// The `/ehrs` screen: a lookup form over a URL-paged recent-EHRs table.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn EhrsPage() -> impl IntoView {
    let finder = finder_section();
    let offset = offset_from_url();
    let table = recent_ehrs_section(offset);

    view! {
        <Title text="EHRs · ehrbase-admin" />
        <div class="p-4">
            <h1 class="text-xl font-semibold mb-4">"EHRs"</h1>
            {finder}
            {table}
        </div>
    }
}

/// The offset the recent-EHRs table is paged at, read from `?offset=` and
/// clamped to a valid `u32` (bad input reads as 0). Deterministic from the
/// URL, so hydration-safe (rules §8/§9).
fn offset_from_url() -> Signal<u32> {
    let query = leptos_router::hooks::use_query_map();
    Signal::derive(move || {
        query
            .with(|q| q.get("offset"))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    })
}

/// The lookup form: an EHR-id input plus a Find button that navigates to the
/// detail route. Client-side navigation (`use_navigate`) — no page reload.
fn finder_section() -> AnyView {
    let lookup = RwSignal::new(String::new());
    let navigate = leptos_router::hooks::use_navigate();
    let on_click = move |_| {
        let id = lookup.get().trim().to_owned();
        if !id.is_empty() {
            navigate(
                &format!("/ehrs/{id}"),
                leptos_router::NavigateOptions::default(),
            );
        }
    };
    // TODO: offer a no-JS fallback (a <Form method="GET"> that posts the id to
    // a route which redirects to /ehrs/{id}) so the finder works before WASM
    // loads; the button+navigate path covers the hydrated case for now.
    view! {
        <div class="mb-6 flex items-end gap-2">
            <div class="flex flex-col gap-1">
                // Plain label + explicit stable input id: thaw::Field hardwires
                // its <label for> to a per-render random UUID, breaking
                // SSR↔hydration determinism (rules §8); an explicit id on
                // thaw::Input keeps the association deterministic without Field.
                <label class="text-sm font-medium" r#for="ehr-lookup">
                    "EHR id or subject id"
                </label>
                <thaw::Input id="ehr-lookup" value=lookup placeholder="ehr_id (UUID)" />
            </div>
            <thaw::Button appearance=thaw::ButtonAppearance::Primary on_click=on_click>
                "Find"
            </thaw::Button>
        </div>
    }
    .into_any()
}

/// The recent-EHRs table section: an AQL-backed [`Resource`] under a
/// `<Transition>` (old rows stay visible across paging — rules §6) that
/// resolves its `Result` inside the transition (an SSR'd `ErrorBoundary`
/// fallback mismatches at hydration in leptos 0.8), and prev/next links.
fn recent_ehrs_section(offset: Signal<u32>) -> AnyView {
    let resource = Resource::new(
        move || offset.get(),
        |offset| async move { list_ehrs(offset).await },
    );
    view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match resource.await {
                    Ok(page) => ehrs_table(&page),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// Render one page of EHRs: a table whose id cells link to the detail route,
/// plus prev/next paging links. The empty page is a first-class state.
fn ehrs_table(page: &ResultPage) -> AnyView {
    if page.rows.is_empty() {
        return view! { <p class="text-sm text-neutral-500">"No EHRs found."</p> }.into_any();
    }
    let headers = page
        .columns
        .iter()
        .map(|name| {
            view! { <th class="text-left font-medium text-neutral-500 py-1 pr-4">{name.clone()}</th> }
        })
        .collect::<Vec<_>>();
    let rows = page.rows.clone();
    let body = view! {
        <For
            each=move || rows.clone()
            key=|row| row.first().map(cell_text).unwrap_or_default()
            let:row
        >
            {ehrs_row(&row)}
        </For>
    };
    let paging = paging_controls(page.offset, page.rows.len(), "/ehrs");
    view! {
        <div class="overflow-x-auto">
            <table class="w-full text-sm border-collapse">
                <thead>
                    <tr class="border-b border-neutral-200 dark:border-neutral-700">{headers}</tr>
                </thead>
                <tbody>{body}</tbody>
            </table>
        </div>
        {paging}
    }
    .into_any()
}

/// One EHR row: the first cell (`ehr_id`) links to `/ehrs/{id}`; the rest are
/// plain text.
fn ehrs_row(row: &[Value]) -> AnyView {
    let id = row.first().map(cell_text).unwrap_or_default();
    let cells = row
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let text = cell_text(value);
            if i == 0 {
                let href = format!("/ehrs/{id}");
                view! {
                    <td class="py-1 pr-4 font-mono">
                        <A href=href attr:class="text-blue-600 hover:underline">
                            {text}
                        </A>
                    </td>
                }
                .into_any()
            } else {
                view! { <td class="py-1 pr-4">{text}</td> }.into_any()
            }
        })
        .collect::<Vec<_>>();
    view! { <tr class="border-b border-neutral-100 dark:border-neutral-800">{cells}</tr> }
        .into_any()
}

/// Prev/next paging links for an AQL-paged table at `base` (e.g. `/ehrs`).
/// Prev appears when `offset > 0`; next appears when the page is full (there
/// may be more). Offsets use saturating arithmetic (reliability rule).
pub(crate) fn paging_controls(offset: u32, row_count: usize, base: &str) -> AnyView {
    let full = u32::try_from(row_count).unwrap_or(u32::MAX) >= PAGE_SIZE;
    let prev = (offset > 0).then(|| {
        let href = format!("{base}?offset={}", offset.saturating_sub(PAGE_SIZE));
        view! {
            <A href=href attr:class="text-sm text-blue-600 hover:underline">
                "← Previous"
            </A>
        }
        .into_any()
    });
    let next = full.then(|| {
        let href = format!("{base}?offset={}", offset.saturating_add(PAGE_SIZE));
        view! {
            <A href=href attr:class="text-sm text-blue-600 hover:underline">
                "Next →"
            </A>
        }
        .into_any()
    });
    view! { <div class="mt-3 flex gap-4">{prev}{next}</div> }.into_any()
}

/// The `<Transition>` fallback shared by the AQL tables: three skeleton bars.
pub(crate) fn table_skeleton() -> impl IntoView {
    view! {
        <thaw::Skeleton>
            <thaw::SkeletonItem class="h-4 mb-2" />
            <thaw::SkeletonItem class="h-4 mb-2" />
            <thaw::SkeletonItem class="h-4" />
        </thaw::Skeleton>
    }
}

/// Render one raw AQL cell value as display text: strings verbatim, JSON null
/// as empty, anything else as compact JSON.
pub(crate) fn cell_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{LIST_EHRS_AQL, PAGE_SIZE, aql_request_body, cell_text, parse_result_set};

    #[test]
    fn fixed_aql_parses() {
        openehr_query::parser::parse_str(LIST_EHRS_AQL)
            .expect("the recent-EHRs AQL const must parse");
    }

    #[test]
    fn request_body_carries_params_and_window() {
        let body = aql_request_body(
            "SELECT e/ehr_id/value FROM EHR e",
            &serde_json::json!({}),
            50,
        );
        let doc: serde_json::Value = serde_json::from_str(&body).expect("valid JSON body");
        assert_eq!(doc["q"], "SELECT e/ehr_id/value FROM EHR e");
        assert_eq!(doc["fetch"], serde_json::json!(PAGE_SIZE));
        assert_eq!(doc["offset"], serde_json::json!(50));
        assert!(doc["query_parameters"].is_object());
    }

    #[test]
    fn parses_result_set_columns_and_rows() {
        let body = r#"{
            "columns": [{"name": "ehr_id", "path": "e/ehr_id/value"}, {"path": "e/time_created/value"}],
            "rows": [["7d44", "2026-07-12T00:00:00Z"], ["ab01", null]]
        }"#;
        let page = parse_result_set(body, 25).expect("valid result set");
        assert_eq!(page.columns, vec!["ehr_id", "e/time_created/value"]);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.offset, 25);
    }

    #[test]
    fn cell_text_renders_scalars_and_null() {
        assert_eq!(cell_text(&serde_json::json!("hello")), "hello");
        assert_eq!(cell_text(&serde_json::json!(null)), "");
        assert_eq!(cell_text(&serde_json::json!(42)), "42");
    }
}
