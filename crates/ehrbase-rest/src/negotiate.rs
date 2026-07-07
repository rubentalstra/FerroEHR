//! Content negotiation between canonical JSON and canonical XML (ITS-REST).
//!
//! Request bodies and responses are negotiated via `openehr-its`
//! (`to_canonical_json`/`from_canonical_json`, `to_canonical_xml`/
//! `from_canonical_xml`). The generated server traits exchange
//! `serde_json::Value` at the boundary, so:
//!
//! - **JSON** is wired end to end for every operation (request and response).
//! - **XML request bodies** are decoded for the RM-typed write paths: the bytes
//!   are parsed into the concrete `openehr-rm` type and re-emitted as the
//!   canonical JSON `Value` the trait expects, so a handler never sees the wire
//!   format. See [`rm_value`].
//! - **XML responses** for the single spec-typed RM objects (composition,
//!   `ehr_status`, ehr, folder) are served by [`respond_rm`]: the handler returns
//!   canonical JSON as usual, and for an XML `Accept` the value is re-typed into
//!   its concrete `openehr-rm` type at the response edge so the generated
//!   `ToXml` runs — the mirror of the [`rm_value`] request path. Responses that
//!   are not a single spec-typed RM value (VERSION-family wrappers, revision
//!   histories, collections, item tags, contribution DTOs) have no spec-defined
//!   canonical-XML shape and stay JSON-only via [`respond`].

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode, header};
use serde::Serialize;
use serde::de::DeserializeOwned;

use openehr_its::rest::runtime::ApiError;
use openehr_its::xml::{FromXml, ToXml};

use crate::response::{ResourceMeta, ServiceResponse};

/// A negotiated wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Json,
    Xml,
}

const APPLICATION_JSON: &str = "application/json";
const APPLICATION_XML: &str = "application/xml";
/// Better `web-template` JSON media type (interop format, `openehr-flat`).
const APPLICATION_WT_JSON: &str = "application/openehr.wt+json";
/// Better `web-template` FLAT (simSDT) JSON media type (`openehr-flat`).
const APPLICATION_WT_FLAT_JSON: &str = "application/openehr.wt.flat+json";
/// Better `web-template` STRUCTURED (structSDT) JSON media type (`openehr-flat`).
const APPLICATION_WT_STRUCTURED_JSON: &str = "application/openehr.wt.structured+json";

/// Whether the client explicitly asks for the Better `web-template` JSON format
/// on `Accept` (`application/openehr.wt+json`).
pub(crate) fn wants_web_template(headers: &HeaderMap) -> bool {
    header_str(headers, header::ACCEPT).is_some_and(|accept| {
        accept
            .split(',')
            .any(|r| r.trim().starts_with(APPLICATION_WT_JSON))
    })
}

/// Whether the client asks for the FLAT (simSDT) format on `Accept`
/// (`application/openehr.wt.flat+json`).
pub(crate) fn wants_flat(headers: &HeaderMap) -> bool {
    header_str(headers, header::ACCEPT).is_some_and(|accept| {
        accept
            .split(',')
            .any(|r| r.trim().starts_with(APPLICATION_WT_FLAT_JSON))
    })
}

/// Whether the request body is a FLAT (simSDT) composition
/// (`Content-Type: application/openehr.wt.flat+json`).
pub(crate) fn is_flat_body(headers: &HeaderMap) -> bool {
    header_str(headers, header::CONTENT_TYPE)
        .is_some_and(|ct| ct.trim().starts_with(APPLICATION_WT_FLAT_JSON))
}

/// Whether the client asks for the STRUCTURED (structSDT) format on `Accept`
/// (`application/openehr.wt.structured+json`).
pub(crate) fn wants_structured(headers: &HeaderMap) -> bool {
    header_str(headers, header::ACCEPT).is_some_and(|accept| {
        accept
            .split(',')
            .any(|r| r.trim().starts_with(APPLICATION_WT_STRUCTURED_JSON))
    })
}

/// Whether the request body is a STRUCTURED (structSDT) composition
/// (`Content-Type: application/openehr.wt.structured+json`).
pub(crate) fn is_structured_body(headers: &HeaderMap) -> bool {
    header_str(headers, header::CONTENT_TYPE)
        .is_some_and(|ct| ct.trim().starts_with(APPLICATION_WT_STRUCTURED_JSON))
}

