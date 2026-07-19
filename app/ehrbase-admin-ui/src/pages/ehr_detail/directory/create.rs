//! The empty-directory create flow: for an EHR the CDR has no directory for
//! (a `404`), choose a console-local folder template (or an empty root),
//! preview the FOLDER tree, and commit it with `POST /ehr/{ehr_id}/directory`
//! (ITS-REST `specifications/operations/directory_create.yaml`).

use leptos::prelude::*;

use crate::components::field::{BTN_PRIMARY, LABEL, SELECT};
use crate::components::surface::{CARD_PAD, CARD_TITLE, WELL};
use crate::error::AdminUiError;
use crate::pages::ehr_detail::directory::tree::read_only_tree;
use crate::pages::ehr_detail::directory::{FolderTemplate, empty_root_folder};

/// The create flow's long-lived state, created ONCE in
/// [`directory_section`](super::directory_section) — ABOVE the
/// `<Transition>`/`Suspend` (rules §4 disposal contract, see
/// [`super::tree::EditorState`]): the selected folder-template key. Held above
/// the Suspend so a re-run cannot dispose the signal out from under the mounted
/// `<select>`/preview closures ("access a reactive value … already disposed").
#[derive(Clone, Copy)]
pub(in crate::pages::ehr_detail::directory) struct CreateState {
    /// The empty string selects the empty root; any other value names a
    /// template.
    choice: RwSignal<String>,
}

impl CreateState {
    /// Create the create-flow's long-lived state.
    pub(in crate::pages::ehr_detail::directory) fn new() -> Self {
        Self {
            choice: RwSignal::new(String::new()),
        }
    }
}

impl Default for CreateState {
    fn default() -> Self {
        Self::new()
    }
}

/// The no-directory view: choose a folder template (or an empty root), preview
/// the FOLDER tree to be committed, and commit a `POST` create via the shared
/// `create` action. Its reactive state lives in the long-lived [`CreateState`]
/// (created above the Suspend — rules §4), so re-running this on a refetch
/// creates no signals of its own.
pub(in crate::pages::ehr_detail::directory) fn create_section(
    state: CreateState,
    templates: Vec<FolderTemplate>,
    ehr_id: Signal<String>,
    create: Action<(String, String), Result<String, AdminUiError>>,
) -> AnyView {
    // The empty string selects the empty root; any other value names a template.
    let choice = state.choice;
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
                "This EHR has no directory yet. Start from a folder template or an empty root, then create it and edit the tree."
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
                    <div class=WELL>{move || read_only_tree(&chosen.get())}</div>
                </div>
                <div class="flex items-center gap-3">
                    <button
                        id="directory-create"
                        type="button"
                        class=BTN_PRIMARY
                        disabled=Signal::derive(move || create.pending().get())
                        on:click=on_create
                    >
                        <leptos_icons::Icon icon=icondata_lu::LuPlus width="14" height="14" />
                        "Create directory"
                    </button>
                    <Show when=move || create.pending().get()>
                        <span class="text-sm text-ink-muted">"Creating…"</span>
                    </Show>
                </div>
                {create_feedback(create)}
            </div>
        </section>
    }
    .into_any()
}

/// The create action's failure pane: the CDR's diagnostics verbatim in a
/// scrollable WELL (`<pre>`). Success is a toast (see the section orchestrator).
fn create_feedback(create: Action<(String, String), Result<String, AdminUiError>>) -> AnyView {
    view! {
        {move || match create.value().get() {
            Some(Err(error)) => {
                view! {
                    <div class=WELL>
                        <pre class="max-h-[40vh] overflow-auto whitespace-pre-wrap font-mono text-xs text-danger">
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
