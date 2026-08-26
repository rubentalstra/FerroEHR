// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Operation bindings — the wire layer, one file per SM operation per ITS.
//!
//! A binding maps request construction, each outcome kind → wire
//! expectation, and each logical capture → wire source; every mapping cites
//! its OAS source (ITS-REST 1.1.0 `specifications/operations/*.yaml` +
//! `responses/*.yaml`). The capture-source, header-matcher, and
//! body-selector vocabularies are closed.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

use crate::ids::{CaptureName, SmOperationRef};
use crate::model::case::Applies;
use crate::refgrammar::Template;
use crate::vocab::{FormatName, HttpMethod, ItsName, OutcomeKind};

/// A wire location a capture reads from (closed grammar):
/// `header <Name>` · `header <Name> last-segment` · `body "<path>"` ·
/// `capture <name>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireFrom {
    /// `header <Name>` — a response header's value.
    Header {
        /// The response header field name.
        name: String,
        /// Take only the last `/`-separated segment of the header value.
        last_segment: bool,
    },
    /// `body "<path>"` — a value inside the response body.
    Body {
        /// The path addressing the value inside the response body.
        path: String,
    },
    /// Derive from another capture.
    Capture(CaptureName),
}

impl WireFrom {
    /// Parse the closed source grammar.
    ///
    /// # Errors
    /// Returns a message when the text is outside the grammar.
    pub fn parse(raw: &str) -> Result<Self, String> {
        if let Some(rest) = raw.strip_prefix("header ") {
            let (name, last_segment) = match rest.strip_suffix(" last-segment") {
                Some(name) => (name, true),
                None => (rest, false),
            };
            let name = name.trim();
            if name.is_empty() || name.contains(char::is_whitespace) {
                return Err(format!(
                    "capture source {raw:?}: header name must be one token"
                ));
            }
            return Ok(Self::Header {
                name: name.to_owned(),
                last_segment,
            });
        }
        if let Some(rest) = raw.strip_prefix("body ") {
            let path = rest
                .trim()
                .strip_prefix('"')
                .and_then(|p| p.strip_suffix('"'))
                .ok_or_else(|| format!("capture source {raw:?}: body path must be quoted"))?;
            if path.is_empty() {
                return Err(format!("capture source {raw:?}: empty body path"));
            }
            return Ok(Self::Body {
                path: path.to_owned(),
            });
        }
        if let Some(rest) = raw.strip_prefix("capture ") {
            return CaptureName::parse(rest.trim())
                .map(Self::Capture)
                .map_err(|e| format!("capture source {raw:?}: {e}"));
        }
        Err(format!(
            "capture source {raw:?} is outside the closed grammar (header … | body \"…\" | capture …)"
        ))
    }
}

impl<'de> Deserialize<'de> for WireFrom {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(D::Error::custom)
    }
}

/// Post-extraction modifier: strip the weak-ETag wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum StripRule {
    /// Unwrap a weak `ETag` (`W/"…"`) down to the bare entity tag.
    #[serde(rename = "weak-quotes")]
    WeakQuotes,
}

/// Post-extraction transform over a captured value. The grammar is closed;
/// every member addresses a component of the value the wire actually carries.
///
/// `root-uid` and `creating-system-id` decompose an `OBJECT_VERSION_ID`,
/// whose lexical form the ITS-REST overview fixes as
/// `object_id :: creating_system_id :: version_tree_id`
/// (`Resources.md` §Identifier types: "The `version_uid` uniquely identifies
/// a VERSION, in the lexical form of `object_id :: creating_system_id ::
/// version_tree_id`"). `uppercase` exists so a case can author a
/// case-VARIANT of a captured identifier (e.g. an `If-Match` naming the same
/// version in different case — BASE `master05` §"Composite Identifiers and
/// Case" makes two identifiers "identical apart from case … identify the
/// same thing"), which the reference grammar itself cannot express
/// (issue #403).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum TransformRule {
    /// The leading `object_id` — the `VERSIONED_OBJECT` identifier.
    #[serde(rename = "root-uid")]
    RootUid,
    /// The MIDDLE segment — the identifier of the system that created the
    /// version. Yields nothing when the value carries no middle segment, so
    /// a truncated identifier leaves the capture unbound (loud downstream)
    /// rather than binding the whole value as if it were a system id.
    #[serde(rename = "creating-system-id")]
    CreatingSystemId,
    /// The captured value, ASCII-uppercased.
    #[serde(rename = "uppercase")]
    Uppercase,
}

impl TransformRule {
    /// Apply the transform, or `None` when the value has no such component.
    #[must_use]
    pub fn apply(self, value: &str) -> Option<String> {
        let mut segments = value.split("::");
        match self {
            Self::RootUid => segments.next().map(ToOwned::to_owned),
            Self::CreatingSystemId => {
                let _object_id = segments.next()?;
                segments
                    .next()
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
            }
            Self::Uppercase => Some(value.to_ascii_uppercase()),
        }
    }

