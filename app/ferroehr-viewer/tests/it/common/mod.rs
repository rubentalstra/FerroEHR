// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Shared E2E journey harness: env-gated `WebDriver` setup (skip-with-reason
//! when the stack isn't up), step screenshots, explicit waits, and the
//! standing browser-console gate — every journey fails on any console
//! error (the cheapest hydration-bug detector).
//!
//! Two lanes drive this harness, and they are not equals. The IMAGE lane is
//! the authoritative one: it runs the shipped OCI artifact, whose WASM is the
//! release build. The HOST lane serves a debug-profile WASM bundle an order of
//! magnitude larger, trading hydration latency for compile speed so a change
//! can be driven in seconds — which is why [`Harness::wait_hydrated`] carries
//! its own, far longer budget than the element waits around it.

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

/// The budget every ordinary element wait allows.
const WAIT: Duration = Duration::from_secs(15);

/// The budget [`Harness::wait_hydrated`] allows, four times [`WAIT`]: the host
/// lane's debug WASM is ~91 MB and the browser has to fetch, compile and run it
/// before the marker appears, which is comfortably slower than any wait that
/// only observes a rendered page (module docs).
const HYDRATION_WAIT: Duration = Duration::from_mins(1);

/// Everything a journey needs.
pub(crate) struct Harness {
    /// The `WebDriver` session.
    pub(crate) driver: WebDriver,
    /// The viewer origin (`http://…`).
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
    reason = "the E2E harness is configured by the environment the CI job / scripts/ui-e2e.sh exports; there is no viewer config tree on the test side"
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
        // Image-mode OIDC: the composed viewer advertises the in-network
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

