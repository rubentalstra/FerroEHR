//! Typed response-header parsing for the version-id the driver threads through
//! its per-patient runtime table.
//!
//! Spec grounding: ITS-REST overview §"`ETag` and Last-Modified" — the `ETag` is
//! weak-type (`W/"…"`) in Release-1.1.0; the bare quoted form is the deprecated
//! Release-1.0.3-era emission. A hand-rolled quote-strip that kept the `W/`
//! prefix poisoned every stored uid, so the parse lives here, in one place.
//! Absorbed from the retired ECC wire layer (edition classification dropped —
//! the benchmark only needs the opaque value).

/// A parsed `ETag`: the opaque value between the quotes (for openEHR versioned
/// resources, an `OBJECT_VERSION_ID` or an `ehr_id`, per the operation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etag {
    /// The value between the quotes.
    pub value: String,
}

/// Parse an `ETag` header value into its opaque value, accepting both the weak
/// `W/"…"` form (Release-1.1.0) and the bare `"…"` form (Release-1.0.3).
///
/// # Errors
/// A message if the header is present but empty/unquotable.
pub fn parse_etag(raw: &str) -> Result<Etag, String> {
    let stripped = raw
        .strip_prefix("W/")
        .or_else(|| raw.strip_prefix("w/"))
        .unwrap_or(raw);
    let value = stripped.trim().trim_matches('"');
    if value.is_empty() {
        return Err(format!("ETag header {raw:?} carries no value"));
    }
    Ok(Etag {
        value: value.to_owned(),
    })
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn weak_etag_value_is_bare() {
        let e = parse_etag("W/\"abc::sys::1\"").expect("parse");
        assert_eq!(e.value, "abc::sys::1");
    }

    #[test]
    fn bare_etag_value_is_bare() {
        let e = parse_etag("\"abc::sys::1\"").expect("parse");
        assert_eq!(e.value, "abc::sys::1");
    }

    #[test]
    fn empty_etag_rejected() {
        assert!(parse_etag("\"\"").is_err());
    }
}