    /// All members, in grammar order (schema emission derives from this).
    pub const ALL: &[TransformRule] = &[
        TransformRule::RootUid,
        TransformRule::CreatingSystemId,
        TransformRule::Uppercase,
    ];
}

/// One logical-capture wire mapping with optional modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireCapture {
    /// The wire location the value is read from.
    pub from: WireFrom,
    /// Wrapper removed from the extracted value, if any.
    #[serde(default)]
    pub strip: Option<StripRule>,
    /// Component/case transform applied to the extracted value, if any.
    #[serde(default)]
    pub transform: Option<TransformRule>,
    /// Tried when `from` yields nothing (e.g. body field under
    /// `Prefer: return=minimal`, falling back to `Location`).
    #[serde(default)]
    pub fallback: Option<WireFrom>,
}

/// The value-side of a header expectation (closed matcher vocabulary):
/// `present` · `absent` · `negotiated` · `latest-version-uid` ·
/// `pattern:<regex>` · a literal string.
///
/// `present?` is NOT a member: it is the authored shorthand for
/// [`HeaderExpectation`]'s `{ match: present, optional: true }`, so
/// presence-optionality is one modifier with one meaning instead of a matcher
/// that silently means something different from every other one.
#[derive(Debug, Clone, PartialEq)]
pub enum HeaderMatcher {
    /// The header must be present, with any value.
    Present,
    /// The header must not be present.
    Absent,
    /// Equals the negotiated media type.
    Negotiated,
    /// The stale-precondition 412 rule: `ETag` carries the latest version uid.
    LatestVersionUid,
    /// A pattern; `<name>` placeholders resolve from case variables before
    /// matching (validated by compiling with placeholders wildcarded).
    Pattern(String),
    /// A literal value template.
    Literal(Template),
}

impl HeaderMatcher {
    /// Parse the closed matcher vocabulary.
    ///
    /// # Errors
    /// Returns a message when the pattern does not compile, when a literal
    /// template is malformed, or when the reserved `present?` shorthand is
    /// used where only a matcher belongs.
    pub fn parse(raw: &str) -> Result<Self, String> {
        Ok(match raw {
            "present" => Self::Present,
            "present?" => {
                return Err(
                    "`present?` is the shorthand for `{ match: present, optional: true }` — \
                     in the mapping form declare `optional: true` instead"
                        .to_owned(),
                );
            }
            "absent" => Self::Absent,
            "negotiated" => Self::Negotiated,
            "latest-version-uid" => Self::LatestVersionUid,
            _ => {
                if let Some(pattern) = raw.strip_prefix("pattern:") {
                    let probe = placeholder_wildcarded(pattern);
                    regex::Regex::new(&probe)
                        .map_err(|e| format!("header pattern {pattern:?} does not compile: {e}"))?;
                    Self::Pattern(pattern.to_owned())
                } else {
                    Self::Literal(Template::parse(raw)?)
                }
            }
        })
    }
}

impl<'de> Deserialize<'de> for HeaderMatcher {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(D::Error::custom)
    }
}

/// One declared header expectation on a wire outcome: a [`HeaderMatcher`]
/// plus the two modifiers a released spec sentence can put on it.
///
/// The authored short form is a bare matcher string (`ETag: present`,
/// `Location: absent`) — unconditional, presence-asserting. The mapping form
/// declares the modifiers:
///
/// ```yaml
/// ETag:
///   match: 'pattern:W/"[^"]+"'
///   optional: true                     # presence is a SHOULD; the form is a MUST
///   applies: { its_rest: ">=1.1.0" }   # the release the rule is dated to
/// ```
///
/// **`optional`** separates the strength of PRESENCE from the strength of
/// FORM, because the released text assigns them differently. ITS-REST
/// `specifications/docs/overview/Requests_and_responses.md` §"`ETag` and
/// Last-Modified": "Both `ETag` and `Last-Modified` SHOULD be included in
/// responses for VERSION, `VERSIONED_OBJECT`, or other resources that have
/// versioning or unique state identifiers" — presence is a SHOULD; while
/// §"Deprecated headers" makes the form a MUST: "all `ETag` headers that hold
/// a resource identifier MUST include a weakness indicator `W/`". An optional
/// expectation is skipped entirely when the header is absent (or blank) and
/// judged in full when it is there, so a SHOULD is never enforced as a MUST
/// and a MUST is never lost. `present?` is the authored shorthand for
/// `{ match: present, optional: true }`.
///
/// **`applies`** carries a version floor for the RULE, because a released
/// requirement can be dated by its own text. The same overview chapter dates
/// two of them to Release 1.1.0: §"Deprecated headers" ("The `ETag` response
/// header was used without a weakness indicator `W/`. This is now deprecated,
/// all `ETag` headers that hold a resource identifier MUST include a weakness
/// indicator `W/`") with §"`ETag` and Last-Modified" naming the release
/// ("DEPRECATION: Prior to Release 1.1.0, the `ETag` header was used without a
/// weakness indicator `W/`"), and §Location ("DEPRECATION: Prior to Release
/// 1.1.0, the `Location` header was used to indicate the canonical location of
/// a representation in a response"). A party declaring an earlier ITS-REST
/// release conforms to the text of THAT release, so a dated matcher is not
/// applied to it — while the operation itself is still driven and every other
/// expectation on the outcome still bites. The floor belongs HERE and not on
/// the case or on the binding: it is the header rule that the release dates,
/// not the operation, and putting it a level up would take a party out of
/// scope for behaviour it does implement.
#[derive(Debug, Clone)]
pub struct HeaderExpectation {
    /// The value-side matcher.
    pub matcher: HeaderMatcher,
    /// Presence is not asserted: an absent (or blank) header satisfies the
    /// expectation, a present one is judged by `matcher`.
    pub optional: bool,
    /// The spec-version floor the RULE is dated to; unsatisfied ⇒ the
    /// expectation is out of scope for this party and not judged.
    pub applies: Option<Applies>,
}

