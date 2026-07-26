//! Content negotiation core (ITS-REST `Resources.md §Data representation` +
//! §Simplified Formats).
//!
//! One [`WireFormat`] enum names every representation the server negotiates —
//! canonical JSON/XML plus the Simplified Formats (FLAT, STRUCTURED, and the
//! Web Template document). Two resolvers, both parameterized by the set of
//! formats an endpoint allows, are the single negotiation seam every endpoint
//! dispatches through:
//!
//! - [`content_type_format`] classifies a request `Content-Type` (unknown →
//!   `None` → the caller answers `415`, Resources.md §Simplified Formats MUST).
//! - [`resolve_accept`] parses `Accept` with RFC 9110 §12.5.1 quality values
//!   and returns the highest-q allowed format (`None` → the caller answers
//!   `406`, same MUST rule).
//!
//! The recognized media types are matched EXACTLY (`application/json`,
//! `application/xml`, `application/openehr.wt.flat+json`,
//! `application/openehr.wt.structured+json`, `application/openehr.wt+json`).
//! The deprecated `…wt.flat.schema+json`/`…wt.structured.schema+json` names and
//! the legacy `application/openehr.nc.flat+json` / `…tds2+xml` types are NOT
//! recognized (Resources.md §Simplified Formats NOTE + §Alternative data
//! formats) — they fall out as `406`/`415` like any other unsupported type.
//!
//! Request bodies and responses for the canonical formats are (de)serialized
//! via `openehr-its` (`to_canonical_json`/`from_canonical_json`,
//! `to_canonical_xml`/`from_canonical_xml`). The generated server traits
//! exchange `serde_json::Value` at the boundary, so an XML body is decoded into
//! its concrete `openehr-rm` type and re-emitted as the canonical JSON `Value`
//! the trait expects (see [`rm_value`]), and an XML response re-types the
//! canonical JSON into its `openehr-rm` type so the generated `ToXml` runs (see
//! [`respond_rm`]). The Simplified-Formats payload conversion is the sibling
//! `crate::formats` adapter.

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode, header};

use openehr_its::json_codec::runtime::{FromJson, ToJson};
use openehr_its::rest::runtime::ApiError;
use openehr_its::xml::{FromXml, ToXml};

use ehrbase::service::response::{ResourceMeta, ServiceResponse};

/// A negotiated wire representation of a resource (ITS-REST
/// `Resources.md §Data representation` + §Simplified Formats).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WireFormat {
    /// `application/json` — canonical openEHR JSON.
    CanonicalJson,
    /// `application/xml` (or `text/xml`) — canonical openEHR XML.
    CanonicalXml,
    /// `application/openehr.wt.flat+json` — Simplified FLAT (simSDT) JSON.
    Flat,
    /// `application/openehr.wt.structured+json` — Simplified STRUCTURED
    /// (structSDT) JSON.
    Structured,
    /// `application/openehr.wt+json` — the Operational Template rendered as a
    /// Web Template document (template resource only).
    WebTemplate,
}

const APPLICATION_JSON: &str = "application/json";
const APPLICATION_XML: &str = "application/xml";
const TEXT_XML: &str = "text/xml";
/// Web Template document media type (`master02 §MIME Types`, template only).
const APPLICATION_WT_JSON: &str = "application/openehr.wt+json";
/// Simplified FLAT (simSDT) media type.
const APPLICATION_WT_FLAT_JSON: &str = "application/openehr.wt.flat+json";
/// Simplified STRUCTURED (structSDT) media type.
const APPLICATION_WT_STRUCTURED_JSON: &str = "application/openehr.wt.structured+json";

/// The canonical-only allowed set (`Accept_LOCATABLE` minus the simplified
/// types) — the negotiation set for every canonical RM object endpoint.
pub(crate) const CANONICAL: &[WireFormat] = &[WireFormat::CanonicalJson, WireFormat::CanonicalXml];

// ── The negotiation core ─────────────────────────────────────────────────

/// The media type token of a header range (parameters after `;` stripped).
fn media_token(range: &str) -> &str {
    range.split(';').next().unwrap_or(range).trim()
}

/// Classify one media type (parameters stripped, ASCII-lowercased) into a
/// [`WireFormat`]. Exact-match only — the deprecated `…schema+json` and legacy
/// `…nc.flat+json`/`…tds2+xml` types are deliberately unrecognized
/// (`Resources.md §Simplified Formats` NOTE + §Alternative data formats), so
/// they return `None`.
fn classify_media(media: &str) -> Option<WireFormat> {
    match media {
        APPLICATION_JSON => Some(WireFormat::CanonicalJson),
        APPLICATION_XML | TEXT_XML => Some(WireFormat::CanonicalXml),
        APPLICATION_WT_FLAT_JSON => Some(WireFormat::Flat),
        APPLICATION_WT_STRUCTURED_JSON => Some(WireFormat::Structured),
        APPLICATION_WT_JSON => Some(WireFormat::WebTemplate),
        _ => None,
    }
}

/// The [`WireFormat`] a request `Content-Type` declares, or `None` when the
/// media type is not one this server recognizes (caller → `415`, Resources.md
/// §Simplified Formats MUST). An absent `Content-Type` defaults to canonical
/// JSON (`Resources.md §JSON Format`).
pub(crate) fn content_type_format(headers: &HeaderMap) -> Option<WireFormat> {
    match header_str(headers, header::CONTENT_TYPE) {
        None => Some(WireFormat::CanonicalJson),
        Some(ct) => classify_media(&media_token(&ct).to_ascii_lowercase()),
    }
}

