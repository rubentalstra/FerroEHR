//! Shared helpers for the suite modules: EHR setup + common assertions used
//! across chapters (design §4.1). Keeps the per-chapter case modules small.

use serde_json::Value;

use crate::assert;
use crate::harness::{CaseError, HttpRequest, RunContext};

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