impl HeaderExpectation {
    /// An unconditional, presence-asserting expectation around `matcher`.
    #[must_use]
    pub fn new(matcher: HeaderMatcher) -> Self {
        Self {
            matcher,
            optional: false,
            applies: None,
        }
    }
}

impl<'de> Deserialize<'de> for HeaderExpectation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Mapping {
            #[serde(rename = "match")]
            matcher: HeaderMatcher,
            #[serde(default)]
            optional: bool,
            #[serde(default)]
            applies: Option<Applies>,
        }
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => {
                if s == "present?" {
                    return Ok(Self {
                        matcher: HeaderMatcher::Present,
                        optional: true,
                        applies: None,
                    });
                }
                HeaderMatcher::parse(&s)
                    .map(Self::new)
                    .map_err(D::Error::custom)
            }
            value @ serde_json::Value::Object(_) => {
                let mapping: Mapping = serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(Self {
                    matcher: mapping.matcher,
                    optional: mapping.optional,
                    applies: mapping.applies,
                })
            }
            other => Err(D::Error::custom(format!(
                "a header expectation is a matcher string or a \
                 {{ match, optional?, applies? }} mapping, got {other}"
            ))),
        }
    }
}

/// The `<name>` placeholders a matcher pattern declares, in order.
///
/// The compile probe below only proves the WILDCARDED form parses as a regex;
/// naming the placeholders is what lets a gate check they can ever resolve
/// (`crate::validate`).
#[must_use]
pub fn placeholder_names(pattern: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = pattern;
    while let Some(start) = rest.find('<') {
        let tail = rest.split_at(start).1;
        let Some(end) = tail.find('>') else { break };
        if let Some(name) = tail.get(1..end) {
            names.push(name);
        }
        rest = tail.get(end + 1..).unwrap_or_default();
    }
    names
}

/// Replace `<name>` placeholders with `.*` for the compile probe — the
/// pattern text is a regex (`pattern:<regex>`), so everything else is left
/// verbatim and must compile.
fn placeholder_wildcarded(pattern: &str) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(start) = rest.find('<') {
        let (head, tail) = rest.split_at(start);
        out.push_str(head);
        if let Some(end) = tail.find('>') {
            out.push_str(".*");
            rest = tail.get(end + 1..).unwrap_or_default();
        } else {
            out.push_str(tail);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// A response-body expectation (closed selector vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BodySelector {
    /// Full resource | `{uid}` | empty, per `Prefer`
    /// (ITS-REST `Requests_and_responses.md` §Prefer).
    #[serde(rename = "prefer_conditional")]
    PreferConditional,
    /// AMB-1: assert at most that a `message` string is present.
    #[serde(rename = "error_loose")]
    ErrorLoose,
    /// The `RESULT_SET` schema (named distinctly from the `result_set`
    /// assertion).
    #[serde(rename = "result_set_body")]
    ResultSetBody,
    /// Body media type equals the negotiated type.
    #[serde(rename = "negotiated")]
    Negotiated,
    /// A body must be present, of any shape.
    #[serde(rename = "present")]
    Present,
    /// The response must carry no body.
    #[serde(rename = "absent")]
    Absent,
}

/// One outcome kind's wire expectation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireExpectation {
    /// The status code the outcome is primarily served with.
    pub status: StatusCode,
    /// Additional non-conflicting status codes the overview permits beyond
    /// the operation's OAS enumeration (ITS-REST `Requests_and_responses.md`
    /// §HTTP status codes: "Additional status codes MAY be used as long as
    /// they do not conflict with the predefined codes"). Each entry's YAML
    /// carries the citation; an observed alt status classifies as this kind
    /// exactly like the primary.
    #[serde(default)]
    pub alt_status: Option<Vec<StatusCode>>,
    /// Per-header expectations, in declaration order.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub headers: Option<Vec<(String, HeaderExpectation)>>,
    /// The body shape the outcome must carry.
    #[serde(default)]
    pub body: Option<BodySelector>,
}