/// Resolve the response [`WireFormat`] from `Accept` against `allowed`, per
/// RFC 9110 §12.5.1 quality-value negotiation (`Resources.md §Data
/// representation`): the highest-q media range that maps to an allowed format
/// wins; a more specific range beats a wildcard at equal q; the endpoint
/// `default` breaks any remaining tie and answers an absent (or empty)
/// `Accept`. Returns `None` when no allowed format is acceptable (caller →
/// `406`, Resources.md §Simplified Formats MUST).
pub(crate) fn resolve_accept(
    headers: &HeaderMap,
    allowed: &[WireFormat],
    default: WireFormat,
) -> Option<WireFormat> {
    let Some(accept) = header_str(headers, header::ACCEPT) else {
        return Some(default);
    };
    if accept.trim().is_empty() {
        return Some(default);
    }
    let mut best: Option<(WireFormat, f64, u8)> = None;
    for &fmt in allowed {
        let Some((q, spec)) = match_quality(&accept, fmt) else {
            continue;
        };
        if q <= 0.0 {
            // `;q=0` explicitly rejects the format (RFC 9110 §12.5.1).
            continue;
        }
        let candidate = (fmt, q, spec);
        best = Some(match best {
            None => candidate,
            Some(current) => choose(current, candidate, default),
        });
    }
    best.map(|(fmt, _, _)| fmt)
}

/// The best `(quality, specificity)` an `Accept` header offers for `fmt`, or
/// `None` if no media range matches it. specificity: `2` = exact type/subtype,
/// `1` = a type wildcard (`application/*`, `text/*`), `0` = `*/*`.
fn match_quality(accept: &str, fmt: WireFormat) -> Option<(f64, u8)> {
    let mut best: Option<(f64, u8)> = None;
    for range in accept.split(',') {
        let range = range.trim();
        if range.is_empty() {
            continue;
        }
        let token = media_token(range).to_ascii_lowercase();
        let Some(spec) = specificity_for(&token, fmt) else {
            continue;
        };
        let q = quality_of(range);
        best = Some(match best {
            None => (q, spec),
            Some((bq, bs)) if q > bq || (q >= bq && spec > bs) => (q, spec),
            Some(current) => current,
        });
    }
    best
}

/// The specificity with which `token` matches `fmt`, or `None` for no match.
fn specificity_for(token: &str, fmt: WireFormat) -> Option<u8> {
    match token {
        "*/*" => Some(0),
        // Every negotiated format has an `application/*` media type.
        "application/*" => Some(1),
        "text/*" => (fmt == WireFormat::CanonicalXml).then_some(1),
        exact => (classify_media(exact) == Some(fmt)).then_some(2),
    }
}

/// The quality value of a media range (`;q=` weight; default `1.0`, clamped to
/// `[0, 1]`, RFC 9110 §12.5.1).
fn quality_of(range: &str) -> f64 {
    for param in range.split(';').skip(1) {
        let param = param.trim();
        if let Some(v) = param
            .strip_prefix("q=")
            .or_else(|| param.strip_prefix("Q="))
        {
            return v.trim().parse::<f64>().map_or(1.0, |q| q.clamp(0.0, 1.0));
        }
    }
    1.0
}

/// Pick the winner of two candidates: higher q, then higher specificity, then
/// the endpoint `default`, then a fixed preference order (canonical JSON
/// first).
fn choose(
    a: (WireFormat, f64, u8),
    b: (WireFormat, f64, u8),
    default: WireFormat,
) -> (WireFormat, f64, u8) {
    if a.1 > b.1 {
        return a;
    }
    if b.1 > a.1 {
        return b;
    }
    if a.2 > b.2 {
        return a;
    }
    if b.2 > a.2 {
        return b;
    }
    if a.0 == default {
        return a;
    }
    if b.0 == default {
        return b;
    }
    if pref_rank(a.0) <= pref_rank(b.0) {
        a
    } else {
        b
    }
}

/// Fixed tie-break order: canonical JSON is the server's preferred default.
fn pref_rank(fmt: WireFormat) -> u8 {
    match fmt {
        WireFormat::CanonicalJson => 0,
        WireFormat::CanonicalXml => 1,
        WireFormat::Flat => 2,
        WireFormat::Structured => 3,
        WireFormat::WebTemplate => 4,
    }
}

// ── Simplified-Formats + Web Template body builders ────────────────────────

/// Serve a pre-serialized STRUCTURED (structSDT) document as
/// `application/openehr.wt.structured+json`.
pub(crate) fn structured_json_body(status: StatusCode, json: String) -> Response {
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_WT_STRUCTURED_JSON),
    );
    resp
}

/// Serve a pre-serialized JSON document as `application/json`.
///
/// The `application/json` sibling of [`wt_json_body`] / [`flat_json_body`] /
/// [`structured_json_body`]: the same body under the canonical JSON media
/// type, for an endpoint whose negotiated `Accept` was `application/json`
/// (`Resources.md` §JSON Format: "Proper header `Content-Type:
/// application/json` MUST be present in the response of the service unless the
/// response has no content body"). Payloads that are serialized here rather
/// than by the RM codec use [`respond`]; this builder takes the already-formed
/// document.
pub(crate) fn json_body(status: StatusCode, json: String) -> Response {
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_JSON),
    );
    resp
}

/// Serve a pre-serialized Web Template document as `application/openehr.wt+json`.
pub(crate) fn wt_json_body(status: StatusCode, json: String) -> Response {
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_WT_JSON),
    );
    resp
}

/// Serve a pre-serialized FLAT (simSDT) document as
/// `application/openehr.wt.flat+json`.
pub(crate) fn flat_json_body(status: StatusCode, json: String) -> Response {
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_WT_FLAT_JSON),
    );
    resp
}

// ── Body decoders ──────────────────────────────────────────────────────────

