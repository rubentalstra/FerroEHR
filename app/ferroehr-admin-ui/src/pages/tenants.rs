// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The `/tenants` screen — the CDR's tenant registry, plus the read-only
//! tenant context of the session looking at it.
//!
//! Two panels, one endpoint family ([`crate::tenants`], which carries the wire
//! facts): the context card says which tenant THIS session's credential
//! resolves to, and the registry table administers the rows — create, edit, and
//! a two-step delete.
//!
//! **There is no tenant switcher, and adding one is not an oversight to fix.**
//! Tenancy is credential-derived; the only ways a console could change the
//! answer are console-local state (banned outright — crate `CLAUDE.md`) or the
//! CDR's dev-only override header, which in production is an authorization
//! bypass. So the context card DISPLAYS and nothing here selects.
//!
//! Discipline (rules §0/§1/§4/§6/§8/§9): every `#[server]` fn guards the
//! session first and keeps the CDR credential server-side; the view is composed
//! from `.into_any()`-erased section locals; reads are [`Resource`]s whose
//! `Result` resolves INSIDE the `<Transition>` (an SSR'd `ErrorBoundary`
//! fallback mismatches at hydration in leptos 0.8); each mutation is an
//! [`Action`] that toasts BOTH outcomes and keeps the CDR's own diagnostic
//! inline beside the failure toast; the table is the shared [`table_shell`]
//! with its explicit `<tbody>`, paged by the shared [`table_footer`] whose page
//! state lives in the URL.

use leptos::component;
use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::data_table::{
    CELL, CELL_MONO, ROW, TablePaging, page_rows, page_window, paging_from_url, row_total,
    table_footer, table_shell, table_skeleton,
};
use crate::components::empty_state::EmptyState;
use crate::components::field::{BTN_DANGER, BTN_PRIMARY, BTN_SECONDARY, INPUT, LABEL};
use crate::components::format_view::inline_error;
use crate::components::page_header::PageHeader;
use crate::components::surface::{CARD_PAD, CARD_TITLE};
use crate::components::toast::{toast_error, toast_success};
use crate::error::AdminUiError;
use crate::tenants::{
    CurrentTenant, TenantRow, context_line, create_tenant, delete_tenant, draft_is_complete,
    fetch_current_tenant, list_tenants, tenant_failure_copy, update_tenant,
};

/// The registry listing: `None` = the CDR does not serve the tenancy extension.
type Registry = Resource<Result<Option<Vec<TenantRow>>, AdminUiError>>;

/// The create action: the name it was dispatched with, paired with the CDR's
/// answer, so both toasts name the exact tenant (rules §6 — the action's value
/// IS the mutation report).
type CreateAction = Action<(String, String), (String, Result<TenantRow, AdminUiError>)>;

/// The update action, reporting the same way.
type UpdateAction = Action<TenantEdit, (String, Result<TenantRow, AdminUiError>)>;

/// The delete action, reporting the same way.
type DeleteAction = Action<TenantRow, (String, Result<(), AdminUiError>)>;

/// One in-flight edit: which registry row, and the values to store on it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TenantEdit {
    /// The registry id of the row being edited.
    id: String,
    /// The tenant name to store.
    name: String,
    /// The `system_id` to store.
    system_id: String,
}

/// The editor's three signals: which row is open, and the two draft fields.
///
/// Seeded from the row's OWN values when its Edit button is clicked — a user
/// event, so the write happens where it arrives and no Effect reads a resource
/// to fill a form (rules §2).
#[derive(Debug, Clone, Copy)]
struct Editor {
    /// The row being edited (`None` = the editor is closed).
    target: RwSignal<Option<TenantRow>>,
    /// The name draft.
    name: RwSignal<String>,
    /// The `system_id` draft.
    system_id: RwSignal<String>,
}

