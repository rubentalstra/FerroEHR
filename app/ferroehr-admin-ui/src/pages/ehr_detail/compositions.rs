// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The EHR-detail Compositions tab: the AQL-driven composition list plus the
//! "Commit composition" form.

#![expect(
    clippy::disallowed_types,
    reason = "the console consumes the CDR JSON wire over ITS-REST — not the CDR internal seams \
              (#1694)"
)]

use leptos::prelude::*;
use leptos::server;
use leptos_router::components::A;
use serde_json::Value;

#[cfg(feature = "ssr")]
use crate::pages::ehr_detail::commit_version_uid;

use crate::components::data_table::{CELL, CELL_MONO, ROW, table_shell, table_skeleton};
use crate::components::empty_state::EmptyState;
use crate::components::field::{BTN_PRIMARY, INPUT, LABEL, SELECT, TEXTAREA};
use crate::components::surface::{CARD_PAD, CARD_TITLE, WELL};
use crate::components::toast::toast_success;
use crate::error::AdminUiError;
use crate::format::ReprFormat;
use crate::pages::ehrs::{ResultPage, cell_text, paging_controls};
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

/// List an EHR's compositions via `LIST_COMPOSITIONS_AQL`, one page at
/// `offset`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] when the result set is not valid JSON.
#[server]
pub async fn list_compositions(
    /// The EHR whose compositions to list.
    ehr_id: String,
    /// First row of the page to return.
    offset: u32,
) -> Result<ResultPage, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
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
    /// The EHR to commit into.
    ehr_id: String,
    /// Which representation the body is written in.
    format: ReprFormat,
    /// The template the composition is built from (required by the simplified formats).
    template_id: String,
    /// The composition document to commit, as text.
    body: String,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
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

/// Compositions tab: `list_compositions` (AQL) → a paged table whose uid
/// cells link to the composition viewer (under `<Transition>` so paging keeps
/// old rows visible), plus a "Commit composition" form below it. A successful
/// commit bumps the commit action's version — a source of the list resource —
/// refetching the table (rules §6 — never fetch-in-effect).
pub(super) fn compositions_section(
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
    // Both outcomes toast (an outside-world side-effect — rules §2; the
    // console's mutation-feedback rule — crate CLAUDE.md); the CDR's
    // validation diagnostic ALSO stays inline in the form, where the pasted
    // body is.
    Effect::new(move |_| match commit.value().get() {
        Some(Ok(uid)) => {
            let detail = if uid.is_empty() {
                "The composition was committed.".to_owned()
            } else {
                format!("New version {uid}")
            };
            toast_success(toaster, "Composition committed", &detail);
        }
        Some(Err(error)) => crate::feedback::toast_write_failure(
            toaster,
            "Commit failed",
            "the composition",
            &error,
        ),
        None => {}
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
        return view! {
            <EmptyState
                icon=icondata_lu::LuFileText
                message="No compositions in this EHR"
                hint="Commit one with the form above, or through the CDR's REST API."
            />
        }
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

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{LIST_COMPOSITIONS_AQL, format_from_value, format_value, versioned_object_id};
    use crate::format::ReprFormat;

    #[test]
    fn fixed_aql_parses() {
        openehr_query::parser::parse_str(LIST_COMPOSITIONS_AQL)
            .expect("the compositions AQL const must parse");
    }

    #[test]
    fn versioned_object_id_strips_the_version_suffix() {
        assert_eq!(
            versioned_object_id("7d44aa01::example.ferroehr.eu::2"),
            "7d44aa01"
        );
        // A bare versioned-object id (no suffix) is returned unchanged.
        assert_eq!(versioned_object_id("7d44aa01"), "7d44aa01");
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