/// An HTTP status code, range-checked at parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(u16);

impl StatusCode {
    /// The numeric code.
    #[must_use]
    pub fn value(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for StatusCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = u16::deserialize(deserializer)?;
        if (100..=599).contains(&raw) {
            Ok(Self(raw))
        } else {
            Err(D::Error::custom(format!("status {raw} outside 100..=599")))
        }
    }
}

/// The request payload of a binding.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestBody {
    /// A named payload role (`composition`, `opt_xml`), optionally optional
    /// (`ehr_status?`).
    Named {
        /// The payload role name the corpus resolves.
        name: String,
        /// The request is legal without this body.
        optional: bool,
    },
    /// A structured body template (the query binding's
    /// `{ q: ${q}, offset: ${offset?} … }`).
    Structured(crate::model::value::TemplatedValue),
    /// The read-modify-write realization of an SM field-setter (AMB-15): the
    /// body is a captured resource with the named fields overwritten.
    Patched {
        /// The case capture holding the current resource body.
        from_capture: CaptureName,
        /// Field overwrites (top-level attribute → literal value).
        set: Vec<(String, serde_json::Value)>,
    },
}

impl<'de> Deserialize<'de> for RequestBody {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        // The patched form: { from_capture: <name>, set: { field: value } }.
        if let serde_json::Value::Object(map) = &value
            && map.contains_key("from_capture")
        {
            let from = map
                .get("from_capture")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| D::Error::custom("from_capture must be a capture name"))?;
            let from_capture = CaptureName::parse(from).map_err(D::Error::custom)?;
            let set = map
                .get("set")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| D::Error::custom("patched body requires a set mapping"))?
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            if map.len() != 2 {
                return Err(D::Error::custom(
                    "patched body carries exactly from_capture + set",
                ));
            }
            return Ok(Self::Patched { from_capture, set });
        }
        match &value {
            serde_json::Value::String(s) => {
                let (name, optional) = match s.strip_suffix('?') {
                    Some(name) => (name, true),
                    None => (s.as_str(), false),
                };
                if name.is_empty() || name.contains(char::is_whitespace) {
                    return Err(D::Error::custom(format!(
                        "request body role {s:?} must be one token"
                    )));
                }
                Ok(Self::Named {
                    name: name.to_owned(),
                    optional,
                })
            }
            serde_json::Value::Object(_) => crate::model::value::TemplatedValue::from_value(&value)
                .map(Self::Structured)
                .map_err(D::Error::custom),
            _ => Err(D::Error::custom(
                "request body must be a role name or a structured template",
            )),
        }
    }
}

/// One query parameter's authored value.
///
/// A scalar template is the single-valued form: one `name=value` pair, or
/// none when its optional reference (`${x?}`) is unbound. A YAML **sequence**
/// of templates is the repeated (RFC 6570 exploded, `{?p*}`) form: each
/// member contributes its own `name=value` pair, in authored order, and a
/// member whose optional reference is unbound is simply absent — so one
/// authored sequence serves every arity up to its length. A member that
/// resolves to a LIST capture expands element-wise.
///
/// Repeatability is declared HERE, in the wire layer, and never inferred
/// from what a case happens to bind: a case core speaks SM operations and
/// outcome kinds only, so it must not be able to change the serialization
/// form of a request by passing a list. The one released use is the admin
/// bulk delete's subset selector (`/admin/ehr/all{?ehr_id*}`).
#[derive(Debug, Clone, PartialEq)]
pub enum QueryValue {
    /// One pair (the single-valued form).
    Single(Template),
    /// One pair per member (the repeated form).
    Repeated(Vec<Template>),
}

impl QueryValue {
    /// The authored templates, in order (one for [`Self::Single`]).
    #[must_use]
    pub fn templates(&self) -> &[Template] {
        match self {
            Self::Single(template) => std::slice::from_ref(template),
            Self::Repeated(templates) => templates,
        }
    }

    /// Whether the parameter is authored in the repeated form.
    #[must_use]
    pub fn is_repeated(&self) -> bool {
        matches!(self, Self::Repeated(_))
    }
}

impl<'de> Deserialize<'de> for QueryValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => Template::parse(&s)
                .map(Self::Single)
                .map_err(D::Error::custom),
            serde_json::Value::Array(items) => {
                if items.is_empty() {
                    return Err(D::Error::custom(
                        "a repeated query parameter declares at least one member",
                    ));
                }
                items
                    .iter()
                    .map(|item| match item {
                        serde_json::Value::String(s) => {
                            Template::parse(s).map_err(D::Error::custom)
                        }
                        _ => Err(D::Error::custom(
                            "a repeated query parameter's members are value templates",
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(Self::Repeated)
            }
            _ => Err(D::Error::custom(
                "a query parameter is a value template or a sequence of them",
            )),
        }
    }
}

/// A request path with `{param}` placeholders resolved from case variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathTemplate {
    raw: String,
    params: Vec<CaptureName>,
}

