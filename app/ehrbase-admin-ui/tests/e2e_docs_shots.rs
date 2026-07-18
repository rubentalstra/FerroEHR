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
        h.wait_xpath("//button[contains(., 'Compositions')]")
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

    h.finish().await;
}