/// Format an HTTP-date (RFC 7231 IMF-fixdate, always GMT) for `Last-Modified`.
pub(crate) fn http_date(at: jiff::Timestamp) -> String {
    at.strftime("%a, %d %b %Y %H:%M:%S GMT").to_string()
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

/// Decode an RM-typed body from canonical JSON or XML into the canonical JSON
/// `Value` the trait expects. `T` is the concrete `openehr-rm` payload type.
///
/// # Errors
/// [`ApiError::BadRequest`] if the body cannot be parsed in the declared
/// format; [`ApiError::UnsupportedMediaType`] for any `Content-Type` other than
/// canonical JSON or XML (a Simplified-Formats or unknown type on a canonical
/// RM endpoint, Resources.md §Simplified Formats MUST).
pub(crate) fn rm_value<T>(headers: &HeaderMap, body: &Bytes) -> Result<serde_json::Value, ApiError>
where
    T: FromXml + ToJson,
{
    match content_type_format(headers) {
        Some(WireFormat::CanonicalJson) => parse_json(body),
        Some(WireFormat::CanonicalXml) => {
            let xml = text_body(body)?;
            let value: T = openehr_its::xml::from_canonical_xml(&xml)
                .map_err(|e| ApiError::BadRequest(format!("invalid canonical XML body: {e}")))?;
            Ok(openehr_its::json::to_canonical_value(&value))
        }
        _ => Err(ApiError::UnsupportedMediaType(format!(
            "this operation accepts application/json or application/xml only, got {}",
            header_str(headers, header::CONTENT_TYPE).unwrap_or_else(|| "<none>".to_owned())
        ))),
    }
}

/// Optional RM-typed body (empty → `None`).
pub(crate) fn optional_rm_value<T>(
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Option<serde_json::Value>, ApiError>
where
    T: FromXml + ToJson,
{
    if body.is_empty() {
        return Ok(None);
    }
    Ok(Some(rm_value::<T>(headers, body)?))
}

/// A JSON-only write op accepts only canonical JSON on the wire.
fn require_json(headers: &HeaderMap) -> Result<(), ApiError> {
    match content_type_format(headers) {
        Some(WireFormat::CanonicalJson) => Ok(()),
        _ => Err(ApiError::UnsupportedMediaType(format!(
            "this operation accepts application/json only, got {}",
            header_str(headers, header::CONTENT_TYPE).unwrap_or_else(|| "<none>".to_owned())
        ))),
    }
}

/// Refuse a request whose `Content-Type` DECLARES a media type outside
/// `allowed`. `expected` names the accepted type(s) in the error message.
///
/// An ABSENT `Content-Type` is accepted: `Resources.md` §XML Format and §JSON
/// Format both make the header a client MAY ("A client MAY use the header
/// `Content-Type: application/xml` in the requests to specify the XML payload
/// format"), so its absence declares nothing to refuse and the operation's own
/// single body type applies.
///
/// # Errors
/// [`ApiError::UnsupportedMediaType`] when the declared media type is not one
/// of `allowed` (`Resources.md` §XML Format: "If the service cannot process
/// the request payload as XML format, it MUST respond with HTTP status code
/// `415 Unsupported Media Type`" — and the symmetric §JSON Format sentence).
pub(crate) fn require_content_type(
    headers: &HeaderMap,
    allowed: &[WireFormat],
    expected: &str,
) -> Result<(), ApiError> {
    let Some(declared) = header_str(headers, header::CONTENT_TYPE) else {
        return Ok(());
    };
    match classify_media(&media_token(&declared).to_ascii_lowercase()) {
        Some(fmt) if allowed.contains(&fmt) => Ok(()),
        _ => Err(ApiError::UnsupportedMediaType(format!(
            "this operation accepts {expected} only, got {declared}"
        ))),
    }
}

fn parse_json(body: &Bytes) -> Result<serde_json::Value, ApiError> {
    serde_json::from_slice(body)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON body: {e}")))
}

/// Render a serializable payload as a JSON response. Used for responses that
/// are not a spec-typed RM value (`serde_json::Value` collections and DTOs:
/// item tags, terminology/query results). If the client's `Accept` cannot be
/// satisfied by canonical JSON, this returns `406` (those payloads have no
/// spec-defined canonical-XML shape). Spec-typed RM objects use [`respond_rm`].
pub(crate) fn respond<T: serde::Serialize>(
    headers: &HeaderMap,
    status: StatusCode,
    value: &T,
) -> Response {
    // `respond` serves JSON-only, non-RM payloads: a `serde_json::Value` the
    // service already produced (already canonical), or an application DTO with
    // its own serde. These are not RM spec types, so they serialize via serde
    // (not the RM canonical codec — that path is `respond_rm`/`json_response`).
    match resolve_accept(
        headers,
        &[WireFormat::CanonicalJson],
        WireFormat::CanonicalJson,
    ) {
        Some(_) => match serde_json::to_string(value) {
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
        },
        None => ApiError::NotAcceptable(
            "this response is available as application/json only".to_owned(),
        )
        .into_response_body(),
    }
}

/// Render a canonical-JSON `Value` that IS a single spec-typed RM object,
/// honouring `Accept` for canonical JSON or XML. `T` is the concrete
/// `openehr-rm` type the value encodes; `root_tag` is the XML root element
/// name. A JSON `null` value renders as a bodyless response.
pub(crate) fn respond_rm<T>(
    headers: &HeaderMap,
    status: StatusCode,
    value: &serde_json::Value,
    root_tag: &str,
) -> Response
where
    T: FromJson + ToXml,
{
    match resolve_accept(headers, CANONICAL, WireFormat::CanonicalJson) {
        Some(WireFormat::CanonicalJson) => json_response(status, value),
        Some(WireFormat::CanonicalXml) => {
            if value.is_null() {
                return empty(status);
            }
            let typed: T = match openehr_its::json::from_canonical_value(value) {
                Ok(t) => t,
                Err(e) => {
                    return ApiError::Internal(format!(
                        "re-typing canonical JSON to <{root_tag}> for the XML response failed: {e}"
                    ))
                    .into_response_body();
                }
            };
            match openehr_its::xml::to_canonical_xml(&typed, root_tag) {
                Ok(xml) => xml_body(status, xml),
                Err(e) => ApiError::Internal(format!("XML serialization failed: {e}"))
                    .into_response_body(),
            }
        }
        // Any other allowed format is impossible for `CANONICAL`; an
        // unsatisfiable `Accept` is a `406` (Resources.md §Simplified Formats).
        _ => ApiError::NotAcceptable(
            "this resource is available as application/json or application/xml".to_owned(),
        )
        .into_response_body(),
    }
}

/// A bodyless success response (204/200 for deletes).
pub(crate) fn empty(status: StatusCode) -> Response {
    status.into_response()
}

/// A bodyless success response carrying a `Location` header.
pub(crate) fn empty_with_location(status: StatusCode, location: &str) -> Response {
    let mut resp = status.into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        resp.headers_mut().insert(header::LOCATION, value);
    }
    resp
}

// ── ITS-REST response-header + `Prefer` handling ─────────────────────

/// Whether the client asked for the full representation on `Prefer`
/// (`return=representation`). The ITS-REST default is `return=minimal`.
pub(crate) fn prefers_representation(headers: &HeaderMap) -> bool {
    headers
        .get("prefer")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|p| {
            p.split(',')
                .any(|t| t.trim().eq_ignore_ascii_case("return=representation"))
        })
}

