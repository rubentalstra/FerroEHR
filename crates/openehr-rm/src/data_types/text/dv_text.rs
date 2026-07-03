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
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_term::{CodeSetAccess, OpenehrCodeSetIdentifiers, TerminologyService};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into [`DvTextData`]'s [`TypeName`] impl (ADR-002).
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Enforced by [`DvTextData::invariant_language_valid`], which takes a
    /// `&TerminologyService` (ADR-003 decision 8: invariants that need
    /// terminology take the service).
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Enforced by [`DvTextData::invariant_encoding_valid`], which takes a
    /// `&TerminologyService` (ADR-003 decision 8: invariants that need
    /// terminology take the service).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<CodePhrase>,
}

/// PORT NOTE (ADR-002 §3, exception clause): `DvTextData` is an embedded
/// shared-state struct (`#[serde(flatten)]`ed into `DvCodedText`), so it
/// carries **no** `type_tag` field of its own — a flattened tag would
/// collide with the embedding class's. But it *doubles as the bare concrete
/// `DV_TEXT` instance*, so it implements [`TypeName`], letting the
/// [`DvText::Text`] struct variant carry a `TypeTag<DvTextData>` beside the
/// flattened data.
impl TypeName for DvTextData {
    const NAME: &'static str = TYPE_NAME;
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
    /// Per ADR-003 decision 8, invariants that need terminology take a
    /// `&TerminologyService`. Trivially `true` when `language` is `Void`
    /// (`None`); otherwise the language's `code_string` must be a member of
    /// the openEHR "languages" code set (`Code_set_id_languages`, ISO 639-1).
    pub fn invariant_language_valid(&self, terminology: &TerminologyService) -> bool {
        match &self.language {
            None => true,
            Some(language) => terminology
                .code_set_for_id(OpenehrCodeSetIdentifiers::CODE_SET_ID_LANGUAGES)
                .is_some_and(|code_set| code_set.has_code(&language.code_string)),
        }
    }

    /// `Encoding_valid`: `encoding /= Void implies
    /// code_set(Code_set_id_character_sets).has_code(encoding)`.
    ///
    /// Per ADR-003 decision 8, invariants that need terminology take a
    /// `&TerminologyService`. Trivially `true` when `encoding` is `Void`
    /// (`None`); otherwise the encoding's `code_string` must be a member of
    /// the openEHR "character sets" code set (`Code_set_id_character_sets`,
    /// IANA character sets).
    pub fn invariant_encoding_valid(&self, terminology: &TerminologyService) -> bool {
        match &self.encoding {
            None => true,
            Some(encoding) => terminology
                .code_set_for_id(OpenehrCodeSetIdentifiers::CODE_SET_ID_CHARACTER_SETS)
                .is_some_and(|code_set| code_set.has_code(&encoding.code_string)),
        }
    }
}

