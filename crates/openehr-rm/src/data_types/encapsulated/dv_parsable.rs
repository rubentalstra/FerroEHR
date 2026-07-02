//! `DV_PARSABLE` — encapsulated data expressed as a parsable string.
//!
//! openEHR class: `DV_PARSABLE`, package `rm.data_types.encapsulated`.
//! Inherits: `DV_ENCAPSULATED`.
//!
//! Encapsulated data expressed as a parsable String. The internal model of
//! the data item is not described in the openEHR model in common with other
//! encapsulated types, but in this case, the form of the data is assumed to
//! be plaintext, rather than compressed or other types of large binary
//! data.
//!
//! This is the class `DV_TIME_SPECIFICATION.value` (see
//! `../time_specification/dv_time_specification.rs`) is itself typed as,
//! carrying the HL7v3 `PIVL`/`EIVL`/GTS syntax text.
use crate::data_types::encapsulated::dv_encapsulated::{DvEncapsulatedApi, DvEncapsulatedData};
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `DV_PARSABLE`.
///
/// openEHR class: `DV_PARSABLE`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvParsable {
    /// Canonical `_type` discriminator (`"DV_PARSABLE"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_ENCAPSULATED` state (`charset`, `language`).
    #[serde(flatten)]
    pub encapsulated: DvEncapsulatedData,

    /// `value`: `String` (`1..1`).
    ///
    /// The string, which may validly be empty in some syntaxes.
    pub value: String,

    /// `formalism`: `String` (`1..1`).
    ///
    /// Name of the formalism, e.g. "GLIF 1.0", "Proforma" etc.
    ///
    /// Invariant `Formalism_valid`: `not formalism.is_empty`.
    pub formalism: String,
}

pub const TYPE_NAME: &str = "DV_PARSABLE";

impl TypeName for DvParsable {
    const NAME: &'static str = TYPE_NAME;
}

impl DvParsable {
    /// `size` `(): Integer`.
    ///
    /// Size in bytes of value.
    ///
    /// PORT NOTE: "size in bytes" of a `String` is ambiguous between UTF-8
    /// byte length (`str::len()`) and character/codepoint count
    /// (`str::chars().count()`) once non-ASCII content is involved. Every
    /// other `String`-carrying RM class in this crate assumes UTF-8 per the
    /// `openehr_foundation::primitive_types::string::OpenEhrString`
    /// transcription's own convention (see `docs/ROSETTA.md`'s `String`
    /// row), so `str::len()` (UTF-8 byte length) is used here rather than a
    /// character count, consistent with that established convention.
    pub fn size(&self) -> i32 {
        // TODO(port): spec `size()` returns `Integer` (32-bit); a `value`
        // longer than `i32::MAX` bytes cannot be faithfully represented by
        // this cast. Same class of unspecified-overflow gap flagged
        // elsewhere in this crate (e.g.
        // `openehr_foundation::structure_types::list::List::count`).
        self.value.len() as i32
    }

    /// `Formalism_valid` invariant: `not formalism.is_empty`.
    pub fn invariant_formalism_valid(&self) -> bool {
        !self.formalism.is_empty()
    }

    /// `Size_valid` invariant: `size >= 0`.
    ///
    /// PORT NOTE: since `size()` is derived from `value.len()` (an unsigned
    /// count cast to `i32`), this invariant can only be violated by the
    /// `i32`-overflow edge case flagged on `size()` above (a byte length
    /// exceeding `i32::MAX` wrapping negative) — it is not a condition that
    /// can be violated by any ordinary `value` content. Transcribed
    /// literally regardless, since the spec states it unconditionally, not
    /// merely "for construction from any positive Integer".
    pub fn invariant_size_valid(&self) -> bool {
        self.size() >= 0
    }
}

impl DvEncapsulatedApi for DvParsable {
    fn encapsulated_data(&self) -> &DvEncapsulatedData {
        &self.encapsulated
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.encapsulated — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_parsable.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master09-encapsulated_package.adoc §Class Descriptions / dv_parsable.adoc §DV_PARSABLE Class
//   confidence: high
//   todos: 1
//   note: fully implemented (no terminology-service dependency, unlike its DvMultimedia sibling) — value/formalism/size and both invariants (Formalism_valid, Size_valid) are complete; size()'s i32-cast-of-UTF-8-byte-length carries the same unspecified-overflow gap noted on List::count and other RM byte/length functions. P4: Serialize/Deserialize added; `encapsulated` flattened; value/formalism are mandatory, no skip needed; ADR-002 self-tagging applied (TypeTag<Self> first field + TypeName from TYPE_NAME).
// ─────────────────────────────────────────────