/// Serve a pre-serialized STRUCTURED (structSDT) composition as
/// `application/openehr.wt.structured+json`.
pub(crate) fn structured_json_body(status: StatusCode, json: String) -> Response {
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_WT_STRUCTURED_JSON),
    );
    resp
}

/// Serve a pre-serialized `WebTemplate` JSON document as
/// `application/openehr.wt+json`.
pub(crate) fn wt_json_body(status: StatusCode, json: String) -> Response {
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_WT_JSON),
    );
    resp
}

/// Serve a pre-serialized FLAT (simSDT) composition as
/// `application/openehr.wt.flat+json`.
pub(crate) fn flat_json_body(status: StatusCode, json: String) -> Response {
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_WT_FLAT_JSON),
    );
    resp
}

fn is_json(media: &str) -> bool {
    let m = media.trim();
    m.starts_with("application/json") || m.ends_with("+json")
}

fn is_xml(media: &str) -> bool {
    let m = media.trim();
    m.starts_with("application/xml") || m.starts_with("text/xml") || m.ends_with("+xml")
}

/// The format the client's `Content-Type` declares (defaults to JSON).
fn request_format(headers: &HeaderMap) -> Format {
    match header_str(headers, header::CONTENT_TYPE) {
        Some(ct) if is_xml(&ct) => Format::Xml,
        _ => Format::Json,
    }
}

/// The format to render the response in, honouring `Accept`. JSON is preferred
/// when the client accepts both or anything (`*/*`); XML only when explicitly
/// and exclusively requested.
pub(crate) fn response_format(headers: &HeaderMap) -> Format {
    let Some(accept) = header_str(headers, header::ACCEPT) else {
        return Format::Json;
    };
    let ranges: Vec<&str> = accept.split(',').collect();
    let accepts_json = ranges
        .iter()
        .any(|r| is_json(r) || r.trim().starts_with("*/*"));
    let accepts_xml = ranges.iter().any(|r| is_xml(r));
    if accepts_json || !accepts_xml {
        Format::Json
    } else {
        Format::Xml
    }
}

