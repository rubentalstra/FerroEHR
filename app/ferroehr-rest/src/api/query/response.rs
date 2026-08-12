// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Shared request-decoding + response-rendering for the QUERY API
//! (`docs/query/Request.md`, `docs/query/Response.md`).
//!
//! The ad-hoc ([`super::adhoc`]) and stored ([`super::stored`]) execution paths
//! both normalize the request (`ehr_id` scope, `offset`/`fetch` paging window,
//! `query_parameters` binds) and render the same `RESULT_SET` document
//! (`schemas/query/ResultSet.yaml`). The decode helpers (`AdhocQueryExecute` /
//! `Query` body, the `ehr_id`-from-query-or-header rule) and the `RESULT_SET`
//! renderer live here so both paths stay identical.
//!
//! Spec: `ehr_id` may arrive as the `ehr_id` query parameter OR the
//! `openehr-ehr-id` request header (`Request.md` §About the `ehr_id` parameter);
//! the `200 OK` response carries an `ETag` identifying the `RESULT_SET`
//! (`responses/200_Query.yaml` + `headers/ETag_RESULT_SET.yaml`).

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): the handler carries the canonical fragment the \
              negotiate seam produced once (stored-content serving / commit interior)"
)]

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use axum::response::Response;
use http::{HeaderMap, HeaderValue, StatusCode, header};
use uuid::Uuid;

use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::negotiate;
use crate::overview::error::RestError;
use ferroehr::service::query::request::AqlQueryRequest;

/// The ABAC pre-filter derived from the request (`extensions::abac::query_pre`):
/// the patient subject-scope id and the touched-attribute collection flag. Both
/// are our own access-control extension — no openEHR spec governs them — and are
/// applied uniformly to every normalized [`AqlQueryRequest`] before execution.
pub(super) struct QueryScope {
    /// The ABAC patient subject-scope id, if a scoped principal is configured.
    pub(super) subject_scope: Option<String>,
    /// Whether the executor must collect the touched EHR/template ids for the
    /// ABAC post-check.
    pub(super) collect: bool,
}

impl QueryScope {
    /// Stamp the ABAC scope + collection flag onto a normalized request.
    pub(super) fn apply(&self, mut request: AqlQueryRequest) -> AqlQueryRequest {
        request.subject_scope.clone_from(&self.subject_scope);
        request.collect_attributes = self.collect;
        request
    }
}

/// Decode a JSON request body into `T` (the `AdhocQueryExecute` / `Query`
/// schema). The QUERY operations declare `application/json` only
/// (`query-codegen.openapi.yaml`), so a non-JSON `Content-Type` is rejected by
/// [`negotiate::json_value`].
pub(super) fn decode_body<T: serde::de::DeserializeOwned>(
    h: &HeaderMap,
    body: &bytes::Bytes,
) -> Result<T, RestError> {
    let value = negotiate::json_value(h, body).map_err(RestError)?;
    serde_json::from_value(value).map_err(|e| {
        RestError(ApiError::BadRequest(format!(
            "invalid query request body: {e}"
        )))
    })
}

/// The canonical name of the `ehr_id` request header — `Request.md` §Common
/// Headers and Query Parameters spells it `openehr-ehr-id`. The pre-1.1.0
/// `openEHR-EHR-id` spelling is the DEPRECATED form of the same header
/// (`Requests_and_responses.md` §Deprecated headers: "the deprecated headers
/// remain available for backward compatibility"), and since HTTP field names
/// are case-insensitive (RFC 9110 §5.1) both spellings resolve through this one
/// lookup.
pub(super) const H_EHR_ID: &str = "openehr-ehr-id";

