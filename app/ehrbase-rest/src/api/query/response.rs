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
//! `openEHR-EHR-id` request header (`Request.md` §About the `ehr_id` parameter);
//! the `200 OK` response carries an `ETag` identifying the `RESULT_SET`
//! (`responses/200_Query.yaml` + `headers/ETag_RESULT_SET.yaml`).

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use axum::response::Response;
use http::{HeaderMap, HeaderValue, StatusCode, header};
use uuid::Uuid;

use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::{negotiate, params};
use ehrbase_sm::AqlQueryRequest;

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

/// The `ehr_id` scope from the `ehr_id` query parameter or the `openEHR-EHR-id`
/// request header (`Request.md` §About the `ehr_id` parameter: either form is
/// accepted). Returned as an [`Option`]; the caller collects it into the
/// [`AqlQueryRequest::ehr_ids`] vec (a single wire `ehr_id` is the one-element
/// case of the SM `List<UUID>` scope).
pub(super) fn ehr_id_from_request(q: Option<&str>, h: &HeaderMap) -> Option<String> {
    params::query_param(q, "ehr_id").or_else(|| {
        h.get("openEHR-EHR-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    })
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
/// spec-mandated `ETag` header (G-1).
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

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
        // Empty Accept → JSON; G-1: the 200 carries the RESULT_SET ETag.
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
    fn ehr_id_prefers_query_param_then_header() {
        // Request.md §About the ehr_id parameter: query parameter OR header.
        let h = headers(&[("openEHR-EHR-id", "from-header")]);
        assert_eq!(
            ehr_id_from_request(Some("ehr_id=from-query"), &h).as_deref(),
            Some("from-query"),
            "query parameter wins"
        );
        assert_eq!(
            ehr_id_from_request(None, &h).as_deref(),
            Some("from-header"),
            "header is the fallback"
        );
        assert_eq!(ehr_id_from_request(None, &HeaderMap::new()), None);
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
