//! `Terminology_code` — a standalone reference to a terminology concept.
//!
//! openEHR class: `Terminology_code`, package
//! `base.foundation_types.terminology`.
//! Inherits: `Any`.
//!
//! Primitive type representing a standalone reference to a terminology
//! concept, in the form of a terminology identifier, optional version, and a
//! code or code string from the terminology. Per the terminology chapter
//! overview, an instance may reference a single term, a value set of
//! multiple terms, or any other terminological entity referenceable with a
//! code. Sometimes called a "concept code" or, when used as a reference, a
//! "concept reference."
use super::super::primitive_types::any::Any;
use super::super::primitive_types::string::OpenEhrString;
use super::super::primitive_types::uri::Uri;

/// `Terminology_code` has four attributes and is a leaf, non-abstract class,
/// so it is transcribed as a plain struct — unlike the behaviour-only
/// abstract classes in `primitive_types` (`Any`, `Ordered`) that become
/// traits, this class carries state per the "abstract class with
/// attributes → struct" / "leaf class → struct" transcription rule.
///
/// Field types follow the spec's attribute table literally: `String`
/// attributes map to `OpenEhrString` (the foundation-types `String` class
/// transcribed in `primitive_types::string`, per that file's own naming
/// note — not `std::string::String` directly, since this struct embeds the
/// foundation-types class as declared, matching the treatment already given
/// to `Uri`'s own `String` parent).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerminologyCode {
    /// `terminology_id: String` (`1..1`).
    ///
    /// The archetype environment namespace identifier used to identify a
    /// terminology. Typically a value like `"snomed_ct"` that is mapped
    /// elsewhere to the full URI identifying the terminology.
    pub terminology_id: OpenEhrString,

    /// `terminology_version: String` (`0..1`).
    ///
    /// Optional string value representing terminology version, typically a
    /// date or dotted numeric.
    pub terminology_version: Option<OpenEhrString>,

    /// `code_string: String` (`1..1`).
    ///
    /// A terminology code or post-coordinated code expression, if supported
    /// by the terminology. The code may refer to a single term, a value set
    /// consisting of multiple terms, or some other entity representable
    /// within the terminology.
    pub code_string: OpenEhrString,

    /// `uri: Uri` (`0..1`).
    ///
    /// The URI reference that may be used as a concrete key into a notional
    /// terminology service for queries that can obtain the term text,
    /// definition, and other associated elements.
    pub uri: Option<Uri>,
}

impl Any for TerminologyCode {
    fn is_equal(&self, other: &Self) -> bool {
        self.terminology_id == other.terminology_id
            && self.terminology_version == other.terminology_version
            && self.code_string == other.code_string
            && self.uri == other.uri
    }

    fn type_of(&self) -> String {
        "Terminology_code".to_string()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.terminology §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/terminology_code.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master07-terminology.adoc §Class Definitions / terminology_code.adoc §Terminology_code Class
//   confidence: high
//   todos: 0
//   note: leaf struct, four attributes transcribed 1:1; String fields use OpenEhrString (the foundation-types class) not std::string::String, matching the Uri precedent for embedding a declared foundation-types parent/attribute type rather than the raw primitive.
// ─────────────────────────────────────────────
