// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The root Leptos application: the HTML document shell, the theme provider,
//! and the route tree.
//!
//! `/login` is public; every other screen is nested under the session-guarded
//! [`crate::pages::shell::AppShell`] layout, which renders the matched child
//! through its `<Outlet/>`.
//!
//! Every routed view sets its own `<Title/>`.

use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

/// The full HTML document shell rendered by the server.
#[must_use]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        // The console ships one locale, and its copy is written in English, so
        // `en` is the honest declaration rather than a placeholder — an
        // assistive technology must be told which language it is reading.
        // TODO(#300): drive lang from the active locale when a second locale lands.
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// The root component: meta context, the `thaw` theme/config provider, and the
/// router. The `theme_id` is fixed (never the component's default random
/// UUID) so the server pass and client hydration emit an identical
/// `data-thaw-id` and generated-style selector — a hydration-determinism
/// requirement (rules §8).
#[expect(
    clippy::must_use_candidate,
    reason = "#[component] rewrites the fn; view!/mount always consumes the value"
)]
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    // Hydration-completion marker: effects run only in the browser (Leptos
    // book, reactivity/14), so `data-hydrated` lands on `<body>` exactly when
    // the client runtime is live and hydrated listeners exist. The E2E
    // harness waits on it before driving hydration-dependent controls — a
    // file set on the upload input BEFORE its listener exists fires no later
    // event (#2285). Set post-hydration, it cannot mismatch the SSR document.
    Effect::new(|_| {
        if let Some(body) = document().body() {
            drop(body.set_attribute("data-hydrated", "true"));
        }
    });
    view! {
        <Stylesheet id="leptos" href="/pkg/ferroehr-admin-ui.css" />
        <Title text="ferroehr-admin" />
        <thaw::ConfigProvider
            theme_id="ferroehr-admin".to_owned()
            theme=RwSignal::new(crate::theme::console_light())
        >
            <Router>
                <Routes fallback=|| view! { <NotFound /> }>
                    // NOTE: /login uses SsrMode::Async so the sign-in form
                    // works with JavaScript disabled (Leptos book, ssr/23
                    // "Async Rendering": "Works if JavaScript is disabled").
                    <Route
                        path=path!("/login")
                        view=crate::pages::login::LoginPage
                        ssr=leptos_router::SsrMode::Async
                    />
                    // NOTE: every authenticated screen uses SsrMode::Async on
                    // the PARENT route — streamed fragments race WASM init and
                    // crash hydration (Leptos book, ssr/23 "Async Rendering").
                    <ParentRoute
                        path=path!("")
                        view=crate::pages::shell::AppShell
                        ssr=leptos_router::SsrMode::Async
                    >
                        <Route path=path!("") view=crate::pages::dashboard::DashboardPage />
                        <Route
                            path=path!("templates")
                            view=crate::pages::templates::TemplatesPage
                        />
                        <Route
                            path=path!("templates/:template_id")
                            view=crate::pages::template_detail::TemplateDetailPage
                        />
                        <Route path=path!("queries") view=crate::pages::queries::QueriesPage />
                        <Route
                            path=path!("queries/builder")
                            view=crate::pages::query_builder::QueryBuilderPage
                        />
                        <Route
                            path=path!("queries/aql")
                            view=crate::pages::query_aql::QueryAqlPage
                        />
                        <Route
                            path=path!("queries/stored")
                            view=crate::pages::query_stored::QueryStoredPage
                        />
                        <Route path=path!("ehrs") view=crate::pages::ehrs::EhrsPage />
                        <Route
                            path=path!("ehrs/:ehr_id")
                            view=crate::pages::ehr_detail::EhrDetailPage
                        />
                        <Route
                            path=path!("ehrs/:ehr_id/compositions/:uid")
                            view=crate::pages::composition::CompositionPage
                        />
                        // The demographic routes are declared BEFORE the
                        // `:kind` ones on purpose: leptos_router matches a
                        // route tuple in declaration order and takes the first
                        // hit, so `relationship`/`contribution` would otherwise
                        // be read as a party kind.
                        <Route
                            path=path!("demographics")
                            view=crate::pages::demographics::browse::DemographicsPage
                        />
                        <Route
                            path=path!("demographics/relationship")
                            view=crate::pages::demographics::relationship::RelationshipsPage
                        />
                        <Route
                            path=path!("demographics/relationship/:uid")
                            view=crate::pages::demographics::relationship::RelationshipDetailPage
                        />
                        <Route
                            path=path!("demographics/contribution/:uid")
                            view=crate::pages::demographics::contribution::DemographicContributionPage
                        />
                        <Route
                            path=path!("demographics/:kind")
                            view=crate::pages::demographics::browse::PartyBrowserPage
                        />
                        <Route
                            path=path!("demographics/:kind/:uid")
                            view=crate::pages::demographics::party::PartyDetailPage
                        />
                        <Route
                            path=path!("terminology")
                            view=crate::pages::terminology::TerminologyPage
                        />
                        <Route path=path!("audit") view=crate::pages::audit::AuditPage />
                        <Route path=path!("system") view=crate::pages::system::SystemPage />
                        <Route
                            path=path!("operations")
                            view=crate::pages::operations::OperationsPage
                        />
                    </ParentRoute>
                </Routes>
            </Router>
        </thaw::ConfigProvider>
    }
}

/// The router's real 404 fallback: sets a distinct title and offers a link
/// back to the dashboard.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <Title text="Not found · ferroehr-admin" />
        <div class="p-6">
            <thaw::Card>
                <thaw::CardHeader>
                    <thaw::Body1>"Page not found"</thaw::Body1>
                </thaw::CardHeader>
                <p class="text-sm opacity-70">
                    "The page you requested does not exist. "
                    <leptos_router::components::A href="/" attr:class="underline">
                        "Return to the dashboard."
                    </leptos_router::components::A>
                </p>
            </thaw::Card>
        </div>
    }
}
