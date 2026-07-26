//! Response-header assertion evaluation — the executed half of the
//! catalogue's `outcomes.*.headers` declarations (issue #403: the matchers
//! were parsed by the binding model but never evaluated, so every header
//! declaration was documentation, not an assertion).
//!
//! Evaluation runs ONLY when the step's observation matched the EXPECTED
//! outcome kind (the declared headers belong to that outcome's wire
//! expectation); a failed matcher is a conformance FAILURE of the row (law
//! (b) — the same channel as the body/RM assertion failures), never an
//! inconclusive error: the exchange completed, and the spec sentence the
//! binding cites assigns the header.
//!
//! Matcher semantics (the closed [`HeaderMatcher`] vocabulary):
//!
//! - `present` — the header exists with a non-empty value.
//! - `present?` — never fails: the declaration documents a MAY-level header
//!   (e.g. `Preference-Applied` — ITS-REST `Requests_and_responses.md`
//!   §Representation details negotiation "MAY include") whose VALUE is
//!   checked when present via the sibling `pattern:`/literal form; presence
//!   itself is not assertable without over-claiming the MAY.
//! - `absent` — the header must not exist (e.g. `Location` on reads —
//!   overview §Location "MUST NOT be used to indicate an alternate
//!   representation").
//! - `negotiated` — the header's media token (parameters stripped) equals
//!   the request's negotiated `Accept` token.
//! - `latest-version-uid` — the stale-precondition rule (overview §"If-Match
//!   and accidental overwrites": the 412 "SHOULD return also latest
//!   `version_uid` in the `ETag`"): the entity-tag's payload (weak/quoted
//!   wrapper stripped) equals the latest version uid this row committed,
//!   compared case-insensitively (BASE `master05` §Composite Identifiers and
//!   Case). When the row committed nothing the runner can compare against
//!   (no tracked uid), the matcher degrades to `present` — honest, never a
//!   false red.
//! - `pattern:<regex>` — full-value match after `<name>` placeholders
//!   resolve: a case variable named `name` substitutes its regex-escaped
//!   value; an unresolvable placeholder wildcards to `.*` (the same rule the
//!   parse-time compile probe uses).
//! - a literal — template-rendered against the case variables, exact match.

use std::collections::BTreeMap;

use crate::exec::assertions;
use crate::exec::state::VarStore;
use crate::model::binding::{HeaderMatcher, WireExpectation};

/// The request-side context a matcher may need.
#[derive(Debug)]
pub struct RequestContext<'a> {
    /// The `Accept` value the driver sent (the negotiated type).
    pub accept: Option<&'a str>,
    /// The latest version uid committed on this row (the newest successful
    /// `version_uid` binding capture), for `latest-version-uid`.
    pub last_version_uid: Option<&'a str>,
}

/// Evaluate every declared header matcher of `expectation` against the
/// response headers; returns one failure line per violated matcher.
#[must_use]
pub fn evaluate(
    expectation: &WireExpectation,
    response_headers: &BTreeMap<String, String>,
    ctx: &RequestContext<'_>,
    vars: &VarStore,
) -> Vec<String> {
    let Some(declared) = expectation.headers.as_deref() else {
        return Vec::new();
    };
    let mut failures = Vec::new();
    for (name, matcher) in declared {
        let observed = header_value(response_headers, name);
        if let Some(failure) = judge(name, matcher, observed, ctx, vars) {
            failures.push(failure);
        }
    }
    failures
}

