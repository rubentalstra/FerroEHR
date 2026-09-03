// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

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
//! End-to-end journeys over the END of a viewer session.
//!
//! The session is ended the way a revocation ends it — the sealed cookie is
//! deleted out from under the browser — because that is observable in seconds,
//! while waiting out the configured idle window is not. What the two journeys
//! pin is the behaviour on either side of the trigger:
//!
//! - with interaction, the first server function that answers "no session"
//!   drives the whole UI to the signed-out state (the transport backstop);
//! - with NO interaction at all, the same state is reached on the viewer's own
//!   schedule — which is what proves there is no interactive window where the
//!   authenticated chrome accepts input against a dead session.
//!
//! Both end on `/login?expired=1`, with the session-ended notice rendered and
//! no authenticated chrome left on the page.

use crate::common;

use std::time::Duration;

use common::{Harness, login_basic, wait_css_absent};

/// The sealed session cookie's name (`ferroehr_viewer::session::SESSION_COOKIE`,
/// which is server-only and so cannot be imported here).
const SESSION_COOKIE: &str = "ferroehr_viewer_session";

/// How long a journey waits for a transition the browser makes on its own
/// schedule: the shell re-checks the session on the health poll's tick, so the
/// budget is that interval plus room for the round trip.
const UNATTENDED_WAIT: Duration = Duration::from_mins(1);

/// The authenticated chrome, by the selectors the other journeys drive it with.
const AUTHED_CHROME: [&str; 2] = ["#user-menu-trigger", "footer"];

/// Deliberate console noise: every server function called with the cookie gone
/// answers an error status, which the browser logs as a failed load. Nothing
/// else is allowed through the gate.
const EXPECTED_CONSOLE: [&str; 1] = ["Failed to load resource"];

/// Delete the sealed session cookie: a revocation the browser cannot predict.
async fn revoke_session(h: &Harness) {
    h.driver
        .delete_cookie(SESSION_COOKIE)
        .await
        .expect("delete the session cookie");
}

/// Assert the signed-out landing: the expiry notice is on screen and none of
/// the authenticated chrome survived the transition.
async fn assert_signed_out(h: &Harness) {
    h.wait_css("#session-expired").await;
    for chrome in AUTHED_CHROME {
        wait_css_absent(h, chrome).await;
    }
}

/// A session revoked mid-visit: the next interaction with the CDR lands the
/// whole UI on the signed-out state, with no reload and nothing left to click.
#[tokio::test]
async fn a_revoked_session_signs_the_ui_out_on_the_next_interaction() {
    let Some(h) = Harness::start("session-expiry-interactive").await else {
        return;
    };
    login_basic(&h).await;
    h.goto("/templates").await;
    h.shot(1, "signed-in").await;

    revoke_session(&h).await;
    // A sidebar navigation — the ordinary click a user makes without knowing
    // anything has changed. The screen it opens reads the CDR, that read is
    // refused, and the refusal is what the transition rides.
    h.wait_css("a[href='/ehrs']")
        .await
        .click()
        .await
        .expect("navigate to EHRs");

    h.wait_url_contains("expired=1").await;
    assert_signed_out(&h).await;
    h.shot(2, "signed-out").await;
    h.assert_console_clean(&EXPECTED_CONSOLE).await;
    h.finish().await;
}

/// A session revoked while the user does NOTHING: the viewer notices on its own
/// and signs out, so no interactive window over a dead session ever exists.
#[tokio::test]
async fn a_revoked_session_signs_the_ui_out_with_no_interaction_at_all() {
    let Some(h) = Harness::start("session-expiry-unattended").await else {
        return;
    };
    login_basic(&h).await;
    h.goto("/").await;
    h.shot(1, "signed-in").await;

    revoke_session(&h).await;
    // Nothing is clicked, typed or navigated from here on.
    h.wait_url_contains_for("expired=1", UNATTENDED_WAIT).await;
    assert_signed_out(&h).await;
    h.shot(2, "signed-out").await;
    h.assert_console_clean(&EXPECTED_CONSOLE).await;
    h.finish().await;
}
