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

use openehr_foundation::serde_support::{TypeName, TypeTag};

use super::uid::{Uid as UidEnum, UidApi, UidData};

/// Rust type name: `Uuid` (PascalCase of the spec's `UUID`).
///
/// PORT NOTE: this type is distinct from `uuid::Uuid` (the external crate
/// used by `uid.rs` only to validate the canonical 8-4-4-4-12 textual
/// form). The spec's `UUID` class remains a string-backed value type
/// embedding `UidData`, mirroring the identification package's stated
/// design decision: "A key design decision has been to choose a string
/// representation for all identifiers ...".
///
/// `#[serde(flatten)]` on the embedded `uid` field folds `UidData`'s single
/// `value` attribute directly into this struct's JSON object, matching the
/// same convention used on `IsoOid`/`InternetId` in this package, so a
/// `Uuid` serializes as the canonical `{"_type": "UUID", "value": "..."}`
/// UID shape (ADR-002 self-tag).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Uuid {
    /// Canonical `_type` discriminator (`"UUID"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `UID` state (the single `value` attribute), holding the
    /// `8-4-4-4-12` hex-hyphenated string form.
    #[serde(flatten)]
    pub uid: UidData,
}

impl TypeName for Uuid {
    const NAME: &'static str = "UUID";
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
//   note: named Uuid to mirror the spec class name; still string-backed per BASE design, with canonical textual validation centralised in uid.rs. P4/ADR-002: self-tags via TypeTag<Self> first field (no prior TYPE_NAME const existed in this file; NAME taken from the ITS-JSON schema string "UUID").
// ─────────────────────────────────────────────
