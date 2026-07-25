//! The `/queries` screen: the CDR's stored queries (left) and the
//! console-local query groups (right).
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The stored-query surface is the ITS-REST Definition/Query API
//! (`docs/specs/openehr/ITS-REST/docs/definition/`); groups are console-local
//! (no ITS-REST resource exists for them). All data flows through the
//! [`crate::queries_api`] server fns plus the admin gate + stored-query delete
//! in [`crate::admin`] — each guards the session itself; this screen declares
//! no server fn of its own.
//!
//! Discipline (rules §1/§4/§6/§8): the view is composed from `.into_any()`-
//! erased section locals; the stored-query list is a `<For>` keyed by
//! `name@version`; refetched data (the groups list, the on-demand AQL detail)
//! reads under `<Transition>`; the table emits an explicit `<tbody>`; internal
//! navigation uses `<A>`; there is zero authored JavaScript (`on:` Rust
//! listeners only).

use leptos::component;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::admin::AdminAvailability;
use crate::components::data_table::{CELL, CELL_MONO, ROW, table_shell};
use crate::components::empty_state::EmptyState;
use crate::components::field::{BTN_DANGER, BTN_PRIMARY, BTN_SECONDARY, INPUT};
use crate::components::format_view::DocumentPane;
use crate::components::page_header::PageHeader;
use crate::components::surface::{CARD_PAD, CARD_TITLE};
use crate::components::toast::{toast_error, toast_success};
use crate::error::AdminUiError;
use crate::pages::dashboard::split_query_ref;
use crate::pages::ehrs::table_skeleton;
use crate::queries_api::{
    QueryGroup, StoredQueryRow, fetch_stored_query, list_groups, list_stored_queries, save_group,
};

/// The stored-query delete action: the `(name, version)` it was dispatched
/// with, paired with the CDR's answer, so both toasts can name the exact
/// query. This deletes from the **CDR's** stored-query store — never the
/// console-local groups (see [`groups_panel`]).
type CdrQueryDelete = Action<(String, String), ((String, String), Result<(), AdminUiError>)>;

/// The `/queries` screen: a stored-queries table with an on-demand AQL detail,
/// alongside the console-local query-group editor.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn QueriesPage() -> impl IntoView {
    let toaster = thaw::ToasterInjection::expect_context();
    // The admin probe and the CDR stored-query delete it gates (discover-and-hide:
    // no advertised admin group, no delete affordance). Created in setup so the
    // gated view can re-render without re-creating them (rules §4).
    let gate = crate::admin::admin_gate();
    let cdr_delete: CdrQueryDelete = Action::new(|key: &(String, String)| {
        let key = key.clone();
        async move {
            let outcome =
                crate::admin::admin_delete_stored_query(key.0.clone(), key.1.clone()).await;
            (key, outcome)
        }
    });
    // Stored queries feed both the table (left) and the group form's member
    // checkboxes (right); a Resource is Copy so both read it. A CDR delete
    // bumps the action's version, which is the resource's source (rules §6).
    let stored = Resource::new(
        move || cdr_delete.version().get(),
        |_| async move { list_stored_queries().await },
    );
    // The group save/delete action; its version drives the groups refetch.
    let save = Action::new(|input: &(String, Vec<String>)| {
        let (name, members) = input.clone();
        async move { save_group(name, members).await }
    });
    // Records the last mutation's intent so the completion toast reads
    // correctly ("Group saved" vs "Group removed"); set at each dispatch site.
    let save_intent = RwSignal::new("save");
    // Report each mutation's outcome as a toast (rules: Effect = sync with the
    // outside world; no signal is written here). Runs client-side only.
    Effect::new(move |_| match save.value().get() {
        Some(Ok(())) => {
            let title = if save_intent.get_untracked() == "delete" {
                "Group removed"
            } else {
                "Group saved"
            };
            toast_success(toaster, title, "");
        }
        Some(Err(error)) => {
            crate::feedback::toast_write_failure(toaster, "Save failed", "the query group", &error);
        }
        None => {}
    });
    // The CDR stored-query delete reports separately — its copy names the
    // query, and a failure carries the actionable next action.
    Effect::new(move |_| match cdr_delete.value().get() {
        Some(((name, version), Ok(()))) => toast_success(
            toaster,
            "Stored query deleted",
            &format!("{name} v{version} was removed from the CDR."),
        ),
        Some(((name, version), Err(error))) => toast_error(
            toaster,
            "Delete failed",
            &crate::admin::delete_failure_copy(&format!("Stored query {name} v{version}"), &error),
        ),
        None => {}
    });

    let table = stored_queries_panel(stored, gate, cdr_delete);
    let groups = groups_panel(stored, save, save_intent);

    view! {
        <Title text="Stored queries · ehrbase-admin" />
        <div class="p-6">
            <PageHeader
                title="Queries"
                subtitle="The CDR's stored queries and the console's local query groups."
            >
                <a href="/queries/builder" class=BTN_PRIMARY>
                    "New query"
                </a>
                <a href="/queries/aql" class=BTN_SECONDARY>
                    "Raw AQL"
                </a>
            </PageHeader>
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 items-start">{table} {groups}</div>
        </div>
    }
}