/// The single wire `ehr_id` scope of a query execution, resolved from the two
/// spec-sanctioned forms and applied identically by every execution operation,
/// `GET` and `POST` alike: the `ehr_id` query parameter (`query_ehr_id`, already
/// decoded by the caller) and the [`H_EHR_ID`] request header. `Request.md`
/// §About the `ehr_id` parameter: clients "MAY supply it as a query parameter
/// `ehr_id` or alternatively as a request header named `openehr-ehr-id`".
///
/// Returned as an [`Option`]; the caller collects it into the
/// [`AqlQueryRequest::ehr_ids`] vec (a single wire `ehr_id` is the one-element
/// case of the SM `List<UUID>` scope).
///
/// # Precedence when both forms are supplied
///
/// `Request.md` says "or alternatively" and never states what happens when a
/// request carries BOTH forms; the released text is silent, so the handling
/// below is our own:
///
/// - exactly one form supplied → that value is the scope;
/// - both supplied naming the SAME EHR → accepted (the request names one EHR,
///   so there is nothing to arbitrate);
/// - both supplied naming DIFFERENT EHRs → `400 Bad Request`. The request is
///   self-contradictory and no released rule picks a winner
///   (`Requests_and_responses.md` §HTTP status codes, row `400`: "the service
///   cannot or will not process the request due to something that is perceived
///   to be a client error"; the same section adds that `400` is "a generic
///   client-side error, used when no other `4xx` error code is appropriate").
///
/// The query parameter is therefore the primary form and the header the
/// alternative, but the two can never disagree on a request the service
/// executes.
///
/// # Errors
///
/// [`ApiError::BadRequest`] when the query parameter and the header name
/// different EHRs, or when repeated header field lines disagree.
pub(super) fn ehr_id_from_request(
    query_ehr_id: Option<String>,
    headers: &HeaderMap,
) -> Result<Option<String>, RestError> {
    let mut from_header: Option<&str> = None;
    for raw in headers.get_all(H_EHR_ID) {
        // An empty field value carries no EHR identifier — RFC 9110 §5.5 permits
        // empty field values — so it is "not supplied" rather than a scope that
        // conflicts with the query parameter.
        let Some(value) = raw.to_str().ok().map(str::trim).filter(|v| !v.is_empty()) else {
            continue;
        };
        match from_header {
            Some(seen) if seen != value => return Err(conflicting_ehr_id(seen, value)),
            _ => from_header = Some(value),
        }
    }
    match (query_ehr_id, from_header) {
        (Some(from_query), Some(header)) if from_query != header => {
            Err(conflicting_ehr_id(&from_query, header))
        }
        (Some(from_query), _) => Ok(Some(from_query)),
        (None, header) => Ok(header.map(str::to_owned)),
    }
}

/// The `400` for a request whose `ehr_id` query parameter and `openehr-ehr-id`
/// header name different EHRs (see [`ehr_id_from_request`] §Precedence).
fn conflicting_ehr_id(first: &str, second: &str) -> RestError {
    RestError(ApiError::BadRequest(format!(
        "conflicting EHR scope: the `ehr_id` query parameter and the `{H_EHR_ID}` \
         request header name different EHRs (`{first}` vs `{second}`)"
    )))
}

/// A required path segment (the generated `*BodyParams` for POST carry only the
/// name/version path parts; read them directly from the matched path).
pub(super) fn path_segment(parts: &RequestParts, key: &str) -> Result<String, RestError> {
    parts.path.get(key).cloned().ok_or_else(|| {
        RestError(ApiError::BadRequest(format!(
            "missing path parameter `{key}`"
        )))
    })
}

/// Render the assembled `RESULT_SET` as the `200 OK` response, emitting the
/// spec-mandated `ETag` header.
///
/// `200_Query.yaml` declares an `ETag` response header — "an identifier of the
/// `RESULT_SET`" — in the weak form (`headers/ETag_RESULT_SET.yaml`:
/// `W/"…"`). The vendored `ResultSet` schema carries no `id` field, so the tag
/// is a stable digest of the document's result-determining content, rendered as
/// a weak `ETag` ([`result_set_etag`]): identical result sets get identical tags,
/// which is exactly the `ETag` contract. The body itself is negotiated by
/// [`negotiate::respond`] (JSON only — the QUERY operations declare no
/// canonical-XML representation, so an XML `Accept` yields `406`, and the `ETag`
/// is set only on the success path).
pub(super) fn respond_result_set(headers: &HeaderMap, result_set: &serde_json::Value) -> Response {
    let mut resp = negotiate::respond(headers, StatusCode::OK, result_set);
    // Only a genuine 200 carries a RESULT_SET identifier; a negotiated 406 (XML
    // Accept) is an error body and gets no ETag.
    if resp.status().is_success()
        && let Ok(value) = HeaderValue::from_str(&result_set_etag(result_set))
    {
        resp.headers_mut().insert(header::ETAG, value);
    }
    resp
}