impl Editor {
    /// Open the editor on `row`, seeded with the values it currently holds.
    fn open(self, row: TenantRow) {
        self.name.set(row.name.clone());
        self.system_id.set(row.system_id.clone());
        self.target.set(Some(row));
    }

    /// Close the editor and drop the draft.
    fn close(self) {
        self.target.set(None);
        self.name.set(String::new());
        self.system_id.set(String::new());
    }
}

/// The tenant-registry screen.
///
/// The whole screen is probe-and-hide at the NAV level ([`crate::tenants`]);
/// reached directly, it renders the registry when the CDR serves it and one
/// naming-the-switch card when it does not — the terminology browser's
/// disabled-surface precedent, never a page of controls that cannot work.
#[expect(
    clippy::must_use_candidate,
    reason = "#[component] rewrites the fn; view!/mount always consumes the value"
)]
#[component]
pub fn TenantsPage() -> impl IntoView {
    let toaster = thaw::ToasterInjection::expect_context();
    // The table's page window, read from the URL in SETUP (never inside the
    // suspense that fetches the rows — rules §4).
    let paging = paging_from_url();
    let editor = Editor {
        target: RwSignal::new(None),
        name: RwSignal::new(String::new()),
        system_id: RwSignal::new(String::new()),
    };
    let new_name = RwSignal::new(String::new());
    let new_system_id = RwSignal::new(String::new());
    // The row awaiting confirmation in the modal (`None` = no dialog). ONE
    // dialog serves every row — the signal is both "which row" and "open".
    let pending_delete = RwSignal::new(Option::<TenantRow>::None);

    // Each mutation clears its own form in its OWN async continuation, never
    // from an Effect reading the action's value: a dispatch is the user event,
    // so the answer is handled where it arrives (rules §2).
    let create: CreateAction = Action::new(move |draft: &(String, String)| {
        let (name, system_id) = draft.clone();
        async move {
            let outcome = create_tenant(name.clone(), system_id).await;
            if outcome.is_ok() {
                new_name.set(String::new());
                new_system_id.set(String::new());
            }
            (name, outcome)
        }
    });
    let update: UpdateAction = Action::new(move |edit: &TenantEdit| {
        let edit = edit.clone();
        async move {
            let outcome = update_tenant(edit.id, edit.name.clone(), edit.system_id).await;
            if outcome.is_ok() {
                editor.close();
            }
            (edit.name, outcome)
        }
    });
    let delete: DeleteAction = Action::new(|row: &TenantRow| {
        let row = row.clone();
        async move {
            let outcome = delete_tenant(row.id).await;
            (row.name, outcome)
        }
    });

    let registry: Registry = Resource::new(
        move || {
            (
                create.version().get(),
                update.version().get(),
                delete.version().get(),
            )
        },
        |_| async move { list_tenants().await },
    );
    // The resolved tenant follows a rename of the tenant the session resolves
    // to, so the update version is a source here as well.
    let current: Resource<Result<Option<CurrentTenant>, AdminUiError>> = Resource::new(
        move || update.version().get(),
        |_| async move { fetch_current_tenant().await },
    );

    mutation_toasts(toaster, create, update, delete);

    let context = context_card(current);
    let table = registry_table(registry, paging, editor, pending_delete, delete);
    let editor_card = edit_card(editor, update);
    let create_card = create_card(new_name, new_system_id, create);
    let confirm = delete_dialog(pending_delete, delete);
    // The delete's own diagnostic, under the table it applies to: a refused
    // delete (the reserved default tenant, a tenant that still owns data)
    // names its reason in the CDR's words, beside the failure toast.
    let delete_failure = failure_bar(Signal::derive(move || {
        delete.value().get().and_then(|(_, outcome)| outcome.err())
    }));

    view! {
        <Title text="Tenants" />
        <div id="tenants-screen" class="p-6">
            <PageHeader
                title="Tenants"
                subtitle="The CDR's tenant registry, and the tenant this console session resolves to."
            />
            {context}
            {create_card}
            {editor_card}
            {table}
            {delete_failure}
            {confirm}
        </div>
    }
}