fn header_str(headers: &HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Decode a required JSON body into a `serde_json::Value`.
///
/// # Errors
/// [`ApiError::UnsupportedMediaType`] if the `Content-Type` is not JSON;
/// [`ApiError::BadRequest`] if the bytes are not valid JSON.
pub(crate) fn json_value(headers: &HeaderMap, body: &Bytes) -> Result<serde_json::Value, ApiError> {
    require_json(headers)?;
    parse_json(body)
}

/// Decode a required JSON array body (e.g. an item-tag list).
pub(crate) fn json_vec(
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Vec<serde_json::Value>, ApiError> {
    require_json(headers)?;
    serde_json::from_slice(body)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON array body: {e}")))
}

/// Decode a plain-text body (e.g. a stored-query YAML document).
pub(crate) fn text_body(body: &Bytes) -> Result<String, ApiError> {
    String::from_utf8(body.to_vec())
        .map_err(|e| ApiError::BadRequest(format!("body is not UTF-8: {e}")))
}

/// Decode a body the contract types as `Value` but which may arrive as another
/// text format (e.g. an ADL/OPT XML template upload): parsed as JSON when it is
/// JSON, otherwise wrapped as a JSON string so the (untyped) handler still
/// receives the bytes. The DEFINITION service then parses the OPT 1.4 XML into
/// `openehr_its::opt14` (P13 template ingestion), so no template-model parsing
/// belongs here — this is only the transport decode.
pub(crate) fn lenient_value(body: &Bytes) -> Result<serde_json::Value, ApiError> {
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) {
        return Ok(v);
    }
    Ok(serde_json::Value::String(text_body(body)?))
}

/// Decode an RM-typed body from either JSON or XML into the canonical JSON
/// `Value` the trait expects. `T` is the concrete `openehr-rm` payload type for
/// the operation (e.g. `Composition`).
///
/// # Errors
/// [`ApiError::BadRequest`] if the body cannot be parsed in the declared format;
/// [`ApiError::UnsupportedMediaType`] for a content type that is neither JSON nor XML.
pub(crate) fn rm_value<T>(headers: &HeaderMap, body: &Bytes) -> Result<serde_json::Value, ApiError>
where
    T: FromXml + Serialize + DeserializeOwned,
{
    match request_format(headers) {
        Format::Json => parse_json(body),
        Format::Xml => {
            let xml = text_body(body)?;
            let value: T = openehr_its::xml::from_canonical_xml(&xml)
                .map_err(|e| ApiError::BadRequest(format!("invalid canonical XML body: {e}")))?;
            serde_json::to_value(&value).map_err(|e| {
                ApiError::Internal(format!("re-encoding XML body to JSON failed: {e}"))
            })
        }
    }
}

/// Optional RM-typed body (empty → `None`).
pub(crate) fn optional_rm_value<T>(
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Option<serde_json::Value>, ApiError>
where
    T: FromXml + Serialize + DeserializeOwned,
{
    if body.is_empty() {
        return Ok(None);
    }
    Ok(Some(rm_value::<T>(headers, body)?))
}

fn require_json(headers: &HeaderMap) -> Result<(), ApiError> {
    match header_str(headers, header::CONTENT_TYPE) {
        // Absent or JSON content types are accepted; XML is not for these ops.
        None => Ok(()),
        Some(ct) if is_json(&ct) => Ok(()),
        Some(ct) if is_xml(&ct) => Err(ApiError::UnsupportedMediaType(format!(
            "operation accepts application/json only, got {ct}"
        ))),
        Some(ct) => Err(ApiError::UnsupportedMediaType(format!(
            "unsupported Content-Type: {ct}"
        ))),
    }
}

fn parse_json(body: &Bytes) -> Result<serde_json::Value, ApiError> {
    serde_json::from_slice(body)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON body: {e}")))
}

/// Render a serializable payload as a JSON response. Used for responses that are
/// not a single spec-typed RM value (`serde_json::Value` wrappers and
/// collections: VERSION-family objects, revision histories, item tags,
/// contribution DTOs); if the client requested XML exclusively, this returns 406
/// since those payloads have no spec-defined canonical-XML shape. Single
/// spec-typed RM objects use [`respond_rm`] instead.
pub(crate) fn respond<T: Serialize>(
    headers: &HeaderMap,
    status: StatusCode,
    value: &T,
) -> Response {
    match response_format(headers) {
        Format::Json => json_response(status, value),
        Format::Xml => ApiError::NotAcceptable(
            "canonical XML for this response is available once typed payloads land (P12); \
             request application/json"
                .to_owned(),
        )
        .into_response_body(),
    }
}

/// Render a canonical-JSON `Value` that IS a single spec-typed RM object,
/// honouring `Accept` for JSON or canonical XML. `T` is the concrete
/// `openehr-rm` type the value encodes (e.g. [`openehr_rm::prelude::Composition`]);
/// `root_tag` is the XML root element name. For XML the value is re-typed into
/// `T` so the generated `ToXml` runs — the mirror of how [`rm_value`] re-types
/// request bodies. A JSON `null` value (e.g. a future minimal-return create with
/// no body) renders as a bodyless response in either format.
pub(crate) fn respond_rm<T>(
    headers: &HeaderMap,
    status: StatusCode,
    value: &serde_json::Value,
    root_tag: &str,
) -> Response
where
    T: DeserializeOwned + Serialize + ToXml,
{
    match response_format(headers) {
        Format::Json => json_response(status, value),
        Format::Xml => {
            if value.is_null() {
                return empty(status);
            }
            let typed: T = match serde_json::from_value(value.clone()) {
                Ok(t) => t,
                Err(e) => {
                    return ApiError::Internal(format!(
                        "re-typing canonical JSON to <{root_tag}> for the XML response failed: {e}"
                    ))
                    .into_response_body();
                }
            };
            match openehr_its::xml::to_canonical_xml(&typed, root_tag) {
                Ok(xml) => {
                    let mut resp = (status, xml).into_response();
                    resp.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static(APPLICATION_XML),
                    );
                    resp
                }
                Err(e) => ApiError::Internal(format!("XML serialization failed: {e}"))
                    .into_response_body(),
            }
        }
    }
}

/// A bodyless success response (204/200 for deletes).
pub(crate) fn empty(status: StatusCode) -> Response {
    status.into_response()
}

/// A bodyless success response carrying a `Location` header — the ITS-REST
/// shape for a resource store/create whose body is empty (e.g. a stored-query
/// store: `200 OK` + `Location: …/definition/query/{name}/{version}`).
pub(crate) fn empty_with_location(status: StatusCode, location: &str) -> Response {
    let mut resp = status.into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        resp.headers_mut().insert(header::LOCATION, value);
    }
    resp
}

