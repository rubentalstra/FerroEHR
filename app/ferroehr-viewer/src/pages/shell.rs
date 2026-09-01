// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The application shell.
//!
//! The session guard plus the persistent chrome (static sidebar nav with
//! icons, topbar with the wordmark + CDR status chip, user menu + access
//! drawer, dark-mode toggle, footer) around the routed `<Outlet/>`.
//!
//! The access drawer ("View scopes") is the viewer's effective-identity
//! surface: the authenticated principal, the policy source deciding what it may
//! do, this session's scopes rendered as their parsed SMART grants, and a free
//! previewer for any scope string — all through [`crate::scopes`] over the ONE
//! master08 grammar the CDR's own gate parses with. Capability is not
//! authorization: the drawer says so, and the CDR stays the enforcer.
//!
//! The shell is a layout, not a route: it is the `view` of the guarded
//! `ParentRoute` in [`crate::app`], and it renders the matched child screen
//! through `<Outlet/>`. Access is gated on a live viewer session — with no
//! session the guard redirects to `/login`.
//!
//! The chrome is styled with STATIC Tailwind classes (the design-system
//! tokens), never with thaw's runtime-injected widget styles: thaw CSS attaches
//! at hydration, so layout-critical chrome built on it collapses into unstyled
//! text on the pre-hydration paint. thaw stays for genuinely interactive widgets
//! (the user-menu popover, the scopes drawer, toasts).

#![expect(
    clippy::disallowed_types,
    reason = "the viewer consumes the CDR JSON wire over ITS-REST — not the CDR internal seams \
              (#1694)"
)]

use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{Outlet, Redirect};
use leptos_use::use_interval_fn;
// The `view!` macro rejects a module-pathed name in slot position, so the
// slot component alone is imported directly (a plain import, not a rename).
use thaw::PopoverTrigger;

use crate::auth::{Logout, SessionInfo, current_session, fetch_status};
use crate::components::brand::Wordmark;
use crate::components::scope_grants::{ScopePreviewer, capability_note, fact_row, grant_cards};
use crate::scopes::{grants_of, policy_source};

/// How often (ms) the topbar re-polls the CDR `/ferroehr/rest/status` health endpoint.
const HEALTH_POLL_MS: u64 = 30_000;

/// The browser `localStorage` key the dark-mode preference persists under.
const THEME_STORAGE_KEY: &str = "ferroehr-viewer-theme";

/// Maps a full URL path to the top-level nav key it belongs under, so the
/// sidebar highlights the active section. Pure logic, unit-tested below.
fn nav_key(path: &str) -> &'static str {
    if path.starts_with("/templates") {
        "/templates"
    } else if path.starts_with("/queries") {
        "/queries"
    } else if path.starts_with("/ehrs") {
        "/ehrs"
    } else if path.starts_with("/demographics") {
        NAV_DEMOGRAPHICS
    } else if path.starts_with("/terminology") {
        "/terminology"
    } else if path.starts_with("/audit") {
        "/audit"
    } else if path.starts_with("/system") {
        "/system"
    } else if path.starts_with("/operations") {
        "/operations"
    } else if path.starts_with("/tenants") {
        "/tenants"
    } else if path.starts_with("/subscriptions") {
        "/subscriptions"
    } else if path.starts_with("/fhir") {
        "/fhir"
    } else {
        "/"
    }
}

/// Applies the dark preference to BOTH theming systems: the `dark` class
/// on `<html>` (drives every Tailwind token) and the thaw widget theme.
/// Browser-only callers (Effect / click handlers).
fn apply_dark(theme: RwSignal<thaw::Theme>, dark: bool) {
    theme.set(if dark {
        crate::theme::viewer_dark()
    } else {
        crate::theme::viewer_light()
    });
    if let Some(root) = document().document_element() {
        let list = root.class_list();
        let outcome = if dark {
            list.add_1("dark")
        } else {
            list.remove_1("dark")
        };
        drop(outcome); // DOM classList mutation — nothing to recover from
    }
}