/// Wire the screen's three mutations to their success/failure toasts.
///
/// Every mutation toasts on BOTH outcomes (the console's mutation-feedback
/// rule — crate `CLAUDE.md`); the CDR's diagnostic ALSO stays inline beside
/// each form, because a `400`/`409` on a registry write is worth reading in
/// full. Dispatching a toast is a side effect on the outside world, so an
/// Effect is its correct home (rules §2) — it never writes a signal, and it
/// never runs on the server pass.
fn mutation_toasts(
    toaster: thaw::ToasterInjection,
    create: CreateAction,
    update: UpdateAction,
    delete: DeleteAction,
) {
    Effect::new(move |_| match create.value().get() {
        Some((name, Ok(row))) => toast_success(
            toaster,
            "Tenant registered",
            &format!("{name} was registered with system_id {}.", row.system_id),
        ),
        Some((name, Err(error))) => toast_error(
            toaster,
            "Registration failed",
            &tenant_failure_copy(&format!("tenant `{name}`"), &error),
        ),
        None => {}
    });

    Effect::new(move |_| match update.value().get() {
        Some((name, Ok(row))) => toast_success(
            toaster,
            "Tenant updated",
            &format!("{name} now carries system_id {}.", row.system_id),
        ),
        Some((name, Err(error))) => toast_error(
            toaster,
            "Update failed",
            &tenant_failure_copy(&format!("tenant `{name}`"), &error),
        ),
        None => {}
    });

    Effect::new(move |_| match delete.value().get() {
        Some((name, Ok(()))) => toast_success(
            toaster,
            "Tenant deleted",
            &format!("{name} was removed from the registry."),
        ),
        Some((name, Err(error))) => toast_error(
            toaster,
            "Delete failed",
            &tenant_failure_copy(&format!("tenant `{name}`"), &error),
        ),
        None => {}
    });
}

/// The context card: which tenant this session's credential resolves to.
///
/// A pure READ, so a failure renders inline and never toasts (the console's one
/// feedback rule). The sentence itself comes from the unit-tested
/// [`context_line`], so the server pass and hydration render the same text.
fn context_card(current: Resource<Result<Option<CurrentTenant>, AdminUiError>>) -> AnyView {
    view! {
        <section id="tenant-context" class=format!("{CARD_PAD} mb-4")>
            <h2 class=CARD_TITLE>"This session's credential resolves to…"</h2>
            <Transition fallback=|| {
                view! { <span class="text-sm text-ink-faint">"resolving…"</span> }
            }>
                {move || Suspend::new(async move {
                    match current.await {
                        Ok(Some(resolved)) => {
                            view! {
                                <p id="tenant-context-value" class="text-sm text-ink">
                                    {context_line(&resolved)}
                                </p>
                            }
                                .into_any()
                        }
                        Ok(None) => {
                            view! {
                                <p class="text-sm text-ink-muted">
                                    "The CDR serves no tenancy extension, so every request runs unscoped."
                                </p>
                            }
                                .into_any()
                        }
                        Err(error) => read_error(&error, "this session's tenant"),
                    }
                })}
            </Transition>
            <p class="mt-2 text-xs text-ink-muted">
                "Tenancy is derived from the credential the request carries. The console displays
                 the resolved tenant and never selects one: signing in with a different credential
                 is the only way to work in another tenant."
            </p>
        </section>
    }
    .into_any()
}