// ── ITS-REST response-header + `Prefer` handling (W2-A) ─────────────────────
//
// The header-bearing EHR operations carry a [`ServiceResponse`] (RM payload +
// typed [`ResourceMeta`]) out of the service seam; these helpers turn that into
// a negotiated response with the spec-mandated `ETag`/`Location` headers and the
// `Prefer` `return=minimal` (default) vs `return=representation` body policy.

/// Whether the client asked for the full representation on `Prefer`
/// (`return=representation`). The ITS-REST default is `return=minimal`
/// (`parameters/header/Prefer.yaml`) — a header-only response.
pub(crate) fn prefers_representation(headers: &HeaderMap) -> bool {
    headers
        .get("prefer")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|p| {
            p.split(',')
                .any(|t| t.trim().eq_ignore_ascii_case("return=representation"))
        })
}

/// Build the (path-absolute) `Location` URL for an EHR sub-resource under the
/// configured base path (`headers/Location_*.yaml`). `segment` is the resource
/// collection (`composition`/`ehr_status`/`directory`/`contribution`); `None`
/// targets the EHR resource itself (`/ehr/{ehr_id}`).
pub(crate) fn location(base_path: &str, ehr_id: &str, segment: Option<&str>, uid: &str) -> String {
    match segment {
        Some(seg) => format!("{base_path}/ehr/{ehr_id}/{seg}/{uid}"),
        None => format!("{base_path}/ehr/{ehr_id}"),
    }
}

/// Set `ETag` (the `uid`, double-quoted) and `Location` on a response from
/// resource metadata (ITS-REST `headers/ETag_*.yaml` + `headers/Location_*.yaml`).
pub(crate) fn set_resource_headers(
    resp: &mut Response,
    base_path: &str,
    segment: Option<&str>,
    meta: &ResourceMeta,
) {
    if let Ok(etag) = HeaderValue::from_str(&format!("\"{}\"", meta.uid)) {
        resp.headers_mut().insert(header::ETAG, etag);
    }
    if let Ok(loc) = HeaderValue::from_str(&location(base_path, &meta.ehr_id, segment, &meta.uid)) {
        resp.headers_mut().insert(header::LOCATION, loc);
    }
    // The single, generic ATNA hook for the participant object: surface the
    // resource ids the envelope already carries for the audit layer (§8.2 step 3).
    resp.extensions_mut().insert(crate::audit::AuditObject {
        ehr_id: Some(meta.ehr_id.clone()),
        uid: Some(meta.uid.clone()),
    });
}

/// Render a create/update response honouring `Prefer` and setting
/// `ETag`/`Location`. Default (`return=minimal`) → `minimal_status` with no
/// body; `return=representation` → `repr_status` with the RM body (JSON or
/// canonical XML via `T`). `segment` is the `Location` resource collection.
pub(crate) fn write_rm<T>(
    headers: &HeaderMap,
    base_path: &str,
    minimal_status: StatusCode,
    repr_status: StatusCode,
    segment: Option<&str>,
    resp: &ServiceResponse,
    root_tag: &str,
) -> Response
where
    T: DeserializeOwned + Serialize + ToXml,
{
    let mut out = if prefers_representation(headers) {
        respond_rm::<T>(headers, repr_status, &resp.body, root_tag)
    } else {
        empty(minimal_status)
    };
    if let Some(meta) = &resp.meta {
        set_resource_headers(&mut out, base_path, segment, meta);
    }
    out
}

/// As [`write_rm`] but for a JSON-only payload (no canonical-XML shape, e.g. a
/// CONTRIBUTION wrapper).
pub(crate) fn write_json(
    headers: &HeaderMap,
    base_path: &str,
    minimal_status: StatusCode,
    repr_status: StatusCode,
    segment: Option<&str>,
    resp: &ServiceResponse,
) -> Response {
    let mut out = if prefers_representation(headers) {
        respond(headers, repr_status, &resp.body)
    } else {
        empty(minimal_status)
    };
    if let Some(meta) = &resp.meta {
        set_resource_headers(&mut out, base_path, segment, meta);
    }
    out
}

