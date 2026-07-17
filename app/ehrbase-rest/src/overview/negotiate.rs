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
//! - **XML responses** for the spec-typed RM objects are served by
//!   [`respond_rm`]: the handler returns canonical JSON as usual, and for an XML
//!   `Accept` the value is re-typed into its concrete `openehr-rm` type at the
//!   response edge so the generated `ToXml` runs — the mirror of the [`rm_value`]
//!   request path. This covers the single objects (composition, `ehr_status`,
//!   ehr, folder) and the VERSION family — `ORIGINAL_VERSION<T>`,
//!   `VERSIONED_OBJECT`, `REVISION_HISTORY` — whose canonical-XML shape ITS-XML
//!   (`Version.xsd`/`Common.xsd`) defines and `emit-xml` generates.
//!   Responses that are genuinely not a spec-typed RM value (collections, item
//!   tags, terminology/query DTOs, the CONTRIBUTION wire DTO) have no
//!   spec-defined canonical-XML shape and stay JSON-only via [`respond`].

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode, header};
use serde::Serialize;
use serde::de::DeserializeOwned;

use openehr_its::rest::runtime::ApiError;
use openehr_its::xml::{FromXml, ToXml};

use ehrbase::service::response::{ResourceMeta, ServiceResponse};

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

/// Format a commit timestamp as an HTTP-date (RFC 7231 IMF-fixdate, always GMT)
/// for the `Last-Modified` response header, e.g. `Wed, 22 Jul 2009 19:15:56 GMT`.
fn http_date(at: jiff::Timestamp) -> String {
    // `Timestamp` formats in UTC; the fixed English weekday/month abbreviations
    // and `GMT` zone give the IMF-fixdate the spec's example shows.
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
/// not a spec-typed RM value (`serde_json::Value` collections and DTOs: item
/// tags, terminology/query results, the CONTRIBUTION wire DTO); if the client
/// requested XML exclusively, this returns 406 since those payloads have no
/// spec-defined canonical-XML shape. Spec-typed RM objects — including the
/// VERSION family — use [`respond_rm`] instead.
pub(crate) fn respond<T: Serialize>(
    headers: &HeaderMap,
    status: StatusCode,
    value: &T,
) -> Response {
    match response_format(headers) {
        Format::Json => json_response(status, value),
        Format::Xml => ApiError::NotAcceptable(
            "this response has no canonical-XML representation; request application/json"
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

// ── ITS-REST response-header + `Prefer` handling ─────────────────────
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

/// Whether the client asked for `OBJECT_REF` resolution on `Prefer`
/// (`resolve_refs` — ITS-REST `Requests_and_responses` §Representation details
/// negotiation: "services that implement `OBJECT_REF` resolution SHOULD accept
/// and honour it").
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
/// (`return=identifier`). Overview §"Prefer only identifier": a minimal response
/// with a non-empty body containing only the affected resource's identifier
/// (`{ "uid": … }`); status is `200 OK` or `201 Created`, never `204`.
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
/// response header (overview §"Representation details negotiation": the service
/// MAY confirm the honoured preference). Precedence mirrors the body branch in
/// [`write_rm`]: representation, then identifier, then the `minimal` default.
fn applied_preference(headers: &HeaderMap) -> &'static str {
    if prefers_representation(headers) {
        "representation"
    } else if prefers_identifier(headers) {
        "identifier"
    } else {
        "minimal"
    }
}

/// Emit `Preference-Applied: return=<kind>` on a write response (overview
/// §"Representation details negotiation"; example line 147). A MAY — a courtesy
/// so a client can detect which preference the server honoured, which matters
/// once the default shifts toward `identifier` (§"Deprecated headers").
fn set_preference_applied(resp: &mut Response, kind: &str) {
    if let Ok(value) = HeaderValue::from_str(&format!("return={kind}")) {
        resp.headers_mut()
            .insert(header::HeaderName::from_static("preference-applied"), value);
    }
}

/// The status for a `return=identifier` write: `201`/`200`, never `204`
/// (overview §"Prefer only identifier"). A create keeps its `minimal_status`
/// (`201`); an update whose minimal status is `204 No Content` uses the
/// representation status (`200 OK`) so the identifier body is not dropped.
fn identifier_status(minimal_status: StatusCode, repr_status: StatusCode) -> StatusCode {
    if minimal_status == StatusCode::NO_CONTENT {
        repr_status
    } else {
        minimal_status
    }
}

/// Render a `return=identifier` response body: `{ "uid": "<uid>" }` in JSON
/// (overview §"Prefer only identifier", example lines 313–319), or the `<uid>`
/// element as the XML equivalent when XML is negotiated. The identifier is the
/// affected resource's `version_uid` (`resp.meta.uid`).
fn identifier_response(headers: &HeaderMap, status: StatusCode, uid: &str) -> Response {
    match response_format(headers) {
        Format::Json => json_response(status, &serde_json::json!({ "uid": uid })),
        Format::Xml => {
            // The OAS defines the identifier body only for JSON; the spec is
            // silent on an XML shape for a bare identifier. We emit a minimal
            // `<uid>` element as the direct XML equivalent of the JSON `{uid}`.
            xml_body(status, format!("<uid>{}</uid>", xml_escape(uid)))
        }
    }
}

/// Escape the XML text-content special characters. `OBJECT_VERSION_ID` values do
/// not contain these, but escape defensively.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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

/// The `ETag` header value for a resource identifier: the weak form
/// `W/"{uid}"`. The overview §"`ETag` and Last-Modified" makes the weak indicator
/// a **MUST**: "all `ETag` headers that hold a resource identifier MUST include
/// a weakness indicator `W/`" (§"Deprecated headers"). The `ETag` value is
/// independent of the JSON/XML serialization, hence weak-typed.
pub(crate) fn resource_etag(uid: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!("W/\"{uid}\"")).ok()
}

/// Set the versioning headers on a response: the weak `ETag`
/// ([`resource_etag`]) and — when the metadata carries a commit time —
/// `Last-Modified` (the version commit time
/// `VERSION.commit_audit.time_committed.value`, overview §"`ETag` and
/// Last-Modified", SHOULD-present on `VERSION`/`VERSIONED_OBJECT` responses).
/// **No `Location`** — that is create/redirect-only ([`set_location`], overview
/// §Location). Used by reads, deletes, and the `409`/`412` error path.
pub(crate) fn set_versioning_headers(resp: &mut Response, meta: &ResourceMeta) {
    if let Some(etag) = resource_etag(&meta.uid) {
        resp.headers_mut().insert(header::ETAG, etag);
    }
    if let Some(at) = meta.last_modified
        && let Ok(lm) = HeaderValue::from_str(&http_date(at))
    {
        resp.headers_mut().insert(header::LAST_MODIFIED, lm);
    }
    // The single, generic ATNA hook for the participant object: surface the
    // resource ids the envelope already carries for the audit layer (§8.2 step 3).
    resp.extensions_mut()
        .insert(crate::system_log::middleware::AuditObject {
            ehr_id: Some(meta.ehr_id.clone()),
            uid: Some(meta.uid.clone()),
        });
}

/// Set the `Location` header for a newly created/updated resource. The overview
/// §Location: "The `Location` header MUST ONLY be used for resource creation
/// (e.g., `201 Created`) or redirect responses" — never to indicate an
/// alternate representation of an existing resource (a `GET`), and it is
/// deprecated from `DELETE` responses.
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

/// Set the full create/update response headers: the versioning headers
/// ([`set_versioning_headers`]) **plus** `Location` ([`set_location`]). Used by
/// the write paths ([`write_rm`]/[`write_json`]) and the create responses of the
/// extension surfaces. Reads and deletes use [`set_versioning_headers`] alone,
/// so they no longer emit `Location` (overview §Location).
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
/// versioning + `Location` headers. `return=representation` → `repr_status` with
/// the RM body (JSON or canonical XML via `T`); `return=identifier` → an
/// identifier-only body (`{ "uid": … }`) at a `200`/`201` status; the default
/// `return=minimal` → `minimal_status` with no body. `Preference-Applied` echoes
/// the honoured preference. `segment` is the `Location` resource collection.
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

/// Render a `200 OK` read of a single spec-typed RM object, additionally setting
/// the weak `ETag`/`Last-Modified` the operation's spec declares (e.g.
/// `200_COMPOSITION_retrieved.yaml`, `200_EHR_STATUS_retrieved.yaml`). No
/// `Location`: it "MUST NOT be used to indicate an alternate representation of
/// an existing resource (e.g. via `GET` method)" (overview §Location).
///
/// `base_path`/`segment` are retained in the signature for the dispatch call
/// sites; a read emits no `Location` so they are currently unused here.
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
    let _ = (base_path, segment);
    let mut out = respond_rm::<T>(headers, StatusCode::OK, &resp.body, root_tag);
    if let Some(meta) = &resp.meta {
        set_versioning_headers(&mut out, meta);
    }
    out
}

/// A `204 No Content` delete outcome carrying the deleted version's weak
/// `ETag`/`Last-Modified` (`204_COMPOSITION_deleted.yaml`). No `Location`: it
/// "was deprecated from responses of `DELETE` methods" (overview §Location).
///
/// `base_path`/`segment` are retained in the signature for the dispatch call
/// sites; a delete emits no `Location` so they are currently unused here.
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
/// spec requires on a `409`/`412` (the current `version_uid`;
/// `409_COMPOSITION_with_uid_based_id.yaml`, `412_*.yaml`). The overview §If-Match
/// asks only for the latest `version_uid` "in the `ETag` response headers" on a
/// false precondition — no `Location` on the error path.
///
/// `base_path`/`segment` are retained in the signature for the dispatch call
/// sites; the error path emits no `Location` so they are currently unused here.
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

    #[tokio::test]
    async fn respond_rm_renders_original_version_xml_with_signature() {
        // F-05-06 / ECC-SIG-001: an ORIGINAL_VERSION response is served as
        // canonical XML (its `ToXml` exists), carrying the `<signature>` element.
        // `OriginalVersion<T>` is generic — `DvText` stands in for the versioned
        // root here; the dispatch uses `OriginalVersion<Composition>`.
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
        // F-05-06 / ECC-COM-022: the VERSIONED_OBJECT container serves as XML.
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

    fn preference_applied(resp: &Response) -> Option<String> {
        resp.headers()
            .get("preference-applied")
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
        // the ETag carries the mandatory weak `W/` indicator.
        assert_eq!(etag(&out).as_deref(), Some("W/\"v::s::1\""));
        // A create keeps `Location` (overview §Location: creation-only).
        assert_eq!(
            loc(&out).as_deref(),
            Some(&*format!("{BASE}/ehr/e1/composition/v::s::1"))
        );
        // Minimal → no content-type body header.
        assert_eq!(content_type(&out), None);
        // `Preference-Applied` echoes the honoured (default) preference.
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
        // Representation → 200 (repr status) with a JSON body + headers.
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
        // `Location` is deprecated from DELETE responses.
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
        // §If-Match: the latest `version_uid` goes in the `ETag`, weak-form.
        assert_eq!(etag(&out).as_deref(), Some("W/\"v::s::5\""));
        // No `Location` on the error path.
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
        // reads MUST NOT carry `Location` (overview §Location).
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
        // Update semantics (minimal=204) → identifier promotes to 200, never 204.
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
        // The overview spec's example: `Wed, 22 Jul 2009 19:15:56 GMT`.
        let ts: jiff::Timestamp = "2009-07-22T19:15:56Z".parse().unwrap();
        assert_eq!(http_date(ts), "Wed, 22 Jul 2009 19:15:56 GMT");
    }

    #[test]
    fn set_resource_headers_emits_last_modified() {
        // §"ETag and Last-Modified": a versioned resource's commit time is
        // surfaced as `Last-Modified` alongside `ETag`/`Location`.
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