/// A weak `ETag` for a `RESULT_SET`: a deterministic 128-bit digest of the
/// document's RESULT-DETERMINING content, rendered as `W/"{uuid}"` (the shape
/// `headers/ETag_RESULT_SET.yaml` exemplifies).
///
/// The digest deliberately covers `name`, `q`, `meta._executed_aql`, `columns`
/// and `rows`, and NOT the response-stamped metadata (`meta._created`, and any
/// `_href`/`_generator` a response may carry): the overview requires an `ETag`
/// to identify the resource and to change only with it — "acts as a unique
/// identifier for a specific version of a resource. It helps clients determine
/// whether a resource has changed between requests, supporting efficient
/// caching" and "It changes as soon as the resource changes"
/// (`Requests_and_responses.md` §`ETag` and Last-Modified) — while `Request.md`
/// §Common Headers and Query Parameters names this one "A unique identifier of
/// the resultSet". A digest over `_created` (stamped per response) would mint a
/// fresh tag for an unchanged result set, which is the opposite of both
/// sentences; the executed AQL is included because two different queries may
/// coincidentally return the same rows and are not the same result set.
fn result_set_etag(result_set: &serde_json::Value) -> String {
    let identity = serde_json::json!({
        "name": result_set.get("name"),
        "q": result_set.get("q"),
        "executed_aql": result_set.pointer("/meta/_executed_aql"),
        "columns": result_set.get("columns"),
        "rows": result_set.get("rows"),
    });
    let bytes = serde_json::to_vec(&identity).unwrap_or_default();
    format!("W/\"{}\"", stable_uuid(&bytes))
}

/// A deterministic UUID from a byte digest — two fixed-seed [`DefaultHasher`]
/// passes (the second salted) fill the 128-bit value. `DefaultHasher::new`
/// uses fixed keys, so the mapping is stable for identical input.
fn stable_uuid(bytes: &[u8]) -> Uuid {
    let mut high = DefaultHasher::new();
    high.write(bytes);
    let mut low = DefaultHasher::new();
    low.write_u8(0x5b);
    low.write(bytes);
    Uuid::from_u64_pair(high.finish(), low.finish())
}

/// Merge a paging value carried in the POST body with the same-named URL
/// query parameter. The docs-text SHOULD-list ("All query execution requests
/// SHOULD support at least the following parameters", `Request.md` §Common
/// Headers and Query Parameters) draws no GET/POST distinction, so the URL
/// forms are accepted on the POSTs too; released text assigns no precedence,
/// so a value carried in BOTH places must agree — a conflict is a `400`, the
/// same rule the two `ehr_id` carriers follow.
pub(super) fn merge_body_and_url_i64(
    body: Option<i64>,
    query: Option<&str>,
    key: &str,
) -> Result<Option<i64>, RestError> {
    #[expect(
        clippy::map_err_ignore,
        reason = "`ParseIntError` adds only \"invalid digit\"/\"out of range\" to a 400 \
                  body that already names the parameter and echoes the rejected value"
    )]
    let url = crate::params::query_param(query, key)
        .map(|raw| {
            raw.parse::<i64>().map_err(|_| {
                RestError(ApiError::BadRequest(format!(
                    "the `{key}` query parameter is not an integer: {raw:?}"
                )))
            })
        })
        .transpose()?;
    match (body, url) {
        (Some(b), Some(u)) if b != u => Err(RestError(ApiError::BadRequest(format!(
            "`{key}` is {b} in the request body but {u} in the URL — a request may \
             carry the value in either place, not two disagreeing ones"
        )))),
        (b, u) => Ok(b.or(u)),
    }
}

