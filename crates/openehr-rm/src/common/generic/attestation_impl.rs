//! Hand-written RM class invariant for `ATTESTATION`.
//!
//! `ATTESTATION` extends `AUDIT_DETAILS` and inherits `System_id_valid`.
//!
//! NOTE: archie's own `Attestation` invariants (`Items_valid`,
//! `Reason_valid`) are both `ignored` (never checked), so only the inherited
//! `System_id_valid` applies. The inherited `Change_type_valid` is
//! terminology-bound (deferred).

use crate::common::generic::attestation::Attestation;
use crate::validate::{InvariantViolation, Validate};

impl Validate for Attestation {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.system_id.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant System_id_valid failed on type ATTESTATION",
            ));
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;
    use crate::common::generic::party_proxy::PartyProxy;
    use crate::common::generic::party_self::PartySelf;
    use crate::data_types::quantity::date_time::dv_date_time::DvDateTime;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_coded_text::DvCodedText;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::prelude::TerminologyId;

    fn coded() -> DvCodedText {
        DvCodedText {
            value: "creation".to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: Vec::new(),
            language: None,
            encoding: None,
            defining_code: CodePhrase {
                terminology_id: TerminologyId {
                    value: "openehr".to_owned(),
                },
                code_string: "249".to_owned(),
                preferred_term: None,
            },
        }
    }

    fn attestation(system_id: &str) -> Attestation {
        Attestation {
            system_id: system_id.to_owned(),
            time_committed: DvDateTime {
                normal_status: None,
                normal_range: None,
                other_reference_ranges: Vec::new(),
                magnitude_status: None,
                accuracy: None,
                value: "2021-01-01T00:00:00".to_owned(),
            },
            change_type: coded(),
            description: None,
            committer: PartyProxy::PartySelf(PartySelf { external_ref: None }),
            attested_view: None,
            proof: None,
            items: Vec::new(),
            reason: DvText::DvText(DvTextData {
                value: "witness".to_owned(),
                hyperlink: None,
                formatting: None,
                mappings: Vec::new(),
                language: None,
                encoding: None,
            }),
            is_pending: false,
        }
    }

    #[test]
    fn valid_attestation() {
        assert!(attestation("system-1").invariants().is_empty());
    }

    #[test]
    fn empty_system_id_invalid() {
        assert_eq!(
            attestation("").invariants()[0].message,
            "Invariant System_id_valid failed on type ATTESTATION"
        );
    }
}