/// Render a `200 OK` read of a single spec-typed RM object, additionally setting
/// the `ETag`/`Location` the operation's spec declares (e.g.
/// `200_COMPOSITION_retrieved.yaml`, `200_EHR_STATUS_retrieved.yaml`).
pub(crate) fn read_rm<T>(
    headers: &HeaderMap,
    base_path: &str,
    segment: Option<&str>,
    resp: &ServiceResponse,
    root_tag: &str,
) -> Response
where
    T: DeserializeOwned + Serialize + ToXml,
{
    let mut out = respond_rm::<T>(headers, StatusCode::OK, &resp.body, root_tag);
    if let Some(meta) = &resp.meta {
        set_resource_headers(&mut out, base_path, segment, meta);
    }
    out
}

/// Render a `200 OK` read of a JSON-only payload (no canonical-XML shape, e.g.
/// an `ORIGINAL_VERSION` wrapper) whose spec response declares `ETag`/`Location`
/// — the `*_version_get_at_time` reads (`200_VERSION_at_time.yaml` /
/// `200_VERSION_of_COMPOSITION_at_time.yaml`: `ETag` = the `version_uid`,
/// `Location` = the VERSION resource URL).
pub(crate) fn read_json(
    headers: &HeaderMap,
    base_path: &str,
    segment: Option<&str>,
    resp: &ServiceResponse,
) -> Response {
    let mut out = respond(headers, StatusCode::OK, &resp.body);
    if let Some(meta) = &resp.meta {
        set_resource_headers(&mut out, base_path, segment, meta);
    }
    out
}

/// A `204 No Content` delete outcome carrying the deleted version's
/// `ETag`/`Location` (`204_COMPOSITION_deleted.yaml`).
pub(crate) fn deleted_with_headers(
    base_path: &str,
    segment: Option<&str>,
    resp: &ServiceResponse,
) -> Response {
    let mut out = empty(StatusCode::NO_CONTENT);
    if let Some(meta) = &resp.meta {
        set_resource_headers(&mut out, base_path, segment, meta);
    }
    out
}

/// Render an error response, additionally setting the latest-version
/// `ETag`/`Location` the spec requires on a `409`/`412` (the current
/// `version_uid`; `409_COMPOSITION_with_uid_based_id.yaml`, `412_*.yaml`).
pub(crate) fn error_with_meta(
    error: ApiError,
    base_path: &str,
    segment: Option<&str>,
    meta: Option<&ResourceMeta>,
) -> Response {
    let mut out = crate::error::RestError(error).into_response();
    if let Some(meta) = meta {
        set_resource_headers(&mut out, base_path, segment, meta);
    }
    out
}

/// Serve a pre-formed XML document (e.g. a stored OPT 1.4 operational template)
/// verbatim as `application/xml`.
pub(crate) fn xml_body(status: StatusCode, xml: String) -> Response {
    let mut resp = (status, xml).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_XML),
    );
    resp
}

fn json_response<T: Serialize>(status: StatusCode, value: &T) -> Response {
    match openehr_its::json::to_canonical_json(value) {
        Ok(json) => {
            let mut resp = (status, json).into_response();
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static(APPLICATION_JSON),
            );
            resp
        }
        Err(e) => {
            ApiError::Internal(format!("JSON serialization failed: {e}")).into_response_body()
        }
    }
}

/// Small helper so error rendering here reuses the crate's [`RestError`] body.
trait IntoErrorResponse {
    fn into_response_body(self) -> Response;
}

