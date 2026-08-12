// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

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
    // One context closure for both render entry points (the routed app and the
    // 404 shell): the app state plus this request's CSP nonce.
    let provide_console_context = move || {
        leptos::context::provide_context(context_state.clone());
        provide_request_nonce();
    };
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
        .leptos_routes_with_context(&leptos_options, routes, provide_console_context.clone(), {
            let options = leptos_options.clone();
            move || ferroehr_admin_ui::app::shell(options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler_with_context(
            provide_console_context,
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
/// - `script-src` carries no inline allowance: the one inline script the
///   console emits is Leptos's hydration bootstrap, and it is authorized by a
///   per-request nonce ([`csp_nonce_layer`]) instead.
/// - `style-src 'unsafe-inline'` stays, and it is the honest remainder of this
///   policy rather than an oversight. `thaw` mounts its generated stylesheets
///   in the browser by creating `<style>` elements through the DOM with no
///   nonce attribute (`thaw_utils::dom::mount_style`), and both `thaw` and
///   `leptos-chartistry` render `style="…"` attributes, which `style-src-attr`
///   inherits. Adding a nonce here would make it strictly WORSE, not better:
///   CSP Level 3 says a nonce in a directive causes `'unsafe-inline'` in that
///   same directive to be ignored, so a nonced `style-src` would block every
///   one of those and leave the console unstyled.
/// - `X-Frame-Options: DENY` plus `frame-ancestors 'none'` — belt and braces
///   against clickjacking a console that performs administrative writes.
/// - `Cache-Control: no-store` — the console renders patient data into HTML,
///   and it is also what keeps a nonced document out of any shared cache.
///
/// `Strict-Transport-Security` is left to the TLS edge, for the same reason as
/// on the API: RFC 6797 §7.2 makes it inert over plain HTTP.
#[cfg(feature = "ssr")]
fn with_security_headers(
    router: axum::Router<leptos::prelude::LeptosOptions>,
) -> axum::Router<leptos::prelude::LeptosOptions> {
    use tower_http::set_header::SetResponseHeaderLayer;
    router
        .layer(axum::middleware::from_fn(csp_nonce_layer))
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

/// Mints this response's script nonce and answers with the policy that names
/// it.
///
/// The nonce is minted here and nowhere else: the layer puts it in the request
/// extensions on the way in, so the renderer stamps the same value on the
/// bootstrap script ([`provide_request_nonce`]), and writes the header on the
/// way out. A fresh value per request is the whole point — a nonce reused
/// across responses is guessable, and therefore worth no more than
/// `'unsafe-inline'`.
#[cfg(feature = "ssr")]
async fn csp_nonce_layer(
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let nonce = leptos::nonce::Nonce::new();
    #[expect(
        clippy::expect_used,
        reason = "console_csp interpolates a base64url nonce into an ASCII template, so every byte is a legal header-value character; falling back to no policy would silently ship the console unprotected"
    )]
    let policy = http::HeaderValue::from_str(&console_csp(&nonce))
        .expect("an ASCII policy string should be a valid header value");
    request.extensions_mut().insert(nonce);
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(http::header::CONTENT_SECURITY_POLICY, policy);
    response
}

/// Re-provides this request's nonce into the Leptos render context.
///
/// `leptos_axum` provides a nonce of its own before the per-request context
/// closure runs, and that one is not the value the response header authorizes.
/// Overriding it here is what makes the two the same string; without it the
/// bootstrap script carries a nonce the browser has never been told to trust,
/// and the console renders but never hydrates.
#[cfg(feature = "ssr")]
fn provide_request_nonce() {
    let Some(parts) = leptos::context::use_context::<http::request::Parts>() else {
        return;
    };
    if let Some(nonce) = parts.extensions.get::<leptos::nonce::Nonce>() {
        leptos::context::provide_context(nonce.clone());
    }
}

/// The console's Content-Security-Policy for one request, naming that
/// request's script nonce.
#[cfg(feature = "ssr")]
fn console_csp(nonce: &str) -> String {
    format!(
        "default-src 'self'; \
         script-src 'self' 'wasm-unsafe-eval' 'nonce-{nonce}'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data:; \
         font-src 'self'; \
         connect-src 'self'; \
         object-src 'none'; \
         base-uri 'self'; \
         form-action 'self'; \
         frame-ancestors 'none'"
    )
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::panic,
        reason = "the exchange helpers panic to report a broken fixture, which is how a test fails (Book ch11); clippy's allow-*-in-tests scoping covers only the #[test] fns themselves"
    )]

    use super::{console_csp, csp_nonce_layer, provide_request_nonce};
    use tower::util::ServiceExt;

    /// A sample policy for the directive assertions; the value stands in for a
    /// minted nonce and is deliberately unlike any other token in the string.
    const SAMPLE_NONCE: &str = "SAMPLENONCEVALUE";

    /// The response header the stub handler reports the render-context nonce
    /// through, so a test can compare it with the policy.
    const RENDERED_NONCE: &str = "x-test-rendered-nonce";

    /// Returns the named directive of `policy`, panicking if it is absent.
    fn directive<'a>(policy: &'a str, name: &str) -> &'a str {
        policy
            .split(';')
            .map(str::trim)
            .find(|d| d.starts_with(name))
            .unwrap_or_else(|| panic!("the policy must carry a {name} directive"))
    }

    /// Stands in for the Leptos render: it takes the request's `Parts` into a
    /// reactive owner the way `leptos_axum` does, runs the console's context
    /// step, and reports whatever `use_nonce` then answers.
    async fn render_nonce(request: axum::extract::Request) -> axum::response::Response {
        let (parts, _body) = request.into_parts();
        let owner = leptos::prelude::Owner::new();
        let rendered = owner.with(|| {
            leptos::context::provide_context(parts);
            provide_request_nonce();
            leptos::nonce::use_nonce().map(|nonce| nonce.to_string())
        });
        let mut response = axum::response::Response::new(axum::body::Body::empty());
        if let Some(rendered) = rendered {
            response.headers_mut().insert(
                http::HeaderName::from_static(RENDERED_NONCE),
                http::HeaderValue::from_str(&rendered).unwrap(),
            );
        }
        response
    }

    /// Drives one request through the nonce layer, returning the policy the
    /// response carries and the nonce the render context saw.
    async fn one_exchange() -> (String, String) {
        let response = axum::Router::new()
            .route("/", axum::routing::get(render_nonce))
            .layer(axum::middleware::from_fn(csp_nonce_layer))
            .oneshot(
                http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let header = |name: http::HeaderName| {
            response
                .headers()
                .get(&name)
                .unwrap_or_else(|| panic!("the response must carry {name}"))
                .to_str()
                .unwrap()
                .to_owned()
        };
        (
            header(http::header::CONTENT_SECURITY_POLICY),
            header(http::HeaderName::from_static(RENDERED_NONCE)),
        )
    }

    /// The whole point of the exercise: the value the renderer stamps on the
    /// hydration bootstrap is the value the response header authorizes. A
    /// mismatch is invisible server-side and leaves the console dead in the
    /// browser.
    #[tokio::test]
    async fn the_rendered_nonce_is_the_one_the_header_authorizes() {
        let (policy, rendered) = one_exchange().await;
        assert!(
            directive(&policy, "script-src").contains(&format!("'nonce-{rendered}'")),
            "script-src must authorize the rendered nonce {rendered}: {policy}"
        );
    }

    /// Renders the component that actually stamps the attribute, and pins that
    /// the `nonce=` it emits is the value the header authorizes. The context
    /// test above covers our half of the handoff; this covers Leptos's half, so
    /// an upgrade that stopped stamping the bootstrap script fails here instead
    /// of in a browser.
    #[test]
    fn the_bootstrap_script_carries_the_authorized_nonce() {
        let nonce = leptos::nonce::Nonce::new();
        let policy = console_csp(&nonce);
        let owner = leptos::prelude::Owner::new();
        let html = owner.with(|| {
            leptos::context::provide_context(nonce);
            let options = leptos::config::LeptosOptions::builder()
                .output_name("ferroehr-admin-ui")
                .build();
            leptos::prelude::RenderHtml::to_html(
                leptos::view! { <leptos::hydration::HydrationScripts options /> },
            )
        });
        let stamped = html
            .split_once("<script")
            .and_then(|(_, rest)| rest.split_once("nonce=\""))
            .and_then(|(_, rest)| rest.split_once('"'));
        let Some((stamped, _)) = stamped else {
            panic!("the bootstrap script must carry a nonce: {html}")
        };
        assert!(
            directive(&policy, "script-src").contains(&format!("'nonce-{stamped}'")),
            "the stamped nonce {stamped} is not the one script-src authorizes: {policy}"
        );
    }

    /// A nonce reused across responses is guessable, and then worth no more
    /// than the `'unsafe-inline'` it replaced.
    #[tokio::test]
    async fn every_response_carries_a_fresh_nonce() {
        let (first_policy, first) = one_exchange().await;
        let (second_policy, second) = one_exchange().await;
        assert_ne!(first, second, "two responses reused one nonce");
        assert_ne!(first_policy, second_policy);
    }

    /// The directives the policy must carry, each for a stated reason.
    #[test]
    fn the_policy_carries_the_audited_directives() {
        let policy = console_csp(SAMPLE_NONCE);
        for expected in [
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
                policy.contains(expected),
                "the policy must carry {expected}"
            );
        }
    }

    /// `'unsafe-eval'` is what `'wasm-unsafe-eval'` exists to avoid, so the
    /// broad form must never appear — including by someone "fixing" a WASM
    /// error with the bigger hammer.
    #[test]
    fn the_policy_never_permits_eval() {
        assert!(!console_csp(SAMPLE_NONCE).contains("'unsafe-eval'"));
    }

    /// The script half is strict: a nonce and no inline allowance. Re-adding
    /// `'unsafe-inline'` here would not merely widen the policy, it would be
    /// ignored outright — CSP Level 3 drops it from any directive that also
    /// carries a nonce — so the regression would look like a working policy.
    #[test]
    fn script_src_is_nonce_only() {
        let policy = console_csp(SAMPLE_NONCE);
        let script_src = directive(&policy, "script-src");
        assert!(script_src.contains(&format!("'nonce-{SAMPLE_NONCE}'")));
        assert!(
            !script_src.contains("'unsafe-inline'"),
            "the nonce replaces the inline allowance: {script_src}"
        );
    }

    /// The style half keeps its inline allowance and must never gain a nonce:
    /// `thaw` creates its stylesheets through the DOM without one, so a nonced
    /// `style-src` would suppress `'unsafe-inline'` and unstyle the console.
    #[test]
    fn style_src_keeps_its_inline_allowance_and_no_nonce() {
        let policy = console_csp(SAMPLE_NONCE);
        let style_src = directive(&policy, "style-src");
        assert!(style_src.contains("'unsafe-inline'"));
        assert!(
            !style_src.contains("'nonce-"),
            "a nonce here would disable 'unsafe-inline': {style_src}"
        );
    }
}
