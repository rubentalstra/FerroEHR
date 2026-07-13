//! Typed response-header parsing, edition-classified.
//!
//! Spec grounding: ITS-REST overview §"ETag and Last-Modified" — the ETag is
//! weak-type (`W/"…"`) in the development edition; the bare quoted form is
//! the deprecated Release-1.0.3-era emission. `Location` per the ITS-REST
//! operation responses; `Last-Modified` per the same overview section.

use crate::edition::Edition;
use crate::engine::harness::{CaseError, HttpResponse};

/// A parsed `ETag`: the opaque value with its observed wire form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etag {
    /// The value between the quotes (for openEHR versioned resources: an
    /// `OBJECT_VERSION_ID` or an ehr_id, per the operation).
    pub value: String,
    /// The edition rung the observed form belongs to: weak `W/"…"` =
    /// [`Edition::Development`]; deprecated bare `"…"` =
    /// [`Edition::Release103`].
    pub edition: Edition,
}

/// Parse an `ETag` header value into its opaque value + observed form.
///
/// # Errors
/// [`CaseError::Assertion`] if the header is present but empty/unquotable.
pub fn parse_etag(raw: &str) -> Result<Etag, CaseError> {
    let (stripped, edition) = match raw.strip_prefix("W/").or_else(|| raw.strip_prefix("w/")) {
        Some(rest) => (rest, Edition::Development),
        None => (raw, Edition::Release103),
    };
    let value = stripped.trim().trim_matches('"');
    if value.is_empty() {
        return Err(CaseError::Assertion(format!(
            "ETag header {raw:?} carries no value"
        )));
    }
    Ok(Etag {
        value: value.to_owned(),
        edition,
    })
}

/// The response's `ETag`, parsed; `Ok(None)` when the header is absent.
///
/// # Errors
/// [`CaseError::Assertion`] on a present-but-malformed header.
pub fn etag(response: &HttpResponse) -> Result<Option<Etag>, CaseError> {
    response.header("etag").map(parse_etag).transpose()
}

/// The response's `Location` header.
///
/// # Errors
/// [`CaseError::Assertion`] when absent.
pub fn location(response: &HttpResponse) -> Result<&str, CaseError> {
    response
        .header("location")
        .ok_or_else(|| CaseError::Assertion("response has no Location header".to_owned()))
}

/// The final path segment of the `Location` header (the created resource's
/// id on ITS-REST create responses).
///
/// # Errors
/// [`CaseError::Assertion`] when the header is absent or ends in `/`.
pub fn location_tail(response: &HttpResponse) -> Result<String, CaseError> {
    let loc = location(response)?;
    let tail = loc.rsplit('/').next().unwrap_or_default();
    if tail.is_empty() {
        return Err(CaseError::Assertion(format!(
            "Location {loc:?} has no trailing resource id"
        )));
    }
    // Percent-decoding is deliberately NOT applied here: ITS-REST resource
    // ids in Location are emitted undecoded; a case needing decoding states
    // it explicitly via `urlencoding` at the call site.
    Ok(tail.to_owned())
}

/// The response's `Last-Modified` header (ITS-REST overview §"ETag and
/// Last-Modified"), when present.
#[must_use]
pub fn last_modified(response: &HttpResponse) -> Option<&str> {
    response.header("last-modified")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resp(headers: Vec<(String, String)>) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers,
            body: Vec::new(),
        }
    }

    #[test]
    fn weak_etag_is_development_form() {
        let e = parse_etag("W/\"abc::sys::1\"").expect("parse");
        assert_eq!(e.value, "abc::sys::1");
        assert_eq!(e.edition, Edition::Development);
    }

    #[test]
    fn bare_etag_is_release_form() {
        let e = parse_etag("\"abc::sys::1\"").expect("parse");
        assert_eq!(e.value, "abc::sys::1");
        assert_eq!(e.edition, Edition::Release103);
    }

    #[test]
    fn empty_etag_rejected_and_absent_is_none() {
        assert!(parse_etag("\"\"").is_err());
        assert!(etag(&resp(vec![])).expect("absent ok").is_none());
    }

    #[test]
    fn location_tail_extracts_last_segment() {
        let r = resp(vec![(
            "location".to_owned(),
            "http://sut/ehr/123/composition/abc::sys::2".to_owned(),
        )]);
        assert_eq!(location_tail(&r).expect("tail"), "abc::sys::2");
    }
}
