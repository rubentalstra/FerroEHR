// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Shared E2E journey harness: env-gated `WebDriver` setup (skip-with-reason
//! when the stack isn't up), step screenshots, explicit waits, and the
//! standing browser-console gate — every journey fails on any console
//! error (the cheapest hydration-bug detector).

#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    reason = "the shared e2e fixture panics when a configured stack cannot be \
              driven, and the skip-with-reason lines ARE this suite's report \
              (the clippy.toml test scoping reaches only #[test] fns, not \
              integration-binary helper modules)"
)]
#![expect(
    clippy::disallowed_types,
    reason = "test fixtures and wire assertions are raw JSON by the testing rule \
              (.claude/rules/testing.md §Test-fixture construction)"
)]

use std::time::Duration;

use thirtyfour::prelude::*;

/// Everything a journey needs.
pub(crate) struct Harness {
    /// The `WebDriver` session.
    pub(crate) driver: WebDriver,
    /// The console origin (`http://…`).
    pub(crate) base: String,
    shots_dir: String,
    journey: &'static str,
    /// Whether this session runs with JavaScript enabled — hydrated pages
    /// only exist in that mode, so [`Harness::goto`] waits for the shell's
    /// hydration marker exactly when one will ever appear.
    js: bool,
}

/// Environment lookup for a journey credential/URL.
#[expect(
    clippy::disallowed_methods,
    reason = "the E2E harness is configured by the environment the CI job / scripts/ui-e2e.sh exports; there is no console config tree on the test side"
)]
pub(crate) fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

impl Harness {
    /// Start a journey: returns `None` (with a printed reason) when the
    /// harness env is absent, so plain `cargo nextest run` stays green.
    ///
    /// # Panics
    /// When the stack env is set but the browser session cannot start —
    /// that IS a failure, not a skip.
    pub(crate) async fn start(journey: &'static str) -> Option<Self> {
        let (Some(base), Some(webdriver_url)) =
            (env("UI_E2E_BASE_URL"), env("UI_E2E_WEBDRIVER_URL"))
        else {
            println!(
                "SKIP {journey}: UI_E2E_BASE_URL/UI_E2E_WEBDRIVER_URL unset (run scripts/ui-e2e.sh)"
            );
            return None;
        };
        let shots_dir =
            env("UI_E2E_SHOTS_DIR").unwrap_or_else(|| "target/ui-e2e/screenshots".to_owned());
        std::fs::create_dir_all(&shots_dir).expect("screenshot dir");

        let mut caps = DesiredCapabilities::chrome();
        caps.add_arg("--headless=new").expect("caps");
        caps.add_arg("--window-size=1440,900").expect("caps");
        // Image-mode OIDC: the composed console advertises the in-network
        // issuer host (`keycloak`); the browser resolves it to the host-
        // mapped port. A no-op in host mode (nothing references the name).
        caps.add_arg("--host-resolver-rules=MAP keycloak 127.0.0.1")
            .expect("caps");
        caps.set_logging_prefs("browser", thirtyfour::LoggingPrefsLogLevel::All)
            .expect("logging prefs");
        let driver = WebDriver::new(&webdriver_url, caps)
            .await
            .expect("webdriver session (is chromedriver up?)");
        Some(Self {
            driver,
            base,
            shots_dir,
            journey,
            js: true,
        })
    }

