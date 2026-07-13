//! openEHR identifier extraction — the ONLY place suites parse wire ids
//! (register 90 §8.3; replaces the legacy `support::version_uid`,
//! `contribution.rs::contribution_uid`/`version_uid_at` ad-hoc scrapers).
//!
//! Every extraction records the observed edition form on the context's
//! recorder, so a wire-form change is an explicit finding. There are NO
//! silent fallbacks: an extraction either yields the id from a declared
//! source or fails naming every source tried (the `unwrap_or_else(v1)`
//! class of masking bug — register 06 G-4 — is structurally impossible).

use serde_json::Value;

use crate::engine::harness::{CaseError, HttpResponse, RunContext};
use crate::wire::headers;

/// A structured `OBJECT_VERSION_ID`: `<object_id>::<creating_system_id>::<version_tree_id>`
/// (RM support §identification / BASE `base_types` §Identification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectVersionId {
    /// The full lexical form.
    pub full: String,
    /// The versioned-object uid (before the first `::`).
    pub object_id: String,
    /// The creating system id (between the `::`s).
    pub creating_system_id: String,
    /// The version tree id (after the last `::`).
    pub version_tree_id: String,
}

/// Parse an `OBJECT_VERSION_ID` lexical form.
///
/// # Errors
/// [`CaseError::Assertion`] if the value does not have the three
/// `::`-separated segments.
pub fn parse_object_version_id(value: &str) -> Result<ObjectVersionId, CaseError> {
    let parts: Vec<&str> = value.split("::").collect();
    let [object_id, creating_system_id, version_tree_id] = parts[..] else {
        return Err(CaseError::Assertion(format!(
            "{value:?} is not an OBJECT_VERSION_ID (expected <id>::<system>::<version>)"
        )));
    };
    if object_id.is_empty() || creating_system_id.is_empty() || version_tree_id.is_empty() {
        return Err(CaseError::Assertion(format!(
            "OBJECT_VERSION_ID {value:?} has an empty segment"
        )));
    }
    Ok(ObjectVersionId {
        full: value.to_owned(),
        object_id: object_id.to_owned(),
        creating_system_id: creating_system_id.to_owned(),
        version_tree_id: version_tree_id.to_owned(),
    })
}

/// The `OBJECT_VERSION_ID` a versioned-object write returned: the `ETag`
/// preferred (works regardless of `Prefer`/format; ITS-REST overview §"`ETag`
/// and Last-Modified"), else the representation body's `uid.value`. The
/// observed `ETag` form is recorded on the edition ladder.
///
/// # Errors
/// [`CaseError::Assertion`] if neither source yields an id — the error names
/// both sources (no silent fallback).
pub fn version_uid(ctx: &RunContext<'_>, response: &HttpResponse) -> Result<String, CaseError> {
    if let Some(tag) = headers::etag(response)? {
        ctx.edition
            .note(tag.edition, "ETag emitted in the deprecated bare form");
        return Ok(tag.value);
    }
    if let Ok(body) = response.json()
        && let Some(uid) = body["uid"]["value"].as_str()
    {
        return Ok(uid.to_owned());
    }
    Err(CaseError::Assertion(
        "no OBJECT_VERSION_ID: response has neither an ETag header nor a JSON body with uid.value"
            .to_owned(),
    ))
}

/// [`version_uid`] parsed into its segments.
///
/// # Errors
/// [`CaseError::Assertion`] if no id is found or it is not an
/// `OBJECT_VERSION_ID`.
pub fn version_id(
    ctx: &RunContext<'_>,
    response: &HttpResponse,
) -> Result<ObjectVersionId, CaseError> {
    parse_object_version_id(&version_uid(ctx, response)?)
}

/// The versioned-object uid — the segment before the first `::` of an
/// `OBJECT_VERSION_ID`; a bare uid passes through unchanged (`HIER_OBJECT_ID`).
#[must_use]
pub fn object_uid(version_uid: &str) -> &str {
    version_uid.split("::").next().unwrap_or(version_uid)
}

/// The `ehr_id` of an EHR representation body (`ehr_id.value`).
///
/// # Errors
/// [`CaseError::Assertion`] if the body carries none.
pub fn ehr_id(body: &Value) -> Result<String, CaseError> {
    body["ehr_id"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| CaseError::Assertion("EHR body has no ehr_id.value".to_owned()))
}

/// The `uid.value` of a versioned-object representation body.
///
/// # Errors
/// [`CaseError::Assertion`] if the body carries none.
pub fn body_uid(body: &Value) -> Result<String, CaseError> {
    body["uid"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| CaseError::Assertion("body has no uid.value".to_owned()))
}

/// The CONTRIBUTION uid a commit returned: `ETag` preferred, else the
/// representation body's `uid.value`, else the `Location` tail — each source
/// declared, the error naming all three.
///
/// # Errors
/// [`CaseError::Assertion`] if no source yields an id.
pub fn contribution_uid(
    ctx: &RunContext<'_>,
    response: &HttpResponse,
) -> Result<String, CaseError> {
    if let Some(tag) = headers::etag(response)? {
        ctx.edition
            .note(tag.edition, "ETag emitted in the deprecated bare form");
        return Ok(tag.value);
    }
    if let Ok(body) = response.json()
        && let Some(uid) = body["uid"]["value"].as_str()
    {
        return Ok(uid.to_owned());
    }
    if let Ok(tail) = headers::location_tail(response) {
        return Ok(tail);
    }
    Err(CaseError::Assertion(
        "no CONTRIBUTION uid: response has no ETag, no body uid.value, and no Location tail"
            .to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_version_id_parses_three_segments() {
        let v = parse_object_version_id("8849182c::local.ehrbase.org::2").expect("parse");
        assert_eq!(v.object_id, "8849182c");
        assert_eq!(v.creating_system_id, "local.ehrbase.org");
        assert_eq!(v.version_tree_id, "2");
        assert!(parse_object_version_id("justakey").is_err());
        assert!(parse_object_version_id("a::b").is_err());
        assert!(parse_object_version_id("a::::1").is_err());
    }

    #[test]
    fn object_uid_takes_first_segment() {
        assert_eq!(object_uid("abc::sys::1"), "abc");
        assert_eq!(object_uid("bare-uid"), "bare-uid");
    }
}
