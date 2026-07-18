//! The structured directory tree editor: a recursive, editable view of the
//! root `FOLDER` (add / rename / delete folders, add / remove item
//! references) backed by a single working-tree signal, with a dirty-state
//! save bar (PUT with `If-Match`), an advanced raw-JSON mode, and the
//! composition picker for adding `OBJECT_REF` items. Also the shared
//! read-only tree renderer used by the create preview and the history / time /
//! path panels.
//!
//! The working tree is one `RwSignal<serde_json::Value>` seeded from the
//! loaded FOLDER; every mutation goes through the pure [`super::edit`]
//! helpers, and every rendered datum reads the tree reactively by node path,
//! so `<For>` rows (keyed by data-derived path keys — rules §4) always show
//! current content even as indices shift on delete.

use leptos::prelude::*;
use serde_json::Value;

use crate::components::field::{BTN_PRIMARY, BTN_SECONDARY, INPUT, TEXTAREA};
use crate::components::format_view::{inline_error, pretty_body};
use crate::components::surface::{CARD_PAD, CARD_TITLE, WELL};
use crate::error::AdminUiError;
use crate::format::ReprFormat;
use crate::pages::ehr_detail::directory::edit::{
    add_item, add_subfolder, child_keys, count_tree, delete_folder, item_keys, item_summary,
    key_of, node_name, object_ref, parse_item_key, parse_key, remove_item, rename_folder,
    versioned_object_id,
};
use crate::pages::ehr_detail::directory::{DirectoryState, PickerResource};
use crate::pages::ehrs::cell_text;

/// A small square icon button (node actions).
const ICON_BTN: &str = "inline-flex items-center justify-center rounded-control p-1 text-ink-muted hover:bg-sunken hover:text-ink focus:outline-none focus:ring-2 focus:ring-accent";

/// A small square destructive icon button (node delete).
const ICON_BTN_DANGER: &str = "inline-flex items-center justify-center rounded-control p-1 text-ink-muted hover:bg-danger-subtle hover:text-danger focus:outline-none focus:ring-2 focus:ring-danger";

/// The editor's shared, `Copy` handle threaded through the recursive render:
/// the working tree plus its UI state and the composition picker.
#[derive(Clone, Copy)]
struct TreeEditor {
    /// The working FOLDER tree (mutated in place; the single source of truth).
    tree: RwSignal<Value>,
    /// Path keys of collapsed folders (a folder is expanded unless listed).
    collapsed: RwSignal<std::collections::HashSet<String>>,
    /// The path key of the folder currently being renamed, if any.
    renaming: RwSignal<Option<String>>,
    /// The in-progress rename text.
    rename_draft: RwSignal<String>,
    /// The composition list for the "add item" picker (created outside the
    /// Suspend, read here — rules §4).
    picker: PickerResource,
    /// The path key of the folder awaiting an item (also opens the picker).
    picker_target: RwSignal<Option<String>>,
}

