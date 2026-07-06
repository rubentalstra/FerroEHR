//! Hand-written RM class invariants (ADR-003) for `DV_SCALE`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_scale.adoc`.
//! `DV_SCALE` declares no own invariants; it inherits the DV_ORDERED
//! `Normal_range_and_status_consistency` (checked via the ordered-magnitude
//! machinery in `dv_ordered_impl`). The comparison functions
//! (`less_than` / `is_strictly_comparable_to`) live in `dv_ordered_impl`.
//!
//! PORT NOTE: the DV_ORDERED `Normal_status_validity` invariant (terminology)
//! is deferred to the composition validator + `openehr-term`.

use crate::data_types::quantity::dv_ordered::DvOrdered;
use crate::data_types::quantity::dv_ordered_impl::push_normal_range_consistency;
use crate::data_types::quantity::dv_scale::DvScale;
use crate::validate::{InvariantViolation, Validate};

impl Validate for DvScale {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_normal_range_consistency(
            out,
            "DV_SCALE",
            self.normal_status.as_ref(),
            self.normal_range.as_deref(),
            &DvOrdered::DvScale(self.clone()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_coded_text::DvCodedText;
    use openehr_base::prelude::TerminologyId;

    fn scale(value: f64) -> DvScale {
        DvScale {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            symbol: DvCodedText {
                value: "very very slight".to_owned(),
                hyperlink: None,
                formatting: None,
                mappings: Vec::new(),
                language: None,
                encoding: None,
                defining_code: CodePhrase {
                    terminology_id: TerminologyId {
                        value: "local".to_owned(),
                    },
                    code_string: "at0002".to_owned(),
                    preferred_term: None,
                },
            },
            value,
        }
    }

    #[test]
    fn plain_scale_valid() {
        assert!(scale(0.5).invariants().is_empty());
    }

    #[test]
    fn ordering_functions_allow_non_integer_values() {
        assert_eq!(scale(0.5).less_than(&scale(1.0)), Some(true));
        assert!(scale(0.5).is_strictly_comparable_to(&scale(3.0)));
    }
}
