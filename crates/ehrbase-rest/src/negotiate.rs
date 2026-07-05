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
//! - **XML responses** for the (currently untyped) stub payloads light up in P12
//!   when handlers return typed RM values; [`respond_negotiated`] already renders
//!   canonical XML for any `T: ToXml`. Until then [`respond`] serves JSON.

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
/// receives the bytes.
// TODO(port): P12 — parse OPT 1.4 XML template uploads into the template model.
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

/// Render a serializable payload as a JSON response. Used for the untyped stub
/// payloads (`serde_json::Value` and collections); if the client requested XML
/// exclusively, this returns 406 since a canonical XML shape needs a typed RM
/// value (available in P12).
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

/// Render a typed RM value, honouring `Accept` for JSON or canonical XML.
/// `root_tag` is the XML root element name (the RM attribute the value binds to,
/// e.g. `composition`). This is the path P12 uses for typed responses.
#[allow(dead_code)] // wired by P12 handlers returning typed RM values
pub(crate) fn respond_negotiated<T: Serialize + ToXml>(
    headers: &HeaderMap,
    status: StatusCode,
    value: &T,
    root_tag: &str,
) -> Response {
    match response_format(headers) {
        Format::Json => json_response(status, value),
        Format::Xml => match openehr_its::xml::to_canonical_xml(value, root_tag) {
            Ok(xml) => {
                let mut resp = (status, xml).into_response();
                resp.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(APPLICATION_XML),
                );
                resp
            }
            Err(e) => {
                ApiError::Internal(format!("XML serialization failed: {e}")).into_response_body()
            }
        },
    }
}

/// A bodyless success response (204/200 for deletes).
pub(crate) fn empty(status: StatusCode) -> Response {
    status.into_response()
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
}