/// The create card: the two required fields and the register button, which is
/// inert until both hold something.
///
/// The button carries a STATIC `disabled` attribute for the server HTML — the
/// form is empty on the first paint, so the control is inert before hydration
/// — with the live state on `prop:disabled`: an attribute sets the INITIAL
/// state and only a property carries the live one, so an attribute binding
/// alone can leave a control stuck at whatever was serialized (rules §2).
fn create_card(
    name: RwSignal<String>,
    system_id: RwSignal<String>,
    create: CreateAction,
) -> AnyView {
    let incomplete = Signal::derive(move || !draft_is_complete(&name.read(), &system_id.read()));
    let failure = failure_bar(Signal::derive(move || {
        create.value().get().and_then(|(_, outcome)| outcome.err())
    }));
    view! {
        <section id="tenant-create" class=format!("{CARD_PAD} mb-4")>
            <h2 class=CARD_TITLE>"Register a tenant"</h2>
            <div class="flex flex-wrap items-end gap-3">
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="tenant-create-name">
                        "Name"
                    </label>
                    <input
                        id="tenant-create-name"
                        type="text"
                        class=INPUT
                        placeholder="acme"
                        prop:value=move || name.get()
                        on:input:target=move |ev| name.set(ev.target().value())
                    />
                </div>
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="tenant-create-system-id">
                        "System ID"
                    </label>
                    <input
                        id="tenant-create-system-id"
                        type="text"
                        class=INPUT
                        placeholder="acme.example.org"
                        prop:value=move || system_id.get()
                        on:input:target=move |ev| system_id.set(ev.target().value())
                    />
                </div>
                <button
                    id="tenant-create-submit"
                    type="button"
                    class=BTN_PRIMARY
                    disabled=true
                    prop:disabled=move || incomplete.get() || create.pending().get()
                    on:click=move |_| {
                        drop(create.dispatch((name.get_untracked(), system_id.get_untracked())));
                    }
                >
                    "Register tenant"
                </button>
            </div>
            <p class="mt-2 text-xs text-ink-muted">
                "The name is unique across the registry and is what a credential's tenant claim
                 resolves by; the system_id is the openEHR system identifier the tenant's data is
                 committed under."
            </p>
            {failure}
        </section>
    }
    .into_any()
}

/// The edit card: nothing at all until a row's Edit button opens it, then that
/// row's values in two fields plus save/cancel.
///
/// Rendered ONCE outside the table, so a list refetch never re-creates it, and
/// closed on both passes of the first render (the editor signal starts empty on
/// server and client alike — rules §8).
fn edit_card(editor: Editor, update: UpdateAction) -> AnyView {
    let incomplete =
        Signal::derive(move || !draft_is_complete(&editor.name.read(), &editor.system_id.read()));
    let failure = failure_bar(Signal::derive(move || {
        update.value().get().and_then(|(_, outcome)| outcome.err())
    }));
    let on_save = move |_| {
        let Some(row) = editor.target.get_untracked() else {
            return;
        };
        drop(update.dispatch(TenantEdit {
            id: row.id,
            name: editor.name.get_untracked(),
            system_id: editor.system_id.get_untracked(),
        }));
    };
    let fields = move || {
        let Some(row) = editor.target.get() else {
            return ().into_any();
        };
        view! {
            <section id="tenant-edit" class=format!("{CARD_PAD} mb-4")>
                <h2 class=CARD_TITLE>{format!("Edit tenant {}", row.name)}</h2>
                <p class="mb-3 text-xs text-ink-muted">
                    <span class="font-medium">"Registry id: "</span>
                    <span class="font-mono break-all text-ink">{row.id}</span>
                </p>
                <div class="flex flex-wrap items-end gap-3">
                    <div class="flex flex-col gap-1">
                        <label class=LABEL r#for="tenant-edit-name">
                            "Name"
                        </label>
                        <input
                            id="tenant-edit-name"
                            type="text"
                            class=INPUT
                            prop:value=move || editor.name.get()
                            on:input:target=move |ev| editor.name.set(ev.target().value())
                        />
                    </div>
                    <div class="flex flex-col gap-1">
                        <label class=LABEL r#for="tenant-edit-system-id">
                            "System ID"
                        </label>
                        <input
                            id="tenant-edit-system-id"
                            type="text"
                            class=INPUT
                            prop:value=move || editor.system_id.get()
                            on:input:target=move |ev| editor.system_id.set(ev.target().value())
                        />
                    </div>
                    <button
                        id="tenant-edit-save"
                        type="button"
                        class=BTN_PRIMARY
                        disabled=true
                        prop:disabled=move || incomplete.get() || update.pending().get()
                        on:click=on_save
                    >
                        "Save tenant"
                    </button>
                    <button
                        id="tenant-edit-cancel"
                        type="button"
                        class=BTN_SECONDARY
                        on:click=move |_| editor.close()
                    >
                        "Cancel"
                    </button>
                </div>
            </section>
        }
        .into_any()
    };
    view! {
        {fields}
        {failure}
    }
    .into_any()
}

