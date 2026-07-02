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
use serde::{Deserialize, Serialize};

/// Embedded parent state for `DV_ENCAPSULATED`'s attributes.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait), every concrete `DV_ENCAPSULATED` subtype (`DvMultimedia`,
/// `DvParsable`) embeds this struct rather than inheriting from it.
///
/// PORT NOTE (P4, ADR-002): the closed [`DvEncapsulated`] enum below (added
/// for `FEEDER_AUDIT.original_content`, the first attribute typed as the
/// abstract class) is `#[serde(untagged)]`; dispatch is driven by each
/// variant payload's own `TypeTag` (`DvMultimedia`/`DvParsable` self-tag).
/// This Data struct is an abstract embedded layer and carries **no** tag of
/// its own — it is flattened into each concrete descendant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvEncapsulatedData {
    /// `charset`: `CODE_PHRASE` (`0..1`).
    ///
    /// Name of character encoding scheme in which this value is encoded.
    /// Coded from openEHR Code Set "character sets". Unicode is the default
    /// assumption in openEHR, with UTF-8 being the assumed encoding. This
    /// attribute allows for variations from these assumptions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charset: Option<CodePhrase>,

    /// `language`: `CODE_PHRASE` (`0..1`).
    ///
    /// Optional indicator of the localised language in which the data is
    /// written, if relevant. Coded from openEHR Code Set `languages`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<CodePhrase>,
}

/// `DV_ENCAPSULATED` is modelled as a Rust trait requiring an accessor for
/// the embedded `DvEncapsulatedData` state, so code can stay polymorphic
/// over any concrete `DV_ENCAPSULATED` implementor (`DvMultimedia`,
/// `DvParsable`) without downcasting first.
///
/// Per ADR-001 §4, the two concrete descendants of `DV_ENCAPSULATED` form a
/// small closed subtype set, realised as the [`DvEncapsulated`] enum below
/// (added when `FEEDER_AUDIT.original_content` became the first attribute
/// typed as the abstract class).
pub trait DvEncapsulatedApi {
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

/// Closed subtype set of `DV_ENCAPSULATED` per ADR-001 §4: exactly two
/// concrete descendants exist in RM 1.1.0. Added when
/// `FEEDER_AUDIT.original_content: DV_ENCAPSULATED [0..1]` became the first
/// attribute typed as the abstract class (the trait alone cannot be a field
/// type).
///
/// PORT NOTE (ADR-002): `#[serde(untagged)]`, never `#[serde(tag =
/// "_type")]` — dispatch is driven by each payload's own `TypeTag` field
/// (`DvMultimedia`/`DvParsable` self-tag with their canonical class names),
/// whose `Deserialize` fails on a mismatched `_type` string, so serde's
/// variant probing is tag-driven; an internally-tagged enum would duplicate
/// the payload's own `_type` key on output. Structurally richer
/// `Multimedia` is listed before `Parsable` so tag-less input in
/// concrete-declared slots resolves correctly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DvEncapsulated {
    /// `DV_MULTIMEDIA`.
    Multimedia(super::dv_multimedia::DvMultimedia),
    /// `DV_PARSABLE`.
    Parsable(super::dv_parsable::DvParsable),
}

impl DvEncapsulatedApi for DvEncapsulated {
    fn encapsulated_data(&self) -> &DvEncapsulatedData {
        match self {
            DvEncapsulated::Multimedia(m) => m.encapsulated_data(),
            DvEncapsulated::Parsable(p) => p.encapsulated_data(),
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.encapsulated — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_encapsulated.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master09-encapsulated_package.adoc §Class Descriptions / dv_encapsulated.adoc §DV_ENCAPSULATED Class
//   confidence: medium
//   todos: 2
//   note: abstract class with attributes -> embedded Data struct + Api trait (ADR-001 §3) + closed DvEncapsulated enum (§4, added for FEEDER_AUDIT.original_content); both invariants require a live TerminologyService code_set lookup not yet threaded through any Validate-context signature — left as trait default todo!() rather than omitted. P4: Serialize/Deserialize added; both Option fields skip when None; ADR-002 applied — the DvEncapsulated enum is #[serde(untagged)] (dispatch via each payload's own TypeTag, richer Multimedia variant first), the abstract DvEncapsulatedData layer carries no tag.
// ─────────────────────────────────────────────
