//! Response-header assertion evaluation.
//!
//! This is the executed half of the
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
//! Two declaration-level modifiers gate whether a matcher is judged at all
//! (both defined on [`crate::model::binding::HeaderExpectation`], which
//! carries the decisive spec sentences):
//!
//! - `optional: true` (authored bare as `present?`) — the PRESENCE of the
//!   header is a SHOULD/MAY, so an absent or blank header satisfies the
//!   expectation outright; a header that IS there is judged in full by the
//!   matcher, so a MUST-strength FORM still bites.
//! - `applies: { its_rest: ">=1.1.0" }` — the rule is dated by the released
//!   text itself (the `W/` weakness indicator and the read/DELETE `Location`
//!   deprecation are both "Prior to Release 1.1.0" changes). A party whose
//!   declared spec versions do not satisfy the floor is not judged on it;
//!   everything else on the outcome still is.
//!
//! Matcher semantics (the closed [`HeaderMatcher`] vocabulary):
//!
//! - `present` — the header exists with a non-empty value.
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

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use std::collections::BTreeMap;

use crate::exec::assertions;
use crate::exec::state::VarStore;
use crate::model::binding::{HeaderExpectation, HeaderMatcher, WireExpectation};
use crate::party::SpecVersions;

/// The request-side context a matcher may need.
#[derive(Debug, Default)]
pub struct RequestContext<'a> {
    /// The `Accept` value the driver sent (the negotiated type).
    pub accept: Option<&'a str>,
    /// The latest version uid committed on this row (the newest successful
    /// `version_uid` binding capture), for `latest-version-uid`.
    pub last_version_uid: Option<&'a str>,
    /// The party statement's declared spec versions — the right-hand side of
    /// a version-dated expectation's `applies` floor. `None` (a
    /// statement-blind sweep) declares nothing, so a dated rule is out of
    /// scope exactly as it is for a party that omits the component.
    pub spec_versions: Option<&'a SpecVersions>,
}

/// Evaluate every declared header expectation of `expectation` against the
/// response headers; returns one failure line per violated expectation.
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
    for (name, header) in declared {
        if !in_scope(header, ctx) {
            continue;
        }
        let observed = header_value(response_headers, name);
        // A SHOULD/MAY-strength presence: nothing to judge when the server
        // exercised its latitude and omitted the header. A header that IS
        // there still faces the matcher in full.
        if header.optional && observed.is_none_or(|v| v.trim().is_empty()) {
            continue;
        }
        if let Some(failure) = judge(name, &header.matcher, observed, ctx, vars) {
            failures.push(failure);
        }
    }
    failures
}

