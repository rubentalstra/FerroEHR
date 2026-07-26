//! The `/ehrs/{ehr_id}/compositions/{uid}` screen — the composition viewer.
//!
//! A read-only view of one COMPOSITION: a format toggle (canonical JSON/XML
//! and the Simplified FLAT/STRUCTURED renderings — the CDR converts, the BFF
//! forwards and pretty-prints), a version selector fed by the versioned
//! object's revision history, and a per-version audit card. The document
//! resource is keyed on `(version, format)` so either switch refetches, under
//! a `<Transition>` (old document stays visible — rules §6).
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The wire it reads IS spec-bound: `Accept` negotiation follows the ITS-REST
//! Simplified Formats spec
//! (`docs/specs/openehr/ITS-REST/docs/simplified_formats/`); a `406` (a
//! representation the CDR declines) surfaces the CDR diagnostic through the
//! normal error path. Path segments are percent-encoded server-side.
//!
//! Every co-located `#[server]` fn guards with
//! [`require_session`](crate::session::require_session) first (rules §0), and
//! the CDR credential never reaches client-visible state.

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use serde_json::Value;

use crate::components::empty_state::EmptyState;
use crate::components::field::{BTN_PRIMARY, BTN_SECONDARY, INPUT, LABEL, SELECT, TEXTAREA};
use crate::components::format_view::{DocumentPane, FormatSelector};
use crate::components::page_header::{Crumb, PageHeader};
use crate::components::surface::{CARD_PAD, CARD_TITLE, WELL};
use crate::components::toast::toast_success;
// Server-side pretty-printing happens in the #[server] body only.
#[cfg(feature = "ssr")]
use crate::components::format_view::pretty_body;
use crate::error::AdminUiError;
use crate::format::ReprFormat;

/// One entry in a versioned composition's history, flattened for the version
/// selector and audit card. All fields fixed-size-safe strings (rules §1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionEntry {
    /// The `OBJECT_VERSION_ID` value (`uuid::system::version`).
    pub version_id: String,
    /// `AUDIT_DETAILS.time_committed` value.
    pub committed: String,
    /// `AUDIT_DETAILS.change_type` value (the `DV_CODED_TEXT` label).
    pub change_type: String,
    /// `AUDIT_DETAILS.committer` name.
    pub committer: String,
}

