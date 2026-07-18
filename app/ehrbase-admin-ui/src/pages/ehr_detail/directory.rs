//! The EHR-detail Directory tab: the EHR's root `FOLDER` tree, its JSON editor,
//! and the create-from-template path, plus the console-local folder templates.

use leptos::prelude::*;
use leptos::server;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[cfg(feature = "ssr")]
use crate::pages::ehr_detail::commit_version_uid;

use crate::components::field::{BTN_PRIMARY, BTN_SECONDARY, LABEL, SELECT, TEXTAREA};
use crate::components::surface::{CARD_PAD, CARD_TITLE, WELL};
use crate::components::toast::toast_success;
use crate::error::AdminUiError;
use crate::pages::ehrs::table_skeleton;

/// The EHR's directory as its canonical FOLDER JSON body plus the current
/// version uid (the FOLDER's `uid.value`, an `OBJECT_VERSION_ID`). The uid is
/// what the update path sends in `If-Match`: [`CdrResponse`] carries no header
/// map, so it is read from the returned FOLDER body — the FOLDER is a
/// `VERSIONABLE` and always carries `uid` (ITS-REST
/// `specifications/schemas/ehr/Folder.yaml`; RM common
/// `master05-directory_package`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryState {
    /// The canonical FOLDER JSON body.
    pub body: String,
    /// The FOLDER's `uid.value` (`OBJECT_VERSION_ID`), or empty when absent.
    pub version_uid: String,
}

/// The EHR's directory (root `FOLDER`) as a [`DirectoryState`], or `None` when
/// the CDR has no directory for this EHR (a `404` is a first-class empty
/// state, not an error).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx, non-404 CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_directory(ehr_id: String) -> Result<Option<DirectoryState>, AdminUiError> {
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
    let body = crate::cdr::CdrClient::expect_success(response)?.body;
    let version_uid = commit_version_uid(&body);
    Ok(Some(DirectoryState { body, version_uid }))
}

/// Create the EHR's directory (`POST /ehr/{ehr_id}/directory`) from a canonical
/// JSON FOLDER `body`. `Content-Type: application/json`,
/// `Prefer: return=representation`, `Accept: application/json`; the new
/// version uid (`uid.value` of the returned FOLDER) is returned (empty when the
/// CDR returned no representation). Directory operations per ITS-REST
/// `specifications/operations/directory_create.yaml`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] on an empty body; CDR transport errors pass
/// through; a non-2xx CDR answer (its validation diagnostics, which the UI
/// renders verbatim, included) normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn create_directory(ehr_id: String, body: String) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    if body.trim().is_empty() {
        return Err(AdminUiError::Invalid(
            "the directory body is empty".to_owned(),
        ));
    }
    let url = state
        .cdr
        .rest_v1(&format!("ehr/{}/directory", urlencoding::encode(&ehr_id)));
    let response = state
        .cdr
        .post(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[("Prefer", "return=representation")],
            body,
        )
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    Ok(commit_version_uid(&response.body))
}

/// Update the EHR's directory (`PUT /ehr/{ehr_id}/directory`) with a new
/// canonical JSON FOLDER `body`. The current `version_uid` (the
/// `preceding_version_uid`) is sent quoted in `If-Match`, per ITS-REST
/// `specifications/operations/directory_update.yaml`;
/// `Prefer: return=representation` yields the new version uid. A stale uid is
/// answered `412` by the CDR and surfaces as the normalized error.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] on an empty body or missing current version uid;
/// CDR transport errors pass through; a non-2xx CDR answer (its validation
/// diagnostics, which the UI renders verbatim, included) normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn update_directory(
    ehr_id: String,
    current_version_uid: String,
    body: String,
) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    if body.trim().is_empty() {
        return Err(AdminUiError::Invalid(
            "the directory body is empty".to_owned(),
        ));
    }
    let current = current_version_uid.trim();
    if current.is_empty() {
        return Err(AdminUiError::Invalid(
            "the current directory version uid is required to update".to_owned(),
        ));
    }
    let if_match = format!("\"{current}\"");
    let url = state
        .cdr
        .rest_v1(&format!("ehr/{}/directory", urlencoding::encode(&ehr_id)));
    let response = state
        .cdr
        .put(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[
                ("If-Match", if_match.as_str()),
                ("Prefer", "return=representation"),
            ],
            body,
        )
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    Ok(commit_version_uid(&response.body))
}

