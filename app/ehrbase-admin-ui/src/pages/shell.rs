//! The application shell: the session guard plus the persistent chrome
//! (static sidebar nav with icons, topbar with the wordmark + CDR status
//! chip, user menu + scopes drawer, dark-mode toggle, footer) around the
//! routed `<Outlet/>`.
//!
//! The shell is a layout, not a route: it is the `view` of the guarded
//! `ParentRoute` in [`crate::app`], and it renders the matched child screen
//! through `<Outlet/>`. Access is gated on a live console session — with no
//! session the guard redirects to `/login`.
//!
//! The chrome is styled with STATIC Tailwind classes (the design-system
//! tokens), never with thaw's runtime-injected widget styles: thaw CSS
//! attaches at hydration, so layout-critical chrome built on it collapses
//! into unstyled text on the pre-hydration paint (seen live in the
//! 2026-07-18 captures). thaw stays for genuinely interactive widgets
//! (the user-menu popover, the scopes drawer, toasts).

use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{Outlet, Redirect};
use leptos_use::use_interval_fn;
// The `view!` macro rejects a module-pathed name in slot position, so the
// slot component alone is imported directly (a plain import, not a rename).
use thaw::PopoverTrigger;

use crate::auth::{Logout, SessionInfo, current_session, fetch_status};
use crate::components::brand::Wordmark;

/// How often (ms) the topbar re-polls the CDR `/ehrbase/rest/status` health endpoint.
const HEALTH_POLL_MS: u64 = 30_000;

/// The browser `localStorage` key the dark-mode preference persists under.
const THEME_STORAGE_KEY: &str = "ehrbase-admin-theme";

/// Maps a full URL path to the top-level nav key it belongs under, so the
/// sidebar highlights the active section. Pure logic, unit-tested below.
fn nav_key(path: &str) -> &'static str {
    if path.starts_with("/templates") {
        "/templates"
    } else if path.starts_with("/queries") {
        "/queries"
    } else if path.starts_with("/ehrs") {
        "/ehrs"
    } else if path.starts_with("/system") {
        "/system"
    } else {
        "/"
    }
}

