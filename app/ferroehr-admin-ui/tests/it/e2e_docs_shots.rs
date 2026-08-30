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
#![expect(
    clippy::disallowed_types,
    reason = "test fixtures posted through a REST seam are raw JSON by the testing rule \
              (.claude/rules/testing.md §Test-fixture construction)"
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

use common::{Harness, env, login_basic, login_basic_as, wait_enabled, wait_visible};
use thirtyfour::prelude::*;

/// The detail-route id of the fixture template the browse journeys upload; its
/// detail screen is captured when the template is present on the stack.
const TEMPLATE_ID: &str = "minimal_evaluation.en.v1";

/// The ADL2 artefact whose detail screen is captured; this pass seeds it
/// itself over the Definition API rather than relying on a journey's leftovers
/// (the `e2e_adl2` scenes clean up after themselves).
const ADL2_TEMPLATE_ID: &str = "openEHR-EHR-COMPOSITION.cnf_adl2_versioned.v1.0.0";

/// Store the OPT capture fixture, so the template-detail screen has something
/// to show whether or not another journey uploaded it first.
///
/// The upload is idempotent for a capture pass: `201` created, `409` already
/// there. The fixture is the same operational template `scripts/ui-e2e.sh`
/// seeds while bringing the stack up; seeding it here as well is what makes the
/// detail capture unconditional.
async fn seed_opt_fixture(cdr: &str, user: &str, pass: &str) {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/minimal_evaluation.opt"
    ))
    .expect("the OPT fixture exists");
    let status = reqwest::Client::new()
        .post(format!(
            "{cdr}/ferroehr/rest/openehr/v1/definition/template/adl1.4"
        ))
        .basic_auth(user, Some(pass))
        .header("Content-Type", "application/xml")
        .body(source)
        .send()
        .await
        .expect("seed the OPT capture fixture")
        .status();
    assert!(
        status == StatusCode::CREATED || status == StatusCode::CONFLICT,
        "OPT capture fixture seed -> {status}"
    );
}

/// Store the ADL2 capture fixture, so the ADL2 detail screen has something to
/// show whether or not the journeys ran before this pass.
///
/// The upload is idempotent for a capture pass: `201` created, `409` already
/// there. Any other answer leaves the artefact absent and the caller falls
/// back to skipping the shot with a printed reason.
async fn seed_adl2_fixture(cdr: &str, user: &str, pass: &str) {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/fixtures/adl2/opt/versioned.v1_0_0.adls"
    ))
    .expect("the ADL2 corpus fixture exists");
    let status = reqwest::Client::new()
        .post(format!(
            "{cdr}/ferroehr/rest/openehr/v1/definition/template/adl2"
        ))
        .basic_auth(user, Some(pass))
        .header("Content-Type", "text/plain")
        .body(source)
        .send()
        .await
        .expect("seed the ADL2 capture fixture")
        .status();
    println!("adl2 capture fixture seed -> {status}");
}

/// Store one FHIR mapping so the connector capture shows a populated mapping
/// store instead of its empty state.
///
/// Idempotent for a capture pass: `201` created, `409` already there. Any other
/// answer leaves the store empty and the capture simply shows that; the shot is
/// never skipped for it. It binds the same template
/// `scripts/ui-e2e.sh` seeds while bringing the stack up.
async fn seed_fhir_mapping(cdr: &str, user: &str, pass: &str) {
    let status = reqwest::Client::new()
        .post(format!("{cdr}/ferroehr/rest/openehr/v1/admin/fhir_mapping"))
        .basic_auth(user, Some(pass))
        .json(&serde_json::json!({
            "name": "observation-weight",
            "enabled": true,
            "definition": {
                "resource_type": "Observation",
                "profile_url": "http://hl7.org/fhir/StructureDefinition/vitalsigns",
                "template_id": TEMPLATE_ID,
                "subject": {
                    "reference_path": "subject.reference",
                    "namespace": "fhir",
                    "strip_prefix": "Patient/"
                },
                "context": {
                    "ctx/language": "en",
                    "ctx/territory": "US",
                    "ctx/composer_name": "fhir-connector"
                },
                "entries": [
                    {
                        "openehr_path": "minimal/minimal:0/quantity",
                        "fhir_path": "valueQuantity.value",
                        "transform": { "kind": "quantity", "unit_path": "valueQuantity.unit" }
                    }
                ]
            }
        }))
        .send()
        .await
        .expect("seed the FHIR capture fixture")
        .status();
    println!("fhir capture fixture seed -> {status}");
}

