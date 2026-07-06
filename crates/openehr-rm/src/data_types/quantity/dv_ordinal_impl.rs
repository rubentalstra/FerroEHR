//! Hand-written RM class invariants (ADR-003) for `DV_ORDINAL`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_ordinal.adoc`.
//! `DV_ORDINAL` declares no own invariants; it inherits the DV_ORDERED
//! `Normal_range_and_status_consistency` (checked via the ordered-magnitude
//! machinery in `dv_ordered_impl`). The comparison functions
//! (`less_than` / `is_strictly_comparable_to`) live in `dv_ordered_impl`.
//!
//! PORT NOTE: the DV_ORDERED `Normal_status_validity` invariant (terminology)
//! is deferred to the composition validator + `openehr-term`; archie's
//! `Reference_range_valid` (symbol resolution) is archetype-level.

use crate::data_types::quantity::dv_ordered::DvOrdered;
use crate::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::data_types::quantity::dv_ordinal::DvOrdinal;
use crate::validate::{InvariantViolation, Validate};

impl Validate for DvOrdinal {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_normal_range_consistency(
            out,
            "DV_ORDINAL",
            self.normal_status.as_ref(),
            self.normal_range.as_deref(),
            &DvOrdered::DvOrdinal(self.clone()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_interval::DvInterval;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_coded_text::DvCodedText;
    use openehr_base::prelude::TerminologyId;

    fn symbol(v: &str) -> DvCodedText {
        DvCodedText {
            value: v.to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: Vec::new(),
            language: None,
            encoding: None,
            defining_code: CodePhrase {
                terminology_id: TerminologyId {
                    value: "local".to_owned(),
                },
                code_string: "at0005".to_owned(),
                preferred_term: None,
            },
        }
    }

    fn ordinal(value: i32) -> DvOrdinal {
        DvOrdinal {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            symbol: symbol("moderate"),
            value,
        }
    }

    #[test]
    fn plain_ordinal_valid() {
        assert!(ordinal(2).invariants().is_empty());
    }

    #[test]
    fn ordering_functions() {
        assert_eq!(ordinal(1).less_than(&ordinal(2)), Some(true));
        assert_eq!(ordinal(3).less_than(&ordinal(2)), Some(false));
        assert!(ordinal(1).is_strictly_comparable_to(&ordinal(2)));
    }

    #[test]
    fn normal_range_status_inconsistency_detected() {
        let mut o = ordinal(5);
        o.normal_range = Some(Box::new(DvInterval {
            lower: Some(DvOrdered::DvOrdinal(ordinal(0))),
            upper: Some(DvOrdered::DvOrdinal(ordinal(2))),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        }));
        // Out of range but flagged normal → inconsistent.
        o.normal_status = Some(CodePhrase {
            terminology_id: TerminologyId {
                value: "openehr_normal_statuses".to_owned(),
            },
            code_string: "N".to_owned(),
            preferred_term: None,
        });
        let v = o.invariants();
        assert!(
            v.iter().any(|m| m.message
                == "Invariant Normal_range_and_status_consistency failed on type DV_ORDINAL"),
            "got {v:?}"
        );
    }
}