/// List the console-local folder templates (built-in defaults when the store
/// file is absent — see [`crate::folder_templates`]).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Internal`] on an unreadable templates file.
#[server]
pub async fn list_folder_templates() -> Result<Vec<FolderTemplate>, AdminUiError> {
    crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let path = crate::folder_templates::templates_path(&state.config.groups_file());
    // File I/O off the async runtime (reliability rule: no sync I/O on it).
    tokio::task::spawn_blocking(move || crate::folder_templates::read_templates(&path))
        .await
        .map_err(|e| AdminUiError::Internal(format!("folder-templates task: {e}")))?
}

/// Save a console-local folder template `name` from a canonical FOLDER JSON
/// `folder_body` (e.g. the EHR's current directory). Upserts by name.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] on an empty name or non-JSON body;
/// [`AdminUiError::Internal`] on an unwritable templates file.
#[server]
pub async fn save_folder_template(name: String, folder_body: String) -> Result<(), AdminUiError> {
    crate::session::require_session().await?;
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(AdminUiError::Invalid(
            "the folder template needs a name".to_owned(),
        ));
    }
    let folder: Value = serde_json::from_str(&folder_body).map_err(|e| {
        AdminUiError::Invalid(format!("the folder template is not valid JSON: {e}"))
    })?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let path = crate::folder_templates::templates_path(&state.config.groups_file());
    tokio::task::spawn_blocking(move || {
        crate::folder_templates::write_template(&path, &name, folder)
    })
    .await
    .map_err(|e| AdminUiError::Internal(format!("folder-templates task: {e}")))?
}

/// One named console-local folder template: a display name (the key) plus the
/// canonical FOLDER JSON tree it commits. No openEHR spec governs an admin-UI
/// convenience like this — our own design/extension; the FOLDER shape it
/// carries IS spec-bound (ITS-REST `specifications/schemas/ehr/Folder.yaml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FolderTemplate {
    /// Display name (also the template key).
    pub name: String,
    /// The canonical FOLDER JSON tree (root FOLDER, no `uid`).
    pub folder: Value,
}

/// The standard openEHR directory root archetype id (the FOLDER example in
/// ITS-REST `specifications/schemas/ehr/Folder.yaml`).
pub(crate) const DIRECTORY_ARCHETYPE: &str = "openEHR-EHR-FOLDER.directory.v1";

/// Node id reused by child folders; siblings differ by `name` (RM common
/// `master05-directory_package` §Paths — uniqueness modifiers). Consumed
/// only by the ssr-side folder-template builder, so it is feature-gated
/// (the wasm lib would flag it dead).
#[cfg(feature = "ssr")]
pub(crate) const FOLDER_NODE_ID: &str = "at0001";

/// Build a canonical FOLDER JSON node from `archetype_node_id`, `name`, and
/// child `folders`. `_type` + mandatory `archetype_node_id`/`name` (from
/// LOCATABLE) + the FOLDER `folders`/`items` arrays per
/// `specifications/schemas/ehr/Folder.yaml`; no `uid` — the CDR assigns the
/// `OBJECT_VERSION_ID` on create.
pub(crate) fn folder_json(archetype_node_id: &str, name: &str, folders: &[Value]) -> Value {
    json!({
        "_type": "FOLDER",
        "archetype_node_id": archetype_node_id,
        "name": { "_type": "DV_TEXT", "value": name },
        "folders": folders,
        "items": [],
    })
}

/// An empty directory root FOLDER (the "empty root" create choice).
fn empty_root_folder() -> Value {
    folder_json(DIRECTORY_ARCHETYPE, "root", &[])
}

/// The toast detail for a directory write: the new version, or a generic line
/// when the CDR returned no representation body.
fn directory_toast_detail(uid: &str) -> String {
    if uid.is_empty() {
        "The directory was committed.".to_owned()
    } else {
        format!("New version {uid}")
    }
}

