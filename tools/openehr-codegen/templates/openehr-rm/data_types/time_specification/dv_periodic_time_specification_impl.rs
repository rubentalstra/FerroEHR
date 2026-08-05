//! Hand-written RM class invariants for `DV_PERIODIC_TIME_SPECIFICATION`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_periodic_time_specification.adoc`
//! — `Value_valid: value.formalism.is_equal("HL7:PIVL") or
//! value.formalism.is_equal("HL7:EIVL")`.

use crate::v1_2::data_types::time_specification::dv_periodic_time_specification::DvPeriodicTimeSpecification;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for DvPeriodicTimeSpecification {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.value.formalism != "HL7:PIVL" && self.value.formalism != "HL7:EIVL" {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type DV_PERIODIC_TIME_SPECIFICATION \
                 (value.formalism must be HL7:PIVL or HL7:EIVL)",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_2::data_types::encapsulated::dv_parsable::DvParsable;

    fn spec(formalism: &str) -> DvPeriodicTimeSpecification {
        DvPeriodicTimeSpecification {
            value: DvParsable {
                charset: None,
                language: None,
                value: "[20260711T1000]".to_owned(),
                formalism: formalism.to_owned(),
            },
        }
    }

    /// `Value_valid` (`dv_periodic_time_specification.adoc`): the inner
    /// parsable's formalism must be HL7:PIVL or HL7:EIVL.
    #[test]
    fn formalism_is_constrained() {
        let mut out = Vec::new();
        spec("HL7:PIVL").validate_invariants(&mut out);
        assert!(out.is_empty(), "{out:?}");
        let mut out = Vec::new();
        spec("HL7:EIVL").validate_invariants(&mut out);
        assert!(out.is_empty(), "{out:?}");
        let mut out = Vec::new();
        spec("ISO8601").validate_invariants(&mut out);
        assert!(
            out.iter().any(|m| m.message.contains("Value_valid")),
            "{out:?}"
        );
    }
}