/// Store one event subscription so the subscriptions capture shows a populated
/// table instead of its empty state.
///
/// Idempotent for a capture pass: `201` created, `409` already there. Any other
/// answer leaves the table empty and the capture simply shows that; the shot is
/// never skipped for it. It binds the same template
/// `scripts/ui-e2e.sh` seeds while bringing the stack up.
async fn seed_event_subscription(cdr: &str, user: &str, pass: &str) {
    let status = reqwest::Client::new()
        .post(format!(
            "{cdr}/ferroehr/rest/openehr/v1/admin/event_subscription"
        ))
        .basic_auth(user, Some(pass))
        .json(&serde_json::json!({
            "name": "vitals-feed",
            "kind": "COMPOSITION",
            "change_type": "249",
            "template_id": TEMPLATE_ID,
            "enabled": true
        }))
        .send()
        .await
        .expect("seed the subscription capture fixture")
        .status();
    println!("event subscription capture fixture seed -> {status}");
}

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

/// Pick a `<select>` option by its value.
///
/// # Panics
/// When the control is not a select or the option is absent.
async fn pick_option(h: &Harness, css: &str, value: &str) {
    let element = h.wait_css(css).await;
    thirtyfour::components::SelectElement::new(&element)
        .await
        .expect("the control is a select")
        .select_by_value(value)
        .await
        .expect("pick the option");
}

/// Pick the first real composition in the Commit tab's amend picker (index 0 is
/// the "— pick a composition —" placeholder).
///
/// # Panics
/// When the picker holds no composition — the seeded EHR always has one.
async fn pick_composition(h: &Harness) {
    let element = h.wait_css("#stage-composition").await;
    thirtyfour::components::SelectElement::new(&element)
        .await
        .expect("the composition picker is a select")
        .select_by_index(1)
        .await
        .expect("pick the seeded composition");
}

/// The settle a themed capture needs: the console's surfaces animate
/// `transition-colors`, and a screenshot racing the token switch freezes a
/// half-themed frame. An animation wait, not a condition wait.
const THEME_SETTLE: std::time::Duration = std::time::Duration::from_millis(700);

/// Turn dark mode ON for this browser session and wait until it is applied.
///
/// The preference persists to `localStorage`, so one flip covers every screen
/// the session visits afterwards.
async fn enable_dark_mode(h: &Harness) {
    h.wait_css("button[aria-label='Toggle dark mode']")
        .await
        .click()
        .await
        .expect("turn dark mode on");
    h.wait_css("html.dark").await;
    tokio::time::sleep(THEME_SETTLE).await;
}

/// Navigate to `path` and write its DARK variant.
///
/// Every navigation waits for `html.dark` again: the class is applied from a
/// browser-only effect that re-reads the stored preference after hydration, so
/// the server pass never carries it (rules §8).
async fn capture_dark(h: &Harness, dir: &Path, path: &str, slug: &str, content: Option<&str>) {
    h.goto(path).await;
    h.wait_css("footer").await;
    h.wait_css("html.dark").await;
    if let Some(selector) = content {
        h.wait_css(selector).await;
    }
    tokio::time::sleep(THEME_SETTLE).await;
    shot_to(h, dir, slug).await;
}

