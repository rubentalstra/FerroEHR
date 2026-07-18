//! The `/ehrs/{ehr_id}` screen — EHR detail: status / directory / compositions /
//! contributions tabs.
//!
//! Four `thaw::TabList` tabs over one EHR. Each tab's data is a `#[server]`
//! fn co-located here; the resources are created once and their sources are
//! gated on the active tab, so only the visible tab fetches (rules §6 — never
//! fetch-in-effect). The tab bodies are always mounted and toggled with
//! `class:hidden`, keeping the server and client view structure identical
//! (rules §8 — no `cfg!`-branched structure).
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The wire it reads IS spec-bound (ITS-REST EHR + Query APIs). User input
//! NEVER concatenates into AQL — the fixed query is a validated const and the
//! `ehr_id` travels as an AQL `query_parameters` binding; path segments are
//! percent-encoded server-side.
//!
//! Every co-located `#[server]` fn guards with
//! [`require_session`](crate::session::require_session) first (rules §0), and
//! the CDR credential never reaches client-visible state.

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;
use leptos_router::components::A;
use serde_json::Value;

use crate::components::format_view::DocumentPane;
use crate::error::AdminUiError;
use crate::pages::ehrs::{ResultPage, cell_text, paging_controls, table_skeleton};
// Server-side helpers, compiled only where the #[server] bodies exist.
#[cfg(feature = "ssr")]
use crate::pages::ehrs::{aql_request_body, parse_result_set};

#[cfg(feature = "ssr")]
/// The fixed AQL that lists an EHR's compositions newest-first. The `ehr_id`
/// is bound as an AQL parameter (`$ehr_id`), never string-interpolated.
/// Validated by [`tests::fixed_aql_parses`].
const LIST_COMPOSITIONS_AQL: &str = "SELECT c/uid/value, c/name/value, \
c/archetype_details/template_id/value, c/context/start_time/value \
FROM EHR e CONTAINS COMPOSITION c WHERE e/ehr_id/value = $ehr_id \
ORDER BY c/context/start_time/value DESC";

/// The EHR's `EHR_STATUS` resource, raw canonical JSON.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_ehr_status(ehr_id: String) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state
        .cdr
        .rest_v1(&format!("ehr/{}/ehr_status", urlencoding::encode(&ehr_id)));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    Ok(crate::cdr::CdrClient::expect_success(response)?.body)
}

/// The EHR's directory (root `FOLDER`) as raw canonical JSON, or `None` when
/// the CDR has no directory for this EHR (a `404` is a first-class empty
/// state, not an error).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx, non-404 CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_directory(ehr_id: String) -> Result<Option<String>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state
        .cdr
        .rest_v1(&format!("ehr/{}/directory", urlencoding::encode(&ehr_id)));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    if response.status == 404 {
        return Ok(None);
    }
    Ok(Some(crate::cdr::CdrClient::expect_success(response)?.body))
}

/// List an EHR's compositions via [`LIST_COMPOSITIONS_AQL`], one page at
/// `offset`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] when the result set is not valid JSON.
#[server]
pub async fn list_compositions(ehr_id: String, offset: u32) -> Result<ResultPage, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1("query/aql");
    let body = aql_request_body(
        LIST_COMPOSITIONS_AQL,
        &serde_json::json!({ "ehr_id": ehr_id }),
        offset,
    );
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

