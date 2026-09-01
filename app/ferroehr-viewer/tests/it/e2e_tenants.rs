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
    reason = "test fixtures and wire assertions are raw JSON by the testing rule \
              (.claude/rules/testing.md §Test-fixture construction)"
)]
//! End-to-end journeys over the console's **tenant registry** (`/tenants`):
//! the read-only tenant-context card, the registry CRUD round trip, and the
//! duplicate-name conflict surfacing the CDR's diagnostic verbatim.
//!
//! The screen is probe-gated on the CDR serving its tenancy extension, which
//! the composed E2E stack enables (`docker/viewer/e2e-env.yml` sets
//! `FERROEHR__TENANCY__ENABLED`); a stack without it hides the nav entry
//! entirely, and these journeys say so in their failure message rather than
//! passing vacuously. The hidden-when-absent half is unit-tested in
//! `ferroehr_viewer::tenants` against a probe that found no surface — the
//! same split as the admin-group and management journeys.
//!
//! The registry is Admin-classed by the CDR's coarse RBAC (it is mounted under
//! `/admin`), so every scene that reads or writes it signs in as the ADMIN dev
//! user; the last scene signs in as the ordinary one to pin the refusal copy.
//! Neither credential carries a tenant claim, which is why the context card's
//! expected answer is the reserved default tenant — the whole point of the
//! card: tenancy is credential-derived and the console never selects it.
//!
//! Isolation: every fixture tenant this file registers is removed over the
//! registry API before AND after the scene that owns it, so a shared stack is
//! left exactly as it was found.

use crate::common;

use reqwest::StatusCode;

use common::{
    Harness, confirm_in_dialog, env, login_basic, login_basic_as, retype, wait_css_absent,
    wait_enabled, wait_text, wait_text_contains,
};
use thirtyfour::prelude::*;

/// The tenant the CRUD round trip owns.
const ROUND_TRIP_TENANT: &str = "e2e-console-tenant";

/// The tenant the conflict scene seeds, then tries to register a second time.
const CONFLICT_TENANT: &str = "e2e-console-duplicate";

/// Every fixture tenant this file may leave behind.
const FIXTURE_TENANTS: [&str; 2] = [ROUND_TRIP_TENANT, CONFLICT_TENANT];

/// The `system_id` a fixture tenant is registered with.
const FIXTURE_SYSTEM_ID: &str = "e2e.example.org";

/// The `system_id` the edit scene stores over it.
const EDITED_SYSTEM_ID: &str = "e2e-edited.example.org";

/// The admin dev user (quickstart `docker/ferroehr.dev.toml`): the registry
/// sits under `/admin`, so the RBAC gate classes every fixture call as admin
/// work.
fn admin_credentials() -> (String, String) {
    (
        env("UI_E2E_ADMIN_USER").unwrap_or_else(|| "ferroehr-admin".to_owned()),
        env("UI_E2E_ADMIN_PASS").unwrap_or_else(|| "ferroehr".to_owned()),
    )
}

/// The registry base URL on the CDR under test.
fn registry_url(cdr: &str) -> String {
    format!("{cdr}/ferroehr/rest/openehr/v1/admin/tenant")
}

