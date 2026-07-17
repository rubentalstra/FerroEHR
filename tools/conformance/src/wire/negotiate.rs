//! Request-side content negotiation + committal headers, built in one place
//! so a wire-form change is one edit, not a suite sweep.
//!
//! Spec grounding: ITS-REST overview §Content negotiation (`Accept` /
//! `Content-Type`), §Prefer (`return=minimal|representation`,
//! `resolve_refs`), and the committal headers `openEHR-VERSION.*` /
//! `openEHR-AUDIT_DETAILS.*` (overview §Committal audit metadata — header
//! *names* are case-insensitive per RFC 9110 §5.1, so casing is not an
//! edition axis; the axis is which headers an edition understands).

use crate::engine::harness::HttpRequest;
use crate::model::case::Format;

/// `Prefer: return=…` values (ITS-REST overview §Prefer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferReturn {
    /// `return=minimal` (the ITS-REST default).
    Minimal,
    /// `return=representation`.
    Representation,
}

impl PreferReturn {
    /// The header value.
    #[must_use]
    pub const fn value(self) -> &'static str {
        match self {
            PreferReturn::Minimal => "return=minimal",
            PreferReturn::Representation => "return=representation",
        }
    }
}

/// Add `Accept` for the run's wire format.
#[must_use]
pub fn accept(request: HttpRequest, format: Format) -> HttpRequest {
    request.header("accept", format.media_type())
}

/// Add `Prefer: return=…`.
#[must_use]
pub fn prefer(request: HttpRequest, ret: PreferReturn) -> HttpRequest {
    request.header("prefer", ret.value())
}

/// Add `Accept` + `Prefer: return=representation` — the common read-back
/// shape for create/update cases that assert content round-trips.
#[must_use]
pub fn representation(request: HttpRequest, format: Format) -> HttpRequest {
    prefer(accept(request, format), PreferReturn::Representation)
}

/// Add an `If-Match` precondition carrying an `OBJECT_VERSION_ID` (ITS-REST
/// overview §Concurrency control). Emitted in the weak-comparison-safe
/// quoted form both editions accept on request.
#[must_use]
pub fn if_match(request: HttpRequest, version_uid: &str) -> HttpRequest {
    request.header("if-match", format!("\"{version_uid}\""))
}

/// The committal audit headers (`openEHR-AUDIT_DETAILS.*`, ITS-REST overview
/// §Committal audit metadata): description + optionally committer name.
#[must_use]
pub fn audit_details(
    mut request: HttpRequest,
    description: &str,
    committer_name: Option<&str>,
) -> HttpRequest {
    request = request.header(
        "openEHR-AUDIT_DETAILS.change_type",
        "code_string=\"249\"".to_owned(),
    );
    request = request.header(
        "openEHR-AUDIT_DETAILS.description",
        format!("value=\"{description}\""),
    );
    if let Some(name) = committer_name {
        request = request.header("openEHR-VERSION.committer", format!("name=\"{name}\""));
    }
    request
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
    use crate::engine::harness::HttpRequest;

    #[test]
    fn representation_sets_accept_and_prefer() {
        let r = representation(HttpRequest::post("/ehr"), Format::Json);
        assert!(
            r.headers
                .iter()
                .any(|(k, v)| k == "accept" && v == "application/json")
        );
        assert!(
            r.headers
                .iter()
                .any(|(k, v)| k == "prefer" && v == "return=representation")
        );
    }

    #[test]
    fn if_match_quotes_the_version_uid() {
        let r = if_match(HttpRequest::put("/x"), "abc::sys::1");
        assert!(
            r.headers
                .iter()
                .any(|(k, v)| k == "if-match" && v == "\"abc::sys::1\"")
        );
    }
}
