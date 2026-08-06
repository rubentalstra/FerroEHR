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
        #[expect(
            clippy::disallowed_methods,
            reason = "LEPTOS_SITE_ADDR is cargo-leptos's own variable, read here for the same reason leptos::config::get_configuration reads it below — the probe must hit the address this process serves on"
        )]
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
        std::sync::Arc::new(ferroehr_admin_ui::config::load().map_err(|e| anyhow::anyhow!("{e}"))?);
    let cdr =
        ferroehr_admin_ui::cdr::CdrClient::new(&config.cdr).map_err(|e| anyhow::anyhow!("{e}"))?;
    let oidc = if config.auth.oidc.enabled {
        Some(std::sync::Arc::new(
            ferroehr_admin_ui::oidc::discover(&config.auth.oidc).await?,
        ))
    } else {
        None
    };
    let app_state = ferroehr_admin_ui::state::AppState {
        config: std::sync::Arc::clone(&config),
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
    let routes = leptos_axum::generate_route_list(ferroehr_admin_ui::app::App);

    let context_state = app_state.clone();
    let service = axum::Router::new()
        .route(
            "/auth/oidc/login",
            axum::routing::get(ferroehr_admin_ui::oidc::login),
        )
        .route(
            "/auth/oidc/callback",
            axum::routing::get(ferroehr_admin_ui::oidc::callback),
        )
        // Result export: a plain form-POST download (no WASM required); the
        // handler enforces the console session itself like every server fn.
        .route(
            "/export/aql",
            axum::routing::post(ferroehr_admin_ui::export::export_aql),
        )
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || leptos::context::provide_context(context_state.clone()),
            {
                let options = leptos_options.clone();
                move || ferroehr_admin_ui::app::shell(options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(
            ferroehr_admin_ui::app::shell,
        ))
        .layer(Extension(app_state))
        .layer(session_layer);
    let service = with_security_headers(service).with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "ferroehr-admin-ui listening");
    axum::serve(listener, service.into_make_service()).await?;
    Ok(())
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // The binary is only meaningful with `ssr`; the WASM client uses
    // `lib.rs::hydrate`. cargo-leptos always builds the bin with `ssr`.
}

/// The browser security-header set for the console, per the OWASP HTTP Headers
/// Cheat Sheet.
///
/// This is a stricter problem than the API's: the console serves HTML, hydrates
/// WebAssembly, and holds a session cookie, so a policy here has to survive
/// contact with a real hydrating application.
///
/// - `Content-Security-Policy` — `'wasm-unsafe-eval'` is required, and is the
///   narrow modern replacement for `'unsafe-eval'`: it permits WebAssembly
///   compilation without permitting `eval` of JavaScript. `connect-src 'self'`
///   is correct because the console reaches the CDR through its own server
///   functions, never the browser calling the CDR directly (the BFF boundary in
///   `.claude/rules/leptos-ui.md`), so a policy that forbids cross-origin
///   connections costs nothing and forecloses exfiltration.
/// - `'unsafe-inline'` is present for both scripts and styles, and it is the
///   honest limitation of this policy rather than an oversight: Leptos emits its
///   hydration bootstrap as an inline script and `thaw` injects generated
///   `<style>` elements. The strict form is a per-request nonce, which Leptos
///   supports — that work is tracked separately because a blocked bootstrap
///   script means a console that renders and never hydrates, so it needs
///   browser verification rather than a plausible-looking header.
/// - `X-Frame-Options: DENY` plus `frame-ancestors 'none'` — belt and braces
///   against clickjacking a console that performs administrative writes.
/// - `Cache-Control: no-store` — the console renders patient data into HTML.
///
/// `Strict-Transport-Security` is left to the TLS edge, for the same reason as
/// on the API: RFC 6797 §7.2 makes it inert over plain HTTP.
#[cfg(feature = "ssr")]
fn with_security_headers(
    router: axum::Router<leptos::prelude::LeptosOptions>,
) -> axum::Router<leptos::prelude::LeptosOptions> {
    use tower_http::set_header::SetResponseHeaderLayer;
    router
        .layer(SetResponseHeaderLayer::overriding(
            http::header::CONTENT_SECURITY_POLICY,
            http::HeaderValue::from_static(CONSOLE_CSP),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            http::header::X_CONTENT_TYPE_OPTIONS,
            http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            http::header::REFERRER_POLICY,
            http::HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            http::header::X_FRAME_OPTIONS,
            http::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            http::header::CACHE_CONTROL,
            http::HeaderValue::from_static("no-store"),
        ))
}

/// The console's Content-Security-Policy.
#[cfg(feature = "ssr")]
const CONSOLE_CSP: &str = "default-src 'self'; \
     script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline'; \
     style-src 'self' 'unsafe-inline'; \
     img-src 'self' data:; \
     font-src 'self'; \
     connect-src 'self'; \
     object-src 'none'; \
     base-uri 'self'; \
     form-action 'self'; \
     frame-ancestors 'none'";

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::CONSOLE_CSP;

    /// The directives the policy must carry, each for a stated reason.
    #[test]
    fn the_policy_carries_the_audited_directives() {
        for directive in [
            "default-src 'self'",
            // WebAssembly compilation, without permitting eval of JavaScript.
            "'wasm-unsafe-eval'",
            // The console reaches the CDR through its own server functions, so
            // the browser never needs a cross-origin connection.
            "connect-src 'self'",
            "object-src 'none'",
            "base-uri 'self'",
            "frame-ancestors 'none'",
        ] {
            assert!(
                CONSOLE_CSP.contains(directive),
                "the policy must carry {directive}"
            );
        }
    }

    /// `'unsafe-eval'` is what `'wasm-unsafe-eval'` exists to avoid, so the
    /// broad form must never appear — including by someone "fixing" a WASM
    /// error with the bigger hammer.
    #[test]
    fn the_policy_never_permits_eval() {
        assert!(!CONSOLE_CSP.contains("'unsafe-eval'"));
    }
}
