//! Hand-written RM class invariant for `AUDIT_DETAILS`.
//!
//! `System_id_valid` (archie `AuditDetails`, `nullOrNotEmpty`): `system_id` must
//! be non-empty.
//!
//! PORT NOTE: archie's `Change_type_valid` (the change-type code belongs to the
//! openEHR "audit change type" group) is terminology-bound — deferred to the
//! composition validator + `openehr-term`.

use crate::common::generic::audit_details::AuditDetailsData;
use crate::validate::{InvariantViolation, Validate};

impl Validate for AuditDetailsData {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.system_id.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant System_id_valid failed on type AUDIT_DETAILS",
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
    use openehr_base::prelude::TerminologyId;

    fn change_type() -> DvCodedText {
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

    fn audit(system_id: &str) -> AuditDetailsData {
        AuditDetailsData {
            system_id: system_id.to_owned(),
            time_committed: DvDateTime {
                normal_status: None,
                normal_range: None,
                other_reference_ranges: Vec::new(),
                magnitude_status: None,
                accuracy: None,
                value: "2021-01-01T00:00:00".to_owned(),
            },
            change_type: change_type(),
            description: None,
            committer: PartyProxy::PartySelf(PartySelf { external_ref: None }),
        }
    }

    #[test]
    fn valid_audit() {
        assert!(audit("system-1").invariants().is_empty());
    }

    #[test]
    fn empty_system_id_invalid() {
        assert_eq!(
            audit("").invariants()[0].message,
            "Invariant System_id_valid failed on type AUDIT_DETAILS"
        );
    }
}
