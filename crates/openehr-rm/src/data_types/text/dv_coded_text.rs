//! `DV_CODED_TEXT` — a text item whose value is a controlled-terminology
//! rubric.
//!
//! openEHR class: `DV_CODED_TEXT`, package `rm.data_types.text`.
//! Inherits: `DV_TEXT`.
//!
//! A text item whose value must be the rubric from a controlled
//! terminology, the key (i.e. the 'code') of which is the `defining_code`
//! attribute. In other words: a `DV_CODED_TEXT` is a combination of a
//! `CODE_PHRASE` (effectively a code) and the rubric of that term, from a
//! terminology service, in the language in which the data were authored.
//!
//! Since `DV_CODED_TEXT` is a subtype of `DV_TEXT`, it can be used in place
//! of it, effectively allowing the type `DV_TEXT` to mean "a text item,
//! which may optionally be coded."
//!
//! Misuse: If the intention is to represent a term code attached in some
//! way to a fragment of plain text, `DV_CODED_TEXT` should not be used;
//! instead use a `DV_TEXT` and a `TERM_MAPPING` to a `CODE_PHRASE`.
use super::code_phrase::CodePhrase;
use super::dv_text::{DvTextApi, DvTextData};
use crate::data_types::data_value::DataValueApi;
use crate::data_types::uri::dv_uri::DvUri;

/// Canonical `_type` discriminator string for this class in serialized
/// form (ADR-001 Refinements: serde derives wait until P4).
pub const TYPE_NAME: &str = "DV_CODED_TEXT";

/// `DV_CODED_TEXT` inherits `DV_TEXT` (a concrete class) and adds exactly
/// one attribute of its own, `defining_code`. Per ADR-001 §3 (abstract
/// class with attributes → embedded struct), extended here to a concrete
/// parent per the [`super::dv_text::DvTextData`] doc comment, this struct
/// embeds `DvTextData` by composition (the `text` field) rather than
/// duplicating its six fields, so field types and order stay identical to
/// a "bare" `DV_TEXT`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DvCodedText {
    /// Embedded `DV_TEXT` state (`value`, `hyperlink`, `formatting`,
    /// `mappings`, `language`, `encoding`).
    pub text: DvTextData,

    /// `defining_code`: `CODE_PHRASE` (`1..1`).
    ///
    /// The term of which the `value` attribute is the textual rendition
    /// (i.e. rubric).
    pub defining_code: CodePhrase,
}

impl DvTextApi for DvCodedText {
    fn value(&self) -> &str {
        self.text.value()
    }
    fn hyperlink(&self) -> Option<&DvUri> {
        self.text.hyperlink()
    }
    fn formatting(&self) -> Option<&str> {
        self.text.formatting()
    }
    fn mappings(&self) -> Option<&[super::term_mapping::TermMapping]> {
        self.text.mappings()
    }
    fn language(&self) -> Option<&CodePhrase> {
        self.text.language()
    }
    fn encoding(&self) -> Option<&CodePhrase> {
        self.text.encoding()
    }
}

impl DataValueApi for DvCodedText {
    fn type_name(&self) -> &'static str {
        TYPE_NAME
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.text — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_coded_text.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-text_package.adoc §Class Descriptions / dv_coded_text.adoc §DV_CODED_TEXT Class
//   confidence: high
//   todos: 0
//   note: embeds DvTextData by composition (single `text` field) rather than duplicating DV_TEXT's six attributes; no invariants published on this class itself (all six inherited invariants live on DvTextData).
// ─────────────────────────────────────────────