/// Merge the body `query_parameters` object with the URL's named
/// `$parameter` binds (the same named-binding law the GETs follow,
/// `Request.md` §Query parameters). A parameter carried in BOTH places must
/// agree — a conflict is a `400`, the same rule the `ehr_id` carriers follow.
pub(super) fn merge_body_and_url_parameters(
    body: std::collections::BTreeMap<String, serde_json::Value>,
    query: Option<&str>,
) -> Result<std::collections::BTreeMap<String, serde_json::Value>, RestError> {
    let url_only = crate::params::named_query_parameters(
        query,
        std::collections::BTreeMap::new(),
        crate::params::QUERY_RESERVED_KEYS,
    );
    // A disagreement between the two carriers is loud, never silently won.
    for (key, url_value) in &url_only {
        if let Some(body_value) = body.get(key)
            && body_value != url_value
        {
            return Err(RestError(ApiError::BadRequest(format!(
                "query parameter `{key}` differs between the request body and the URL"
            ))));
        }
    }
    let mut merged = body;
    for (key, url_value) in url_only {
        merged.insert(key, url_value);
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    fn etag(resp: &Response) -> Option<String> {
        resp.headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    #[test]
    fn stable_uuid_is_deterministic_and_content_sensitive() {
        assert_eq!(stable_uuid(b"abc"), stable_uuid(b"abc"));
        assert_ne!(stable_uuid(b"abc"), stable_uuid(b"abd"));
    }

    #[test]
    fn result_set_etag_is_weak_and_stable() {
        let rs = serde_json::json!({"rows": [[1, 2], [3, 4]]});
        let tag = result_set_etag(&rs);
        // 200_Query.yaml / ETag_RESULT_SET.yaml: weak form `W/"…"`.
        assert!(tag.starts_with("W/\""), "weak ETag: {tag}");
        assert!(tag.ends_with('"'), "quoted: {tag}");
        // Stable for an identical RESULT_SET; different for a different one.
        assert_eq!(tag, result_set_etag(&rs));
        assert_ne!(tag, result_set_etag(&serde_json::json!({"rows": []})));
    }

    /// The tag identifies the RESULT SET, not the response instance: two
    /// executions of the same query over the same data differ only in the
    /// per-response `meta._created` stamp and MUST carry the same `ETag`
    /// ("It changes as soon as the resource changes",
    /// `Requests_and_responses.md` §`ETag` and Last-Modified) — while a genuine
    /// change in the executed AQL, the columns or the rows MUST change it
    /// ("A unique identifier of the resultSet", `Request.md` §Common Headers
    /// and Query Parameters).
    #[test]
    fn result_set_etag_ignores_the_per_response_created_stamp() {
        let execution = |created: &str, rows: serde_json::Value| {
            serde_json::json!({
                "meta": {
                    "_type": "RESULTSET",
                    "_schema_version": "1.0.0",
                    "_created": created,
                    "_executed_aql": "SELECT c/uid/value FROM COMPOSITION c",
                },
                "q": "SELECT c/uid/value FROM COMPOSITION c",
                "columns": [{ "name": "#0", "path": "/uid/value" }],
                "rows": rows,
            })
        };
        let rows = serde_json::json!([["8849182c-82ad-4088-a07f-48ead4180515::s::1"]]);
        assert_eq!(
            result_set_etag(&execution("2026-08-03T09:00:00Z", rows.clone())),
            result_set_etag(&execution("2026-08-03T11:30:42.117Z", rows.clone())),
            "re-executing the same query over the same data must yield the same tag"
        );
        assert_ne!(
            result_set_etag(&execution("2026-08-03T09:00:00Z", rows)),
            result_set_etag(&execution("2026-08-03T09:00:00Z", serde_json::json!([]))),
            "a different result set must yield a different tag"
        );
        // The executed AQL is part of the identity: identical rows from a
        // different query are not the same result set.
        let mut other_query = execution("2026-08-03T09:00:00Z", serde_json::json!([]));
        other_query["meta"]["_executed_aql"] =
            serde_json::json!("SELECT e/ehr_id/value FROM EHR e");
        assert_ne!(
            result_set_etag(&other_query),
            result_set_etag(&execution("2026-08-03T09:00:00Z", serde_json::json!([]))),
        );
    }

    #[test]
    fn respond_result_set_emits_etag_on_json_200() {
        // Empty Accept → JSON; the 200 carries the RESULT_SET ETag.
        let rs = serde_json::json!({"rows": []});
        let resp = respond_result_set(&HeaderMap::new(), &rs);
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(
            etag(&resp).is_some_and(|t| t.starts_with("W/\"")),
            "weak ETag present on 200"
        );
    }

    #[test]
    fn respond_result_set_no_etag_when_xml_only_accept() {
        // The query response has no canonical-XML shape → 406, and an error body
        // carries no RESULT_SET ETag.
        let rs = serde_json::json!({"rows": []});
        let resp = respond_result_set(&headers(&[("accept", "application/xml")]), &rs);
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
        assert!(etag(&resp).is_none());
    }

    #[test]
    fn ehr_id_from_either_form() {
        // Request.md §About the ehr_id parameter: the query parameter OR the
        // `openehr-ehr-id` header; either alone is the scope.
        let h = headers(&[(H_EHR_ID, "from-header")]);
        assert_eq!(
            ehr_id_from_request(None, &h).unwrap().as_deref(),
            Some("from-header"),
            "the header alone scopes the execution"
        );
        assert_eq!(
            ehr_id_from_request(Some("from-query".to_owned()), &HeaderMap::new())
                .unwrap()
                .as_deref(),
            Some("from-query"),
            "the query parameter alone scopes the execution"
        );
        assert_eq!(
            ehr_id_from_request(None, &HeaderMap::new()).unwrap(),
            None,
            "neither form supplied is an unscoped (population) query"
        );
    }

    #[test]
    fn ehr_id_header_name_is_case_insensitive() {
        // Requests_and_responses.md §Deprecated headers pairs `openEHR-EHR-id`
        // with `openehr-ehr-id`; RFC 9110 §5.1 makes field names
        // case-insensitive, so the deprecated spelling resolves identically.
        let h = headers(&[("openEHR-EHR-id", "from-header")]);
        assert_eq!(
            ehr_id_from_request(None, &h).unwrap().as_deref(),
            Some("from-header")
        );
    }

    #[test]
    fn ehr_id_both_forms_agreeing_is_accepted() {
        // Both forms naming the SAME EHR: the request names one EHR, so there
        // is nothing to arbitrate.
        let h = headers(&[(H_EHR_ID, "same-ehr")]);
        assert_eq!(
            ehr_id_from_request(Some("same-ehr".to_owned()), &h)
                .unwrap()
                .as_deref(),
            Some("same-ehr")
        );
    }

    #[test]
    fn ehr_id_conflicting_forms_are_bad_request() {
        // Both forms naming DIFFERENT EHRs is self-contradictory → 400
        // (Requests_and_responses.md §HTTP status codes, row 400); no released
        // text assigns a precedence, so this handling is our own.
        let h = headers(&[(H_EHR_ID, "from-header")]);
        let err = ehr_id_from_request(Some("from-query".to_owned()), &h).unwrap_err();
        assert!(matches!(err.0, ApiError::BadRequest(_)), "got {:?}", err.0);
    }

    #[test]
    fn ehr_id_repeated_headers_must_agree() {
        // Two field lines of the same header naming different EHRs is the same
        // self-contradiction; agreeing lines resolve to the one value.
        let mut conflicting = HeaderMap::new();
        conflicting.append(H_EHR_ID, HeaderValue::from_static("a"));
        conflicting.append(H_EHR_ID, HeaderValue::from_static("b"));
        assert!(matches!(
            ehr_id_from_request(None, &conflicting).unwrap_err().0,
            ApiError::BadRequest(_)
        ));

        let mut agreeing = HeaderMap::new();
        agreeing.append(H_EHR_ID, HeaderValue::from_static("a"));
        agreeing.append(H_EHR_ID, HeaderValue::from_static("a"));
        assert_eq!(
            ehr_id_from_request(None, &agreeing).unwrap().as_deref(),
            Some("a")
        );
    }

    #[test]
    fn ehr_id_empty_header_value_is_not_supplied() {
        // RFC 9110 §5.5 permits empty field values; an empty `openehr-ehr-id`
        // carries no EHR identifier, so it neither scopes nor conflicts.
        let h = headers(&[(H_EHR_ID, "")]);
        assert_eq!(ehr_id_from_request(None, &h).unwrap(), None);
        assert_eq!(
            ehr_id_from_request(Some("from-query".to_owned()), &h)
                .unwrap()
                .as_deref(),
            Some("from-query")
        );
    }

    #[test]
    fn decode_body_parses_json_and_rejects_xml() {
        let json_h = headers(&[("content-type", "application/json")]);
        let value: serde_json::Value =
            decode_body(&json_h, &Bytes::from_static(br#"{"offset":0}"#)).unwrap();
        assert_eq!(value["offset"], 0);

        let xml_h = headers(&[("content-type", "application/xml")]);
        let err =
            decode_body::<serde_json::Value>(&xml_h, &Bytes::from_static(b"<q/>")).unwrap_err();
        assert!(
            matches!(err.0, ApiError::UnsupportedMediaType(_)),
            "got {:?}",
            err.0
        );
    }

    #[test]
    fn query_scope_apply_stamps_subject_and_collect() {
        let scope = QueryScope {
            subject_scope: Some("patient-1".to_owned()),
            collect: true,
        };
        let req = scope.apply(AqlQueryRequest::default());
        assert_eq!(req.subject_scope.as_deref(), Some("patient-1"));
        assert!(req.collect_attributes);
    }
}

// ── Query error-status mapping (the QUERY responses enumerate 400/404/408) ─────
//
// The status codes are assembled platform-side as typed `SmError`s and turned
// into the ITS-REST status by `crate::overview::error::sm_api_error`; this
// renderer emits the success `RESULT_SET` (and its `meta`, including the
// parameter-substituted `_executed_aql`) verbatim. `ehr_id_does_not_exist` →
// `404` for a well-formed-but-absent EHR id, while a malformed UUID stays a
// `400` (`Request.md` §About the ehr_id parameter).
// NOTE: a query-execution timeout is `408` (`responses/408_Query.yaml`),
// bounded by the `FERROEHR__QUERY__TIMEOUT_MS` budget; unset, only the blunt
// global `TimeoutLayer` applies.
