//! `DV_URI` — a reference to an object conforming to RFC-3986 URI syntax.
//!
//! openEHR class: `DV_URI`, package `rm.data_types.uri`.
//! Inherits: `DATA_VALUE`.
//!
//! A reference to an object which structurally conforms to the Universal
//! Resource Identifier (URI) RFC-3986 standard. The reference is contained
//! in the `value` attribute, which is a `String`. So-called 'plain-text
//! URIs' that contain RFC-3986 forbidden characters such as spaces etc, are
//! allowed on the basis that they need to be RFC-3986 encoded prior to use
//! in e.g. REST APIs or other contexts relying on machine-level
//! conformance.
use crate::data_types::data_value::DataValueApi;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into its [`TypeName`] impl (ADR-002).
pub const TYPE_NAME: &str = "DV_URI";

/// Shared attribute state of `DV_URI` and its descendant `DV_EHR_URI`.
///
/// `DV_URI` is a concrete class in its own right (unlike a genuinely
/// abstract RM class), but `DV_EHR_URI` narrows it with an additional
/// invariant (`Scheme_valid`) rather than adding fields — following the
/// same embedding shape used for `DV_TEXT`/`DV_CODED_TEXT`
/// (`crates::data_types::text::dv_text`), the single `value: String`
/// attribute is held in a `DvUriData` struct that `DV_EHR_URI` embeds
/// directly, since `DV_URI` has no closed set of *other* subtypes that
/// would need a wrapping enum here (only the one descendant, `DV_EHR_URI`,
/// exists in this cluster, and it is transcribed as its own leaf type in
/// `dv_ehr_uri.rs` rather than folded into an enum with this one — there is
/// no RM attribute in this cluster declared bare `DV_URI` that must accept
/// either form interchangeably, unlike `DV_TEXT`'s `DV_PARAGRAPH.items`
/// case).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DvUriData {
    /// `value`: `String` (`1..1`).
    ///
    /// Value of URI as a String. 'Plain-text' URIs are allowed, enabling
    /// better readability, but must be RFC-3986 encoded in use.
    ///
    /// Invariant `Value_valid`: `not value.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant
    /// framework.
    pub value: String,
}

impl DvUriData {
    /// `scheme(): String`.
    ///
    /// A distributed information 'space' in which information objects
    /// exist. The scheme simultaneously specifies an information space and
    /// a mechanism for accessing objects in that space. For example if
    /// scheme = `"ftp"`, it identifies the information space in which all
    /// ftp-able objects exist, and also the application (ftp) which can be
    /// used to access them. Values may include: `"ftp"`, `"telnet"`,
    /// `"mailto"`, etc. Refer to RFC-3986 for a full list.
    ///
    pub fn scheme(&self) -> String {
        parse_uri_components(&self.value).scheme
    }

    /// `path(): String`.
    ///
    /// A string whose format is a function of the scheme. Identifies the
    /// location in `<scheme>`-space of an information entity. Typical
    /// values include hierarchical directory paths for any machine. For
    /// example, with scheme = `"ftp"`, path might be
    /// `"/pub/images/image_01"`. The strings `"."` and `".."` are reserved
    /// for use in the path. Paths may include internet/intranet location
    /// identifiers of the form: `sub_domain...domain`, e.g.
    /// `"info.cern.ch"`.
    ///
    pub fn path(&self) -> String {
        parse_uri_components(&self.value).path
    }

    /// `fragment_id(): String`.
    ///
    /// A part of, a fragment or a sub-function within an object. Allows
    /// references to sub-parts of objects, such as a certain line and
    /// character position in a text object. The syntax and semantics are
    /// defined by the application responsible for the object.
    ///
    pub fn fragment_id(&self) -> String {
        parse_uri_components(&self.value).fragment
    }

    /// `query(): String`.
    ///
    /// Query string to send to application implied by scheme and path.
    /// Enables queries to applications, including databases, to be included
    /// in the URI. Supports any query meaningful to the server, including
    /// SQL.
    ///
    pub fn query(&self) -> String {
        parse_uri_components(&self.value).query
    }

    /// `Value_valid`: `not value.is_empty`.
    ///
    /// TODO(port): wire into a `Validate` impl once the RM invariant
    /// framework lands.
    pub fn invariant_value_valid(&self) -> bool {
        !self.value.is_empty()
    }
}