/// The existing-directory experience: the structured tree editor with a
/// dirty-state save bar (`PUT` with `If-Match`), an advanced raw-JSON mode,
/// and the composition picker. The working tree is seeded from `state`; a
/// successful save refetches the directory (via the shared `update` action's
/// version — rules §6), re-running this section with the fresh version.
#[allow(clippy::too_many_lines)] // the editor view + its working-tree state + save bar + advanced mode assembled as one unit
pub(in crate::pages::ehr_detail::directory) fn tree_editor(
    state: &DirectoryState,
    ehr_id: Signal<String>,
    update: Action<(String, String, String), Result<String, AdminUiError>>,
    picker: PickerResource,
    picker_target: RwSignal<Option<String>>,
) -> AnyView {
    let doc: Value = match serde_json::from_str(&state.body) {
        Ok(value) => value,
        Err(e) => {
            return inline_error(&AdminUiError::Internal(format!("directory JSON: {e}")));
        }
    };
    let version_uid = state.version_uid.clone();
    let pretty = pretty_body(&state.body, ReprFormat::CanonicalJson);

    let tree = RwSignal::new(doc.clone());
    let original = StoredValue::new(doc);
    let collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let renaming = RwSignal::new(Option::<String>::None);
    let rename_draft = RwSignal::new(String::new());
    let advanced = RwSignal::new(false);
    let json_draft = RwSignal::new(pretty);
    let json_error = RwSignal::new(Option::<String>::None);

    let dirty = Memo::new(move |_| tree.with(|t| original.with_value(|o| t != o)));

    let ed = TreeEditor {
        tree,
        collapsed,
        renaming,
        rename_draft,
        picker,
        picker_target,
    };

    // Save the working tree as a new version (PUT + If-Match).
    let on_save = move |_| {
        let body = serde_json::to_string(&tree.get()).unwrap_or_default();
        update.dispatch((ehr_id.get(), version_uid.clone(), body));
    };
    let on_discard = move |_| {
        tree.set(original.get_value());
        renaming.set(None);
        json_error.set(None);
    };

    // Advanced mode: toggling on seeds the JSON draft from the current tree;
    // "Apply" parses it back into the working tree.
    let on_toggle_advanced = move |_| {
        advanced.update(|open| {
            *open = !*open;
            if *open {
                let text = tree
                    .with(|t| serde_json::to_string_pretty(t).unwrap_or_else(|_| "{}".to_owned()));
                json_draft.set(text);
                json_error.set(None);
            }
        });
    };
    let on_apply_json = move |_| match serde_json::from_str::<Value>(&json_draft.get()) {
        Ok(value) => {
            tree.set(value);
            json_error.set(None);
        }
        Err(e) => json_error.set(Some(format!("Invalid JSON: {e}"))),
    };

    let tree_body = tree_view(ed);
    let picker_modal = picker_modal(ed);

    view! {
        <section class=CARD_PAD>
            <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="flex items-center gap-2">
                    <h2 class=CARD_TITLE>"Directory"</h2>
                    <span class="text-xs text-ink-muted">
                        {move || {
                            let (folders, items) = tree.with(count_tree);
                            format!("{folders} folders · {items} items")
                        }}
                    </span>
                </div>
                <button
                    id="directory-edit"
                    type="button"
                    class=BTN_SECONDARY
                    on:click=on_toggle_advanced
                >
                    <leptos_icons::Icon icon=icondata_lu::LuCode width="14" height="14" />
                    {move || {
                        if advanced.get() { "Hide JSON" } else { "Advanced: edit as JSON" }
                    }}
                </button>
            </div>

            <div class:hidden=move || advanced.get()>
                <ul class="mt-2 text-sm text-ink">{tree_body}</ul>
            </div>

            <div class="mt-2 flex flex-col gap-2" class:hidden=move || !advanced.get()>
                <textarea
                    id="directory-body"
                    class=format!("{TEXTAREA} min-h-[16rem]")
                    prop:value=move || json_draft.get()
                    on:input:target=move |ev| json_draft.set(ev.target().value())
                >
                    {json_draft.get_untracked()}
                </textarea>
                <div class="flex items-center gap-3">
                    <button type="button" class=BTN_SECONDARY on:click=on_apply_json>
                        <leptos_icons::Icon icon=icondata_lu::LuCheck width="14" height="14" />
                        "Apply to tree"
                    </button>
                    {move || {
                        json_error
                            .get()
                            .map(|msg| {
                                view! { <span class="text-sm text-danger">{msg}</span> }
                            })
                    }}
                </div>
            </div>

            // Sticky save bar — visible only when the working tree is dirty.
            <div
                class="sticky bottom-0 mt-3 flex flex-wrap items-center gap-3 border-t border-edge bg-raised/95 py-3 backdrop-blur"
                class:hidden=move || !dirty.get()
            >
                <button
                    id="directory-save"
                    type="button"
                    class=BTN_PRIMARY
                    disabled=Signal::derive(move || update.pending().get())
                    on:click=on_save
                >
                    <leptos_icons::Icon icon=icondata_lu::LuSave width="14" height="14" />
                    "Save directory"
                </button>
                <button type="button" class=BTN_SECONDARY on:click=on_discard>
                    "Discard changes"
                </button>
                <Show when=move || update.pending().get()>
                    <span class="text-sm text-ink-muted">"Saving…"</span>
                </Show>
            </div>

            {save_feedback(update)}
            {picker_modal}
        </section>
    }
    .into_any()
}

