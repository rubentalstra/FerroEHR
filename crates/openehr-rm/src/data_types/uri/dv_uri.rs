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

/// Canonical `_type` discriminator string for this class in serialized
/// form (ADR-001 Refinements: serde derives wait until P4).
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    /// TODO(port): parse `value` per RFC-3986 grammar (`scheme ":" ...`);
    /// no RFC-3986 parser dependency exists in this crate yet.
    pub fn scheme(&self) -> String {
        todo!("DvUriData::scheme: RFC-3986 parsing not yet implemented")
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
    /// TODO(port): parse `value` per RFC-3986 grammar; no RFC-3986 parser
    /// dependency exists in this crate yet.
    pub fn path(&self) -> String {
        todo!("DvUriData::path: RFC-3986 parsing not yet implemented")
    }

    /// `fragment_id(): String`.
    ///
    /// A part of, a fragment or a sub-function within an object. Allows
    /// references to sub-parts of objects, such as a certain line and
    /// character position in a text object. The syntax and semantics are
    /// defined by the application responsible for the object.
    ///
    /// TODO(port): parse `value` per RFC-3986 grammar; no RFC-3986 parser
    /// dependency exists in this crate yet.
    pub fn fragment_id(&self) -> String {
        todo!("DvUriData::fragment_id: RFC-3986 parsing not yet implemented")
    }

    /// `query(): String`.
    ///
    /// Query string to send to application implied by scheme and path.
    /// Enables queries to applications, including databases, to be included
    /// in the URI. Supports any query meaningful to the server, including
    /// SQL.
    ///
    /// TODO(port): parse `value` per RFC-3986 grammar; no RFC-3986 parser
    /// dependency exists in this crate yet.
    pub fn query(&self) -> String {
        todo!("DvUriData::query: RFC-3986 parsing not yet implemented")
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DvUri {
    /// Embedded `DV_URI` state (the single `value` attribute).
    pub uri: DvUriData,
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

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.uri — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_uri.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master10-uri_package.adoc §Class Descriptions / dv_uri.adoc §DV_URI Class
//   confidence: medium
//   todos: 6
//   note: DvUriData embedded struct pattern mirrors DvTextData (single attribute, one concrete descendant); scheme/path/fragment_id/query all left as todo!() pending an RFC-3986 parser dependency decision (no such dependency exists in openehr-rm yet, and none was authorized for this transcription pass); Value_valid invariant mentioned on both the field doc and the invariant method doc.
// ─────────────────────────────────────────────
