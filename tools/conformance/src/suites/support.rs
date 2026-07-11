//! Shared helpers for the suite modules: EHR setup + common assertions used
//! across chapters (design §4.1). Keeps the per-chapter case modules small.

use serde_json::Value;

use crate::assert;
use crate::fixtures;
use crate::harness::{CaseError, HttpRequest, HttpResponse, RunContext};

/// Create a default EHR (POST `/ehr`, no body), asserting `201`, and return its
/// `ehr_id` from the `Location`/representation.
///
/// # Errors
/// [`CaseError`] on a transport failure or a non-`201` response.
pub async fn create_ehr(ctx: &RunContext<'_>) -> Result<String, CaseError> {
    let resp = ctx
        .send(
            HttpRequest::post("/ehr")
                .header("accept", "application/json")
                .header("prefer", "return=representation"),
        )
        .await?;
    assert::status(&resp, 201)?;
    let body: Value = resp.json()?;
    body["ehr_id"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| CaseError::Assertion("EHR body has no ehr_id".to_owned()))
}

/// The `uid.value` (`OBJECT_VERSION_ID`) of a versioned-object body.
///
/// # Errors
/// [`CaseError::Assertion`] if the body has no `uid.value`.
pub fn uid_of(body: &Value) -> Result<String, CaseError> {
    body["uid"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| CaseError::Assertion("body has no uid".to_owned()))
}

/// Upload an OPT (path relative to the corpus `valid_templates/`) to the ADL 1.4
/// definition endpoint, tolerating a re-upload: the SUT is shared
/// across cases, so `2xx` = newly loaded and `409` = already present are both a
/// success (the OPT is provisioned either way — master07/08 preconditions
/// "the OPT … should exist on the server"). Any other status is a real failure.
///
/// # Errors
/// [`CaseError`] on a transport failure or an OPT upload that is neither a
/// success nor an already-present conflict.
pub async fn ensure_opt(ctx: &RunContext<'_>, opt_rel: &str) -> Result<(), CaseError> {
    let xml = fixtures::read(&format!("valid_templates/{opt_rel}"))
        .map_err(|e| CaseError::Codec(e.to_string()))?;
    let resp = ctx
        .send(
            HttpRequest::post("/definition/template/adl1.4")
                .text_body(xml, "application/xml")
                // ITS-REST `definition_template_adl1.4_upload` declares NO `Accept`
                // parameter and produces `application/xml` only (the 201 body is an
                // `OperationalTemplate`); requesting `application/json` is a strict
                // 406 (see `operations/definition_template_adl1.4_upload.yaml` +
                // `responses/201_Template_adl1_4_upload.yaml`).
                .header("accept", "application/xml"),
        )
        .await?;
    if (200..300).contains(&resp.status) || resp.status == 409 {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "OPT {opt_rel} upload returned {} (expected 2xx or 409 already-present)",
            resp.status
        )))
    }
}

/// Upload an **in-memory** OPT (already-serialized ADL 1.4 XML) to the definition
/// endpoint, tolerating a re-upload (`2xx` or `409` already-present) exactly like
/// [`ensure_opt`].
///
/// This is the provisioning path for **authored** constraint templates (the
/// vendored corpus ships no OPT per archetype-constraint variant; master15–17
/// §Implementation notes: the archetypes "should be generated") — the suite
/// tightens a base OPT programmatically (`super::content::author`) and provisions
/// the result here, so a constraint case is executable *as specified* instead of
/// skipped.
///
/// # Errors
/// [`CaseError`] on a transport failure or an upload that is neither a success
/// nor an already-present conflict.
pub async fn ensure_opt_xml(ctx: &RunContext<'_>, xml: &str) -> Result<(), CaseError> {
    let resp = ctx
        .send(
            HttpRequest::post("/definition/template/adl1.4")
                .text_body(xml.to_owned(), "application/xml")
                // XML-only upload endpoint (no `Accept` param) — see `ensure_opt`.
                .header("accept", "application/xml"),
        )
        .await?;
    if (200..300).contains(&resp.status) || resp.status == 409 {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "authored OPT upload returned {} (expected 2xx or 409 already-present)",
            resp.status
        )))
    }
}

/// The version uid (`OBJECT_VERSION_ID`) a versioned-object write returns — the
/// `ETag` (ITS-REST `headers/ETag_*.yaml`, double-quoted) preferred (works
/// regardless of `Prefer`/wire format), else the representation body's
/// `uid.value`.
///
/// # Errors
/// [`CaseError::Assertion`] if neither an `ETag` nor a `uid.value` is present.
pub fn version_uid(resp: &HttpResponse) -> Result<String, CaseError> {
    if let Some(etag) = resp.header("etag") {
        let trimmed = etag.trim_matches('"');
        if !trimmed.is_empty() {
            return Ok(trimmed.to_owned());
        }
    }
    let body = resp.json()?;
    body["uid"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| CaseError::Assertion("write response has no ETag or uid.value".to_owned()))
}

/// The versioned-object uid — the id segment before `::` of an
/// `OBJECT_VERSION_ID` (`<vo_id>::<system>::<ver>`).
#[must_use]
pub fn object_uid(version_uid: &str) -> &str {
    version_uid.split("::").next().unwrap_or(version_uid)
}

/// Assert a response is a negative (client-error) response — the schedule's
/// prose "negative response" concretized as an ITS-REST `4xx`. Used where the
/// operation's spec declares several client-error codes and the schedule case
/// only requires that the request be rejected (not a specific code).
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
