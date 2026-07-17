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
use leptos_router::components::A;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use serde_json::Value;

use crate::components::format_view::{DocumentPane, FormatSelector};
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
    parse_versions(&response.body)
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
pub fn CompositionPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let ehr_id = Signal::derive(move || params.with(|p| p.get("ehr_id").unwrap_or_default()));
    let uid = Signal::derive(move || params.with(|p| p.get("uid").unwrap_or_default()));

    let format = RwSignal::new(ReprFormat::CanonicalJson);
    // Empty = "latest" (fetch by the bare versioned-object id); a non-empty
    // value is a specific OBJECT_VERSION_ID.
    let selected_version = RwSignal::new(String::new());

    let versions = Resource::new(
        move || (ehr_id.get(), uid.get()),
        |(ehr_id, uid)| async move { fetch_versions(ehr_id, uid).await },
    );
    let document = Resource::new(
        move || {
            let chosen = selected_version.get();
            let version_uid = if chosen.is_empty() { uid.get() } else { chosen };
            (ehr_id.get(), version_uid, format.get())
        },
        |(ehr_id, version_uid, format)| async move {
            fetch_composition(ehr_id, version_uid, format).await
        },
    );

    let toolbar = toolbar_section(format, versions, selected_version);
    let body = document_section(document);
    let audit = audit_section(versions, selected_version);

    let title = Signal::derive(move || {
        let short: String = uid.get().chars().take(8).collect();
        format!("Composition {short}…")
    });

    view! {
        <Title text="Composition · ehrbase-admin" />
        <div class="p-4">
            <div class="flex items-center gap-3 mb-4">
                <A
                    href=move || format!("/ehrs/{}", ehr_id.get())
                    attr:class="text-sm text-blue-600 hover:underline"
                >
                    "← EHR"
                </A>
                <h1 class="text-xl font-semibold font-mono">{move || title.get()}</h1>
            </div>
            {toolbar}
            {body}
            {audit}
        </div>
    }
}

/// The toolbar: the shared [`FormatSelector`] plus the version `<select>`
/// (populated from the revision history under `<Suspense>`).
fn toolbar_section(
    format: RwSignal<ReprFormat>,
    versions: Resource<Result<Vec<VersionEntry>, AdminUiError>>,
    selected_version: RwSignal<String>,
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
                        view! { <span class="text-xs text-red-600">"versions unavailable"</span> }
                            .into_any()
                    }
                }
            })}
        </Suspense>
    }
    .into_any();
    view! {
        <div class="mb-3 flex flex-wrap items-center gap-4">
            <FormatSelector offered=offered selected=format />
            <div class="flex items-center gap-2">
                <label class="text-sm font-medium" r#for="version-select">
                    "Version"
                </label>
                {select}
            </div>
        </div>
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
            class="rounded border border-neutral-300 dark:border-neutral-700 bg-transparent text-sm px-2 py-1"
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

/// Render the audit metadata for one version as a card, or a neutral note when
/// no matching version is found.
fn audit_card(entry: Option<&VersionEntry>) -> AnyView {
    let Some(entry) = entry else {
        return view! { <p class="text-sm text-neutral-500">"No audit for the selected version."</p> }
        .into_any();
    };
    let version_id = entry.version_id.clone();
    let committed_at = entry.committed.clone();
    let change_type = entry.change_type.clone();
    let committer = entry.committer.clone();
    view! {
        <thaw::Card>
            <div class="p-3 text-sm grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-1">
                {audit_row("version", version_id)} {audit_row("committed", committed_at)}
                {audit_row("change type", change_type)} {audit_row("committer", committer)}
            </div>
        </thaw::Card>
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
            <span class="font-medium text-neutral-500 mr-1">{label}":"</span>
            <span class="font-mono break-all">{shown}</span>
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

#[cfg(test)]
mod tests {
    use super::{parse_versions, short_version};

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
}
