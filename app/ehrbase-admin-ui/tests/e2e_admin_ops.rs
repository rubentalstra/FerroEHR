#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    unreachable_pub,
    dead_code // each test binary uses a subset of the shared harness methods
)]
// e2e journeys are assertive by design; skip-with-reason prints; the shared
// harness module is per-test-binary (the corpus.rs test-file precedent)
//! End-to-end journeys over the console's **admin destructive operations**:
//! template delete (from the list row and from the detail screen), stored-query
//! delete against the CDR store, and physical EHR delete — each a
//! create → delete → assert-gone round trip through the real UI.
//!
//! Every affordance is gated on the CDR advertising `/admin` in its System API
//! conformance manifest, and the deletes themselves need the ADMIN role, so
//! these journeys sign in as the quickstart ADMIN user (`UI_E2E_ADMIN_USER`/
//! `UI_E2E_ADMIN_PASS`, defaulting to `ehrbase-admin`/`ehrbase` from
//! `docker/ehrbase.dev.toml`, which also enables the admin group).
//!
//! Capability is not authorization, and the two halves are covered separately:
//! [`plain_user_is_refused_the_admin_delete`] proves that a session WITHOUT the
//! ADMIN role still sees the affordances (the group IS mounted) and gets an
//! actionable refusal instead of a silent failure; the hidden-when-not-mounted
//! half cannot be shown on this stack (the composed CDR always runs
//! admin-ENABLED) and is unit-tested in `ehrbase_admin_ui::admin` against a
//! manifest without `/admin`.
//!
//! Isolation: each journey creates its OWN object (a template fixture no other
//! journey uses, its own stored-query name, its own EHR), so nothing here
//! deletes data another journey or the documentation-screenshot pass depends
//! on.

mod common;

use std::time::Duration;

use common::{Harness, env, login_basic, login_basic_as};
use thirtyfour::prelude::*;

/// A fixture OPT no other journey touches, and its template id — deleted from
/// the LIST row. The id carries spaces on purpose: it proves the delete path
/// segment is percent-encoded.
const LIST_FIXTURE: &str = "minimal_action2.opt";
/// The template id of [`LIST_FIXTURE`].
const LIST_TEMPLATE_ID: &str = "Minimal action 2";

/// A second unused fixture OPT, deleted from the DETAIL screen.
const DETAIL_FIXTURE: &str = "minimal_instruction.opt";
/// The template id of [`DETAIL_FIXTURE`].
const DETAIL_TEMPLATE_ID: &str = "minimal_instruction.en.v1";

/// The qualified stored-query name this battery saves and then deletes.
const QUERY_NAME: &str = "org.example::e2e-admin-delete";

/// The template `scripts/ui-e2e.sh` seeds (with compositions committed against
/// it), used only by the REFUSED-delete journey — the CDR protects it both by
/// RBAC and by its `409` in-use guard.
const SHARED_TEMPLATE_ID: &str = "minimal_evaluation.en.v1";

/// The admin dev user (quickstart `docker/ehrbase.dev.toml`) — the admin group
/// is RBAC-gated, so the destructive affordances only appear for this session.
fn admin_credentials() -> (String, String) {
    (
        env("UI_E2E_ADMIN_USER").unwrap_or_else(|| "ehrbase-admin".to_owned()),
        env("UI_E2E_ADMIN_PASS").unwrap_or_else(|| "ehrbase".to_owned()),
    )
}

