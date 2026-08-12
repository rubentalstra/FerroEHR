// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Response-body selector evaluation.
//!
//! This is the executed half of the catalogue's
//! `outcomes.*.body` declarations (issue #415: the selectors were parsed by
//! the binding model but never evaluated, so every body declaration was
//! documentation, not an assertion — the same defect issue #403 closed for
//! the header matchers in [`crate::exec::headers`]).
//!
//! Evaluation runs ONLY when the step's observation matched the EXPECTED
//! outcome kind (the declared selector belongs to that outcome's wire
//! expectation); a violated selector is a conformance FAILURE of the row
//! (law (b) — the same channel as the RM/header assertion failures), never
//! an inconclusive error: the exchange completed, and the spec sentence the
//! binding cites assigns the body.
//!
//! Selector semantics (the closed [`BodySelector`] vocabulary). Each one is
//! deliberately the FLOOR the ITS-REST docs text (the wire oracle) supports —
//! a MAY is never turned into an assertion:
//!
//! - `present` — the response carries content. "Content" is any body other
//!   than none, JSON `null`, or an all-whitespace string (the driver already
//!   maps an empty response text to no body); the selector says nothing
//!   about the body's shape, so it is the one selector that judges a
//!   non-JSON (canonical-XML, ADL2 text) body too.
//! - `absent` — the response carries no content, by the same definition.
//!   Grounded per declaration by the binding's cited sentence (e.g.
//!   `Requests_and_responses.md` §HTTP status codes, the 204 row: "The
//!   request has been fulfilled and there is no additional content to send
//!   in the response payload body").
//! - `error_loose` — the loose error-detail shape. The detail itself is a
//!   MAY: "For `4xx` and `5xx` status codes, services MAY return additional
//!   error details if the `Prefer: return=representation` header is present
//!   in the request" (`Requests_and_responses.md` §HTTP status codes). So a
//!   missing body, a non-JSON body, and a non-object JSON body ALL pass, and
//!   nothing is judged at all unless the request actually sent
//!   `Prefer: return=representation`. What IS asserted, when the service
//!   does return a JSON object under that preference, is the one member the
//!   section's worked example fixes: a non-empty `message` string.
//! - `result_set_body` — the body is a `RESULT_SET`: a JSON object carrying
//!   the `rows` array (ITS-REST query `Response.md` §`RESULT_SET` response →
//!   `schemas/query/ResultSet.yaml`, whose only `required` member is
//!   `rows`). This is the same discriminator the normative comparator uses
//!   ([`crate::exec::resultset`]), so a selector pass and a `result_set`
//!   assertion can never disagree about what a result set is.
//! - `negotiated` — the body's media type is the negotiated one:
//!   "Proper header `Content-Type: application/json` MUST be present in the
//!   response of the service unless the response has no content body (HTTP
//!   status code `204`)" (`Resources.md` §JSON Format; §XML Format and
//!   §Simplified Formats carry the identical sentence for their types). The
//!   MUST is exempted for a no-content body, so a body-less response passes;
//!   with no `Accept` sent there is nothing sound to compare against (the
//!   endpoint default was negotiated) and the selector passes; an `Accept`
//!   offering several media ranges passes on ANY of them, and a wildcard
//!   range (`*/*`, `type/*`) makes every type acceptable.
//! - `prefer_conditional` — the §Representation details negotiation contract,
//!   branched on the `Prefer` the driver actually SENT:
//!   * `return=representation` — "the response body SHOULD contain the full
//!     representation of the resource" (§Prefer minimal, identifier or full
//!     representation response), so an empty body is a violation. Only
//!     non-emptiness is asserted: "full representation" is not a shape the
//!     docs text pins format-independently, and the resource assertions on
//!     the case step are where a body's content is judged.
//!   * `return=identifier` — "This is a variant of preference that implies
//!     minimal response semantics, but with a non-empty response body (i.e.
//!     the status will be `201 Created` or `200 OK`, never `204 No
//!     Content`). … when `application/json` is requested as above, the
//!     response body will be a single JSON object with a single `uid`
//!     attribute" (§Prefer only identifier). Asserted conservatively: a
//!     non-empty body, and — for a JSON body — a `uid` member. The "single
//!     attribute" half is NOT asserted (a service adding a member is not
//!     refuted by any MUST), and a non-JSON body is left to the negotiated
//!     selector.
//!   * `return=minimal`, an unrecognized preference, or no `Prefer` at all —
//!     never fails. The released text leaves the minimal branch open on both
//!     sides: "The HTTP status is typically `201 Created`. If no response
//!     body is returned, the service SHOULD use `204 No Content`" — neither
//!     presence nor absence is assigned (the same silence the catalogue
//!     carries as the `header-prefer-return-minimal` wire-surface element).

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use std::collections::BTreeMap;

