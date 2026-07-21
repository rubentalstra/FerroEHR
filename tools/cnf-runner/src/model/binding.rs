//! Operation bindings — the wire layer, one file per SM operation per ITS.
//!
//! A binding maps request construction, each outcome kind → wire
//! expectation, and each logical capture → wire source; every mapping cites
//! its OAS source (ITS-REST 1.1.0 `specifications/operations/*.yaml` +
//! `responses/*.yaml`). The capture-source, header-matcher, and
//! body-selector vocabularies are closed.

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};

use crate::ids::{CaptureName, SmOperationRef};
use crate::model::case::Applies;
use crate::refgrammar::Template;
use crate::vocab::{FormatName, HttpMethod, ItsName, OutcomeKind};

/// A wire location a capture reads from (closed grammar):
/// `header <Name>` · `header <Name> last-segment` · `body "<path>"` ·
/// `capture <name>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireFrom {
    Header {
        name: String,
        last_segment: bool,
    },
    Body {
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
    #[serde(rename = "weak-quotes")]
    WeakQuotes,
}

/// Post-extraction transform: reduce an `OBJECT_VERSION_ID` to its root uid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TransformRule {
    #[serde(rename = "root-uid")]
    RootUid,
}

/// One logical-capture wire mapping with optional modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireCapture {
    pub from: WireFrom,
    #[serde(default)]
    pub strip: Option<StripRule>,
    #[serde(default)]
    pub transform: Option<TransformRule>,
    /// Tried when `from` yields nothing (e.g. body field under
    /// `Prefer: return=minimal`, falling back to `Location`).
    #[serde(default)]
    pub fallback: Option<WireFrom>,
}

/// A header expectation on a wire outcome (closed matcher vocabulary):
/// `present` · `present?` · `absent` · `negotiated` · `latest-version-uid` ·
/// `pattern:<regex>` · a literal string.
#[derive(Debug, Clone, PartialEq)]
pub enum HeaderMatcher {
    Present,
    /// Assert only if the schedule row says so.
    PresentOptional,
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

impl<'de> Deserialize<'de> for HeaderMatcher {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "present" => Self::Present,
            "present?" => Self::PresentOptional,
            "absent" => Self::Absent,
            "negotiated" => Self::Negotiated,
            "latest-version-uid" => Self::LatestVersionUid,
            _ => {
                if let Some(pattern) = s.strip_prefix("pattern:") {
                    let probe = placeholder_wildcarded(pattern);
                    regex::Regex::new(&probe).map_err(|e| {
                        D::Error::custom(format!(
                            "header pattern {pattern:?} does not compile: {e}"
                        ))
                    })?;
                    Self::Pattern(pattern.to_owned())
                } else {
                    Self::Literal(Template::parse(&s).map_err(D::Error::custom)?)
                }
            }
        })
    }
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
    #[serde(rename = "present")]
    Present,
    #[serde(rename = "absent")]
    Absent,
}

/// One outcome kind's wire expectation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireExpectation {
    pub status: StatusCode,
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub headers: Option<Vec<(String, HeaderMatcher)>>,
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
    Named { name: String, optional: bool },
    /// A structured body template (the query binding's
    /// `{ q: ${q}, offset: ${offset?} … }`).
    Structured(crate::model::value::TemplatedValue),
}

impl<'de> Deserialize<'de> for RequestBody {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
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
    pub method: HttpMethod,
    pub path: PathTemplate,
    /// Query parameters (name → value template; optional refs `${x?}` omit
    /// the parameter when unresolved).
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub query: Option<Vec<(String, Template)>>,
    #[serde(default)]
    pub body: Option<RequestBody>,
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub headers: Option<Vec<(String, Template)>>,
}

/// A per-format extra-header requirement (`openehr-template-id: required`).
#[derive(Debug, Clone, PartialEq)]
pub enum FormatHeaderReq {
    Required,
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

/// An explicit unrealized-operation declaration: the ITS surfaces no wire
/// for this SM operation, so cases anchored to it are `not-applicable` with
/// this citation on this ITS (machine-readable, never silent absence).
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

/// One binding file: either a full wire realization, or an explicit
/// `unrealized` declaration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationBinding {
    pub sm_operation: SmOperationRef,
    pub its: ItsName,
    #[serde(default)]
    pub applies: Option<Applies>,
    #[serde(default)]
    pub unrealized: Option<UnrealizedDecl>,
    #[serde(default)]
    pub request: Option<RequestSpec>,
    #[serde(default)]
    pub formats: Vec<FormatName>,
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub format_headers: Option<Vec<(FormatKey, FormatHeaderMap)>>,
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub outcomes: Option<Vec<(OutcomeKey, WireExpectation)>>,
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

    /// Realization-shape invariant: exactly one of `unrealized` or the full
    /// wire form (`request` + `outcomes`).
    ///
    /// # Errors
    /// Returns a message naming the violated invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
        match (&self.unrealized, &self.request, &self.outcomes) {
            (Some(_), None, None) => Ok(()),
            (None, Some(_), Some(_)) => Ok(()),
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

/// Newtype keys so [`crate::model::de::ordered_map`] can parse them from
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
            .map_err(|_| format!("{s:?} is not a format name"))
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
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)] // test assertions/fixtures
mod tests {
    use super::*;

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
        let m: HeaderMatcher = serde_json::from_value(serde_json::json!("present?")).unwrap();
        assert_eq!(m, HeaderMatcher::PresentOptional);
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

    #[test]
    fn duplicate_outcome_keys_rejected() {
        // serde_json::Map deduplicates silently, so feed the YAML front-end.
        let yaml = "sm_operation: I_EHR_SERVICE.create_ehr\nits: its-rest\nrequest: { method: POST, path: /ehr }\noutcomes:\n  created: { status: 201 }\n  created: { status: 200 }\n";
        let result: Result<OperationBinding, _> = serde_saphyr::from_str(yaml);
        assert!(result.is_err());
    }
}