/// The absolute host path of a fixture OPT under `crates/openehr-its`, for the
/// `WebDriver` file-upload `send_keys` mechanism.
///
/// # Panics
/// When the fixture is missing (a repo-layout error, not a skip).
fn fixture_opt_path(name: &str) -> String {
    let raw = format!(
        "{}/../../crates/openehr-its/tests/fixtures/sdk/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::canonicalize(&raw)
        .unwrap_or_else(|e| panic!("fixture OPT {raw} exists: {e}"))
        .to_string_lossy()
        .into_owned()
}

/// Land on `/templates` and guarantee `template_id` is installed, uploading
/// `fixture` through the real upload control when it is absent.
///
/// # Panics
/// On any navigation/interaction failure.
async fn ensure_template(h: &Harness, fixture: &str, template_id: &str) {
    h.goto("/templates").await;
    h.wait_css("input[type=file]").await;
    // Let the list settle on either rows or the empty-state bar first.
    h.wait_css("a[href^='/templates/'], .thaw-message-bar")
        .await;
    let row_delete = format!("[data-template-delete=\"{template_id}\"]");
    if h.driver.find(By::Css(row_delete.clone())).await.is_ok() {
        return;
    }
    h.wait_css("input[type=file]")
        .await
        .send_keys(&fixture_opt_path(fixture))
        .await
        .expect("upload the fixture OPT via the hidden file input");
    h.wait_css(&row_delete).await;
}

/// Poll until no element matches `css` (the assert-gone half of every journey).
///
/// # Panics
/// When the element is still present after 15 s.
async fn wait_css_absent(h: &Harness, css: &str) {
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
    panic!("`{css}` was still present after the delete (at {url})");
}

/// Whether `css` matches a currently VISIBLE element.
///
/// thaw's dialog is never removed from the DOM: `leptos_transition_group`'s
/// `CSSTransition` hides it with `display: none`, so a closed dialog is still
/// findable. Openness is therefore visibility, never mere presence.
async fn is_visible(h: &Harness, css: &str) -> bool {
    match h.driver.find(By::Css(css)).await {
        Ok(element) => element.is_displayed().await.unwrap_or(false),
        Err(_) => false,
    }
}

/// Poll until `css` is no longer visible.
///
/// # Panics
/// When it is still visible after 15 s.
async fn wait_hidden(h: &Harness, css: &str) {
    for _ in 0..75 {
        if !is_visible(h, css).await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let url = h.driver.current_url().await.expect("current url");
    panic!("`{css}` never hid (at {url})");
}

/// Drive one destructive action through its confirmation MODAL: click the
/// trigger, wait for the dialog to become visible, then click its confirm
/// button. Explicit conditions, never a sleep.
///
/// The trigger click carries a bounded retry (the login-submit precedent): a
/// click landing before hydration attaches the listener is simply lost, and
/// re-clicking is safe precisely because the dialog is not open yet.
///
/// # Panics
/// When the dialog never opens.
async fn confirm_in_dialog(h: &Harness, trigger_css: &str, confirm_id: &str) {
    // A visible toast overlays the bottom-right corner and intercepts clicks.
    h.wait_toasts_cleared().await;
    let trigger = h.wait_css(trigger_css).await;
    let confirm_css = format!("#{confirm_id}");
    let mut opened = false;
    for _ in 0..10 {
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
        "`{trigger_css}` never opened the `{confirm_css}` dialog (pre-hydration clicks exhausted)"
    );
    h.wait_css(&confirm_css)
        .await
        .click()
        .await
        .expect("confirm in the dialog");
    // The dialog hides on confirm — that it hid proves the click landed.
    wait_hidden(h, &confirm_css).await;
}

/// Template delete from the list row: upload a fixture OPT, delete it through
/// the confirmation modal, and the row is gone.
#[tokio::test]
async fn admin_deletes_a_template_from_the_list() {
    let Some(h) = Harness::start("admin-template-delete-list").await else {
        return;
    };
    let (user, pass) = admin_credentials();
    login_basic_as(&h, &user, &pass).await;

    ensure_template(&h, LIST_FIXTURE, LIST_TEMPLATE_ID).await;
    h.shot(1, "template-listed").await;

    let selector = format!("[data-template-delete=\"{LIST_TEMPLATE_ID}\"]");
    confirm_in_dialog(&h, &selector, "template-delete-confirm").await;
    wait_css_absent(&h, &selector).await;
    h.shot(2, "template-deleted").await;

    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// Template delete from the detail screen: upload a fixture, open its detail
/// route, delete there, land back on the list without the row.
#[tokio::test]
async fn admin_deletes_a_template_from_the_detail_screen() {
    let Some(h) = Harness::start("admin-template-delete-detail").await else {
        return;
    };
    let (user, pass) = admin_credentials();
    login_basic_as(&h, &user, &pass).await;

    ensure_template(&h, DETAIL_FIXTURE, DETAIL_TEMPLATE_ID).await;
    h.goto(&format!("/templates/{DETAIL_TEMPLATE_ID}")).await;
    h.wait_css("nav[aria-label='Template views']").await;
    h.shot(1, "template-detail").await;

    confirm_in_dialog(&h, "#template-delete", "template-delete-confirm").await;
    // A successful detail delete returns to the list.
    h.wait_url_not_contains(DETAIL_TEMPLATE_ID).await;
    h.wait_css("input[type=file]").await;
    wait_css_absent(
        &h,
        &format!("[data-template-delete=\"{DETAIL_TEMPLATE_ID}\"]"),
    )
    .await;
    h.shot(2, "template-deleted").await;

    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// Stored-query delete: save a query through the raw-AQL editor, then delete
/// that version from the CDR store on `/queries` and the row is gone.
#[tokio::test]
async fn admin_deletes_a_stored_query() {
    let Some(h) = Harness::start("admin-stored-query-delete").await else {
        return;
    };
    let (user, pass) = admin_credentials();
    login_basic_as(&h, &user, &pass).await;

    // Save the query through the real editor (its Save button stays disabled
    // until both fields hold text, which is also the hydration signal — retry
    // the typing until it enables, the login-submit precedent).
    h.goto("/queries/aql").await;
    let mut saved = false;
    for _ in 0..5 {
        h.wait_css("#aql-editor")
            .await
            .send_keys("SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c")
            .await
            .expect("type the AQL");
        h.wait_css("#aql-save-name")
            .await
            .send_keys(QUERY_NAME)
            .await
            .expect("type the stored-query name");
        let save = h.wait_xpath("//button[normalize-space(.)='Save']").await;
        if save.is_enabled().await.unwrap_or(false) {
            save.click().await.expect("save the query");
            saved = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    assert!(saved, "the Save button never enabled (typing never took)");

    // The row appears on /queries with its admin delete affordance (the CDR
    // assigns the version, so match the name prefix).
    let selector = format!("[data-query-delete^=\"{QUERY_NAME}@\"]");
    h.goto("/queries").await;
    h.wait_css(&selector).await;
    h.shot(1, "stored-query-listed").await;

    confirm_in_dialog(&h, &selector, "stored-query-delete-confirm").await;
    wait_css_absent(&h, &selector).await;
    h.shot(2, "stored-query-deleted").await;

    h.assert_console_clean(&["401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// Physical EHR delete: create an EHR through the UI, delete it from its detail
/// screen, land back on `/ehrs`, and the EHR is gone (its detail route now
/// surfaces the CDR's `404` inline).
#[tokio::test]
async fn admin_deletes_an_ehr() {
    let Some(h) = Harness::start("admin-ehr-delete").await else {
        return;
    };
    let (user, pass) = admin_credentials();
    login_basic_as(&h, &user, &pass).await;

    // Create an anonymous EHR; the console navigates to its detail route.
    h.goto("/ehrs").await;
    h.wait_css("#ehr-create-submit")
        .await
        .click()
        .await
        .expect("create an EHR");
    h.wait_url_contains("/ehrs/").await;
    let url = h.driver.current_url().await.expect("current url");
    let ehr_id = url
        .as_str()
        .rsplit('/')
        .next()
        .map(|segment| segment.split('?').next().unwrap_or(segment).to_owned())
        .filter(|id| !id.is_empty())
        .expect("the create navigated to /ehrs/{ehr_id}");
    println!("created EHR {ehr_id}");
    h.shot(1, "ehr-created").await;

    confirm_in_dialog(&h, "#ehr-delete", "ehr-delete-confirm").await;
    // A successful delete returns to the EHR list.
    h.wait_url_not_contains(&ehr_id).await;
    h.wait_css("#ehr-lookup").await;
    h.shot(2, "ehr-deleted").await;

    // Assert-gone: the detail route now renders the CDR's 404 inline.
    h.goto(&format!("/ehrs/{ehr_id}")).await;
    h.wait_css("[role='alert']").await;
    h.shot(3, "ehr-gone").await;

    // Deliberate negative step: the CDR 404 for the deleted EHR is expected.
    h.assert_console_clean(&["404", "401", "Failed to load resource"])
        .await;
    h.finish().await;
}

/// Capability vs authorization: a session without the ADMIN role still SEES the
/// affordances (the CDR advertises `/admin`, so the group exists), and the
/// refusal arrives as actionable copy on confirm — never a silent no-op, and
/// never a deleted object.
#[tokio::test]
async fn plain_user_is_refused_the_admin_delete() {
    let Some(h) = Harness::start("admin-ops-refused").await else {
        return;
    };
    login_basic(&h).await;

    // The affordance is present for this session: the group is mounted.
    h.goto("/templates").await;
    h.wait_css("[data-template-delete]").await;
    h.shot(1, "templates-delete-visible").await;

    // Confirm a delete of the shared fixture template. The CDR refuses it
    // twice over — RBAC (no ADMIN role) and, even for an admin, the committed
    // compositions that reference it — so this step cannot destroy the fixture.
    let selector = format!("[data-template-delete=\"{SHARED_TEMPLATE_ID}\"]");
    confirm_in_dialog(&h, &selector, "template-delete-confirm").await;

    // The failure is reported with actionable copy, and the row survives.
    h.wait_xpath(
        "//*[contains(@class, 'thaw-toast-body') and (contains(., 'ADMIN') or contains(., 'may not delete'))]",
    )
    .await;
    h.shot(2, "delete-refused").await;
    assert!(
        h.driver.find(By::Css(&selector)).await.is_ok(),
        "the refused delete must leave `{SHARED_TEMPLATE_ID}` in the list"
    );

    // Deliberate negative step: the CDR's refusal reaches the browser as a
    // failed server-fn call, which the console gate must allow.
    h.assert_console_clean(&[
        "403",
        "Forbidden",
        "server function",
        "500",
        "Failed to load resource",
    ])
    .await;
    h.finish().await;
}