impl PathTemplate {
    /// The authored path.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The `{param}` names, in order.
    #[must_use]
    pub fn params(&self) -> &[CaptureName] {
        &self.params
    }
}

impl<'de> Deserialize<'de> for PathTemplate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        if !raw.starts_with('/') {
            return Err(D::Error::custom(format!("path {raw:?} must start with /")));
        }
        let mut params = Vec::new();
        let mut rest = raw.as_str();
        while let Some(start) = rest.find('{') {
            let tail = rest.get(start + 1..).unwrap_or_default();
            let end = tail
                .find('}')
                .ok_or_else(|| D::Error::custom(format!("path {raw:?}: unterminated {{param}}")))?;
            let name = tail.get(..end).unwrap_or_default();
            params.push(CaptureName::parse(name).map_err(D::Error::custom)?);
            rest = tail.get(end + 1..).unwrap_or_default();
        }
        Ok(Self { raw, params })
    }
}

/// Request construction.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestSpec {
    /// The HTTP method the operation is driven with.
    pub method: HttpMethod,
    /// The request path, with `${…}` placeholders resolved per step.
    pub path: PathTemplate,
    /// Query parameters (name → value; optional refs `${x?}` omit the
    /// parameter when unresolved, a sequence declares the repeated form —
    /// see [`QueryValue`]).
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub query: Option<Vec<(String, QueryValue)>>,
    /// The request payload, when the operation carries one.
    #[serde(default)]
    pub body: Option<RequestBody>,
    /// Request headers, in declaration order.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub headers: Option<Vec<(String, Template)>>,
}

/// A per-format extra-header requirement (`openehr-template-id: required`).
#[derive(Debug, Clone, PartialEq)]
pub enum FormatHeaderReq {
    /// The header must be sent; its value comes from the case.
    Required,
    /// The header is sent with this fixed (templated) value.
    Literal(Template),
}

impl<'de> Deserialize<'de> for FormatHeaderReq {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s == "required" {
            Ok(Self::Required)
        } else {
            Template::parse(&s)
                .map(Self::Literal)
                .map_err(D::Error::custom)
        }
    }
}

/// An explicit unrealized-operation declaration.
///
/// The ITS surfaces no wire for this SM operation, so cases anchored to it are
/// `not-applicable` with this citation on this ITS (machine-readable, never
/// silent absence).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnrealizedDecl {
    /// Why the ITS cannot carry the operation.
    pub reason: String,
    /// The spec citation for the gap.
    pub source: String,
    /// The ambiguity-register entry tracking the gap.
    pub ambiguity: crate::ids::AmbiguityId,
}

/// An EXTENSION realization: the operation is driven over a route no
/// openEHR specification governs — our own design/extension, declared as a
/// family of `vocab/wire_surface.yaml` `served_extensions`.
///
/// A binding carrying this block is still a full wire realization (request +
/// outcomes, executed like any other), but it is fenced off from every
/// released-wire judgement: the released-path claim check skips it, and the
/// capabilities its cases carry must be `realization: extension` in the
/// capability matrix, which may never be `required`. Wire-level ITS-REST
/// conformance therefore never rests on it; only the CAPABILITY verdict does.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionDecl {
    /// The `served_extensions` family that declares the route (resolved).
    pub family: String,
    /// Why the released ITS surfaces no wire for the operation, and what
    /// this product serves instead.
    pub reason: String,
    /// The spec citation for the gap + the explicit spec-silence flag.
    pub source: String,
    /// The ambiguity-register entry adjudicating the boundary.
    pub ambiguity: crate::ids::AmbiguityId,
}

