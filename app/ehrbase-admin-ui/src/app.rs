//! The root Leptos application: the HTML document shell, the theme provider,
//! and the route tree. `/login` is public; every other screen is nested
//! under the session-guarded [`crate::pages::shell::AppShell`] layout, which
//! renders the matched child through its `<Outlet/>`.
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
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/ehrbase-admin-ui.css" />
        <Title text="ehrbase-admin" />
        <thaw::ConfigProvider
            theme_id="ehrbase-admin".to_owned()
            theme=RwSignal::new(crate::theme::console_light())
        >
            <Router>
                <Routes fallback=|| view! { <NotFound /> }>
                    // NOTE: /login deviates from the out-of-order streaming
                    // default: the sign-in form must work with JavaScript
                    // disabled (the ActionForm progressive-enhancement
                    // contract), and out-of-order streaming needs JS to move
                    // suspended fragments into place — SsrMode::Async sends
                    // the resolved HTML instead (Leptos book, ssr/23 "Async
                    // Rendering": "Works if JavaScript is disabled").
                    <Route
                        path=path!("/login")
                        view=crate::pages::login::LoginPage
                        ssr=leptos_router::SsrMode::Async
                    />
                    // NOTE: every authenticated screen deviates from the
                    // out-of-order streaming default. Streamed resource
                    // fragments race WASM init: the E2E console gate caught
                    // "expected a text node" hydration crashes whenever the
                    // WASM finished loading after the fragments had swapped
                    // in (the serialized-resource arrays showed a dozen
                    // resources still "pending" at hydration). Async waits
                    // server-side and sends one complete, stable document
                    // (Leptos book, ssr/23 "Async Rendering"); the mode must
                    // sit on the PARENT route — a child-route override does
                    // not take effect under a streaming parent (verified
                    // live 2026-07-18).
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
                        <Route path=path!("ehrs") view=crate::pages::ehrs::EhrsPage />
                        <Route
                            path=path!("ehrs/:ehr_id")
                            view=crate::pages::ehr_detail::EhrDetailPage
                        />
                        <Route
                            path=path!("ehrs/:ehr_id/compositions/:uid")
                            view=crate::pages::composition::CompositionPage
                        />
                        <Route path=path!("audit") view=crate::pages::audit::AuditPage />
                        <Route path=path!("system") view=crate::pages::system::SystemPage />
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
        <Title text="Not found · ehrbase-admin" />
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