/// Whether the client asked for `OBJECT_REF` resolution on `Prefer`
/// (`resolve_refs`).
pub(crate) fn prefers_resolve_refs(headers: &HeaderMap) -> bool {
    headers
        .get("prefer")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|p| {
            p.split(',')
                .any(|t| t.trim().eq_ignore_ascii_case("resolve_refs"))
        })
}

/// Whether the client asked for an identifier-only response on `Prefer`
/// (`return=identifier`).
pub(crate) fn prefers_identifier(headers: &HeaderMap) -> bool {
    headers
        .get("prefer")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|p| {
            p.split(',')
                .any(|t| t.trim().eq_ignore_ascii_case("return=identifier"))
        })
}

/// The return preference the server is honouring, for the `Preference-Applied`
/// response header.
fn applied_preference(headers: &HeaderMap) -> &'static str {
    if prefers_representation(headers) {
        "representation"
    } else if prefers_identifier(headers) {
        "identifier"
    } else {
        "minimal"
    }
}

/// Emit `Preference-Applied: return=<kind>` on a write response.
fn set_preference_applied(resp: &mut Response, kind: &str) {
    if let Ok(value) = HeaderValue::from_str(&format!("return={kind}")) {
        resp.headers_mut()
            .insert(header::HeaderName::from_static("preference-applied"), value);
    }
}

/// The status for a `return=identifier` write: `201`/`200`, never `204`.
fn identifier_status(minimal_status: StatusCode, repr_status: StatusCode) -> StatusCode {
    if minimal_status == StatusCode::NO_CONTENT {
        repr_status
    } else {
        minimal_status
    }
}

/// Render a `return=identifier` response body: `{ "uid": "<uid>" }` in JSON, or
/// the `<uid>` element when XML is negotiated.
pub(crate) fn identifier_response(headers: &HeaderMap, status: StatusCode, uid: &str) -> Response {
    match resolve_accept(headers, CANONICAL, WireFormat::CanonicalJson) {
        Some(WireFormat::CanonicalXml) => {
            // The OAS defines the identifier body only for JSON; the spec is
            // silent on an XML shape, so we emit a minimal `<uid>` element as
            // the direct XML equivalent of the JSON `{uid}`.
            xml_body(status, format!("<uid>{}</uid>", xml_escape(uid)))
        }
        _ => json_response(status, &serde_json::json!({ "uid": uid })),
    }
}

/// Escape the XML text-content special characters.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Build the (path-absolute) `Location` URL for an EHR sub-resource.
pub(crate) fn location(base_path: &str, ehr_id: &str, segment: Option<&str>, uid: &str) -> String {
    match segment {
        Some(seg) => format!("{base_path}/ehr/{ehr_id}/{seg}/{uid}"),
        None => format!("{base_path}/ehr/{ehr_id}"),
    }
}

/// The `ETag` header value for a resource identifier: the weak form `W/"{uid}"`
/// (overview §"`ETag` and Last-Modified" — the `W/` weakness indicator is a MUST).
pub(crate) fn resource_etag(uid: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!("W/\"{uid}\"")).ok()
}

/// Set the weak `ETag` of a resource identifier on a response (overview
/// §"`ETag` and Last-Modified" — the value "is usually taken from e.g.
/// `VERSIONED_OBJECT.uid.value`, `VERSION.uid.value`, `EHR.ehr_id.value`").
/// The single place the `W/"…"` header is written.
pub(crate) fn set_etag(resp: &mut Response, uid: &str) {
    if let Some(etag) = resource_etag(uid) {
        resp.headers_mut().insert(header::ETAG, etag);
    }
}

