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
//! answer are console-local state (banned outright) or the CDR's dev-only
//! override header, which in production is an authorization bypass. So the
//! context card DISPLAYS and nothing here selects.

use leptos::component;
use leptos::prelude::*;
use leptos_meta::Title;

use crate::components::confirm_dialog::delete_confirmation;
use crate::components::data_table::{
    CELL, CELL_MONO, ROW, TablePaging, paged_table, paging_from_url, table_skeleton,
};
use crate::components::empty_state::EmptyState;
use crate::components::field::{BTN_DANGER, BTN_PRIMARY, BTN_SECONDARY, text_field};
use crate::components::notice::inline_error;
use crate::components::notice::{alert_note, failure_bar};
use crate::components::page_header::PageHeader;
use crate::components::surface::{CARD_PAD, CARD_TITLE};
use crate::components::toast::toast_outcome;
use crate::error::AdminUiError;
use crate::tenants::{
    CurrentTenant, TenantRow, context_line, create_tenant, delete_tenant, draft_is_complete,
    fetch_current_tenant, list_tenants, tenant_failure_copy, update_tenant,
};

/// The registry listing: `None` = the CDR does not serve the tenancy extension.
type Registry = Resource<Result<Option<Vec<TenantRow>>, AdminUiError>>;

/// The create action: the name it was dispatched with, paired with the CDR's
/// answer, so both toasts name the exact tenant (the action's value IS the
/// mutation report).
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
/// to fill a form.
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
    // suspense that fetches the rows).
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
    // so the answer is handled where it arrives.
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
/// rule); the CDR's diagnostic ALSO stays inline beside each form, because
/// a `400`/`409` on a registry write is worth reading in full.
fn mutation_toasts(
    toaster: thaw::ToasterInjection,
    create: CreateAction,
    update: UpdateAction,
    delete: DeleteAction,
) {
    toast_outcome(
        toaster,
        create.value().into(),
        ("Tenant registered", "Registration failed"),
        |name, row: &TenantRow| format!("{name} was registered with system_id {}.", row.system_id),
        write_failure,
    );
    toast_outcome(
        toaster,
        update.value().into(),
        ("Tenant updated", "Update failed"),
        |name, row: &TenantRow| format!("{name} now carries system_id {}.", row.system_id),
        write_failure,
    );
    toast_outcome(
        toaster,
        delete.value().into(),
        ("Tenant deleted", "Delete failed"),
        |name, ()| format!("{name} was removed from the registry."),
        write_failure,
    );
}

/// The actionable copy every refused registry write is reported with, naming
/// the tenant the operator was working on.
fn write_failure(name: &str, error: &AdminUiError) -> String {
    tenant_failure_copy(&format!("tenant `{name}`"), error)
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
                "Tenancy is derived from the credential the request carries. The console displays \
                 the resolved tenant and never selects one: signing in with a different credential \
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
/// alone can leave a control stuck at whatever was serialized.
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
                {text_field("tenant-create-name".to_owned(), "Name", Some("acme"), name)}
                {text_field(
                    "tenant-create-system-id".to_owned(),
                    "System ID",
                    Some("acme.example.org"),
                    system_id,
                )}
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
                "The name is unique across the registry and is what a credential's tenant claim \
                 resolves by; the system_id is the openEHR system identifier the tenant's data is \
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
/// server and client alike).
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
                    {text_field("tenant-edit-name".to_owned(), "Name", None, editor.name)}
                    {text_field(
                        "tenant-edit-system-id".to_owned(),
                        "System ID",
                        None,
                        editor.system_id,
                    )}
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

/// The registry table: the listing read under `<Transition>` (keep the current
/// rows visible while a refetch runs), resolving its `Result` inside the
/// transition, then the paged rows.
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
/// "forbidden" alone tells the reader nothing about what to do next. A `401` —
/// the credential itself not accepted — lands here too, with its own next
/// action. Capability is not authorization: the screen renders because the
/// surface EXISTS, and this is where the per-request refusal lands.
fn read_error(error: &AdminUiError, object: &str) -> AnyView {
    match error {
        AdminUiError::CdrUnauthorized(_) | AdminUiError::Forbidden(_) => {
            alert_note("tenant-refused", tenant_failure_copy(object, error))
        }
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
/// without re-running the suspense that fetched them, and the `<For>` key
/// is the registry id — stable and data-derived, never an index.
fn rows_view(
    rows: Vec<TenantRow>,
    paging: TablePaging,
    editor: Editor,
    pending_delete: RwSignal<Option<TenantRow>>,
    delete: DeleteAction,
) -> AnyView {
    paged_table(
        rows,
        paging,
        "/tenants",
        "tenants",
        &["Name", "System ID", "Tenant ID", "Created", ""],
        |row: &TenantRow| row.id.clone(),
        move |row| row_view(row, editor, pending_delete, delete),
    )
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
    delete_confirmation(
        pending_delete,
        delete,
        "Delete tenant",
        "Delete tenant",
        "tenant-delete-confirm",
        message,
    )
}
