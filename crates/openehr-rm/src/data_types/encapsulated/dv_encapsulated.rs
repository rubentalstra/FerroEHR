//! `DV_ENCAPSULATED` — abstract ancestor of all encapsulated data types.
//!
//! openEHR class: `DV_ENCAPSULATED` (abstract), package
//! `rm.data_types.encapsulated`.
//! Inherits: `DATA_VALUE`.
//!
//! Abstract class defining the common meta-data of all types of
//! encapsulated data. The `data_types.encapsulated` package contains
//! classes representing data values whose internal structure is defined
//! outside the EHR model, such as multimedia and parsable data.
//!
//! # Forward references
//!
//! `DATA_VALUE` (the RM-wide closed abstract root, ADR-001 §4) and
//! `CODE_PHRASE` are both transcribed in the concurrent `data_types` root
//! and `data_types.text` clusters respectively, neither of which has landed
//! at the time of this transcription pass. `CODE_PHRASE` is imported
//! directly by its eventual module path per the invoking task's
//! instruction; `DATA_VALUE`'s own state is not embedded here since this
//! abstract class's own per-class table declares no attributes it inherits
//! from `DATA_VALUE` beyond the implicit closed-subtype-set membership.
use crate::data_types::text::code_phrase::CodePhrase;

/// Embedded parent state for `DV_ENCAPSULATED`'s attributes.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait), every concrete `DV_ENCAPSULATED` subtype (`DvMultimedia`,
/// `DvParsable`) embeds this struct rather than inheriting from it.
#[derive(Debug, Clone, PartialEq)]
pub struct DvEncapsulatedData {
    /// `charset`: `CODE_PHRASE` (`0..1`).
    ///
    /// Name of character encoding scheme in which this value is encoded.
    /// Coded from openEHR Code Set "character sets". Unicode is the default
    /// assumption in openEHR, with UTF-8 being the assumed encoding. This
    /// attribute allows for variations from these assumptions.
    pub charset: Option<CodePhrase>,

    /// `language`: `CODE_PHRASE` (`0..1`).
    ///
    /// Optional indicator of the localised language in which the data is
    /// written, if relevant. Coded from openEHR Code Set `languages`.
    pub language: Option<CodePhrase>,
}

/// `DV_ENCAPSULATED` is modelled as a Rust trait requiring an accessor for
/// the embedded `DvEncapsulatedData` state, so code can stay polymorphic
/// over any concrete `DV_ENCAPSULATED` implementor (`DvMultimedia`,
/// `DvParsable`) without downcasting first.
///
/// Per ADR-001 §4, the two concrete descendants of `DV_ENCAPSULATED` form a
/// small closed subtype set; an `Encapsulated` enum wrapping both could be
/// added once both concrete types are wired, but is not written in this
/// file for the same reason documented on `DvTimeSpecification` — no call
/// site in this transcription pass requires the closed-enum shape yet.
pub trait DvEncapsulated {
    /// Access to the embedded `DV_ENCAPSULATED` state.
    fn encapsulated_data(&self) -> &DvEncapsulatedData;

    /// `Language_valid` invariant:
    /// `language /= Void implies code_set(Code_set_id_languages).has_code(language)`.
    ///
    /// TODO(port): requires a live `TERMINOLOGY_SERVICE`/`code_set` lookup
    /// (`openehr_terminology::TerminologyService`, transcribed in P2) bound
    /// to `OPENEHR_CODE_SET_IDENTIFIERS::LANGUAGES`
    /// (`openehr_terminology::openehr_code_set_identifiers`); no service
    /// instance is threaded through this trait's signature yet — the RM
    /// invariant framework (a `Validate` trait taking a validation context,
    /// path, and error accumulator per `.claude/rules/rm-transcription.md`)
    /// is the natural home for that context parameter once it exists.
    fn invariant_language_valid(&self) -> bool {
        todo!(
            "DV_ENCAPSULATED.invariant_language_valid: requires a TerminologyService code_set(languages) lookup, not yet threaded through a Validate context"
        )
    }

    /// `Charset_valid` invariant:
    /// `charset /= Void implies code_set(Code_set_id_character_sets).has_code(charset)`.
    ///
    /// TODO(port): same shape as `invariant_language_valid` above, bound to
    /// `OPENEHR_CODE_SET_IDENTIFIERS::CHARACTER_SETS` instead.
    fn invariant_charset_valid(&self) -> bool {
        todo!(
            "DV_ENCAPSULATED.invariant_charset_valid: requires a TerminologyService code_set(character_sets) lookup, not yet threaded through a Validate context"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.encapsulated — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_encapsulated.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master09-encapsulated_package.adoc §Class Descriptions / dv_encapsulated.adoc §DV_ENCAPSULATED Class
//   confidence: medium
//   todos: 2
//   note: abstract class with attributes -> embedded Data struct + marker trait (ADR-001 §3); charset/language forward-reference the not-yet-landed CODE_PHRASE (data_types.text cluster, concurrent); both invariants require a live TerminologyService code_set lookup not yet threaded through any Validate-context signature — left as trait default todo!() rather than omitted.
// ─────────────────────────────────────────────