/// Case-insensitive header lookup (RFC 9110 §5.1: field names are
/// case-insensitive; reqwest lowercases, but the map is the transcript seam
/// so the lookup must not depend on that).
fn header_value<'a>(headers: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Judge one matcher; `None` = satisfied, `Some(reason)` = failed.
fn judge(
    name: &str,
    matcher: &HeaderMatcher,
    observed: Option<&str>,
    ctx: &RequestContext<'_>,
    vars: &VarStore,
) -> Option<String> {
    match matcher {
        HeaderMatcher::Present => match observed {
            Some(v) if !v.trim().is_empty() => None,
            _ => Some(format!("header {name}: expected present, got none")),
        },
        HeaderMatcher::PresentOptional => None,
        HeaderMatcher::Absent => observed.map(|v| {
            format!("header {name}: expected absent, got {v:?} (the binding's cited spec sentence forbids it on this outcome)")
        }),
        HeaderMatcher::Negotiated => {
            let Some(accept) = ctx.accept.map(media_token) else {
                // No Accept was sent — the endpoint default was negotiated;
                // nothing sound to compare against.
                return None;
            };
            match observed {
                Some(v) if media_token(v).eq_ignore_ascii_case(accept) => None,
                Some(v) => Some(format!(
                    "header {name}: expected the negotiated media type {accept:?}, got {v:?}"
                )),
                None => Some(format!(
                    "header {name}: expected the negotiated media type {accept:?}, got none"
                )),
            }
        }
        HeaderMatcher::LatestVersionUid => match observed {
            None => Some(format!(
                "header {name}: expected the latest version uid, got none"
            )),
            Some(v) => {
                let payload = strip_entity_tag(v);
                if payload.is_empty() {
                    return Some(format!(
                        "header {name}: expected the latest version uid, got an empty entity tag"
                    ));
                }
                match ctx.last_version_uid {
                    // Compared case-insensitively: OBJECT_VERSION_ID is a
                    // composite identifier (BASE master05 §Composite
                    // Identifiers and Case).
                    Some(latest) if payload.eq_ignore_ascii_case(latest) => None,
                    Some(latest) => Some(format!(
                        "header {name}: expected the latest version uid {latest:?}, got {payload:?}"
                    )),
                    // Nothing committed on this row to compare against —
                    // presence + non-emptiness is all the runner can
                    // soundly assert.
                    None => None,
                }
            }
        },
        HeaderMatcher::Pattern(pattern) => {
            let Some(v) = observed else {
                return Some(format!(
                    "header {name}: expected a value matching {pattern:?}, got none"
                ));
            };
            let resolved = resolve_placeholders(pattern, vars);
            match regex::Regex::new(&format!("^(?:{resolved})$")) {
                Ok(re) if re.is_match(v) => None,
                Ok(_) => Some(format!(
                    "header {name}: value {v:?} does not match {pattern:?} (resolved: {resolved:?})"
                )),
                // The parse-time probe compiled the wildcarded form; a
                // resolved form that no longer compiles is a runner bug —
                // surface it loudly as a failure line.
                Err(e) => Some(format!(
                    "header {name}: resolved pattern {resolved:?} does not compile: {e}"
                )),
            }
        }
        HeaderMatcher::Literal(template) => {
            let Some(v) = observed else {
                return Some(format!("header {name}: expected a value, got none"));
            };
            match assertions::render_template(template, vars) {
                Ok(want) if want == v => None,
                Ok(want) => Some(format!("header {name}: expected {want:?}, got {v:?}")),
                Err(e) => Some(format!(
                    "header {name}: literal template unresolvable ({e}) — declared value cannot be checked"
                )),
            }
        }
    }
}

/// The media token of a `Content-Type`/`Accept` value — parameters
/// (`; charset=…`) stripped, whitespace trimmed (RFC 9110 §8.3).
fn media_token(value: &str) -> &str {
    value.split(';').next().unwrap_or(value).trim()
}

/// The payload of an entity tag: the weak indicator and surrounding quotes
/// stripped (`W/"x"` → `x`; `"x"` → `x`; bare stays bare) — the same
/// tolerance the SUT-side decode grants (overview §"`ETag` and Last-Modified"
/// weak form + §"Deprecated headers" bare form).
fn strip_entity_tag(value: &str) -> &str {
    let v = value.trim();
    let v = v
        .strip_prefix("W/")
        .or_else(|| v.strip_prefix("w/"))
        .unwrap_or(v);
    v.trim_matches('"')
}