/// Set the versioning headers on a response: the weak `ETag` and — when the
/// metadata carries a commit time — `Last-Modified`. No `Location`.
pub(crate) fn set_versioning_headers(resp: &mut Response, meta: &ResourceMeta) {
    set_etag(resp, &meta.uid);
    if let Some(at) = meta.last_modified
        && let Ok(lm) = HeaderValue::from_str(&http_date(at))
    {
        resp.headers_mut().insert(header::LAST_MODIFIED, lm);
    }
    resp.extensions_mut()
        .insert(crate::system_log::middleware::AuditObject {
            ehr_id: Some(meta.ehr_id.clone()),
            uid: Some(meta.uid.clone()),
        });
}

/// Set the `Location` header for a newly created/updated resource
/// (overview §Location — creation/redirect only).
pub(crate) fn set_location(
    resp: &mut Response,
    base_path: &str,
    segment: Option<&str>,
    meta: &ResourceMeta,
) {
    if let Ok(loc) = HeaderValue::from_str(&location(base_path, &meta.ehr_id, segment, &meta.uid)) {
        resp.headers_mut().insert(header::LOCATION, loc);
    }
}

/// Set the full create/update response headers: the versioning headers plus
/// `Location`.
pub(crate) fn set_resource_headers(
    resp: &mut Response,
    base_path: &str,
    segment: Option<&str>,
    meta: &ResourceMeta,
) {
    set_versioning_headers(resp, meta);
    set_location(resp, base_path, segment, meta);
}

/// Render a create/update response honouring `Prefer` and setting the
/// versioning + `Location` headers.
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
    T: FromJson + ToXml,
{
    let uid = resp.meta.as_ref().map(|m| m.uid.clone());
    let mut out = if prefers_representation(headers) {
        respond_rm::<T>(headers, repr_status, &resp.body, root_tag)
    } else if let (true, Some(uid)) = (prefers_identifier(headers), uid.as_deref()) {
        identifier_response(headers, identifier_status(minimal_status, repr_status), uid)
    } else {
        empty(minimal_status)
    };
    if let Some(meta) = &resp.meta {
        set_resource_headers(&mut out, base_path, segment, meta);
    }
    set_preference_applied(&mut out, applied_preference(headers));
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
    let uid = resp.meta.as_ref().map(|m| m.uid.clone());
    let mut out = if prefers_representation(headers) {
        respond(headers, repr_status, &resp.body)
    } else if let (true, Some(uid)) = (prefers_identifier(headers), uid.as_deref()) {
        identifier_response(headers, identifier_status(minimal_status, repr_status), uid)
    } else {
        empty(minimal_status)
    };
    if let Some(meta) = &resp.meta {
        set_resource_headers(&mut out, base_path, segment, meta);
    }
    set_preference_applied(&mut out, applied_preference(headers));
    out
}

/// Render a `200 OK` read of a single spec-typed RM object, additionally
/// setting the weak `ETag`/`Last-Modified`. No `Location` (overview §Location).
pub(crate) fn read_rm<T>(
    headers: &HeaderMap,
    base_path: &str,
    segment: Option<&str>,
    resp: &ServiceResponse,
    root_tag: &str,
) -> Response
where
    T: FromJson + ToXml,
{
    let _ = (base_path, segment);
    let mut out = respond_rm::<T>(headers, StatusCode::OK, &resp.body, root_tag);
    if let Some(meta) = &resp.meta {
        set_versioning_headers(&mut out, meta);
    }
    out
}

/// A `204 No Content` delete outcome carrying the deleted version's weak
/// `ETag`/`Last-Modified`. No `Location` (overview §Location).
pub(crate) fn deleted_with_headers(
    base_path: &str,
    segment: Option<&str>,
    resp: &ServiceResponse,
) -> Response {
    let _ = (base_path, segment);
    let mut out = empty(StatusCode::NO_CONTENT);
    if let Some(meta) = &resp.meta {
        set_versioning_headers(&mut out, meta);
    }
    out
}

/// Render an error response, additionally setting the latest-version `ETag` the
/// spec requires on a `409`/`412`. No `Location` on the error path.
pub(crate) fn error_with_meta(
    error: ApiError,
    base_path: &str,
    segment: Option<&str>,
    meta: Option<&ResourceMeta>,
) -> Response {
    let _ = (base_path, segment);
    let mut out = crate::overview::error::RestError(error).into_response();
    if let Some(meta) = meta {
        set_versioning_headers(&mut out, meta);
    }
    out
}

/// Serve a pre-formed XML document verbatim as `application/xml`.
pub(crate) fn xml_body(status: StatusCode, xml: String) -> Response {
    let mut resp = (status, xml).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_XML),
    );
    resp
}

fn json_response<T: ToJson>(status: StatusCode, value: &T) -> Response {
    // The native codec serializes canonical JSON infallibly.
    let json = openehr_its::json::to_canonical_json(value);
    let mut resp = (status, json).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_JSON),
    );
    resp
}

/// Small helper so error rendering here reuses the crate's [`RestError`] body.
trait IntoErrorResponse {
    fn into_response_body(self) -> Response;
}

