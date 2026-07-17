//! The admin-console server binary: serves the SSR app + static assets and
//! hosts the BFF (server functions). Wiring only — logic lives in the lib.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use leptos::prelude::LeptosOptions;
    use leptos_axum::LeptosRoutes;

    let conf = leptos::config::get_configuration(None)?;
    let leptos_options: LeptosOptions = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = leptos_axum::generate_route_list(ehrbase_admin_ui::app::App);

    let service = axum::Router::new()
        .leptos_routes(&leptos_options, routes, {
            let options = leptos_options.clone();
            move || ehrbase_admin_ui::app::shell(options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(
            ehrbase_admin_ui::app::shell,
        ))
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
