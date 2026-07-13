//! Shared helpers for the suite modules: EHR/OPT setup and cross-chapter
//! assertion utilities. Wire ids come exclusively from [`crate::wire`]
//! (register 90 §8.3); per-SUT facts from the descriptor — there are no
//! system-id or template-id literals here.

use serde_json::Value;

use crate::engine::assert;
use crate::engine::harness::{CaseError, HttpRequest, HttpResponse, RunContext};
use crate::testdata::fixtures;
use crate::wire::ids;
use crate::wire::negotiate;

/// Create a default EHR (`POST /ehr`, no body — schedule master06
/// §create_ehr data-set class 1.b: the server creates the default
/// structures), asserting `201`, returning the `ehr_id`.
///
/// # Errors
/// [`CaseError`] on transport failure or a non-`201` response.
pub async fn create_ehr(ctx: &RunContext<'_>) -> Result<String, CaseError> {
    let resp = ctx
        .send(negotiate::representation(
            HttpRequest::post("/ehr"),
            crate::model::case::Format::Json,
        ))
        .await?;
    assert::status(&resp, 201)?;
    ids::ehr_id(&resp.json()?)
}

/// The `uid.value` (`OBJECT_VERSION_ID`) of a versioned-object body.
///
/// # Errors
/// [`CaseError::Assertion`] if the body has no `uid.value`.
pub fn uid_of(body: &Value) -> Result<String, CaseError> {
    ids::body_uid(body)
}

/// Upload an OPT (a `testdata` manifest key resolving under the corpus
/// `valid_templates/`) to the ADL 1.4 definition endpoint, tolerating a
/// re-upload: the SUT is shared across cases, so `2xx` = newly loaded and
/// `409` = already present both satisfy the master07/08 precondition "the
/// OPT … should exist on the server". Any other status is a real failure.
///
/// # Errors
/// [`CaseError`] on transport failure or an upload that is neither a
/// success nor an already-present conflict.
pub async fn ensure_opt(ctx: &RunContext<'_>, key: &str, file: &str) -> Result<(), CaseError> {
    let xml = fixtures::read_from(key, file).map_err(|e| CaseError::Codec(e.to_string()))?;
    ensure_opt_xml(ctx, &xml).await
}

/// Upload an in-memory OPT (already-serialized ADL 1.4 XML), tolerating a
/// re-upload (`2xx`/`409`) exactly like [`ensure_opt`]. This is the
/// provisioning path for **authored** constraint templates (master15
/// §Implementation notes: the archetypes "should be generated").
///
/// # Errors
/// [`CaseError`] on transport failure or an upload that is neither a
/// success nor an already-present conflict.
pub async fn ensure_opt_xml(ctx: &RunContext<'_>, xml: &str) -> Result<(), CaseError> {
    let resp = ctx
        .send(
            HttpRequest::post("/definition/template/adl1.4")
                .text_body(xml.to_owned(), "application/xml")
                // ITS-REST `definition_template_adl1.4_upload` declares no
                // `Accept` parameter and produces `application/xml` only —
                // requesting JSON is a strict 406 (ITS-REST definition API,
                // operation `…adl1.4_upload` + its 201 response schema).
                .header("accept", "application/xml"),
        )
        .await?;
    if (200..300).contains(&resp.status) || resp.status == 409 {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "OPT upload returned {} (expected 2xx or 409 already-present)",
            resp.status
        )))
    }
}

/// A syntactically valid `OBJECT_VERSION_ID` that names a version the SUT
/// does not hold: the observed id's object uid is replaced with a fresh
/// UUID, keeping the SUT's **own** creating system id and version tree id.
/// This is the If-Match / bad-version negative-input builder — it replaces
/// the legacy hardcoded `::conformance::` literals, which encoded our
/// server's system id into the instrument (registers 03/04/08 G-rows).
#[must_use]
pub fn nonexistent_version_like(observed: &ids::ObjectVersionId) -> String {
    format!(
        "{}::{}::{}",
        uuid::Uuid::new_v4(),
        observed.creating_system_id,
        observed.version_tree_id
    )
}

/// Assert a response is a negative (client-error) response — the schedule's
/// prose "negative response" concretized as an ITS-REST `4xx`. Used ONLY
/// where the operation's spec genuinely allows several client-error codes;
/// where an edition pins distinct codes, use
/// [`crate::engine::assert::status_ladder`] instead (register 05 G-1).
///
/// # Errors
/// [`CaseError::Assertion`] if the status is not in `400..500`.
pub fn assert_negative(resp: &HttpResponse) -> Result<(), CaseError> {
    if (400..500).contains(&resp.status) {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "expected a negative (4xx) response, got {}",
            resp.status
        )))
    }
}

/// Assert the retrieved representation equals the committed one under the
/// case's comparison mode — the content check the schedule mandates on
/// every `get_*` case (master07 flows: "check the content is the same
/// committed"; register 04 G-1 restored it from a structural-only check).
///
/// # Errors
/// [`CaseError::Assertion`] on a content mismatch.
pub fn assert_round_trip(
    mode: crate::model::case::Compare,
    committed: &Value,
    retrieved: &Value,
) -> Result<(), CaseError> {
    assert::compare(mode, committed, retrieved)
}
