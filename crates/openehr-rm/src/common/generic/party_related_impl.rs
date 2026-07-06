//! Hand-written RM class invariants (ADR-003) for `PARTY_RELATED`.
//!
//! `PARTY_RELATED` extends `PARTY_IDENTIFIED` and so inherits `Basic_validity`
//! and `Name_valid`.
//!
//! PORT NOTE: archie's own `PartyRelated` invariant `Relationship_valid` (the
//! relationship code belongs to the openEHR "subject relationship" group) is
//! terminology-bound — deferred to the composition validator + `openehr-term`.

use crate::common::generic::party_related::PartyRelated;
use crate::validate::{InvariantViolation, Validate};

impl Validate for PartyRelated {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.name.is_none() && self.identifiers.is_empty() && self.external_ref.is_none() {
            out.push(InvariantViolation::here(
                "Invariant Basic_validity failed on type PARTY_RELATED",
            ));
        }
        if self.name.as_deref().is_some_and(str::is_empty) {
            out.push(InvariantViolation::here(
                "Invariant Name_valid failed on type PARTY_RELATED",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_coded_text::DvCodedText;
    use openehr_base::prelude::TerminologyId;

    fn relationship() -> DvCodedText {
        DvCodedText {
            value: "mother".to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: Vec::new(),
            language: None,
            encoding: None,
            defining_code: CodePhrase {
                terminology_id: TerminologyId {
                    value: "openehr".to_owned(),
                },
                code_string: "10".to_owned(),
                preferred_term: None,
            },
        }
    }

    fn party(name: Option<&str>) -> PartyRelated {
        PartyRelated {
            external_ref: None,
            name: name.map(str::to_owned),
            identifiers: Vec::new(),
            relationship: relationship(),
        }
    }

    #[test]
    fn named_party_valid() {
        assert!(party(Some("Jane")).invariants().is_empty());
    }

    #[test]
    fn no_identity_invalid() {
        let v = party(None).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Basic_validity failed on type PARTY_RELATED")
        );
    }
}