/// `DV_URI` — a leaf, non-abstract class holding exactly the shared
/// [`DvUriData`] state, with no attributes of its own beyond it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DvUri {
    /// Canonical `_type` discriminator (`"DV_URI"`), always serialized
    /// first (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_URI` state (the single `value` attribute).
    ///
    /// `#[serde(flatten)]` per ADR-001 §3 — `DvUriData`'s single `value`
    /// field appears directly on the `DV_URI` JSON object, not nested under
    /// a `"uri"` key. `DvUriData` carries no tag of its own (ADR-002 §3:
    /// embedded `*Data` structs are untagged), so this flatten cannot
    /// collide with the `type_tag` above.
    #[serde(flatten)]
    pub uri: DvUriData,
}

impl TypeName for DvUri {
    const NAME: &'static str = TYPE_NAME;
}

impl DvUri {
    /// `scheme(): String`. See [`DvUriData::scheme`].
    pub fn scheme(&self) -> String {
        self.uri.scheme()
    }

    /// `path(): String`. See [`DvUriData::path`].
    pub fn path(&self) -> String {
        self.uri.path()
    }

    /// `fragment_id(): String`. See [`DvUriData::fragment_id`].
    pub fn fragment_id(&self) -> String {
        self.uri.fragment_id()
    }

    /// `query(): String`. See [`DvUriData::query`].
    pub fn query(&self) -> String {
        self.uri.query()
    }
}

impl DataValueApi for DvUri {
    fn type_name(&self) -> &'static str {
        TYPE_NAME
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UriComponents {
    scheme: String,
    path: String,
    query: String,
    fragment: String,
}

fn parse_uri_components(value: &str) -> UriComponents {
    let (without_fragment, fragment) = split_once_optional(value, '#');
    let (without_query, query) = split_once_optional(without_fragment, '?');
    let (scheme, rest) = split_scheme(without_query);
    let path = uri_path(rest);

    UriComponents {
        scheme: scheme.to_string(),
        path: path.to_string(),
        query: query.to_string(),
        fragment: fragment.to_string(),
    }
}

fn uri_path(rest: &str) -> &str {
    if let Some(after_authority) = rest.strip_prefix("//") {
        after_authority
            .find('/')
            .map_or("", |idx| &after_authority[idx..])
    } else {
        rest
    }
}

fn split_once_optional(value: &str, delimiter: char) -> (&str, &str) {
    value
        .split_once(delimiter)
        .map_or((value, ""), |(left, right)| (left, right))
}

fn split_scheme(value: &str) -> (&str, &str) {
    let Some((scheme, rest)) = value.split_once(':') else {
        return ("", value);
    };
    if is_valid_scheme(scheme) {
        (scheme, rest)
    } else {
        ("", value)
    }
}

fn is_valid_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_parts_follow_rfc3986_components() {
        let uri = DvUriData {
            value: "https://example.org/path/to/doc?format=json#line-7".to_string(),
        };

        assert_eq!(uri.scheme(), "https");
        assert_eq!(uri.path(), "/path/to/doc");
        assert_eq!(uri.query(), "format=json");
        assert_eq!(uri.fragment_id(), "line-7");
    }

    #[test]
    fn plain_text_uri_parts_are_still_extractable() {
        let uri = DvUriData {
            value: "ehr:/ehr_id/composition with spaces?x=1#section 1".to_string(),
        };

        assert_eq!(uri.scheme(), "ehr");
        assert_eq!(uri.path(), "/ehr_id/composition with spaces");
        assert_eq!(uri.query(), "x=1");
        assert_eq!(uri.fragment_id(), "section 1");
    }

    #[test]
    fn uri_path_extraction_does_not_normalize_the_stored_string() {
        let no_path = DvUriData {
            value: "https://example.org?format=json#top".to_string(),
        };
        let dot_segments = DvUriData {
            value: "https://example.org/a/../b".to_string(),
        };

        assert_eq!(no_path.path(), "");
        assert_eq!(no_path.query(), "format=json");
        assert_eq!(no_path.fragment_id(), "top");
        assert_eq!(dot_segments.path(), "/a/../b");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.uri — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_uri.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master10-uri_package.adoc §Class Descriptions / dv_uri.adoc §DV_URI Class
//   confidence: medium
//   todos: 2
//   note: DvUriData embedded struct pattern mirrors DvTextData (single attribute, one concrete descendant); scheme/path/fragment_id/query extract RFC3986 components without normalising the stored string, preserving the spec's explicit allowance for unencoded human-readable strings. Value_valid invariant mentioned on both the field doc and the invariant method doc. P4/ADR-002: DvUri self-tags via TypeTag<Self> first field + TypeName ("DV_URI"), inert struct-level #[serde(rename)] deleted; DvUriData stays untagged (embedded *Data struct, flattened here and in DvEhrUri).
// ─────────────────────────────────────────────
