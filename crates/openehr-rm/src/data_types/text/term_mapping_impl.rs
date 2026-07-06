//! Hand-written RM class invariant (ADR-003) for `TERM_MAPPING`.
//!
//! `Match_valid` (archie `TermMapping.VALID_MATCH_CODES`): `match` must be one
//! of `< = > ?`.
//!
//! PORT NOTE: archie also has `Purpose_valid` (the purpose `DV_CODED_TEXT` must
//! belong to the openEHR "term mapping purpose" group) — a terminology-bound
//! check deferred to the composition validator + `openehr-term` (this crate has
//! no terminology dependency). The spec's "purpose is DV_CODED_TEXT" is a
//! structural guarantee here (`purpose: Option<DvCodedText>`).

use crate::data_types::text::term_mapping::TermMapping;
use crate::validate::{InvariantViolation, Validate};

impl Validate for TermMapping {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !matches!(self.r#match, '<' | '=' | '>' | '?') {
            out.push(InvariantViolation::here(
                "Invariant Match_valid failed on type TERM_MAPPING",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::code_phrase::CodePhrase;
    use openehr_base::prelude::TerminologyId;

    fn term_mapping(m: char) -> TermMapping {
        TermMapping {
            r#match: m,
            purpose: None,
            target: CodePhrase {
                terminology_id: TerminologyId {
                    value: "SNOMED-CT".to_owned(),
                },
                code_string: "123".to_owned(),
                preferred_term: None,
            },
        }
    }

    #[test]
    fn valid_match_codes() {
        for m in ['<', '=', '>', '?'] {
            assert!(
                term_mapping(m).invariants().is_empty(),
                "{m} should be valid"
            );
        }
    }

    #[test]
    fn invalid_match_code() {
        let v = term_mapping('Q').invariants();
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "Invariant Match_valid failed on type TERM_MAPPING"
        );
    }
}
