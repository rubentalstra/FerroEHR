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
// e2e journeys are assertive by design; skip-with-reason prints; the shared
// harness module is per-test-binary (the corpus.rs test-file precedent)
//! End-to-end journey over the console's shared TABLE PAGINATION footer, driven
//! on the stored-queries screen (`/queries`).
//!
//! The footer's whole contract is that the page window is URL state, not a
//! private signal: paging forward writes `?page=`, the row window follows the
//! URL (so the browser's back/forward and a reload land on the same rows), a
//! page link preserves the screen's other parameters (`?size=` here), and the
//! first page is the plain path again. The journey asserts exactly that —
//! the URL, the rendered window, and the `x–y of N` range line.
//!
//! Fixtures: two stored queries of its own, seeded and removed over the
//! Definition API (`UI_E2E_CDR_URL`) — test setup deliberately bypasses the UI,
//! whose save/delete paths are covered by `e2e_admin_ops`. `?size=1` makes two
//! rows enough to prove the window moves, which keeps the journey cheap and
//! independent of whatever else is on the stack.

use crate::common;

use std::time::Duration;

use reqwest::StatusCode;

use common::{Harness, env, login_basic};
use thirtyfour::prelude::*;

/// The two stored queries this journey owns, qualified `namespace::name` (the
/// namespace also keeps them out of every other journey's grouping card).
const QUERY_NAMES: [&str; 2] = ["e2e.paging::page-alpha", "e2e.paging::page-beta"];

/// Their stored version — explicit, so the seed and the cleanup address the
/// exact `(name, version)` pair.
const QUERY_VERSION: &str = "1.0.0";

/// The AQL the fixtures hold; the journey never runs them, it only lists them.
const SEED_AQL: &str = "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c";

/// The admin dev user (quickstart `docker/ferroehr.dev.toml`) — the fixture
/// cleanup goes through the RBAC-gated admin stored-query delete.
fn admin_credentials() -> (String, String) {
    (
        env("UI_E2E_ADMIN_USER").unwrap_or_else(|| "ferroehr-admin".to_owned()),
        env("UI_E2E_ADMIN_PASS").unwrap_or_else(|| "ferroehr".to_owned()),
    )
}

/// Remove the journey's fixtures from the CDR store (404 = already absent), so
/// the journey both starts and ends from a known state.
///
/// # Panics
/// On any answer other than `204`/`404`.
async fn delete_fixtures(cdr: &str) {
    let http = reqwest::Client::new();
    let (user, pass) = admin_credentials();
    for name in QUERY_NAMES {
        let status = http
            .delete(format!(
                "{cdr}/ferroehr/rest/openehr/v1/admin/query/{name}/{QUERY_VERSION}"
            ))
            .basic_auth(&user, Some(&pass))
            .send()
            .await
            .expect("delete a stored-query fixture")
            .status();
        assert!(
            status == StatusCode::NO_CONTENT || status == StatusCode::NOT_FOUND,
            "stored-query cleanup -> {status}"
        );
    }
}

/// Store the journey's fixtures over the Definition API.
///
/// # Panics
/// When the CDR refuses the store.
async fn seed_fixtures(cdr: &str, user: &str, pass: &str) {
    let http = reqwest::Client::new();
    for name in QUERY_NAMES {
        let status = http
            .put(format!(
                "{cdr}/ferroehr/rest/openehr/v1/definition/query/{name}/{QUERY_VERSION}"
            ))
            .basic_auth(user, Some(pass))
            .header("Content-Type", "text/plain")
            .body(SEED_AQL)
            .send()
            .await
            .expect("seed a stored-query fixture")
            .status();
        assert!(status.is_success(), "stored-query seed -> {status}");
    }
}

/// How many stored-query rows the table currently renders.
async fn row_count(h: &Harness) -> usize {
    h.driver
        .find_all(By::Css("[data-stored-query]"))
        .await
        .unwrap_or_default()
        .len()
}

/// The `name@version` key of the first rendered row — the window's identity.
///
/// # Panics
/// When no row is rendered within the wait budget.
async fn first_row_key(h: &Harness) -> String {
    h.wait_css("[data-stored-query]")
        .await
        .attr("data-stored-query")
        .await
        .expect("read the row hook")
        .unwrap_or_default()
}

