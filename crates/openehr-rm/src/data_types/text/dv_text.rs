//! `DV_TEXT` — a text item.
//!
//! openEHR class: `DV_TEXT`, package `rm.data_types.text`.
//! Inherits: `DATA_VALUE`.
//!
//! A text item, which may contain any amount of legal characters arranged
//! as e.g. words, sentences etc (i.e. one `DV_TEXT` may be more than one
//! word). Visual formatting and hyperlinks may be included via markdown.
//!
//! If the `formatting` field is set, the `value` field is affected as
//! follows:
//!
//! * `formatting = "plain"`: plain text, may contain newlines;
//! * `formatting = "plain_no_newlines"`: plain text with no newlines;
//! * `formatting = "markdown"`: text in markdown format; use of CommonMark
//!   strongly recommended.
//!
//! A `DV_TEXT` can be coded by adding mappings to it.
use super::code_phrase::CodePhrase;
use super::dv_coded_text::DvCodedText;
use super::term_mapping::TermMapping;
use crate::data_types::data_value::DataValueApi;
use crate::data_types::uri::dv_uri::DvUri;

/// Canonical `_type` discriminator string for this class in serialized
/// form (ADR-001 Refinements: serde derives wait until P4).
pub const TYPE_NAME: &str = "DV_TEXT";

/// Shared attribute state of `DV_TEXT` and its sole descendant,
/// `DV_CODED_TEXT`.
///
/// # Transcription approach
///
/// `DV_TEXT` is not marked abstract in the specification (it is directly
/// constructible as plain, uncoded text) but its own "Design" narrative
/// states it is substitutable by `DV_CODED_TEXT` wherever it is declared as
/// a field type ("Since `DV_CODED_TEXT` is a subtype of `DV_TEXT`, it can
/// be used in place of it") — the same substitutability concern ADR-001 §3
/// /Refinements addresses for genuinely abstract classes. This cluster has
/// exactly one load-bearing polymorphic use site: `DV_PARAGRAPH.items:
/// List<DV_TEXT>`, a list of plain-or-coded text items. Applying the same
/// shape used for `UID`/`OBJECT_ID` (ADR-001 Refinements): an embeddable
/// `DvTextData` struct carrying the six shared attributes, a closed
/// [`DvText`] enum with one variant per concrete descendant
/// (`Text(DvTextData)` for a "bare" `DV_TEXT` instance,
/// `Coded(DvCodedText)` for `DV_CODED_TEXT`), and a [`DvTextApi`] trait
/// exposing the shared accessors uniformly.
///
/// `DV_CODED_TEXT` embeds this same `DvTextData` (see `dv_coded_text.rs`)
/// rather than re-declaring the six fields, so field order and types stay
/// identical between the two concrete forms.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DvTextData {
    /// `value`: `String` (`1..1`).
    ///
    /// Displayable rendition of the item, regardless of its underlying
    /// structure. For `DV_CODED_TEXT`, this is the rubric of the complete
    /// term as provided by the terminology service.
    pub value: String,

    /// `hyperlink`: `DV_URI` (`0..1`).
    ///
    /// DEPRECATED: this field is deprecated; use markdown link/text in the
    /// `value` attribute, and `"markdown"` as the value of the
    /// `formatting` field.
    ///
    /// Original usage, prior to RM Release 1.0.4: optional link sitting
    /// behind a section of plain text or coded term item.
    pub hyperlink: Option<DvUri>,

    /// `formatting`: `String` (`0..1`).
    ///
    /// If set, contains one of the following values:
    ///
    /// * `"plain"`: use for plain text, possibly containing newlines, but
    ///   otherwise unformatted (same as `Void`);
    /// * `"plain_no_newlines"`: use for text containing no newlines or
    ///   other formatting;
    /// * `"markdown"`: use for markdown formatted text, strongly
    ///   recommended in the format of the CommonMark specification.
    ///
    /// DEPRECATED usage: contains a string of the form
    /// `"name:value; name:value..."`, e.g. `"font-weight : bold;
    /// font-family : Arial; font-size : 12pt;"`. Values taken from W3C CSS2
    /// properties lists for background and font.
    ///
    /// Invariant `Formatting_valid`: `formatting /= void implies not
    /// formatting.is_empty`.
    pub formatting: Option<String>,

    /// `mappings`: `List<TERM_MAPPING>` (`0..1`).
    ///
    /// Terms from other terminologies most closely matching this term,
    /// typically used where the originator (e.g. pathology lab) of
    /// information uses a local terminology but also supplies one or more
    /// equivalents from well known terminologies (e.g. LOINC).
    ///
    /// Invariant `Mappings_valid`: `mappings /= void implies not
    /// mappings.is_empty`.
    ///
    /// PORT NOTE: `List<T>` maps to `Vec<T>` per `docs/PORTING.md` §14.2;
    /// the `0..1` cardinality plus `Mappings_valid` (non-empty-if-present)
    /// is modelled with `Option<Vec<..>>` rather than a bare possibly-empty
    /// `Vec`, matching the `translations`/`annotations` precedent in
    /// `crates/openehr-base/src/resource/authored_resource.rs`.
    pub mappings: Option<Vec<TermMapping>>,

    /// `language`: `CODE_PHRASE` (`0..1`).
    ///
    /// Optional indicator of the localised language in which the value is
    /// written. Coded from openEHR Code Set "languages". Only used when
    /// either the text object is in a different language from the
    /// enclosing `ENTRY`, or else the text object is being used outside of
    /// an `ENTRY` or other enclosing structure which indicates the
    /// language.
    ///
    /// Invariant `Language_valid`: `language /= Void implies
    /// code_set(Code_set_id_languages).has_code(language)`.
    ///
    /// TODO(port): invariant requires a live terminology-service code-set
    /// lookup (`openehr_terminology`); not yet enforced.
    pub language: Option<CodePhrase>,

    /// `encoding`: `CODE_PHRASE` (`0..1`).
    ///
    /// Name of character encoding scheme in which this value is encoded.
    /// Coded from openEHR Code Set "character sets". Unicode is the default
    /// assumption in openEHR, with UTF-8 being the assumed encoding. This
    /// attribute allows for variations from these assumptions.
    ///
    /// Invariant `Encoding_valid`: `encoding /= Void implies
    /// code_set(Code_set_id_character_sets).has_code(encoding)`.
    ///
    /// TODO(port): invariant requires a live terminology-service code-set
    /// lookup (`openehr_terminology`); not yet enforced.
    pub encoding: Option<CodePhrase>,
}

