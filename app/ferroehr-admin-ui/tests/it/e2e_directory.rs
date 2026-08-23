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
// e2e journeys are assertive by design; skip-with-reason prints; the shared
// harness module is per-test-binary (the corpus.rs test-file precedent)
//! End-to-end DIRECTORY journeys — the create-empty flow, the structured tree
//! editor (add a subfolder + save with `If-Match`), the item picker's
//! add/remove round trip, version history with restore, and the two-step
//! directory delete with the create state coming back. Each journey creates its
//! OWN anonymous EHR through the console (never the seeded one — these journeys
//! mutate/delete the directory) and fails on any browser console error (the
//! standing hydration gate).
//!
//! The console keeps no folder-shape library of its own, so the create-empty
//! root plus the tree editor is the ONLY way directory structure is built —
//! which is what every journey here drives.
//!
//! The item-picker journey seeds its two compositions over ITS-REST
//! (`UI_E2E_CDR_URL`) rather than through the UI: the commit form has its own
//! journey, and what this one is about is which REFERENCE survives a sibling
//! removal.

use crate::common;

use common::{Harness, env, login_basic, wait_css_absent, wait_text};
use thirtyfour::prelude::*;

/// Create an anonymous EHR through the console and land on its detail page,
/// returning the new EHR id (parsed from the navigated URL).
async fn create_ehr(h: &Harness) -> String {
    h.goto("/ehrs").await;
    // The create dispatch + navigation are hydrated behaviour; a click landing
    // before hydration is silently lost (#2285's class).
    h.wait_hydrated().await;
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

/// Create the EHR's directory through the create-empty flow — the ONLY create
/// path now that the console keeps no folder-shape library of its own — and
/// wait for the edit view. Structure is built afterwards in the tree editor,
/// which is exactly what each journey below exercises.
async fn create_empty_directory(h: &Harness, ehr_id: &str) {
    h.goto(&format!("/ehrs/{ehr_id}?tab=directory")).await;
    // A full navigation restarts hydration; the create dispatch is hydrated
    // behaviour, and a click landing earlier is silently lost (#2285's class).
    h.wait_hydrated().await;
    h.wait_css("#directory-create")
        .await
        .click()
        .await
        .expect("create the empty directory");
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
    create_empty_directory(&h, &ehr_id).await;

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

/// The template the seeded picker compositions are committed against — the one
/// `scripts/ui-e2e.sh` uploads while bringing the stack up.
const PICKER_TEMPLATE: &str = "minimal_evaluation.en.v1";

/// Commit one FLAT composition into `ehr_id` and return its VERSIONED-OBJECT
/// id — the `HIER_OBJECT_ID` the directory's `OBJECT_REF` items carry, and
/// therefore the identity a picker row and an item row are matched on.
///
/// # Panics
/// When the CDR refuses the commit or answers without a `uid.value`.
async fn seed_picker_composition(
    cdr: &str,
    user: &str,
    pass: &str,
    ehr_id: &str,
    composer: &str,
    time: &str,
) -> String {
    let body: serde_json::Value = reqwest::Client::new()
        .post(format!(
            "{cdr}/ferroehr/rest/openehr/v1/ehr/{ehr_id}/composition"
        ))
        .basic_auth(user, Some(pass))
        .header("Content-Type", "application/openehr.wt.flat+json")
        .header("Accept", "application/json")
        .header("openehr-template-id", PICKER_TEMPLATE)
        .header("Prefer", "return=representation")
        .json(&serde_json::json!({
            "ctx/language": "en",
            "ctx/territory": "US",
            "ctx/composer_name": composer,
            "ctx/time": time,
            "minimal/minimal/quantity|magnitude": 37.2,
            "minimal/minimal/quantity|unit": "kg",
        }))
        .send()
        .await
        .expect("commit a picker fixture composition")
        .error_for_status()
        .expect("the CDR accepted the picker fixture composition")
        .json()
        .await
        .expect("the committed composition");
    let version_uid = body
        .pointer("/uid/value")
        .and_then(serde_json::Value::as_str)
        .expect("the committed composition's uid");
    version_uid
        .split("::")
        .next()
        .expect("a version uid has a container half")
        .to_owned()
}

/// The item ids the CDR's own directory read reports, in document order.
///
/// The browser half of an assertion can only prove what the SCREEN shows; this
/// is the other half — what the repository actually holds after the save.
///
/// # Panics
/// When the CDR refuses the read or answers a body that is not a FOLDER.
async fn stored_item_ids(cdr: &str, user: &str, pass: &str, ehr_id: &str) -> Vec<String> {
    let folder: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "{cdr}/ferroehr/rest/openehr/v1/ehr/{ehr_id}/directory"
        ))
        .basic_auth(user, Some(pass))
        .header("Accept", "application/json")
        .send()
        .await
        .expect("read the directory back")
        .error_for_status()
        .expect("the CDR served the directory")
        .json()
        .await
        .expect("the directory FOLDER");
    folder
        .get("items")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.pointer("/id/value")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Add the composition whose version uid contains `object_id` to the root
/// folder through the item picker.
///
/// The picker overlay stays mounted and hidden (`class:hidden`), so its rows
/// are always PRESENT — the wait therefore has to be on clickability, which is
/// the condition that covers both the overlay opening and the composition list
/// resolving.
///
/// # Panics
/// When the picker never offers that composition.
async fn add_item_through_picker(h: &Harness, object_id: &str) {
    h.wait_css("[aria-label='Add item reference']")
        .await
        .click()
        .await
        .expect("open the item picker");
    h.wait_clickable_xpath(&format!("//button[contains(., '{object_id}')]"))
        .await
        .click()
        .await
        .expect("pick the composition");
    // The picker closes on pick, and the row it added carries the reference id.
    h.wait_css(&format!("[data-item-id='{object_id}']")).await;
}

/// Commit the pending tree edit and wait for the CDR's own confirmation.
///
/// The success toast is the "the PUT finished" condition: a REST re-read fired
/// straight after the click races the in-flight write and reads the directory
/// as the CDR held it BEFORE the save (measured — it read zero items while the
/// screen already showed two). Waiting for no toast is not that condition: it
/// is satisfied in the instant before the toast even appears. The toasts are
/// then cleared, because a visible one intercepts the next click.
///
/// # Panics
/// When the save never confirms.
async fn save_directory(h: &Harness, what: &'static str) {
    // The create's own toast overlays the corner; let it go before clicking, so
    // the "Directory updated" wait below cannot see a stale card either.
    h.wait_toasts_cleared().await;
    h.wait_css("#directory-save")
        .await
        .click()
        .await
        .expect(what);
    assert!(
        wait_text(h, "Directory updated").await,
        "the directory save never confirmed ({what})"
    );
    h.wait_toasts_cleared().await;
}

/// Journey: the item picker adds two references and removes ONE — and the
/// reference that survives is the specific OTHER one, on screen and in the CDR.
///
/// Removing a sibling is exactly where a position-keyed row hands its
/// neighbour's identity to the delete (rules §4), so counting rows would pass
/// against that bug. Every assertion here names an id.
#[tokio::test]
async fn directory_item_picker_add_remove_journey() {
    let Some(h) = Harness::start("directory-item-picker").await else {
        return;
    };
    let (Some(cdr), Some(user), Some(pass)) = (
        env("UI_E2E_CDR_URL"),
        env("UI_E2E_BASIC_USER"),
        env("UI_E2E_BASIC_PASS"),
    ) else {
        println!("SKIP directory-item-picker: seeding needs UI_E2E_CDR_URL/UI_E2E_BASIC_*");
        h.finish().await;
        return;
    };
    login_basic(&h).await;
    let ehr_id = create_ehr(&h).await;

    // Two compositions to reference, distinguishable by composer and time so a
    // failure report says WHICH one the picker offered.
    let kept = seed_picker_composition(
        &cdr,
        &user,
        &pass,
        &ehr_id,
        "Picker keeper",
        "2026-07-20T09:00:00Z",
    )
    .await;
    let removed = seed_picker_composition(
        &cdr,
        &user,
        &pass,
        &ehr_id,
        "Picker removed",
        "2026-07-21T09:00:00Z",
    )
    .await;
    assert_ne!(kept, removed, "the two fixtures must be distinct objects");

    create_empty_directory(&h, &ehr_id).await;

    // Add BOTH references through the picker, then commit them.
    add_item_through_picker(&h, &removed).await;
    add_item_through_picker(&h, &kept).await;
    save_directory(&h, "commit both references").await;
    // Both survive the refetch — the tree the CDR served back holds them.
    h.wait_css(&format!("[data-item-id='{removed}']")).await;
    h.wait_css(&format!("[data-item-id='{kept}']")).await;
    let stored = stored_item_ids(&cdr, &user, &pass, &ehr_id).await;
    assert_eq!(
        stored.len(),
        2,
        "the CDR holds both references (got {stored:?})"
    );
    h.shot(1, "two-references").await;

    // Remove exactly ONE of them and commit again.
    h.wait_css(&format!(
        "[data-item-id='{removed}'] button[aria-label='Remove item']"
    ))
    .await
    .click()
    .await
    .expect("remove one reference");
    save_directory(&h, "commit the removal").await;

    // The survivor is the OTHER one, by identity — never by count.
    wait_css_absent(&h, &format!("[data-item-id='{removed}']")).await;
    h.wait_css(&format!("[data-item-id='{kept}']")).await;
    let on_screen = h
        .driver
        .find_all(By::Css("[data-item-id]"))
        .await
        .expect("the remaining item rows");
    assert_eq!(
        on_screen.len(),
        1,
        "exactly one reference is left on screen after removing a sibling"
    );
    // …and the repository agrees, which is the half the browser cannot prove.
    assert_eq!(
        stored_item_ids(&cdr, &user, &pass, &ehr_id).await,
        vec![kept.clone()],
        "the committed directory keeps exactly the reference that was not removed"
    );
    h.shot(2, "one-reference-left").await;

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
    create_empty_directory(&h, &ehr_id).await;

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
    create_empty_directory(&h, &ehr_id).await;

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
    h.wait_css("#directory-create").await;

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
