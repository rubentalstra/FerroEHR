//! `PARTY_RELATED` — a party and its relationship to the subject of the
//! record.
//!
//! openEHR class: `PARTY_RELATED` (concrete), package `common.generic`.
//! Inherits: `PARTY_IDENTIFIED`.
//!
//! Proxy type for identifying a party and its relationship to the subject
//! of the record. Use where the relationship between the party and the
//! subject of the record must be known.
//!
//! The `RELATED_PARTY` concept is used whenever the relationship of the
//! party to the record subject is required. Relationships are coded and
//! include familial ones ("mother", "uncle", etc) as well as relationships
//! like "donor", "travelling companion" and so on.
use openehr_base::identification::party_ref::PartyRef;
use serde::{Deserialize, Serialize};

// TODO(port): `DV_CODED_TEXT` is RM 1.1.0 `data_types.text`, transcribed
// by a sibling agent in this same phase but not yet landed in this
// worktree. Forward-reference to its eventual module path.
use crate::data_types::text::dv_coded_text::DvCodedText;
use openehr_foundation::serde_support::{TypeName, TypeTag};

use super::party_identified::PartyIdentifiedData;
use super::party_proxy::PartyProxyApi;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sources the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "PARTY_RELATED";

/// `PARTY_RELATED inherits PARTY_IDENTIFIED` per its `Inherit` row. Per
/// ADR-001 §3, the entire inherited attribute set (`external_ref`, `name`,
/// `identifiers`) is carried via an embedded field, with the one new
/// attribute (`relationship`) declared directly on this struct. This is a
/// two-level composition chain: `PartyRelated` embeds the
/// `PARTY_IDENTIFIED` field set, which itself embeds `PartyProxyData`
/// (see `party_identified.rs`) — each level's own file embeds only its
/// immediate parent's field set, not a flattened copy of the whole
/// ancestor chain, so a change to `PARTY_PROXY`'s or `PARTY_IDENTIFIED`'s
/// own attribute set only requires touching that one file.
///
/// PORT NOTE (ADR-002 restructure): the embedded field is
/// [`PartyIdentifiedData`] (the untagged shared field carrier), not the
/// self-tagged concrete `PartyIdentified` struct it was pre-P4 —
/// flattening the concrete parent would emit a second, wrong
/// `_type: "PARTY_IDENTIFIED"` key inside `PARTY_RELATED` output. See the
/// PORT NOTE on `PartyIdentifiedData` for the full rationale.
// PartialOrd/Ord dropped from the derive set: `relationship` is a
// `DV_CODED_TEXT`, which derives no ordering (the spec defines none), and
// the embedded `PARTY_IDENTIFIED` state dropped its ordering for the same
// reason.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartyRelated {
    /// Canonical `_type` discriminator (`"PARTY_RELATED"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `PARTY_IDENTIFIED` state (`external_ref`, `name`,
    /// `identifiers`), untagged — see the struct-level PORT NOTE.
    #[serde(flatten)]
    pub party_identified: PartyIdentifiedData,

    /// `relationship`: `DV_CODED_TEXT`, cardinality `1..1`.
    ///
    /// Relationship of subject of this `ENTRY` to the subject of the
    /// record. May be coded. If it is the patient, coded as "self".
    ///
    /// Invariant `Relationship_valid`:
    /// `terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_subject_relationship, relationship.defining_code)`.
    ///
    /// TODO(port): invariant references the `TERMINOLOGY_SERVICE`
    /// (`openehr_terminology::TerminologyService`) and the openEHR
    /// Terminology group "subject relationship"
    /// (`OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS`, already transcribed in
    /// `openehr-terminology`); not yet wired into a constructor or the RM
    /// `Validate` framework.
    pub relationship: DvCodedText,
}

impl TypeName for PartyRelated {
    const NAME: &'static str = TYPE_NAME;
}

impl PartyRelated {
    /// Invariant `Relationship_valid`:
    /// `terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_subject_relationship, relationship.defining_code)`.
    ///
    /// TODO(port): requires a live `TerminologyService` instance to check
    /// `relationship.defining_code` against the "subject relationship"
    /// openEHR Terminology group; the `Validate` framework this will
    /// eventually be wired through (context + path + error accumulator)
    /// is not yet implemented. Left as `todo!()` rather than a bare
    /// boolean stub, since — unlike the invariants in sibling files that
    /// close over only their own struct's fields — this one cannot be
    /// evaluated without external service state.
    pub fn is_relationship_valid(
        &self,
        _terminology: &openehr_terminology::TerminologyService,
    ) -> bool {
        todo!(
            "PartyRelated::is_relationship_valid: needs TerminologyService.has_code_for_group_id against Group_id_subject_relationship"
        )
    }
}

impl PartyProxyApi for PartyRelated {
    fn external_ref(&self) -> Option<&PartyRef> {
        self.party_identified.external_ref()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/party_related.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Referring to Demographic Entities / uml_classes/party_related.adoc §PARTY_RELATED Class
//   confidence: high
//   todos: 1
//   note: Relationship_valid invariant left as a todo!()-bodied method (needs live TerminologyService + Validate framework, not a self-contained boolean check). Embeds the PARTY_IDENTIFIED field set per its Inherit row; two-level composition chain (PartyRelated -> PartyIdentifiedData -> PartyProxyData) kept unflattened so each ancestor's own file stays the single place its attribute set is declared. P4/ADR-002: self-tags via TypeName + first-field TypeTag<Self> (_type = "PARTY_RELATED"); embedded parent switched from the self-tagged PartyIdentified to the untagged PartyIdentifiedData so no inner _type leaks.
// ─────────────────────────────────────────────