/// The session-guarded application shell.
///
/// Loads the current session with a [`Resource`]; under `<Suspense>` a missing
/// session renders a [`Redirect`] to `/login`, and a present session renders
/// the full chrome + `<Outlet/>`. The topbar status chip reads a second
/// resource over `fetch_status`, re-polled every `HEALTH_POLL_MS` via
/// [`use_interval_fn`] (a no-op on the server; the effect-safe browser timer
/// pattern). Dark mode is applied through `apply_dark` (the `dark` root class
/// + the thaw theme together) and persists to `localStorage`; the persisted
/// choice is re-applied after hydration inside an [`Effect`], keeping the
/// initial render deterministic.
#[expect(
    clippy::must_use_candidate,
    reason = "#[component] rewrites the fn; view!/mount always consumes the value"
)]
#[component]
pub fn AppShell() -> impl IntoView {
    let session = Resource::new(|| (), |()| current_session());
    let status = Resource::new(|| (), |()| fetch_status());
    // The four surface probes are created HERE, in component setup — never
    // inside a Suspend closure, which re-runs and would re-create the resource.
    // They gate the operations, tenants, FHIR and subscriptions nav entries.
    let management = crate::management::management_gate();
    let tenants = crate::tenants::tenant_gate();
    let fhir = crate::fhir::fhir_gate();
    let subscriptions = crate::subscriptions::event_subscription_gate();
    let theme = thaw::ConfigInjection::expect_context().theme;
    let is_dark = RwSignal::new(false);
    let nav_open = RwSignal::new(false);
    let scopes_open = RwSignal::new(false);
    let logout_action = ServerAction::<Logout>::new();

    // Browser-only: re-apply the persisted theme after hydration. Reads the
    // outside world (localStorage), so an Effect is the correct home; it
    // never runs on the server.
    Effect::new(move |_| {
        if let Ok(Some(storage)) = window().local_storage()
            && let Ok(Some(pref)) = storage.get_item(THEME_STORAGE_KEY)
        {
            let dark = pref == "dark";
            is_dark.set(dark);
            apply_dark(theme, dark);
        }
    });

    // Browser-only: poll the CDR health endpoint on an interval.
    let _pausable = use_interval_fn(
        move || {
            status.refetch();
        },
        HEALTH_POLL_MS,
    );

    // The chrome (and the routed <Outlet/>) is created exactly ONCE, outside
    // any Suspend closure; the session gate below renders ONLY the redirect
    // decision. A Suspend closure re-runs on every notification of the
    // resources it awaits, and re-creating the Outlet re-creates every resource
    // the routed page owns, so server and client resource ids diverge and
    // hydration reads the wrong serialized slots. The chrome is a COMPONENT,
    // not a pre-built view value: component bodies run lazily at render, inside
    // the ToasterProvider's context scope.
    view! {
        <thaw::ToasterProvider>
            <Suspense fallback=|| ()>
                {move || {
                    Suspend::new(async move {
                        match session.await {
                            Ok(Some(_)) => ().into_any(),
                            _ => view! { <Redirect path="/login" /> }.into_any(),
                        }
                    })
                }}
            </Suspense>
            <AuthedChrome
                session
                status
                management
                tenants
                fhir
                subscriptions
                theme
                is_dark
                nav_open
                scopes_open
                logout_action
            />
        </thaw::ToasterProvider>
    }
}