/// The root of the editable tree (the root folder rendered from path `[]`).
fn tree_view(ed: TreeEditor) -> AnyView {
    render_folder(ed, &[])
}

/// Render one FOLDER node (at `path`) with its actions, its child folders, and
/// its item references. Every dynamic datum reads the tree reactively by path,
/// so `<For>` rows always reflect the current tree (rules §4).
fn render_folder(ed: TreeEditor, path: &[usize]) -> AnyView {
    let key = key_of(path);
    let is_root = path.is_empty();

    let key_collapsed = key.clone();
    let is_collapsed = move || ed.collapsed.with(|c| c.contains(&key_collapsed));

    let path_icon = path.to_vec();
    let key_icon = key.clone();
    let folder_icon = Signal::derive(move || {
        let expanded = ed.collapsed.with(|c| !c.contains(&key_icon));
        let has_content = ed.tree.with(|t| {
            !child_keys(t, &path_icon).is_empty() || !item_keys(t, &path_icon).is_empty()
        });
        if expanded && has_content {
            icondata_lu::LuFolderOpen
        } else {
            icondata_lu::LuFolder
        }
    });
    let key_chevron = key.clone();
    let chevron = Signal::derive(move || {
        if ed.collapsed.with(|c| c.contains(&key_chevron)) {
            icondata_lu::LuChevronRight
        } else {
            icondata_lu::LuChevronDown
        }
    });

    let key_toggle = key.clone();
    let on_toggle = move |_| {
        ed.collapsed.update(|c| {
            if !c.remove(&key_toggle) {
                c.insert(key_toggle.clone());
            }
        });
    };

    let name_area = name_area(ed, path, key.clone());
    let actions = folder_actions(ed, path, &key, is_root);

    let path_children = path.to_vec();
    let path_items = path.to_vec();

    view! {
        <li class="py-0.5">
            <div class="flex items-center gap-1">
                <button type="button" class=ICON_BTN aria-label="Toggle folder" on:click=on_toggle>
                    <leptos_icons::Icon icon=chevron width="14" height="14" />
                </button>
                <leptos_icons::Icon icon=folder_icon width="15" height="15" />
                {name_area}
                {actions}
            </div>
            <ul class="ml-2 border-l border-edge pl-4" class:hidden=is_collapsed>
                <For
                    each=move || ed.tree.with(|t| child_keys(t, &path_children))
                    key=|k: &String| k.clone()
                    let:child_key
                >
                    {render_folder(ed, &parse_key(&child_key))}
                </For>
                <For
                    each=move || ed.tree.with(|t| item_keys(t, &path_items))
                    key=|k: &String| k.clone()
                    let:item_key
                >
                    {render_item(ed, &item_key)}
                </For>
            </ul>
        </li>
    }
    .into_any()
}

/// The name cell: the folder name, or an inline rename input when this folder
/// is being renamed (a single reactive closure over `renaming` + `tree`).
fn name_area(ed: TreeEditor, path: &[usize], key: String) -> AnyView {
    let path = path.to_vec();
    let inner = move || {
        if ed.renaming.with(|r| r.as_deref() == Some(key.as_str())) {
            let confirm_path = path.clone();
            let on_confirm = move |_| {
                let draft = ed.rename_draft.get();
                ed.tree
                    .update(|t| rename_folder(t, &confirm_path, draft.trim()));
                ed.renaming.set(None);
            };
            let on_cancel = move |_| ed.renaming.set(None);
            view! {
                <span class="inline-flex items-center gap-1">
                    <input
                        class=INPUT
                        aria-label="Folder name"
                        prop:value=move || ed.rename_draft.get()
                        on:input:target=move |ev| ed.rename_draft.set(ev.target().value())
                    />
                    <button
                        type="button"
                        class=ICON_BTN
                        aria-label="Confirm rename"
                        on:click=on_confirm
                    >
                        <leptos_icons::Icon icon=icondata_lu::LuCheck width="14" height="14" />
                    </button>
                    <button
                        type="button"
                        class=ICON_BTN
                        aria-label="Cancel rename"
                        on:click=on_cancel
                    >
                        <leptos_icons::Icon icon=icondata_lu::LuX width="14" height="14" />
                    </button>
                </span>
            }
            .into_any()
        } else {
            let path_name = path.clone();
            let name = ed.tree.with(|t| node_name(t, &path_name));
            view! { <span class="font-medium text-ink">{name}</span> }.into_any()
        }
    };
    view! { {inner} }.into_any()
}

