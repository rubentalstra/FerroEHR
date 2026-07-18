#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    unreachable_pub,
    dead_code // this binary uses a subset of the shared harness methods
)]
// A capture pass, not an assertive journey: it drives the console and writes
// the canonical per-screen screenshots the website book embeds. Gated behind
// UI_E2E_DOCS_SHOTS so the normal E2E run (and plain `cargo nextest`) skips it.
//! Documentation-screenshot capture — one full-window PNG per console screen,
//! written directly under `website/book/src/admin-ui/img/`. Run via
//! `scripts/ui-e2e.sh` with `UI_E2E_DOCS_SHOTS` set; skips with a printed
//! reason when the harness environment or the gate flag is absent.

mod common;

use std::path::{Path, PathBuf};

use common::{Harness, env, login_basic};
use thirtyfour::prelude::*;

/// The detail-route id of the fixture template the browse journeys upload; its
/// detail screen is captured when the template is present on the stack.
const TEMPLATE_ID: &str = "minimal_evaluation.en.v1";

/// The website book's screenshot directory (`website/book/src/admin-ui/img`),
/// resolved from this crate's manifest dir (`app/ehrbase-admin-ui`).
fn book_img_dir() -> PathBuf {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let dir = Path::new(root)
        .join("website")
        .join("book")
        .join("src")
        .join("admin-ui")
        .join("img");
    std::fs::create_dir_all(&dir).expect("create the book screenshot dir");
    dir
}

/// Navigate to `path`, wait for the authenticated chrome plus an optional
/// content marker, and write a full-window PNG to `{dir}/{slug}.png`.
async fn capture(h: &Harness, dir: &Path, path: &str, slug: &str, content: Option<&str>) {
    h.goto(path).await;
    h.wait_css("footer").await;
    if let Some(selector) = content {
        h.wait_css(selector).await;
    }
    let out = dir.join(format!("{slug}.png"));
    h.driver
        .screenshot(&out)
        .await
        .expect("write the documentation screenshot");
    println!("captured {slug} -> {}", out.display());
}

