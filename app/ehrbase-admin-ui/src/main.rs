//! The admin-console server binary: config → CDR client → (optional) OIDC
//! discovery → session layer → axum router (Leptos SSR + the two OIDC
//! routes). Wiring only — logic lives in the lib.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::Extension;
    use leptos::prelude::LeptosOptions;
    use leptos_axum::LeptosRoutes;

    // Console logs: env-filtered (RUST_LOG), stderr. Without a subscriber
    // every server-side error is invisible — an operational defect.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // `healthcheck` subcommand: the distroless image has no shell, so the
    // container healthcheck is the binary probing its own /login over HTTP
    // (exit 0 on 2xx/3xx). The bind address comes from LEPTOS_SITE_ADDR,
    // matching the serving path below.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        let addr =
            std::env::var("LEPTOS_SITE_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
        let addr = addr.replace("0.0.0.0", "127.0.0.1");
        let status = reqwest::Client::new()
            .get(format!("http://{addr}/login"))
            .send()
            .await?
            .status();
        anyhow::ensure!(
            status.is_success() || status.is_redirection(),
            "healthcheck: /login answered {status}"
        );
        return Ok(());
    }

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
            // Lax, not the Strict default: the OIDC callback arrives as a
            // top-level cross-site redirect from the identity provider, and
            // Strict would withhold the session cookie holding the PKCE/state
            // — the flow would always fail "no login in progress". CSRF on
            // the callback is covered by the state + PKCE checks.
            .with_same_site(tower_sessions::cookie::SameSite::Lax)
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
        // Result export: a plain form-POST download (no WASM required); the
        // handler enforces the console session itself like every server fn.
        .route(
            "/export/aql",
            axum::routing::post(ehrbase_admin_ui::export::export_aql),
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