/// Directory tab: `fetch_directory` → the recursive `FOLDER` tree with an
/// "Edit as JSON" editor committing a `PUT` update, or — when the EHR has no
/// directory (the CDR 404s) — a "Create directory" section that commits a
/// `POST` from a folder template or an empty root. The list resource depends
/// on both write actions' versions, so a successful commit refetches it (rules
/// §6 — never fetch-in-effect).
pub(super) fn directory_section(ehr_id: Signal<String>, selected: Memo<String>) -> AnyView {
    let toaster = thaw::ToasterInjection::expect_context();
    let create = Action::new(|(ehr_id, body): &(String, String)| {
        let ehr_id = ehr_id.clone();
        let body = body.clone();
        async move { create_directory(ehr_id, body).await }
    });
    let update = Action::new(|(ehr_id, uid, body): &(String, String, String)| {
        let ehr_id = ehr_id.clone();
        let uid = uid.clone();
        let body = body.clone();
        async move { update_directory(ehr_id, uid, body).await }
    });
    // Toast either write's success (an outside-world side-effect — rules §2);
    // the failure diagnostic stays inline in the WELL feedback pane.
    Effect::new(move |_| {
        if let Some(Ok(uid)) = create.value().get() {
            toast_success(toaster, "Directory created", &directory_toast_detail(&uid));
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(uid)) = update.value().get() {
            toast_success(toaster, "Directory updated", &directory_toast_detail(&uid));
        }
    });
    // Both resources are created ONCE here (never inside the Suspend — rules
    // §4); the templates load feeds only the create section.
    let directory = Resource::new(
        move || {
            let versions = (create.version().get(), update.version().get());
            (selected.get() == "directory").then(|| (ehr_id.get(), versions))
        },
        |active| async move {
            match active {
                Some((id, _)) => fetch_directory(id).await,
                None => Ok(None),
            }
        },
    );
    let templates = Resource::new(
        move || (selected.get() == "directory").then_some(()),
        |active| async move {
            match active {
                Some(()) => list_folder_templates().await.map(Some),
                None => Ok(None),
            }
        },
    );
    view! {
        <Suspense fallback=table_skeleton>
            {move || Suspend::new(async move {
                match directory.await {
                    Ok(Some(state)) => directory_edit_section(&state, ehr_id, update),
                    Ok(None) => {
                        match templates.await {
                            Ok(list) => {
                                directory_create_section(list.unwrap_or_default(), ehr_id, create)
                            }
                            Err(e) => crate::components::format_view::inline_error(&e),
                        }
                    }
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// The existing-directory view: the recursive `FOLDER` tree plus an
/// "Edit as JSON" toggle whose textarea (prefilled with the current canonical
/// FOLDER JSON) commits a `PUT` update via the shared `update` action (mirrors
/// the composition commit editor). The editor block stays mounted and is
/// toggled with `class:hidden` (rules §8 — identical server/client structure).
fn directory_edit_section(
    state: &DirectoryState,
    ehr_id: Signal<String>,
    update: Action<(String, String, String), Result<String, AdminUiError>>,
) -> AnyView {
    let doc: Value = match serde_json::from_str(&state.body) {
        Ok(value) => value,
        Err(e) => {
            return crate::components::format_view::inline_error(&AdminUiError::Internal(format!(
                "directory JSON: {e}"
            )));
        }
    };
    let tree = folder_node(&doc);
    let version_uid = state.version_uid.clone();
    let pretty = crate::components::format_view::pretty_body(
        &state.body,
        crate::format::ReprFormat::CanonicalJson,
    );
    let editing = RwSignal::new(false);
    let body = RwSignal::new(pretty);
    let on_commit = move |_| {
        update.dispatch((ehr_id.get(), version_uid.clone(), body.get()));
    };
    view! {
        <section class=CARD_PAD>
            <div class="flex flex-wrap items-center justify-between gap-2">
                <h2 class=CARD_TITLE>"Directory"</h2>
                <button
                    id="directory-edit"
                    type="button"
                    class=BTN_SECONDARY
                    on:click=move |_| editing.update(|open| *open = !*open)
                >
                    {move || if editing.get() { "Hide editor" } else { "Edit as JSON" }}
                </button>
            </div>
            <ul class="text-sm text-ink">{tree}</ul>
            <div class="mt-3 flex flex-col gap-3" class:hidden=move || !editing.get()>
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="directory-body">
                        "Directory FOLDER (canonical JSON)"
                    </label>
                    <textarea
                        id="directory-body"
                        class=format!("{TEXTAREA} min-h-[16rem]")
                        prop:value=move || body.get()
                        on:input:target=move |ev| body.set(ev.target().value())
                    >
                        {body.get_untracked()}
                    </textarea>
                </div>
                <div class="flex items-center gap-3">
                    <button
                        id="directory-commit"
                        type="button"
                        class=BTN_PRIMARY
                        disabled=Signal::derive(move || update.pending().get())
                        on:click=on_commit
                    >
                        "Save directory"
                    </button>
                    <Show when=move || update.pending().get()>
                        <span class="text-sm text-ink-muted">"Saving…"</span>
                    </Show>
                </div>
                {directory_feedback(update)}
            </div>
        </section>
    }
    .into_any()
}

/// The no-directory view: a "Create directory" section choosing a folder
/// template (or an empty root), previewing the FOLDER tree to be committed,
/// and committing a `POST` create via the shared `create` action.
fn directory_create_section(
    templates: Vec<FolderTemplate>,
    ehr_id: Signal<String>,
    create: Action<(String, String), Result<String, AdminUiError>>,
) -> AnyView {
    // The empty string selects the empty root; any other value names a template.
    let choice = RwSignal::new(String::new());
    let templates_for_pick = templates.clone();
    let chosen = Signal::derive(move || {
        let key = choice.get();
        if key.is_empty() {
            empty_root_folder()
        } else {
            templates_for_pick
                .iter()
                .find(|t| t.name == key)
                .map_or_else(empty_root_folder, |t| t.folder.clone())
        }
    });
    let on_create = move |_| {
        let body = serde_json::to_string(&chosen.get()).unwrap_or_default();
        create.dispatch((ehr_id.get(), body));
    };
    let options = templates
        .into_iter()
        .map(|template| {
            let value = template.name.clone();
            let label = template.name;
            view! { <option value=value>{label}</option> }.into_any()
        })
        .collect::<Vec<_>>();
    view! {
        <section class=CARD_PAD>
            <h2 class=CARD_TITLE>"Create directory"</h2>
            <p class="mb-3 text-sm text-ink-muted">
                "This EHR has no directory yet. Start from a folder template or an empty root."
            </p>
            <div class="flex flex-col gap-3">
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="folder-template">
                        "Folder template"
                    </label>
                    <select
                        id="folder-template"
                        class=SELECT
                        prop:value=move || choice.get()
                        on:change=move |ev| choice.set(event_target_value(&ev))
                    >
                        <option value="">"Empty root"</option>
                        {options}
                    </select>
                </div>
                <div class="flex flex-col gap-1">
                    <span class=LABEL>"Preview"</span>
                    <div class=WELL>
                        <ul class="text-sm text-ink">{move || folder_node(&chosen.get())}</ul>
                    </div>
                </div>
                <div class="flex items-center gap-3">
                    <button
                        id="directory-create"
                        type="button"
                        class=BTN_PRIMARY
                        disabled=Signal::derive(move || create.pending().get())
                        on:click=on_create
                    >
                        "Create directory"
                    </button>
                    <Show when=move || create.pending().get()>
                        <span class="text-sm text-ink-muted">"Creating…"</span>
                    </Show>
                </div>
                {directory_feedback(create)}
            </div>
        </section>
    }
    .into_any()
}

/// A directory write action's failure pane: the CDR's validation diagnostics
/// verbatim in a scrollable WELL (a `<pre>`, mirroring the composition commit
/// feedback). Success is a toast (see [`directory_section`]).
fn directory_feedback<I: Send + Sync + 'static>(
    action: Action<I, Result<String, AdminUiError>>,
) -> AnyView {
    view! {
        {move || match action.value().get() {
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

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{
        DIRECTORY_ARCHETYPE, FOLDER_NODE_ID, directory_toast_detail, empty_root_folder, folder_json,
    };

    #[test]
    fn folder_json_builds_a_spec_valid_folder_node() {
        let leaf = folder_json(FOLDER_NODE_ID, "2026", &[]);
        let root = folder_json(DIRECTORY_ARCHETYPE, "episodes", &[leaf]);
        assert_eq!(root["_type"], "FOLDER");
        assert_eq!(root["archetype_node_id"], DIRECTORY_ARCHETYPE);
        assert_eq!(root["name"]["value"], "episodes");
        assert_eq!(root["name"]["_type"], "DV_TEXT");
        // Child folder present; no uid (the CDR assigns the OBJECT_VERSION_ID).
        assert_eq!(root["folders"][0]["name"]["value"], "2026");
        assert!(root.get("uid").is_none());
        assert!(root["items"].as_array().unwrap().is_empty());
    }

    #[test]
    fn empty_root_folder_is_an_empty_directory_root() {
        let root = empty_root_folder();
        assert_eq!(root["_type"], "FOLDER");
        assert_eq!(root["archetype_node_id"], DIRECTORY_ARCHETYPE);
        assert!(root["folders"].as_array().unwrap().is_empty());
    }

    #[test]
    fn directory_toast_detail_names_the_version_or_a_generic_line() {
        assert_eq!(
            directory_toast_detail("7d44::sys::1"),
            "New version 7d44::sys::1"
        );
        assert_eq!(directory_toast_detail(""), "The directory was committed.");
    }
}
