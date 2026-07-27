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
                // A 501-only operation (declared, not yet implemented — e.g.
                // the deferred ADL2 projections) documents its whole real
                // contract; a described 501 satisfies the rule.
                (code.starts_with('2') || code == "501")
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
        // Static/UI surface (config-gated at mount time, otherwise 200-only).
        "/ehrbase/rest/swagger-ui",
        // Probes and the status surface: 200-only by design (liveness never
        // errors at the application layer; readiness declares its 503).
        "/health",
        "/health/liveness",
        "/ehrbase/rest/status",
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
        // Overview §"If-Match and accidental overwrites": If-Match is required
        // only "when the preceding_version_uid is not part of the endpoint
        // path segment" — a DELETE addressing {uid_based_id} carries the
        // preceding version IN THE PATH (stale → 409 with the latest ETag per
        // the delete response YAMLs), so it is the spec's own exemption.
        let preceding_in_path = method == "delete" && path.ends_with("{uid_based_id}");
        let exempt = path.contains("item_tag")
            || path.contains("/tags")
            || path.contains("/admin/")
            || path.contains("/versioned_")
            || preceding_in_path;
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

/// The four EHR-resource operations are documented to full step-6
/// completeness: every real status branch, a `headers(...)` block on every
/// success response, the `Prefer`-conditional 201 body as a named example
/// pair, and a UUID-only `ehr_id`.
///
/// The branches are the released ITS-REST text, not the stalled OAS:
/// `Requests_and_responses.md` §"HTTP status codes" assigns `400` to
/// "malformed request syntax, syntactically invalid content", `409` to a
/// request that "might generate a duplicate or a conflict" and `422` to a
/// well-formed request "unable to be followed due to semantic errors";
/// `Resources.md` §"XML Format"/§"JSON Format"/§"Simplified Formats" make an
/// unprocessable request payload a `415` MUST and an unfulfillable `Accept` a
/// `406` MUST. `Requests_and_responses.md` §Location confines `Location` to
/// creation responses, so a read MUST NOT declare it.
#[tokio::test]
#[allow(clippy::too_many_lines)] // one regression test per audited group keeps the pins together
async fn ehr_resource_operations_are_fully_documented() {
    const EHR: &str = "/ehrbase/rest/openehr/v1/ehr";
    const EHR_BY_ID: &str = "/ehrbase/rest/openehr/v1/ehr/{ehr_id}";

    let doc = served_document().await;

    let codes = |op: &Value| -> Vec<String> {
        op["responses"]
            .as_object()
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default()
    };
    let header_names = |op: &Value, status: &str| -> Vec<String> {
        op["responses"][status]["headers"]
            .as_object()
            .map(|h| h.keys().cloned().collect())
            .unwrap_or_default()
    };
    let param_description = |op: &Value, name: &str| -> String {
        op["parameters"]
            .as_array()
            .and_then(|params| {
                params
                    .iter()
                    .find(|p| p["name"].as_str() == Some(name))
                    .and_then(|p| p["description"].as_str())
            })
            .unwrap_or_default()
            .to_owned()
    };
    // The media-type entry of a `content` map, whatever the negotiated type is
    // keyed as (these operations declare one canonical media type).
    let first_content = |holder: &Value| -> Value {
        holder["content"]
            .as_object()
            .and_then(|c| c.values().next())
            .cloned()
            .unwrap_or_default()
    };

    // ── The two creates: every branch, every committal header, both bodies ──
    for (path, method) in [(EHR, "post"), (EHR_BY_ID, "put")] {
        let op = doc["paths"][path][method].clone();
        assert!(op.is_object(), "{method} {path} must be documented");
        let present = codes(&op);
        for expected in ["201", "400", "406", "409", "415", "422"] {
            assert!(
                present.iter().any(|c| c == expected),
                "{method} {path} must document {expected}; has {present:?}"
            );
        }
        let headers = header_names(&op, "201");
        for expected in ["ETag", "Location", "Last-Modified", "Preference-Applied"] {
            assert!(
                headers.iter().any(|h| h == expected),
                "{method} {path} 201 must document the {expected} header; has {headers:?}"
            );
        }
        // The Prefer-conditional body: utoipa cannot express a
        // request-header-conditional schema, so the two non-empty variants are
        // named examples on the 201 (§"Prefer minimal, identifier or full
        // representation response").
        let examples = first_content(&op["responses"]["201"])["examples"].clone();
        assert_eq!(
            examples["representation"]["value"]["_type"], "EHR",
            "{method} {path} 201 must carry the `representation` RM EHR example: {examples}"
        );
        assert!(
            examples["identifier"]["value"]["uid"].is_string(),
            "{method} {path} 201 must carry the `identifier` single-uid example: {examples}"
        );
        // The request body shows a real EHR_STATUS, archetype_details included.
        let body = first_content(&op["requestBody"])["example"].clone();
        assert_eq!(body["_type"], "EHR_STATUS", "{method} {path} body example");
        assert_eq!(
            body["archetype_details"]["_type"], "ARCHETYPED",
            "{method} {path} body example must carry archetype_details"
        );
        // All three Prefer tokens are enumerated on the header parameter.
        let prefer = param_description(&op, "Prefer");
        for token in [
            "return=minimal",
            "return=identifier",
            "return=representation",
        ] {
            assert!(
                prefer.contains(token),
                "{method} {path} Prefer description must enumerate {token}: {prefer}"
            );
        }
    }

    // ── The two reads: 400/406 branches, ETag documented, Location absent ───
    for path in [EHR, EHR_BY_ID] {
        let op = doc["paths"][path]["get"].clone();
        assert!(op.is_object(), "GET {path} must be documented");
        let present = codes(&op);
        for expected in ["200", "400", "404", "406"] {
            assert!(
                present.iter().any(|c| c == expected),
                "GET {path} must document {expected}; has {present:?}"
            );
        }
        let headers = header_names(&op, "200");
        assert!(
            headers.iter().any(|h| h == "ETag"),
            "GET {path} 200 must document the weak ETag; has {headers:?}"
        );
        assert!(
            !headers.iter().any(|h| h == "Location"),
            "GET {path} 200 must NOT declare Location (§Location: creation only)"
        );
        assert_eq!(
            first_content(&op["responses"]["200"])["example"]["_type"],
            "EHR",
            "GET {path} 200 must carry the real served EHR example"
        );
    }

    // ── `ehr_id` is UUID-only on the create-with-id path ────────────────────
    let put = doc["paths"][EHR_BY_ID]["put"].clone();
    let ehr_id = param_description(&put, "ehr_id");
    assert!(
        ehr_id.contains("UUID"),
        "PUT {EHR_BY_ID} must type ehr_id as a UUID: {ehr_id}"
    );
    assert!(
        !ehr_id.contains("strongly recommended"),
        "PUT {EHR_BY_ID} must not claim non-UUID HIER_OBJECT_IDs are accepted: {ehr_id}"
    );
}