impl IntoErrorResponse for ApiError {
    fn into_response_body(self) -> Response {
        crate::overview::error::RestError(self).into_response()
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
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

    /// The prior `accept_selection` assertions, re-expressed against the
    /// [`resolve_accept`] core (RFC 9110 §12.5.1): the same canonical
    /// json-preferred-over-xml behaviour, plus `text/xml` → XML.
    #[test]
    fn accept_selection() {
        let json = Some(WireFormat::CanonicalJson);
        let xml = Some(WireFormat::CanonicalXml);
        assert_eq!(
            resolve_accept(&HeaderMap::new(), CANONICAL, WireFormat::CanonicalJson),
            json
        );
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "*/*")]),
                CANONICAL,
                WireFormat::CanonicalJson
            ),
            json
        );
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "application/xml")]),
                CANONICAL,
                WireFormat::CanonicalJson
            ),
            xml
        );
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "application/json, application/xml")]),
                CANONICAL,
                WireFormat::CanonicalJson
            ),
            json
        );
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "text/xml")]),
                CANONICAL,
                WireFormat::CanonicalJson
            ),
            xml
        );
    }

    /// q-values pick the highest-weight acceptable format even when it is not
    /// the server default (RFC 9110 §12.5.1).
    #[test]
    fn accept_qvalue_prefers_highest_weight() {
        // XML at q=1 beats JSON at q=0.5.
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "application/json;q=0.5, application/xml;q=1.0")]),
                CANONICAL,
                WireFormat::CanonicalJson
            ),
            Some(WireFormat::CanonicalXml)
        );
        // A specific type beats a `*/*` wildcard at equal q.
        let allowed = &[
            WireFormat::CanonicalJson,
            WireFormat::CanonicalXml,
            WireFormat::Flat,
            WireFormat::Structured,
        ];
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "application/openehr.wt.flat+json, */*")]),
                allowed,
                WireFormat::CanonicalJson
            ),
            Some(WireFormat::Flat)
        );
        // `;q=0` explicitly rejects a format.
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "application/json;q=0, application/xml")]),
                CANONICAL,
                WireFormat::CanonicalJson
            ),
            Some(WireFormat::CanonicalXml)
        );
    }

    /// The Simplified data-instance types resolve only where the endpoint
    /// allows them; on a canonical-only endpoint they are unacceptable (`406`).
    #[test]
    fn accept_simplified_only_where_allowed() {
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "application/openehr.wt.flat+json")]),
                CANONICAL,
                WireFormat::CanonicalJson
            ),
            None
        );
        let with_flat = &[WireFormat::CanonicalJson, WireFormat::Flat];
        assert_eq!(
            resolve_accept(
                &headers(&[("accept", "application/openehr.wt.flat+json")]),
                with_flat,
                WireFormat::CanonicalJson
            ),
            Some(WireFormat::Flat)
        );
    }

    /// The deprecated `…schema+json` and legacy `…nc.flat+json` types are not
    /// recognized (Resources.md §Simplified Formats NOTE + §Alternative data
    /// formats): unacceptable on `Accept`, unsupported on `Content-Type`.
    #[test]
    fn deprecated_and_legacy_types_unrecognized() {
        let all = &[
            WireFormat::CanonicalJson,
            WireFormat::CanonicalXml,
            WireFormat::Flat,
            WireFormat::Structured,
            WireFormat::WebTemplate,
        ];
        for t in [
            "application/openehr.wt.flat.schema+json",
            "application/openehr.wt.structured.schema+json",
            "application/openehr.nc.flat+json",
            "application/openehr.tds2+xml",
        ] {
            assert_eq!(
                resolve_accept(&headers(&[("accept", t)]), all, WireFormat::CanonicalJson),
                None,
                "{t} must not be an acceptable Accept"
            );
            assert_eq!(
                content_type_format(&headers(&[("content-type", t)])),
                None,
                "{t} must not be a recognized Content-Type"
            );
        }
    }

    #[test]
    fn content_type_selection() {
        assert_eq!(
            content_type_format(&HeaderMap::new()),
            Some(WireFormat::CanonicalJson)
        );
        assert_eq!(
            content_type_format(&headers(&[(
                "content-type",
                "application/xml; charset=utf-8"
            )])),
            Some(WireFormat::CanonicalXml)
        );
        assert_eq!(
            content_type_format(&headers(&[("content-type", "application/json")])),
            Some(WireFormat::CanonicalJson)
        );
        assert_eq!(
            content_type_format(&headers(&[(
                "content-type",
                "application/openehr.wt.flat+json"
            )])),
            Some(WireFormat::Flat)
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

    /// `Resources.md` §XML Format: a request payload the service cannot
    /// process as XML MUST be answered `415`; the header itself is a client
    /// MAY, so an absent `Content-Type` is not a refusal.
    #[test]
    fn require_content_type_refuses_only_a_declared_foreign_type() {
        let xml_only = &[WireFormat::CanonicalXml];
        assert!(
            require_content_type(&HeaderMap::new(), xml_only, "application/xml").is_ok(),
            "an absent Content-Type declares nothing to refuse (Resources.md \
             §XML Format: a client MAY use the header)"
        );
        for accepted in [
            "application/xml",
            "text/xml",
            "application/xml; charset=utf-8",
        ] {
            assert!(
                require_content_type(
                    &headers(&[("content-type", accepted)]),
                    xml_only,
                    "application/xml"
                )
                .is_ok(),
                "{accepted} is the XML payload format"
            );
        }
        for refused in [
            "application/json",
            "text/plain",
            "application/openehr.wt+json",
        ] {
            let err = require_content_type(
                &headers(&[("content-type", refused)]),
                xml_only,
                "application/xml",
            )
            .expect_err("refused");
            assert!(
                matches!(err, ApiError::UnsupportedMediaType(_)),
                "Resources.md §XML Format: a payload the service cannot process as \
                 XML MUST be 415, got {err:?} for {refused}"
            );
        }
    }

    #[test]
    fn rm_value_rejects_simplified_content_type() {
        use openehr_rm::prelude::DvText;
        let h = headers(&[("content-type", "application/openehr.wt.flat+json")]);
        let err = rm_value::<DvText>(&h, &Bytes::from_static(b"{}")).expect_err("reject");
        assert!(
            matches!(err, ApiError::UnsupportedMediaType(_)),
            "a simplified Content-Type on a canonical RM op is 415: {err:?}"
        );
    }

    #[test]
    fn rm_body_decodes_from_both_json_and_xml() {
        use openehr_rm::prelude::DvText;

        let dv: DvText = openehr_its::json::from_canonical_value(
            &serde_json::json!({"_type": "DV_TEXT", "value": "hello"}),
        )
        .expect("dv_text");

        let xml = openehr_its::xml::to_canonical_xml(&dv, "value").expect("to xml");
        let mut xml_headers = HeaderMap::new();
        xml_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/xml"),
        );
        let from_xml = rm_value::<DvText>(&xml_headers, &Bytes::from(xml)).expect("xml decode");
        assert_eq!(from_xml["value"], "hello");

        let json = openehr_its::json::to_canonical_json(&dv).into_bytes();
        let mut json_headers = HeaderMap::new();
        json_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let from_json = rm_value::<DvText>(&json_headers, &Bytes::from(json)).expect("json decode");
        assert_eq!(from_json["value"], "hello");
    }

    /// The OPT 1.4 upload reads its XML body verbatim; the retired
    /// `lenient_value` JSON-or-string wrapper (which let a JSON-declared
    /// payload reach the parser and fail `400` instead of `415`) is gone.
    #[test]
    fn text_body_reads_an_xml_payload_verbatim() {
        assert_eq!(
            text_body(&Bytes::from_static(b"<template/>")).unwrap(),
            "<template/>"
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
    fn respond_rm_rejects_simplified_accept_with_406() {
        use openehr_rm::prelude::DvText;
        let h = headers(&[("accept", "application/openehr.wt.flat+json")]);
        let value = serde_json::json!({"_type": "DV_TEXT", "value": "hello"});
        let resp = respond_rm::<DvText>(&h, StatusCode::OK, &value, "value");
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
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

        let value = serde_json::json!({"_type": "DV_TEXT", "value": "hello"});
        let h = headers(&[("accept", "application/xml")]);
        let resp = respond_rm::<DvText>(&h, StatusCode::OK, &value, "value");

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(content_type(&resp).as_deref(), Some(APPLICATION_XML));

        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        let xml = String::from_utf8(bytes.to_vec()).expect("utf8");
        assert!(xml.contains("<value"), "root element present: {xml}");
        assert!(xml.contains("hello"), "leaf value present: {xml}");
        assert!(!xml.contains("_type"), "not a serialized JSON blob: {xml}");
    }

    #[tokio::test]
    async fn respond_rm_renders_original_version_xml_with_signature() {
        use http_body_util::BodyExt;
        use openehr_rm::prelude::{DvText, OriginalVersion};

        let value = serde_json::json!({
            "_type": "ORIGINAL_VERSION",
            "contribution": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "CONTRIBUTION",
                "id": { "_type": "HIER_OBJECT_ID", "value": "c1" }
            },
            "commit_audit": {
                "_type": "AUDIT_DETAILS",
                "system_id": "ehrbase-rs",
                "time_committed": { "_type": "DV_DATE_TIME", "value": "2024-01-01T00:00:00Z" },
                "change_type": {
                    "_type": "DV_CODED_TEXT", "value": "creation",
                    "defining_code": { "_type": "CODE_PHRASE",
                        "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                        "code_string": "249" }
                },
                "committer": { "_type": "PARTY_IDENTIFIED", "name": "clinician" }
            },
            "signature": "-----BEGIN PGP SIGNATURE-----\nDEADBEEF\n-----END PGP SIGNATURE-----",
            "uid": { "_type": "OBJECT_VERSION_ID", "value": "v1::openEHRSys::1" },
            "lifecycle_state": {
                "_type": "DV_CODED_TEXT", "value": "complete",
                "defining_code": { "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": "532" }
            },
            "data": { "_type": "DV_TEXT", "value": "hello" }
        });
        let h = headers(&[("accept", "application/xml")]);
        let resp =
            respond_rm::<OriginalVersion<DvText>>(&h, StatusCode::OK, &value, "original_version");

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(content_type(&resp).as_deref(), Some(APPLICATION_XML));
        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        let xml = String::from_utf8(bytes.to_vec()).expect("utf8");
        assert!(xml.contains("<original_version"), "root element: {xml}");
        assert!(
            xml.contains("<signature"),
            "signature element present: {xml}"
        );
        assert!(xml.contains("DEADBEEF"), "signature value present: {xml}");
        assert!(!xml.contains("\"_type\""), "not a JSON blob: {xml}");
    }

    #[tokio::test]
    async fn respond_rm_renders_versioned_object_xml() {
        use http_body_util::BodyExt;
        use openehr_rm::prelude::VersionedComposition;

        let value = serde_json::json!({
            "_type": "VERSIONED_COMPOSITION",
            "uid": { "_type": "HIER_OBJECT_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515" },
            "owner_id": {
                "_type": "OBJECT_REF", "namespace": "local", "type": "EHR",
                "id": { "_type": "HIER_OBJECT_ID", "value": "e1" }
            },
            "time_created": { "_type": "DV_DATE_TIME", "value": "2024-01-01T00:00:00Z" }
        });
        let h = headers(&[("accept", "application/xml")]);
        let resp =
            respond_rm::<VersionedComposition>(&h, StatusCode::OK, &value, "versioned_composition");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(content_type(&resp).as_deref(), Some(APPLICATION_XML));
        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        let xml = String::from_utf8(bytes.to_vec()).expect("utf8");
        assert!(
            xml.contains("<versioned_composition"),
            "root element: {xml}"
        );
        assert!(
            xml.contains("8849182c-82ad-4088-a07f-48ead4180515"),
            "uid present: {xml}"
        );
    }

    // ── header + `Prefer` handling ──────────────────────────────────────────

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

    fn preference_applied(resp: &Response) -> Option<String> {
        resp.headers()
            .get("preference-applied")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    #[test]
    fn prefer_default_is_minimal() {
        assert!(!prefers_representation(&HeaderMap::new()));
        assert!(!prefers_representation(&headers(&[(
            "prefer",
            "return=minimal"
        )])));
        assert!(prefers_representation(&headers(&[(
            "prefer",
            "return=representation"
        )])));
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
        assert_eq!(etag(&out).as_deref(), Some("W/\"v::s::1\""));
        assert_eq!(
            loc(&out).as_deref(),
            Some(&*format!("{BASE}/ehr/e1/composition/v::s::1"))
        );
        assert_eq!(content_type(&out), None);
        assert_eq!(preference_applied(&out).as_deref(), Some("return=minimal"));
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
        assert_eq!(out.status(), StatusCode::OK);
        assert_eq!(content_type(&out).as_deref(), Some(APPLICATION_JSON));
        assert_eq!(etag(&out).as_deref(), Some("W/\"v::s::2\""));
        assert_eq!(
            preference_applied(&out).as_deref(),
            Some("return=representation")
        );
    }

    #[test]
    fn deleted_with_headers_is_204_with_weak_etag_no_location() {
        let resp = ServiceResponse::deleted(meta("e1", "v::s::3"));
        let out = deleted_with_headers(BASE, Some("composition"), &resp);
        assert_eq!(out.status(), StatusCode::NO_CONTENT);
        assert_eq!(etag(&out).as_deref(), Some("W/\"v::s::3\""));
        assert!(loc(&out).is_none());
    }

    #[test]
    fn error_with_meta_sets_latest_version_etag_only() {
        let out = error_with_meta(
            ApiError::PreconditionFailed("stale".to_owned()),
            BASE,
            Some("ehr_status"),
            Some(&meta("e1", "v::s::5")),
        );
        assert_eq!(out.status(), StatusCode::PRECONDITION_FAILED);
        assert_eq!(etag(&out).as_deref(), Some("W/\"v::s::5\""));
        assert!(loc(&out).is_none());
    }

    #[test]
    fn read_rm_emits_weak_etag_and_no_location() {
        use openehr_rm::prelude::Composition;
        let value = serde_json::json!({"_type": "COMPOSITION"});
        let resp = ServiceResponse::new(value, meta("e1", "v::s::7"));
        let out = read_rm::<Composition>(
            &HeaderMap::new(),
            BASE,
            Some("composition"),
            &resp,
            "composition",
        );
        assert_eq!(out.status(), StatusCode::OK);
        assert_eq!(etag(&out).as_deref(), Some("W/\"v::s::7\""));
        assert!(loc(&out).is_none());
    }

    #[test]
    fn resource_etag_is_weak() {
        let v = resource_etag("8849182c::openEHRSys::2").expect("etag");
        assert_eq!(v.to_str().unwrap(), "W/\"8849182c::openEHRSys::2\"");
    }

    #[test]
    fn prefers_identifier_detects_return_identifier() {
        assert!(prefers_identifier(&headers(&[(
            "prefer",
            "return=identifier"
        )])));
        assert!(prefers_identifier(&headers(&[(
            "prefer",
            "RETURN=IDENTIFIER"
        )])));
        assert!(!prefers_identifier(&headers(&[(
            "prefer",
            "return=minimal"
        )])));
        assert!(!prefers_identifier(&HeaderMap::new()));
    }

    #[tokio::test]
    async fn write_rm_identifier_returns_uid_body() {
        use http_body_util::BodyExt;
        use openehr_rm::prelude::Composition;
        let value = serde_json::json!({"_type": "COMPOSITION"});
        let resp = ServiceResponse::new(value, meta("e1", "v::s::9"));
        let h = headers(&[("prefer", "return=identifier")]);
        let out = write_rm::<Composition>(
            &h,
            BASE,
            StatusCode::NO_CONTENT,
            StatusCode::OK,
            Some("composition"),
            &resp,
            "composition",
        );
        assert_eq!(out.status(), StatusCode::OK);
        assert_eq!(content_type(&out).as_deref(), Some(APPLICATION_JSON));
        assert_eq!(etag(&out).as_deref(), Some("W/\"v::s::9\""));
        assert_eq!(
            preference_applied(&out).as_deref(),
            Some("return=identifier")
        );
        let bytes = out.into_body().collect().await.expect("body").to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(body, serde_json::json!({ "uid": "v::s::9" }));
    }

    #[test]
    fn http_date_is_imf_fixdate() {
        let ts: jiff::Timestamp = "2009-07-22T19:15:56Z".parse().unwrap();
        assert_eq!(http_date(ts), "Wed, 22 Jul 2009 19:15:56 GMT");
    }

    #[test]
    fn set_resource_headers_emits_last_modified() {
        let ts: jiff::Timestamp = "2024-03-04T05:06:07Z".parse().unwrap();
        let m = ResourceMeta::new("e1".to_owned(), "v::s::1".to_owned()).with_last_modified(ts);
        let mut resp = empty(StatusCode::OK);
        set_resource_headers(&mut resp, BASE, Some("composition"), &m);
        let lm = resp
            .headers()
            .get(header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok());
        assert_eq!(lm, Some("Mon, 04 Mar 2024 05:06:07 GMT"));
    }

    #[test]
    fn set_resource_headers_omits_last_modified_when_absent() {
        let m = ResourceMeta::new("e1".to_owned(), "v::s::1".to_owned());
        let mut resp = empty(StatusCode::OK);
        set_resource_headers(&mut resp, BASE, Some("composition"), &m);
        assert!(resp.headers().get(header::LAST_MODIFIED).is_none());
    }
}
