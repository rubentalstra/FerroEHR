#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    unreachable_pub,
    dead_code // each test binary uses a subset of the shared harness methods
)]
// e2e journeys are assertive by design; skip-with-reason prints; the shared
// harness module is per-test-binary (the corpus.rs test-file precedent)
//! End-to-end journeys over the seeded EHR + two-version composition
//! (`scripts/ui-e2e.sh` seeds them over REST): the EHR detail tabs, the
//! composition viewer's format toggle + version history, and the no-JS
//! progressive-enhancement contract.

mod common;

use common::{Harness, env, login_basic};
use thirtyfour::prelude::*;

/// The seeded ids, exported by the harness; `None` skips with a reason.
fn seeded() -> Option<(String, String)> {
    if let (Some(ehr), Some(vo)) = (env("UI_E2E_SEEDED_EHR_ID"), env("UI_E2E_SEEDED_VO_ID")) {
        Some((ehr, vo))
    } else {
        println!("SKIP: UI_E2E_SEEDED_EHR_ID/UI_E2E_SEEDED_VO_ID unset (run scripts/ui-e2e.sh)");
        None
    }
}

/// The EHR detail screen renders the seeded EHR: the status tab shows the
/// queryable badge, and the compositions tab lists the seeded composition
/// whose link opens the viewer.
#[tokio::test]
async fn ehr_detail_lists_seeded_composition() {
    let Some(h) = Harness::start("ehr-detail").await else {
        return;
    };
    let Some((ehr_id, vo_id)) = seeded() else {
        h.finish().await;
        return;
    };
    login_basic(&h).await;
    h.goto(&format!("/ehrs/{ehr_id}")).await;
    // Status tab (default): the EHR_STATUS document resolved.
    h.wait_xpath("//*[contains(., 'queryable')]").await;
    h.shot(1, "status-tab").await;

    // Compositions tab: the seeded row links to the viewer.
    h.wait_xpath("//button[contains(., 'Compositions')]")
        .await
        .click()
        .await
        .expect("open compositions tab");
    let link = h.wait_css(&format!("a[href*='{vo_id}']")).await;
    h.shot(2, "compositions-tab").await;
    link.click().await.expect("open the composition viewer");
    h.wait_url_contains(&format!("/compositions/{vo_id}")).await;
    h.wait_css("pre").await;
    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// The composition viewer round-trips all four representations and walks
/// the two-version history: JSON carries `_type`, XML carries the openEHR
/// namespace, FLAT carries flat keys, and switching to version 1 re-renders.
#[tokio::test]
async fn composition_viewer_switches_formats_and_versions() {
    let Some(h) = Harness::start("composition-viewer").await else {
        return;
    };
    let Some((ehr_id, vo_id)) = seeded() else {
        h.finish().await;
        return;
    };
    login_basic(&h).await;
    h.goto(&format!("/ehrs/{ehr_id}/compositions/{vo_id}"))
        .await;

    // Canonical JSON (the default): the document pane carries `_type`.
    wait_pre_contains(&h, "\"_type\"").await;
    h.shot(1, "canonical-json").await;

    // Canonical XML: the openEHR namespace appears.
    click_format(&h, "XML").await;
    wait_pre_contains(&h, "schemas.openehr.org").await;
    h.shot(2, "canonical-xml").await;

    // FLAT: flat path keys (the template id prefixes every key).
    click_format(&h, "FLAT").await;
    wait_pre_contains(&h, "minimal_evaluation").await;
    h.shot(3, "flat").await;

    // Version history: two versions; selecting v1 re-renders the pane.
    let select = h.wait_css("select").await;
    let options = select
        .find_all(By::Tag("option"))
        .await
        .expect("version options");
    assert!(
        options.len() >= 2,
        "the seeded composition has two versions (got {})",
        options.len()
    );
    // Choose the oldest (last listed) version.
    options
        .last()
        .expect("a version option")
        .click()
        .await
        .expect("select version 1");
    click_format(&h, "JSON").await;
    wait_pre_contains(&h, "\"_type\"").await;
    h.shot(4, "version-1-json").await;

    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// Progressive enhancement: with JavaScript disabled the Basic login (an
/// `ActionForm` — a plain HTML form pre-hydration) still authenticates via
/// a full-page POST + redirect, and the SSR'd dashboard renders.
#[tokio::test]
async fn login_works_with_javascript_disabled() {
    let Some(h) = Harness::start_without_javascript("no-js").await else {
        return;
    };
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
        .expect("submit (plain form POST)");
    h.wait_url_not_contains("/login").await;
    // The SSR'd authenticated chrome, no WASM involved.
    h.wait_css("footer").await;
    h.shot(1, "dashboard-no-js").await;
    h.finish().await;
}

/// Poll the first `<pre>` until it contains `needle`.
async fn wait_pre_contains(h: &Harness, needle: &str) {
    for _ in 0..75 {
        if let Ok(pre) = h.driver.find(By::Css("pre")).await
            && let Ok(text) = pre.text().await
            && text.contains(needle)
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    panic!("document pane never contained `{needle}`");
}

/// Click a format-selector button by its label.
async fn click_format(h: &Harness, label: &str) {
    h.wait_xpath(&format!("//button[contains(., '{label}')]"))
        .await
        .click()
        .await
        .expect("switch format");
}