/// Look up a single CONTRIBUTION by uid (ITS-REST exposes only GET-by-id — no
/// list surface — so the Contributions tab is a lookup box; see
/// [`contributions_section`]). Returns the raw canonical JSON.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer (a `404` for an unknown uid
/// included) normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_contribution(
    ehr_id: String,
    contribution_uid: String,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1(&format!(
        "ehr/{}/contribution/{}",
        urlencoding::encode(&ehr_id),
        urlencoding::encode(&contribution_uid)
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    Ok(crate::cdr::CdrClient::expect_success(response)?.body)
}

/// The `/ehrs/{ehr_id}` screen: the tab bar plus four always-mounted,
/// visibility-toggled tab bodies.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn EhrDetailPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let ehr_id = Signal::derive(move || params.with(|p| p.get("ehr_id").unwrap_or_default()));
    let query = leptos_router::hooks::use_query_map();
    let offset = Signal::derive(move || {
        query
            .with(|q| q.get("offset"))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    });
    let selected = RwSignal::new("status".to_owned());

    let status = status_section(ehr_id, selected);
    let directory = directory_section(ehr_id, selected);
    let compositions = compositions_section(ehr_id, offset, selected);
    let contributions = contributions_section(ehr_id, selected);

    let heading = Signal::derive(move || {
        let id = ehr_id.get();
        let short: String = id.chars().take(8).collect();
        format!("EHR {short}…")
    });

    view! {
        <Title text="EHR detail · ehrbase-admin" />
        <div class="p-4">
            <div class="flex items-center gap-3 mb-4">
                <A href="/ehrs" attr:class="text-sm text-blue-600 hover:underline">
                    "← EHRs"
                </A>
                <h1 class="text-xl font-semibold font-mono">{move || heading.get()}</h1>
            </div>
            <thaw::TabList selected_value=selected>
                <thaw::Tab value="status">"Status"</thaw::Tab>
                <thaw::Tab value="directory">"Directory"</thaw::Tab>
                <thaw::Tab value="compositions">"Compositions"</thaw::Tab>
                <thaw::Tab value="contributions">"Contributions"</thaw::Tab>
            </thaw::TabList>
            <div class="mt-4">
                <div class:hidden=move || selected.get() != "status">{status}</div>
                <div class:hidden=move || selected.get() != "directory">{directory}</div>
                <div class:hidden=move || selected.get() != "compositions">{compositions}</div>
                <div class:hidden=move || {
                    selected.get() != "contributions"
                }>{contributions}</div>
            </div>
        </div>
    }
}

