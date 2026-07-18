//! The structured directory tree editor: a recursive, editable view of the
//! root `FOLDER` (add / rename / delete folders, add / remove item
//! references) backed by a single working-tree signal, with a dirty-state
//! save bar (PUT with `If-Match`), an advanced raw-JSON mode, and the
//! composition picker for adding `OBJECT_REF` items. Also the shared
//! read-only tree renderer used by the create preview and the history / time /
//! path panels.
//!
//! The working tree is one `RwSignal<serde_json::Value>` seeded from the
//! loaded FOLDER, then stamped with an ephemeral, client-only `_key` identity
//! on every folder ([`super::edit::stamp_keys`]); every mutation goes through
//! the pure [`super::edit`] helpers, and every rendered datum reads the tree
//! reactively. `<For>` rows and the collapse / rename / picker UI state are
//! keyed by that stable, data-derived `_key` (never a positional path — rules
//! §4), so a folder keeps its own state and row after a sibling delete shifts
//! indices; a folder's positional path is re-derived from its `_key`
//! ([`super::edit::find_path_by_key`]) for each read and mutation. The `_key`
//! is stripped ([`super::edit::strip_keys`]) from every body sent to the CDR
//! and from the advanced-JSON view — it never leaves the console.

use leptos::prelude::*;
use serde_json::Value;

use crate::components::field::{BTN_PRIMARY, BTN_SECONDARY, INPUT, TEXTAREA};
use crate::components::format_view::{inline_error, pretty_body};
use crate::components::surface::{CARD_PAD, CARD_TITLE, WELL};
use crate::error::AdminUiError;
use crate::format::ReprFormat;
use crate::pages::ehr_detail::directory::edit::{
    add_item, add_subfolder, child_node_keys, count_tree, delete_folder, find_path_by_key,
    item_count, item_summary, node_key_at, node_name, object_ref, remove_item, rename_folder,
    stamp_keys, strip_keys, versioned_object_id,
};
use crate::pages::ehr_detail::directory::{DirectoryState, PickerResource};
use crate::pages::ehrs::cell_text;

/// A small square icon button (node actions).
const ICON_BTN: &str = "inline-flex items-center justify-center rounded-control p-1 text-ink-muted hover:bg-sunken hover:text-ink focus:outline-none focus:ring-2 focus:ring-accent";

/// A small square destructive icon button (node delete).
const ICON_BTN_DANGER: &str = "inline-flex items-center justify-center rounded-control p-1 text-ink-muted hover:bg-danger-subtle hover:text-danger focus:outline-none focus:ring-2 focus:ring-danger";

/// Serialize the working tree for the CDR with the ephemeral `_key` identity
/// stripped (rules §4 contract — console-local identity never leaves the BFF).
fn strip_keys_to_string(tree: &Value) -> String {
    let mut stripped = tree.clone();
    strip_keys(&mut stripped);
    serde_json::to_string(&stripped).unwrap_or_default()
}