/// Remove every fixture tenant from the registry (absent = nothing to do), so
/// a scene both starts and ends from a known state.
///
/// # Panics
/// On any answer other than `200` to the listing, or `204`/`404` to a delete.
async fn remove_fixture_tenants(cdr: &str) {
    let http = reqwest::Client::new();
    let (user, pass) = admin_credentials();
    let response = http
        .get(registry_url(cdr))
        .basic_auth(&user, Some(&pass))
        .send()
        .await
        .expect("list the tenant registry");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "the tenant registry must be served for these journeys to mean anything"
    );
    let rows = response
        .json::<serde_json::Value>()
        .await
        .expect("the registry answers a JSON array");
    let ids: Vec<String> = rows
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row.get("name")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|name| FIXTURE_TENANTS.contains(&name))
                })
                .filter_map(|row| row.get("id").and_then(serde_json::Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    for id in ids {
        let status = http
            .delete(format!("{}/{id}", registry_url(cdr)))
            .basic_auth(&user, Some(&pass))
            .send()
            .await
            .expect("delete a fixture tenant")
            .status();
        assert!(
            status == StatusCode::NO_CONTENT || status == StatusCode::NOT_FOUND,
            "tenant cleanup -> {status}"
        );
    }
}

/// Register one fixture tenant over the registry API (test setup deliberately
/// bypasses the UI, whose create path has its own scene).
///
/// # Panics
/// When the CDR refuses the registration.
async fn seed_tenant(cdr: &str, name: &str) {
    let (user, pass) = admin_credentials();
    let status = reqwest::Client::new()
        .post(registry_url(cdr))
        .basic_auth(&user, Some(&pass))
        .json(&serde_json::json!({ "name": name, "system_id": FIXTURE_SYSTEM_ID }))
        .send()
        .await
        .expect("seed a fixture tenant")
        .status();
    assert_eq!(status, StatusCode::CREATED, "tenant seed -> {status}");
}

/// Land on `/tenants` with the registry actually served, or fail naming the
/// switch that turns it on.
///
/// # Panics
/// When the CDR under test runs with the tenancy extension disabled — the
/// screen then renders its disabled card and every assertion below would be
/// vacuous.
async fn open_registry(h: &Harness) {
    // The nav entry is the probe's own verdict: it renders only when the CDR
    // answered the registry probe with something other than a 404.
    h.wait_css("a[href='/tenants']").await;
    h.goto("/tenants").await;
    h.wait_css("#tenants-screen").await;
    assert!(
        h.driver.find(By::Css("#tenants-disabled")).await.is_err(),
        "the CDR under test runs with the tenancy extension disabled — set \
         FERROEHR__TENANCY__ENABLED=true on the composed `ferroehr` service"
    );
}

/// Fill the create card and send it. The submit is inert until both fields
/// hold something, so `wait_enabled` IS the "the form is complete" condition —
/// never a sleep.
///
/// # Panics
/// On any interaction failure.
async fn register_tenant(h: &Harness, name: &str, system_id: &str) {
    retype(h, "#tenant-create-name", name).await;
    retype(h, "#tenant-create-system-id", system_id).await;
    wait_enabled(h, "#tenant-create-submit").await;
    h.wait_css("#tenant-create-submit")
        .await
        .click()
        .await
        .expect("register the tenant");
}

/// The CSS selector of one registry row's cell.
fn row_cell(name: &str, cell: &str) -> String {
    format!("tr[data-tenant='{name}'] [data-tenant-cell='{cell}']")
}

/// The context card renders the tenant this session's credential resolves to,
/// and the screen offers no way to change it.
#[tokio::test]
async fn the_context_card_reports_the_credentials_own_tenant() {
    let Some(h) = Harness::start("tenant-context").await else {
        return;
    };
    let (user, pass) = admin_credentials();
    login_basic_as(&h, &user, &pass).await;
    open_registry(&h).await;

    // The session's Basic credential carries no tenant claim, so
    // the CDR resolves the request to the reserved default tenant and the card
    // says exactly that.
    wait_text_contains(&h, "#tenant-context-value", "the reserved default tenant").await;
    h.shot(1, "tenant-context").await;

    // The read-only stance is structural, not copy: no control on the screen
    // selects a tenant.
    let selectors = h
        .driver
        .find_all(By::Css("#tenant-context select, #tenant-context input"))
        .await
        .unwrap_or_default();
    assert!(
        selectors.is_empty(),
        "the tenant context is display-only — a selector would be a tenant switcher"
    );

    h.assert_console_clean(&["Failed to load resource"]).await;
    h.finish().await;
}

/// The registry round trip: register a tenant, see its row, edit its
/// `system_id`, then delete it through the confirmation dialog and watch the
/// row go.
#[tokio::test]
async fn the_registry_creates_edits_and_deletes_a_tenant() {
    let Some(h) = Harness::start("tenant-registry").await else {
        return;
    };
    let Some(cdr) = env("UI_E2E_CDR_URL") else {
        println!("SKIP tenant-registry: fixture cleanup needs UI_E2E_CDR_URL");
        h.finish().await;
        return;
    };
    remove_fixture_tenants(&cdr).await;
    let (user, pass) = admin_credentials();
    login_basic_as(&h, &user, &pass).await;
    open_registry(&h).await;

    register_tenant(&h, ROUND_TRIP_TENANT, FIXTURE_SYSTEM_ID).await;
    // The row is the CDR's answer, not the form's: the listing refetches on the
    // action's version and renders what the registry now holds.
    wait_text_contains(
        &h,
        &row_cell(ROUND_TRIP_TENANT, "system-id"),
        FIXTURE_SYSTEM_ID,
    )
    .await;
    assert!(
        wait_text(&h, "Tenant registered").await,
        "a successful registration must toast"
    );
    h.shot(1, "tenant-registered").await;

    // Edit: the row's own values seed the editor, the save stores the new
    // system_id, and the row follows.
    h.wait_toasts_cleared().await;
    h.wait_css(&format!("[data-tenant-edit='{ROUND_TRIP_TENANT}']"))
        .await
        .click()
        .await
        .expect("open the tenant editor");
    h.wait_css("#tenant-edit").await;
    retype(&h, "#tenant-edit-system-id", EDITED_SYSTEM_ID).await;
    wait_enabled(&h, "#tenant-edit-save").await;
    h.wait_css("#tenant-edit-save")
        .await
        .click()
        .await
        .expect("save the tenant");
    wait_text_contains(
        &h,
        &row_cell(ROUND_TRIP_TENANT, "system-id"),
        EDITED_SYSTEM_ID,
    )
    .await;
    // The editor closes on success, so the screen is back to its resting state.
    wait_css_absent(&h, "#tenant-edit").await;
    h.shot(2, "tenant-edited").await;

    // Delete: two steps — the row's button opens the shared confirmation
    // dialog, and only the dialog's own button dispatches.
    confirm_in_dialog(
        &h,
        &format!("[data-tenant-delete='{ROUND_TRIP_TENANT}']"),
        "tenant-delete-confirm",
    )
    .await;
    wait_css_absent(&h, &format!("tr[data-tenant='{ROUND_TRIP_TENANT}']")).await;
    assert!(
        wait_text(&h, "Tenant deleted").await,
        "a successful delete must toast"
    );
    h.shot(3, "tenant-deleted").await;

    h.assert_console_clean(&["Failed to load resource"]).await;
    remove_fixture_tenants(&cdr).await;
    h.finish().await;
}

/// A duplicate name is refused by the CDR, and its `409` diagnostic reaches
/// the reader VERBATIM — inline beside the failure toast.
#[tokio::test]
async fn a_duplicate_tenant_name_surfaces_the_conflict_verbatim() {
    let Some(h) = Harness::start("tenant-conflict").await else {
        return;
    };
    let Some(cdr) = env("UI_E2E_CDR_URL") else {
        println!("SKIP tenant-conflict: fixture seeding needs UI_E2E_CDR_URL");
        h.finish().await;
        return;
    };
    remove_fixture_tenants(&cdr).await;
    seed_tenant(&cdr, CONFLICT_TENANT).await;
    let (user, pass) = admin_credentials();
    login_basic_as(&h, &user, &pass).await;
    open_registry(&h).await;

    register_tenant(&h, CONFLICT_TENANT, "second.example.org").await;

    // The CDR's own words, unedited — the console never paraphrases a
    // diagnostic it did not author.
    wait_text_contains(
        &h,
        ".thaw-message-bar",
        "a tenant with that name already exists",
    )
    .await;
    // The refusal ALSO toasts: an inline-only failure reads as "nothing
    // happened" (the console's mutation-feedback rule).
    assert!(
        wait_text(&h, "Registration failed").await,
        "a refused registration must toast as well as render the diagnostic inline"
    );
    h.shot(1, "tenant-conflict").await;

    // Nothing was stored twice: the seeded row still carries its original
    // system_id.
    wait_text_contains(
        &h,
        &row_cell(CONFLICT_TENANT, "system-id"),
        FIXTURE_SYSTEM_ID,
    )
    .await;

    // The 409 the CDR answered the server fn with is the point of this journey.
    h.assert_console_clean(&["409", "Failed to load resource"])
        .await;
    remove_fixture_tenants(&cdr).await;
    h.finish().await;
}

/// The reserved default tenant cannot be deleted, and the CDR's refusal reaches
/// the reader VERBATIM — inline under the table as well as in the failure
/// toast. Nothing is destroyed by this scene: the refusal IS the assertion.
#[tokio::test]
async fn deleting_the_reserved_default_tenant_is_refused_verbatim() {
    let Some(h) = Harness::start("tenant-default-guard").await else {
        return;
    };
    let (user, pass) = admin_credentials();
    login_basic_as(&h, &user, &pass).await;
    open_registry(&h).await;

    confirm_in_dialog(
        &h,
        "[data-tenant-delete='default']",
        "tenant-delete-confirm",
    )
    .await;

    wait_text_contains(
        &h,
        ".thaw-message-bar",
        "the reserved default tenant cannot be deleted",
    )
    .await;
    assert!(
        wait_text(&h, "Delete failed").await,
        "a refused delete must toast as well as render the diagnostic inline"
    );
    h.shot(1, "tenant-default-refused").await;

    // The row is still there — a refused delete removes nothing.
    h.wait_css("tr[data-tenant='default']").await;

    // The 409 the CDR answered the server fn with is the point of this journey.
    h.assert_console_clean(&["409", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// A session without the ADMIN role still SEES the registry entry — capability
/// is not authorization — and the refusal reaches it as actionable copy on the
/// screen that asked, never as a missing screen or a bare "forbidden".
#[tokio::test]
async fn a_session_without_the_admin_role_reads_the_refusal_on_the_screen() {
    let Some(h) = Harness::start("tenant-refused").await else {
        return;
    };
    // The ordinary dev user (USER role): the CDR mounts the group, so the probe
    // says present and the nav entry renders, but every call is answered 403.
    login_basic(&h).await;
    open_registry(&h).await;

    wait_text_contains(&h, "#tenant-refused", "may not administer").await;
    wait_text_contains(&h, "#tenant-refused", "ADMIN-role").await;
    // The CDR's own diagnostic travels with it, unedited.
    wait_text_contains(&h, "#tenant-refused", "ADMIN").await;
    h.shot(1, "tenant-refused").await;

    // A refused READ never toasts (the console's one feedback rule).
    assert!(
        h.driver
            .find_all(By::Css(".thaw-toast-body"))
            .await
            .unwrap_or_default()
            .is_empty(),
        "a refused read reports inline only — a toast would be the mutation rule leaking"
    );

    // The 403 the CDR answered the server fn with is the point of this journey.
    h.assert_console_clean(&["403", "Failed to load resource"])
        .await;
    h.finish().await;
}
