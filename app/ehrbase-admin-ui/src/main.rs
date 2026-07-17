//! The admin-console server binary: config → CDR client → (optional) OIDC
//! discovery → session layer → axum router (Leptos SSR + the two OIDC
//! routes). Wiring only — logic lives in the lib.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::Extension;
    use leptos::prelude::LeptosOptions;
    use leptos_axum::LeptosRoutes;

    let config =
        std::sync::Arc::new(ehrbase_admin_ui::config::load().map_err(|e| anyhow::anyhow!("{e}"))?);
    let cdr =
        ehrbase_admin_ui::cdr::CdrClient::new(&config.cdr).map_err(|e| anyhow::anyhow!("{e}"))?;
    let oidc = if config.auth.oidc.enabled {
        Some(std::sync::Arc::new(
            ehrbase_admin_ui::oidc::discover(&config.auth.oidc).await?,
        ))
    } else {
        None
    };
    let app_state = ehrbase_admin_ui::state::AppState {
        config: config.clone(),
        cdr,
        oidc,
    };

    let session_layer =
        tower_sessions::SessionManagerLayer::new(tower_sessions::MemoryStore::default())
            .with_secure(config.session.cookie_secure)
            .with_expiry(tower_sessions::Expiry::OnInactivity(
                tower_sessions::cookie::time::Duration::minutes(
                    i64::try_from(config.session.idle_minutes).unwrap_or(60),
                ),
            ));

    let conf = leptos::config::get_configuration(None)?;
    let leptos_options: LeptosOptions = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = leptos_axum::generate_route_list(ehrbase_admin_ui::app::App);

    let context_state = app_state.clone();
    let service = axum::Router::new()
        .route(
            "/auth/oidc/login",
            axum::routing::get(ehrbase_admin_ui::oidc::login),
        )
        .route(
            "/auth/oidc/callback",
            axum::routing::get(ehrbase_admin_ui::oidc::callback),
        )
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || leptos::context::provide_context(context_state.clone()),
            {
                let options = leptos_options.clone();
                move || ehrbase_admin_ui::app::shell(options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(
            ehrbase_admin_ui::app::shell,
        ))
        .layer(Extension(app_state))
        .layer(session_layer)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "ehrbase-admin-ui listening");
    axum::serve(listener, service.into_make_service()).await?;
    Ok(())
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // The binary is only meaningful with `ssr`; the WASM client uses
    // `lib.rs::hydrate`. cargo-leptos always builds the bin with `ssr`.
}