/// Status tab: `fetch_ehr_status` → queryable/modifiable badges, the subject,
/// and the raw JSON in a [`DocumentPane`]. The source is gated on the tab
/// being active so it fetches only when shown.
fn status_section(ehr_id: Signal<String>, selected: RwSignal<String>) -> AnyView {
    let resource = Resource::new(
        move || (selected.get() == "status").then(|| ehr_id.get()),
        |active| async move {
            match active {
                Some(id) => fetch_ehr_status(id).await.map(Some),
                None => Ok(None),
            }
        },
    );
    view! {
        <Suspense fallback=table_skeleton>
            {move || Suspend::new(async move {
                let rendered = resource
                    .await
                    .and_then(|opt| match opt {
                        Some(body) => status_body(&body),
                        None => Ok(().into_any()),
                    });
                match rendered {
                    Ok(view) => view,
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// Render an `EHR_STATUS` document: the two capability badges, the subject
/// reference, and the raw JSON.
///
/// # Errors
/// [`AdminUiError::Internal`] when the body is not valid JSON.
fn status_body(body: &str) -> Result<AnyView, AdminUiError> {
    let doc: Value = serde_json::from_str(body)
        .map_err(|e| AdminUiError::Internal(format!("ehr_status JSON: {e}")))?;
    let queryable = doc
        .get("is_queryable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let modifiable = doc
        .get("is_modifiable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let subject = doc
        .get("subject")
        .and_then(|s| s.get("external_ref"))
        .and_then(|r| r.get("id"))
        .and_then(|i| i.get("value"))
        .and_then(Value::as_str)
        .map_or_else(|| "self (no external subject)".to_owned(), str::to_owned);
    let pretty =
        crate::components::format_view::pretty_body(body, crate::format::ReprFormat::CanonicalJson);
    let doc_sig = RwSignal::new(pretty);
    Ok(view! {
        <div class="flex flex-col gap-3">
            <div class="flex flex-wrap gap-2 items-center">
                {capability_badge("queryable", queryable)}
                {capability_badge("modifiable", modifiable)}
            </div>
            <div class="text-sm">
                <span class="font-medium text-neutral-500">"subject: "</span>
                <span class="font-mono break-all">{subject}</span>
            </div>
            {(!queryable)
                .then(|| {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Warning>
                            <thaw::MessageBarBody>
                                "This EHR is not queryable — AQL over it returns nothing."
                            </thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                })}
            <DocumentPane body=doc_sig />
        </div>
    }
    .into_any())
}

/// A green/red capability chip for an `EHR_STATUS` boolean flag.
fn capability_badge(label: &'static str, on: bool) -> AnyView {
    let (mark, class) = if on {
        ("✓", "text-emerald-600 border-emerald-500")
    } else {
        ("✗", "text-red-600 border-red-500")
    };
    view! {
        <span class=format!(
            "inline-flex items-center gap-1 rounded border px-2 py-0.5 text-xs font-medium {class}",
        )>{mark} " " {label}</span>
    }
    .into_any()
}

/// Directory tab: `fetch_directory` → a recursive `FOLDER` tree, or the
/// "no directory" empty state when the CDR 404s.
fn directory_section(ehr_id: Signal<String>, selected: RwSignal<String>) -> AnyView {
    let resource = Resource::new(
        move || (selected.get() == "directory").then(|| ehr_id.get()),
        |active| async move {
            match active {
                Some(id) => fetch_directory(id).await,
                None => Ok(None),
            }
        },
    );
    view! {
        <Suspense fallback=table_skeleton>
            {move || Suspend::new(async move {
                let rendered = resource
                    .await
                    .and_then(|opt| match opt {
                        Some(body) => directory_body(&body),
                        None => {
                            Ok(
                                // Resolve inside the Suspense: an SSR'd ErrorBoundary fallback
                                // mismatches at hydration in leptos 0.8 (E2E console gate).
                                view! {
                                    <p class="text-sm text-neutral-500">
                                        "No directory for this EHR."
                                    </p>
                                }
                                    .into_any(),
                            )
                        }
                    });
                match rendered {
                    Ok(view) => view,
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// Render the directory root `FOLDER` as a nested list.
///
/// # Errors
/// [`AdminUiError::Internal`] when the body is not valid JSON.
fn directory_body(body: &str) -> Result<AnyView, AdminUiError> {
    let doc: Value = serde_json::from_str(body)
        .map_err(|e| AdminUiError::Internal(format!("directory JSON: {e}")))?;
    Ok(view! { <ul class="text-sm">{folder_node(&doc)}</ul> }.into_any())
}

/// One `FOLDER` node: its name, its child folders (recursively), and its
/// item references. Returns [`AnyView`] (recursive tree fns erase — rules §1).
fn folder_node(folder: &Value) -> AnyView {
    let name = folder
        .get("name")
        .and_then(|n| n.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("(folder)")
        .to_owned();
    let subfolders = folder
        .get("folders")
        .and_then(Value::as_array)
        .map(|folders| folders.iter().map(folder_node).collect::<Vec<_>>())
        .unwrap_or_default();
    let items = folder
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(item_ref_node).collect::<Vec<_>>())
        .unwrap_or_default();
    view! {
        <li class="py-0.5">
            <span class="font-medium">"📁 " {name}</span>
            <ul class="pl-4 ml-2 border-l border-neutral-200 dark:border-neutral-700">
                {subfolders} {items}
            </ul>
        </li>
    }
    .into_any()
}

/// One `OBJECT_REF` item under a folder: its type and id value.
fn item_ref_node(item: &Value) -> AnyView {
    let id = item
        .get("id")
        .and_then(|i| i.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("(ref)")
        .to_owned();
    let ref_type = item
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("OBJECT")
        .to_owned();
    view! {
        <li class="py-0.5 text-neutral-600 dark:text-neutral-400">
            "• " <span class="uppercase text-xs mr-1">{ref_type}</span>
            <span class="font-mono break-all">{id}</span>
        </li>
    }
    .into_any()
}

/// Compositions tab: `list_compositions` (AQL) → a paged table whose uid
/// cells link to the composition viewer. Under `<Transition>` so paging keeps
/// old rows visible.
fn compositions_section(
    ehr_id: Signal<String>,
    offset: Signal<u32>,
    selected: RwSignal<String>,
) -> AnyView {
    let resource = Resource::new(
        move || (selected.get() == "compositions").then(|| (ehr_id.get(), offset.get())),
        |active| async move {
            match active {
                Some((id, offset)) => list_compositions(id, offset).await.map(Some),
                None => Ok(None),
            }
        },
    );
    view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match resource.await {
                    Ok(Some(page)) => compositions_table(&page, &ehr_id.get()),
                    Ok(None) => ().into_any(),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// Render one page of compositions: a table whose uid cell links to the
/// composition viewer (the versioned-object id — any `::system::version`
/// suffix stripped for the link, the full uid kept visible), plus paging.
fn compositions_table(page: &ResultPage, ehr_id: &str) -> AnyView {
    if page.rows.is_empty() {
        return view! { <p class="text-sm text-neutral-500">"No compositions in this EHR."</p> }
            .into_any();
    }
    let headers = page
        .columns
        .iter()
        .map(|name| {
            view! { <th class="text-left font-medium text-neutral-500 py-1 pr-4">{name.clone()}</th> }
        })
        .collect::<Vec<_>>();
    let rows = page.rows.clone();
    let ehr_id_owned = ehr_id.to_owned();
    let body = view! {
        <For
            each=move || rows.clone()
            key=|row| row.first().map(cell_text).unwrap_or_default()
            let:row
        >
            {composition_row(&row, &ehr_id_owned)}
        </For>
    };
    let paging = paging_controls(page.offset, page.rows.len(), &format!("/ehrs/{ehr_id}"));
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

/// One composition row: the uid cell links to the viewer at the
/// versioned-object id; the full uid stays visible.
fn composition_row(row: &[Value], ehr_id: &str) -> AnyView {
    let uid = row.first().map(cell_text).unwrap_or_default();
    let vo_id = versioned_object_id(&uid).to_owned();
    let cells = row
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let text = cell_text(value);
            if i == 0 {
                let href = format!("/ehrs/{ehr_id}/compositions/{vo_id}");
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

/// The versioned-object id from an `OBJECT_VERSION_ID` value: everything
/// before the first `::` (`uuid::system::version` → `uuid`), which is what
/// the composition route keys on.
fn versioned_object_id(uid: &str) -> &str {
    uid.split_once("::").map_or(uid, |(head, _)| head)
}

/// Contributions tab: ITS-REST exposes CONTRIBUTION only by id (no list
/// surface — verified against `docs/endpoint-map.md`: only
/// `GET /ehr/{ehr_id}/contribution/{contribution_uid}` exists), so this is a
/// lookup box. Submitting a uid drives [`fetch_contribution`] under a
/// `<Transition>`.
fn contributions_section(ehr_id: Signal<String>, selected: RwSignal<String>) -> AnyView {
    let uid_input = RwSignal::new(String::new());
    let submitted = RwSignal::new(String::new());
    let on_click = move |_| submitted.set(uid_input.get().trim().to_owned());
    let resource = Resource::new(
        move || {
            let uid = submitted.get();
            (selected.get() == "contributions" && !uid.is_empty()).then(|| (ehr_id.get(), uid))
        },
        |active| async move {
            match active {
                Some((id, uid)) => fetch_contribution(id, uid).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let lookup = view! {
        <div class="mb-4 flex items-end gap-2">
            <div class="flex flex-col gap-1">
                <label class="text-sm font-medium" r#for="contribution-uid">
                    "Contribution uid"
                </label>
                <thaw::Input
                    id="contribution-uid"
                    value=uid_input
                    placeholder="contribution uid (UUID)"
                />
            </div>
            <thaw::Button appearance=thaw::ButtonAppearance::Primary on_click=on_click>
                "Look up"
            </thaw::Button>
        </div>
    }
    .into_any();
    let result = view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match resource.await {
                    Ok(Some(body)) => contribution_body(&body),
                    Ok(None) => {
                        // Resolve inside the Transition: an SSR'd ErrorBoundary fallback
                        // mismatches at hydration in leptos 0.8 (E2E console gate).
                        view! {
                            <p class="text-sm text-neutral-500">
                                "Enter a contribution uid to look it up."
                            </p>
                        }
                            .into_any()
                    }
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any();
    view! { <div>{lookup} {result}</div> }.into_any()
}

/// Render a fetched CONTRIBUTION as pretty JSON in the shared document pane.
fn contribution_body(body: &str) -> AnyView {
    let pretty =
        crate::components::format_view::pretty_body(body, crate::format::ReprFormat::CanonicalJson);
    let doc_sig = RwSignal::new(pretty);
    view! { <DocumentPane body=doc_sig /> }.into_any()
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{LIST_COMPOSITIONS_AQL, versioned_object_id};

    #[test]
    fn fixed_aql_parses() {
        openehr_query::parser::parse_str(LIST_COMPOSITIONS_AQL)
            .expect("the compositions AQL const must parse");
    }

    #[test]
    fn versioned_object_id_strips_the_version_suffix() {
        assert_eq!(
            versioned_object_id("7d44aa01::example.ehrbase.org::2"),
            "7d44aa01"
        );
        // A bare versioned-object id (no suffix) is returned unchanged.
        assert_eq!(versioned_object_id("7d44aa01"), "7d44aa01");
    }
}
