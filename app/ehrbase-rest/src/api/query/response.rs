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

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use axum::response::Response;
use http::{HeaderMap, HeaderValue, StatusCode, header};
use uuid::Uuid;

use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::negotiate;
use crate::overview::error::RestError;
use ehrbase::service::query::request::AqlQueryRequest;

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
/// request carries BOTH forms; the released text is silent (ambiguity register
/// `AMB-59`). Fixed handling:
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
/// is derived deterministically from the assembled document (a stable content
/// digest rendered as a weak ETag): identical result sets get identical tags,
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

/// A weak `ETag` for a `RESULT_SET`: a deterministic 128-bit content digest of
/// the assembled document rendered as `W/"{uuid}"` (the shape
/// `headers/ETag_RESULT_SET.yaml` exemplifies). Deterministic so that
/// re-executing the same query over the same data yields the same tag.
fn result_set_etag(result_set: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(result_set).unwrap_or_default();
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

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        // is nothing to arbitrate (register AMB-59).
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
        // (Requests_and_responses.md §HTTP status codes, row 400). Register
        // AMB-59 records the spec silence this handling settles.
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
// The status codes for a query are assembled platform-side (`app/ehrbase`,
// `service/aql_query.rs`) as typed `SmError`s and turned into the ITS-REST
// status by `crate::overview::error::sm_api_error`; this renderer emits the
// success `RESULT_SET` (and its `meta`) verbatim. The three query codes:
//
// - `ehr_id_does_not_exist` → `404`: a scoped query probes existence
//   (`aql_query::resolve_ehr_ids`) and raises `SmError::ehr_not_found`
//   (`CallStatusType::EhrIdDoesNotExist` → `ApiError::NotFound`) for a
//   well-formed-but-absent EHR id; a malformed UUID stays a `400`
//   (`SmError::precondition`). (`Request.md` §About the ehr_id parameter.)
//
// - query-execution timeout → `408` (`responses/408_Query.yaml`): the executor
//   bounds the DB execution by the `EHRBASE__QUERY__TIMEOUT_MS` budget and, on
//   overrun, raises the timeout-tagged `SmError` that `sm_api_error`/
//   `RestError::into_response` render as `408 Request Timeout`
//   (`Requests_and_responses.md` §HTTP status codes, row `408`). With the budget
//   unset, an over-long query trips only the blunt global `TimeoutLayer`.
//
// - `meta._executed_aql` (the parameter-substituted query text) is assembled by
//   `aql_query::substitute_params` into the `RESULT_SET.meta`; this renderer
//   emits it as-is.