use serde_json::Value;

use crate::model::binding::{BodySelector, WireExpectation};

/// The request-side context a selector may need.
#[derive(Debug)]
pub struct RequestContext<'a> {
    /// The `Accept` value the driver sent (the negotiated type).
    pub accept: Option<&'a str>,
    /// The `Prefer` value the driver sent, for `prefer_conditional` and the
    /// `error_loose` precondition.
    pub prefer: Option<&'a str>,
}

/// Evaluate the declared body selector of `expectation` against the observed
/// response; returns one failure line when the selector is violated.
#[must_use]
pub fn evaluate(
    expectation: &WireExpectation,
    body: Option<&Value>,
    response_headers: &BTreeMap<String, String>,
    ctx: &RequestContext<'_>,
) -> Vec<String> {
    let Some(selector) = expectation.body else {
        return Vec::new();
    };
    judge(selector, body, response_headers, ctx)
        .into_iter()
        .collect()
}

/// Judge one selector; `None` = satisfied, `Some(reason)` = failed.
fn judge(
    selector: BodySelector,
    body: Option<&Value>,
    response_headers: &BTreeMap<String, String>,
    ctx: &RequestContext<'_>,
) -> Option<String> {
    match selector {
        BodySelector::Present => content(body)
            .is_none()
            .then(|| "body: expected a response body, got none".to_owned()),
        BodySelector::Absent => {
            content(body).map(|v| format!("body: expected no response body, got {}", preview(v)))
        }
        BodySelector::ErrorLoose => judge_error_loose(body, ctx),
        BodySelector::ResultSetBody => judge_result_set(body),
        BodySelector::Negotiated => judge_negotiated(body, response_headers, ctx),
        BodySelector::PreferConditional => judge_prefer_conditional(body, ctx),
    }
}

/// `error_loose`: judged only under `Prefer: return=representation` (the
/// detail is a MAY otherwise) and only when a JSON object came back.
fn judge_error_loose(body: Option<&Value>, ctx: &RequestContext<'_>) -> Option<String> {
    if return_preference(ctx.prefer) != Some(ReturnPreference::Representation) {
        return None;
    }
    // A missing / non-JSON / non-object body is the MAY not exercised —
    // never a failure.
    let Some(Value::Object(map)) = content(body) else {
        return None;
    };
    match map.get("message") {
        Some(Value::String(m)) if !m.trim().is_empty() => None,
        Some(Value::String(_)) => {
            Some("body: the error detail's `message` is empty, expected the error text".to_owned())
        }
        Some(other) => Some(format!(
            "body: the error detail's `message` is {}, expected a string",
            preview(other)
        )),
        None => Some(
            "body: the error detail object carries no `message` (the error text the section's worked example fixes)"
                .to_owned(),
        ),
    }
}

/// `result_set_body`: a `RESULT_SET` — an object carrying the required
/// `rows` array.
fn judge_result_set(body: Option<&Value>) -> Option<String> {
    match content(body) {
        Some(Value::Object(map)) => match map.get("rows") {
            Some(Value::Array(_)) => None,
            Some(other) => Some(format!(
                "body: RESULT_SET `rows` is {}, expected an array",
                preview(other)
            )),
            None => Some(
                "body: expected a RESULT_SET, got an object without the required `rows` array"
                    .to_owned(),
            ),
        },
        Some(other) => Some(format!(
            "body: expected a RESULT_SET object, got {}",
            preview(other)
        )),
        None => Some("body: expected a RESULT_SET object, got no body".to_owned()),
    }
}