/// One binding file: a full wire realization (released or, with an
/// `extension` block, over a declared extension route), or an explicit
/// `unrealized` declaration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationBinding {
    /// The SM operation this file realizes on the wire.
    pub sm_operation: SmOperationRef,
    /// The implementation technology specification the realization targets.
    pub its: ItsName,
    /// Realization discriminator when several bindings share one
    /// `sm_operation` (e.g. the plain `get_opt` OPT GET vs the `example`
    /// data-generation realization): a flow step's `variant` selects the
    /// matching binding; a variant-less step selects the variant-less
    /// binding. Absent for the sole realization of an operation.
    #[serde(default)]
    pub variant: Option<String>,
    /// The OPERATION's spec-version floor: the releases on which this wire
    /// realization exists at all (an endpoint, request header spelling, or
    /// request shape a later release introduced). ENFORCED at selection time
    /// (`crate::run`): a case whose flow drives a binding whose floor the
    /// party's declared versions do not satisfy is not-applicable with that
    /// citation, never driven against a server the release never asked to
    /// serve it.
    ///
    /// Distinct from [`HeaderExpectation::applies`], which dates one RESPONSE
    /// expectation rather than the operation: a release that only changed how
    /// an answer must look leaves the operation itself in scope, so the floor
    /// goes on the matcher and the case still runs.
    #[serde(default)]
    pub applies: Option<Applies>,
    /// Present instead of `request` when the released ITS publishes no wire
    /// for the operation at all.
    #[serde(default)]
    pub unrealized: Option<UnrealizedDecl>,
    /// Present when the realization drives a declared extension route
    /// instead of a released ITS-REST operation ([`ExtensionDecl`]).
    #[serde(default)]
    pub extension: Option<ExtensionDecl>,
    /// How the request is constructed (absent only for an `unrealized` file).
    #[serde(default)]
    pub request: Option<RequestSpec>,
    /// The wire formats this operation is served in.
    #[serde(default)]
    pub formats: Vec<FormatName>,
    /// Extra headers a given format requires, keyed by format.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub format_headers: Option<Vec<(FormatKey, FormatHeaderMap)>>,
    /// The wire expectation per outcome kind, in declaration order.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub outcomes: Option<Vec<(OutcomeKey, WireExpectation)>>,
    /// Where each logical capture is read from on the wire.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub captures: Option<Vec<(CaptureName, WireCapture)>>,
    /// The operation's server-assigned ignore-set membership (the paths the
    /// `equivalent` assertion excludes as `server_assigned`).
    #[serde(default)]
    pub server_assigned: Vec<String>,
}

impl OperationBinding {
    /// Whether this binding declares the operation unrealized on its ITS.
    #[must_use]
    pub fn is_unrealized(&self) -> bool {
        self.unrealized.is_some()
    }

    /// Whether this binding realizes the operation over a declared EXTENSION
    /// route rather than a released ITS-REST operation.
    #[must_use]
    pub fn is_extension(&self) -> bool {
        self.extension.is_some()
    }

    /// Realization-shape invariant: exactly one of `unrealized` or the full
    /// wire form (`request` + `outcomes`), and an `extension` declaration
    /// only on the realized form (an extension route IS a realization — the
    /// two blocks are mutually exclusive by construction).
    ///
    /// # Errors
    /// Returns a message naming the violated invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
        if self.unrealized.is_some() && self.extension.is_some() {
            return Err(
                "a binding is either unrealized or an extension realization, never both".to_owned(),
            );
        }
        match (&self.unrealized, &self.request, &self.outcomes) {
            (Some(_), None, None) | (None, Some(_), Some(_)) => Ok(()),
            (Some(_), _, _) => Err("unrealized binding must carry no request/outcomes".to_owned()),
            _ => Err("realized binding must carry request and outcomes".to_owned()),
        }
    }

    /// The wire expectation mapped for an outcome kind, if any.
    #[must_use]
    pub fn outcome(&self, kind: OutcomeKind) -> Option<&WireExpectation> {
        self.outcomes
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|(k, _)| k.0 == kind)
            .map(|(_, e)| e)
    }

    /// Whether a logical capture has a wire source.
    #[must_use]
    pub fn maps_capture(&self, name: &CaptureName) -> bool {
        self.captures
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|(n, _)| n == name)
    }
}

/// Newtype keys so the crate's `model::de::ordered_map` can parse them from
/// mapping keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutcomeKey(pub OutcomeKind);

impl std::str::FromStr for OutcomeKey {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        OutcomeKind::from_token(s)
            .map(Self)
            .ok_or_else(|| format!("{s:?} is not an outcome kind"))
    }
}

impl std::fmt::Display for OutcomeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.token())
    }
}

/// Format key of `format_headers`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatKey(pub FormatName);

impl std::str::FromStr for FormatKey {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_owned()))
            .map(Self)
            .map_err(|error| format!("{s:?} is not a format name: {error}"))
    }
}

impl std::fmt::Display for FormatKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_value(self.0) {
            Ok(serde_json::Value::String(s)) => f.write_str(&s),
            _ => f.write_str("?"),
        }
    }
}

/// The per-format header map.
#[derive(Debug, Clone)]
pub struct FormatHeaderMap(pub Vec<(String, FormatHeaderReq)>);