/// The per-folder action buttons: add subfolder, add item, rename, and (for
/// non-root folders) delete.
fn folder_actions(ed: TreeEditor, path: &[usize], key: &str, is_root: bool) -> AnyView {
    let key = key.to_owned();
    let path = path.to_vec();
    let add_folder_path = path.clone();
    let add_folder_key = key.clone();
    let on_add_folder = move |_| {
        ed.tree
            .update(|t| add_subfolder(t, &add_folder_path, "New folder"));
        ed.collapsed.update(|c| {
            c.remove(&add_folder_key);
        });
    };

    let picker_key = key.clone();
    let on_add_item = move |_| ed.picker_target.set(Some(picker_key.clone()));

    let rename_path = path.clone();
    let rename_key = key.clone();
    let on_rename = move |_| {
        ed.rename_draft
            .set(ed.tree.with(|t| node_name(t, &rename_path)));
        ed.renaming.set(Some(rename_key.clone()));
    };

    let delete_path = path.clone();
    let on_delete = move |_| {
        ed.tree.update(|t| delete_folder(t, &delete_path));
    };

    let delete_button = (!is_root).then(|| {
        view! {
            <button
                type="button"
                class=ICON_BTN_DANGER
                aria-label="Delete folder"
                title="Delete folder"
                on:click=on_delete
            >
                <leptos_icons::Icon icon=icondata_lu::LuTrash width="14" height="14" />
            </button>
        }
    });

    view! {
        <span class="ml-1 flex items-center gap-0.5">
            <button
                type="button"
                class=ICON_BTN
                aria-label="Add subfolder"
                title="Add subfolder"
                on:click=on_add_folder
            >
                <leptos_icons::Icon icon=icondata_lu::LuFolderPlus width="14" height="14" />
            </button>
            <button
                type="button"
                class=ICON_BTN
                aria-label="Add item reference"
                title="Add item reference"
                on:click=on_add_item
            >
                <leptos_icons::Icon icon=icondata_lu::LuFilePlus width="14" height="14" />
            </button>
            <button
                type="button"
                class=ICON_BTN
                aria-label="Rename folder"
                title="Rename folder"
                on:click=on_rename
            >
                <leptos_icons::Icon icon=icondata_lu::LuPencil width="14" height="14" />
            </button>
            {delete_button}
        </span>
    }
    .into_any()
}

/// One item reference row: the ref type + id value, with a remove button.
fn render_item(ed: TreeEditor, item_key_str: &str) -> AnyView {
    let (folder_path, idx) = parse_item_key(item_key_str);
    let (ref_type, id) = ed.tree.with(|t| item_summary(t, &folder_path, idx));
    let on_remove = move |_| {
        ed.tree.update(|t| remove_item(t, &folder_path, idx));
    };
    view! {
        <li class="flex items-center gap-1 py-0.5 text-ink-muted">
            <leptos_icons::Icon icon=icondata_lu::LuFileText width="14" height="14" />
            <span class="mr-1 text-xs uppercase">{ref_type}</span>
            <span class="break-all font-mono text-xs">{id}</span>
            <button
                type="button"
                class=ICON_BTN_DANGER
                aria-label="Remove item"
                title="Remove item"
                on:click=on_remove
            >
                <leptos_icons::Icon icon=icondata_lu::LuX width="12" height="12" />
            </button>
        </li>
    }
    .into_any()
}

