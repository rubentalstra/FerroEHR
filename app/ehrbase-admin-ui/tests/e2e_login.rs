#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    unreachable_pub
)]
// e2e journeys are assertive by design; skip-with-reason prints; the shared
// harness module is per-test-binary (the corpus.rs test-file precedent)
//! E2E journeys J1 (login), J2 (hydration proof), J7 (auth discipline) —
//! design doc §8d. Driven by `scripts/ui-e2e.sh`; each test skips with a
//! printed reason when the harness env is absent.

mod common;

use common::{Harness, env, login_basic};

/// J1 (Basic half): the Basic form logs in and lands on the dashboard with
/// the authenticated chrome; wrong credentials render an inline error and
/// do NOT create a session.
#[tokio::test]
async fn j01_login_basic() {
    let Some(h) = Harness::start("j01").await else {
        return;
    };
    // Wrong password first: error surface, still on /login.
    h.goto("/login").await;
    h.shot(1, "login-form").await;
    h.wait_css("#login-username")
        .await
        .send_keys("ehrbase")
        .await
        .expect("user");
    h.wait_css("#login-password")
        .await
        .send_keys("definitely-wrong")
        .await
        .expect("pass");
    h.wait_css("button[type=submit]")
        .await
        .click()
        .await
        .expect("submit");
    h.wait_css(".thaw-message-bar").await;
    h.shot(2, "login-error").await;
    assert!(
        h.driver
            .current_url()
            .await
            .expect("url")
            .as_str()
            .contains("/login"),
        "wrong credentials must stay on /login"
    );

    // Right credentials: dashboard + chrome.
    login_basic(&h).await;
    h.shot(3, "dashboard-after-login").await;
    let footer = h
        .wait_css("footer")
        .await
        .text()
        .await
        .expect("footer text");
    assert!(!footer.is_empty(), "authenticated footer renders");
    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// J1 (OIDC half): the OIDC button starts the Keycloak redirect flow; after
/// authenticating at Keycloak the browser lands back on the console with a
/// session (identity visible in the user menu trigger).
#[tokio::test]
async fn j01_login_oidc() {
    let Some(h) = Harness::start("j01-oidc").await else {
        return;
    };
    let (Some(user), Some(pass)) = (env("UI_E2E_OIDC_USER"), env("UI_E2E_OIDC_PASS")) else {
        println!("SKIP j01_login_oidc: UI_E2E_OIDC_USER/PASS unset");
        h.finish().await;
        return;
    };
    h.goto("/login").await;
    h.wait_css("a[href='/auth/oidc/login']")
        .await
        .click()
        .await
        .expect("oidc button");
    // Keycloak's login form.
    h.wait_url_contains("/auth/realms/ehrbase").await;
    h.shot(1, "keycloak-form").await;
    h.wait_css("#username")
        .await
        .send_keys(&user)
        .await
        .expect("kc user");
    h.wait_css("#password")
        .await
        .send_keys(&pass)
        .await
        .expect("kc pass");
    h.wait_css("#kc-login")
        .await
        .click()
        .await
        .expect("kc submit");
    // Back on the console, authenticated.
    h.wait_css("footer").await;
    h.shot(2, "console-after-oidc").await;
    let url = h.driver.current_url().await.expect("url");
    assert!(
        !url.as_str().contains("/login"),
        "OIDC flow must land on the console, not back at /login (got {url})"
    );
    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// J2: hydration proof — after first paint, an interaction actually mutates
/// the DOM (WASM attached), and the console log carries no hydration error.
#[tokio::test]
async fn j02_hydration_interactivity() {
    let Some(h) = Harness::start("j02").await else {
        return;
    };
    login_basic(&h).await;
    h.shot(1, "shell-before-toggle").await;
    // The scopes drawer opens on click — DOM state that only WASM can flip.
    h.wait_css("button[aria-haspopup], .thaw-button").await;
    let user_button = h.wait_css("header .thaw-button").await;
    user_button.click().await.expect("open user menu");
    h.wait_css(".thaw-popover-surface, .thaw-drawer, [role=dialog]")
        .await;
    h.shot(2, "user-menu-open").await;
    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// J7 (first half): an unauthenticated deep link redirects to `/login`; the
/// login page renders (the streaming-SSR client redirect completes).
#[tokio::test]
async fn j07_unauthenticated_redirect() {
    let Some(h) = Harness::start("j07").await else {
        return;
    };
    h.goto("/system").await;
    h.wait_url_contains("/login").await;
    h.wait_css("#login-username").await;
    h.shot(1, "redirected-to-login").await;
    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}
