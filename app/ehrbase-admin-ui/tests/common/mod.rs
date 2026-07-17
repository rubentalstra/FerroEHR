//! Shared E2E journey harness: env-gated `WebDriver` setup (skip-with-reason
//! when the stack isn't up), step screenshots, explicit waits, and the
//! standing browser-console gate — every journey fails on any console
//! error (the cheapest hydration-bug detector).

use std::time::Duration;

use thirtyfour::prelude::*;

/// Everything a journey needs.
pub struct Harness {
    /// The `WebDriver` session.
    pub driver: WebDriver,
    /// The console origin (`http://…`).
    pub base: String,
    shots_dir: String,
    journey: &'static str,
}

/// Environment lookup for a journey credential/URL.
pub fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

impl Harness {
    /// Start a journey: returns `None` (with a printed reason) when the
    /// harness env is absent, so plain `cargo nextest run` stays green.
    ///
    /// # Panics
    /// When the stack env is set but the browser session cannot start —
    /// that IS a failure, not a skip.
    pub async fn start(journey: &'static str) -> Option<Self> {
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
        })
    }

    /// Navigate to a console path.
    ///
    /// # Panics
    /// On navigation failure (journeys are assertive end-to-end).
    pub async fn goto(&self, path: &str) {
        self.driver
            .goto(format!("{}{path}", self.base))
            .await
            .expect("navigate");
    }

    /// Explicit wait: the first element matching `css`, within 15 s.
    ///
    /// # Panics
    /// When the element never appears — with the selector in the message.
    pub async fn wait_css(&self, css: &str) -> WebElement {
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

    /// Wait until the current URL no longer contains `fragment`.
    ///
    /// # Panics
    /// When the URL still matches after 15 s.
    pub async fn wait_url_not_contains(&self, fragment: &str) {
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

    /// Wait until the current URL contains `fragment` (redirect chains).
    ///
    /// # Panics
    /// When the URL never matches within 15 s.
    pub async fn wait_url_contains(&self, fragment: &str) {
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

    /// Numbered step screenshot: `{journey}-{step}-{slug}.png`.
    ///
    /// # Panics
    /// On capture/IO failure.
    pub async fn shot(&self, step: u8, slug: &str) {
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
    pub async fn assert_console_clean(&self, allowed: &[&str]) {
        let entries = self
            .driver
            .get_log("browser")
            .await
            .expect("browser log (chromedriver legacy endpoint)");
        let severe: Vec<String> = entries
            .into_iter()
            .filter(|e| e.level == "SEVERE")
            .map(|e| e.message)
            .filter(|m| !allowed.iter().any(|a| m.contains(a)))
            .collect();
        assert!(
            severe.is_empty(),
            "browser console has SEVERE entries:\n{}",
            severe.join("\n")
        );
    }

    /// End the session (screenshots + console gate are per-journey calls).
    pub async fn finish(self) {
        self.driver.quit().await.expect("quit");
    }
}

/// Log in through the Basic form (journeys that need a session).
///
/// # Panics
/// When the login flow does not land on the dashboard.
pub async fn login_basic(h: &Harness) {
    let user = env("UI_E2E_BASIC_USER").unwrap_or_else(|| "ehrbase".to_owned());
    let pass = env("UI_E2E_BASIC_PASS").unwrap_or_else(|| "ehrbase".to_owned());
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
    h.wait_css("button[type=submit]")
        .await
        .click()
        .await
        .expect("submit");
    // Off the login screen, then the shell footer as the
    // authenticated-chrome marker.
    h.wait_url_not_contains("/login").await;
    h.wait_css("footer").await;
}
