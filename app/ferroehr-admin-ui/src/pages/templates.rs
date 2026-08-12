// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The `/templates` screen — the Template Manager list + OPT upload.
//!
//! A list of the CDR's ADL 1.4 operational templates with a client-side text
//! filter, plus an OPT/XML upload that surfaces the CDR's validation
//! diagnostics verbatim. No openEHR spec governs an admin UI — our own design /
//! product extension; the wire it reads/writes is the ITS-REST Definition API.
//!
//! Discipline (rules §0/§1/§6/§8/§9): the two `#[server]` fns guard the session
//! first (a server fn is a public HTTP endpoint) and never let a CDR credential
//! reach client-visible state; the view is composed from `.into_any()`-erased
//! section locals; async is a [`Resource`] read under `<Transition>` and an
//! [`Action`] for the mutating upload (refetch the list on the action's
//! version); the table is the shared [`table_shell`], which emits an explicit
//! `<tbody>` (hydration correctness — rules §8), paged by the shared
//! [`table_footer`] whose page state lives in the URL.

#![allow(
    clippy::disallowed_types,
    reason = "the console consumes the CDR JSON wire over ITS-REST — not the CDR internal seams \
              (#1694); the carriers here are ssr-only, so #[expect] would be unfulfilled on the \
              hydrate target"
)]

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;

use crate::admin::AdminAvailability;
use crate::components::data_table::{
    CELL, CELL_MONO, ROW, TablePaging, page_rows, page_window, paging_from_url, row_total,
    table_footer, table_shell, table_skeleton,
};
use crate::components::empty_state::EmptyState;
use crate::components::field::{BTN_DANGER, INPUT};
use crate::components::page_header::PageHeader;
use crate::components::toast::{toast_error, toast_success};
use crate::error::AdminUiError;

/// The template-delete action: the id it was dispatched with, paired with the
/// CDR's answer, so both the success and the failure toast can name the exact
/// template (rules §6 — the action's value IS the mutation report).
type TemplateDeleteAction = Action<String, (String, Result<(), AdminUiError>)>;

/// One row of the template list, distilled from the ITS-REST Definition list
/// shape (`template_id` / `concept` / `created_timestamp`).
///
/// Shared across both compilation targets, so it carries only fixed-size,
/// client-safe fields (rules §1 — no `usize` in serialized types).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TemplateRow {
    /// The operational-template id (the detail route segment).
    pub template_id: String,
    /// The template concept / display name.
    pub concept: String,
    /// The root archetype id the template constrains.
    pub archetype_id: String,
    /// The creation timestamp as the CDR reported it (raw string).
    pub created: String,
}

/// List the CDR's ADL 1.4 operational templates.
///
/// GET `definition/template/adl1.4?version=*` with
/// `Accept: application/json`; the body is an array whose elements are parsed
/// defensively (missing fields become empty strings) into [`TemplateRow`]s.
/// `version=*` pins the FULL inventory: an absent `version` collapses the
/// CDR's listing to the latest version of each template (the released OAS
/// `parameters/query/filter_version.yaml`), and a management console must
/// show every stored version, not just the latest.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Forbidden`] / [`AdminUiError::Cdr`] /
/// [`AdminUiError::CdrUnreachable`] from the CDR; [`AdminUiError::Internal`]
/// when the CDR body is not valid JSON.
#[server]
pub async fn list_templates() -> Result<Vec<TemplateRow>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1("definition/template/adl1.4?version=*");
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    let body = crate::cdr::CdrClient::expect_success(response)?.body;
    let value = serde_json::from_str::<serde_json::Value>(&body)
        .map_err(|e| AdminUiError::Internal(format!("template list JSON: {e}")))?;
    let rows = value
        .as_array()
        .map(|items| items.iter().map(template_row).collect())
        .unwrap_or_default();
    Ok(rows)
}