    /// Navigate to a console path.
    ///
    /// # Panics
    /// On navigation failure (journeys are assertive end-to-end).
    pub(crate) async fn goto(&self, path: &str) {
        // Sweep the console BEFORE leaving the current page: a SEVERE entry
        // is attributed to the page that produced it, not discovered by the
        // end-of-journey sweep with no locality (`get_log` drains, so the
        // final sweep still covers everything after the last navigation).
        let leaving = self
            .driver
            .current_url()
            .await
            .map(|u| u.to_string())
            .unwrap_or_default();
        let entries = self
            .driver
            .get_log("browser")
            .await
            .expect("browser log (chromedriver legacy endpoint)");
        let severe: Vec<String> = entries
            .into_iter()
            .filter(|e| e.level == "SEVERE")
            .map(|e| e.message)
            .filter(|m| !m.contains("Failed to load resource"))
            .collect();
        assert!(
            severe.is_empty(),
            "browser console has SEVERE entries on {leaving} (before navigating to {path}):\n{}",
            severe.join("\n")
        );
        self.driver
            .goto(format!("{}{path}", self.base))
            .await
            .expect("navigate");
        // Every full navigation restarts hydration, and any first click or
        // file selection landing before it completes is silently lost —
        // unrecoverably for same-value re-sends (#2285). Waiting here makes
        // every journey's first interaction land on live listeners; the
        // no-JS sessions skip it, since their pages never hydrate.
        if self.js {
            self.wait_hydrated().await;
        }
    }

    /// Explicit wait: the first element matching `css`, within 15 s.
    ///
    /// # Panics
    /// When the element never appears — with the selector in the message.
    pub(crate) async fn wait_css(&self, css: &str) -> WebElement {
        match self
            .driver
            .query(By::Css(css))
            .wait(Duration::from_secs(15), Duration::from_millis(200))
            .first()
            .await
        {
            Ok(element) => element,
            Err(e) => {
                // Failure evidence: where the browser actually was.
                let url = self
                    .driver
                    .current_url()
                    .await
                    .map(|u| u.to_string())
                    .unwrap_or_default();
                let path = format!("{}/{}-fail.png", self.shots_dir, self.journey);
                drop(self.driver.screenshot(std::path::Path::new(&path)).await);
                panic!("waiting for `{css}` at {url}: {e}");
            }
        }
    }

    /// Start a journey with JavaScript DISABLED (the progressive-enhancement
    /// contract: SSR + plain HTML forms must work before WASM ever loads).
    ///
    /// # Panics
    /// When the stack env is set but the browser session cannot start.
    pub(crate) async fn start_without_javascript(journey: &'static str) -> Option<Self> {
        let (Some(base), Some(webdriver_url)) =
            (env("UI_E2E_BASE_URL"), env("UI_E2E_WEBDRIVER_URL"))
        else {
            println!(
                "SKIP {journey}: UI_E2E_BASE_URL/UI_E2E_WEBDRIVER_URL unset (run scripts/ui-e2e.sh)"
            );
            return None;
        };
        let shots_dir =
            env("UI_E2E_SHOTS_DIR").unwrap_or_else(|| "target/ui-e2e/screenshots".to_owned());
        std::fs::create_dir_all(&shots_dir).expect("screenshot dir");

        let mut caps = DesiredCapabilities::chrome();
        caps.add_arg("--headless=new").expect("caps");
        caps.add_arg("--window-size=1440,900").expect("caps");
        // Image-mode OIDC: the composed console advertises the in-network
        // issuer host (`keycloak`); the browser resolves it to the host-
        // mapped port. A no-op in host mode (nothing references the name).
        caps.add_arg("--host-resolver-rules=MAP keycloak 127.0.0.1")
            .expect("caps");
        // Chrome content-settings: 2 = block JavaScript.
        caps.add_experimental_option(
            "prefs",
            serde_json::json!({"profile.managed_default_content_settings.javascript": 2}),
        )
        .expect("prefs");
        let driver = WebDriver::new(&webdriver_url, caps)
            .await
            .expect("webdriver session (is chromedriver up?)");
        Some(Self {
            driver,
            base,
            shots_dir,
            journey,
            js: false,
        })
    }