    /// Navigate to a viewer path.
    ///
    /// # Panics
    /// On navigation failure (journeys are assertive end-to-end).
    pub(crate) async fn goto(&self, path: &str) {
        // Sweep the browser console BEFORE leaving the page: a SEVERE entry
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

    /// Explicit wait: the first element matching `css`, within [`WAIT`].
    ///
    /// # Panics
    /// When the element never appears — with the selector in the message.
    pub(crate) async fn wait_css(&self, css: &str) -> WebElement {
        self.wait_css_for(css, WAIT).await
    }

    /// [`Self::wait_css`] with an explicit budget, so the hydration wait can be
    /// long without lengthening every other wait.
    ///
    /// # Panics
    /// When the element never appears — with the selector in the message.
    async fn wait_css_for(&self, css: &str, budget: Duration) -> WebElement {
        match self
            .driver
            .query(By::Css(css))
            .wait(budget, Duration::from_millis(200))
            .first()
            .await
        {
            Ok(element) => element,
            Err(e) => {
                // Failure evidence: where the browser actually was, and what
                // its console says. The console half is what separates "the
                // screen simply has not got there yet" from "the client
                // runtime is dead": an unrecoverable hydration error traps the
                // WASM module, after which no client-side navigation completes
                // and the page only moves on a full reload. Without it a
                // timeout reports a missing selector and hides its cause.
                let url = self
                    .driver
                    .current_url()
                    .await
                    .map(|u| u.to_string())
                    .unwrap_or_default();
                let path = format!("{}/{}-fail.png", self.shots_dir, self.journey);
                drop(self.driver.screenshot(std::path::Path::new(&path)).await);
                let console: Vec<String> = self
                    .driver
                    .get_log("browser")
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|entry| entry.level == "SEVERE")
                    .map(|entry| entry.message)
                    .collect();
                let console = if console.is_empty() {
                    "  (no SEVERE console entries)".to_owned()
                } else {
                    console
                        .iter()
                        .map(|m| format!("  {m}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                panic!("waiting for `{css}` at {url}: {e}\nbrowser console:\n{console}");
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
        // Image-mode OIDC: the composed viewer advertises the in-network
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
            .wait(WAIT, Duration::from_millis(200))
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
    /// A control the viewer disables until its form is valid is already
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
            .wait(WAIT, Duration::from_millis(200))
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
    /// When the URL never matches within [`WAIT`].
    pub(crate) async fn wait_url_contains(&self, fragment: &str) {
        self.wait_url_contains_for(fragment, WAIT).await;
    }

    /// [`Self::wait_url_contains`] with an explicit budget, for a transition
    /// the browser makes on its OWN schedule rather than in answer to a click
    /// — the session-expiry journeys wait out a whole poll interval.
    ///
    /// # Panics
    /// When the URL never matches within `budget`.
    pub(crate) async fn wait_url_contains_for(&self, fragment: &str, budget: Duration) {
        let step = Duration::from_millis(200);
        let mut waited = Duration::ZERO;
        loop {
            let url = self.driver.current_url().await.expect("current url");
            if url.as_str().contains(fragment) {
                return;
            }
            assert!(
                waited < budget,
                "URL never contained `{fragment}` (last: {url})"
            );
            tokio::time::sleep(step).await;
            waited += step;
        }
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
    ///
    /// This one wait gets [`HYDRATION_WAIT`] rather than [`WAIT`]: it is the
    /// only condition whose latency is dominated by the WASM bundle's size,
    /// which differs by an order of magnitude between the two lanes (module
    /// docs).
    pub(crate) async fn wait_hydrated(&self) {
        self.wait_css_for("body[data-hydrated]", HYDRATION_WAIT)
            .await;
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

    /// The standing browser-console gate: read the browser log (thirtyfour's
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

/// Upload `path` through the Template Manager's one upload dialog: open it
/// from the page-header trigger, choose the file, and send it.
///
/// Both template families share this control (#2955), so both families' seed
/// helpers drive the same routine. The submit button is inert until the chosen
/// file has been read into the dialog's source editor, which makes
/// [`wait_enabled`] the exact "the file arrived" condition — never a sleep.
///
/// # Panics
/// On any interaction failure.
pub(crate) async fn upload_via_dialog(h: &Harness, path: &str) {
    h.wait_css("#template-upload-open")
        .await
        .click()
        .await
        .expect("open the template upload dialog");
    h.wait_css("#template-upload-picker input[type=file]")
        .await
        .send_keys(path)
        .await
        .expect("choose the fixture through the dialog's hidden file input");
    wait_enabled(h, "#template-upload-submit").await;
    h.wait_css("#template-upload-submit")
        .await
        .click()
        .await
        .expect("send the chosen template source");
}

/// Poll until the control at `css` is present and ENABLED.
///
/// The viewer keeps an edit form inert until the document it edits has been
/// seeded into it, so this is the condition that makes typing (or saving) safe:
/// input accepted earlier would be replaced by the seed and the save would then
/// commit the pre-seed draft.
///
/// # Panics
/// When it never becomes enabled within 15 s.
pub(crate) async fn wait_enabled(h: &Harness, css: &str) {
    for _ in 0..75 {
        if let Ok(element) = h.driver.find(By::Css(css)).await
            && element.is_enabled().await.unwrap_or(false)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("`{css}` never became enabled — its form was never seeded");
}

/// Poll until no element matches `css` — the assert-gone half of a delete.
///
/// # Panics
/// When something still matches after [`WAIT`], with the page it was on.
pub(crate) async fn wait_css_absent(h: &Harness, css: &str) {
    for _ in 0..75 {
        if h.driver
            .find_all(By::Css(css))
            .await
            .unwrap_or_default()
            .is_empty()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let url = h.driver.current_url().await.expect("current url");
    panic!("`{css}` never disappeared (at {url})");
}

/// Wait until some element's text contains `needle` (a toast title, a status
/// line), returning whether it appeared.
pub(crate) async fn wait_text(h: &Harness, needle: &str) -> bool {
    let xpath = format!("//*[contains(normalize-space(.), '{needle}')]");
    for _ in 0..75 {
        if h.driver.find(By::XPath(&xpath)).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

/// Poll until the text of the element at `css` contains `fragment` — the "the
/// CDR actually applied it" assertion, an explicit condition rather than a
/// sleep.
///
/// # Panics
/// When it never does within [`WAIT`], reporting what it said instead.
pub(crate) async fn wait_text_contains(h: &Harness, css: &str, fragment: &str) {
    let mut last = String::new();
    for _ in 0..75 {
        if let Ok(element) = h.driver.find(By::Css(css)).await
            && let Ok(text) = element.text().await
        {
            if text.contains(fragment) {
                return;
            }
            last = text;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("`{css}` never contained `{fragment}` (last text: {last})");
}

/// Wait until the element at `css` has text ending in `suffix` (a version
/// number the screen must have caught up to).
///
/// # Panics
/// When it never does within [`WAIT`], reporting the last text seen.
pub(crate) async fn wait_text_suffix(h: &Harness, css: &str, suffix: &str) {
    let mut last = String::new();
    for _ in 0..75 {
        if let Ok(element) = h.driver.find(By::Css(css)).await {
            last = element.text().await.unwrap_or_default();
            if last.trim_end().ends_with(suffix) {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("`{css}` never ended in `{suffix}` (last text: `{last}`)");
}

/// Type `text` into the field at `css`, clearing whatever is there first.
///
/// # Panics
/// On any interaction failure.
pub(crate) async fn retype(h: &Harness, css: &str, text: &str) {
    let field = h.wait_css(css).await;
    field.clear().await.expect("clear the field");
    field.send_keys(text).await.expect("type into the field");
}

/// Click `css` until `target_css` shows up, returning whether it did (the
/// pre-hydration-click precedent; re-clicking an "open this version" button is
/// idempotent).
pub(crate) async fn click_until_css(h: &Harness, css: &str, target_css: &str) -> bool {
    for _ in 0..5 {
        h.wait_css(css).await.click().await.expect("click");
        for _ in 0..25 {
            if h.driver.find(By::Css(target_css)).await.is_ok() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    false
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

/// Poll until `css` is VISIBLE — the mirror of [`wait_hidden`].
///
/// thaw keeps a closed dialog in the DOM, so "the dialog opened" is a
/// visibility condition and never mere presence ([`is_visible`]).
///
/// # Panics
/// When it is still not visible after 15 s.
pub(crate) async fn wait_visible(h: &Harness, css: &str) {
    for _ in 0..75 {
        if is_visible(h, css).await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let url = h.driver.current_url().await.expect("current url");
    panic!("`{css}` never became visible (at {url})");
}

/// Empty the field at `css` BY TYPING.
///
/// Deliberately not `WebDriver`'s element-clear command: measured on the
/// event-subscription journey, clearing that way empties the DOM value without
/// the viewer's `on:input` listener ever running, so the form's state keeps
/// the old value and the save sends it back — a green screen and a wrong wire.
/// Backspacing is what a person does, and it fires the events the binding
/// listens for. ([`retype`] is unaffected: the keystrokes it sends after
/// clearing re-deliver the whole value.)
///
/// # Panics
/// On any interaction failure, or when the field is not empty afterwards.
pub(crate) async fn clear_field(h: &Harness, css: &str) {
    let field = h.wait_css(css).await;
    let held = field
        .prop("value")
        .await
        .expect("read the field's value")
        .unwrap_or_default();
    let mut keys = String::from(Key::End.value());
    keys.extend(std::iter::repeat_n(
        Key::Backspace.value(),
        held.chars().count(),
    ));
    field.send_keys(keys).await.expect("erase the field");
    let left = field
        .prop("value")
        .await
        .expect("read the field's value")
        .unwrap_or_default();
    assert!(
        left.is_empty(),
        "`{css}` still reads `{left}` after erasing"
    );
}

/// One attempt at reading `attribute` off the first element matching `css`,
/// or `None`.
///
/// `None` covers both "nothing matches yet" and a STALE element handle: a
/// re-rendering table detaches the handle between the find and the read, and
/// `attr` then answers `stale element reference` — a retry, never a failure.
pub(crate) async fn read_attr(h: &Harness, css: &str, attribute: &str) -> Option<String> {
    h.driver
        .find(By::Css(css))
        .await
        .ok()?
        .attr(attribute)
        .await
        .ok()
        .flatten()
}

/// `attribute` of the first element matching `css`, waited for — the rendered
/// window's identity.
///
/// # Panics
/// When nothing readable matches within the wait budget.
pub(crate) async fn wait_attr(h: &Harness, css: &str, attribute: &str) -> String {
    for _ in 0..75 {
        if let Some(value) = read_attr(h, css, attribute).await {
            return value;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let url = h.driver.current_url().await.expect("current url");
    panic!("no element matched `{css}` with a readable `{attribute}` (at {url})");
}

/// Poll until `attribute` of the first element matching `css` is no longer
/// `previous`, and return the new value.
///
/// This is the content-moved condition (never a sleep): a paging link is a real
/// navigation and a `<Transition>` keeps the previous rows on screen while the
/// next window loads.
///
/// # Panics
/// When it has not changed after 15 s.
pub(crate) async fn wait_attr_change(
    h: &Harness,
    css: &str,
    attribute: &str,
    previous: &str,
) -> String {
    for _ in 0..75 {
        if let Some(current) = read_attr(h, css, attribute).await
            && current != previous
        {
            return current;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let url = h.driver.current_url().await.expect("current url");
    panic!("`{css}`'s `{attribute}` never moved off `{previous}` (at {url})");
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