/// Poll until the first rendered row is no longer `previous`, and return the
/// new key. This is the window-moved condition (never a sleep): a page link is
/// a real navigation, so the swap is asynchronous either side of hydration.
///
/// # Panics
/// When the window has not moved after 15 s.
async fn wait_row_change(h: &Harness, previous: &str) -> String {
    for _ in 0..75 {
        let current = first_row_key(h).await;
        if current != previous {
            return current;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let url = h.driver.current_url().await.expect("current url");
    panic!("the row window never moved off `{previous}` (at {url})");
}

/// Poll until the footer's range line starts with `prefix` (`1–1 of `, …).
///
/// # Panics
/// When it never does — reporting the line it actually showed.
async fn wait_range_prefix(h: &Harness, prefix: &str) {
    let mut seen = String::new();
    for _ in 0..75 {
        seen = h
            .wait_css("[data-page=\"range\"]")
            .await
            .text()
            .await
            .unwrap_or_default();
        if seen.starts_with(prefix) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("the footer range line never started with `{prefix}` (last: `{seen}`)");
}

/// Page the stored-queries table forward and back: the URL carries the page,
/// the rendered window follows it, `?size=` survives the link, and the
/// page-size choice re-pages from the top.
#[tokio::test]
async fn stored_queries_page_through_the_shared_footer() {
    let Some(h) = Harness::start("table-paging").await else {
        return;
    };
    let (Some(cdr), Some(user), Some(pass)) = (
        env("UI_E2E_CDR_URL"),
        env("UI_E2E_BASIC_USER"),
        env("UI_E2E_BASIC_PASS"),
    ) else {
        println!("SKIP table-paging: fixture seeding needs UI_E2E_CDR_URL/UI_E2E_BASIC_*");
        h.finish().await;
        return;
    };
    delete_fixtures(&cdr).await;
    seed_fixtures(&cdr, &user, &pass).await;
    login_basic(&h).await;

    // `?size=1` — one row per page, so two stored queries prove the window
    // moves regardless of what else the stack holds. The size is user input the
    // footer clamps to a sane range; any positive window is honoured, the
    // presets are only links.
    h.goto("/queries?size=1").await;
    let first = first_row_key(&h).await;
    assert_eq!(row_count(&h).await, 1, "?size=1 renders exactly one row");
    wait_range_prefix(&h, "1–1 of ").await;
    // The first page cannot go back: the control keeps its place, inert.
    h.wait_css("button[data-page=\"prev\"]").await;
    h.shot(1, "first-page").await;

    // Forward: the page lands in the URL, the window moves, and the size
    // parameter rides along.
    h.wait_css("a[data-page=\"next\"]")
        .await
        .click()
        .await
        .expect("page forward");
    h.wait_url_contains("page=1").await;
    let second = wait_row_change(&h, &first).await;
    assert_eq!(row_count(&h).await, 1, "the second page renders one row");
    wait_range_prefix(&h, "2–2 of ").await;
    let url = h.driver.current_url().await.expect("current url");
    assert!(
        url.as_str().contains("size=1"),
        "a page link must preserve the screen's other parameters (at {url})"
    );
    h.shot(2, "second-page").await;

    // Back: the first page is the plain path again (the default page is written
    // as its absence) and the original window returns.
    h.wait_css("a[data-page=\"prev\"]")
        .await
        .click()
        .await
        .expect("page back");
    h.wait_url_not_contains("page=1").await;
    let back = wait_row_change(&h, &second).await;
    assert_eq!(back, first, "paging back must restore the first window");
    wait_range_prefix(&h, "1–1 of ").await;
    h.shot(3, "back-on-the-first-page").await;

    // A page-size choice re-pages from the top at that window; the default size
    // clears the parameter entirely, so both fixtures are on one page.
    h.wait_css("a[data-page-size=\"25\"]")
        .await
        .click()
        .await
        .expect("choose 25 rows per page");
    h.wait_url_not_contains("size=1").await;
    h.wait_css("[data-stored-query]").await;
    assert!(
        row_count(&h).await >= 2,
        "the default window must hold both seeded queries"
    );
    h.shot(4, "default-window").await;

    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    delete_fixtures(&cdr).await;
    h.finish().await;
}
