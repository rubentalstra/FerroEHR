//! `ISO_OID` — ISO Object Identifier.
//!
//! openEHR class: `ISO_OID`, package `base.base_types.identification`.
//! Inherits: `UID`.
//!
//! Model of ISO's Object Identifier (oid) as defined by the standard
//! ISO/IEC 8824. Oids are formed from integers separated by dots. Each
//! non-leaf node in an Oid starting from the left corresponds to an
//! assigning authority, and identifies that authority's namespace, inside
//! which the remaining part of the identifier is locally unique.
//!
//! Lexical form (Syntaxes, BASE 1.2.0 identification package):
//! `iso_oid = number, { '.', number } ;`
use super::uid::{Uid, UidApi, UidData};

/// `ISO_OID` declares no attributes or functions of its own beyond those
/// inherited from `UID`, so it embeds `UidData` verbatim (ADR-001 §3).
///
/// `#[serde(flatten)]` on the embedded `uid` field folds `UidData`'s single
/// `value` attribute directly into this struct's JSON object, so a bare
/// `IsoOid` (outside the `Uid` enum's `_type`-tagged context) still
/// serializes as `{"value": "..."}` rather than `{"uid": {"value": "..."}}`.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct IsoOid {
    /// Embedded `UID` state (the single `value` attribute).
    #[serde(flatten)]
    pub uid: UidData,
}

impl IsoOid {
    /// `value`: the value of the id, in the `number { '.' number }` lexical
    /// form defined by the identification package grammar.
    pub fn value(&self) -> &str {
        &self.uid.value
    }
}

impl UidApi for IsoOid {
    fn value(&self) -> &str {
        &self.uid.value
    }
}

impl From<IsoOid> for Uid {
    fn from(value: IsoOid) -> Self {
        Uid::IsoOid(value)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §ISO_OID — docs/research/spec-cache/BASE-1.2.0/uml_classes/iso_oid.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / iso_oid.adoc §ISO_OID Class
//   confidence: high
//   todos: 0
//   note: pure UID subtype with no added attributes/functions; grammar-form validation (dot-separated numbers) not yet enforced.
// ─────────────────────────────────────────────