/// The authenticated chrome as a component so its body (and the `<Outlet/>`
/// subtree) is constructed lazily under the `ToasterProvider` context.
#[component]
fn AuthedChrome(
    /// The shell's session resource (identity/scopes fragments resolve it).
    session: Resource<Result<Option<SessionInfo>, crate::error::ViewerError>>,
    /// The CDR health resource (the topbar chip).
    status: Resource<Result<String, crate::error::ViewerError>>,
    /// The management-surface probe (gates the operations nav entry).
    management: Resource<
        Result<crate::management::ManagementAvailability, crate::error::ViewerError>,
    >,
    /// The tenancy-extension probe (gates the tenants nav entry).
    tenants: Resource<Result<crate::tenants::TenantAvailability, crate::error::ViewerError>>,
    /// The FHIR-connector probe (gates the FHIR nav entry).
    fhir: Resource<Result<crate::fhir::FhirAvailability, crate::error::ViewerError>>,
    /// The event-subscription probe (gates the subscriptions nav entry).
    subscriptions: Resource<
        Result<crate::subscriptions::SubscriptionAvailability, crate::error::ViewerError>,
    >,
    /// The thaw widget theme signal.
    theme: RwSignal<thaw::Theme>,
    /// Dark-mode state.
    is_dark: RwSignal<bool>,
    /// Mobile nav visibility.
    nav_open: RwSignal<bool>,
    /// Scopes drawer visibility.
    scopes_open: RwSignal<bool>,
    /// The logout server action.
    logout_action: ServerAction<Logout>,
) -> impl IntoView {
    authed_shell(
        session,
        status,
        management,
        tenants,
        fhir,
        subscriptions,
        theme,
        is_dark,
        nav_open,
        scopes_open,
        logout_action,
    )
}

/// The demographics entry's href, which is also its [`nav_key`] value: the
/// section has no kind-agnostic landing page (every screen in it is per-kind or
/// per-object), so the nav opens the default kind's browser directly and every
/// `/demographics/…` path highlights this entry.
const NAV_DEMOGRAPHICS: &str = "/demographics/person";

/// The CDR probe deciding whether a gated sidebar entry renders at all.
///
/// Each surface is optional on the CDR side and answers `404` as if unmounted,
/// so its nav entry follows the crate's discover-and-hide pattern: a deployment
/// that does not serve the surface shows no link to a screen whose cards would
/// all read "not available".
enum NavProbe {
    /// The FHIR connector (see [`crate::fhir`]).
    Fhir,
    /// The event-subscription admin API (see [`crate::subscriptions`]).
    Subscriptions,
    /// The tenancy extension (see [`crate::tenants`]).
    Tenants,
    /// The management surface, off by default (see [`crate::management`]).
    Management,
}

