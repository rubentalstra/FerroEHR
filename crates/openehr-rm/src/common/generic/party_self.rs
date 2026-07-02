//! `PARTY_SELF` — party proxy representing the subject of the record.
//!
//! openEHR class: `PARTY_SELF` (concrete), package `common.generic`.
//! Inherits: `PARTY_PROXY`.
//!
//! Party proxy representing the subject of the record. Used to indicate
//! that the party is the owner of the record. May or may not have
//! `external_ref` set.
//!
//! There are three schemes which are likely to be used for referring to
//! the patient (i.e. the record subject) demographic or patient master
//! index (PMI) data from within the EHR, each valid in different
//! circumstances, all using a `PARTY_SELF` object but with varying usage
//! of `external_ref`:
//!
//! * `external_ref` is not set on any instance of `PARTY_SELF` anywhere in
//!   the EHR — the most secure approach; the EHR-to-patient link is made
//!   outside the EHR (e.g. via `EHR.ehr_id`).
//! * `external_ref` is set once only, in `EHR_STATUS.subject`.
//! * `external_ref` is set in every instance of `PARTY_SELF` — the most
//!   visible/convenient approach, reasonable in a secure environment.
use openehr_base::identification::party_ref::PartyRef;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

use super::party_proxy::{PartyProxyApi, PartyProxyData};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sources the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "PARTY_SELF";

/// `PARTY_SELF` adds no attribute of its own beyond the inherited
/// `external_ref` from `PARTY_PROXY` (its spec table has no `Attributes`
/// section at all), so it embeds [`PartyProxyData`] verbatim, per
/// ADR-001 §3.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PartySelf {
    /// Canonical `_type` discriminator (`"PARTY_SELF"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `PARTY_PROXY` state (`external_ref`).
    #[serde(flatten)]
    pub party_proxy: PartyProxyData,
}

impl TypeName for PartySelf {
    const NAME: &'static str = TYPE_NAME;
}

impl PartyProxyApi for PartySelf {
    fn external_ref(&self) -> Option<&PartyRef> {
        self.party_proxy.external_ref.as_ref()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/party_self.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §PARTY_SELF and Referring to the Patient from the EHR / uml_classes/party_self.adoc §PARTY_SELF Class
//   confidence: high
//   todos: 0
//   note: No new attributes or invariants beyond the embedded PARTY_PROXY state. P4/ADR-002: self-tags via TypeName + first-field TypeTag<Self> (_type = "PARTY_SELF"); a bare PartySelf serializes as exactly {"_type":"PARTY_SELF"} (pinned by a unit test in party_proxy.rs).
// ─────────────────────────────────────────────
