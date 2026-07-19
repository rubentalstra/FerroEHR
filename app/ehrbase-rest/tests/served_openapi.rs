#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! Structural completeness gate over the SERVED `OpenAPI` document — the only
//! `OpenAPI` we publish (owner hard rule: serve only what we generate). The
//! rules encode the ITS-REST conventions every declaration must document
//! (`docs/specs/openehr/ITS-REST/specifications/docs/overview/
//! Requests_and_responses.md`: §"If-Match and accidental overwrites" — a
//! mismatch MUST be `412`; §Prefer; plus plain `OpenAPI` hygiene: every path
//! template parameter documented, every operation described, error outcomes
//! never omitted). A new endpoint that ships with a skeleton declaration
//! fails here — the completeness ratchet.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;

mod common;

/// HTTP methods an `OpenAPI` path item may carry.
const METHODS: &[&str] = &[
    "get", "put", "post", "delete", "patch", "head", "options", "trace",
];

/// Fetch the full served document through the real router.
async fn served_document() -> Value {
    use axum::body::Body;
    use http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let (_pg, app) = common::test_router().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/ehrbase/rest/api-docs/openapi.json")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("openapi.json response");
    assert_eq!(resp.status(), http::StatusCode::OK);
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    serde_json::from_slice(&bytes).expect("served openapi json")
}

/// The `{param}` names in a path template.
fn template_params(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        out.push(rest[open + 1..open + close].to_owned());
        rest = &rest[open + close + 1..];
    }
    out
}

/// The documented parameter names of an operation, by location.
fn documented_params(op: &Value, location: &str) -> Vec<String> {
    op.get("parameters")
        .and_then(Value::as_array)
        .map(|params| {
            params
                .iter()
                .filter(|p| p.get("in").and_then(Value::as_str) == Some(location))
                .filter_map(|p| p.get("name").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Iterate `(path, method, operation)` over the whole document.
fn operations(doc: &Value) -> Vec<(String, String, Value)> {
    let mut out = Vec::new();
    if let Some(paths) = doc.get("paths").and_then(Value::as_object) {
        for (path, item) in paths {
            if let Some(item) = item.as_object() {
                for (method, op) in item {
                    if METHODS.contains(&method.as_str()) {
                        out.push((path.clone(), method.clone(), op.clone()));
                    }
                }
            }
        }
    }
    out
}

/// Rule 1: every `{param}` in a path template is a documented `Path`
/// parameter on every operation of that path.
#[tokio::test]
async fn every_path_template_parameter_is_documented() {
    let doc = served_document().await;
    let mut missing = Vec::new();
    for (path, method, op) in operations(&doc) {
        let documented = documented_params(&op, "path");
        for template in template_params(&path) {
            if !documented.contains(&template) {
                missing.push(format!("{} {path}: {{{template}}}", method.to_uppercase()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "path template parameters missing a documented Path parameter:\n{}",
        missing.join("\n")
    );
}

/// Rule 2: every operation carries a non-empty summary or description, and
/// at least one success (2xx) response with a non-empty description.
#[tokio::test]
async fn every_operation_is_described_with_a_success_outcome() {
    let doc = served_document().await;
    let mut findings = Vec::new();
    for (path, method, op) in operations(&doc) {
        let described = ["summary", "description"].iter().any(|k| {
            op.get(*k)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty())
        });
        if !described {
            findings.push(format!(
                "{} {path}: no summary/description",
                method.to_uppercase()
            ));
        }
        let responses = op.get("responses").and_then(Value::as_object);
        let has_success = responses.is_some_and(|r| {
            r.iter().any(|(code, body)| {
                code.starts_with('2')
                    && body
                        .get("description")
                        .and_then(Value::as_str)
                        .is_some_and(|d| !d.trim().is_empty())
            })
        });
        if !has_success {
            findings.push(format!(
                "{} {path}: no described 2xx response",
                method.to_uppercase()
            ));
        }
    }
    assert!(
        findings.is_empty(),
        "underdocumented operations:\n{}",
        findings.join("\n")
    );
}

/// Rule 3: every operation documents at least one error (4xx/5xx) outcome —
/// except the discovery/document endpoints that genuinely cannot fail at
/// the application layer (each exemption is deliberate and listed).
#[tokio::test]
async fn every_operation_documents_an_error_outcome() {
    // Discovery documents and static surfaces: a bare 200 is their whole
    // contract (no parameters, no state) — exempt by explicit decision.
    const EXEMPT: &[&str] = &[
        "/ehrbase/rest/api-docs/openapi.json",
        "/ehrbase/rest/.well-known/smart-configuration",
        "/ehrbase/rest/openehr/v1/definition/openapi.json",
    ];
    let doc = served_document().await;
    let mut findings = Vec::new();
    for (path, method, op) in operations(&doc) {
        if EXEMPT.contains(&path.as_str()) || path.starts_with("/ehrbase/rest/api-docs/") {
            continue;
        }
        let has_error = op
            .get("responses")
            .and_then(Value::as_object)
            .is_some_and(|r| {
                r.keys()
                    .any(|code| code.starts_with('4') || code.starts_with('5'))
            });
        if !has_error {
            findings.push(format!("{} {path}", method.to_uppercase()));
        }
    }
    assert!(
        findings.is_empty(),
        "operations documenting no error outcome:\n{}",
        findings.join("\n")
    );
}

/// Rule 4: every PUT/DELETE on a change-controlled openEHR resource
/// documents the `If-Match` precondition AND its `412` outcome (overview
/// §"If-Match and accidental overwrites"). The exemptions are the surfaces
/// whose spec genuinely has no `If-Match` (item tags carry no version;
/// admin/extension deletes are not optimistic-concurrency controlled).
#[tokio::test]
async fn versioned_writes_document_if_match_and_412() {
    let doc = served_document().await;
    let mut findings = Vec::new();
    for (path, method, op) in operations(&doc) {
        if method != "put" && method != "delete" {
            continue;
        }
        let versioned = path.starts_with("/ehrbase/rest/openehr/v1/ehr/")
            && (path.ends_with("/directory")
                || path.ends_with("/ehr_status")
                || path.contains("/composition/"))
            || path.starts_with("/ehrbase/rest/openehr/v1/demographic/") && !path.contains("_tags");
        let exempt = path.contains("item_tag")
            || path.contains("/tags")
            || path.contains("/admin/")
            || path.contains("/versioned_");
        if !versioned || exempt {
            continue;
        }
        let has_if_match = documented_params(&op, "header")
            .iter()
            .any(|n| n.eq_ignore_ascii_case("if-match"));
        let has_412 = op
            .get("responses")
            .and_then(Value::as_object)
            .is_some_and(|r| r.contains_key("412"));
        if !has_if_match || !has_412 {
            findings.push(format!(
                "{} {path}: if-match={has_if_match} 412={has_412}",
                method.to_uppercase()
            ));
        }
    }
    assert!(
        findings.is_empty(),
        "versioned writes missing If-Match/412 documentation (overview §If-Match):\n{}",
        findings.join("\n")
    );
}
