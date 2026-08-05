//! The console's axum router: the BFF's own routes plus the Leptos SSR
//! handlers and the session layer.
//!
//! It lives in the library rather than the binary so the boot path is
//! testable — a binary-only wiring path cannot be imported from `tests/`
//! (the Rust Book's thin-`main`-over-`lib` split, ch12.3,
//! <https://doc.rust-lang.org/book/ch12-03-improving-error-handling-and-modularity.html>).

use axum::Extension;
use leptos::prelude::LeptosOptions;
use leptos_axum::LeptosRoutes;

/// Assemble the console's router: the OIDC + export routes, every Leptos
/// route with [`crate::state::AppState`] in context, the static-file
/// fallback, and the session layer.
///
/// Nothing here touches the network, so the router is servable even when the
/// CDR and the identity provider are both down — which is exactly what makes
/// the login screen reachable during an outage.
pub fn router(state: crate::state::AppState, leptos_options: LeptosOptions) -> axum::Router {
    let session_layer =
        tower_sessions::SessionManagerLayer::new(tower_sessions::MemoryStore::default())
            .with_secure(state.config.session.cookie_secure)
            // Lax, not the Strict default: the OIDC callback arrives as a
            // top-level cross-site redirect from the identity provider, and
            // Strict would withhold the session cookie holding the PKCE/state
            // — the flow would always fail "no login in progress". CSRF on
            // the callback is covered by the state + PKCE checks.
            .with_same_site(tower_sessions::cookie::SameSite::Lax)
            .with_expiry(tower_sessions::Expiry::OnInactivity(
                tower_sessions::cookie::time::Duration::minutes(
                    i64::try_from(state.config.session.idle_minutes).unwrap_or(60),
                ),
            ));

    let routes = leptos_axum::generate_route_list(crate::app::App);
    let context_state = state.clone();

    axum::Router::new()
        .route("/auth/oidc/login", axum::routing::get(crate::oidc::login))
        .route(
            "/auth/oidc/callback",
            axum::routing::get(crate::oidc::callback),
        )
        // Result export: a plain form-POST download (no WASM required); the
        // handler enforces the console session itself like every server fn.
        .route(
            "/export/aql",
            axum::routing::post(crate::export::export_aql),
        )
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || leptos::context::provide_context(context_state.clone()),
            {
                let options = leptos_options.clone();
                move || crate::app::shell(options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(crate::app::shell))
        .layer(Extension(state))
        .layer(session_layer)
        .with_state(leptos_options)
}
