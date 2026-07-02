//! `UUID` — DCE Universal Unique Identifier.
//!
//! openEHR class: `UUID`, package `base.base_types.identification`.
//! Inherits: `UID`.
//!
//! Model of the DCE Universal Unique Identifier or UUID which takes the
//! form of hexadecimal integers separated by hyphens, following the pattern
//! 8-4-4-4-12 as defined by the Open Group, CDE 1.1 Remote Procedure Call
//! specification, Appendix A. Also known as a GUID.
//!
//! Lexical form (Syntaxes, BASE 1.2.0 identification package):
//! `uuid = hex-number, '-', hex-number, '-', hex-number, '-', hex-number, '-', hex-number ;`
use super::uid::{Uid as UidEnum, UidApi, UidData};

/// Rust type name: `Uuid` (PascalCase of the spec's `UUID`).
///
/// PORT NOTE: this type is distinct from `uuid::Uuid` (the `uuid` crate
/// listed in CLAUDE.md's pinned dependency table). This transcription
/// deliberately does not depend on the `uuid` crate — per the task
/// instructions for this pass, the spec's `UUID` class is transcribed as a
/// string-backed value type (embedding `UidData`, i.e. a `value: String`),
/// mirroring the identification package's stated design decision ("A key
/// design decision has been to choose a string representation for all
/// identifiers ..."). A later phase may add a conversion to/from
/// `uuid::Uuid` once that crate dependency is wired into `openehr-base`;
/// until then this struct stands alone and no external `uuid` dependency is
/// introduced here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uuid {
    /// Embedded `UID` state (the single `value` attribute), holding the
    /// `8-4-4-4-12` hex-hyphenated string form.
    pub uid: UidData,
}

impl Uuid {
    /// `value`: the value of the id, in the hyphenated hex-digit lexical
    /// form defined by the identification package grammar.
    pub fn value(&self) -> &str {
        &self.uid.value
    }
}

impl UidApi for Uuid {
    fn value(&self) -> &str {
        &self.uid.value
    }
}

impl From<Uuid> for UidEnum {
    fn from(value: Uuid) -> Self {
        UidEnum::Uuid(value)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §UUID — docs/research/spec-cache/BASE-1.2.0/uml_classes/uuid.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / uuid.adoc §UUID Class
//   confidence: high
//   todos: 0
//   note: named Uuid to mirror the spec class name; deliberately not backed by the uuid crate in this pass, see PORT NOTE; grammar-form (8-4-4-4-12) validation not yet enforced.
// ─────────────────────────────────────────────