/// [`capture_dark`] for a PROBE-GATED screen: skip with a printed reason when
/// the screen renders its own disabled card, exactly as the light capture does.
async fn capture_dark_gated(
    h: &Harness,
    dir: &Path,
    path: &str,
    slug: &str,
    screen_css: &str,
    disabled_css: &str,
) {
    h.goto(path).await;
    h.wait_css(screen_css).await;
    h.wait_css("html.dark").await;
    if h.driver.find(By::Css(disabled_css)).await.is_ok() {
        println!("SKIP docs-shots: {slug} not captured — the CDR under test does not serve it");
        return;
    }
    tokio::time::sleep(THEME_SETTLE).await;
    shot_to(h, dir, slug).await;
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

/// Capture the stored-query screens: FIRST the true empty state (fresh
/// database), then the populated screen and the dashboard that derives from
/// it.
///
/// Three stored queries are seeded over the Definition API under TWO different
/// namespaces, so the derived namespace grouping (and the dashboard's
/// namespace tiles) have something real to show. Nothing is created through a
/// group form: a query's group IS its namespace.
async fn capture_stored_query_screens(h: &Harness, dir: &Path, cdr: &str, user: &str, pass: &str) {
    let http = reqwest::Client::new();
    // The cleanup below hits the ADMIN extension, which the plain
    // `UI_E2E_BASIC_*` account has no ADMIN role for — so it carries the
    // admin credential, read from the environment like every journey.
    let admin_user = env("UI_E2E_ADMIN_USER").unwrap_or_else(|| "ferroehr-admin".to_owned());
    let admin_pass = env("UI_E2E_ADMIN_PASS").unwrap_or_else(|| "ferroehr".to_owned());
    // Self-cleaning: earlier runs may have seeded these — delete first (the
    // admin extension endpoint; 404 = already absent) so the empty-state
    // capture is honest.
    for name in [
        "org.example::recent-compositions",
        "org.example::quantity-series",
        "ehr.demo::cohort-watch",
    ] {
        let status = http
            .delete(format!(
                "{cdr}/ferroehr/rest/openehr/v1/admin/query/{name}/1.0.0"
            ))
            .basic_auth(&admin_user, Some(&admin_pass))
            .send()
            .await
            .expect("clean stored query")
            .status();
        assert!(
            status == StatusCode::NO_CONTENT || status == StatusCode::NOT_FOUND,
            "stored-query cleanup -> {status}"
        );
    }
    capture(h, dir, "/queries", "queries/queries-empty", None).await;
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
            .basic_auth(user, Some(pass))
            .header("Content-Type", "text/plain")
            .body(aql)
            .send()
            .await
            .expect("seed stored query")
            .status();
        assert!(status.is_success(), "stored-query seed -> {status}");
    }
    // Capture the populated screen: rows + open-in-editor links on the left,
    // and the DERIVED namespace cards on the right (both seeded namespaces
    // present — no group was created by hand).
    h.goto("/queries").await;
    h.wait_css("a[href^='/queries/aql?load=']").await;
    h.wait_css("[data-query-namespace=\"org.example\"]").await;
    h.wait_css("[data-query-namespace=\"ehr.demo\"]").await;
    shot_to(h, dir, "queries/queries").await;
    // Re-capture the DASHBOARD now that stored queries exist: its namespace
    // tiles are derived from this same listing, and the first pass (before
    // seeding) could only show the empty state.
    h.goto("/").await;
    h.wait_css("footer").await;
    h.wait_css("[data-namespace-tile]").await;
    shot_to(h, dir, "dashboard/dashboard").await;
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

    // Both template families' capture fixtures, seeded by this pass itself so
    // every template screen is captured unconditionally rather than depending
    // on what an earlier journey happened to leave behind.
    if let (Some(cdr), Some(user), Some(pass)) = (
        env("UI_E2E_CDR_URL"),
        env("UI_E2E_BASIC_USER"),
        env("UI_E2E_BASIC_PASS"),
    ) {
        seed_opt_fixture(&cdr, &user, &pass).await;
        seed_adl2_fixture(&cdr, &user, &pass).await;
    }

    // The authenticated screens, each with a stable content marker so the shot
    // is taken after the screen's primary content has rendered.
    capture(&h, &dir, "/", "dashboard/dashboard", None).await;
    capture(
        &h,
        &dir,
        "/templates",
        "templates/templates",
        Some("#template-upload-open"),
    )
    .await;
    capture(
        &h,
        &dir,
        &format!("/templates/{TEMPLATE_ID}"),
        "templates/template-detail",
        Some("ul.text-sm li"),
    )
    .await;

    // The ADL2 family of the same screen: the listing with its source-upload
    // card, then the detail's version bar + stored-source pane.
    capture(
        &h,
        &dir,
        "/templates?family=adl2",
        "templates/templates-adl2",
        Some("#template-upload-open"),
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
             stack (the seed needs UI_E2E_CDR_URL + UI_E2E_BASIC_*)"
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
        capture_stored_query_screens(&h, &dir, &cdr, &user, &pass).await;
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
        // wait for the seeded composition's link. Two more markers, because
        // both are published surfaces in their own right: the header's
        // identity strip (subject + capability badges) and the row-filter bar.
        h.goto(&format!("/ehrs/{ehr_id}")).await;
        h.wait_css("footer").await;
        h.wait_css("#ehr-identity").await;
        h.wait_xpath("//a[contains(., 'Compositions')]")
            .await
            .click()
            .await
            .expect("open the compositions tab");
        h.wait_css("#compositions-filter").await;
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
        // EHR detail: the Commit tab with TWO changes staged — a composition
        // amend and the EHR status modification, both seeded from the CDR, so
        // the published shot shows a real change set rather than an empty
        // staging area. Nothing is committed: staging is session-only, so the
        // seeded EHR the later captures depend on is untouched.
        h.goto(&format!("/ehrs/{ehr_id}?tab=commit")).await;
        h.wait_css("#stage-notice").await;
        pick_option(&h, "#stage-kind", "amend").await;
        pick_composition(&h).await;
        wait_enabled(&h, "#stage-body").await;
        h.wait_css("#stage-add-change")
            .await
            .click()
            .await
            .expect("stage the composition amendment");
        pick_option(&h, "#stage-kind", "status").await;
        wait_enabled(&h, "#stage-body").await;
        h.wait_css("#stage-add-change")
            .await
            .click()
            .await
            .expect("stage the status modification");
        h.wait_css("#stage-description")
            .await
            .send_keys("Encounter amended; EHR status refreshed")
            .await
            .expect("describe the change set");
        h.wait_css("#stage-list")
            .await
            .scroll_into_view()
            .await
            .expect("scroll to the staging list");
        shot_to(&h, &dir, "ehrs/contributions/commit").await;
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

    // Dark mode: flipped ONCE (the preference persists for the rest of this
    // session) and then swept over the same screens, so the book documents the
    // whole console in both themes rather than one representative shot. The
    // sweep is this session's last act, so the theme is never flipped back.
    h.goto("/").await;
    enable_dark_mode(&h).await;
    shot_to(&h, &dir, "dashboard/dashboard-dark").await;
    dark_ordinary_screens(&h, &dir).await;

    h.finish().await;

    capture_admin_screens(&dir).await;
}

/// The DARK sweep over the screens the ORDINARY session can read.
///
/// Its admin-gated other half is [`dark_admin_screens`] — the same split the
/// light captures make, and for the same reason: a screen reading an `/admin`
/// route answers this session `403`, so a capture taken here would publish the
/// console's refusal card.
async fn dark_ordinary_screens(h: &Harness, dir: &Path) {
    capture_dark(
        h,
        dir,
        "/templates",
        "templates/templates-dark",
        Some("#template-upload-open"),
    )
    .await;
    capture_dark(
        h,
        dir,
        "/queries/builder",
        "queries/query-builder-dark",
        Some("#qb-template"),
    )
    .await;
    capture_dark(h, dir, "/ehrs", "ehrs/ehrs-dark", Some("#ehr-lookup")).await;
    capture_dark(
        h,
        dir,
        "/demographics/person",
        "demographics/demographics-dark",
        Some("#party-lookup"),
    )
    .await;
    capture_dark(
        h,
        dir,
        "/operations",
        "operations/operations-dark",
        Some("#ops-build-info"),
    )
    .await;
    // The terminology surface is config-gated on the CDR side and answers as
    // if unmounted when off; its READY marker is the descriptor card, so the
    // gate here is presence rather than a disabled card.
    h.goto("/terminology?terminology=openehr").await;
    h.wait_css("footer").await;
    h.wait_css("html.dark").await;
    if h.driver
        .find(By::Css("#terminology-descriptor"))
        .await
        .is_ok()
    {
        tokio::time::sleep(THEME_SETTLE).await;
        shot_to(h, dir, "terminology/terminology-dark").await;
    } else {
        println!(
            "SKIP docs-shots: terminology-dark not captured — the CDR under test serves \
             [terminology] api_enabled = false"
        );
    }
    // One EHR-detail tab, so the master-detail chrome is documented dark too.
    if let Some(ehr_id) = env("UI_E2E_SEEDED_EHR_ID") {
        capture_dark(
            h,
            dir,
            &format!("/ehrs/{ehr_id}?tab=compositions"),
            "ehrs/compositions/list-dark",
            Some("#compositions-filter"),
        )
        .await;
    } else {
        println!("SKIP docs-shots: the dark EHR-detail capture needs UI_E2E_SEEDED_EHR_ID");
    }
}

/// The DARK sweep over the ADMIN-gated screens, run inside the admin session
/// that captured their light variants.
async fn dark_admin_screens(h: &Harness, dir: &Path) {
    h.goto("/").await;
    enable_dark_mode(h).await;
    capture_dark(h, dir, "/system", "system/system-dark", None).await;
    capture_dark(h, dir, "/audit", "audit/audit-dark", Some("table tbody tr")).await;
    capture_dark_gated(
        h,
        dir,
        "/tenants",
        "tenants/tenants-dark",
        "#tenants-screen",
        "#tenants-disabled",
    )
    .await;
    capture_dark_gated(
        h,
        dir,
        "/subscriptions",
        "subscriptions/subscriptions-dark",
        "#subscriptions-screen",
        "#subscriptions-disabled",
    )
    .await;
    capture_dark_gated(
        h,
        dir,
        "/fhir",
        "fhir/fhir-dark",
        "#fhir-screen",
        "#fhir-disabled",
    )
    .await;
}

/// Capture every screen whose data the CDR classes as ADMIN work, in one
/// session signed in as the admin dev user.
///
/// **Every admin-gated capture belongs here, and nowhere else.** The main pass
/// signs in as the ORDINARY dev user, and a screen reading an `/admin` route
/// answers that session `403` — so a capture taken there publishes the console's
/// refusal card instead of the screen the book documents. That is not
/// hypothetical: the committed `/tenants` shot was exactly that (issue #2578),
/// and `/system`'s runtime-configuration card and the whole `/fhir` screen have
/// the same shape. One session, one place to add the next one.
///
/// A fresh [`Harness`] rather than a re-login: a new browser session starts with
/// no console cookie, so the admin sign-in cannot land on top of the ordinary
/// one. The main pass has already finished by the time this runs, so nothing
/// later depends on the ordinary session.
#[expect(
    clippy::too_many_lines,
    reason = "one linear capture script over the admin-gated views — sectioning it would obscure the walkthrough order"
)]
async fn capture_admin_screens(dir: &Path) {
    let Some(h) = Harness::start("docs-shots-admin").await else {
        return;
    };
    let admin_user = env("UI_E2E_ADMIN_USER").unwrap_or_else(|| "ferroehr-admin".to_owned());
    let admin_pass = env("UI_E2E_ADMIN_PASS").unwrap_or_else(|| "ferroehr".to_owned());
    login_basic_as(&h, &admin_user, &admin_pass).await;

    // The audit-log screen is admin-only (the ATNA trail is an operator
    // surface): first the POPULATED table (the stack's own audited activity —
    // every journey request lands in the trail), then the first-class EMPTY
    // state via a filter that cannot match, so the book shows both faces of the
    // screen.
    capture(&h, dir, "/audit", "audit/audit", Some("table tbody tr")).await;

    // The raw-record view: open the first row's disclosure and capture the
    // full stored FHIR AuditEvent as the reader will see it.
    h.wait_css("tbody tr details summary")
        .await
        .click()
        .await
        .expect("open the raw record");
    h.wait_css("tbody tr details pre").await;
    shot_to(&h, dir, "audit/audit-record").await;

    h.goto("/audit?patient=docs-shots-no-such-patient").await;
    h.wait_css("footer").await;
    h.wait_xpath("//*[contains(text(), 'No audit records match')]")
        .await;
    shot_to(&h, dir, "audit/audit-empty").await;

    // The System screen: its runtime-configuration card reads `GET
    // /admin/config`, so the ordinary session gets a refusal there — below the
    // capture fold today, which is exactly how it stayed unnoticed.
    capture(&h, dir, "/system", "system/system", None).await;

    // The tenant registry: probe-gated on the CDR's tenancy extension, which
    // the E2E stack enables (docker/admin-ui/e2e-env.yml). Absent, the screen
    // renders its disabled card, which is not what the book documents.
    h.goto("/tenants").await;
    h.wait_css("#tenants-screen").await;
    if h.driver.find(By::Css("#tenants-disabled")).await.is_ok() {
        println!(
            "SKIP docs-shots: tenants not captured — the CDR under test runs with \
             [tenancy] enabled = false"
        );
    } else {
        shot_to(&h, dir, "tenants/tenants").await;
    }

    // The event subscriptions: probe-gated on `[events] admin_api`, which the
    // E2E stack enables (docker/admin-ui/e2e-env.yml). The capture seeds one
    // subscription first, so the book shows a populated table rather than the
    // empty state the journeys leave behind.
    if let Some(cdr) = env("UI_E2E_CDR_URL") {
        seed_event_subscription(&cdr, &admin_user, &admin_pass).await;
    }
    h.goto("/subscriptions").await;
    h.wait_css("#subscriptions-screen").await;
    if h.driver
        .find(By::Css("#subscriptions-disabled"))
        .await
        .is_ok()
    {
        println!(
            "SKIP docs-shots: subscriptions not captured — the CDR under test runs with \
             [events] admin_api = false"
        );
    } else {
        shot_to(&h, dir, "subscriptions/subscriptions").await;
    }

    // The FHIR connector: probe-gated on `[fhir] api_enabled`, which the E2E
    // stack enables (docker/admin-ui/e2e-env.yml). The capture seeds one
    // mapping first, so the book shows a populated store rather than the empty
    // state the journeys leave behind.
    if let Some(cdr) = env("UI_E2E_CDR_URL") {
        seed_fhir_mapping(&cdr, &admin_user, &admin_pass).await;
    }
    h.goto("/fhir").await;
    h.wait_css("#fhir-screen").await;
    if h.driver.find(By::Css("#fhir-disabled")).await.is_ok() {
        println!(
            "SKIP docs-shots: fhir not captured — the CDR under test runs with \
             [fhir] api_enabled = false"
        );
    } else {
        let dry_run = h.wait_css("#fhir-dry-run").await;
        shot_to(&h, dir, "fhir/fhir").await;
        // The two verification panels sit below the fold on the capture window,
        // and a screenshot is the VIEWPORT: scroll them into view for their own
        // shot, or the book documents them with a picture of the store.
        dry_run
            .scroll_into_view()
            .await
            .expect("scroll to the verification panels");
        shot_to(&h, dir, "fhir/fhir-verify").await;
    }

    // ── The ADMIN destructive operations. They are probe-gated on the CDR's
    //    admin API, so they render only for an ADMIN session.
    capture(
        &h,
        dir,
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
        shot_to(&h, dir, "queries/queries-admin-delete").await;
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
        wait_visible(&h, "#ehr-delete-confirm").await;
        shot_to(&h, dir, "ehrs/ehr-admin-delete").await;
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
    // One click, no retry: `Harness::goto` waits for the shell's hydration
    // marker, so the listener is attached before anything here runs — a dialog
    // that does not open is a defect to fail on, never a shot to skip.
    h.wait_css("#ops-log-apply")
        .await
        .click()
        .await
        .expect("open the log-filter dialog");
    wait_visible(&h, "#ops-log-apply-confirm").await;
    shot_to(&h, dir, "operations/operations-log-filter").await;

    dark_admin_screens(&h, dir).await;

    h.finish().await;
}