impl<'de> Deserialize<'de> for FormatHeaderMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::model::de::ordered_map(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::case::VersionRange;

    #[test]
    fn wire_from_grammar_is_closed() {
        assert_eq!(
            WireFrom::parse("header Location last-segment").unwrap(),
            WireFrom::Header {
                name: "Location".into(),
                last_segment: true
            }
        );
        assert_eq!(
            WireFrom::parse("body \"ehr_id.value\"").unwrap(),
            WireFrom::Body {
                path: "ehr_id.value".into()
            }
        );
        assert!(matches!(
            WireFrom::parse("capture version_uid").unwrap(),
            WireFrom::Capture(_)
        ));
        assert!(WireFrom::parse("body ehr_id.value").is_err()); // unquoted
        assert!(WireFrom::parse("body-or-location").is_err()); // outside the grammar: express via fallback
        assert!(WireFrom::parse("jsonpath $.x").is_err());
    }

    #[test]
    fn header_matchers_parse() {
        let m: HeaderMatcher = serde_json::from_value(serde_json::json!(
            "pattern:W/\"<versioned_object_uid>::<system_id>::1\""
        ))
        .unwrap();
        assert!(matches!(m, HeaderMatcher::Pattern(_)));
        let m: HeaderMatcher =
            serde_json::from_value(serde_json::json!("\"${preceding_version_uid}\"")).unwrap();
        assert!(matches!(m, HeaderMatcher::Literal(_)));
        assert!(
            serde_json::from_value::<HeaderMatcher>(serde_json::json!("pattern:([unclosed"))
                .is_err()
        );
        // `present?` is the expectation-level shorthand, never a matcher —
        // one modifier with one meaning.
        assert!(serde_json::from_value::<HeaderMatcher>(serde_json::json!("present?")).is_err());
    }

    #[test]
    fn header_expectations_carry_the_two_modifiers() {
        let short: HeaderExpectation =
            serde_json::from_value(serde_json::json!("present")).unwrap();
        assert_eq!(short.matcher, HeaderMatcher::Present);
        assert!(!short.optional);
        assert!(short.applies.is_none());

        // `present?` = optional presence, no form assertion.
        let sugar: HeaderExpectation =
            serde_json::from_value(serde_json::json!("present?")).unwrap();
        assert_eq!(sugar.matcher, HeaderMatcher::Present);
        assert!(sugar.optional);

        let long: HeaderExpectation = serde_json::from_value(serde_json::json!({
            "match": "pattern:W/\"[^\"]+\"",
            "optional": true,
            "applies": { "its_rest": ">=1.1.0" }
        }))
        .unwrap();
        assert!(matches!(long.matcher, HeaderMatcher::Pattern(_)));
        assert!(long.optional);
        let applies = long.applies.unwrap();
        assert_eq!(applies.entries().len(), 1);
        assert_eq!(
            applies.its_rest.as_ref().map(VersionRange::raw),
            Some(">=1.1.0")
        );

        // The mapping form is closed and `match` is mandatory.
        assert!(
            serde_json::from_value::<HeaderExpectation>(serde_json::json!({ "optional": true }))
                .is_err()
        );
        assert!(
            serde_json::from_value::<HeaderExpectation>(
                serde_json::json!({ "match": "present", "since": "1.1.0" })
            )
            .is_err()
        );
        assert!(serde_json::from_value::<HeaderExpectation>(serde_json::json!(7)).is_err());
    }

    #[test]
    fn binding_parses_the_create_ehr_shape() {
        let b: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "request": {
                "method": "POST",
                "path": "/ehr",
                "body": "ehr_status?",
                "headers": { "Prefer": "return=representation" }
            },
            "formats": ["canonical-json", "canonical-xml"],
            "outcomes": {
                "created": { "status": 201,
                             "headers": { "ETag": "present", "Location": "present" },
                             "body": "prefer_conditional" },
                "already_exists": { "status": 409 },
                "validation_failed": { "status": 400 }
            },
            "captures": {
                "ehr_id": { "from": "body \"ehr_id.value\"",
                            "fallback": "header Location last-segment" },
                "version_uid": { "from": "header ETag", "strip": "weak-quotes" }
            },
            "server_assigned": ["ehr_id", "system_id", "time_created"]
        }))
        .unwrap();
        assert_eq!(b.sm_operation.to_string(), "I_EHR_SERVICE.create_ehr");
        assert!(b.outcome(OutcomeKind::Created).is_some());
        assert!(b.outcome(OutcomeKind::NotFound).is_none());
        assert!(b.maps_capture(&CaptureName::parse("ehr_id").unwrap()));
        assert!(b.check_invariants().is_ok());
        let request = b.request.clone().unwrap();
        assert!(matches!(
            request.body,
            Some(RequestBody::Named { optional: true, .. })
        ));
        assert_eq!(request.path.params().len(), 0);
    }

    /// An `extension` realization is a full wire form fenced off from the
    /// released-wire judgements, and it is never also `unrealized`.
    #[test]
    fn extension_realization_is_a_realized_binding_and_never_unrealized() {
        let shape = |extra: serde_json::Value| {
            let mut v = serde_json::json!({
                "sm_operation": "I_PARTY_RELATIONSHIP.get_party_relationship",
                "its": "its-rest",
                "extension": {
                    "family": "party-relationship",
                    "reason": "the release surfaces no PARTY_RELATIONSHIP resource",
                    "source": "SM i_party_relationship.adoc vs ITS-REST demographic.openapi.yaml",
                    "ambiguity": "AMB-32"
                },
                "request": { "method": "GET", "path": "/demographic/party_relationship/{versioned_object_uid}" },
                "outcomes": { "ok": { "status": 200 }, "not_found": { "status": 404 } }
            });
            if let (Some(object), Some(extra)) = (v.as_object_mut(), extra.as_object()) {
                for (k, value) in extra {
                    object.insert(k.clone(), value.clone());
                }
            }
            v
        };

        let b: OperationBinding = serde_json::from_value(shape(serde_json::json!({}))).unwrap();
        assert!(b.is_extension());
        assert!(!b.is_unrealized());
        assert!(b.check_invariants().is_ok());

        // extension + unrealized is unrepresentable as a coherent binding.
        let mut both: OperationBinding = serde_json::from_value(shape(serde_json::json!({
            "unrealized": {
                "reason": "r", "source": "s", "ambiguity": "AMB-32"
            }
        })))
        .unwrap();
        assert!(both.check_invariants().is_err());
        both.request = None;
        both.outcomes = None;
        assert!(both.check_invariants().is_err());
    }

    #[test]
    fn path_params_extract() {
        let r: RequestSpec = serde_json::from_value(serde_json::json!({
            "method": "PUT",
            "path": "/ehr/{ehr_id}/composition/{versioned_object_uid}"
        }))
        .unwrap();
        assert_eq!(r.path.params().len(), 2);
        assert!(
            serde_json::from_value::<RequestSpec>(serde_json::json!({
                "method": "GET", "path": "/ehr/{unclosed"
            }))
            .is_err()
        );
    }

    /// The closed transform grammar decomposes an `OBJECT_VERSION_ID` by its
    /// released lexical form `object_id :: creating_system_id ::
    /// version_tree_id` (ITS-REST Resources.md §Identifier types).
    #[test]
    fn transforms_address_object_version_id_components() {
        let uid = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1";
        assert_eq!(
            TransformRule::RootUid.apply(uid).as_deref(),
            Some("8849182c-82ad-4088-a07f-48ead4180515")
        );
        assert_eq!(
            TransformRule::CreatingSystemId.apply(uid).as_deref(),
            Some("openEHRSys.example.com")
        );
        assert_eq!(
            TransformRule::Uppercase.apply("a::b::1").as_deref(),
            Some("A::B::1")
        );

        // No middle segment => NO capture: a bare versioned_object_uid must
        // never bind as if it were a creating system id.
        assert_eq!(
            TransformRule::CreatingSystemId.apply("8849182c-82ad-4088-a07f-48ead4180515"),
            None
        );
        assert_eq!(TransformRule::CreatingSystemId.apply("uid::"), None);
        // …while root-uid still answers on the same truncated value.
        assert_eq!(TransformRule::RootUid.apply("uid").as_deref(), Some("uid"));

        // The token is the authored form and the grammar stays closed.
        let spec: WireCapture = serde_json::from_value(serde_json::json!({
            "from": "header ETag", "strip": "weak-quotes",
            "transform": "creating-system-id"
        }))
        .unwrap();
        assert_eq!(spec.transform, Some(TransformRule::CreatingSystemId));
        assert!(
            serde_json::from_value::<WireCapture>(serde_json::json!({
                "from": "header ETag", "transform": "middle-segment"
            }))
            .is_err()
        );
    }

    #[test]
    fn query_values_are_single_or_repeated() {
        let r: RequestSpec = serde_json::from_value(serde_json::json!({
            "method": "DELETE",
            "path": "/admin/ehr/all",
            "query": { "ehr_id": ["${ehr_id_subset?}", "${ehr_id_subset_2?}"],
                       "fetch": "${url_fetch?}" }
        }))
        .unwrap();
        let query = r.query.unwrap();
        let repeated = &query.iter().find(|(name, _)| name == "ehr_id").unwrap().1;
        assert!(repeated.is_repeated());
        assert_eq!(repeated.templates().len(), 2);
        let single = &query.iter().find(|(name, _)| name == "fetch").unwrap().1;
        assert!(!single.is_repeated());
        assert_eq!(single.templates().len(), 1);

        // An empty sequence declares nothing; a non-string member is outside
        // the template grammar; a non-string, non-sequence value is neither.
        for bad in [
            serde_json::json!({ "method": "GET", "path": "/x", "query": { "p": [] } }),
            serde_json::json!({ "method": "GET", "path": "/x", "query": { "p": [1] } }),
            serde_json::json!({ "method": "GET", "path": "/x", "query": { "p": 1 } }),
            // an illegal reference inside a member is still rejected
            serde_json::json!({ "method": "GET", "path": "/x", "query": { "p": ["${step2.body}"] } }),
        ] {
            assert!(
                serde_json::from_value::<RequestSpec>(bad.clone()).is_err(),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn duplicate_outcome_keys_rejected() {
        // serde_json::Map deduplicates silently, so feed the YAML front-end.
        let yaml = "sm_operation: I_EHR_SERVICE.create_ehr\nits: its-rest\nrequest: { method: POST, path: /ehr }\noutcomes:\n  created: { status: 201 }\n  created: { status: 200 }\n";
        let result: Result<OperationBinding, _> = serde_saphyr::from_str(yaml);
        assert!(result.is_err());
    }
}