// ── Stored queries (left) ────────────────────────────────────────────────

/// The stored-queries panel: the list table plus the single-select AQL detail
/// shown below it.
fn stored_queries_panel(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    gate: Resource<Result<AdminAvailability, AdminUiError>>,
    cdr_delete: CdrQueryDelete,
) -> AnyView {
    // The row (name, version) whose AQL detail is currently expanded.
    let selected = RwSignal::new(Option::<(String, String)>::None);
    // The selected query's AQL, fetched on demand; `None` source → no call.
    let detail = Resource::new(
        move || selected.get(),
        |key| async move {
            match key {
                Some((name, version)) => fetch_stored_query(name, version).await.map(Some),
                None => Ok(None),
            }
        },
    );
    // The `(name, version)` awaiting confirmation in the modal (`None` = no
    // dialog). ONE dialog serves every row — the signal is both "which row" and
    // "open".
    let pending_delete = RwSignal::new(Option::<(String, String)>::None);
    let table = stored_table(stored, selected, gate, cdr_delete, pending_delete);
    let detail_view = stored_detail(detail);
    let confirm = cdr_delete_dialog(pending_delete, cdr_delete);
    view! {
        <section class="space-y-3">
            <h2 class=CARD_TITLE>"Stored queries"</h2>
            {table}
            {detail_view}
            {confirm}
        </section>
    }
    .into_any()
}

/// The panel's ONE CDR-delete confirmation modal, driven by `pending_delete`
/// (which stored-query version triggered it). Rendered once outside the table so
/// a list refetch never re-creates it, and inert while nothing is pending —
/// which is why it needs no admin gate of its own: only an admin-gated trigger
/// can set the signal. The copy is explicit that this is the CDR's store, not a
/// console-local grouping.
fn cdr_delete_dialog(
    pending_delete: RwSignal<Option<(String, String)>>,
    cdr_delete: CdrQueryDelete,
) -> AnyView {
    let message = Signal::derive(move || {
        pending_delete
            .get()
            .map_or_else(String::new, |(name, version)| {
                format!(
                    "Permanently delete stored query “{name}” version {version} from the CDR? \
                     Every client loses that version — this is not the console-local group \
                     removal, and it cannot be undone."
                )
            })
    });
    view! {
        <crate::components::confirm_dialog::ConfirmDialog
            open=Signal::derive(move || pending_delete.get().is_some())
            title="Delete stored query from the CDR"
            message=message
            confirm_label="Delete from CDR"
            confirm_id="stored-query-delete-confirm"
            on_cancel=Callback::new(move |()| pending_delete.set(None))
            on_confirm=Callback::new(move |()| {
                if let Some(key) = pending_delete.get_untracked() {
                    cdr_delete.dispatch(key);
                }
                pending_delete.set(None);
            })
        />
    }
    .into_any()
}

