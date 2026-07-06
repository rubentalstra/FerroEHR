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
}