/// Capture the canonical documentation screenshots for every console screen.
#[tokio::test]
async fn capture_documentation_screenshots() {
    let Some(h) = Harness::start("docs-shots").await else {
        return;
    };
    if env("UI_E2E_DOCS_SHOTS").is_none() {
        println!("SKIP docs-shots: UI_E2E_DOCS_SHOTS unset (set it to capture book screenshots)");
        h.finish().await;
        return;
    }
    let dir = book_img_dir();

    // The login screen is captured BEFORE authenticating (there is no footer
    // yet — wait on the username field instead).
    h.goto("/login").await;
    h.wait_css("#login-username").await;
    let login_out = dir.join("login.png");
    h.driver
        .screenshot(&login_out)
        .await
        .expect("write the login screenshot");
    println!("captured login -> {}", login_out.display());

    login_basic(&h).await;

    // The authenticated screens, each with a stable content marker so the shot
    // is taken after the screen's primary content has rendered.
    capture(&h, &dir, "/", "dashboard", None).await;
    capture(
        &h,
        &dir,
        "/templates",
        "templates",
        Some("input[type=file]"),
    )
    .await;

    // The template-detail shot needs the fixture template present (the browse
    // journeys upload it earlier in the same stacked run).
    if h.driver
        .find(By::Css("a[href='/templates/minimal_evaluation.en.v1']"))
        .await
        .is_ok()
    {
        capture(
            &h,
            &dir,
            &format!("/templates/{TEMPLATE_ID}"),
            "template-detail",
            Some("ul.text-sm li"),
        )
        .await;
    } else {
        println!(
            "TODO docs-shots: template-detail skipped — `{TEMPLATE_ID}` not present on the stack \
             (run the browse journeys first to seed it)"
        );
    }

    capture(&h, &dir, "/queries", "queries", None).await;
    capture(
        &h,
        &dir,
        "/queries/builder",
        "query-builder",
        Some("#qb-template"),
    )
    .await;
    capture(&h, &dir, "/queries/aql", "query-aql", Some("#aql-editor")).await;
    capture(&h, &dir, "/ehrs", "ehrs", Some("#ehr-lookup")).await;
    capture(&h, &dir, "/system", "system", None).await;

    // The ehr-detail and composition-viewer screens render the EHR + the
    // two-version composition scripts/ui-e2e.sh seeds over REST.
    if let (Some(ehr_id), Some(vo_id)) = (env("UI_E2E_SEEDED_EHR_ID"), env("UI_E2E_SEEDED_VO_ID")) {
        // The EHR detail shot shows the compositions tab (the seeded row),
        // reached the way the journey proves works: navigate, open the tab,
        // wait for the seeded composition's link.
        h.goto(&format!("/ehrs/{ehr_id}")).await;
        h.wait_css("footer").await;
        h.wait_xpath("//a[contains(., 'Compositions')]")
            .await
            .click()
            .await
            .expect("open the compositions tab");
        h.wait_css(&format!("a[href*='{vo_id}']")).await;
        let out = dir.join("ehr-detail.png");
        h.driver
            .screenshot(&out)
            .await
            .expect("write the documentation screenshot");
        println!("captured ehr-detail -> {}", out.display());
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}/compositions/{vo_id}"),
            "composition-viewer",
            Some("pre"),
        )
        .await;
    } else {
        println!(
            "SKIP docs-shots: ehr-detail + composition-viewer not captured \
             (UI_E2E_SEEDED_EHR_ID/UI_E2E_SEEDED_VO_ID unset — run scripts/ui-e2e.sh)"
        );
    }

    // ── The feature VIEWS (owner directive 2026-07-18: every view has a
    //    published screenshot so the console can be reviewed without
    //    running it). ─────────────────────────────────────────────────────
    if let (Some(ehr_id), Some(vo_id)) = (env("UI_E2E_SEEDED_EHR_ID"), env("UI_E2E_SEEDED_VO_ID")) {
        // EHR detail: the status tab (URL-driven tab state).
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}?tab=status"),
            "ehr-detail-status",
            Some("pre"),
        )
        .await;
        // EHR detail: the contributions table (needs the extension endpoint).
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}?tab=contributions"),
            "ehr-detail-contributions",
            Some("table tbody"),
        )
        .await;
        // EHR detail: the commit-composition form (scrolled into view).
        h.goto(&format!("/ehrs/{ehr_id}?tab=compositions")).await;
        let commit_body = h.wait_css("#commit-body").await;
        commit_body
            .scroll_into_view()
            .await
            .expect("scroll to the commit form");
        let out = dir.join("composition-commit.png");
        h.driver.screenshot(&out).await.expect("shot");
        println!("captured composition-commit -> {}", out.display());
        // Composition viewer: the edit-as-new-version editor open.
        h.goto(&format!("/ehrs/{ehr_id}/compositions/{vo_id}"))
            .await;
        h.wait_css("#edit-new-version")
            .await
            .click()
            .await
            .expect("open the version editor");
        let edit_body = h.wait_css("#edit-body").await;
        edit_body
            .scroll_into_view()
            .await
            .expect("scroll to the editor");
        let out = dir.join("composition-editor.png");
        h.driver.screenshot(&out).await.expect("shot");
        println!("captured composition-editor -> {}", out.display());
    } else {
        println!("SKIP docs-shots: feature views need the seeded ids");
    }

    // Raw AQL: run a data query — results table + export buttons, then the
    // chart view (the seeded quantity magnitudes over the row order).
    h.goto("/queries/aql").await;
    h.wait_css("#aql-editor")
        .await
        .send_keys(
            "SELECT c/context/start_time/value AS time,              c/content[openEHR-EHR-EVALUATION.minimal.v1]/data[at0001]/items[at0002]/value/magnitude AS magnitude              FROM EHR e CONTAINS COMPOSITION c              WHERE c/archetype_details/template_id/value = 'minimal_evaluation.en.v1'",
        )
        .await
        .expect("type the AQL");
    h.wait_xpath("//button[normalize-space(.)='Run']")
        .await
        .click()
        .await
        .expect("run");
    h.wait_xpath("//button[contains(., 'Export CSV')]").await;
    let out = dir.join("query-aql-results.png");
    h.driver.screenshot(&out).await.expect("shot");
    println!("captured query-aql-results -> {}", out.display());
    h.wait_xpath("//button[normalize-space(.)='Chart']")
        .await
        .click()
        .await
        .expect("chart toggle");
    h.wait_css("svg.chartistry_chart, div.overflow-x-auto svg").await;
    let out = dir.join("query-results-chart.png");
    h.driver.screenshot(&out).await.expect("shot");
    println!("captured query-results-chart -> {}", out.display());

    // The user menu + scopes drawer.
    h.goto("/").await;
    h.wait_css("#user-menu-trigger button")
        .await
        .click()
        .await
        .expect("open the user menu");
    h.wait_css(".thaw-popover-surface").await;
    let out = dir.join("user-menu.png");
    h.driver.screenshot(&out).await.expect("shot");
    println!("captured user-menu -> {}", out.display());
    h.wait_xpath("//button[contains(., 'View scopes')]")
        .await
        .click()
        .await
        .expect("open the scopes drawer");
    h.wait_xpath("//*[contains(., 'Access scopes')]").await;
    let out = dir.join("scopes-drawer.png");
    h.driver.screenshot(&out).await.expect("shot");
    println!("captured scopes-drawer -> {}", out.display());

    // Dark mode (one representative capture; the toggle persists, so flip
    // back afterwards to leave the session light for any later steps).
    h.goto("/").await;
    h.wait_css("button[aria-label='Toggle dark mode']")
        .await
        .click()
        .await
        .expect("dark on");
    h.wait_css("html.dark").await;
    let out = dir.join("dashboard-dark.png");
    h.driver.screenshot(&out).await.expect("shot");
    println!("captured dashboard-dark -> {}", out.display());
    h.wait_css("button[aria-label='Toggle dark mode']")
        .await
        .click()
        .await
        .expect("dark off");

    h.finish().await;
}