/// Applies the dark preference to BOTH theming systems: the `dark` class
/// on `<html>` (drives every Tailwind token) and the thaw widget theme.
/// Browser-only callers (Effect / click handlers).
fn apply_dark(theme: RwSignal<thaw::Theme>, dark: bool) {
    theme.set(if dark {
        crate::theme::console_dark()
    } else {
        crate::theme::console_light()
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
/// resource over `fetch_status`, re-polled every [`HEALTH_POLL_MS`] via
/// [`use_interval_fn`] (a no-op on the server; the effect-safe browser timer
/// pattern). Dark mode is applied through [`apply_dark`] (the `dark` root
/// class + the thaw theme together) and persists to `localStorage`; the
/// persisted choice is re-applied after hydration inside an [`Effect`],
/// keeping the initial render deterministic (rules §8).
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[allow(clippy::too_many_lines)] // resource/effect setup plus the guarded view — one cohesive component
#[component]
pub fn AppShell() -> impl IntoView {
    let session = Resource::new(|| (), |()| current_session());
    let status = Resource::new(|| (), |()| fetch_status());
    let theme = thaw::ConfigInjection::expect_context().theme;
    let is_dark = RwSignal::new(false);
    let nav_open = RwSignal::new(false);
    let scopes_open = RwSignal::new(false);
    let logout_action = ServerAction::<Logout>::new();

    // Browser-only: re-apply the persisted theme after hydration. Reads the
    // outside world (localStorage), so an Effect is the correct home (rules
    // §2/§8); it never runs on the server.
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

    view! {
        <thaw::ToasterProvider>
            <Suspense fallback=|| {
                view! {
                    <div class="min-h-screen flex items-center justify-center bg-surface">
                        <thaw::Spinner />
                    </div>
                }
            }>
                {move || {
                    Suspend::new(async move {
                        match session.await {
                            Ok(Some(info)) => {
                                authed_shell(
                                    info,
                                    status,
                                    theme,
                                    is_dark,
                                    nav_open,
                                    scopes_open,
                                    logout_action,
                                )
                            }
                            _ => view! { <Redirect path="/login" /> }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </thaw::ToasterProvider>
    }
}

/// One sidebar entry: route, label, Lucide icon.
const NAV_ITEMS: [(&str, &str, &icondata_core::IconData); 5] = [
    ("/", "Dashboard", icondata_lu::LuLayoutDashboard),
    ("/templates", "Templates", icondata_lu::LuFileCode2),
    ("/queries", "Queries", icondata_lu::LuSearchCode),
    ("/ehrs", "EHRs", icondata_lu::LuDatabase),
    ("/system", "System", icondata_lu::LuServer),
];

/// Builds the authenticated chrome (topbar, nav, footer, scopes drawer) around
/// the routed `<Outlet/>`. Split out of the component body so each section is a
/// `.into_any()`-erased local (the section-boundary erasure rule, §1) and so
/// the identity/scopes from the resolved session flow in as owned data.
#[allow(clippy::too_many_arguments)] // one flat call carrying the shell's shared reactive state; a wrapper struct would not aid clarity
#[allow(clippy::too_many_lines)] // one screen assembled from `.into_any()`-erased section locals (rules §1) — deliberately one function
fn authed_shell(
    info: SessionInfo,
    status: Resource<Result<String, crate::error::AdminUiError>>,
    theme: RwSignal<thaw::Theme>,
    is_dark: RwSignal<bool>,
    nav_open: RwSignal<bool>,
    scopes_open: RwSignal<bool>,
    logout_action: ServerAction<Logout>,
) -> AnyView {
    let SessionInfo {
        identity,
        method,
        scopes,
    } = info;
    let scope_count = scopes.len();

    // Reactive active section: follows client-side navigation too (the URL
    // is identical on the server pass and at hydration — deterministic).
    let pathname = leptos_router::hooks::use_location().pathname;
    let active = Memo::new(move |_| nav_key(&pathname.get()).to_owned());

    // <Transition>, not <Suspense>: the 30 s poll refetches the resource and
    // Transition keeps the previous chip visible during reload instead of
    // flashing the fallback every poll (leptos-ui.md §6, book async/12).
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

    let trigger_label = identity.clone();
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
                        {trigger_label}
                    </button>
                </div>
            </PopoverTrigger>
            <div class="flex flex-col gap-2 min-w-44">
                <span class="text-sm font-medium">{identity}</span>
                <span class="text-xs opacity-60">{method}</span>
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

    let scopes_body = if scopes.is_empty() {
        view! {
            <p class="text-sm opacity-70">
                "No scopes — Basic authentication grants full console access."
            </p>
        }
        .into_any()
    } else {
        let items = scopes
            .iter()
            .map(|s| view! { <li class="text-sm">{s.clone()}</li> })
            .collect_view();
        view! { <ul class="list-disc pl-5 flex flex-col gap-1">{items}</ul> }.into_any()
    };

    let scopes_drawer = view! {
        <thaw::OverlayDrawer open=scopes_open position=thaw::DrawerPosition::Right>
            <thaw::DrawerHeader>
                <thaw::DrawerHeaderTitle>"Access scopes"</thaw::DrawerHeaderTitle>
            </thaw::DrawerHeader>
            <thaw::DrawerBody>{scopes_body}</thaw::DrawerBody>
        </thaw::OverlayDrawer>
    }
    .into_any();

    // Static Tailwind nav — never thaw (the chrome must be styled on the
    // pre-hydration paint; see the module doc).
    let nav_links = NAV_ITEMS
        .into_iter()
        .map(|(href, label, icon)| {
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
        })
        .collect_view();
    let nav = view! {
        <aside
            class="w-52 shrink-0 border-r border-edge bg-raised md:block"
            class:hidden=move || !nav_open.get()
        >
            <nav aria-label="Main" class="p-3">
                <ul class="flex flex-col gap-1">{nav_links}</ul>
            </nav>
        </aside>
    }
    .into_any();

    let footer = view! {
        <footer class="flex h-10 shrink-0 items-center gap-2 border-t border-edge bg-raised px-4 text-xs text-ink-muted">
            <span>{format!("console v{}", env!("CARGO_PKG_VERSION"))}</span>
            <span>"·"</span>
            <span>{format!("{scope_count} scope(s)")}</span>
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
    use super::nav_key;

    #[test]
    fn nav_key_maps_paths_to_top_level_sections() {
        assert_eq!(nav_key("/"), "/");
        assert_eq!(nav_key("/templates"), "/templates");
        assert_eq!(nav_key("/templates/vitals.v1"), "/templates");
        assert_eq!(nav_key("/queries/builder"), "/queries");
        assert_eq!(nav_key("/ehrs/abc/compositions/xyz"), "/ehrs");
        assert_eq!(nav_key("/system"), "/system");
        assert_eq!(nav_key("/unknown"), "/");
    }
}