/// One slot of the sidebar, in render order.
enum NavSlot {
    /// An always-present entry: route, label, Lucide icon.
    Item(&'static str, &'static str, &'static icondata_core::IconData),
    /// A probe-gated entry: the probe deciding it, then route, label, icon.
    Gated(
        NavProbe,
        &'static str,
        &'static str,
        &'static icondata_core::IconData,
    ),
    /// The hairline between the domain group and the meta group.
    Divider,
}

/// The sidebar, in the order it renders: the DOMAIN group (the clinical and
/// definitional sections a deployment's own data lives in), a divider, then the
/// META group (the platform-about-itself screens), with `/system` last.
///
/// Gated entries sit in their group rather than after the static ones, so a
/// full-featured deployment reads in the same order as a minimal one. The
/// divider belongs to the meta group, whose anchors (`Audit log`, `System`) are
/// unconditional — so no hidden entry can ever strand it or double a gap.
const NAV_SLOTS: [NavSlot; 13] = [
    NavSlot::Item("/", "Dashboard", icondata_lu::LuLayoutDashboard),
    NavSlot::Item("/templates", "Templates", icondata_lu::LuFileCode2),
    NavSlot::Item("/queries", "Queries", icondata_lu::LuSearchCode),
    NavSlot::Item("/ehrs", "EHRs", icondata_lu::LuDatabase),
    NavSlot::Item(NAV_DEMOGRAPHICS, "Demographics", icondata_lu::LuUsers),
    NavSlot::Item("/terminology", "Terminology", icondata_lu::LuBookA),
    NavSlot::Gated(NavProbe::Fhir, "/fhir", "FHIR", icondata_lu::LuPlug),
    NavSlot::Gated(
        NavProbe::Subscriptions,
        "/subscriptions",
        "Subscriptions",
        icondata_lu::LuRadioTower,
    ),
    NavSlot::Gated(
        NavProbe::Tenants,
        "/tenants",
        "Tenants",
        icondata_lu::LuBuilding2,
    ),
    NavSlot::Divider,
    NavSlot::Gated(
        NavProbe::Management,
        "/operations",
        "Operations",
        icondata_lu::LuGauge,
    ),
    NavSlot::Item("/audit", "Audit log", icondata_lu::LuShieldCheck),
    NavSlot::Item("/system", "System", icondata_lu::LuServer),
];

/// One sidebar `<li>`: the link, its active styling, and its icon. Every
/// [`NAV_SLOTS`] entry draws through it — static and probe-gated alike — so the
/// two look identical and only one place styles a nav link.
fn nav_entry(
    active: Memo<String>,
    href: &'static str,
    label: &'static str,
    icon: &'static icondata_core::IconData,
) -> impl IntoView {
    let is_active = move || active.get() == href;
    view! {
        <li>
            <a
                href=href
                aria-current=move || if is_active() { Some("page") } else { None }
                class="group flex items-center gap-2.5 rounded-control px-3 py-2 text-sm font-medium"
                class=(["bg-accent-subtle", "text-accent-ink"], is_active)
                class=(
                    ["text-ink-muted", "hover:bg-sunken", "hover:text-ink"],
                    move || !is_active(),
                )
            >
                <Icon icon width="16" height="16" />
                {label}
            </a>
        </li>
    }
}

/// The hairline `<li>` splitting the sidebar's domain group from its meta
/// group. Purely decorative — hidden from assistive technology, since the
/// grouping it draws carries no information the labels do not already give —
/// and static chrome like the rest of the nav, so it paints before hydration.
fn nav_divider() -> AnyView {
    view! { <li aria-hidden="true" class="my-1.5 border-t border-edge"></li> }.into_any()
}

/// Builds the authenticated chrome (topbar, nav, footer, scopes drawer) around
/// the routed `<Outlet/>`. Split out of the component body so each section is a
/// `.into_any()`-erased local and so the identity/scopes from the resolved
/// session flow in as owned data.
#[expect(
    clippy::too_many_arguments,
    reason = "one flat call carrying the shell's shared reactive state; a wrapper struct would not aid clarity"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one screen assembled from `.into_any()`-erased section locals (rules §1) — deliberately one function"
)]
fn authed_shell(
    session: Resource<Result<Option<SessionInfo>, crate::error::ViewerError>>,
    status: Resource<Result<String, crate::error::ViewerError>>,
    management: Resource<
        Result<crate::management::ManagementAvailability, crate::error::ViewerError>,
    >,
    tenants: Resource<Result<crate::tenants::TenantAvailability, crate::error::ViewerError>>,
    fhir: Resource<Result<crate::fhir::FhirAvailability, crate::error::ViewerError>>,
    subscriptions: Resource<
        Result<crate::subscriptions::SubscriptionAvailability, crate::error::ViewerError>,
    >,
    theme: RwSignal<thaw::Theme>,
    is_dark: RwSignal<bool>,
    nav_open: RwSignal<bool>,
    scopes_open: RwSignal<bool>,
    logout_action: ServerAction<Logout>,
) -> AnyView {
    // Reactive active section: follows client-side navigation too (the URL
    // is identical on the server pass and at hydration — deterministic).
    let pathname = leptos_router::hooks::use_location().pathname;
    let active = Memo::new(move |_| nav_key(&pathname.get()).to_owned());

    // <Transition>, not <Suspense>: the 30 s poll refetches the resource and
    // Transition keeps the previous chip visible during reload instead of
    // flashing the fallback every poll (book async/12).
    let status_chip = view! {
        <Transition fallback=|| {
            view! { <span class="text-xs text-ink-faint">"checking…"</span> }
        }>
            {move || {
                Suspend::new(async move {
                    let chip = |dot: &'static str, text_cls: &'static str, label: String| {
                        view! {
                            <span class=format!(
                                "inline-flex items-center gap-1.5 rounded-full border border-edge bg-raised px-2.5 py-1 text-xs font-medium {text_cls}",
                            )>
                                <span class=format!("h-1.5 w-1.5 rounded-full {dot}")></span>
                                {label}
                            </span>
                        }
                            .into_any()
                    };
                    match status.await {
                        Ok(body) => {
                            let doc = serde_json::from_str::<serde_json::Value>(&body).ok();
                            let version = doc
                                .as_ref()
                                .and_then(|v| v.get("server_version"))
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_owned)
                                .unwrap_or_default();
                            match doc
                                .as_ref()
                                .and_then(|v| v.get("status"))
                                .and_then(serde_json::Value::as_str)
                            {
                                Some("UP") => {
                                    chip("bg-ok", "text-ink", format!("CDR UP · v{version}"))
                                }
                                _ => chip("bg-warn", "text-ink", "CDR DEGRADED".to_owned()),
                            }
                        }
                        Err(_) => chip("bg-danger", "text-danger", "CDR DOWN".to_owned()),
                    }
                })
            }}
        </Transition>
    }
    .into_any();

    let dark_toggle = view! {
        <button
            type="button"
            aria-label="Toggle dark mode"
            class="flex h-8 w-8 items-center justify-center rounded-control text-ink-muted hover:bg-sunken hover:text-ink focus:outline-none focus:ring-2 focus:ring-accent"
            on:click=move |_| {
                let now_dark = !is_dark.get_untracked();
                is_dark.set(now_dark);
                apply_dark(theme, now_dark);
                if let Ok(Some(storage)) = window().local_storage() {
                    let stored = storage
                        .set_item(THEME_STORAGE_KEY, if now_dark { "dark" } else { "light" });
                    drop(stored);
                }
            }
        >
            {move || {
                if is_dark.get() {
                    view! { <Icon icon=icondata_lu::LuSun width="16" height="16" /> }.into_any()
                } else {
                    view! { <Icon icon=icondata_lu::LuMoon width="16" height="16" /> }.into_any()
                }
            }}
        </button>
    }
    .into_any();

    // Identity text resolves the session in a resource-free section (safe to
    // re-run — it allocates no resources, so ids cannot diverge).
    let identity_text = move || {
        view! {
            <Suspense fallback=|| ()>
                {move || {
                    Suspend::new(async move {
                        let identity = session
                            .await
                            .ok()
                            .flatten()
                            .map(|info| info.identity)
                            .unwrap_or_default();
                        view! { <span>{identity}</span> }.into_any()
                    })
                }}
            </Suspense>
        }
    };
    let method_text = move || {
        view! {
            <Suspense fallback=|| ()>
                {move || {
                    Suspend::new(async move {
                        let method = session
                            .await
                            .ok()
                            .flatten()
                            .map(|info| info.method)
                            .unwrap_or_default();
                        view! { <span>{method}</span> }.into_any()
                    })
                }}
            </Suspense>
        }
    };
    // The user menu stays a thaw widget: it needs click-outside dismissal and
    // anchored positioning, and it only opens after a click — i.e. after
    // hydration — so thaw's runtime-injected CSS is off the pre-hydration paint
    // path this module's doc warns about. `style/tailwind.css` restates
    // `.thaw-popover-surface` in the design tokens (raised background, edge
    // hairline, card radius + shadow) so every popover matches the kit.
    let user_menu = view! {
        <thaw::Popover trigger_type=thaw::PopoverTriggerType::Click>
            <PopoverTrigger slot>
                // Stable wrapper id: the E2E journeys target the user-menu
                // trigger deterministically.
                <div id="user-menu-trigger" class="inline-flex">
                    <button
                        type="button"
                        class="inline-flex items-center gap-2 rounded-control px-2 py-1.5 text-sm font-medium text-ink hover:bg-sunken focus:outline-none focus:ring-2 focus:ring-accent"
                    >
                        <Icon icon=icondata_lu::LuCircleUser width="18" height="18" />
                        {identity_text()}
                    </button>
                </div>
            </PopoverTrigger>
            <div class="flex flex-col gap-2 min-w-44">
                <span class="text-sm font-medium text-ink">{identity_text()}</span>
                <span class="text-xs text-ink-muted">{method_text()}</span>
                <thaw::Button
                    appearance=thaw::ButtonAppearance::Subtle
                    on_click=move |_| scopes_open.set(true)
                >
                    "View scopes"
                </thaw::Button>
                <ActionForm action=logout_action>
                    <thaw::Button
                        button_type=thaw::ButtonType::Submit
                        appearance=thaw::ButtonAppearance::Secondary
                        block=true
                    >
                        "Sign out"
                    </thaw::Button>
                </ActionForm>
            </div>
        </thaw::Popover>
    }
    .into_any();

    // The effective-access block: who this session is, what decides what it may
    // do, and the scopes it carries rendered as their parsed master08 grants
    // (`crate::scopes` over the ONE shared grammar). It resolves the session
    // inside its own resource-free Suspend — safe to re-run, since it allocates
    // no resources — and renders ONLY facts the session actually carries: the
    // viewer never invents a claim it was not given.
    let effective_access = view! {
        <Suspense fallback=|| ()>
            {move || {
                Suspend::new(async move {
                    let info = session.await.ok().flatten();
                    let (identity, method) = info
                        .as_ref()
                        .map(|info| (info.identity.clone(), info.method.clone()))
                        .unwrap_or_default();
                    let scopes = info.map(|info| info.scopes).unwrap_or_default();
                    let policy = policy_source(&method);
                    let scope_view = if scopes.is_empty() {
                        // Deliberately NOT an EmptyState: an empty scope list is
                        // not a void to fill but the complete answer to "what am
                        // I allowed to do" under Basic auth — the sentence IS the
                        // content, and there is no action that would add scopes
                        // from here.
                        view! {
                            <p class="text-sm text-ink-muted">
                                "No scopes — Basic authentication grants full viewer access."
                            </p>
                        }
                            .into_any()
                    } else {
                        grant_cards(grants_of(&scopes))
                    };
                    view! {
                        <div class="flex flex-col gap-3">
                            <div class="flex flex-col gap-1">
                                {fact_row("Identity", identity)}
                                {fact_row("Signed in with", policy.label.to_owned())}
                            </div>
                            <p class="text-[11px] leading-snug text-ink-muted">{policy.note}</p>
                            <div id="session-scopes" class="flex flex-col gap-2">
                                <h3 class="text-xs font-medium text-ink">
                                    "Scopes on this session"
                                </h3>
                                {scope_view}
                            </div>
                        </div>
                    }
                        .into_any()
                })
            }}
        </Suspense>
    }
    .into_any();

    let scopes_drawer = view! {
        <thaw::OverlayDrawer open=scopes_open position=thaw::DrawerPosition::Right>
            <thaw::DrawerHeader>
                <thaw::DrawerHeaderTitle>"Access scopes"</thaw::DrawerHeaderTitle>
            </thaw::DrawerHeader>
            <thaw::DrawerBody>
                // Stable id: the E2E journeys read the drawer's whole body
                // (identity, policy source, session grants, previewer).
                <div id="access-drawer" class="flex flex-col gap-4">
                    {capability_note()}
                    {effective_access}
                    <hr class="border-edge" />
                    <ScopePreviewer />
                </div>
            </thaw::DrawerBody>
        </thaw::OverlayDrawer>
    }
    .into_any();

    // Static Tailwind nav — never thaw (the chrome must be styled on the
    // pre-hydration paint; see the module doc). One pass over `NAV_SLOTS` is
    // what puts each probe-gated entry in its GROUP instead of after the static
    // ones: the table is the only place the order is declared, so the rendered
    // sidebar cannot drift from it. Each gated slot renders its own
    // discover-and-hide wrapper in place, collapsing to nothing when the CDR
    // does not serve that surface.
    let nav_slots = NAV_SLOTS
        .into_iter()
        .map(|slot| match slot {
            NavSlot::Item(href, label, icon) => nav_entry(active, href, label, icon).into_any(),
            NavSlot::Gated(NavProbe::Fhir, href, label, icon) => {
                crate::fhir::when_fhir_connector_usable(fhir, move || {
                    nav_entry(active, href, label, icon).into_any()
                })
            }
            NavSlot::Gated(NavProbe::Subscriptions, href, label, icon) => {
                crate::subscriptions::when_event_subscriptions_usable(subscriptions, move || {
                    nav_entry(active, href, label, icon).into_any()
                })
            }
            NavSlot::Gated(NavProbe::Tenants, href, label, icon) => {
                crate::tenants::when_tenant_registry_usable(tenants, move || {
                    nav_entry(active, href, label, icon).into_any()
                })
            }
            NavSlot::Gated(NavProbe::Management, href, label, icon) => {
                crate::management::when_management_usable(management, move || {
                    nav_entry(active, href, label, icon).into_any()
                })
            }
            NavSlot::Divider => nav_divider(),
        })
        .collect_view();
    let nav = view! {
        <aside
            class="w-52 shrink-0 border-r border-edge bg-raised md:block"
            class:hidden=move || !nav_open.get()
        >
            <nav aria-label="Main" class="p-3">
                <ul class="flex flex-col gap-1">{nav_slots}</ul>
            </nav>
        </aside>
    }
    .into_any();

    let footer = view! {
        <footer class="flex h-10 shrink-0 items-center gap-2 border-t border-edge bg-raised px-4 text-xs text-ink-muted">
            <span>{format!("viewer v{}", env!("CARGO_PKG_VERSION"))}</span>
            <span>"·"</span>
            <Suspense fallback=|| ()>
                {move || {
                    Suspend::new(async move {
                        let n = session
                            .await
                            .ok()
                            .flatten()
                            .map(|info| info.scopes.len())
                            .unwrap_or_default();
                        view! { <span>{format!("{n} scope(s)")}</span> }.into_any()
                    })
                }}
            </Suspense>
        </footer>
    }
    .into_any();

    let topbar = view! {
        <header class="flex h-14 shrink-0 items-center gap-3 border-b border-edge bg-raised px-4">
            <button
                type="button"
                aria-label="Toggle navigation"
                class="flex h-8 w-8 items-center justify-center rounded-control text-ink-muted hover:bg-sunken hover:text-ink md:hidden"
                on:click=move |_| nav_open.update(|open| *open = !*open)
            >
                <Icon icon=icondata_lu::LuMenu width="18" height="18" />
            </button>
            <a href="/" class="flex items-center">
                <Wordmark />
            </a>
            <div class="flex-1"></div>
            {status_chip}
            {user_menu}
            {dark_toggle}
        </header>
    }
    .into_any();

    view! {
        <div class="flex min-h-screen flex-col bg-surface text-ink">
            {topbar}
            <div class="flex min-h-0 flex-1">
                {nav} <main class="min-w-0 flex-1 overflow-auto">
                    <Outlet />
                </main>
            </div> {footer} {scopes_drawer}
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::{NAV_SLOTS, NavProbe, NavSlot, nav_key};

    /// Renders one slot as the token the order assertions read: a plain label,
    /// a label carrying the probe that gates it, or the divider's rule.
    fn slot_token(slot: &NavSlot) -> String {
        match slot {
            NavSlot::Item(_, label, _) => (*label).to_owned(),
            NavSlot::Gated(probe, _, label, _) => {
                let probe = match probe {
                    NavProbe::Fhir => "fhir",
                    NavProbe::Subscriptions => "subscriptions",
                    NavProbe::Tenants => "tenants",
                    NavProbe::Management => "management",
                };
                format!("{label} (gated: {probe})")
            }
            NavSlot::Divider => "──".to_owned(),
        }
    }

    /// The decided sidebar order (owner call, issue #2577): the domain group on
    /// top with its gated entries in place, then the divider, then the meta
    /// group with `System` last.
    #[test]
    fn sidebar_renders_domain_group_then_meta_group_with_system_last() {
        let order: Vec<String> = NAV_SLOTS.iter().map(slot_token).collect();
        assert_eq!(
            order,
            [
                "Dashboard",
                "Templates",
                "Queries",
                "EHRs",
                "Demographics",
                "Terminology",
                "FHIR (gated: fhir)",
                "Subscriptions (gated: subscriptions)",
                "Tenants (gated: tenants)",
                "──",
                "Operations (gated: management)",
                "Audit log",
                "System",
            ]
        );
    }

    /// The divider is meta-group chrome, and the meta group's anchors are
    /// unconditional — so the leanest possible deployment (no FHIR connector,
    /// no tenancy extension, no management surface) still shows the rule with
    /// content on both sides: no stray divider, no doubled gap.
    #[test]
    fn hiding_every_gated_entry_leaves_the_divider_between_two_groups() {
        let visible: Vec<String> = NAV_SLOTS
            .iter()
            .filter(|slot| !matches!(slot, NavSlot::Gated(..)))
            .map(slot_token)
            .collect();
        assert_eq!(
            visible,
            [
                "Dashboard",
                "Templates",
                "Queries",
                "EHRs",
                "Demographics",
                "Terminology",
                "──",
                "Audit log",
                "System",
            ]
        );
    }

    /// Every sidebar entry's href is also its [`nav_key`] value, so the entry
    /// the user clicked is the entry that highlights.
    #[test]
    fn every_sidebar_href_highlights_its_own_entry() {
        for slot in &NAV_SLOTS {
            let href = match slot {
                NavSlot::Item(href, _, _) | NavSlot::Gated(_, href, _, _) => *href,
                NavSlot::Divider => continue,
            };
            assert_eq!(nav_key(href), href, "{href}");
        }
    }

    #[test]
    fn nav_key_maps_paths_to_top_level_sections() {
        assert_eq!(nav_key("/"), "/");
        assert_eq!(nav_key("/templates"), "/templates");
        assert_eq!(nav_key("/templates/vitals.v1"), "/templates");
        assert_eq!(nav_key("/queries/builder"), "/queries");
        assert_eq!(nav_key("/ehrs/abc/compositions/xyz"), "/ehrs");
        // Every demographic screen — per-kind, per-party, per-relationship,
        // per-contribution — highlights the one nav entry.
        for path in [
            "/demographics",
            "/demographics/person",
            "/demographics/role/8849182c?tab=history",
            "/demographics/relationship/7d44aa01",
            "/demographics/contribution/c9",
        ] {
            assert_eq!(nav_key(path), super::NAV_DEMOGRAPHICS, "{path}");
        }
        assert_eq!(nav_key("/terminology"), "/terminology");
        assert_eq!(nav_key("/terminology?terminology=openehr"), "/terminology");
        assert_eq!(nav_key("/audit"), "/audit");
        assert_eq!(nav_key("/audit?patient=p-1"), "/audit");
        assert_eq!(nav_key("/system"), "/system");
        assert_eq!(nav_key("/operations"), "/operations");
        assert_eq!(
            nav_key("/operations?metric=aql_queries_total"),
            "/operations"
        );
        assert_eq!(nav_key("/tenants"), "/tenants");
        assert_eq!(nav_key("/tenants?page=1"), "/tenants");
        assert_eq!(nav_key("/subscriptions"), "/subscriptions");
        assert_eq!(nav_key("/subscriptions?page=1"), "/subscriptions");
        assert_eq!(nav_key("/fhir"), "/fhir");
        assert_eq!(
            nav_key("/fhir?resource_type=Observation&patient=p-42"),
            "/fhir"
        );
        assert_eq!(nav_key("/unknown"), "/");
    }
}
