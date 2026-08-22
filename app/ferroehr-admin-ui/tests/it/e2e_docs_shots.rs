// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "a browser journey asserts by panicking, and the shared harness panics when a configured stack cannot be driven"
)]
#![allow(
    clippy::print_stdout,
    reason = "the skip-with-reason and progress lines ARE this suite's report"
)]
#![allow(
    unreachable_pub,
    dead_code,
    reason = "the shared `common` harness is compiled into every journey binary; each one drives a different subset of it"
)]
// A capture pass, not an assertive journey: it drives the console and writes
// the canonical per-screen screenshots the website book embeds. Gated behind
// UI_E2E_DOCS_SHOTS so the normal E2E run (and plain `cargo nextest`) skips it.
//! Documentation-screenshot capture — one full-window PNG per console screen,
//! written directly under `website/book/src/admin-ui/img/`. Run via
//! `scripts/ui-e2e.sh` with `UI_E2E_DOCS_SHOTS` set; skips with a printed
//! reason when the harness environment or the gate flag is absent.

use crate::common;

use reqwest::StatusCode;

use std::path::{Path, PathBuf};

use common::{Harness, env, login_basic, login_basic_as};
use thirtyfour::prelude::*;

/// The detail-route id of the fixture template the browse journeys upload; its
/// detail screen is captured when the template is present on the stack.
const TEMPLATE_ID: &str = "minimal_evaluation.en.v1";

/// The ADL2 artefact the `e2e_adl2` journeys upload; its detail screen is
/// captured when the template is present on the stack.
const ADL2_TEMPLATE_ID: &str = "openEHR-EHR-COMPOSITION.cnf_adl2_versioned.v1.0.0";

/// The website book's screenshot directory (`website/book/src/admin-ui/img`),
/// resolved from this crate's manifest dir (`app/ferroehr-admin-ui`).
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
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).expect("create the section dir");
    }
    h.driver
        .screenshot(&out)
        .await
        .expect("write the documentation screenshot");
    println!("captured {slug} -> {}", out.display());
}

/// Write a full-window PNG for an already-prepared page state.
async fn shot_to(h: &Harness, dir: &Path, slug: &str) {
    let out = dir.join(format!("{slug}.png"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).expect("create the section dir");
    }
    h.driver.screenshot(&out).await.expect("shot");
    println!("captured {slug} -> {}", out.display());
}

