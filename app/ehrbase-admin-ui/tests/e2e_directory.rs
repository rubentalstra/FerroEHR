#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    unreachable_pub,
    dead_code // each test binary uses a subset of the shared harness methods
)]
// e2e journeys are assertive by design; skip-with-reason prints; the shared
// harness module is per-test-binary (the corpus.rs test-file precedent)
//! End-to-end DIRECTORY journeys — the structured tree editor (add a
//! subfolder + save with `If-Match`), version history with restore, and the
//! two-step directory delete with the create state coming back. Each journey
//! creates its OWN anonymous EHR through the console (never the seeded one —
//! these journeys mutate/delete the directory) and fails on any browser
//! console error (the standing hydration gate).

mod common;

use common::{Harness, env, login_basic};

/// Create an anonymous EHR through the console and land on its detail page,
/// returning the new EHR id (parsed from the navigated URL).
async fn create_ehr(h: &Harness) -> String {
    h.goto("/ehrs").await;
    h.wait_css("#ehr-create-submit")
        .await
        .click()
        .await
        .expect("create an anonymous EHR");
    // The success Effect navigates to /ehrs/{uuid}; wait for a detail-only
    // element, then read the id from the URL.
    h.wait_css("#ehr-detail, [id^='tab-'], main").await;
    let mut url = String::new();
    for _ in 0..50u8 {
        url = h
            .driver
            .current_url()
            .await
            .expect("current url")
            .to_string();
        if let Some(tail) = url.split("/ehrs/").nth(1)
            && tail.len() >= 36
        {
            return tail.chars().take(36).collect::<String>();
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("EHR creation never navigated to a detail route (last url {url})");
}

/// Create the directory from the LAST folder-template option (the richest
/// built-in tree) and wait for the edit view.
async fn create_directory_from_template(h: &Harness, ehr_id: &str) {
    h.goto(&format!("/ehrs/{ehr_id}?tab=directory")).await;
    h.wait_css("#folder-template option:last-child")
        .await
        .click()
        .await
        .expect("pick a folder template");
    h.wait_css("#directory-create")
        .await
        .click()
        .await
        .expect("create the directory");
    h.wait_css("#directory-edit").await;
}

/// Journey: structured tree editing — add a subfolder at the root, save with
/// the dirty-bar `If-Match` PUT, and see the change survive the refetch.
#[tokio::test]
async fn directory_tree_edit_journey() {
    let Some(h) = Harness::start("directory-edit").await else {
        return;
    };
    login_basic(&h).await;
    let ehr_id = create_ehr(&h).await;
    create_directory_from_template(&h, &ehr_id).await;

    // Add a subfolder at the root ("New folder"), which marks the tree dirty.
    h.wait_css("[aria-label='Add subfolder']")
        .await
        .click()
        .await
        .expect("add a subfolder at the root");
    h.wait_xpath("//*[@id='directory-tree']//*[contains(normalize-space(.), 'New folder')]")
        .await;

    // Save → PUT with If-Match; the resource refetches and re-seeds.
    h.wait_css("#directory-save")
        .await
        .click()
        .await
        .expect("save the edited tree");
    // The save bar disappears once the refetched tree is clean again, and the
    // committed subfolder is still rendered.
    h.wait_xpath("//*[@id='directory-tree']//*[contains(normalize-space(.), 'New folder')]")
        .await;
    h.assert_console_clean(&[]).await;
    h.finish().await;
}

/// Journey: version history — after an edit there are two versions; v1 is
/// previewable read-only and restorable (a PUT of v1's tree with the current
/// latest `If-Match`).
#[tokio::test]
async fn directory_history_and_restore_journey() {
    let Some(h) = Harness::start("directory-history").await else {
        return;
    };
    login_basic(&h).await;
    let ehr_id = create_ehr(&h).await;
    create_directory_from_template(&h, &ehr_id).await;

    // v2: add a subfolder and save.
    h.wait_css("[aria-label='Add subfolder']")
        .await
        .click()
        .await
        .expect("add a subfolder");
    h.wait_css("#directory-save")
        .await
        .click()
        .await
        .expect("save v2");

    // The save toast overlays the bottom-right corner and intercepts clicks
    // on anything underneath — wait for it to clear (explicit condition).
    h.wait_toasts_cleared().await;

    // Open the history panel: two versions listed, newest first.
    h.wait_xpath("//button[contains(normalize-space(.), 'Version history')]")
        .await
        .click()
        .await
        .expect("open version history");
    h.wait_xpath("//button[contains(normalize-space(.), 'v2')]")
        .await;
    let v1_row = h
        .wait_xpath("//button[contains(normalize-space(.), 'v1')]")
        .await;
    v1_row.click().await.expect("select v1");

    // v1 preview offers a restore (v1 is not the latest); restoring commits
    // v3 = v1's tree. Any lingering toast would intercept the click.
    h.wait_toasts_cleared().await;
    h.wait_xpath("//button[contains(normalize-space(.), 'Restore this version')]")
        .await
        .click()
        .await
        .expect("restore v1");
    h.wait_xpath("//button[contains(normalize-space(.), 'v3')]")
        .await;
    h.assert_console_clean(&[]).await;
    h.finish().await;
}

/// Journey: the two-step delete — confirm, the deleted/empty state renders
/// the create flow again, and (per RM master06 §Logical Deletion + our
/// live-slot rule) creating a NEW directory afterwards succeeds.
#[tokio::test]
async fn directory_delete_and_recreate_journey() {
    let Some(h) = Harness::start("directory-delete").await else {
        return;
    };
    login_basic(&h).await;
    let ehr_id = create_ehr(&h).await;
    create_directory_from_template(&h, &ehr_id).await;

    // Two-step delete: the danger button opens the inline confirmation.
    h.wait_xpath("//button[contains(normalize-space(.), 'Delete directory')]")
        .await
        .click()
        .await
        .expect("open the delete confirmation");
    h.wait_css("#directory-delete-confirm")
        .await
        .click()
        .await
        .expect("confirm the delete");

    // The tab falls back to the create state (no live directory)…
    h.wait_css("#folder-template").await;

    // …and a NEW directory can be created (the deleted container remains,
    // the slot is vacant — proven at the wire by directory_http). The delete
    // toast must clear first (it intercepts clicks underneath).
    h.wait_toasts_cleared().await;
    h.wait_css("#directory-create")
        .await
        .click()
        .await
        .expect("re-create after delete");
    h.wait_css("#directory-edit").await;
    h.assert_console_clean(&[]).await;
    h.finish().await;
}

/// The env accessor is used by the harness in the other journeys; referenced
/// here so the shared module's API stays exercised uniformly.
#[test]
fn harness_env_accessor_shape() {
    // A plain unit assertion (no browser): absent variables are None.
    assert!(env("UI_E2E_THIS_VARIABLE_DOES_NOT_EXIST").is_none());
}
