//! The console's OIDC degradation contract: it boots and serves Basic login
//! while the identity provider is unreachable, and recovers OIDC without a
//! restart once the provider answers.
//!
//! The outage test uses `.invalid` hostnames — the reserved TLD guaranteed
//! never to resolve (RFC 2606 §2) — so it fails fast and a developer's own CDR
//! on port 8080 cannot make it pass or hang. The recovery test runs a local
//! stand-in provider that answers `503` once and then serves a valid discovery
//! document.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use axum::response::IntoResponse;
use ferroehr_admin_ui::cdr::CdrClient;
use ferroehr_admin_ui::config::{AdminUiConfig, AuthConfig, CdrConfig, OidcConfig, SessionConfig};
use ferroehr_admin_ui::error::AdminUiError;
use ferroehr_admin_ui::oidc::OidcState;
use ferroehr_admin_ui::server::router;
use ferroehr_admin_ui::state::AppState;
use leptos::prelude::LeptosOptions;

/// The console configuration under test: Basic enabled, OIDC enabled against
/// an issuer that cannot resolve, and a CDR that cannot resolve either.
fn outage_config() -> AdminUiConfig {
    AdminUiConfig {
        cdr: CdrConfig {
            base_url: "http://cdr.invalid:8080".to_owned(),
            ..CdrConfig::default()
        },
        auth: AuthConfig {
            basic_enabled: true,
            oidc: OidcConfig {
                enabled: true,
                issuer: "https://idp.invalid/realms/console".to_owned(),
                client_id: "ferroehr-admin-ui".to_owned(),
                public_base_url: "http://127.0.0.1:3000".to_owned(),
                ..OidcConfig::default()
            },
        },
        session: SessionConfig::default(),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::panic_in_result_fn,
    reason = "the Book's Result-returning test shape asserts by panicking (.claude/rules/testing.md §Test shapes)"
)]
async fn login_is_served_with_the_basic_form_while_the_issuer_is_unreachable()
-> Result<(), Box<dyn std::error::Error>> {
    let config = outage_config();
    // The boot path: an enabled-but-unreachable issuer must yield a usable
    // OidcState, because nothing here contacts the provider.
    let oidc = OidcState::new(&config.auth.oidc)?;
    let cdr = CdrClient::new(&config.cdr)?;
    let state = AppState {
        config: Arc::new(config),
        cdr,
        oidc: Some(Arc::new(oidc)),
    };

    let service = router(
        state,
        LeptosOptions::builder()
            .output_name("ferroehr-admin-ui")
            .build(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server =
        tokio::spawn(async move { axum::serve(listener, service.into_make_service()).await });

    let response = reqwest::Client::new()
        .get(format!("http://{addr}/login"))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    server.abort();

    assert_eq!(status, 200, "/login must be served during an IdP outage");
    // The route is SsrMode::Async, so the resolved form is in the first byte
    // of the body — no client-side fill-in and no WASM needed.
    assert!(
        body.contains(r#"id="login-username""#),
        "the Basic credential form must be present during an IdP outage: {body}"
    );
    Ok(())
}

/// A minimal OpenID Provider Metadata document (OpenID Connect Discovery 1.0
/// §3): the required members plus the endpoints the console's client reads.
fn discovery_document(issuer: &str) -> String {
    format!(
        r#"{{"issuer":"{issuer}",
            "authorization_endpoint":"{issuer}/auth",
            "token_endpoint":"{issuer}/token",
            "jwks_uri":"{issuer}/jwks",
            "response_types_supported":["code"],
            "subject_types_supported":["public"],
            "id_token_signing_alg_values_supported":["RS256"]}}"#
    )
}

#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::panic_in_result_fn,
    reason = "the Book's Result-returning test shape asserts by panicking (.claude/rules/testing.md §Test shapes)"
)]
async fn discovery_retries_after_an_outage_and_caches_once_it_succeeds()
-> Result<(), Box<dyn std::error::Error>> {
    // Bind first: the issuer must carry the port the stand-in provider listens
    // on, because discovery refuses a document whose `issuer` differs.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let issuer = format!("http://{}", listener.local_addr()?);
    let discovery_hits = Arc::new(AtomicU32::new(0));

    let idp = axum::Router::new()
        .route(
            "/.well-known/openid-configuration",
            axum::routing::get({
                let hits = Arc::clone(&discovery_hits);
                let issuer = issuer.clone();
                move || {
                    let hits = Arc::clone(&hits);
                    let issuer = issuer.clone();
                    async move {
                        if hits.fetch_add(1, Ordering::SeqCst) == 0 {
                            // The realm import is still running — exactly the
                            // composed-startup race this contract is about.
                            return (http::StatusCode::SERVICE_UNAVAILABLE, "starting up")
                                .into_response();
                        }
                        (
                            [(http::header::CONTENT_TYPE, "application/json")],
                            discovery_document(&issuer),
                        )
                            .into_response()
                    }
                }
            }),
        )
        .route(
            "/jwks",
            axum::routing::get(|| async {
                (
                    [(http::header::CONTENT_TYPE, "application/json")],
                    r#"{"keys":[]}"#,
                )
            }),
        );
    let idp_server =
        tokio::spawn(async move { axum::serve(listener, idp.into_make_service()).await });

    let oidc = OidcState::new(&OidcConfig {
        enabled: true,
        issuer: issuer.clone(),
        client_id: "ferroehr-admin-ui".to_owned(),
        public_base_url: "http://127.0.0.1:3000".to_owned(),
        ..OidcConfig::default()
    })?;

    // The provider is not ready: a typed IdP-unavailable error, nothing cached.
    let during_outage = oidc.metadata().await;
    assert!(
        matches!(during_outage, Err(AdminUiError::IdpUnavailable { .. })),
        "an unready provider must yield IdpUnavailable, got {during_outage:?}"
    );

    // The provider comes up and the SAME OidcState discovers successfully —
    // recovery without a restart, which is why failures are not cached.
    let recovered = oidc.metadata().await?;
    assert!(
        recovered.jwks_uri().as_str().ends_with("/jwks"),
        "the recovered document must be the one the provider served, got {:?}",
        recovered.jwks_uri()
    );

    // A success IS cached: no further round trip to the provider.
    let _cached = oidc.metadata().await?;
    let hits = discovery_hits.load(Ordering::SeqCst);
    idp_server.abort();
    assert_eq!(
        hits, 2,
        "discovery must retry after the failure and then be cached"
    );
    Ok(())
}
