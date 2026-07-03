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
use openehr_term::{
    OpenehrTerminologyGroupIdentifiers, TerminologyAccess, TerminologyCode, TerminologyService,
};

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
    /// Checked by [`PartyRelated::is_relationship_valid`] (ADR-003 d.8)
    /// against the openEHR "subject relationship" group; P11 Validate-
    /// framework wiring is pending.
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
    /// Working method per ADR-003 decision 8 (terminology-bound invariants
    /// take `&TerminologyService`). `relationship` is a mandatory
    /// `DV_CODED_TEXT`, so its `defining_code` is checked directly against
    /// the openEHR "subject relationship" group (no `DV_CODED_TEXT` runtime
    /// discrimination and no `/= Void` antecedent, unlike the conditional
    /// invariants on `ATTESTATION`/`PARTICIPATION`).
    pub fn is_relationship_valid(&self, terminology: &TerminologyService) -> bool {
        let defining_code = &self.relationship.defining_code;
        terminology
            .terminology(OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR)
            .is_some_and(|access| {
                access.has_code_for_group_id(
                    OpenehrTerminologyGroupIdentifiers::GROUP_ID_SUBJECT_RELATIONSHIP,
                    &TerminologyCode::new(
                        defining_code.terminology_id.value(),
                        defining_code.code_string.clone(),
                    ),
                )
            })
    }
}

impl PartyProxyApi for PartyRelated {
    fn external_ref(&self) -> Option<&PartyRef> {
        self.party_identified.external_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::change_control::versioned_object::test_support::coded;
    use crate::common::generic::party_proxy::PartyProxyData;

    fn party_related(relationship: DvCodedText) -> PartyRelated {
        PartyRelated {
            type_tag: TypeTag::new(),
            party_identified: PartyIdentifiedData {
                party_proxy: PartyProxyData { external_ref: None },
                name: Some("A. Carer".to_string()),
                identifiers: None,
            },
            relationship,
        }
    }

    #[test]
    fn relationship_valid_checks_the_subject_relationship_group() {
        let service = TerminologyService::bundled().expect("bundled terminology parses");
        // 10 = "mother" in the openEHR "subject relationship" group.
        let ok = party_related(coded("10", "mother"));
        assert!(ok.is_relationship_valid(service));

        // A bogus code is not in the group.
        let bad = party_related(coded("999999", "nonsense"));
        assert!(!bad.is_relationship_valid(service));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/party_related.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Referring to Demographic Entities / uml_classes/party_related.adoc §PARTY_RELATED Class
//   confidence: high
//   todos: 0
//   note: Relationship_valid now a working method (ADR-003 d.8) with a spec-derived test: mandatory DV_CODED_TEXT relationship checked against the openEHR "subject relationship" group via &TerminologyService (no runtime discrimination / no /= Void antecedent). Only remaining deferral is P11 Validate-framework wiring. Embeds the PARTY_IDENTIFIED field set per its Inherit row; two-level composition chain (PartyRelated -> PartyIdentifiedData -> PartyProxyData) kept unflattened. P4/ADR-002: self-tags via TypeName + first-field TypeTag<Self> (_type = "PARTY_RELATED"); embedded parent is the untagged PartyIdentifiedData so no inner _type leaks.
// ─────────────────────────────────────────────
