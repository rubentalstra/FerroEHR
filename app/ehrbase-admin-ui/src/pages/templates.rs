//! The `/templates` screen — the Template Manager list + OPT upload.
//!
//! A list of the CDR's ADL 1.4 operational templates with a client-side text
//! filter, plus an OPT/XML upload that surfaces the CDR's validation
//! diagnostics verbatim. No openEHR spec governs an admin UI — our own design /
//! product extension; the wire it reads/writes is the ITS-REST Definition API.
//!
//! Discipline (rules §0/§1/§6/§8): the two `#[server]` fns guard the session
//! first (a server fn is a public HTTP endpoint) and never let a CDR credential
//! reach client-visible state; the view is composed from `.into_any()`-erased
//! section locals; async is a [`Resource`] read under `<Transition>` and an
//! [`Action`] for the mutating upload (refetch the list on the action's
//! version); the table is the shared [`table_shell`], which emits an explicit
//! `<tbody>` (hydration correctness — rules §8).

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;

use crate::components::data_table::{CELL, CELL_MONO, ROW, table_shell};
use crate::components::empty_state::EmptyState;
use crate::components::field::INPUT;
use crate::components::page_header::PageHeader;
use crate::components::toast::toast_success;
use crate::error::AdminUiError;

/// One row of the template list, distilled from the ITS-REST Definition list
/// shape (`template_id` / `concept` / `created_timestamp`). Shared across both
/// compilation targets, so it carries only fixed-size, client-safe fields
/// (rules §1 — no `usize` in serialized types).
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
/// GET `definition/template/adl1.4` with `Accept: application/json`; the body
/// is an array whose elements are parsed defensively (missing fields become
/// empty strings) into [`TemplateRow`]s.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Forbidden`] / [`AdminUiError::Cdr`] /
/// [`AdminUiError::CdrUnreachable`] from the CDR; [`AdminUiError::Internal`]
/// when the CDR body is not valid JSON.
#[server]
pub async fn list_templates() -> Result<Vec<TemplateRow>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1("definition/template/adl1.4");
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
pub async fn upload_template(opt_xml: String) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    if opt_xml.trim().is_empty() {
        return Err(AdminUiError::Invalid(
            "the selected file was empty".to_owned(),
        ));
    }
    let state: crate::state::AppState = leptos::prelude::expect_context();
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
/// would add nothing here). A successful upload bumps the upload action's
/// version, which is the list resource's source, refetching it.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn TemplatesPage() -> impl IntoView {
    let toaster = thaw::ToasterInjection::expect_context();
    let filter = RwSignal::new(String::new());
    let upload = Action::new(|opt_xml: &String| {
        let opt_xml = opt_xml.clone();
        async move { upload_template(opt_xml).await }
    });
    let list: Resource<Result<Vec<TemplateRow>, AdminUiError>> = Resource::new(
        move || upload.version().get(),
        |_| async move { list_templates().await },
    );

    // Success feedback is a transient toast (the CDR-diagnostic failure path
    // keeps its inline MessageBar). Dispatching a toast is a side-effect on the
    // outside world (the thaw toaster), so an Effect is its correct home
    // (rules §2); it never runs on the server pass.
    Effect::new(move |_| {
        if let Some(Ok(_)) = upload.value().get() {
            toast_success(
                toaster,
                "Template uploaded",
                "The operational template was accepted by the CDR.",
            );
        }
    });

    let action_slot = upload_trigger(upload);
    let feedback = upload_feedback(upload);
    let table_section = templates_table(filter, list);

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
        </div>
    }
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
            <thaw::Button appearance=thaw::ButtonAppearance::Primary>"Upload OPT ▲"</thaw::Button>
        </thaw::Upload>
    }
    .into_any()
}

/// The upload action's inline state: a pending hint plus the CDR diagnostic on
/// failure (verbatim — an error payload worth reading stays inline). Success is
/// reported by a toast (see [`TemplatesPage`]).
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
fn templates_table(
    filter: RwSignal<String>,
    list: Resource<Result<Vec<TemplateRow>, AdminUiError>>,
) -> AnyView {
    view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match list.await {
                    Ok(rows) => rows_view(rows, filter),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// The `<Transition>` fallback while the list loads.
fn table_skeleton() -> impl IntoView {
    view! {
        <thaw::Skeleton>
            <thaw::SkeletonItem class="h-6 mb-2" />
            <thaw::SkeletonItem class="h-6 mb-2" />
            <thaw::SkeletonItem class="h-6" />
        </thaw::Skeleton>
    }
}

/// Render the loaded rows: the empty-catalogue state, or a filterable table.
fn rows_view(rows: Vec<TemplateRow>, filter: RwSignal<String>) -> AnyView {
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

    // Shared, cheaply-cloned backing store for the reactive filter closures.
    let rows = std::sync::Arc::new(rows);
    let each_rows = std::sync::Arc::clone(&rows);
    let empty_rows = std::sync::Arc::clone(&rows);

    let matches = |row: &TemplateRow, needle: &str| {
        row.template_id.to_lowercase().contains(needle)
            || row.concept.to_lowercase().contains(needle)
            || row.archetype_id.to_lowercase().contains(needle)
    };

    let none_match = move || {
        let needle = filter.get().to_lowercase();
        !needle.is_empty() && !empty_rows.iter().any(|row| matches(row, &needle))
    };

    let body = view! {
        <For
            each=move || {
                let needle = filter.get().to_lowercase();
                each_rows.iter().filter(|&row| matches(row, &needle)).cloned().collect::<Vec<_>>()
            }
            key=|row| row.template_id.clone()
            children=row_view
        />
    }
    .into_any();

    view! {
        {table_shell(&["Template ID", "Concept", "Archetype ID", "Created"], body)}
        <Show when=none_match>
            <p class="mt-3 text-sm text-ink-muted">"No templates match the filter."</p>
        </Show>
    }
    .into_any()
}

/// One table row: the template id links to the detail route; concept and
/// created are plain cells.
fn row_view(row: TemplateRow) -> impl IntoView {
    // TODO: template ids in the corpus are URL-safe path segments; an id with
    // reserved characters (`/`, `#`, `?`) would need percent-escaping of this
    // segment on the client. Add it when the CDR corpus proves such an id.
    let href = format!("/templates/{}", row.template_id);
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
        </tr>
    }
}