/// Distil one Definition-list element into a [`TemplateRow`], reading each
/// field defensively so a missing/renamed field yields an empty string rather
/// than dropping the row.
#[cfg(feature = "ssr")]
fn template_row(item: &serde_json::Value) -> TemplateRow {
    let text = |key: &str| {
        item.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    TemplateRow {
        template_id: text("template_id"),
        concept: text("concept"),
        archetype_id: text("archetype_id"),
        created: text("created_timestamp"),
    }
}

/// Upload an operational template (OPT/XML) to the CDR.
///
/// POST `definition/template/adl1.4` with `Content-Type: application/xml`,
/// `Accept: application/json`, `Prefer: return=minimal`. The CDR's 400/409/422
/// validation diagnostics surface verbatim through
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success). On
/// success the (possibly empty) response body is returned; the UI ignores it
/// and refetches the list.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] for empty content;
/// [`AdminUiError::Forbidden`] / [`AdminUiError::Cdr`] /
/// [`AdminUiError::CdrUnreachable`] from the CDR (the diagnostic verbatim).
#[server]
pub async fn upload_template(
    /// The operational-template XML to upload, as read in the browser.
    opt_xml: String,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    if opt_xml.trim().is_empty() {
        return Err(AdminUiError::Invalid(
            "the selected file was empty".to_owned(),
        ));
    }
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1("definition/template/adl1.4");
    let response = state
        .cdr
        .post(
            &session.credential,
            &url,
            "application/xml",
            "application/json",
            &[("Prefer", "return=minimal")],
            opt_xml,
        )
        .await?;
    Ok(crate::cdr::CdrClient::expect_success(response)?.body)
}

/// The Template Manager screen: an upload bar, a client-side filter, and the
/// filterable template table.
///
/// The filter is a private client-side `contains` over already-loaded rows (a
/// bound signal, per the screen spec — no server round-trip, so URL state
/// would add nothing here); the PAGE the filtered rows are windowed at does
/// live in the URL (`?page=`/`?size=`, rules §9), so a reload or a shared link
/// lands on the same rows. A successful upload or delete bumps its action's
/// version, both of which are the list resource's source, refetching it.
///
/// The per-row delete is admin-gated: it renders only when the
/// [`admin_gate`](crate::admin::admin_gate) probe finds the CDR advertising its
/// Admin API (discover-and-hide — no admin group, no buttons). Whether the
/// session may USE it is the CDR's per-request answer, surfaced as actionable
/// copy on refusal.
#[expect(
    clippy::must_use_candidate,
    reason = "#[component] rewrites the fn; view!/mount always consumes the value"
)]
#[component]
pub fn TemplatesPage() -> impl IntoView {
    let toaster = thaw::ToasterInjection::expect_context();
    let filter = RwSignal::new(String::new());
    // The table's page window, read from the URL in SETUP (never inside the
    // suspense that fetches the list — rules §4).
    let paging = paging_from_url();
    let upload = Action::new(|opt_xml: &String| {
        let opt_xml = opt_xml.clone();
        async move { upload_template(opt_xml).await }
    });
    // The admin probe, and the delete action it gates. Both live in setup so
    // the gated view can re-render without re-creating them (rules §4).
    let gate = crate::admin::admin_gate();
    let delete: TemplateDeleteAction = Action::new(|template_id: &String| {
        let template_id = template_id.clone();
        async move {
            let outcome = crate::admin::admin_delete_template(template_id.clone()).await;
            (template_id, outcome)
        }
    });
    // The template awaiting confirmation in the modal (`None` = no dialog).
    // ONE dialog serves every row — the signal is both "which row" and "open".
    let pending_delete = RwSignal::new(Option::<String>::None);
    let list: Resource<Result<Vec<TemplateRow>, AdminUiError>> = Resource::new(
        move || (upload.version().get(), delete.version().get()),
        |_| async move { list_templates().await },
    );

    // Both outcomes toast (the console's mutation-feedback rule — crate
    // CLAUDE.md); the CDR's validation diagnostic ALSO keeps its inline
    // MessageBar, because an OPT rejection is a list worth reading line by
    // line. Dispatching a toast is a side-effect on the outside world (the
    // thaw toaster), so an Effect is its correct home (rules §2); it never
    // runs on the server pass.
    Effect::new(move |_| match upload.value().get() {
        Some(Ok(_)) => toast_success(
            toaster,
            "Template uploaded",
            "The operational template was accepted by the CDR.",
        ),
        Some(Err(error)) => crate::feedback::toast_write_failure(
            toaster,
            "Upload failed",
            "the operational template",
            &error,
        ),
        None => {}
    });

    // The delete outcome is reported as a toast naming the template: success
    // plainly, failure with the actionable copy (the CDR's in-use `409`
    // diagnostic included).
    Effect::new(move |_| match delete.value().get() {
        Some((template_id, Ok(()))) => toast_success(
            toaster,
            "Template deleted",
            &format!("{template_id} was removed from the CDR."),
        ),
        Some((template_id, Err(error))) => toast_error(
            toaster,
            "Delete failed",
            &crate::admin::delete_failure_copy(&format!("Template {template_id}"), &error),
        ),
        None => {}
    });

    let action_slot = upload_trigger(upload);
    let feedback = upload_feedback(upload);
    let table_section = templates_table(filter, paging, list, gate, delete, pending_delete);
    let confirm = delete_dialog(pending_delete, delete);

    view! {
        <Title text="Templates" />
        <div class="p-6">
            <PageHeader
                title="Templates"
                subtitle="Operational templates registered in the CDR (ADL 1.4)."
            >
                {action_slot}
            </PageHeader>
            <div class="mb-3">
                <input
                    type="text"
                    class=format!("w-full max-w-sm {INPUT}")
                    placeholder="filter by id or concept…"
                    prop:value=move || filter.get()
                    on:input:target=move |ev| filter.set(ev.target().value())
                />
            </div>
            {feedback}
            {table_section}
            {confirm}
        </div>
    }
}