/// The stored-queries table, read under `<Transition>` (a CDR delete refetches
/// the list — keep the current rows visible instead of flashing the skeleton,
/// rules §6). The admin probe is awaited in the SAME `Suspend` as the list, so
/// every row agrees on whether the delete affordance exists.
fn stored_table(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    selected: RwSignal<Option<(String, String)>>,
    gate: Resource<Result<AdminAvailability, AdminUiError>>,
    cdr_delete: CdrQueryDelete,
    pending_delete: RwSignal<Option<(String, String)>>,
) -> AnyView {
    view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                let admin = crate::admin::renders_admin_ops(&gate.await);
                match stored.await {
                    Ok(rows) => stored_rows_view(rows, selected, admin, cdr_delete, pending_delete),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// Render the stored-query rows (or the empty state). The row list is a `<For>`
/// keyed by `name@version` (rules §4 — a stable, data-derived key).
fn stored_rows_view(
    rows: Vec<StoredQueryRow>,
    selected: RwSignal<Option<(String, String)>>,
    admin: bool,
    cdr_delete: CdrQueryDelete,
    pending_delete: RwSignal<Option<(String, String)>>,
) -> AnyView {
    if rows.is_empty() {
        return view! {
            <EmptyState
                icon=icondata_lu::LuSearchCode
                message="No stored queries"
                hint="Build one in the query builder, or write raw AQL, then save it."
            />
        }
        .into_any();
    }
    let body = view! {
        <For each=move || rows.clone() key=|row| format!("{}@{}", row.name, row.version) let:row>
            {stored_row(row, selected, admin, cdr_delete, pending_delete)}
        </For>
    }
    .into_any();
    table_shell(&["Name", "Version", "Saved", ""], body)
}

/// One stored-query row. Clicking it toggles the single-select detail below the
/// table (`on:click` is a Rust listener — zero authored JS, rules §0).
fn stored_row(
    row: StoredQueryRow,
    selected: RwSignal<Option<(String, String)>>,
    admin: bool,
    cdr_delete: CdrQueryDelete,
    pending_delete: RwSignal<Option<(String, String)>>,
) -> impl IntoView {
    let key_for_class = (row.name.clone(), row.version.clone());
    let key_for_click = (row.name.clone(), row.version.clone());
    let is_selected = move || selected.with(|current| current.as_ref() == Some(&key_for_class));
    let on_click = move |_| {
        let key = key_for_click.clone();
        selected.update(|current| {
            if current.as_ref() == Some(&key) {
                *current = None;
            } else {
                *current = Some(key);
            }
        });
    };
    // "Open in editor": a link to the raw-editor route, which reads `?load=` to
    // fetch and seed the query. The click is stopped from bubbling to the row's
    // toggle handler (so a click here never expands the detail); with no router
    // delegation reached, the browser does a plain navigation to the fresh page.
    // `name@version` is percent-encoded as one value.
    let load_href = crate::pages::query_aql::load_href(&row.name, &row.version);
    let delete_button = if admin {
        cdr_delete_button(&row.name, &row.version, cdr_delete, pending_delete)
    } else {
        ().into_any()
    };
    view! {
        <tr
            class=format!("{ROW} cursor-pointer")
            class=("bg-accent-subtle", is_selected)
            on:click=on_click
        >
            <td class=CELL_MONO>{row.name}</td>
            <td class=CELL_MONO>{row.version}</td>
            <td class=format!("{CELL} text-xs text-ink-muted")>{row.saved}</td>
            <td class=format!("{CELL} text-right")>
                // `whitespace-nowrap`: the panel is half-width, so without it the
                // action labels wrap mid-phrase ("Delete from / CDR"). The row
                // stacks the two actions instead.
                <div class="flex flex-wrap items-center justify-end gap-2 whitespace-nowrap">
                    <a href=load_href class=BTN_SECONDARY on:click=|ev| ev.stop_propagation()>
                        "Open in editor"
                    </a>
                    {delete_button}
                </div>
            </td>
        </tr>
    }
}

/// The admin **Delete from CDR** button for one stored-query version: it opens
/// the panel's confirmation modal for THIS `(name, version)` (only the dialog's
/// confirm dispatches). Deliberately labelled to distinguish it from the groups
/// panel's console-local "Remove group" — this one removes the query VERSION
/// from the CDR's stored-query store for every client, not a local grouping.
/// The click never bubbles to the row's detail toggle.
/// `data-query-delete` (`name@version`) is the stable E2E hook.
fn cdr_delete_button(
    name: &str,
    version: &str,
    cdr_delete: CdrQueryDelete,
    pending_delete: RwSignal<Option<(String, String)>>,
) -> AnyView {
    let key = format!("{name}@{version}");
    let target = (name.to_owned(), version.to_owned());
    let on_click = move |ev: leptos::ev::MouseEvent| {
        ev.stop_propagation();
        pending_delete.set(Some(target.clone()));
    };
    view! {
        <button
            type="button"
            class=BTN_DANGER
            data-query-delete=key
            disabled=Signal::derive(move || cdr_delete.pending().get())
            on:click=on_click
        >
            "Delete from CDR"
        </button>
    }
    .into_any()
}

/// The AQL detail below the table: the selected query's AQL in a
/// [`DocumentPane`] plus a Run button. Read under `<Transition>` so the old
/// detail stays visible while a new selection loads (rules §6).
fn stored_detail(detail: Resource<Result<Option<String>, AdminUiError>>) -> AnyView {
    view! {
        <div class="mt-3">
            <Transition fallback=|| {
                view! { <p class="text-sm text-ink-muted">"Loading query…"</p> }
            }>
                {move || Suspend::new(async move {
                    match detail.await {
                        Ok(Some(text)) => detail_panel(text),
                        Ok(None) => ().into_any(),
                        Err(e) => crate::components::format_view::inline_error(&e),
                    }
                })}
            </Transition>
        </div>
    }
    .into_any()
}

/// The expanded detail for one stored query: its AQL and a Run button that
/// navigates to the raw-AQL screen with the query pre-filled via `?aql=`.
fn detail_panel(aql: String) -> AnyView {
    let navigate = use_navigate();
    let aql_for_run = aql.clone();
    let on_run = move |_| {
        navigate(
            &crate::pages::query_aql::aql_href(&aql_for_run),
            NavigateOptions::default(),
        );
    };
    let body = Signal::derive(move || aql.clone());
    view! {
        <div class=CARD_PAD>
            <div class="flex items-center justify-between mb-2">
                <span class="text-xs font-semibold uppercase tracking-wide text-ink-muted">
                    "AQL"
                </span>
                <button type="button" class=BTN_PRIMARY on:click=on_run>
                    "Run"
                </button>
            </div>
            <DocumentPane body=body />
        </div>
    }
    .into_any()
}

// ── Query groups (right) ─────────────────────────────────────────────────

/// The groups panel: a create/edit form over the console-local groups, and the
/// list of existing groups with edit/delete controls.
fn groups_panel(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    save: Action<(String, Vec<String>), Result<(), AdminUiError>>,
    save_intent: RwSignal<&'static str>,
) -> AnyView {
    // Groups refetch whenever the save action's version bumps (rules §6).
    let groups = Resource::new(
        move || save.version().get(),
        |_| async move { list_groups().await },
    );
    // Form state, shared between the form and the per-group Edit buttons.
    let name = RwSignal::new(String::new());
    let selected = RwSignal::new(Vec::<String>::new());
    // Which existing group is awaiting confirmation in the modal (`None` = no
    // dialog): both "which group" and "open", one signal.
    let pending_delete = RwSignal::new(Option::<String>::None);

    let form = group_form(stored, save, save_intent, name, selected);
    let list = groups_list(groups, name, selected, pending_delete);
    let confirm = group_remove_dialog(pending_delete, save, save_intent);
    view! {
        <section class="space-y-3">
            <h2 class=CARD_TITLE>"Groups"</h2>
            {form}
            {list}
            {confirm}
        </section>
    }
    .into_any()
}

/// The create/edit form: a name input, member checkboxes over the stored
/// queries (`name@version`), and Save/Clear buttons dispatching [`save_group`].
fn group_form(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    save: Action<(String, Vec<String>), Result<(), AdminUiError>>,
    save_intent: RwSignal<&'static str>,
    name: RwSignal<String>,
    selected: RwSignal<Vec<String>>,
) -> AnyView {
    let on_save = move |_| {
        let group_name = name.get().trim().to_owned();
        save_intent.set("save");
        save.dispatch((group_name, selected.get()));
    };
    let on_clear = move |_| {
        name.set(String::new());
        selected.set(Vec::new());
    };
    let saving = Signal::derive(move || save.pending().get());
    let checkboxes = member_checkboxes(stored, selected);
    view! {
        <div class=CARD_PAD>
            <input
                type="text"
                class=format!("{INPUT} w-full mb-3")
                placeholder="group name"
                prop:value=move || name.get()
                on:input:target=move |ev| name.set(ev.target().value())
            />
            <div class="text-xs font-medium text-ink-muted mb-1">"Members"</div>
            <div class="max-h-48 overflow-y-auto mb-3">{checkboxes}</div>
            <div class="flex items-center gap-2">
                <button type="button" class=BTN_PRIMARY disabled=saving on:click=on_save>
                    "Save group"
                </button>
                <button type="button" class=BTN_SECONDARY on:click=on_clear>
                    "Clear"
                </button>
            </div>
        </div>
    }
    .into_any()
}

/// The member checkbox list, read under `<Suspense>` (the same stored-query
/// load as the table).
fn member_checkboxes(
    stored: Resource<Result<Vec<StoredQueryRow>, AdminUiError>>,
    selected: RwSignal<Vec<String>>,
) -> AnyView {
    view! {
        <Suspense fallback=|| {
            view! { <p class="text-xs text-ink-muted">"Loading queries…"</p> }
        }>
            {move || Suspend::new(async move {
                match stored.await {
                    Ok(rows) => checkbox_list(&rows, selected),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// Render one checkbox per stored query (or an empty hint).
fn checkbox_list(rows: &[StoredQueryRow], selected: RwSignal<Vec<String>>) -> AnyView {
    if rows.is_empty() {
        return view! { <p class="text-xs text-ink-muted">"No stored queries to add."</p> }
            .into_any();
    }
    let items = rows
        .iter()
        .map(|row| member_checkbox(row, selected))
        .collect::<Vec<_>>();
    view! { <div class="flex flex-col gap-1">{items}</div> }.into_any()
}

/// One member checkbox: a controlled checkbox whose `checked` reflects the
/// selected set and whose `on:change` toggles the `name@version` member.
fn member_checkbox(row: &StoredQueryRow, selected: RwSignal<Vec<String>>) -> AnyView {
    let member = format!("{}@{}", row.name, row.version);
    let member_for_check = member.clone();
    let member_for_toggle = member.clone();
    let checked = move || selected.with(|set| set.iter().any(|m| m == &member_for_check));
    let on_change = move |_| {
        let member = member_for_toggle.clone();
        selected.update(|set| {
            if let Some(pos) = set.iter().position(|m| m == &member) {
                set.remove(pos);
            } else {
                set.push(member);
            }
        });
    };
    view! {
        <label class="flex items-center gap-2 text-sm text-ink">
            <input type="checkbox" class="accent-accent" prop:checked=checked on:change=on_change />
            <span class="font-mono">{member}</span>
        </label>
    }
    .into_any()
}

/// The existing groups, read under `<Transition>` (refetched on save).
fn groups_list(
    groups: Resource<Result<Vec<QueryGroup>, AdminUiError>>,
    name: RwSignal<String>,
    selected: RwSignal<Vec<String>>,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    view! {
        <Transition fallback=|| {
            view! { <p class="text-sm text-ink-muted">"Loading groups…"</p> }
        }>
            {move || Suspend::new(async move {
                match groups.await {
                    Ok(loaded) => groups_list_view(loaded, name, selected, pending_delete),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// Render the group cards (or the empty state).
fn groups_list_view(
    groups: Vec<QueryGroup>,
    name: RwSignal<String>,
    selected: RwSignal<Vec<String>>,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    if groups.is_empty() {
        return view! {
            <EmptyState
                icon=icondata_lu::LuInbox
                message="No groups yet"
                hint="Name a group and tick the stored queries to include, then save."
            />
        }
        .into_any();
    }
    let cards = groups
        .into_iter()
        .map(|group| group_card(&group, name, selected, pending_delete))
        .collect::<Vec<_>>();
    view! { <div class="flex flex-col gap-2">{cards}</div> }.into_any()
}

/// One group card: the name, member chips, an Edit button (loads the group into
/// the form) and a "Remove group" button that opens the panel's confirmation
/// modal for THIS group (the dialog dispatches the removal — a [`save_group`]
/// with no members).
///
/// The label says *group* on purpose: this removes only the console-local
/// grouping — the stored queries themselves stay in the CDR, which is what the
/// stored-query rows' "Delete from CDR" does instead.
fn group_card(
    group: &QueryGroup,
    name: RwSignal<String>,
    selected: RwSignal<Vec<String>>,
    pending_delete: RwSignal<Option<String>>,
) -> AnyView {
    let chips = group
        .members
        .iter()
        .map(|member| {
            let (label, version) =
                split_query_ref(member).unwrap_or_else(|| (member.clone(), String::new()));
            view! {
                <span class="inline-flex items-center rounded-control bg-accent-subtle px-2 py-0.5 text-xs text-accent-ink">
                    <span class="font-mono">{label}</span>
                    {(!version.is_empty())
                        .then(|| view! { <span class="opacity-60">" @"{version}</span> })}
                </span>
            }
        })
        .collect::<Vec<_>>();

    let group_name = group.name.clone();
    let name_for_edit = group.name.clone();
    let members_for_edit = group.members.clone();
    let on_edit = move |_| {
        name.set(name_for_edit.clone());
        selected.set(members_for_edit.clone());
    };

    let name_for_delete = group.name.clone();
    let on_delete = move |_| pending_delete.set(Some(name_for_delete.clone()));

    view! {
        <div class=CARD_PAD>
            <div class="flex items-center justify-between mb-2">
                <span class="font-medium text-ink">{group_name}</span>
                <div class="flex gap-2">
                    <button type="button" class=BTN_SECONDARY on:click=on_edit>
                        "Edit"
                    </button>
                    <button
                        type="button"
                        class=BTN_DANGER
                        data-group-remove=group.name.clone()
                        on:click=on_delete
                    >
                        "Remove group"
                    </button>
                </div>
            </div>
            <div class="flex flex-wrap gap-1">{chips}</div>
        </div>
    }
    .into_any()
}

/// The groups panel's ONE removal-confirmation modal, driven by
/// `pending_delete` (which group card triggered it). Its copy is explicit that
/// only the console-local grouping goes — the member queries stay in the CDR.
fn group_remove_dialog(
    pending_delete: RwSignal<Option<String>>,
    save: Action<(String, Vec<String>), Result<(), AdminUiError>>,
    save_intent: RwSignal<&'static str>,
) -> AnyView {
    let message = Signal::derive(move || {
        pending_delete.get().map_or_else(String::new, |group| {
            format!(
                "Remove the query group “{group}” from this console? The stored queries it \
                 grouped stay in the CDR — only the grouping (and its dashboard tile) goes."
            )
        })
    });
    view! {
        <crate::components::confirm_dialog::ConfirmDialog
            open=Signal::derive(move || pending_delete.get().is_some())
            title="Remove query group"
            message=message
            confirm_label="Remove group"
            confirm_id="group-remove-confirm"
            on_cancel=Callback::new(move |()| pending_delete.set(None))
            on_confirm=Callback::new(move |()| {
                if let Some(group) = pending_delete.get_untracked() {
                    save_intent.set("delete");
                    save.dispatch((group, Vec::new()));
                }
                pending_delete.set(None);
            })
        />
    }
    .into_any()
}
