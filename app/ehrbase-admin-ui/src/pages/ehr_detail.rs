//! The `/ehrs/{ehr_id}` screen — EHR detail: status / directory / compositions /
//! contributions tabs.
//!
//! Four URL-driven tabs (`?tab=`, rules §9) over one EHR. Each tab's data is a
//! `#[server]` fn co-located here; the resources are created once and their
//! sources are gated on the active tab (a `Memo` over the query map), so only
//! the visible tab fetches (rules §6 — never fetch-in-effect). The tab bodies
//! are always mounted and toggled with `class:hidden`, keeping the server and
//! client view structure identical (rules §8 — no `cfg!`-branched structure).
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

use crate::components::data_table::{CELL, CELL_MONO, ROW, table_shell};
use crate::components::field::{BTN_PRIMARY, BTN_SECONDARY, INPUT, LABEL, SELECT, TEXTAREA};
use crate::components::format_view::DocumentPane;
use crate::components::page_header::{Crumb, PageHeader};
use crate::components::surface::{CARD_PAD, CARD_TITLE, WELL};
use crate::components::toast::toast_success;
use crate::error::AdminUiError;
use crate::format::ReprFormat;
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

#[cfg(feature = "ssr")]
/// The new version uid of a just-committed COMPOSITION: `uid.value` from the
/// `Prefer: return=representation` body (an `OBJECT_VERSION_ID`). Empty when
/// the CDR returned no representation body — the UI then shows a generic
/// success message rather than a uid.
pub(crate) fn commit_version_uid(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|doc| {
            doc.get("uid")
                .and_then(|u| u.get("value"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

/// Commit a new COMPOSITION to the EHR (`POST /ehr/{ehr_id}/composition`). The
/// `format` picks the `Content-Type` (canonical JSON `application/json`,
/// canonical XML `application/xml`, FLAT `application/openehr.wt.flat+json`);
/// a FLAT commit additionally requires the `openehr-template-id` header.
/// `Accept: application/json` + `Prefer: return=representation` yields a
/// canonical composition body whose `uid.value` is returned as the new
/// version uid.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] on an empty body or a FLAT commit without a
/// template id; CDR transport errors pass through; a non-2xx CDR answer (its
/// validation diagnostics, which the UI renders verbatim, included)
/// normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn commit_composition(
    ehr_id: String,
    format: ReprFormat,
    template_id: String,
    body: String,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    if body.trim().is_empty() {
        return Err(AdminUiError::Invalid(
            "the composition body is empty".to_owned(),
        ));
    }
    let template_id = template_id.trim();
    let mut headers: Vec<(&str, &str)> = vec![("Prefer", "return=representation")];
    if matches!(format, ReprFormat::Flat) {
        if template_id.is_empty() {
            return Err(AdminUiError::Invalid(
                "a template id is required to commit a FLAT composition".to_owned(),
            ));
        }
        headers.push(("openehr-template-id", template_id));
    }
    let url = state
        .cdr
        .rest_v1(&format!("ehr/{}/composition", urlencoding::encode(&ehr_id)));
    let response = state
        .cdr
        .post(
            &session.credential,
            &url,
            format.media_type(),
            "application/json",
            &headers,
            body,
        )
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    Ok(commit_version_uid(&response.body))
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
    // Tab state lives in the URL (`?tab=`, rules §9): shareable and refresh-safe.
    // A Memo (not an Effect) derives the active tab, defaulting to "status".
    let selected: Memo<String> = Memo::new(move |_| {
        query
            .with(|q| q.get("tab"))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "status".to_owned())
    });

    let status = status_section(ehr_id, selected);
    let directory = directory_section(ehr_id, selected);
    let compositions = compositions_section(ehr_id, offset, selected);
    let contributions = contributions_section(ehr_id, selected);

    let heading = Signal::derive(move || {
        let id = ehr_id.get();
        let short: String = id.chars().take(8).collect();
        format!("EHR {short}…")
    });

    let tabs = tab_bar(ehr_id, selected);

    view! {
        <Title text="EHR detail · ehrbase-admin" />
        <div class="p-6">
            <PageHeader
                title=Signal::derive(move || heading.get())
                crumbs=vec![Crumb::new("EHRs", "/ehrs")]
                mono=true
            />
            {tabs}
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

/// The URL-driven tab bar: four pill anchors (`?tab=…`) replacing the thaw
/// `TabList`. Selected = `bg-accent-subtle text-accent-ink`; idle =
/// `text-ink-muted hover:bg-sunken`. Plain anchors keep the tabs working
/// before hydration (the router intercepts them once WASM loads).
fn tab_bar(ehr_id: Signal<String>, selected: Memo<String>) -> AnyView {
    let link = move |value: &'static str, label: &'static str| {
        let href = move || format!("/ehrs/{}?tab={value}", ehr_id.get());
        let class = move || {
            if selected.get() == value {
                "rounded-control px-3 py-1.5 text-sm font-medium bg-accent-subtle text-accent-ink"
            } else {
                "rounded-control px-3 py-1.5 text-sm font-medium text-ink-muted hover:bg-sunken"
            }
        };
        view! {
            <a href=href class=class>
                {label}
            </a>
        }
    };
    view! {
        <div class="flex flex-wrap gap-1 border-b border-edge pb-2">
            {link("status", "Status")} {link("directory", "Directory")}
            {link("compositions", "Compositions")} {link("contributions", "Contributions")}
        </div>
    }
    .into_any()
}

/// Status tab: `fetch_ehr_status` → queryable/modifiable badges, the subject,
/// and the raw JSON in a [`DocumentPane`]. The source is gated on the tab
/// being active so it fetches only when shown.
fn status_section(ehr_id: Signal<String>, selected: Memo<String>) -> AnyView {
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
        <div class=format!("{CARD_PAD} flex flex-col gap-3")>
            <div class="flex flex-wrap gap-2 items-center">
                {capability_badge("queryable", queryable)}
                {capability_badge("modifiable", modifiable)}
            </div>
            <div class="text-sm">
                <span class="font-medium text-ink-muted">"subject: "</span>
                <span class="font-mono break-all text-ink">{subject}</span>
            </div>
            {(!queryable)
                .then(|| {
                    view! {
                        <div
                            role="status"
                            class="rounded-control border border-warn/40 bg-warn-subtle px-3 py-2 text-sm text-warn"
                        >
                            "This EHR is not queryable — AQL over it returns nothing."
                        </div>
                    }
                })}
            <DocumentPane body=doc_sig />
        </div>
    }
    .into_any())
}

/// An ok/danger capability chip for an `EHR_STATUS` boolean flag.
fn capability_badge(label: &'static str, on: bool) -> AnyView {
    let (mark, class) = if on {
        ("✓", "bg-ok-subtle text-ok")
    } else {
        ("✗", "bg-danger-subtle text-danger")
    };
    view! {
        <span class=format!(
            "inline-flex items-center gap-1 rounded-control px-2 py-0.5 text-xs font-medium {class}",
        )>{mark} " " {label}</span>
    }
    .into_any()
}

/// Directory tab: `fetch_directory` → a recursive `FOLDER` tree, or the
/// "no directory" empty state when the CDR 404s.
fn directory_section(ehr_id: Signal<String>, selected: Memo<String>) -> AnyView {
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
                                    <p class="text-sm text-ink-muted">
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
    Ok(view! {
        <section class=CARD_PAD>
            <ul class="text-sm text-ink">{folder_node(&doc)}</ul>
        </section>
    }
    .into_any())
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
            <span class="font-medium text-ink">"📁 " {name}</span>
            <ul class="pl-4 ml-2 border-l border-edge">{subfolders} {items}</ul>
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
        <li class="py-0.5 text-ink-muted">
            "• " <span class="uppercase text-xs mr-1">{ref_type}</span>
            <span class="font-mono break-all">{id}</span>
        </li>
    }
    .into_any()
}

/// Compositions tab: `list_compositions` (AQL) → a paged table whose uid
/// cells link to the composition viewer (under `<Transition>` so paging keeps
/// old rows visible), plus a "Commit composition" form below it. A successful
/// commit bumps the commit action's version — a source of the list resource —
/// refetching the table (rules §6 — never fetch-in-effect).
fn compositions_section(
    ehr_id: Signal<String>,
    offset: Signal<u32>,
    selected: Memo<String>,
) -> AnyView {
    let toaster = thaw::ToasterInjection::expect_context();
    let commit = Action::new(
        |(ehr_id, format, template_id, body): &(String, ReprFormat, String, String)| {
            let ehr_id = ehr_id.clone();
            let format = *format;
            let template_id = template_id.clone();
            let body = body.clone();
            async move { commit_composition(ehr_id, format, template_id, body).await }
        },
    );
    // Toast the outcome on success (an outside-world side-effect — rules §2);
    // the failure diagnostic stays inline in the form.
    Effect::new(move |_| {
        if let Some(Ok(uid)) = commit.value().get() {
            let detail = if uid.is_empty() {
                "The composition was committed.".to_owned()
            } else {
                format!("New version {uid}")
            };
            toast_success(toaster, "Composition committed", &detail);
        }
    });
    let resource = Resource::new(
        move || {
            let version = commit.version().get();
            (selected.get() == "compositions").then(|| (ehr_id.get(), offset.get(), version))
        },
        |active| async move {
            match active {
                Some((id, offset, _)) => list_compositions(id, offset).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let table = view! {
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
    .into_any();
    let form = commit_form(ehr_id, commit);
    view! { <div>{table} {form}</div> }.into_any()
}

/// The "Commit composition" form: a format select, a template-id input shown
/// only for FLAT (its `openehr-template-id` header is required there — kept in
/// the DOM and toggled with `class:hidden` so the server and client view
/// structure stay identical, rules §8), a large body textarea, and a Commit
/// button dispatching the shared `commit` action.
fn commit_form(
    ehr_id: Signal<String>,
    commit: Action<(String, ReprFormat, String, String), Result<String, AdminUiError>>,
) -> AnyView {
    let format = RwSignal::new(ReprFormat::CanonicalJson);
    let template_id = RwSignal::new(String::new());
    let body = RwSignal::new(String::new());
    let is_flat = move || format.get() == ReprFormat::Flat;
    let on_commit = move |_| {
        commit.dispatch((ehr_id.get(), format.get(), template_id.get(), body.get()));
    };
    view! {
        <section class=format!("{CARD_PAD} mt-4")>
            <h2 class=CARD_TITLE>"Commit composition"</h2>
            <div class="flex flex-col gap-3">
                <div class="flex flex-wrap items-end gap-3">
                    <div class="flex flex-col gap-1">
                        <label class=LABEL r#for="commit-format">
                            "Format"
                        </label>
                        <select
                            id="commit-format"
                            class=SELECT
                            prop:value=move || format_value(format.get())
                            on:change=move |ev| {
                                format.set(format_from_value(&event_target_value(&ev)));
                            }
                        >
                            <option value="json">"Canonical JSON"</option>
                            <option value="xml">"Canonical XML"</option>
                            <option value="flat">"FLAT"</option>
                        </select>
                    </div>
                    <div class="flex flex-col gap-1" class:hidden=move || !is_flat()>
                        <label class=LABEL r#for="commit-template-id">
                            "Template id"
                        </label>
                        <input
                            id="commit-template-id"
                            type="text"
                            class=INPUT
                            placeholder="template id (required for FLAT)"
                            prop:value=move || template_id.get()
                            on:input:target=move |ev| template_id.set(ev.target().value())
                        />
                    </div>
                </div>
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="commit-body">
                        "Composition"
                    </label>
                    <textarea
                        id="commit-body"
                        class=format!("{TEXTAREA} min-h-[16rem]")
                        placeholder="paste the composition document (JSON, XML, or FLAT)…"
                        prop:value=move || body.get()
                        on:input:target=move |ev| body.set(ev.target().value())
                    >
                        {body.get_untracked()}
                    </textarea>
                </div>
                <div class="flex items-center gap-3">
                    <button
                        id="commit-submit"
                        type="button"
                        class=BTN_PRIMARY
                        disabled=Signal::derive(move || commit.pending().get())
                        on:click=on_commit
                    >
                        "Commit"
                    </button>
                    <Show when=move || commit.pending().get()>
                        <span class="text-sm text-ink-muted">"Committing…"</span>
                    </Show>
                </div>
                {commit_feedback(commit)}
            </div>
        </section>
    }
    .into_any()
}

/// The commit action's failure pane: the CDR's validation diagnostics
/// verbatim in a scrollable WELL (they are long and precious — a `<pre>`, not
/// a one-line error). Success is a toast (see [`compositions_section`]).
fn commit_feedback(
    commit: Action<(String, ReprFormat, String, String), Result<String, AdminUiError>>,
) -> AnyView {
    view! {
        {move || match commit.value().get() {
            Some(Err(error)) => {
                view! {
                    <div class=WELL>
                        <pre class="overflow-auto max-h-[40vh] whitespace-pre-wrap font-mono text-xs text-danger">
                            {error.to_string()}
                        </pre>
                    </div>
                }
                    .into_any()
            }
            _ => ().into_any(),
        }}
    }
    .into_any()
}

/// The `<select>` option value for a committable format.
fn format_value(format: ReprFormat) -> &'static str {
    match format {
        ReprFormat::CanonicalXml => "xml",
        ReprFormat::Flat => "flat",
        _ => "json",
    }
}

/// The committable format for a `<select>` option value (unknown → canonical
/// JSON).
fn format_from_value(value: &str) -> ReprFormat {
    match value {
        "xml" => ReprFormat::CanonicalXml,
        "flat" => ReprFormat::Flat,
        _ => ReprFormat::CanonicalJson,
    }
}

/// Render one page of compositions: a table whose uid cell links to the
/// composition viewer (the versioned-object id — any `::system::version`
/// suffix stripped for the link, the full uid kept visible), plus paging.
fn compositions_table(page: &ResultPage, ehr_id: &str) -> AnyView {
    if page.rows.is_empty() {
        return view! { <p class="text-sm text-ink-muted">"No compositions in this EHR."</p> }
            .into_any();
    }
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
    }
    .into_any();
    let paging = paging_controls(page.offset, page.rows.len(), &format!("/ehrs/{ehr_id}"));
    view! {
        {table_shell(&["Composition", "Name", "Template", "Started"], body)}
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
                    <td class=CELL_MONO>
                        <A href=href attr:class="text-accent hover:underline">
                            {text}
                        </A>
                    </td>
                }
                .into_any()
            } else {
                view! { <td class=CELL>{text}</td> }.into_any()
            }
        })
        .collect::<Vec<_>>();
    view! { <tr class=ROW>{cells}</tr> }.into_any()
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
fn contributions_section(ehr_id: Signal<String>, selected: Memo<String>) -> AnyView {
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
        <section class=format!("{CARD_PAD} mb-4")>
            <div class="flex flex-wrap items-end gap-3">
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="contribution-uid">
                        "Contribution uid"
                    </label>
                    <input
                        id="contribution-uid"
                        type="text"
                        class=INPUT
                        placeholder="contribution uid (UUID)"
                        prop:value=move || uid_input.get()
                        on:input:target=move |ev| uid_input.set(ev.target().value())
                    />
                </div>
                <button type="button" class=BTN_SECONDARY on:click=on_click>
                    "Look up"
                </button>
            </div>
        </section>
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
                            <p class="text-sm text-ink-muted">
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
    use super::{
        LIST_COMPOSITIONS_AQL, commit_version_uid, format_from_value, format_value,
        versioned_object_id,
    };
    use crate::format::ReprFormat;

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

    #[test]
    fn commit_version_uid_reads_uid_value_or_empty() {
        let body =
            r#"{"_type":"COMPOSITION","uid":{"_type":"OBJECT_VERSION_ID","value":"7d44::sys::1"}}"#;
        assert_eq!(commit_version_uid(body), "7d44::sys::1");
        // A return=minimal (empty) or non-JSON body yields no uid.
        assert_eq!(commit_version_uid(""), "");
        assert_eq!(commit_version_uid("{}"), "");
    }

    #[test]
    fn format_value_round_trips() {
        for format in [
            ReprFormat::CanonicalJson,
            ReprFormat::CanonicalXml,
            ReprFormat::Flat,
        ] {
            assert_eq!(format_from_value(format_value(format)), format);
        }
        // An unknown value falls back to canonical JSON.
        assert_eq!(format_from_value("bogus"), ReprFormat::CanonicalJson);
    }
}