/// The "add item reference" picker: an overlay revealed when a folder's add
/// button sets `picker_target`. Offers the EHR's compositions (first page,
/// from the shared picker resource) and a manual `OBJECT_REF` entry form.
#[allow(clippy::too_many_lines)] // the modal composes the composition list + the manual-entry form as one overlay
fn picker_modal(ed: TreeEditor) -> AnyView {
    let manual_namespace = RwSignal::new("local".to_owned());
    let manual_type = RwSignal::new("COMPOSITION".to_owned());
    let manual_id_type = RwSignal::new("HIER_OBJECT_ID".to_owned());
    let manual_id = RwSignal::new(String::new());

    let close = move |_| ed.picker_target.set(None);

    let add_manual = move |_| {
        if let Some(key) = ed.picker_target.get() {
            let path = parse_key(&key);
            let item = object_ref(
                manual_namespace.get().trim(),
                manual_type.get().trim(),
                manual_id_type.get().trim(),
                manual_id.get().trim(),
            );
            ed.tree.update(|t| add_item(t, &path, item));
            ed.collapsed.update(|c| {
                c.remove(&key);
            });
            manual_id.set(String::new());
            ed.picker_target.set(None);
        }
    };

    let composition_list = view! {
        <Transition fallback=|| {
            view! { <p class="text-sm text-ink-muted">"Loading compositions…"</p> }
        }>
            {move || Suspend::new(async move {
                match ed.picker.await {
                    Ok(Some(page)) => composition_choices(ed, &page.rows),
                    Ok(None) => ().into_any(),
                    Err(e) => inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any();

    view! {
        <div
            class="fixed inset-0 z-40 flex items-start justify-center bg-black/40 p-4 sm:items-center"
            class:hidden=move || ed.picker_target.with(Option::is_none)
        >
            <div class="max-h-[85vh] w-full max-w-xl overflow-auto rounded-card border border-edge bg-raised p-4 shadow-card">
                <div class="mb-3 flex items-center justify-between">
                    <h3 class=CARD_TITLE>"Add item reference"</h3>
                    <button type="button" class=ICON_BTN aria-label="Close" on:click=close>
                        <leptos_icons::Icon icon=icondata_lu::LuX width="16" height="16" />
                    </button>
                </div>

                <p class="mb-2 text-sm font-medium text-ink">"This EHR's compositions"</p>
                <div class="mb-4 flex flex-col gap-1">{composition_list}</div>

                <p class="mb-2 text-sm font-medium text-ink">"Or enter a reference manually"</p>
                <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
                    <input
                        class=INPUT
                        aria-label="Namespace"
                        placeholder="namespace (e.g. local)"
                        prop:value=move || manual_namespace.get()
                        on:input:target=move |ev| manual_namespace.set(ev.target().value())
                    />
                    <input
                        class=INPUT
                        aria-label="Reference type"
                        placeholder="type (e.g. COMPOSITION)"
                        prop:value=move || manual_type.get()
                        on:input:target=move |ev| manual_type.set(ev.target().value())
                    />
                    <input
                        class=INPUT
                        aria-label="Id type"
                        placeholder="id type (e.g. HIER_OBJECT_ID)"
                        prop:value=move || manual_id_type.get()
                        on:input:target=move |ev| manual_id_type.set(ev.target().value())
                    />
                    <input
                        class=INPUT
                        aria-label="Id value"
                        placeholder="id value"
                        prop:value=move || manual_id.get()
                        on:input:target=move |ev| manual_id.set(ev.target().value())
                    />
                </div>
                <div class="mt-3 flex justify-end gap-2">
                    <button type="button" class=BTN_SECONDARY on:click=close>
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        class=BTN_PRIMARY
                        disabled=Signal::derive(move || manual_id.with(|v| v.trim().is_empty()))
                        on:click=add_manual
                    >
                        <leptos_icons::Icon icon=icondata_lu::LuPlus width="14" height="14" />
                        "Add reference"
                    </button>
                </div>
            </div>
        </div>
    }
    .into_any()
}

/// The composition rows in the picker: each adds a `COMPOSITION` `OBJECT_REF`
/// (namespace `local`, `HIER_OBJECT_ID` = the versioned-object id) to the
/// target folder.
fn composition_choices(ed: TreeEditor, rows: &[Vec<Value>]) -> AnyView {
    if rows.is_empty() {
        return view! { <p class="text-sm text-ink-muted">"No compositions in this EHR yet."</p> }
            .into_any();
    }
    let rows = rows.to_vec();
    view! {
        <For
            each=move || rows.clone()
            key=|row| row.first().map(cell_text).unwrap_or_default()
            let:row
        >
            {composition_choice(ed, &row)}
        </For>
    }
    .into_any()
}

/// One composition choice row.
fn composition_choice(ed: TreeEditor, row: &[Value]) -> AnyView {
    let uid = row.first().map(cell_text).unwrap_or_default();
    let name = row.get(1).map(cell_text).unwrap_or_default();
    let object_id = versioned_object_id(&uid).to_owned();
    let uid_display = uid.clone();
    let on_add = move |_| {
        if let Some(key) = ed.picker_target.get() {
            let path = parse_key(&key);
            let item = object_ref("local", "COMPOSITION", "HIER_OBJECT_ID", &object_id);
            ed.tree.update(|t| add_item(t, &path, item));
            ed.collapsed.update(|c| {
                c.remove(&key);
            });
            ed.picker_target.set(None);
        }
    };
    view! {
        <button
            type="button"
            class="flex w-full items-center justify-between gap-2 rounded-control border border-edge px-3 py-1.5 text-left text-sm hover:bg-sunken"
            on:click=on_add
        >
            <span class="min-w-0">
                <span class="block truncate text-ink">{name}</span>
                <span class="block truncate font-mono text-xs text-ink-muted">{uid_display}</span>
            </span>
            <leptos_icons::Icon icon=icondata_lu::LuPlus width="14" height="14" />
        </button>
    }
    .into_any()
}

/// The save action's failure pane: the CDR's diagnostics verbatim in a
/// scrollable WELL (`<pre>`). Success is a toast (see the section orchestrator).
fn save_feedback(
    update: Action<(String, String, String), Result<String, AdminUiError>>,
) -> AnyView {
    view! {
        {move || match update.value().get() {
            Some(Err(error)) => {
                view! {
                    <div class=format!("{WELL} mt-3")>
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

/// A read-only FOLDER tree (the create preview and the history / time / path
/// panels): folders and their item references, no actions.
pub(in crate::pages::ehr_detail::directory) fn read_only_tree(folder: &Value) -> AnyView {
    view! { <ul class="text-sm text-ink">{read_only_folder(folder)}</ul> }.into_any()
}

/// One read-only FOLDER node.
fn read_only_folder(folder: &Value) -> AnyView {
    let name = folder
        .get("name")
        .and_then(|n| n.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("(folder)")
        .to_owned();
    let subfolders = folder
        .get("folders")
        .and_then(Value::as_array)
        .map(|folders| folders.iter().map(read_only_folder).collect::<Vec<_>>())
        .unwrap_or_default();
    let items = folder
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(read_only_item).collect::<Vec<_>>())
        .unwrap_or_default();
    view! {
        <li class="py-0.5">
            <span class="inline-flex items-center gap-1.5 font-medium text-ink">
                <leptos_icons::Icon icon=icondata_lu::LuFolder width="14" height="14" />
                {name}
            </span>
            <ul class="ml-2 border-l border-edge pl-4">{subfolders} {items}</ul>
        </li>
    }
    .into_any()
}

/// One read-only `OBJECT_REF` item.
fn read_only_item(item: &Value) -> AnyView {
    let ref_type = item
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("OBJECT")
        .to_owned();
    let id = item
        .get("id")
        .and_then(|i| i.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("(ref)")
        .to_owned();
    view! {
        <li class="flex items-center gap-1.5 py-0.5 text-ink-muted">
            <leptos_icons::Icon icon=icondata_lu::LuFileText width="14" height="14" />
            <span class="text-xs uppercase">{ref_type}</span>
            <span class="break-all font-mono text-xs">{id}</span>
        </li>
    }
    .into_any()
}
