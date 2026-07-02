//! `INTERNET_ID` — reverse internet domain identifier.
//!
//! openEHR class: `INTERNET_ID`, package `base.base_types.identification`.
//! Inherits: `UID`.
//!
//! Model of a reverse internet domain, as used to uniquely identify an
//! internet domain. In the form of a dot-separated string in the reverse
//! order of a domain name, specified by IETF RFC 1034.
//!
//! Lexical form (Syntaxes, BASE 1.2.0 identification package):
//! `internet_id = subdomain ; subdomain = label | subdomain, '.', label ;`
use super::uid::{Uid, UidApi, UidData};

/// `INTERNET_ID` declares no attributes or functions of its own beyond
/// those inherited from `UID`, so it embeds `UidData` verbatim (ADR-001
/// §3).
///
/// `#[serde(flatten)]` on the embedded `uid` field folds `UidData`'s single
/// `value` attribute directly into this struct's JSON object, matching the
/// same convention used on `IsoOid`/`Uuid` in this package.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct InternetId {
    /// Embedded `UID` state (the single `value` attribute).
    #[serde(flatten)]
    pub uid: UidData,
}

impl InternetId {
    /// `value`: the value of the id, in the reverse-domain lexical form
    /// defined by the identification package grammar (RFC 1034/1035,
    /// relaxed per RFC 1123, plus underscores).
    pub fn value(&self) -> &str {
        &self.uid.value
    }
}

impl UidApi for InternetId {
    fn value(&self) -> &str {
        &self.uid.value
    }
}

impl From<InternetId> for Uid {
    fn from(value: InternetId) -> Self {
        Uid::InternetId(value)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §INTERNET_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/internet_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / internet_id.adoc §INTERNET_ID Class
//   confidence: high
//   todos: 0
//   note: pure UID subtype with no added attributes/functions; RFC 1034/1035/1123 domain-label validation not yet enforced.
// ─────────────────────────────────────────────