/// The screen's ONE delete-confirmation modal, driven by `pending_delete`
/// (which row triggered it). Rendered once outside the table, so a list
/// refetch never re-creates it; it is inert (nothing in the DOM) while no row
/// is pending, which is also why it needs no admin gate of its own — only an
/// admin-gated trigger can set the signal.
fn delete_dialog(
    pending_delete: RwSignal<Option<String>>,
    delete: TemplateDeleteAction,
) -> AnyView {
    let message = Signal::derive(move || {
        pending_delete
            .get()
            .map_or_else(String::new, |template_id| {
                format!(
                    "Permanently delete the operational template “{template_id}” from the CDR? \
                 This cannot be undone. The CDR refuses the delete while a committed version \
                 still references the template."
                )
            })
    });
    view! {
        <crate::components::confirm_dialog::ConfirmDialog
            open=Signal::derive(move || pending_delete.get().is_some())
            title="Delete template"
            message=message
            confirm_label="Delete template"
            confirm_id="template-delete-confirm"
            on_cancel=Callback::new(move |()| pending_delete.set(None))
            on_confirm=Callback::new(move |()| {
                if let Some(template_id) = pending_delete.get_untracked() {
                    delete.dispatch(template_id);
                }
                pending_delete.set(None);
            })
        />
    }
    .into_any()
}

/// The upload trigger for the page-header action slot: a [`thaw::Upload`] whose
/// selected file is read to text browser-side (the `File` Web API via the
/// component, then
/// [`Blob::text`](https://developer.mozilla.org/en-US/docs/Web/API/Blob/text)),
/// dispatched to the [`upload_template`] action. Kept as `thaw::Upload` so it
/// renders a real `<input type="file">`.
fn upload_trigger(upload: Action<String, Result<String, AdminUiError>>) -> AnyView {
    // `custom_request` runs only in the browser (a file-selection event), so
    // reading the file with the Web `File`/`Blob` API here is hydration-safe
    // (rules §8 — browser-only APIs never run on the server pass).
    let custom_request = move |files: thaw::FileList| {
        let Some(file) = files.get(0) else {
            return;
        };
        let promise = file.text();
        leptos::task::spawn_local(async move {
            if let Ok(value) = wasm_bindgen_futures::JsFuture::from(promise).await
                && let Some(text) = value.as_string()
            {
                upload.dispatch(text);
            }
        });
    };

    view! {
        <thaw::Upload accept=Signal::derive(|| ".opt,.xml".to_owned()) custom_request>
            <thaw::Button appearance=thaw::ButtonAppearance::Primary>
                <leptos_icons::Icon icon=icondata_lu::LuUpload width="14" height="14" />
                " Upload OPT"
            </thaw::Button>
        </thaw::Upload>
    }
    .into_any()
}