/// `negotiated`: the content type of a non-empty body equals a media range
/// the request offered.
fn judge_negotiated(
    body: Option<&Value>,
    response_headers: &BTreeMap<String, String>,
    ctx: &RequestContext<'_>,
) -> Option<String> {
    // No Accept sent — the endpoint default was negotiated; nothing sound to
    // compare against.
    let accept = ctx.accept?;
    // The MUST is exempted for a response with no content body.
    content(body)?;
    let offered: Vec<&str> = accept.split(',').map(media_token).collect();
    if offered.iter().any(|t| t.ends_with("/*") || *t == "*") {
        return None;
    }
    let Some(v) = header_value(response_headers, "Content-Type") else {
        return Some(format!(
            "body: a response with content must carry Content-Type (expected the negotiated {accept:?}), got none"
        ));
    };
    let observed = media_token(v);
    if offered.iter().any(|t| t.eq_ignore_ascii_case(observed)) {
        None
    } else {
        Some(format!(
            "body: expected the negotiated media type {accept:?}, got Content-Type {v:?}"
        ))
    }
}

/// `prefer_conditional`: the representation / identifier / minimal branches.
fn judge_prefer_conditional(body: Option<&Value>, ctx: &RequestContext<'_>) -> Option<String> {
    match return_preference(ctx.prefer) {
        Some(ReturnPreference::Representation) => content(body).is_none().then(|| {
            "body: Prefer return=representation was sent, expected the resource representation, got no body"
                .to_owned()
        }),
        Some(ReturnPreference::Identifier) => match content(body) {
            // A non-JSON body is the negotiated representation's business;
            // only the JSON shape is pinned by the worked example.
            Some(Value::Object(map)) if map.contains_key("uid") => None,
            Some(Value::Object(_)) => Some(
                "body: Prefer return=identifier was sent, expected an object carrying `uid`, got one without it"
                    .to_owned(),
            ),
            Some(_) => None,
            None => Some(
                "body: Prefer return=identifier was sent, expected the identifier body (never 204), got no body"
                    .to_owned(),
            ),
        },
        // return=minimal, an unrecognized preference, or none: the released
        // text assigns neither presence nor absence.
        _ => None,
    }
}

/// The `return=` preference of a `Prefer` field value (RFC 7240 §2: a
/// comma-separated preference list; token comparison is case-insensitive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReturnPreference {
    Minimal,
    Identifier,
    Representation,
}

fn return_preference(prefer: Option<&str>) -> Option<ReturnPreference> {
    prefer?.split(',').find_map(|part| {
        let token = part.split(';').next().unwrap_or(part).trim();
        // The preference NAME is case-insensitive (RFC 7240 §2), so the
        // prefix is matched case-insensitively rather than with a literal
        // `strip_prefix`.
        let name_len = "return=".len();
        if !token
            .get(..name_len)
            .is_some_and(|p| p.eq_ignore_ascii_case("return="))
        {
            return None;
        }
        let value = token.get(name_len..)?.trim().trim_matches('"');
        if value.eq_ignore_ascii_case("minimal") {
            Some(ReturnPreference::Minimal)
        } else if value.eq_ignore_ascii_case("identifier") {
            Some(ReturnPreference::Identifier)
        } else if value.eq_ignore_ascii_case("representation") {
            Some(ReturnPreference::Representation)
        } else {
            None
        }
    })
}

/// The body's carried content: `None` when the response carried nothing to
/// judge (no body, JSON `null`, or an all-whitespace non-JSON payload — the
/// driver already maps an empty response text to no body).
fn content(body: Option<&Value>) -> Option<&Value> {
    match body {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.trim().is_empty() => None,
        Some(v) => Some(v),
    }
}

/// Case-insensitive header lookup (RFC 9110 §5.1: field names are
/// case-insensitive), mirroring [`crate::exec::headers`].
fn header_value<'a>(headers: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// The media token of a `Content-Type`/`Accept` media range — parameters
/// (`; charset=…`, `; q=…`) stripped, whitespace trimmed (RFC 9110 §8.3).
fn media_token(value: &str) -> &str {
    value.split(';').next().unwrap_or(value).trim()
}