/// The System API's one operation appears in the served document, fully
/// described (#418): a closure route mounted outside `OpenApiRouter` still
/// belongs to the composed `OpenAPI` — the served document describes the
/// whole served surface (ITS-REST System API, `operations/options.yaml`).
#[tokio::test]
async fn system_options_operation_is_documented() {
    let doc = served_document().await;
    let op = doc["paths"]["/ehrbase/rest/openehr/v1"]["options"].clone();
    assert!(
        op.is_object(),
        "OPTIONS on the API base path must be documented"
    );
    assert_eq!(op["operationId"], "options");
    let ok = &op["responses"]["200"];
    assert!(
        ok["headers"]["Allow"].is_object(),
        "the 200 documents the Allow header (200_options.yaml): {op}"
    );
    assert!(
        op["responses"]["406"].is_object(),
        "the exclusively-XML Accept branch is documented"
    );
    // The Options schema rode along into components.
    assert!(
        doc["components"]["schemas"]["Options"].is_object(),
        "the Options manifest schema is registered"
    );
}

/// The Demographic API group's 26 released operations are documented to full
/// step-6 completeness: every served status branch, a `headers(...)` block on
/// every success response that emits headers, released-shaped examples, and no
/// `Location` on a read or a delete.
///
/// The branches are the RELEASED ITS-REST text, not the stalled OAS. The five
/// party CRUD quintets are byte-identical on the wire
/// (`specifications/operations/{person,agent,group,organisation,role}_*.yaml`
/// and their `$ref`d responses), so the loop below asserts each kind
/// individually — a quintet that drifts apart fails here.
/// `Requests_and_responses.md` §Location confines `Location` to creation
/// responses and §"Deprecated headers" deprecates it on `GET`/`DELETE`;
/// §"If-Match and accidental overwrites" gives the update's `412` and its
/// missing-header `400`; `Resources.md` §"XML Format"/§"JSON Format"/
/// §"Simplified Formats" make an unprocessable payload a `415` and an
/// unfulfillable `Accept` a `406`.
#[tokio::test]
#[allow(clippy::too_many_lines)] // one regression test per audited group keeps the pins together
async fn demographic_operations_are_fully_documented() {
    const BASE: &str = "/ehrbase/rest/openehr/v1/demographic";
    const KINDS: &[(&str, &str)] = &[
        ("person", "PERSON"),
        ("agent", "AGENT"),
        ("group", "GROUP"),
        ("organisation", "ORGANISATION"),
        ("role", "ROLE"),
    ];

    let doc = served_document().await;

    let codes = |op: &Value| -> Vec<String> {
        op["responses"]
            .as_object()
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default()
    };
    let header_names = |op: &Value, status: &str| -> Vec<String> {
        op["responses"][status]["headers"]
            .as_object()
            .map(|h| h.keys().cloned().collect())
            .unwrap_or_default()
    };
    let first_content = |holder: &Value| -> Value {
        holder["content"]
            .as_object()
            .and_then(|c| c.values().next())
            .cloned()
            .unwrap_or_default()
    };
    let require = |op: &Value, path: &str, method: &str, expected: &[&str]| {
        let present = codes(op);
        for want in expected {
            assert!(
                present.iter().any(|c| c == want),
                "{method} {path} must document {want}; has {present:?}"
            );
        }
    };

    for (segment, rm_type) in KINDS {
        // ── create: every branch, both Prefer bodies, the full header set ────
        let path = format!("{BASE}/{segment}");
        let op = doc["paths"][&path]["post"].clone();
        assert!(op.is_object(), "POST {path} must be documented");
        require(
            &op,
            &path,
            "POST",
            &["201", "400", "404", "406", "415", "422"],
        );
        let headers = header_names(&op, "201");
        for want in [
            "ETag",
            "Location",
            "Last-Modified",
            "Preference-Applied",
            "openehr-item-tag",
            "openehr-version-item-tag",
        ] {
            assert!(
                headers.iter().any(|h| h == want),
                "POST {path} 201 must document the {want} header; has {headers:?}"
            );
        }
        let examples = first_content(&op["responses"]["201"])["examples"].clone();
        assert_eq!(
            examples["representation"]["value"]["_type"], *rm_type,
            "POST {path} 201 must carry the `representation` RM {rm_type} example"
        );
        assert!(
            examples["identifier"]["value"]["uid"].is_string(),
            "POST {path} 201 must carry the `identifier` single-uid example"
        );
        let body = first_content(&op["requestBody"])["example"].clone();
        assert_eq!(body["_type"], *rm_type, "POST {path} request-body example");
        assert!(
            body["identities"].as_array().is_some_and(|i| !i.is_empty()),
            "POST {path} body example must satisfy PARTY.Identities_valid"
        );
        assert_eq!(
            body["name"]["value"], *rm_type,
            "POST {path} body example must satisfy PARTY.Type_valid (type = name)"
        );

        // ── get: the deleted-at-time 204, ETag + Last-Modified, no Location ──
        let path = format!("{BASE}/{segment}/{{uid_based_id}}");
        let op = doc["paths"][&path]["get"].clone();
        assert!(op.is_object(), "GET {path} must be documented");
        require(
            &op,
            &path,
            "GET",
            &["200", "204", "400", "404", "406", "415"],
        );
        let headers = header_names(&op, "200");
        for want in [
            "ETag",
            "Last-Modified",
            "openehr-item-tag",
            "openehr-version-item-tag",
        ] {
            assert!(
                headers.iter().any(|h| h == want),
                "GET {path} 200 must document the {want} header; has {headers:?}"
            );
        }
        assert!(
            !headers.iter().any(|h| h == "Location"),
            "GET {path} 200 must NOT declare Location (§Location: creation only)"
        );
        assert_eq!(
            first_content(&op["responses"]["200"])["example"]["_type"],
            *rm_type,
            "GET {path} 200 must carry the served RM {rm_type} example"
        );

        // ── update: If-Match + 412 with its ETag, both Prefer statuses ───────
        let op = doc["paths"][&path]["put"].clone();
        assert!(op.is_object(), "PUT {path} must be documented");
        require(
            &op,
            &path,
            "PUT",
            &["200", "204", "400", "404", "406", "412", "415", "422"],
        );
        assert!(
            documented_params(&op, "header")
                .iter()
                .any(|n| n.eq_ignore_ascii_case("if-match")),
            "PUT {path} must document the required If-Match precondition"
        );
        for status in ["200", "204"] {
            let headers = header_names(&op, status);
            for want in ["ETag", "Location", "Last-Modified", "Preference-Applied"] {
                assert!(
                    headers.iter().any(|h| h == want),
                    "PUT {path} {status} must document the {want} header; has {headers:?}"
                );
            }
        }
        let headers = header_names(&op, "412");
        assert!(
            headers.iter().any(|h| h == "ETag"),
            "PUT {path} 412 must echo the latest version_uid in ETag; has {headers:?}"
        );
        assert!(
            !headers.iter().any(|h| h == "Location"),
            "PUT {path} 412 must NOT declare Location (§Location: creation only)"
        );

        // ── delete: the deleted-version ETag, 409 with the latest ETag ───────
        let op = doc["paths"][&path]["delete"].clone();
        assert!(op.is_object(), "DELETE {path} must be documented");
        require(
            &op,
            &path,
            "DELETE",
            &["204", "400", "404", "406", "409", "415"],
        );
        let headers = header_names(&op, "204");
        assert!(
            headers.iter().any(|h| h == "ETag"),
            "DELETE {path} 204 must document the deleted version's ETag; has {headers:?}"
        );
        assert!(
            !headers.iter().any(|h| h == "Location"),
            "DELETE {path} 204 must NOT declare Location (§\"Deprecated headers\")"
        );
        assert!(
            header_names(&op, "409").iter().any(|h| h == "ETag"),
            "DELETE {path} 409 must echo the latest version_uid in ETag"
        );
    }

    // ── the four versioned_party reads: ETag, no Location, 400/404/406 ───────
    let versioned = [
        format!("{BASE}/versioned_party/{{versioned_object_uid}}"),
        format!("{BASE}/versioned_party/{{versioned_object_uid}}/revision_history"),
        format!("{BASE}/versioned_party/{{versioned_object_uid}}/version"),
        format!("{BASE}/versioned_party/{{versioned_object_uid}}/version/{{version_uid}}"),
    ];
    for path in &versioned {
        let op = doc["paths"][path]["get"].clone();
        assert!(op.is_object(), "GET {path} must be documented");
        require(&op, path, "GET", &["200", "400", "404", "406"]);
        let headers = header_names(&op, "200");
        assert!(
            headers.iter().any(|h| h == "ETag"),
            "GET {path} 200 must document the weak ETag; has {headers:?}"
        );
        assert!(
            !headers.iter().any(|h| h == "Location"),
            "GET {path} 200 must NOT declare Location (§Location: creation only)"
        );
    }
    // The container exposes no commit audit, the other three do.
    assert!(
        !header_names(&doc["paths"][&versioned[0]]["get"], "200")
            .iter()
            .any(|h| h == "Last-Modified"),
        "the VERSIONED_PARTY container has no commit audit to derive Last-Modified from"
    );
    for path in &versioned[1..] {
        assert!(
            header_names(&doc["paths"][path]["get"], "200")
                .iter()
                .any(|h| h == "Last-Modified"),
            "GET {path} 200 must document Last-Modified (§\"ETag and Last-Modified\")"
        );
    }

    // ── the demographic CONTRIBUTION pair ────────────────────────────────────
    let path = format!("{BASE}/contribution");
    let op = doc["paths"][&path]["post"].clone();
    assert!(op.is_object(), "POST {path} must be documented");
    require(&op, &path, "POST", &["201", "400", "406", "409", "415"]);
    let headers = header_names(&op, "201");
    for want in ["ETag", "Location", "Preference-Applied"] {
        assert!(
            headers.iter().any(|h| h == want),
            "POST {path} 201 must document the {want} header; has {headers:?}"
        );
    }
    let body = first_content(&op["requestBody"])["example"].clone();
    assert!(
        body["versions"].as_array().is_some_and(|v| !v.is_empty()) && body["audit"].is_object(),
        "POST {path} body example must be a NewContribution (versions + audit)"
    );

    let path = format!("{BASE}/contribution/{{contribution_uid}}");
    let op = doc["paths"][&path]["get"].clone();
    assert!(op.is_object(), "GET {path} must be documented");
    require(&op, &path, "GET", &["200", "400", "404", "406"]);
    assert!(
        !header_names(&op, "200").iter().any(|h| h == "Location"),
        "GET {path} 200 must NOT declare Location (§Location: creation only)"
    );
    assert_eq!(
        first_content(&op["responses"]["200"])["example"]["_type"],
        "CONTRIBUTION",
        "GET {path} 200 must carry the served CONTRIBUTION example"
    );
}

/// The eight `PARTY_RELATIONSHIP` operations are OUR OWN EXTENSION: the
/// released Demographic API defines no `party_relationship` path, so every one
/// of them must say so in its description and none may be counted towards a
/// conformance-profile claim.
#[tokio::test]
async fn party_relationship_operations_are_flagged_as_an_extension() {
    const BASE: &str = "/ehrbase/rest/openehr/v1/demographic";
    let doc = served_document().await;

    let mut seen = 0;
    for (path, method, op) in operations(&doc) {
        if !path.starts_with(&format!("{BASE}/party_relationship"))
            && !path.starts_with(&format!("{BASE}/versioned_party_relationship"))
        {
            continue;
        }
        seen += 1;
        let text = format!(
            "{} {}",
            op.get("summary")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            op.get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
        );
        assert!(
            text.contains("no ITS-REST operation governs this"),
            "{} {path} must carry the our-own-extension flag; has: {text}",
            method.to_uppercase()
        );
    }
    assert_eq!(
        seen, 8,
        "the PARTY_RELATIONSHIP extension serves eight operations"
    );
}