/// The CDR's own diagnostic for a failed write, verbatim, inline BESIDE the
/// failure toast (the console's feedback rule: a registry refusal names the
/// field or the conflict, and that is worth reading line by line).
fn failure_bar(error: Signal<Option<AdminUiError>>) -> AnyView {
    view! {
        {move || {
            error
                .get()
                .map(|error| {
                    view! {
                        <div class="mt-2">
                            <thaw::MessageBar intent=thaw::MessageBarIntent::Error>
                                <thaw::MessageBarBody>{error.to_string()}</thaw::MessageBarBody>
                            </thaw::MessageBar>
                        </div>
                    }
                })
        }}
    }
    .into_any()
}

/// The registry table: the listing read under `<Transition>` (keep the current
/// rows visible while a refetch runs — rules §6), resolving its `Result` inside
/// the transition, then the paged rows.
fn registry_table(
    registry: Registry,
    paging: TablePaging,
    editor: Editor,
    pending_delete: RwSignal<Option<TenantRow>>,
    delete: DeleteAction,
) -> AnyView {
    view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match registry.await {
                    Ok(Some(rows)) if rows.is_empty() => empty_registry(),
                    Ok(Some(rows)) => rows_view(rows, paging, editor, pending_delete, delete),
                    Ok(None) => disabled_card(),
                    Err(error) => read_error(&error, "the tenant registry"),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// A failed READ, rendered inline (never a toast — the console's one feedback
/// rule) and, for a refusal, as ACTIONABLE copy rather than the bare wire
/// error.
///
/// The registry sits under `/admin`, so the CDR's coarse RBAC classes every
/// call here as admin work: a session without the role is answered `403`, and
/// "forbidden" alone tells the reader nothing about what to do next. Capability
/// is not authorization — the screen renders because the surface EXISTS, and
/// this is where the per-request refusal lands.
fn read_error(error: &AdminUiError, object: &str) -> AnyView {
    match error {
        AdminUiError::Forbidden(_) => view! {
            <p
                id="tenant-refused"
                role="alert"
                class="rounded-control border border-danger/40 bg-danger-subtle px-3 py-2 text-sm text-danger"
            >
                {tenant_failure_copy(object, error)}
            </p>
        }
        .into_any(),
        other => inline_error(other),
    }
}

/// The whole listing when the CDR does not serve the tenancy extension.
///
/// A `404` on `GET admin/tenant` means the routes answer as if unmounted —
/// what `[tenancy] enabled = false` does — so the honest screen names the
/// switch instead of showing a registry that cannot be read or written.
fn disabled_card() -> AnyView {
    view! {
        <section id="tenants-disabled" class=CARD_PAD>
            <EmptyState
                icon=icondata_lu::LuBuilding2
                message="The tenancy extension is disabled on this server"
                hint="The CDR answers its tenant routes as if unmounted. Set enabled = true under [tenancy] in the CDR's configuration to administer tenants here."
            />
        </section>
    }
    .into_any()
}

/// The empty registry — which a served registry never actually is (the CDR
/// reserves a default tenant), so this states the fact rather than inventing an
/// action.
fn empty_registry() -> AnyView {
    view! {
        <EmptyState
            icon=icondata_lu::LuBuilding2
            message="No tenants registered"
            hint="Register the first tenant with the card above."
        />
    }
    .into_any()
}

/// Render the loaded rows: the paged table plus its footer.
///
/// The window comes from the URL, so turning the page re-renders the rows
/// without re-running the suspense that fetched them (rules §9), and the
/// `<For>` key is the registry id — stable and data-derived, never an index
/// (rules §4).
fn rows_view(
    rows: Vec<TenantRow>,
    paging: TablePaging,
    editor: Editor,
    pending_delete: RwSignal<Option<TenantRow>>,
    delete: DeleteAction,
) -> AnyView {
    let count = row_total(rows.len());
    let total = Signal::derive(move || count);
    let body = view! {
        <For
            each=move || {
                let window = page_window(total.get(), paging.page.get(), paging.size.get());
                page_rows(&rows, window)
            }
            key=|row: &TenantRow| row.id.clone()
            children=move |row| row_view(row, editor, pending_delete, delete)
        />
    }
    .into_any();
    let footer = table_footer("/tenants", "tenants", paging, total);
    view! {
        {table_shell(&["Name", "System ID", "Tenant ID", "Created", ""], body)}
        {footer}
    }
    .into_any()
}

/// One registry row: its four facts, plus the edit trigger and the delete
/// trigger (which only opens the screen's confirmation modal).
fn row_view(
    row: TenantRow,
    editor: Editor,
    pending_delete: RwSignal<Option<TenantRow>>,
    delete: DeleteAction,
) -> impl IntoView {
    let hook = row.name.clone();
    let edit_hook = row.name.clone();
    let delete_hook = row.name.clone();
    let for_edit = row.clone();
    let for_delete = row.clone();
    view! {
        <tr class=ROW data-tenant=hook>
            <td class=CELL>{row.name}</td>
            <td class=CELL_MONO data-tenant-cell="system-id">
                {row.system_id}
            </td>
            <td class=CELL_MONO data-tenant-cell="id">
                {row.id}
            </td>
            <td class=CELL_MONO>{row.created_at}</td>
            <td class=format!("{CELL} text-right")>
                <div class="flex justify-end gap-2">
                    <button
                        type="button"
                        class=BTN_SECONDARY
                        data-tenant-edit=edit_hook
                        on:click=move |_| editor.open(for_edit.clone())
                    >
                        "Edit"
                    </button>
                    <button
                        type="button"
                        class=BTN_DANGER
                        data-tenant-delete=delete_hook
                        disabled=Signal::derive(move || delete.pending().get())
                        on:click=move |_| pending_delete.set(Some(for_delete.clone()))
                    >
                        "Delete"
                    </button>
                </div>
            </td>
        </tr>
    }
}

/// The screen's ONE delete-confirmation modal, driven by `pending_delete`
/// (which row triggered it). Rendered once outside the table, so a list
/// refetch never re-creates it; it is inert while no row is pending.
fn delete_dialog(pending_delete: RwSignal<Option<TenantRow>>, delete: DeleteAction) -> AnyView {
    let message = Signal::derive(move || {
        pending_delete.get().map_or_else(String::new, |row| {
            format!(
                "Permanently remove the tenant “{}” ({}) from the registry? The CDR refuses the \
                 delete while the tenant still owns data, and the reserved default tenant can \
                 never be deleted.",
                row.name, row.system_id
            )
        })
    });
    view! {
        <ConfirmDialog
            open=Signal::derive(move || pending_delete.get().is_some())
            title="Delete tenant"
            message=message
            confirm_label="Delete tenant"
            confirm_id="tenant-delete-confirm"
            on_cancel=Callback::new(move |()| pending_delete.set(None))
            on_confirm=Callback::new(move |()| {
                if let Some(row) = pending_delete.get_untracked() {
                    drop(delete.dispatch(row));
                }
                pending_delete.set(None);
            })
        />
    }
    .into_any()
}
