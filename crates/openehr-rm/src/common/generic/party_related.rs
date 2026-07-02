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

// TODO(port): `DV_CODED_TEXT` is RM 1.1.0 `data_types.text`, transcribed
// by a sibling agent in this same phase but not yet landed in this
// worktree. Forward-reference to its eventual module path.
use crate::data_types::text::dv_coded_text::DvCodedText;

use super::party_identified::PartyIdentified;
use super::party_proxy::PartyProxyApi;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "PARTY_RELATED";

/// `PARTY_RELATED inherits PARTY_IDENTIFIED` per its `Inherit` row. Per
/// ADR-001 §3, the entire inherited attribute set (`external_ref`, `name`,
/// `identifiers`) is carried via an embedded [`PartyIdentified`] field,
/// with the one new attribute (`relationship`) declared directly on this
/// struct. This mirrors the task's own framing ("embeds PARTY_IDENTIFIED
/// per its Inherit row") and is a two-level composition chain:
/// `PartyRelated` embeds `PartyIdentified`, which itself embeds
/// `PartyProxyData` (see `party_identified.rs`) — each level's own file
/// embeds only its immediate parent's struct, not a flattened copy of the
/// whole ancestor chain, so a change to `PARTY_PROXY`'s or
/// `PARTY_IDENTIFIED`'s own attribute set only requires touching that
/// one file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartyRelated {
    /// Embedded `PARTY_IDENTIFIED` state (`external_ref`, `name`,
    /// `identifiers`).
    pub party_identified: PartyIdentified,

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
//   note: Relationship_valid invariant left as a todo!()-bodied method (needs live TerminologyService + Validate framework, not a self-contained boolean check). Embeds PartyIdentified per its Inherit row, per the task framing; two-level composition chain (PartyRelated -> PartyIdentified -> PartyProxyData) kept unflattened so each ancestor's own file stays the single place its attribute set is declared.
// ─────────────────────────────────────────────