/// The editor's shared, `Copy` handle threaded through the recursive render:
/// the working tree plus its UI state and the composition picker.
#[derive(Clone, Copy)]
struct TreeEditor {
    /// The working FOLDER tree (mutated in place; the single source of truth).
    /// Every folder carries an ephemeral `_key` identity (see the module doc).
    tree: RwSignal<Value>,
    /// Ephemeral `_key`s of collapsed folders (expanded unless listed).
    collapsed: RwSignal<std::collections::HashSet<String>>,
    /// The ephemeral `_key` of the folder currently being renamed, if any.
    renaming: RwSignal<Option<String>>,
    /// The in-progress rename text.
    rename_draft: RwSignal<String>,
    /// The next free `_key` ordinal, advanced as folders are added (see
    /// [`super::edit::stamp_keys`]); persisted across clicks for uniqueness.
    counter: StoredValue<u64>,
    /// The composition list for the "add item" picker (created outside the
    /// Suspend, read here — rules §4).
    picker: PickerResource,
    /// The ephemeral `_key` of the folder awaiting an item (also opens the
    /// picker).
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
    force_save: Action<(String, String), Result<String, AdminUiError>>,
    reload: RwSignal<u32>,
    picker: PickerResource,
    picker_target: RwSignal<Option<String>>,
) -> AnyView {
    let mut doc: Value = match serde_json::from_str(&state.body) {
        Ok(value) => value,
        Err(e) => {
            return inline_error(&AdminUiError::Internal(format!("directory JSON: {e}")));
        }
    };
    let version_uid = state.version_uid.clone();
    // The advanced-JSON draft shows the STRIPPED tree; the server body already
    // carries no `_key`, so seeding it directly is correct (rules §4 contract).
    let pretty = pretty_body(&state.body, ReprFormat::CanonicalJson);

    // Stamp every folder with its ephemeral `_key` identity ONCE, then seed
    // both the working tree and the pristine baseline from the SAME stamped
    // copy so `dirty` stays a plain equality compare (rules §4; see
    // [`super::edit::stamp_keys`]).
    let mut seed_counter = 0u64;
    stamp_keys(&mut doc, &mut seed_counter);
    let tree = RwSignal::new(doc.clone());
    let original = StoredValue::new(doc);
    let counter = StoredValue::new(seed_counter);
    let collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let renaming = RwSignal::new(Option::<String>::None);
    let rename_draft = RwSignal::new(String::new());
    let advanced = RwSignal::new(false);
    let json_draft = RwSignal::new(pretty);
    let json_error = RwSignal::new(Option::<String>::None);

    let dirty = Memo::new(move |_| tree.with(|t| original.with_value(|o| t != o)));

    // A `412` on THIS loaded version (completions after this editor's
    // creation baseline): the refetch trigger deliberately ignores failed
    // writes so the working tree survives — this banner offers the two
    // explicit ways out (discard-and-reload, or informed overwrite).
    let editor_baseline = update.version().get_untracked();
    let conflicted = Memo::new(move |_| {
        update.version().get() > editor_baseline
            && update
                .value()
                .with(|v| matches!(v, Some(Err(e)) if super::is_conflict(e)))
    });

    let ed = TreeEditor {
        tree,
        collapsed,
        renaming,
        rename_draft,
        counter,
        picker,
        picker_target,
    };

    // Save the working tree as a new version (PUT + If-Match). The ephemeral
    // `_key` identity is stripped from the wire body (rules §4 contract).
    let on_save = move |_| {
        let body = tree.with(strip_keys_to_string);
        update.dispatch((ehr_id.get(), version_uid.clone(), body));
    };
    let on_discard = move |_| {
        tree.set(original.get_value());
        renaming.set(None);
        json_error.set(None);
    };

    // Advanced mode: toggling on seeds the JSON draft from the current tree
    // (with `_key` stripped — the user never sees console-local identity);
    // "Apply" parses it back into the working tree and re-stamps.
    let on_toggle_advanced = move |_| {
        advanced.update(|open| {
            *open = !*open;
            if *open {
                let text = tree.with(|t| {
                    let mut stripped = t.clone();
                    strip_keys(&mut stripped);
                    serde_json::to_string_pretty(&stripped).unwrap_or_else(|_| "{}".to_owned())
                });
                json_draft.set(text);
                json_error.set(None);
            }
        });
    };
    let on_apply_json = move |_| match serde_json::from_str::<Value>(&json_draft.get()) {
        Ok(mut value) => {
            counter.update_value(|c| stamp_keys(&mut value, c));
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

            // The optimistic-concurrency conflict banner: shown after a 412
            // on this loaded version; the unsaved tree is intact.
            <div
                id="directory-conflict"
                class="mt-3 flex flex-wrap items-center gap-3 rounded-control border border-danger/40 bg-danger-subtle px-3 py-2"
                class:hidden=move || !conflicted.get()
            >
                <span class="text-sm text-danger">
                    "This directory changed on the server since it was loaded. Your unsaved edits are still here."
                </span>
                <button
                    type="button"
                    class=BTN_PRIMARY
                    disabled=Signal::derive(move || force_save.pending().get())
                    on:click=move |_| {
                        let body = tree.with(strip_keys_to_string);
                        force_save.dispatch((ehr_id.get(), body));
                    }
                >
                    "Save anyway (overwrite the server change)"
                </button>
                <button
                    type="button"
                    class=BTN_SECONDARY
                    on:click=move |_| reload.update(|n| *n = n.wrapping_add(1))
                >
                    "Discard my edits and load the server version"
                </button>
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

/// The root of the editable tree (the root folder, addressed by its ephemeral
/// `_key`). The root is always present and stamped at seed.
fn tree_view(ed: TreeEditor) -> AnyView {
    let root_key = ed.tree.with(|t| node_key_at(t, &[])).unwrap_or_default();
    render_folder(ed, root_key, true)
}

/// Render one FOLDER node, identified by its ephemeral `_key` (`node_key`).
/// Its live positional `path` is re-derived from that `_key` on every reactive
/// read and mutation ([`find_path_by_key`]) so the row stays correct after a
/// sibling delete shifts indices; `<For>` rows and the collapse / rename UI
/// state key on `node_key`, never a positional path (rules §4).
fn render_folder(ed: TreeEditor, node_key: String, is_root: bool) -> AnyView {
    let key_collapsed = node_key.clone();
    let is_collapsed = move || ed.collapsed.with(|c| c.contains(&key_collapsed));

    let key_icon = node_key.clone();
    let folder_icon = Signal::derive(move || {
        let expanded = ed.collapsed.with(|c| !c.contains(&key_icon));
        let has_content = ed.tree.with(|t| {
            find_path_by_key(t, &key_icon)
                .is_some_and(|p| !child_node_keys(t, &p).is_empty() || item_count(t, &p) > 0)
        });
        if expanded && has_content {
            icondata_lu::LuFolderOpen
        } else {
            icondata_lu::LuFolder
        }
    });
    let key_chevron = node_key.clone();
    let chevron = Signal::derive(move || {
        if ed.collapsed.with(|c| c.contains(&key_chevron)) {
            icondata_lu::LuChevronRight
        } else {
            icondata_lu::LuChevronDown
        }
    });

    let key_toggle = node_key.clone();
    let on_toggle = move |_| {
        ed.collapsed.update(|c| {
            if !c.remove(&key_toggle) {
                c.insert(key_toggle.clone());
            }
        });
    };

    let name_area = name_area(ed, node_key.clone());
    let actions = folder_actions(ed, node_key.clone(), is_root);

    let key_children = node_key.clone();
    let key_items = node_key;

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
                    each=move || {
                        ed.tree
                            .with(|t| {
                                find_path_by_key(t, &key_children)
                                    .map(|p| child_node_keys(t, &p))
                                    .unwrap_or_default()
                            })
                    }
                    key=|k: &String| k.clone()
                    let:child_key
                >
                    {render_folder(ed, child_key, false)}
                </For>
                <For
                    each=move || {
                        ed.tree
                            .with(|t| {
                                let count = find_path_by_key(t, &key_items)
                                    .map_or(0, |p| item_count(t, &p));
                                (0..count).map(|i| (key_items.clone(), i)).collect::<Vec<_>>()
                            })
                    }
                    key=|(parent, idx): &(String, usize)| format!("{parent}#{idx}")
                    let:item
                >
                    {render_item(ed, item.0, item.1)}
                </For>
            </ul>
        </li>
    }
    .into_any()
}

/// The name cell: the folder name, or an inline rename input when this folder
/// is being renamed (a single reactive closure over `renaming` + `tree`).
/// Identity is the folder's ephemeral `_key`; the path is re-derived per use.
fn name_area(ed: TreeEditor, node_key: String) -> AnyView {
    let inner = move || {
        if ed
            .renaming
            .with(|r| r.as_deref() == Some(node_key.as_str()))
        {
            let confirm_key = node_key.clone();
            let on_confirm = move |_| {
                let draft = ed.rename_draft.get();
                ed.tree.update(|t| {
                    if let Some(p) = find_path_by_key(t, &confirm_key) {
                        rename_folder(t, &p, draft.trim());
                    }
                });
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
            let name_key = node_key.clone();
            let name = ed.tree.with(|t| {
                find_path_by_key(t, &name_key)
                    .map_or_else(|| "(folder)".to_owned(), |p| node_name(t, &p))
            });
            view! { <span class="font-medium text-ink">{name}</span> }.into_any()
        }
    };
    view! { {inner} }.into_any()
}

/// The per-folder action buttons: add subfolder, add item, rename, and (for
/// non-root folders) delete. The folder is identified by its ephemeral `_key`;
/// each mutation re-derives the live path from it before calling the pure
/// path-based [`super::edit`] helpers (rules §4).
fn folder_actions(ed: TreeEditor, node_key: String, is_root: bool) -> AnyView {
    let add_folder_key = node_key.clone();
    let on_add_folder = move |_| {
        // Append the subfolder, then stamp the (only) unstamped folder with a
        // fresh `_key` so the new row gets a stable identity too.
        ed.counter.update_value(|c| {
            ed.tree.update(|t| {
                if let Some(p) = find_path_by_key(t, &add_folder_key) {
                    add_subfolder(t, &p, "New folder");
                    stamp_keys(t, c);
                }
            });
        });
        ed.collapsed.update(|c| {
            c.remove(&add_folder_key);
        });
    };

    let picker_key = node_key.clone();
    let on_add_item = move |_| ed.picker_target.set(Some(picker_key.clone()));

    let rename_key = node_key.clone();
    let on_rename = move |_| {
        let current = ed.tree.with(|t| {
            find_path_by_key(t, &rename_key).map_or_else(String::new, |p| node_name(t, &p))
        });
        ed.rename_draft.set(current);
        ed.renaming.set(Some(rename_key.clone()));
    };

    let delete_key = node_key;
    let on_delete = move |_| {
        ed.tree.update(|t| {
            if let Some(p) = find_path_by_key(t, &delete_key) {
                delete_folder(t, &p);
            }
        });
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
/// Keyed by the owning folder's ephemeral `_key` plus the item index (items
/// carry no per-row state, so a positional index is a fine tiebreaker — rules
/// §4). The folder path is re-derived from `parent_key` for the reactive read
/// and the remove mutation.
fn render_item(ed: TreeEditor, parent_key: String, idx: usize) -> AnyView {
    let summary_key = parent_key.clone();
    let (ref_type, id) = ed.tree.with(|t| {
        find_path_by_key(t, &summary_key).map_or_else(
            || ("OBJECT".to_owned(), "(ref)".to_owned()),
            |p| item_summary(t, &p, idx),
        )
    });
    let on_remove = move |_| {
        ed.tree.update(|t| {
            if let Some(p) = find_path_by_key(t, &parent_key) {
                remove_item(t, &p, idx);
            }
        });
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
            let item = object_ref(
                manual_namespace.get().trim(),
                manual_type.get().trim(),
                manual_id_type.get().trim(),
                manual_id.get().trim(),
            );
            ed.tree.update(|t| {
                if let Some(p) = find_path_by_key(t, &key) {
                    add_item(t, &p, item);
                }
            });
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
            let item = object_ref("local", "COMPOSITION", "HIER_OBJECT_ID", &object_id);
            ed.tree.update(|t| {
                if let Some(p) = find_path_by_key(t, &key) {
                    add_item(t, &p, item);
                }
            });
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