impl DvTextData {
    /// `Mappings_valid`: `mappings /= void implies not mappings.is_empty`.
    ///
    /// PORT NOTE: structurally guaranteed by the `Option<Vec<..>>`
    /// modelling above (a `Some` value is never constructed empty by this
    /// crate's own code, but nothing yet prevents a caller from doing so
    /// directly since there is no private-field/constructor boundary). Kept
    /// as an explicit check for when the `Validate` framework lands.
    pub fn invariant_mappings_valid(&self) -> bool {
        self.mappings.as_ref().is_none_or(|m| !m.is_empty())
    }

    /// `Formatting_valid`: `formatting /= void implies not
    /// formatting.is_empty`.
    pub fn invariant_formatting_valid(&self) -> bool {
        self.formatting.as_ref().is_none_or(|f| !f.is_empty())
    }

    /// `Language_valid`: `language /= Void implies
    /// code_set(Code_set_id_languages).has_code(language)`.
    ///
    /// TODO(port): requires a live `TerminologyService`/`CodeSetAccess`
    /// lookup (`openehr_terminology`), not available at this layer without
    /// threading a service reference through; left unimplemented.
    pub fn invariant_language_valid(&self) -> bool {
        // TODO(port): call into openehr_terminology::CodeSetAccess::has_code
        // for the openEHR "languages" code set once a service handle is
        // available here.
        todo!("Language_valid requires a live terminology code-set lookup")
    }

    /// `Encoding_valid`: `encoding /= Void implies
    /// code_set(Code_set_id_character_sets).has_code(encoding)`.
    ///
    /// TODO(port): requires a live `TerminologyService`/`CodeSetAccess`
    /// lookup (`openehr_terminology`), not available at this layer without
    /// threading a service reference through; left unimplemented.
    pub fn invariant_encoding_valid(&self) -> bool {
        // TODO(port): call into openehr_terminology::CodeSetAccess::has_code
        // for the openEHR "character sets" code set once a service handle
        // is available here.
        todo!("Encoding_valid requires a live terminology code-set lookup")
    }
}