/// `DV_TEXT` is used polymorphically wherever the spec declares a field of
/// type `DV_TEXT` and a `DV_CODED_TEXT` value must be substitutable there
/// (see the [`DvTextData`] doc comment). Per ADR-001 Refinements, the two
/// concrete forms are collected into this closed `enum`.
///
/// PORT NOTE (ADR-002): `#[serde(untagged)]` rather than `#[serde(tag =
/// "_type")]` — each variant already carries its own `TypeTag`
/// (`DvCodedText`'s own self-tag field; `Text`'s explicit
/// `TypeTag<DvTextData>` beside the flattened data), whose `Deserialize`
/// fails on a mismatched `_type` string, making untagged probing tag-driven
/// instead of structure-driven. A container-level `tag` here would
/// duplicate the payloads' own tags. Variant order still lists the
/// structurally richer shape first: `Coded` before `Text`, so tag-less
/// input carrying `defining_code` is not swallowed by the weaker `Text`
/// arm.
///
/// The former `TODO(port)` about `DvText::Text` serializing with **no
/// `_type` discriminator at all** is resolved by this shape: the `Text`
/// struct variant now emits `"_type": "DV_TEXT"` first, alongside the
/// flattened `DvTextData` fields (ADR-002 §3's bare-concrete-parent
/// exception).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DvText {
    /// A `DV_CODED_TEXT` instance, substitutable wherever `DV_TEXT` is
    /// declared. Listed first — see the enum-level PORT NOTE on untagged
    /// variant ordering.
    Coded(DvCodedText),
    /// A "bare" `DV_TEXT` instance: plain or markdown-formatted text with
    /// no attached defining code.
    ///
    /// PORT NOTE (ADR-002): a struct variant, not a newtype variant, so the
    /// `_type: "DV_TEXT"` discriminator can sit beside the flattened
    /// shared-state struct (`DvTextData` itself carries no tag — it is also
    /// `#[serde(flatten)]`ed into `DvCodedText`, where a second tag would
    /// collide).
    Text {
        /// Canonical `_type` discriminator (`"DV_TEXT"`), always serialized
        /// first (ADR-002).
        #[serde(rename = "_type", default = "TypeTag::new")]
        type_tag: TypeTag<DvTextData>,
        /// The six shared `DV_TEXT` attributes, flattened onto the same
        /// JSON object as the discriminator.
        #[serde(flatten)]
        data: DvTextData,
    },
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
            DvText::Text { data, .. } => data.value(),
            DvText::Coded(v) => v.value(),
        }
    }
    fn hyperlink(&self) -> Option<&DvUri> {
        match self {
            DvText::Text { data, .. } => data.hyperlink(),
            DvText::Coded(v) => v.hyperlink(),
        }
    }
    fn formatting(&self) -> Option<&str> {
        match self {
            DvText::Text { data, .. } => data.formatting(),
            DvText::Coded(v) => v.formatting(),
        }
    }
    fn mappings(&self) -> Option<&[TermMapping]> {
        match self {
            DvText::Text { data, .. } => data.mappings(),
            DvText::Coded(v) => v.mappings(),
        }
    }
    fn language(&self) -> Option<&CodePhrase> {
        match self {
            DvText::Text { data, .. } => data.language(),
            DvText::Coded(v) => v.language(),
        }
    }
    fn encoding(&self) -> Option<&CodePhrase> {
        match self {
            DvText::Text { data, .. } => data.encoding(),
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
            DvText::Text { .. } => TYPE_NAME,
            DvText::Coded(v) => v.type_name(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare_text(value: &str) -> DvTextData {
        DvTextData {
            value: value.to_string(),
            hyperlink: None,
            formatting: None,
            mappings: None,
            language: None,
            encoding: None,
        }
    }

    /// ADR-002 round-trip: a bare `DvText::Text` self-tags `"DV_TEXT"` as
    /// the first key, and a `"_type": "DV_CODED_TEXT"` payload dispatches
    /// to the `Coded` variant; both round-trip to equality.
    #[test]
    fn dv_text_round_trips_with_type_discriminators() {
        // Bare DV_TEXT: serializes with the discriminator first.
        let text = DvText::Text {
            type_tag: TypeTag::new(),
            data: bare_text("plain prose"),
        };
        let json = serde_json::to_string(&text).unwrap();
        assert!(
            json.starts_with(r#"{"_type":"DV_TEXT","#),
            "bare DV_TEXT must self-tag first, got: {json}"
        );
        let text_back: DvText = serde_json::from_str(&json).unwrap();
        assert_eq!(text_back, text);

        // DV_CODED_TEXT input: the _type tag must drive dispatch to Coded.
        let coded_json = r#"{"_type":"DV_CODED_TEXT","value":"event","defining_code":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"openehr"},"code_string":"433"}}"#;
        let coded: DvText = serde_json::from_str(coded_json).unwrap();
        assert!(matches!(coded, DvText::Coded(_)));
        let coded_round = serde_json::to_string(&coded).unwrap();
        assert!(
            coded_round.starts_with(r#"{"_type":"DV_CODED_TEXT","#),
            "DV_CODED_TEXT must self-tag first, got: {coded_round}"
        );
        let coded_back: DvText = serde_json::from_str(&coded_round).unwrap();
        assert_eq!(coded_back, coded);
    }

    /// A `CODE_PHRASE` built from JSON so the test does not hard-code the
    /// `TerminologyId` field shape (owned by `openehr-base`); a missing
    /// `_type` is tolerated on concrete slots per ADR-002.
    fn code_phrase(code: &str) -> CodePhrase {
        serde_json::from_value(serde_json::json!({
            "terminology_id": { "value": "openehr" },
            "code_string": code,
        }))
        .unwrap()
    }

    /// `Language_valid`/`Encoding_valid` check membership in the bundled
    /// openEHR "languages"/"character sets" code sets: `Void` is trivially
    /// valid, a real ISO 639-1 / IANA charset code passes, an unknown code
    /// fails.
    #[test]
    fn language_and_encoding_invariants_check_bundled_code_sets() {
        let terminology = TerminologyService::bundled().expect("bundled terminology");

        let mut data = bare_text("hi");
        assert!(data.invariant_language_valid(terminology));
        assert!(data.invariant_encoding_valid(terminology));

        data.language = Some(code_phrase("en"));
        data.encoding = Some(code_phrase("UTF-8"));
        assert!(data.invariant_language_valid(terminology));
        assert!(data.invariant_encoding_valid(terminology));

        data.language = Some(code_phrase("zz"));
        data.encoding = Some(code_phrase("not-a-charset"));
        assert!(!data.invariant_language_valid(terminology));
        assert!(!data.invariant_encoding_valid(terminology));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.text — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_text.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-text_package.adoc §Class Descriptions / dv_text.adoc §DV_TEXT Class
//   confidence: high
//   todos: 0
//   note: DvTextData+DvText(enum)+DvTextApi triple applied to a *concrete* (not spec-abstract) parent class, extending the ADR-001 Refinements pattern to DV_TEXT/DV_CODED_TEXT substitutability (the one load-bearing use site in this cluster is DV_PARAGRAPH.items: List<DV_TEXT>). Language_valid/Encoding_valid now implemented per ADR-003 decision 8: both take a &TerminologyService and check membership in the bundled "languages"/"character sets" code sets (in-file test pins Void/valid/invalid). P4/ADR-002: DvTextData implements TypeName ("DV_TEXT") but carries no tag field (it is flattened into DvCodedText); DvText::Text is a struct variant {type_tag: TypeTag<DvTextData>, #[serde(flatten)] data} so a bare DV_TEXT self-tags; enum stays #[serde(untagged)], Coded first; round-trip pinned by the in-file unit test.
// ─────────────────────────────────────────────