/// Whether a version-dated expectation is in scope for the declaring party.
fn in_scope(header: &HeaderExpectation, ctx: &RequestContext<'_>) -> bool {
    match &header.applies {
        None => true,
        Some(applies) => ctx
            .spec_versions
            .is_some_and(|versions| applies.satisfied_by(versions)),
    }
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
            let resolved = match resolve_placeholders(pattern, vars) {
                Ok(resolved) => resolved,
                Err(missing) => {
                    return Some(format!(
                        "header {name}: pattern {pattern:?} names <{missing}>, which is \
                         neither a captured/with-supplied case variable nor a structural \
                         token (<n>, <system_id>) — refusing the vacuous wildcard (#1852)"
                    ));
                }
            };
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

/// Substitute `<name>` placeholders. A resolvable case variable inserts its
/// regex-escaped scalar; the two STRUCTURAL tokens the catalogue vocabulary
/// declares resolve to their grammars when no variable shadows them —
/// `<n>` to the `VERSION_TREE_ID` shape (BASE `base_types` master05
/// §`VERSION_TREE_ID`: a trunk ordinal or `trunk.branch.version`) and
/// `<system_id>` to a non-empty `::`-free segment. Any OTHER unresolvable
/// placeholder is a LOUD error, never a silent `.*` wildcard: a matcher like
/// `W/"<versioned_object_uid>::…"` degrading to a near-tautology is the
/// vacuous-assertion class of #1830, on the expectation side (#1852).
///
/// # Errors
/// The name of the first placeholder that is neither a case variable nor a
/// structural token.
fn resolve_placeholders(pattern: &str, vars: &VarStore) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(start) = rest.find('<') {
        let (head, tail) = rest.split_at(start);
        out.push_str(head);
        if let Some(end) = tail.find('>') {
            let name = tail.get(1..end).unwrap_or_default();
            let captured = crate::ids::CaptureName::parse(name)
                .ok()
                .and_then(|n| vars.scalar(&n).map(str::to_owned));
            match (captured, name) {
                (Some(value), _) => out.push_str(&regex::escape(&value)),
                // VERSION_TREE_ID: `[1-9][0-9]*` or dotted triple (BASE
                // base_types master05 §VERSION_TREE_ID).
                (None, "n") => out.push_str(r"[1-9][0-9]*(?:\.[0-9]+\.[0-9]+)?"),
                // A creating-system-id segment: non-empty, free of the `::`
                // separator and the quote that closes a weak ETag.
                (None, "system_id") => out.push_str(r#"[^:"]+"#),
                (None, missing) => return Err(missing.to_owned()),
            }
            rest = tail.get(end + 1..).unwrap_or_default();
        } else {
            out.push_str(tail);
            rest = "";
        }
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
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
        RequestContext::default()
    }

    fn versions(its_rest: &str) -> SpecVersions {
        SpecVersions {
            its_rest: Some(its_rest.to_owned()),
            ..SpecVersions::default()
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
            ..RequestContext::default()
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
            last_version_uid: Some("abc::sys::2"),
            ..RequestContext::default()
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
        // A different resolved uid fails; the structural tokens (<system_id>,
        // <n>) resolve to their grammars, not to a `.*` wildcard (#1852).
        let bad = response(&[("etag", "W/\"other-uid::any.system::2\"")]);
        assert_eq!(evaluate(&e, &bad, &ctx(), &vars).len(), 1);
        // The structural grammars are real constraints: an empty system
        // segment and a zero-led tree ordinal both fail.
        let empty_system = response(&[("etag", "W/\"abc-123::::2\"")]);
        assert_eq!(evaluate(&e, &empty_system, &ctx(), &vars).len(), 1);
        let zero_led = response(&[("etag", "W/\"abc-123::any.system::02\"")]);
        assert_eq!(evaluate(&e, &zero_led, &ctx(), &vars).len(), 1);
    }

    /// #1852 seeded defect: a placeholder naming neither a case variable nor
    /// a structural token is a loud failure — never a silent `.*` that turns
    /// the matcher into a tautology.
    #[test]
    fn unresolvable_placeholder_is_a_loud_failure_not_a_wildcard() {
        let e = expectation(&serde_json::json!({
            "ETag": "pattern:W/\"<no_such_capture>::x\""
        }));
        // The observed value would MATCH a `.*` degradation — the failure
        // must come from the refusal, not from a mismatch.
        let would_pass = response(&[("etag", "W/\"anything::x\"")]);
        let failures = evaluate(&e, &would_pass, &ctx(), &VarStore::default());
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(
            failures[0].contains("no_such_capture") && failures[0].contains("#1852"),
            "the failure names the unresolvable placeholder: {failures:?}"
        );
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

    /// `optional: true` splits SHOULD-strength PRESENCE from MUST-strength
    /// FORM (issue #628): the query `ETag` is a SHOULD to emit (overview
    /// §"`ETag` and Last-Modified") but a MUST to weaken when emitted
    /// (§"Deprecated headers"). An omitted header passes; a malformed one
    /// still fails.
    #[test]
    fn optional_presence_still_judges_the_form_when_present() {
        let e = expectation(&serde_json::json!({
            "ETag": { "match": "pattern:W/\"[^\"]+\"", "optional": true }
        }));
        assert!(evaluate(&e, &response(&[]), &ctx(), &VarStore::default()).is_empty());
        let blank = response(&[("etag", "   ")]);
        assert!(evaluate(&e, &blank, &ctx(), &VarStore::default()).is_empty());
        let weak = response(&[("etag", "W/\"rs-1\"")]);
        assert!(evaluate(&e, &weak, &ctx(), &VarStore::default()).is_empty());
        let bare = response(&[("etag", "\"rs-1\"")]);
        assert_eq!(evaluate(&e, &bare, &ctx(), &VarStore::default()).len(), 1);
    }

    /// A version-dated rule binds only the parties that declare the release
    /// dating it (issue #627): the `W/` MUST is "Prior to Release 1.1.0"
    /// deprecation text, so a 1.0.3 declarant is not judged on it and a
    /// 1.1.0 declarant is — the SAME `applies` grammar and the SAME
    /// `satisfied_by` polarity the case cores use.
    #[test]
    fn a_dated_expectation_binds_only_the_declaring_releases() {
        let e = expectation(&serde_json::json!({
            "ETag": {
                "match": "pattern:W/\"[^\"]+\"",
                "applies": { "its_rest": ">=1.1.0" }
            }
        }));
        let bare = response(&[("etag", "\"x::sys::1\"")]);

        let v11 = versions("1.1.0");
        let judged = RequestContext {
            spec_versions: Some(&v11),
            ..RequestContext::default()
        };
        assert_eq!(evaluate(&e, &bare, &judged, &VarStore::default()).len(), 1);

        let v103 = versions("1.0.3");
        let dated_out = RequestContext {
            spec_versions: Some(&v103),
            ..RequestContext::default()
        };
        assert!(evaluate(&e, &bare, &dated_out, &VarStore::default()).is_empty());

        // Undeclared behaves exactly as the case-level filter does: out of
        // scope, never a silently-applied requirement.
        assert!(evaluate(&e, &bare, &ctx(), &VarStore::default()).is_empty());
    }

    /// The two modifiers are independent: dating gates whether the rule is
    /// consulted at all, optionality gates whether an absent header is a
    /// violation.
    #[test]
    fn dating_and_optionality_compose() {
        let e = expectation(&serde_json::json!({
            "ETag": {
                "match": "pattern:W/\"[^\"]+\"",
                "optional": true,
                "applies": { "its_rest": ">=1.1.0" }
            }
        }));
        let v11 = versions("1.1.0");
        let judged = RequestContext {
            spec_versions: Some(&v11),
            ..RequestContext::default()
        };
        assert!(evaluate(&e, &response(&[]), &judged, &VarStore::default()).is_empty());
        let bare = response(&[("etag", "\"rs-1\"")]);
        assert_eq!(evaluate(&e, &bare, &judged, &VarStore::default()).len(), 1);
    }
}