impl IntoErrorResponse for ApiError {
    fn into_response_body(self) -> Response {
        crate::error::RestError(self).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn accept_selection() {
        assert_eq!(response_format(&HeaderMap::new()), Format::Json);
        assert_eq!(
            response_format(&headers(&[("accept", "*/*")])),
            Format::Json
        );
        assert_eq!(
            response_format(&headers(&[("accept", "application/xml")])),
            Format::Xml
        );
        assert_eq!(
            response_format(&headers(&[("accept", "application/json, application/xml")])),
            Format::Json
        );
        assert_eq!(
            response_format(&headers(&[("accept", "text/xml")])),
            Format::Xml
        );
    }

    #[test]
    fn detects_web_template_accept() {
        assert!(wants_web_template(&headers(&[(
            "accept",
            "application/openehr.wt+json"
        )])));
        assert!(wants_web_template(&headers(&[(
            "accept",
            "application/xml, application/openehr.wt+json"
        )])));
        assert!(!wants_web_template(&headers(&[(
            "accept",
            "application/xml"
        )])));
        assert!(!wants_web_template(&HeaderMap::new()));
    }

    #[test]
    fn content_type_selection() {
        assert_eq!(request_format(&HeaderMap::new()), Format::Json);
        assert_eq!(
            request_format(&headers(&[(
                "content-type",
                "application/xml; charset=utf-8"
            )])),
            Format::Xml
        );
        assert_eq!(
            request_format(&headers(&[("content-type", "application/json")])),
            Format::Json
        );
    }

    #[test]
    fn json_body_decodes() {
        let h = headers(&[("content-type", "application/json")]);
        let v = json_value(&h, &Bytes::from_static(br#"{"a":1}"#)).expect("json");
        assert_eq!(v, serde_json::json!({"a": 1}));
    }

    #[test]
    fn xml_content_type_rejected_for_json_only_op() {
        let h = headers(&[("content-type", "application/xml")]);
        let err = json_value(&h, &Bytes::from_static(b"<x/>")).expect_err("reject");
        assert!(
            matches!(err, ApiError::UnsupportedMediaType(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn rm_body_decodes_from_both_json_and_xml() {
        use openehr_rm::prelude::DvText;

        // A real RM value, obtained from its canonical JSON.
        let dv: DvText =
            serde_json::from_value(serde_json::json!({"_type": "DV_TEXT", "value": "hello"}))
                .expect("dv_text");

        // XML request body → canonical JSON Value (the shape the trait receives).
        let xml = openehr_its::xml::to_canonical_xml(&dv, "value").expect("to xml");
        let mut xml_headers = HeaderMap::new();
        xml_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/xml"),
        );
        let from_xml = rm_value::<DvText>(&xml_headers, &Bytes::from(xml)).expect("xml decode");
        assert_eq!(from_xml["value"], "hello");

        // JSON request body → the same canonical JSON Value.
        let json = serde_json::to_vec(&dv).expect("to json");
        let mut json_headers = HeaderMap::new();
        json_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let from_json = rm_value::<DvText>(&json_headers, &Bytes::from(json)).expect("json decode");
        assert_eq!(from_json["value"], "hello");
    }

    #[test]
    fn lenient_value_wraps_non_json_text() {
        assert_eq!(
            lenient_value(&Bytes::from_static(b"<template/>")).unwrap(),
            serde_json::Value::String("<template/>".to_owned())
        );
        assert_eq!(
            lenient_value(&Bytes::from_static(br#"{"a":1}"#)).unwrap(),
            serde_json::json!({"a": 1})
        );
    }

    fn content_type(resp: &Response) -> Option<String> {
        resp.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    #[test]
    fn respond_rm_serves_json_by_default() {
        use openehr_rm::prelude::DvText;
        let value = serde_json::json!({"_type": "DV_TEXT", "value": "hello"});
        let resp = respond_rm::<DvText>(&HeaderMap::new(), StatusCode::OK, &value, "value");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(content_type(&resp).as_deref(), Some(APPLICATION_JSON));
    }

    #[test]
    fn respond_rm_null_is_bodyless() {
        use openehr_rm::prelude::DvText;
        let h = headers(&[("accept", "application/xml")]);
        let resp = respond_rm::<DvText>(
            &h,
            StatusCode::NO_CONTENT,
            &serde_json::Value::Null,
            "value",
        );
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_ne!(content_type(&resp).as_deref(), Some(APPLICATION_XML));
    }

    #[tokio::test]
    async fn respond_rm_renders_canonical_xml_for_xml_accept() {
        use http_body_util::BodyExt;
        use openehr_rm::prelude::DvText;

        // The value the handler would return: a DV_TEXT as canonical JSON.
        let value = serde_json::json!({"_type": "DV_TEXT", "value": "hello"});
        let h = headers(&[("accept", "application/xml")]);
        let resp = respond_rm::<DvText>(&h, StatusCode::OK, &value, "value");

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(content_type(&resp).as_deref(), Some(APPLICATION_XML));

        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        let xml = String::from_utf8(bytes.to_vec()).expect("utf8");
        // Real canonical XML from the generated `ToXml`, not the JSON-as-text stub.
        assert!(xml.contains("<value"), "root element present: {xml}");
        assert!(xml.contains("hello"), "leaf value present: {xml}");
        assert!(!xml.contains("_type"), "not a serialized JSON blob: {xml}");
    }

    // ── W2-A: header + `Prefer` handling ────────────────────────────────────

    const BASE: &str = "/ehrbase/rest/openehr/v1";

    fn etag(resp: &Response) -> Option<String> {
        resp.headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    fn loc(resp: &Response) -> Option<String> {
        resp.headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    #[test]
    fn prefer_default_is_minimal() {
        // Absent → minimal (spec default).
        assert!(!prefers_representation(&HeaderMap::new()));
        assert!(!prefers_representation(&headers(&[(
            "prefer",
            "return=minimal"
        )])));
        assert!(prefers_representation(&headers(&[(
            "prefer",
            "return=representation"
        )])));
        // Case-insensitive.
        assert!(prefers_representation(&headers(&[(
            "prefer",
            "RETURN=REPRESENTATION"
        )])));
    }

    #[test]
    fn location_builds_per_segment() {
        assert_eq!(location(BASE, "e1", None, "e1"), format!("{BASE}/ehr/e1"));
        assert_eq!(
            location(BASE, "e1", Some("composition"), "v::s::1"),
            format!("{BASE}/ehr/e1/composition/v::s::1")
        );
    }

    fn meta(ehr: &str, uid: &str) -> ResourceMeta {
        ResourceMeta::new(ehr.to_owned(), uid.to_owned())
    }

    #[test]
    fn write_rm_minimal_is_headers_only() {
        use openehr_rm::prelude::Composition;
        let value = serde_json::json!({"_type": "COMPOSITION"});
        let resp = ServiceResponse::new(value, meta("e1", "v::s::1"));
        // Default (no Prefer) → minimal: 201, no body, ETag + Location set.
        let out = write_rm::<Composition>(
            &HeaderMap::new(),
            BASE,
            StatusCode::CREATED,
            StatusCode::CREATED,
            Some("composition"),
            &resp,
            "composition",
        );
        assert_eq!(out.status(), StatusCode::CREATED);
        assert_eq!(etag(&out).as_deref(), Some("\"v::s::1\""));
        assert_eq!(
            loc(&out).as_deref(),
            Some(&*format!("{BASE}/ehr/e1/composition/v::s::1"))
        );
        // Minimal → no content-type body header.
        assert_eq!(content_type(&out), None);
    }

    #[test]
    fn write_rm_representation_returns_body() {
        use openehr_rm::prelude::Composition;
        let value = serde_json::json!({"_type": "COMPOSITION"});
        let resp = ServiceResponse::new(value, meta("e1", "v::s::2"));
        let h = headers(&[("prefer", "return=representation")]);
        let out = write_rm::<Composition>(
            &h,
            BASE,
            StatusCode::NO_CONTENT,
            StatusCode::OK,
            Some("composition"),
            &resp,
            "composition",
        );
        // Representation → 200 (repr status) with a JSON body + headers.
        assert_eq!(out.status(), StatusCode::OK);
        assert_eq!(content_type(&out).as_deref(), Some(APPLICATION_JSON));
        assert_eq!(etag(&out).as_deref(), Some("\"v::s::2\""));
    }

    #[test]
    fn deleted_with_headers_is_204_with_etag() {
        let resp = ServiceResponse::deleted(meta("e1", "v::s::3"));
        let out = deleted_with_headers(BASE, Some("composition"), &resp);
        assert_eq!(out.status(), StatusCode::NO_CONTENT);
        assert_eq!(etag(&out).as_deref(), Some("\"v::s::3\""));
        assert!(loc(&out).is_some());
    }

    #[test]
    fn error_with_meta_sets_latest_version_headers() {
        let out = error_with_meta(
            ApiError::PreconditionFailed("stale".to_owned()),
            BASE,
            Some("ehr_status"),
            Some(&meta("e1", "v::s::5")),
        );
        assert_eq!(out.status(), StatusCode::PRECONDITION_FAILED);
        assert_eq!(etag(&out).as_deref(), Some("\"v::s::5\""));
        assert_eq!(
            loc(&out).as_deref(),
            Some(&*format!("{BASE}/ehr/e1/ehr_status/v::s::5"))
        );
    }
}