/// The upload action's inline state: a pending hint plus the CDR diagnostic on
/// failure (verbatim — a validation payload worth reading line by line stays
/// inline ALONGSIDE the failure toast). Both outcomes also toast (see
/// [`TemplatesPage`]).
fn upload_feedback(upload: Action<String, Result<String, AdminUiError>>) -> AnyView {
    view! {
        <div class="text-sm mb-3">
            <Show when=move || upload.pending().get()>
                <span class="text-ink-muted">"Uploading…"</span>
            </Show>
            {move || match upload.value().get() {
                Some(Err(error)) => {
                    view! {
                        <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                            <thaw::MessageBarBody>{error.to_string()}</thaw::MessageBarBody>
                        </thaw::MessageBar>
                    }
                        .into_any()
                }
                _ => ().into_any(),
            }}
        </div>
    }
    .into_any()
}

/// The template table: the list resource read under `<Transition>` (keep the
/// current rows visible while a refetch runs — rules §6), resolving its
/// `Result` inside the transition (an SSR'd `ErrorBoundary` fallback mismatches
/// at hydration in leptos 0.8), then the filtered rows.
///
/// The admin probe is awaited in the SAME `Suspend` as the list (rules §6 —
/// several resources awaited in one suspend, no nested `Option` matching), so
/// the header row and the rows agree on whether the delete column exists.
fn templates_table(
    filter: RwSignal<String>,
    paging: TablePaging,
    list: Resource<Result<Vec<TemplateRow>, AdminUiError>>,
    gate: Resource<Result<AdminAvailability, AdminUiError>>,
    delete: TemplateDeleteAction,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                let admin = crate::admin::renders_admin_ops(&gate.await);
                match list.await {
                    Ok(rows) => rows_view(rows, filter, paging, admin, delete, pending_delete),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// Render the loaded rows: the empty-catalogue state, or a filterable, paged
/// table (with the admin delete column when `admin`).
///
/// The filtered listing is a [`Memo`], so the rendered window and the footer's
/// total are computed from ONE derivation of the filter and can never disagree
/// (rules §2 — a derived value, not an effect writing a signal). The window
/// itself comes from the URL, so paging never re-runs the enclosing suspense.
fn rows_view(
    rows: Vec<TemplateRow>,
    filter: RwSignal<String>,
    paging: TablePaging,
    admin: bool,
    delete: TemplateDeleteAction,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    if rows.is_empty() {
        return view! {
            <EmptyState
                icon=icondata_lu::LuFileCode2
                message="No templates yet"
                hint="Upload your first operational template (OPT/XML) with the Upload OPT button above."
            />
        }
        .into_any();
    }

    let matched = Memo::new(move |_| {
        let needle = filter.read().to_lowercase();
        rows.iter()
            .filter(|&row| matches_filter(row, &needle))
            .cloned()
            .collect::<Vec<_>>()
    });
    // `.read()` guards, never `.get()`: the filtered listing is a collection
    // and cloning it per read would be wasted work (rules §2).
    let total = Signal::derive(move || row_total(matched.read().len()));
    let none_match = move || {
        // Two statements on purpose: the filter guard must be released before
        // `matched` is read, because recomputing the memo reads the filter
        // again (rules §2 — never hold one signal's guard across another read
        // that depends on it).
        let filtering = !filter.read().is_empty();
        filtering && matched.read().is_empty()
    };

    let body = view! {
        <For
            each=move || {
                let window = page_window(total.get(), paging.page.get(), paging.size.get());
                matched.with(|rows| page_rows(rows, window))
            }
            key=|row| row.template_id.clone()
            children=move |row| row_view(row, admin, delete, pending_delete)
        />
    }
    .into_any();

    let headers: &[&str] = if admin {
        &["Template ID", "Concept", "Archetype ID", "Created", ""]
    } else {
        &["Template ID", "Concept", "Archetype ID", "Created"]
    };
    let footer = table_footer("/templates", "templates", paging, total);

    view! {
        {table_shell(headers, body)}
        {footer}
        <Show when=none_match>
            <div class="mt-3">
                <EmptyState
                    icon=icondata_lu::LuSearchX
                    message="No templates match the filter"
                    hint="Clear or shorten the filter to see the whole catalogue again."
                />
            </div>
        </Show>
    }
    .into_any()
}

/// Whether one row matches the screen's client-side filter: a case-insensitive
/// `contains` over the id, the concept, and the archetype id. `needle` is
/// already lowercased.
fn matches_filter(row: &TemplateRow, needle: &str) -> bool {
    row.template_id.to_lowercase().contains(needle)
        || row.concept.to_lowercase().contains(needle)
        || row.archetype_id.to_lowercase().contains(needle)
}

/// The `/templates/{template_id}` detail-route link for one template id.
///
/// The id is a CDR-supplied string, so the path segment is percent-encoded
/// with the `urlencoding` crate: reserved characters (`/`, `#`, `?`, `%`) and
/// non-ASCII bytes would otherwise split the segment or truncate the URL.
/// `leptos_router` percent-DEcodes route params on both targets
/// (`ParamsMap::insert` → `Url::unescape`), so `use_params_map` in
/// [`crate::pages::template_detail`] reads the original id back — the encode
/// here is the whole round trip, and no decode belongs on the read side.
/// NOTE: no openEHR spec governs an admin UI's internal links — our own
/// design/extension.
fn detail_href(template_id: &str) -> String {
    format!("/templates/{}", urlencoding::encode(template_id))
}

/// One table row: the template id links to the detail route; concept and
/// created are plain cells; the admin delete cell renders only when the probe
/// said so.
fn row_view(
    row: TemplateRow,
    admin: bool,
    delete: TemplateDeleteAction,
    pending_delete: RwSignal<Option<String>>,
) -> impl IntoView {
    let href = detail_href(&row.template_id);
    let action = if admin {
        delete_cell(&row.template_id, delete, pending_delete)
    } else {
        ().into_any()
    };
    view! {
        <tr class=ROW>
            <td class=CELL_MONO>
                <leptos_router::components::A href=href attr:class="text-accent hover:underline">
                    {row.template_id}
                </leptos_router::components::A>
            </td>
            <td class=CELL>{row.concept}</td>
            <td class=CELL_MONO>{row.archetype_id}</td>
            <td class=CELL_MONO>{row.created}</td>
            {action}
        </tr>
    }
}

/// The admin delete cell for one row: the danger button that opens the screen's
/// confirmation modal for THIS template (it only sets `pending_delete`; the
/// dialog owns the confirm). `data-template-delete` is the stable hook the E2E
/// journeys select on.
fn delete_cell(
    template_id: &str,
    delete: TemplateDeleteAction,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    let id_for_click = template_id.to_owned();
    let on_click = move |_| pending_delete.set(Some(id_for_click.clone()));
    view! {
        <td class=format!("{CELL} text-right")>
            <button
                type="button"
                class=BTN_DANGER
                data-template-delete=template_id.to_owned()
                disabled=Signal::derive(move || delete.pending().get())
                on:click=on_click
            >
                "Delete"
            </button>
        </td>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use crate::pages::templates::detail_href;

    #[test]
    fn detail_href_leaves_a_url_safe_template_id_alone() {
        assert_eq!(
            detail_href("Vital signs-v2.0_TEST~x"),
            "/templates/Vital%20signs-v2.0_TEST~x"
        );
    }

    #[test]
    fn detail_href_percent_encodes_reserved_and_non_ascii_bytes() {
        // A slash would otherwise split the segment and route elsewhere.
        assert_eq!(detail_href("a/b"), "/templates/a%2Fb");
        // A hash would otherwise truncate the URL into a fragment.
        assert_eq!(detail_href("a#b"), "/templates/a%23b");
        // A literal percent must survive as `%25`, not be read as an escape.
        assert_eq!(detail_href("a%2Fb"), "/templates/a%252Fb");
        // Non-ASCII is escaped per UTF-8 byte, uppercase hex.
        assert_eq!(
            detail_href("temperatur-°C"),
            "/templates/temperatur-%C2%B0C"
        );
        // Every reserved character at once.
        assert_eq!(
            detail_href("a b/c?d#e%f&g=h+i"),
            "/templates/a%20b%2Fc%3Fd%23e%25f%26g%3Dh%2Bi"
        );
    }
}