/// Substitute `<name>` placeholders: a resolvable case variable inserts its
/// regex-escaped scalar; an unresolvable one wildcards to `.*` (mirroring
/// the parse-time compile probe).
fn resolve_placeholders(pattern: &str, vars: &VarStore) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(start) = rest.find('<') {
        let (head, tail) = rest.split_at(start);
        out.push_str(head);
        if let Some(end) = tail.find('>') {
            let name = tail.get(1..end).unwrap_or_default();
            match crate::ids::CaptureName::parse(name)
                .ok()
                .and_then(|n| vars.scalar(&n).map(str::to_owned))
            {
                Some(value) => out.push_str(&regex::escape(&value)),
                None => out.push_str(".*"),
            }
            rest = tail.get(end + 1..).unwrap_or_default();
        } else {
            out.push_str(tail);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;
    use crate::exec::state::Captured;
    use crate::ids::CaptureName;

    fn expectation(headers_yaml: &serde_json::Value) -> WireExpectation {
        serde_json::from_value(serde_json::json!({
            "status": 200,
            "headers": headers_yaml
        }))
        .unwrap()
    }

    fn response(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn ctx<'a>() -> RequestContext<'a> {
        RequestContext {
            accept: None,
            last_version_uid: None,
        }
    }

    #[test]
    fn present_and_absent_are_enforced() {
        let e = expectation(&serde_json::json!({ "ETag": "present", "Location": "absent" }));
        let ok = response(&[("etag", "W/\"x::sys::1\"")]);
        assert!(evaluate(&e, &ok, &ctx(), &VarStore::default()).is_empty());

        let bad = response(&[("location", "/somewhere")]);
        let failures = evaluate(&e, &bad, &ctx(), &VarStore::default());
        assert_eq!(failures.len(), 2, "{failures:?}");
        assert!(failures[0].contains("ETag"));
        assert!(failures[1].contains("Location"));
    }

    #[test]
    fn present_optional_never_fails() {
        let e = expectation(&serde_json::json!({ "Preference-Applied": "present?" }));
        assert!(evaluate(&e, &response(&[]), &ctx(), &VarStore::default()).is_empty());
    }

    #[test]
    fn negotiated_compares_the_media_token() {
        let e = expectation(&serde_json::json!({ "Content-Type": "negotiated" }));
        let ctx = RequestContext {
            accept: Some("application/json"),
            last_version_uid: None,
        };
        let ok = response(&[("content-type", "application/json; charset=utf-8")]);
        assert!(evaluate(&e, &ok, &ctx, &VarStore::default()).is_empty());
        let bad = response(&[("content-type", "application/openehr.wt+json")]);
        assert_eq!(evaluate(&e, &bad, &ctx, &VarStore::default()).len(), 1);
    }

    #[test]
    fn latest_version_uid_compares_case_insensitively() {
        let e = expectation(&serde_json::json!({ "ETag": "latest-version-uid" }));
        let ctx = RequestContext {
            accept: None,
            last_version_uid: Some("abc::sys::2"),
        };
        // The weak wrapper strips; case differences are the same identifier
        // (BASE master05 §Composite Identifiers and Case).
        let ok = response(&[("etag", "W/\"ABC::SYS::2\"")]);
        assert!(evaluate(&e, &ok, &ctx, &VarStore::default()).is_empty());
        let stale = response(&[("etag", "W/\"abc::sys::1\"")]);
        assert_eq!(evaluate(&e, &stale, &ctx, &VarStore::default()).len(), 1);
        let missing = response(&[]);
        assert_eq!(evaluate(&e, &missing, &ctx, &VarStore::default()).len(), 1);
    }

    #[test]
    fn pattern_resolves_placeholders_from_vars() {
        let e = expectation(&serde_json::json!({
            "ETag": "pattern:W/\"<versioned_object_uid>::<system_id>::<n>\""
        }));
        let mut vars = VarStore::default();
        vars.set(
            CaptureName::parse("versioned_object_uid").unwrap(),
            Captured::Scalar("abc-123".to_owned()),
        );
        let ok = response(&[("etag", "W/\"abc-123::any.system::2\"")]);
        assert!(evaluate(&e, &ok, &ctx(), &vars).is_empty());
        // A different resolved uid fails; the unresolved placeholders stay
        // wildcards.
        let bad = response(&[("etag", "W/\"other-uid::any.system::2\"")]);
        assert_eq!(evaluate(&e, &bad, &ctx(), &vars).len(), 1);
    }

    #[test]
    fn literal_renders_the_template() {
        let e = expectation(&serde_json::json!({ "Preference-Applied": "return=minimal" }));
        let ok = response(&[("preference-applied", "return=minimal")]);
        assert!(evaluate(&e, &ok, &ctx(), &VarStore::default()).is_empty());
        let bad = response(&[("preference-applied", "return=representation")]);
        assert_eq!(evaluate(&e, &bad, &ctx(), &VarStore::default()).len(), 1);
    }

    #[test]
    fn header_lookup_is_case_insensitive() {
        let e = expectation(&serde_json::json!({ "Last-Modified": "present" }));
        let ok = response(&[("LAST-MODIFIED", "Wed, 22 Jul 2009 19:15:56 GMT")]);
        assert!(evaluate(&e, &ok, &ctx(), &VarStore::default()).is_empty());
    }
}