/// Capture the canonical documentation screenshots for every console screen.
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one linear capture script over every console view — sectioning it would obscure the walkthrough order"
)]
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
    let login_out = dir.join("login").join("login.png");
    std::fs::create_dir_all(login_out.parent().expect("parent")).expect("dir");
    h.driver
        .screenshot(&login_out)
        .await
        .expect("write the login screenshot");
    println!("captured login -> {}", login_out.display());

    login_basic(&h).await;

    // The authenticated screens, each with a stable content marker so the shot
    // is taken after the screen's primary content has rendered.
    capture(&h, &dir, "/", "dashboard/dashboard", None).await;
    capture(
        &h,
        &dir,
        "/templates",
        "templates/templates",
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
            "templates/template-detail",
            Some("ul.text-sm li"),
        )
        .await;
    } else {
        println!(
            "TODO docs-shots: template-detail skipped — `{TEMPLATE_ID}` not present on the stack \
             (run the browse journeys first to seed it)"
        );
    }

    // The ADL2 family of the same screen: the listing with its source-upload
    // card, then the detail's version bar + stored-source pane. Both need the
    // ADL2 fixture present (`e2e_adl2` uploads it earlier in the same stacked
    // run).
    capture(
        &h,
        &dir,
        "/templates?family=adl2",
        "templates/templates-adl2",
        Some("#adl2-upload-submit"),
    )
    .await;
    if h.driver
        .find(By::Css(format!(
            "a[href='/templates/adl2/{ADL2_TEMPLATE_ID}']"
        )))
        .await
        .is_ok()
    {
        capture(
            &h,
            &dir,
            &format!("/templates/adl2/{ADL2_TEMPLATE_ID}"),
            "templates/template-adl2-detail",
            Some("#adl2-source-pane pre"),
        )
        .await;
    } else {
        println!(
            "SKIP docs-shots: template-adl2-detail — `{ADL2_TEMPLATE_ID}` not present on the \
             stack (run the e2e_adl2 journeys first to seed it)"
        );
    }

    // Stored queries: FIRST the true empty state (fresh database), then the
    // populated screen — three stored queries seeded over the Definition API
    // under TWO different namespaces, so the derived namespace grouping (and
    // the dashboard's namespace tiles) have something real to show. Nothing is
    // created through a group form any more: a query's group IS its namespace.
    if let (Some(cdr), Some(user), Some(pass)) = (
        env("UI_E2E_CDR_URL"),
        env("UI_E2E_BASIC_USER"),
        env("UI_E2E_BASIC_PASS"),
    ) {
        let http = reqwest::Client::new();
        // Self-cleaning: earlier runs may have seeded these — delete first
        // (the admin extension endpoint; 404 = already absent) so the
        // empty-state capture is honest.
        for name in [
            "org.example::recent-compositions",
            "org.example::quantity-series",
            "ehr.demo::cohort-watch",
        ] {
            let status = http
                .delete(format!(
                    "{cdr}/ferroehr/rest/openehr/v1/admin/query/{name}/1.0.0"
                ))
                .basic_auth("ferroehr-admin", Some("ferroehr"))
                .send()
                .await
                .expect("clean stored query")
                .status();
            assert!(
                status == StatusCode::NO_CONTENT || status == StatusCode::NOT_FOUND,
                "stored-query cleanup -> {status}"
            );
        }
        capture(&h, &dir, "/queries", "queries/queries-empty", None).await;
        for (name, aql) in [
            (
                "org.example::recent-compositions",
                "SELECT c/uid/value AS uid, c/context/start_time/value AS time                  FROM EHR e CONTAINS COMPOSITION c                  ORDER BY c/context/start_time/value DESC LIMIT 20",
            ),
            (
                "org.example::quantity-series",
                "SELECT c/context/start_time/value AS time,                  c/content[openEHR-EHR-EVALUATION.minimal.v1]/data[at0001]/items[at0002]/value/magnitude AS magnitude                  FROM EHR e CONTAINS COMPOSITION c",
            ),
            // A SECOND namespace, so the grouping renders more than one card
            // (`ehr::…` is one of the spec's own qualified-name examples).
            (
                "ehr.demo::cohort-watch",
                "SELECT COUNT(*) AS cohort FROM EHR e CONTAINS COMPOSITION c",
            ),
        ] {
            let status = http
                .put(format!(
                    "{cdr}/ferroehr/rest/openehr/v1/definition/query/{name}/1.0.0"
                ))
                .basic_auth(&user, Some(&pass))
                .header("Content-Type", "text/plain")
                .body(aql)
                .send()
                .await
                .expect("seed stored query")
                .status();
            assert!(status.is_success(), "stored-query seed -> {status}");
        }
        // Capture the populated screen: rows + open-in-editor links on the
        // left, and the DERIVED namespace cards on the right (both seeded
        // namespaces present — no group was created by hand).
        h.goto("/queries").await;
        h.wait_css("a[href^='/queries/aql?load=']").await;
        h.wait_css("[data-query-namespace=\"org.example\"]").await;
        h.wait_css("[data-query-namespace=\"ehr.demo\"]").await;
        shot_to(&h, &dir, "queries/queries").await;
        // Re-capture the DASHBOARD now that stored queries exist: its namespace
        // tiles are derived from this same listing, and the first pass (before
        // seeding) could only show the empty state.
        h.goto("/").await;
        h.wait_css("footer").await;
        h.wait_css("[data-namespace-tile]").await;
        shot_to(&h, &dir, "dashboard/dashboard").await;
    } else {
        capture(&h, &dir, "/queries", "queries/queries-empty", None).await;
        println!("SKIP docs-shots: stored-query seeding needs UI_E2E_CDR_URL/UI_E2E_BASIC_*");
    }
    capture(
        &h,
        &dir,
        "/queries/builder",
        "queries/query-builder",
        Some("#qb-template"),
    )
    .await;
    capture(
        &h,
        &dir,
        "/queries/aql",
        "queries/query-aql",
        Some("#aql-editor"),
    )
    .await;
    capture(&h, &dir, "/ehrs", "ehrs/ehrs", Some("#ehr-lookup")).await;
    // The demographics party browser — the kind switcher plus the create card
    // render on an empty stack, which is what the book's walkthrough opens on.
    capture(
        &h,
        &dir,
        "/demographics/person",
        "demographics/demographics",
        Some("#party-lookup"),
    )
    .await;
    // The terminology browser, with a terminology already selected so the
    // descriptor card has content. The surface is config-gated on the CDR side
    // (`[terminology] api_enabled`), and a stack that serves it disabled renders
    // one empty-state card — which is not what this page documents, so the
    // capture is skipped with a reason rather than publishing the wrong screen.
    h.goto("/terminology?terminology=openehr").await;
    h.wait_css("footer").await;
    if h.driver
        .find(By::Css("#terminology-descriptor"))
        .await
        .is_ok()
    {
        shot_to(&h, &dir, "terminology/terminology").await;
    } else {
        println!(
            "SKIP docs-shots: terminology not captured — the CDR under test serves \
             [terminology] api_enabled = false"
        );
    }
    capture(&h, &dir, "/system", "system/system", None).await;

    // The operations panel: the base view (dependency health, build provenance,
    // the metric tiles, log control), then the metric browser showing one
    // metric's samples. Probe-gated on the CDR's management surface, which the
    // composed stack enables (docker/ferroehr.dev.toml).
    capture(
        &h,
        &dir,
        "/operations",
        "operations/operations",
        Some("#ops-build-info"),
    )
    .await;
    h.goto("/operations?metric=db_pool_connections").await;
    h.wait_css("footer").await;
    let detail = h.wait_css("#ops-metric-detail").await;
    // The metrics card sits below the fold on the capture window, and a
    // screenshot is the VIEWPORT: scroll the samples table into view or the
    // published shot shows the top of the page twice.
    detail
        .scroll_into_view()
        .await
        .expect("scroll to the metric samples");
    shot_to(&h, &dir, "operations/operations-metric").await;

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
        shot_to(&h, &dir, "ehrs/compositions/list").await;
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}/compositions/{vo_id}"),
            "ehrs/compositions/viewer",
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
        // EHR detail: the status tab (URL-driven tab state) — the current
        // document plus the edit form. The marker is the ENABLED save: the
        // edit card is mounted before its document loads and stays disabled
        // until seeded, so waiting on the card alone would publish a dimmed,
        // half-loaded form.
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}?tab=status"),
            "ehrs/status/status",
            Some("#status-save:not([disabled])"),
        )
        .await;
        // EHR detail: the VERSIONED_EHR_STATUS history tab — the revision
        // history table and the at-time lookup.
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}?tab=status-history"),
            "ehrs/status/history",
            Some("[data-status-version]"),
        )
        .await;
        // EHR detail: the contributions table (needs the extension endpoint).
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}?tab=contributions"),
            "ehrs/contributions/contributions",
            Some("table tbody"),
        )
        .await;
        // EHR detail: the directory tab (the create-empty state — the seeded
        // EHR has no directory).
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}?tab=directory"),
            "ehrs/directory/create",
            Some("#directory-create"),
        )
        .await;
        // EHR detail: the POPULATED directory view — create the empty root
        // (the only create path), then build one sub-folder in the tree editor
        // and commit it, so the published shot shows a real tree rather than a
        // bare root (owner directive: every possible view, including directory
        // both before and after it exists).
        h.wait_css("#directory-create")
            .await
            .click()
            .await
            .expect("create the empty directory");
        h.wait_css("#directory-edit").await;
        h.wait_css("[aria-label='Add subfolder']")
            .await
            .click()
            .await
            .expect("add a subfolder at the root");
        h.wait_css("#directory-save")
            .await
            .click()
            .await
            .expect("commit the subfolder");
        // The toast overlays the bottom-right corner; let it clear before the
        // shot and before the history panel click below.
        h.wait_toasts_cleared().await;
        shot_to(&h, &dir, "ehrs/directory/directory").await;
        // The version-history panel open (the new read-side toolbar).
        h.wait_xpath("//button[contains(normalize-space(.), 'Version history')]")
            .await
            .click()
            .await
            .expect("open the version history panel");
        h.wait_xpath("//button[contains(normalize-space(.), 'v1')]")
            .await;
        shot_to(&h, &dir, "ehrs/directory/history").await;
        // EHR detail: the commit-composition form (scrolled into view).
        h.goto(&format!("/ehrs/{ehr_id}?tab=compositions")).await;
        let commit_body = h.wait_css("#commit-body").await;
        commit_body
            .scroll_into_view()
            .await
            .expect("scroll to the commit form");
        shot_to(&h, &dir, "ehrs/compositions/commit").await;
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
        shot_to(&h, &dir, "ehrs/compositions/editor").await;
        // EHR detail: the tag browser, with one real tag on it — set through
        // the composition viewer's own panel, so the published shot shows a
        // populated group rather than the empty state.
        h.wait_css("#tag-key")
            .await
            .send_keys("reviewed")
            .await
            .expect("type the tag key");
        h.wait_css("#tag-value")
            .await
            .send_keys("true")
            .await
            .expect("type the tag value");
        h.wait_css("#tag-save")
            .await
            .click()
            .await
            .expect("save the tag");
        h.wait_css("[data-tag-key='reviewed']").await;
        h.wait_toasts_cleared().await;
        capture(
            &h,
            &dir,
            &format!("/ehrs/{ehr_id}?tab=tags"),
            "ehrs/tags/tags",
            Some("[data-tag-key='reviewed']"),
        )
        .await;
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
    // Run is disabled until the typed AQL reaches the signal; a click before
    // then is intercepted by the toolbar, so the wait carries the condition.
    h.wait_clickable_xpath("//button[normalize-space(.)='Run']")
        .await
        .click()
        .await
        .expect("run");
    let export_button = h.wait_xpath("//button[contains(., 'Export CSV')]").await;
    // The results section renders below the editor/params cards — scroll it
    // into view so the capture actually shows rows, not just the header.
    export_button
        .scroll_into_view()
        .await
        .expect("scroll to the results");
    h.wait_css("table tbody tr").await;
    shot_to(&h, &dir, "queries/query-aql-results").await;
    h.wait_xpath("//button[normalize-space(.)='Chart']")
        .await
        .click()
        .await
        .expect("chart toggle");
    let chart = h
        .wait_css("svg.chartistry_chart, div.overflow-x-auto svg")
        .await;
    chart.scroll_into_view().await.expect("scroll to the chart");
    shot_to(&h, &dir, "queries/query-results-chart").await;

    // The user menu + the access drawer (identity, policy source, session
    // grants, scope previewer).
    h.goto("/").await;
    h.wait_css("#user-menu-trigger button")
        .await
        .click()
        .await
        .expect("open the user menu");
    h.wait_css(".thaw-popover-surface").await;
    shot_to(&h, &dir, "dashboard/user-menu").await;
    h.wait_xpath("//button[contains(., 'View scopes')]")
        .await
        .click()
        .await
        .expect("open the scopes drawer");
    h.wait_css("#access-drawer").await;
    // Fill the previewer so the published capture shows what it is FOR: two
    // parsed master08 grants, not an empty field.
    h.wait_css("#scope-previewer-input")
        .await
        .send_keys("patient/composition-*.rs user/template-MyHospital::Template.v0.crud")
        .await
        .expect("preview two scopes");
    h.wait_css("#scope-preview-results [data-scope-grant='resource']")
        .await;
    shot_to(&h, &dir, "dashboard/scopes-drawer").await;

    // Dark mode (one representative capture; the toggle persists, so flip
    // back afterwards to leave the session light for any later steps).
    h.goto("/").await;
    h.wait_css("button[aria-label='Toggle dark mode']")
        .await
        .click()
        .await
        .expect("dark on");
    h.wait_css("html.dark").await;
    // The tiles animate `transition-colors`; a capture racing the token
    // switch freezes a half-themed frame. Fixed settle for the CSS
    // transition duration — an animation wait, not a condition wait.
    tokio::time::sleep(std::time::Duration::from_millis(700)).await;
    shot_to(&h, &dir, "dashboard/dashboard-dark").await;
    h.wait_css("button[aria-label='Toggle dark mode']")
        .await
        .click()
        .await
        .expect("dark off");

    h.finish().await;

    // The audit-log screen is admin-only (the ATNA trail is an operator
    // surface), so its captures run in a fresh session as the quickstart
    // admin user: first the POPULATED table (the stack's own audited
    // activity — every journey request lands in the trail), then the
    // first-class EMPTY state via a filter that cannot match, so the book
    // shows both faces of the screen.
    let Some(h) = Harness::start("docs-shots-audit").await else {
        return;
    };
    let admin_user = env("UI_E2E_ADMIN_USER").unwrap_or_else(|| "ferroehr-admin".to_owned());
    let admin_pass = env("UI_E2E_ADMIN_PASS").unwrap_or_else(|| "ferroehr".to_owned());
    login_basic_as(&h, &admin_user, &admin_pass).await;
    capture(&h, &dir, "/audit", "audit/audit", Some("table tbody tr")).await;

    // The raw-record view: open the first row's disclosure and capture the
    // full stored FHIR AuditEvent as the reader will see it.
    h.wait_css("tbody tr details summary")
        .await
        .click()
        .await
        .expect("open the raw record");
    h.wait_css("tbody tr details pre").await;
    shot_to(&h, &dir, "audit/audit-record").await;

    h.goto("/audit?patient=docs-shots-no-such-patient").await;
    h.wait_css("footer").await;
    h.wait_xpath("//*[contains(text(), 'No audit records match')]")
        .await;
    shot_to(&h, &dir, "audit/audit-empty").await;

    // ── The ADMIN destructive operations. They are probe-gated on the CDR's
    //    admin API, so they render only for this ADMIN session — never in the
    //    plain-user pass above, which is exactly why they are captured here.
    capture(
        &h,
        &dir,
        "/templates",
        "templates/templates-admin-delete",
        Some("[data-template-delete]"),
    )
    .await;
    // The stored-query row's CDR delete needs a stored query on the stack (the
    // pass above seeds two over the Definition API).
    h.goto("/queries").await;
    h.wait_css("footer").await;
    if h.driver.find(By::Css("[data-query-delete]")).await.is_ok() {
        shot_to(&h, &dir, "queries/queries-admin-delete").await;
    } else {
        println!("SKIP docs-shots: no stored query on the stack to show the CDR delete on");
    }
    if let Some(ehr_id) = env("UI_E2E_SEEDED_EHR_ID") {
        // Captured with the confirmation MODAL open (the trigger click only —
        // never the dialog's confirm): the dialog is the informative state, it
        // spells out the EHR id and what the physical delete destroys.
        h.goto(&format!("/ehrs/{ehr_id}")).await;
        h.wait_css("#ehr-delete")
            .await
            .click()
            .await
            .expect("open the EHR delete dialog");
        // thaw's dialog is never removed from the DOM (CSSTransition hides it
        // with `display: none`), so wait for VISIBILITY — presence would let the
        // capture race the open.
        let confirm = h.wait_css("#ehr-delete-confirm").await;
        for _ in 0..75 {
            if confirm.is_displayed().await.unwrap_or(false) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        shot_to(&h, &dir, "ehrs/ehr-admin-delete").await;
    } else {
        println!("SKIP docs-shots: the EHR delete needs UI_E2E_SEEDED_EHR_ID");
    }

    // The operations panel's log-filter confirmation modal — the informative
    // state: it spells out that logging changes immediately for every request.
    // The trigger click only; the dialog is NEVER confirmed, so the stack's log
    // filter is untouched.
    h.goto("/operations").await;
    h.wait_css("#ops-log-filter")
        .await
        .send_keys("ferroehr=debug,sqlx=warn")
        .await
        .expect("type the filter directives");
    // The dialog surface is absent from the DOM until it opens, so the confirm
    // button is looked up AFTER each click — and the click is retried, because
    // one landing before hydration attaches the listener is simply lost and
    // would otherwise publish a screenshot of the closed screen.
    let mut opened = false;
    for _ in 0..10 {
        h.wait_css("#ops-log-apply")
            .await
            .click()
            .await
            .expect("open the log-filter dialog");
        for _ in 0..10 {
            if let Ok(confirm) = h.driver.find(By::Css("#ops-log-apply-confirm")).await
                && confirm.is_displayed().await.unwrap_or(false)
            {
                opened = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        if opened {
            break;
        }
    }
    if opened {
        shot_to(&h, &dir, "operations/operations-log-filter").await;
    } else {
        println!(
            "TODO docs-shots: operations/operations-log-filter not captured — the confirmation \
             dialog never opened (pre-hydration clicks exhausted)"
        );
    }

    h.finish().await;
}