/// An 80-character diagnostic preview of a body (or body member).
fn preview(value: &Value) -> String {
    value.to_string().chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expectation(selector: &str) -> WireExpectation {
        serde_json::from_value(serde_json::json!({ "status": 200, "body": selector })).unwrap()
    }

    fn headers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn ctx<'a>() -> RequestContext<'a> {
        RequestContext {
            accept: None,
            prefer: None,
        }
    }

    fn run(selector: &str, body: Option<&Value>, ctx: &RequestContext<'_>) -> Vec<String> {
        evaluate(&expectation(selector), body, &headers(&[]), ctx)
    }

    #[test]
    fn no_declaration_evaluates_nothing() {
        let e: WireExpectation =
            serde_json::from_value(serde_json::json!({ "status": 204 })).unwrap();
        assert!(evaluate(&e, None, &headers(&[]), &ctx()).is_empty());
    }

    #[test]
    fn present_requires_content_of_any_shape() {
        let json = serde_json::json!({ "_type": "CONTRIBUTION" });
        assert!(run("present", Some(&json), &ctx()).is_empty());
        // A canonical-XML body is stored as a string — still a body.
        let xml = Value::String("<contribution/>".to_owned());
        assert!(run("present", Some(&xml), &ctx()).is_empty());

        assert_eq!(run("present", None, &ctx()).len(), 1);
        assert_eq!(run("present", Some(&Value::Null), &ctx()).len(), 1);
        let blank = Value::String("  ".to_owned());
        assert_eq!(run("present", Some(&blank), &ctx()).len(), 1);
    }

    #[test]
    fn absent_forbids_content() {
        assert!(run("absent", None, &ctx()).is_empty());
        assert!(run("absent", Some(&Value::Null), &ctx()).is_empty());
        let body = serde_json::json!({ "uid": "x::sys::1" });
        let failures = run("absent", Some(&body), &ctx());
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].contains("expected no response body"));
    }

    #[test]
    fn error_loose_only_judges_under_return_representation() {
        let detail = serde_json::json!({ "error": "Unprocessable Entity" });
        // No Prefer sent: the MAY is not engaged, nothing is judged.
        assert!(run("error_loose", Some(&detail), &ctx()).is_empty());
        let minimal = RequestContext {
            accept: None,
            prefer: Some("return=minimal"),
        };
        assert!(run("error_loose", Some(&detail), &minimal).is_empty());
    }

    #[test]
    fn error_loose_asserts_the_message_string() {
        let representation = RequestContext {
            accept: None,
            prefer: Some("return=representation"),
        };
        let ok = serde_json::json!({ "error": "Unprocessable Entity", "message": "bad content" });
        assert!(run("error_loose", Some(&ok), &representation).is_empty());

        // No body / a non-JSON body: the detail is a MAY, never a failure.
        assert!(run("error_loose", None, &representation).is_empty());
        let xml = Value::String("<error/>".to_owned());
        assert!(run("error_loose", Some(&xml), &representation).is_empty());

        let no_message = serde_json::json!({ "error": "Unprocessable Entity" });
        assert_eq!(
            run("error_loose", Some(&no_message), &representation).len(),
            1
        );
        let wrong_type = serde_json::json!({ "message": 42 });
        assert_eq!(
            run("error_loose", Some(&wrong_type), &representation).len(),
            1
        );
        let empty = serde_json::json!({ "message": "  " });
        assert_eq!(run("error_loose", Some(&empty), &representation).len(), 1);
    }

    #[test]
    fn result_set_body_requires_the_rows_array() {
        let ok = serde_json::json!({ "q": "SELECT e/ehr_id/value FROM EHR e", "rows": [] });
        assert!(run("result_set_body", Some(&ok), &ctx()).is_empty());

        let no_rows = serde_json::json!({ "q": "SELECT e/ehr_id/value FROM EHR e" });
        assert_eq!(run("result_set_body", Some(&no_rows), &ctx()).len(), 1);
        let wrong_type = serde_json::json!({ "rows": 3 });
        assert_eq!(run("result_set_body", Some(&wrong_type), &ctx()).len(), 1);
        let not_an_object = Value::String("<resultSet/>".to_owned());
        assert_eq!(
            run("result_set_body", Some(&not_an_object), &ctx()).len(),
            1
        );
        assert_eq!(run("result_set_body", None, &ctx()).len(), 1);
    }

    #[test]
    fn negotiated_compares_the_media_token() {
        let e = expectation("negotiated");
        let body = serde_json::json!({ "_type": "EHR" });
        let json = RequestContext {
            accept: Some("application/json"),
            prefer: None,
        };
        let ok = headers(&[("content-type", "application/json; charset=utf-8")]);
        assert!(evaluate(&e, Some(&body), &ok, &json).is_empty());

        let bad = headers(&[("content-type", "application/xml")]);
        assert_eq!(evaluate(&e, Some(&body), &bad, &json).len(), 1);
        // A response with content and no Content-Type violates the MUST.
        assert_eq!(evaluate(&e, Some(&body), &headers(&[]), &json).len(), 1);
    }

    #[test]
    fn negotiated_is_silent_without_accept_and_without_a_body() {
        let e = expectation("negotiated");
        let body = serde_json::json!({ "_type": "EHR" });
        // No Accept: the endpoint default was negotiated.
        let none = headers(&[("content-type", "application/xml")]);
        assert!(evaluate(&e, Some(&body), &none, &ctx()).is_empty());
        // No content body: the MUST is explicitly exempted (204).
        let json = RequestContext {
            accept: Some("application/json"),
            prefer: None,
        };
        assert!(evaluate(&e, None, &headers(&[]), &json).is_empty());
        // A wildcard range accepts every type.
        let any = RequestContext {
            accept: Some("*/*"),
            prefer: None,
        };
        assert!(evaluate(&e, Some(&body), &none, &any).is_empty());
        // A list passes on any offered range.
        let list = RequestContext {
            accept: Some("application/json, application/xml;q=0.9"),
            prefer: None,
        };
        assert!(evaluate(&e, Some(&body), &none, &list).is_empty());
    }

    #[test]
    fn prefer_conditional_representation_requires_a_body() {
        let representation = RequestContext {
            accept: None,
            prefer: Some("return=representation"),
        };
        let body = serde_json::json!({ "_type": "COMPOSITION" });
        assert!(run("prefer_conditional", Some(&body), &representation).is_empty());
        let failures = run("prefer_conditional", None, &representation);
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].contains("return=representation"));
        // The resolve_refs companion preference does not hide the branch.
        let with_refs = RequestContext {
            accept: None,
            prefer: Some("return=representation, resolve_refs"),
        };
        assert_eq!(run("prefer_conditional", None, &with_refs).len(), 1);
    }

    #[test]
    fn prefer_conditional_identifier_requires_uid() {
        let identifier = RequestContext {
            accept: None,
            prefer: Some("return=identifier"),
        };
        let ok = serde_json::json!({ "uid": "8849182c::openEHRSys.example.com::3" });
        assert!(run("prefer_conditional", Some(&ok), &identifier).is_empty());
        assert_eq!(run("prefer_conditional", None, &identifier).len(), 1);
        let full = serde_json::json!({ "_type": "COMPOSITION" });
        assert_eq!(run("prefer_conditional", Some(&full), &identifier).len(), 1);
        // A non-JSON identifier representation is the negotiated selector's
        // business, not this one's.
        let xml = Value::String("<uid>x::sys::1</uid>".to_owned());
        assert!(run("prefer_conditional", Some(&xml), &identifier).is_empty());
    }

    #[test]
    fn prefer_conditional_minimal_and_absent_never_fail() {
        let minimal = RequestContext {
            accept: None,
            prefer: Some("Return=Minimal"),
        };
        assert!(run("prefer_conditional", None, &minimal).is_empty());
        let body = serde_json::json!({ "_type": "EHR" });
        assert!(run("prefer_conditional", Some(&body), &minimal).is_empty());
        // No Prefer at all: the assumed default is return=minimal.
        assert!(run("prefer_conditional", None, &ctx()).is_empty());
        assert!(run("prefer_conditional", Some(&body), &ctx()).is_empty());
    }
}