/// The revision history of a versioned composition, newest-first.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] when the history is not valid JSON.
#[server]
pub async fn fetch_versions(
    ehr_id: String,
    uid: String,
) -> Result<Vec<VersionEntry>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1(&format!(
        "ehr/{}/versioned_composition/{}/revision_history",
        urlencoding::encode(&ehr_id),
        urlencoding::encode(&uid)
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    // NOTE: REVISION_HISTORY.items arrives most-recent-LAST on the wire (RM
    // common, generic package — the `items` attribute documentation); the
    // selector presents newest-first, so reverse here.
    parse_versions(&response.body).map(|mut entries| {
        entries.reverse();
        entries
    })
}

/// One representation of a composition version, pretty-printed for display.
/// `version_uid` may be the bare versioned-object id (the latest version) or a
/// full `OBJECT_VERSION_ID` (that exact version).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer (a `406` for a declined
/// representation included) normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_composition(
    ehr_id: String,
    version_uid: String,
    format: ReprFormat,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1(&format!(
        "ehr/{}/composition/{}",
        urlencoding::encode(&ehr_id),
        urlencoding::encode(&version_uid)
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, format.media_type())
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    Ok(pretty_body(&response.body, format))
}

/// Resolve the `OBJECT_VERSION_ID` of the VERSION of a VERSIONED_COMPOSITION
/// that was extant at `at_time` (a browser `datetime-local` value):
/// `GET /ehr/{ehr}/versioned_composition/{uid}/version?version_at_time=…`
/// (ITS-REST VERSIONED_COMPOSITION API `versioned_composition_version_get_at_time`
/// — "if `version_at_time` is supplied, retrieves the VERSION extant at
/// specified time"). The 200 body is a VERSION envelope whose `uid.value` is
/// the `OBJECT_VERSION_ID` (RM common — a VERSION's `uid` is an
/// `OBJECT_VERSION_ID`); that string is returned so the caller can select it.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] when `at_time` is empty; CDR transport errors pass
/// through; a non-2xx CDR answer (a `404` for no version at that time included,
/// which the UI renders as an inline note) normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_version_at_time(
    ehr_id: String,
    versioned_object_uid: String,
    at_time: String,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let at_time = datetime_local_to_rfc3339(&at_time);
    if at_time.is_empty() {
        return Err(AdminUiError::Invalid(
            "pick a date and time to travel to".to_owned(),
        ));
    }
    let url = state.cdr.rest_v1(&format!(
        "ehr/{}/versioned_composition/{}/version?version_at_time={}",
        urlencoding::encode(&ehr_id),
        urlencoding::encode(&versioned_object_uid),
        urlencoding::encode(&at_time),
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    // The VERSION envelope's `uid.value` is the `OBJECT_VERSION_ID`; the update
    // response reader follows the identical path, so reuse it.
    Ok(new_version_uid(&response.body))
}

#[cfg(feature = "ssr")]
/// Complete a browser `datetime-local` value (`YYYY-MM-DDTHH:MM`, optionally
/// with seconds) into an RFC 3339 / extended-ISO-8601 UTC instant for the
/// ITS-REST `version_at_time` query parameter ("a given time in the extended
/// ISO 8601 format"). A `datetime-local` control emits no seconds and no zone,
/// so absent seconds default to `:00` and the zone to `Z`; an already-zoned
/// value is returned unchanged. Empty input yields an empty string (the caller
/// rejects it). Interpreting the wall-clock value as UTC is a console
/// convenience — no openEHR spec governs the admin UI.
pub(crate) fn datetime_local_to_rfc3339(local: &str) -> String {
    let trimmed = local.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.ends_with('Z') || trimmed.ends_with('z') {
        return trimmed.to_owned();
    }
    // `HH:MM` carries one colon, `HH:MM:SS` two — add seconds when absent.
    let with_seconds = if trimmed.matches(':').count() < 2 {
        format!("{trimmed}:00")
    } else {
        trimmed.to_owned()
    };
    format!("{with_seconds}Z")
}

#[cfg(feature = "ssr")]
/// The new version uid of a just-updated COMPOSITION: `uid.value` from the
/// `Prefer: return=representation` body (the new `OBJECT_VERSION_ID`). Empty
/// when the CDR returned no representation body.
pub(crate) fn new_version_uid(body: &str) -> String {
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

/// Commit a new version of a COMPOSITION
/// (`PUT /ehr/{ehr_id}/composition/{versioned_object_uid}`). `If-Match` carries
/// the CURRENT (latest) `version_uid` — the `preceding_version_uid` — so the
/// update is conditional (ITS-REST COMPOSITION API `composition_update`; the
/// header value is the `version_uid` enclosed in double quotes). Canonical
/// JSON body; `Prefer: return=representation` yields the new `uid.value`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] on an empty body, a missing current version, or a
/// `412` mid-air collision (prefixed with a reload hint, the CDR diagnostic
/// appended); CDR transport errors pass through; any other non-2xx CDR answer
/// (its validation diagnostics included) normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn update_composition(
    ehr_id: String,
    versioned_object_uid: String,
    current_version_uid: String,
    body: String,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    if body.trim().is_empty() {
        return Err(AdminUiError::Invalid(
            "the composition body is empty".to_owned(),
        ));
    }
    let current_version_uid = current_version_uid.trim();
    if current_version_uid.is_empty() {
        return Err(AdminUiError::Invalid(
            "no current version is known — reload the page and retry".to_owned(),
        ));
    }
    // If-Match value is the version_uid in double quotes (composition_update).
    let if_match = format!("\"{current_version_uid}\"");
    let url = state.cdr.rest_v1(&format!(
        "ehr/{}/composition/{}",
        urlencoding::encode(&ehr_id),
        urlencoding::encode(&versioned_object_uid)
    ));
    let response = state
        .cdr
        .put(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[("Prefer", "return=representation"), ("If-Match", &if_match)],
            body,
        )
        .await?;
    // 412 Precondition Failed = a mid-air collision (the latest version moved
    // since this page loaded); give it a friendly prefix, the CDR diagnostic
    // appended verbatim.
    if response.status == 412 {
        let detail = crate::cdr::CdrClient::expect_success(response)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        return Err(AdminUiError::Invalid(format!(
            "the composition changed since this page loaded — reload and retry. {detail}"
        )));
    }
    let response = crate::cdr::CdrClient::expect_success(response)?;
    Ok(new_version_uid(&response.body))
}

#[cfg(feature = "ssr")]
/// Parse a `REVISION_HISTORY` body (either the canonical `{ "items": [...] }`
/// wrapper or a bare array) into version entries. Defensive throughout — a
/// missing field reads as empty rather than failing.
///
/// # Errors
/// [`AdminUiError::Internal`] when the body is not valid JSON.
pub(crate) fn parse_versions(body: &str) -> Result<Vec<VersionEntry>, AdminUiError> {
    let doc: Value = serde_json::from_str(body)
        .map_err(|e| AdminUiError::Internal(format!("revision history JSON: {e}")))?;
    let items = doc
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| doc.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(items.iter().map(version_entry).collect())
}

#[cfg(feature = "ssr")]
/// Flatten one `REVISION_HISTORY_ITEM` (`version_id` + `audits`) into a
/// [`VersionEntry`], reading the first audit for the commit metadata.
fn version_entry(item: &Value) -> VersionEntry {
    let version_id = json_str(item, &["version_id", "value"]);
    let audit = item
        .get("audits")
        .and_then(Value::as_array)
        .and_then(|audits| audits.first());
    VersionEntry {
        version_id,
        committed: audit
            .map(|a| json_str(a, &["time_committed", "value"]))
            .unwrap_or_default(),
        change_type: audit
            .map(|a| json_str(a, &["change_type", "value"]))
            .unwrap_or_default(),
        committer: audit
            .map(|a| json_str(a, &["committer", "name"]))
            .unwrap_or_default(),
    }
}

#[cfg(feature = "ssr")]
/// Follow a chain of object keys and return the terminal string value, or an
/// empty string if any hop is missing / not a string.
fn json_str(value: &Value, path: &[&str]) -> String {
    let mut current = value;
    for key in path {
        match current.get(*key) {
            Some(next) => current = next,
            None => return String::new(),
        }
    }
    current.as_str().unwrap_or_default().to_owned()
}

/// The composition viewer screen.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
#[allow(clippy::too_many_lines)] // resource/action setup plus the erased section locals — one screen, one function (rules §1)
pub fn CompositionPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let ehr_id = Signal::derive(move || params.with(|p| p.get("ehr_id").unwrap_or_default()));
    let uid = Signal::derive(move || params.with(|p| p.get("uid").unwrap_or_default()));

    let format = RwSignal::new(ReprFormat::CanonicalJson);
    // Empty = "latest" (fetch by the bare versioned-object id); a non-empty
    // value is a specific OBJECT_VERSION_ID.
    let selected_version = RwSignal::new(String::new());

    // The version_at_time picker: a `datetime-local` value resolves (server-side)
    // to the VERSION extant at that instant; on success its OBJECT_VERSION_ID
    // becomes the shared `selected_version` and the document pane refetches
    // through the existing resource keys. A `404` (no version at that time)
    // stays an inline note in the toolbar (resolved from the action value in the
    // view — rules §4), never an error bar.
    let at_time_input = RwSignal::new(String::new());
    let version_at_time = Action::new(
        |(ehr_id, versioned_object_uid, at_time): &(String, String, String)| {
            let ehr_id = ehr_id.clone();
            let versioned_object_uid = versioned_object_uid.clone();
            let at_time = at_time.clone();
            async move { fetch_version_at_time(ehr_id, versioned_object_uid, at_time).await }
        },
    );

    // The "Edit as new version" affordance state and its commit action.
    // Created before the resources so its `version()` can trigger their
    // refetch after a successful commit (rules §6 — never fetch-in-effect).
    let edit_open = RwSignal::new(false);
    let editor_body = RwSignal::new(String::new());
    let update = Action::new(
        |(ehr_id, versioned_object_uid, current_version_uid, body): &(
            String,
            String,
            String,
            String,
        )| {
            let ehr_id = ehr_id.clone();
            let versioned_object_uid = versioned_object_uid.clone();
            let current_version_uid = current_version_uid.clone();
            let body = body.clone();
            async move {
                update_composition(ehr_id, versioned_object_uid, current_version_uid, body).await
            }
        },
    );

    let versions = Resource::new(
        move || (ehr_id.get(), uid.get(), update.version().get()),
        |(ehr_id, uid, _)| async move { fetch_versions(ehr_id, uid).await },
    );
    let document = Resource::new(
        move || {
            let chosen = selected_version.get();
            let version_uid = if chosen.is_empty() { uid.get() } else { chosen };
            (
                ehr_id.get(),
                version_uid,
                format.get(),
                update.version().get(),
            )
        },
        |(ehr_id, version_uid, format, _)| async move {
            fetch_composition(ehr_id, version_uid, format).await
        },
    );

    // Both outcomes toast (an outside-world side-effect — rules §2; the
    // console's mutation-feedback rule — crate CLAUDE.md); the resources
    // refetch via `update.version()` in their sources above, and the CDR's
    // diagnostic ALSO stays inline in the editor, next to the edited body.
    let toaster = thaw::ToasterInjection::expect_context();
    Effect::new(move |_| match update.value().get() {
        Some(Ok(uid)) => {
            let detail = if uid.is_empty() {
                "A new version was committed.".to_owned()
            } else {
                format!("New version {uid}")
            };
            toast_success(toaster, "New version committed", &detail);
        }
        Some(Err(error)) => crate::feedback::toast_write_failure(
            toaster,
            "Commit failed",
            "the new composition version",
            &error,
        ),
        None => {}
    });

    // Sync a successful at-time resolution into the shared selection. This is
    // the async-load-into-local-state case (rules §2 — the one-directional
    // pattern the AQL editor uses to seed from a loaded query): the Effect
    // reads only the action value and writes only `selected_version`, so there
    // is no reactive loop, and Effects never run on the server (no hydration
    // divergence). A failure leaves the selection untouched (the toolbar note
    // renders it).
    Effect::new(move |_| {
        if let Some(Ok(resolved)) = version_at_time.value().get() {
            selected_version.set(resolved);
        }
    });

    let toolbar = toolbar_section(
        format,
        versions,
        selected_version,
        ehr_id,
        uid,
        version_at_time,
        at_time_input,
    );
    let body = document_section(document);
    let edit = edit_section(
        ehr_id,
        uid,
        format,
        versions,
        document,
        edit_open,
        editor_body,
        update,
    );
    let timeline = timeline_section(versions, selected_version);
    let audit = audit_section(versions, selected_version);

    let title = Signal::derive(move || {
        let short: String = uid.get().chars().take(8).collect();
        format!("Composition {short}…")
    });

    // The parent-EHR crumb href/label reads the route param once at setup (a
    // composition route instance keeps its `ehr_id` for its lifetime).
    let ehr_id_value = ehr_id.get_untracked();
    let ehr_short: String = ehr_id_value.chars().take(8).collect();
    let crumbs = vec![
        Crumb::new("EHRs", "/ehrs"),
        Crumb::new(format!("EHR {ehr_short}…"), format!("/ehrs/{ehr_id_value}")),
    ];

    view! {
        <Title text="Composition · ehrbase-admin" />
        <div class="p-6">
            <PageHeader title=Signal::derive(move || title.get()) crumbs=crumbs mono=true />
            {toolbar}
            {body}
            {edit}
            {timeline}
            {audit}
        </div>
    }
}

/// The "Edit as new version" affordance: a toggle button opening a
/// prefilled-from-the-current-document editor that PUTs a new version. Editing
/// is offered only when the current format is canonical JSON (the other
/// formats show a switch hint). The `If-Match` always targets the NEWEST
/// version (a muted hint says so), never the version selected in the dropdown —
/// the update `commits on top of the latest version`. Structure is constant
/// (visibility toggled with `class:hidden`) so server and client views match
/// (rules §8).
#[allow(clippy::too_many_arguments)] // the section wires several page-level signals + two resources
fn edit_section(
    ehr_id: Signal<String>,
    uid: Signal<String>,
    format: RwSignal<ReprFormat>,
    versions: Resource<Result<Vec<VersionEntry>, AdminUiError>>,
    document: Resource<Result<String, AdminUiError>>,
    edit_open: RwSignal<bool>,
    editor_body: RwSignal<String>,
    update: Action<(String, String, String, String), Result<String, AdminUiError>>,
) -> AnyView {
    let is_json = move || format.get() == ReprFormat::CanonicalJson;
    // Opening the editor prefills the textarea from the currently displayed
    // document (canonical JSON only). Reading a resource in an event handler
    // is untracked — it takes the value already loaded for the pane.
    let toggle = move |_| {
        let opening = !edit_open.get();
        if opening
            && format.get() == ReprFormat::CanonicalJson
            && let Some(Ok(current)) = document.get()
        {
            editor_body.set(current);
        }
        edit_open.set(opening);
    };
    let on_commit = move |_| {
        // If-Match the NEWEST version (entries are newest-first), regardless of
        // which version the dropdown shows.
        let current = versions
            .get()
            .and_then(Result::ok)
            .and_then(|entries| entries.first().map(|entry| entry.version_id.clone()))
            .unwrap_or_default();
        update.dispatch((ehr_id.get(), uid.get(), current, editor_body.get()));
    };
    view! {
        <div class="mt-3">
            <button id="edit-new-version" type="button" class=BTN_SECONDARY on:click=toggle>
                {move || if edit_open.get() { "Close editor" } else { "Edit as new version" }}
            </button>
            <div class:hidden=move || !edit_open.get()>
                <section class=format!("{CARD_PAD} mt-3")>
                    <h2 class=CARD_TITLE>"Commit new version"</h2>
                    <p class="mb-2 text-xs text-ink-muted">
                        "Commits on top of the latest version."
                    </p>
                    <div class:hidden=move || is_json()>
                        <p class="text-sm text-ink-muted">"Switch to canonical JSON to edit."</p>
                    </div>
                    <div class:hidden=move || !is_json() class="flex flex-col gap-3">
                        <textarea
                            id="edit-body"
                            class=format!("{TEXTAREA} min-h-[16rem]")
                            placeholder="edit the composition document…"
                            prop:value=move || editor_body.get()
                            on:input:target=move |ev| editor_body.set(ev.target().value())
                        >
                            {editor_body.get_untracked()}
                        </textarea>
                        <div class="flex items-center gap-3">
                            <button
                                id="edit-commit"
                                type="button"
                                class=BTN_PRIMARY
                                disabled=Signal::derive(move || update.pending().get())
                                on:click=on_commit
                            >
                                "Commit new version"
                            </button>
                            <Show when=move || update.pending().get()>
                                <span class="text-sm text-ink-muted">"Committing…"</span>
                            </Show>
                        </div>
                        {move || match update.value().get() {
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
                    </div>
                </section>
            </div>
        </div>
    }
    .into_any()
}

/// The toolbar: the shared [`FormatSelector`], the version `<select>`
/// (populated from the revision history under `<Suspense>`), and the
/// `version_at_time` picker (a `datetime-local` input + an "At time" button that
/// resolves the version extant at that instant into the shared selection).
fn toolbar_section(
    format: RwSignal<ReprFormat>,
    versions: Resource<Result<Vec<VersionEntry>, AdminUiError>>,
    selected_version: RwSignal<String>,
    ehr_id: Signal<String>,
    uid: Signal<String>,
    version_at_time: Action<(String, String, String), Result<String, AdminUiError>>,
    at_time_input: RwSignal<String>,
) -> AnyView {
    let offered = vec![
        ReprFormat::CanonicalJson,
        ReprFormat::CanonicalXml,
        ReprFormat::Flat,
        ReprFormat::Structured,
    ];
    let select = view! {
        <Suspense fallback=|| {
            view! { <thaw::Spinner size=thaw::SpinnerSize::Tiny /> }
        }>
            {move || Suspend::new(async move {
                match versions.await {
                    Ok(entries) => version_select(entries, selected_version),
                    Err(_) => {
                        // Resolve inside the Suspense: an SSR'd ErrorBoundary fallback
                        // mismatches at hydration in leptos 0.8 (E2E console gate).
                        view! { <span class="text-xs text-danger">"versions unavailable"</span> }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
    .into_any();
    // Dispatch the at-time resolution (server-fn does the RFC 3339 completion);
    // skip an empty input so no needless round-trip is made.
    let on_at_time = move |_| {
        let at_time = at_time_input.get();
        if !at_time.trim().is_empty() {
            version_at_time.dispatch((ehr_id.get(), uid.get(), at_time));
        }
    };
    // A 404 (no version at that time) is a neutral note; any other failure
    // renders through the normal inline-error path. Deliberately NOT an
    // EmptyState: this is the answer to the time-travel control standing right
    // beside it, not a data region that came back empty — nothing was replaced
    // by a void, and the kit's dashed box would read as a broken panel wedged
    // into a toolbar row.
    let note = move || match version_at_time.value().get() {
        Some(Err(AdminUiError::Cdr { status: 404, .. })) => view! {
            <p class="mt-2 text-sm text-ink-muted">
                "No version of this composition existed at that time."
            </p>
        }
        .into_any(),
        Some(Err(error)) => crate::components::format_view::inline_error(&error),
        _ => ().into_any(),
    };
    view! {
        <section class=format!("{CARD_PAD} mb-3")>
            <div class="flex flex-wrap items-end gap-4">
                <FormatSelector offered=offered selected=format />
                <div class="flex items-center gap-2">
                    <label class=LABEL r#for="version-select">
                        "Version"
                    </label>
                    {select}
                </div>
                <div class="flex items-end gap-2">
                    <div class="flex flex-col gap-1">
                        <label class=LABEL r#for="version-at-time">
                            "Time travel"
                        </label>
                        <input
                            id="version-at-time"
                            type="datetime-local"
                            class=INPUT
                            prop:value=move || at_time_input.get()
                            on:input:target=move |ev| at_time_input.set(ev.target().value())
                        />
                    </div>
                    <button
                        id="version-at-time-go"
                        type="button"
                        class=BTN_SECONDARY
                        disabled=Signal::derive(move || version_at_time.pending().get())
                        on:click=on_at_time
                    >
                        "At time"
                    </button>
                </div>
            </div>
            {note}
        </section>
    }
    .into_any()
}

/// The version `<select>`: a "Latest" option (empty value) plus one option per
/// version. Driven by `prop:value` + `on:change` (rules §5 — no JS).
fn version_select(entries: Vec<VersionEntry>, selected: RwSignal<String>) -> AnyView {
    let mut options = vec![view! { <option value="">"Latest"</option> }.into_any()];
    options.extend(entries.into_iter().map(|entry| {
        let value = entry.version_id.clone();
        let label = format!("{} — {}", short_version(&entry.version_id), entry.committed);
        view! { <option value=value>{label}</option> }.into_any()
    }));
    view! {
        <select
            id="version-select"
            class=SELECT
            prop:value=move || selected.get()
            on:change=move |ev| selected.set(event_target_value(&ev))
        >
            {options}
        </select>
    }
    .into_any()
}

/// The document pane: the pretty-printed representation for the current
/// `(version, format)` selection, under a `<Transition>` so switching either
/// keeps the prior document visible. A `406` (declined representation) or any
/// other CDR error renders through the boundary.
fn document_section(document: Resource<Result<String, AdminUiError>>) -> AnyView {
    view! {
        <Transition fallback=|| {
            view! {
                <thaw::Skeleton>
                    <thaw::SkeletonItem class="h-64" />
                </thaw::Skeleton>
            }
        }>
            {move || Suspend::new(async move {
                match document.await {
                    Ok(body) => {
                        let body_sig = RwSignal::new(body);
                        // Resolve inside the Transition: an SSR'd ErrorBoundary fallback
                        // mismatches at hydration in leptos 0.8 (E2E console gate).
                        view! { <DocumentPane body=body_sig /> }
                            .into_any()
                    }
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// The version timeline strip above the audit card: one chip per version,
/// oldest→newest left-to-right, the selected chip accented and the newest
/// tagged "current". Clicking a chip sets the shared selection (the newest →
/// empty string = Latest, matching the dropdown). Resolved inside the existing
/// suspense pattern (rules §4 — no resource is created here).
fn timeline_section(
    versions: Resource<Result<Vec<VersionEntry>, AdminUiError>>,
    selected: RwSignal<String>,
) -> AnyView {
    view! {
        <div class="mt-3">
            <Suspense fallback=|| {
                ().into_any()
            }>
                {move || Suspend::new(async move {
                    match versions.await {
                        Ok(entries) => {
                            let stored = StoredValue::new(entries);
                            // Resolve inside the Suspense: an SSR'd ErrorBoundary fallback
                            // mismatches at hydration in leptos 0.8 (E2E console gate). A
                            // failed history renders nothing here (the document/toolbar
                            // sections surface the error).
                            view! {
                                {move || {
                                    stored.with_value(|entries| timeline_strip(entries, selected))
                                }}
                            }
                                .into_any()
                        }
                        Err(_) => ().into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
    .into_any()
}

/// Render the timeline chips. `entries` is newest-first (the revision-history
/// order the selector uses), so the strip displays it reversed (oldest→newest).
fn timeline_strip(entries: &[VersionEntry], selected: RwSignal<String>) -> AnyView {
    if entries.is_empty() {
        return ().into_any();
    }
    let newest = entries
        .first()
        .map(|entry| entry.version_id.clone())
        .unwrap_or_default();
    let mut ordered: Vec<VersionEntry> = entries.to_vec();
    ordered.reverse();
    let chips = view! {
        <For each=move || ordered.clone() key=|entry| entry.version_id.clone() let:entry>
            {timeline_chip(&entry, &newest, selected)}
        </For>
    };
    view! {
        <section class=CARD_PAD>
            <h2 class=CARD_TITLE>"Version timeline"</h2>
            <div class="flex flex-wrap items-center gap-2">{chips}</div>
        </section>
    }
    .into_any()
}

/// One timeline chip: a `rounded-full border` button labelled with the short
/// `vN`, accented while it is the current selection, and suffixed "· current"
/// for the newest version. Clicking selects it — the newest selects Latest
/// (empty string), the others their `OBJECT_VERSION_ID`.
fn timeline_chip(entry: &VersionEntry, newest: &str, selected: RwSignal<String>) -> AnyView {
    let version_id = entry.version_id.clone();
    let is_newest = version_id == newest;
    let target = if is_newest {
        String::new()
    } else {
        version_id.clone()
    };
    let label = short_version(&version_id);
    let newest_owned = newest.to_owned();
    let version_for_class = version_id.clone();
    let class = move || {
        let current = selected.get();
        let is_selected = if is_newest {
            current.is_empty() || current == newest_owned
        } else {
            current == version_for_class
        };
        if is_selected {
            "rounded-full border border-accent bg-accent-subtle px-3 py-1 text-xs font-medium text-accent-ink"
        } else {
            "rounded-full border border-edge px-3 py-1 text-xs font-medium text-ink-muted hover:bg-sunken"
        }
    };
    view! {
        <button type="button" class=class on:click=move |_| selected.set(target.clone())>
            {label}
            {is_newest.then_some(" · current")}
        </button>
    }
    .into_any()
}

/// The audit card for the currently selected version (or the latest when
/// "Latest" is selected), derived reactively from the loaded history.
fn audit_section(
    versions: Resource<Result<Vec<VersionEntry>, AdminUiError>>,
    selected: RwSignal<String>,
) -> AnyView {
    view! {
        <div class="mt-3">
            <Suspense fallback=|| {
                ().into_any()
            }>
                {move || Suspend::new(async move {
                    match versions.await {
                        Ok(entries) => {
                            let stored = StoredValue::new(entries);
                            // Resolve inside the Suspense: an SSR'd ErrorBoundary fallback
                            // mismatches at hydration in leptos 0.8 (E2E console gate). A
                            // failed history renders nothing here (the document/toolbar
                            // sections surface the error).
                            view! {
                                {move || {
                                    let chosen = selected.get();
                                    stored
                                        .with_value(|entries| {
                                            let entry = if chosen.is_empty() {
                                                entries.first()
                                            } else {
                                                entries.iter().find(|e| e.version_id == chosen)
                                            };
                                            audit_card(entry)
                                        })
                                }}
                            }
                                .into_any()
                        }
                        Err(_) => ().into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
    .into_any()
}

/// Render the audit metadata for one version as a card, or the empty state when
/// no matching version is found.
fn audit_card(entry: Option<&VersionEntry>) -> AnyView {
    let Some(entry) = entry else {
        return view! {
            <EmptyState
                icon=icondata_lu::LuShieldCheck
                message="No audit for the selected version"
                hint="Pick another version above — every committed version carries its own audit."
            />
        }
        .into_any();
    };
    let version_id = entry.version_id.clone();
    let committed_at = entry.committed.clone();
    let change_type = entry.change_type.clone();
    let committer = entry.committer.clone();
    view! {
        <section class=CARD_PAD>
            <h2 class=CARD_TITLE>"Audit"</h2>
            <div class="grid grid-cols-1 sm:grid-cols-2 items-start gap-x-4 gap-y-2 text-sm">
                {audit_row("version", version_id)} {audit_row("committed", committed_at)}
                {audit_row("change type", change_type)} {audit_row("committer", committer)}
            </div>
        </section>
    }
    .into_any()
}

/// One label/value line in the audit card (empty values shown as an em dash).
fn audit_row(label: &'static str, value: String) -> AnyView {
    let shown = if value.is_empty() {
        "—".to_owned()
    } else {
        value
    };
    view! {
        <div>
            <span class="font-medium text-ink-muted mr-1">{label}":"</span>
            <span class="font-mono break-all text-ink">{shown}</span>
        </div>
    }
    .into_any()
}

/// The short `vN` label from an `OBJECT_VERSION_ID` (the segment after the
/// last `::`), or the whole id when there is no version segment.
fn short_version(version_id: &str) -> String {
    version_id
        .rsplit_once("::")
        .map_or_else(|| version_id.to_owned(), |(_, v)| format!("v{v}"))
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{datetime_local_to_rfc3339, new_version_uid, parse_versions, short_version};

    #[test]
    fn datetime_local_completes_to_rfc3339_utc() {
        // A `datetime-local` value with no seconds gains `:00` and a `Z` zone.
        assert_eq!(
            datetime_local_to_rfc3339("2026-07-12T10:30"),
            "2026-07-12T10:30:00Z"
        );
        // With seconds, only the zone is appended.
        assert_eq!(
            datetime_local_to_rfc3339("2026-07-12T10:30:45"),
            "2026-07-12T10:30:45Z"
        );
        // Surrounding whitespace is trimmed.
        assert_eq!(
            datetime_local_to_rfc3339("  2026-07-12T08:00  "),
            "2026-07-12T08:00:00Z"
        );
        // Empty stays empty (the server fn rejects it before the round-trip).
        assert_eq!(datetime_local_to_rfc3339(""), "");
        // An already-zoned value is returned unchanged (never double-stamped).
        assert_eq!(
            datetime_local_to_rfc3339("2026-07-12T10:30:00Z"),
            "2026-07-12T10:30:00Z"
        );
    }

    #[test]
    fn parses_revision_history_wrapper_and_bare_array() {
        let wrapper = r#"{
            "items": [{
                "version_id": {"value": "7d44::sys::2"},
                "audits": [{
                    "time_committed": {"value": "2026-07-12T10:00:00Z"},
                    "change_type": {"value": "creation"},
                    "committer": {"name": "Dr Bob"}
                }]
            }]
        }"#;
        let entries = parse_versions(wrapper).expect("valid history");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version_id, "7d44::sys::2");
        assert_eq!(entries[0].committed, "2026-07-12T10:00:00Z");
        assert_eq!(entries[0].change_type, "creation");
        assert_eq!(entries[0].committer, "Dr Bob");

        // A bare array (no wrapper) parses identically.
        let bare = r#"[{"version_id":{"value":"a::b::1"},"audits":[]}]"#;
        let entries = parse_versions(bare).expect("valid bare history");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version_id, "a::b::1");
        assert_eq!(entries[0].committed, "");
    }

    #[test]
    fn short_version_takes_the_trailing_segment() {
        assert_eq!(short_version("7d44::example.org::3"), "v3");
        assert_eq!(short_version("no-version"), "no-version");
    }

    #[test]
    fn new_version_uid_reads_uid_value_or_empty() {
        let body =
            r#"{"_type":"COMPOSITION","uid":{"_type":"OBJECT_VERSION_ID","value":"7d44::sys::2"}}"#;
        assert_eq!(new_version_uid(body), "7d44::sys::2");
        // A return=minimal (empty) or non-JSON body yields no uid.
        assert_eq!(new_version_uid(""), "");
        assert_eq!(new_version_uid("{}"), "");
    }
}