    /// Wait until the current URL no longer contains `fragment`.
    ///
    /// # Panics
    /// When the URL still matches after 15 s.
    pub(crate) async fn wait_url_not_contains(&self, fragment: &str) {
        for _ in 0..75 {
            let url = self.driver.current_url().await.expect("current url");
            if !url.as_str().contains(fragment) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let url = self.driver.current_url().await.expect("current url");
        panic!("URL still contains `{fragment}` (last: {url})");
    }

    /// Explicit wait on an `XPath` (same budget + failure evidence as
    /// [`Self::wait_css`]).
    ///
    /// # Panics
    /// When the element never appears.
    pub(crate) async fn wait_xpath(&self, xpath: &str) -> WebElement {
        match self
            .driver
            .query(By::XPath(xpath))
            .wait(Duration::from_secs(15), Duration::from_millis(200))
            .first()
            .await
        {
            Ok(element) => element,
            Err(e) => {
                let url = self
                    .driver
                    .current_url()
                    .await
                    .map(|u| u.to_string())
                    .unwrap_or_default();
                let path = format!("{}/{}-fail.png", self.shots_dir, self.journey);
                drop(self.driver.screenshot(std::path::Path::new(&path)).await);
                panic!("waiting for xpath `{xpath}` at {url}: {e}");
            }
        }
    }

    /// Explicit wait on an `XPath` that additionally requires the element to be
    /// CLICKABLE — displayed and enabled — before returning it.
    ///
    /// A control the console disables until its form is valid is already
    /// PRESENT, so [`Self::wait_xpath`] hands it back and the click is
    /// INTERCEPTED by whatever sits above it. That is an error rather than a
    /// lost interaction, so the re-click loop other journeys use for
    /// pre-hydration clicks does not cover it — the condition has to be part of
    /// the wait.
    ///
    /// # Panics
    /// When the element never becomes clickable.
    pub(crate) async fn wait_clickable_xpath(&self, xpath: &str) -> WebElement {
        match self
            .driver
            .query(By::XPath(xpath))
            .and_clickable()
            .wait(Duration::from_secs(15), Duration::from_millis(200))
            .first()
            .await
        {
            Ok(element) => element,
            Err(e) => {
                let url = self
                    .driver
                    .current_url()
                    .await
                    .map(|u| u.to_string())
                    .unwrap_or_default();
                let path = format!("{}/{}-fail.png", self.shots_dir, self.journey);
                drop(self.driver.screenshot(std::path::Path::new(&path)).await);
                panic!("waiting for xpath `{xpath}` to become clickable at {url}: {e}");
            }
        }
    }

    /// Wait until the current URL contains `fragment` (redirect chains).
    ///
    /// # Panics
    /// When the URL never matches within 15 s.
    pub(crate) async fn wait_url_contains(&self, fragment: &str) {
        for _ in 0..75 {
            let url = self.driver.current_url().await.expect("current url");
            if url.as_str().contains(fragment) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let url = self.driver.current_url().await.expect("current url");
        panic!("URL never contained `{fragment}` (last: {url})");
    }

    /// Wait until no toast card is on screen (a visible `thaw` toast overlays
    /// the bottom-right corner and INTERCEPTS clicks on buttons underneath —
    /// an explicit condition, not a sleep). Toasts auto-dismiss; bounded wait.
    ///
    /// # Panics
    /// When a toast is still visible after 15 s.
    pub(crate) async fn wait_toasts_cleared(&self) {
        for _ in 0..75 {
            let toasts = self
                .driver
                .find_all(By::Css(".thaw-toast-body"))
                .await
                .unwrap_or_default();
            if toasts.is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        panic!("a toast never cleared (it would intercept the next click)");
    }

    /// Wait until client hydration has completed on the current page (the
    /// shell stamps `data-hydrated` on `<body>` from a browser-only effect).
    /// Required before driving any control whose handler exists only
    /// hydrated — a click or file selection landing earlier is silently
    /// lost, and a same-value re-send fires no later event (#2285).
    pub(crate) async fn wait_hydrated(&self) {
        self.wait_css("body[data-hydrated]").await;
    }

    /// Failure evidence at a journey-defined point, for a panic that would
    /// otherwise preempt the next `goto`'s console sweep: a screenshot, the
    /// DRAINED browser console printed to the test log (hydration errors and
    /// panics land there), and any visible message-bar text — returned as one
    /// line for the panic message.
    pub(crate) async fn evidence_dump(&self, slug: &str) -> String {
        let url = self
            .driver
            .current_url()
            .await
            .map(|u| u.to_string())
            .unwrap_or_default();
        let path = format!("{}/{}-{slug}.png", self.shots_dir, self.journey);
        drop(self.driver.screenshot(std::path::Path::new(&path)).await);
        let entries = self.driver.get_log("browser").await.unwrap_or_default();
        let mut severe = 0usize;
        for entry in &entries {
            if entry.level == "SEVERE" {
                severe += 1;
            }
            println!("console[{}]: {}", entry.level, entry.message);
        }
        let bar = match self.driver.find(By::Css(".thaw-message-bar")).await {
            Ok(el) => el.text().await.unwrap_or_default(),
            Err(_) => String::new(),
        };
        format!(
            "at {url}; {} console entries ({severe} SEVERE — printed above); message bar: {bar:?}",
            entries.len()
        )
    }

    /// Numbered step screenshot: `{journey}-{step}-{slug}.png`.
    ///
    /// # Panics
    /// On capture/IO failure.
    pub(crate) async fn shot(&self, step: u8, slug: &str) {
        let path = format!("{}/{}-{step:02}-{slug}.png", self.shots_dir, self.journey);
        self.driver
            .screenshot(std::path::Path::new(&path))
            .await
            .expect("screenshot");
    }

    /// The standing console gate: read the browser log (thirtyfour's
    /// legacy-log support over chromedriver) and fail on any SEVERE entry
    /// (hydration errors and panics land there). Network 4xx from
    /// deliberate negative steps can be allowed by substring.
    ///
    /// # Panics
    /// When the log contains a SEVERE entry not covered by `allowed`.
    pub(crate) async fn assert_console_clean(&self, allowed: &[&str]) {
        let entries = self
            .driver
            .get_log("browser")
            .await
            .expect("browser log (chromedriver legacy endpoint)");
        let severe: Vec<String> = entries
            .into_iter()
            .filter(|e| e.level == "SEVERE")
            .map(|e| format!("[ts={}] {}", e.timestamp, e.message))
            .filter(|m| !allowed.iter().any(|a| m.contains(a)))
            .collect();
        let at = self
            .driver
            .current_url()
            .await
            .map(|u| u.to_string())
            .unwrap_or_default();
        assert!(
            severe.is_empty(),
            "browser console has SEVERE entries (last page: {at}):\n{}",
            severe.join("\n")
        );
    }

    /// End the session (screenshots + console gate are per-journey calls).
    pub(crate) async fn finish(self) {
        self.driver.quit().await.expect("quit");
    }
}

/// Whether `css` matches a currently VISIBLE element.
///
/// thaw's dialog is never removed from the DOM: `leptos_transition_group`'s
/// `CSSTransition` hides it with `display: none`, so a closed dialog is still
/// findable. Openness is therefore visibility, never mere presence.
pub(crate) async fn is_visible(h: &Harness, css: &str) -> bool {
    match h.driver.find(By::Css(css)).await {
        Ok(element) => element.is_displayed().await.unwrap_or(false),
        Err(_) => false,
    }
}

/// Poll until `css` is no longer visible.
///
/// # Panics
/// When it is still visible after 15 s.
pub(crate) async fn wait_hidden(h: &Harness, css: &str) {
    for _ in 0..75 {
        if !is_visible(h, css).await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let url = h.driver.current_url().await.expect("current url");
    panic!("`{css}` never hid (at {url})");
}

/// Drive one action through its confirmation MODAL: click the trigger, wait for
/// the dialog to become visible, then click its confirm button. Explicit
/// conditions, never a sleep.
///
/// The trigger click carries a bounded retry (the login-submit precedent): a
/// click landing before hydration attaches the listener is simply lost, and
/// re-clicking is safe precisely because the dialog is not open yet.
///
/// # Panics
/// When the dialog never opens.
pub(crate) async fn confirm_in_dialog(h: &Harness, trigger_css: &str, confirm_id: &str) {
    // A visible toast overlays the bottom-right corner and intercepts clicks.
    h.wait_toasts_cleared().await;
    let trigger = h.wait_css(trigger_css).await;
    let confirm_css = format!("#{confirm_id}");
    let mut opened = false;
    for attempt in 0..10 {
        // Retries exist for a pre-hydration click that does nothing. Once a
        // dialog surface is actually up, clicking the trigger again would be
        // swallowed by the modal and surface as `ElementClickIntercepted` — a
        // confusing report for what is really "the dialog opened, but not with
        // the confirm id this call expects". Stop and let the assertion say so.
        if attempt > 0 && is_visible(h, ".thaw-dialog-surface").await {
            break;
        }
        trigger.click().await.expect("open the confirmation dialog");
        for _ in 0..10 {
            if is_visible(h, &confirm_css).await {
                opened = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        if opened {
            break;
        }
    }
    assert!(
        opened,
        "`{trigger_css}` never opened the `{confirm_css}` dialog — either the click \
         never landed (pre-hydration) or a dialog opened whose confirm id is not \
         `{confirm_id}`"
    );
    h.wait_css(&confirm_css)
        .await
        .click()
        .await
        .expect("confirm in the dialog");
    // The dialog hides on confirm — that it hid proves the click landed.
    wait_hidden(h, &confirm_css).await;
}

/// Log in through the Basic form (journeys that need a session).
///
/// # Panics
/// When the login flow does not land on the dashboard.
pub(crate) async fn login_basic(h: &Harness) {
    let user = env("UI_E2E_BASIC_USER").unwrap_or_else(|| "ferroehr".to_owned());
    let pass = env("UI_E2E_BASIC_PASS").unwrap_or_else(|| "ferroehr".to_owned());
    login_basic_as(h, &user, &pass).await;
}

/// [`login_basic`] with explicit credentials — for journeys that need a
/// specific dev user (the audit screens require the CDR's admin role:
/// `UI_E2E_ADMIN_USER`/`UI_E2E_ADMIN_PASS`, defaulting to the quickstart
/// `ferroehr-admin`/`ferroehr` Basic user).
///
/// # Panics
/// When the login flow does not land on the dashboard.
pub(crate) async fn login_basic_as(h: &Harness, user: &str, pass: &str) {
    let user = user.to_owned();
    let pass = pass.to_owned();
    h.goto("/login").await;
    h.wait_css("#login-username")
        .await
        .send_keys(&user)
        .await
        .expect("type user");
    h.wait_css("#login-password")
        .await
        .send_keys(&pass)
        .await
        .expect("type pass");
    // Submit with a bounded retry: a click landing exactly while hydration
    // swaps the form can be lost (the ActionForm is valid both pre- and
    // post-hydration, but the swap instant is a real race). Each attempt
    // gets a short bounded wait; leaving /login ends the loop.
    let mut attempts = 0;
    loop {
        h.wait_css("button[type=submit]")
            .await
            .click()
            .await
            .expect("submit");
        for _ in 0..15 {
            if !h
                .driver
                .current_url()
                .await
                .expect("current url")
                .as_str()
                .contains("/login")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        let off_login = !h
            .driver
            .current_url()
            .await
            .expect("current url")
            .as_str()
            .contains("/login");
        if off_login {
            break;
        }
        attempts += 1;
        if attempts >= 3 {
            let path = format!("{}/{}-login-stuck.png", h.shots_dir, h.journey);
            drop(h.driver.screenshot(std::path::Path::new(&path)).await);
            let log = h.driver.get_log("browser").await.unwrap_or_default();
            for entry in &log {
                println!("console[{}]: {}", entry.level, entry.message);
            }
            panic!("login submit did not leave /login after 3 attempts");
        }
    }
    // The shell footer is the authenticated-chrome marker.
    h.wait_css("footer").await;
}