/// `DV_TEXT` is used polymorphically wherever the spec declares a field of
/// type `DV_TEXT` and a `DV_CODED_TEXT` value must be substitutable there
/// (see the [`DvTextData`] doc comment). Per ADR-001 Refinements, the two
/// concrete forms are collected into this closed `enum`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DvText {
    /// A "bare" `DV_TEXT` instance: plain or markdown-formatted text with
    /// no attached defining code.
    Text(DvTextData),
    /// A `DV_CODED_TEXT` instance, substitutable wherever `DV_TEXT` is
    /// declared.
    Coded(DvCodedText),
}

/// Marker/accessor trait shared by `DV_TEXT` and `DV_CODED_TEXT`, exposing
/// the six shared attributes uniformly whether the caller holds a concrete
/// type or a [`DvText`] enum value.
pub trait DvTextApi {
    /// `value`: displayable rendition of the item.
    fn value(&self) -> &str;
    /// `hyperlink`: deprecated optional web link.
    fn hyperlink(&self) -> Option<&DvUri>;
    /// `formatting`: optional markdown/legacy CSS formatting marker.
    fn formatting(&self) -> Option<&str>;
    /// `mappings`: optional list of term mappings to other terminologies.
    fn mappings(&self) -> Option<&[TermMapping]>;
    /// `language`: optional `CODE_PHRASE` naming the text's language.
    fn language(&self) -> Option<&CodePhrase>;
    /// `encoding`: optional `CODE_PHRASE` naming the text's character
    /// encoding.
    fn encoding(&self) -> Option<&CodePhrase>;
}

impl DvTextApi for DvTextData {
    fn value(&self) -> &str {
        &self.value
    }
    fn hyperlink(&self) -> Option<&DvUri> {
        self.hyperlink.as_ref()
    }
    fn formatting(&self) -> Option<&str> {
        self.formatting.as_deref()
    }
    fn mappings(&self) -> Option<&[TermMapping]> {
        self.mappings.as_deref()
    }
    fn language(&self) -> Option<&CodePhrase> {
        self.language.as_ref()
    }
    fn encoding(&self) -> Option<&CodePhrase> {
        self.encoding.as_ref()
    }
}

impl DvTextApi for DvText {
    fn value(&self) -> &str {
        match self {
            DvText::Text(v) => v.value(),
            DvText::Coded(v) => v.value(),
        }
    }
    fn hyperlink(&self) -> Option<&DvUri> {
        match self {
            DvText::Text(v) => v.hyperlink(),
            DvText::Coded(v) => v.hyperlink(),
        }
    }
    fn formatting(&self) -> Option<&str> {
        match self {
            DvText::Text(v) => v.formatting(),
            DvText::Coded(v) => v.formatting(),
        }
    }
    fn mappings(&self) -> Option<&[TermMapping]> {
        match self {
            DvText::Text(v) => v.mappings(),
            DvText::Coded(v) => v.mappings(),
        }
    }
    fn language(&self) -> Option<&CodePhrase> {
        match self {
            DvText::Text(v) => v.language(),
            DvText::Coded(v) => v.language(),
        }
    }
    fn encoding(&self) -> Option<&CodePhrase> {
        match self {
            DvText::Text(v) => v.encoding(),
            DvText::Coded(v) => v.encoding(),
        }
    }
}

impl DataValueApi for DvTextData {
    fn type_name(&self) -> &'static str {
        TYPE_NAME
    }
}

impl DataValueApi for DvText {
    fn type_name(&self) -> &'static str {
        match self {
            DvText::Text(_) => TYPE_NAME,
            DvText::Coded(v) => v.type_name(),
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.text — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_text.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-text_package.adoc §Class Descriptions / dv_text.adoc §DV_TEXT Class
//   confidence: medium
//   todos: 6
//   note: DvTextData+DvText(enum)+DvTextApi triple applied to a *concrete* (not spec-abstract) parent class, extending the ADR-001 Refinements pattern to DV_TEXT/DV_CODED_TEXT substitutability (the one load-bearing use site in this cluster is DV_PARAGRAPH.items: List<DV_TEXT>); Language_valid/Encoding_valid invariants left as todo!() pending a terminology-service handle (each mentioned on both its field doc and its invariant method, hence 4 of the 6).
// ─────────────────────────────────────────────
