//! Hand-written RM class invariant for `EVENT_CONTEXT`.
//!
//! `location_valid` (BMM `EVENT_CONTEXT.location_valid`,
//! `location /= Void implies not location.is_empty`): if present, `location`
//! must be non-empty.
//!
//! NOTE: archie's `Setting_valid` is terminology-bound (deferred), and its
//! `Participations_validity` is `ignored`. archie does **not** enforce
//! "start_time present" (it is structurally guaranteed here — `start_time` is a
//! required field).

use crate::composition::event_context::EventContext;
use crate::validate::{InvariantViolation, Validate};

impl Validate for EventContext {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::validate::generated::event_context_core(self.location.as_deref(), out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::date_time::dv_date_time::DvDateTime;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_coded_text::DvCodedText;
    use openehr_base::prelude::TerminologyId;

    fn setting() -> DvCodedText {
        DvCodedText {
            value: "other care".to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: openehr_base::containers::present(Vec::new()),
            language: None,
            encoding: None,
            defining_code: CodePhrase {
                terminology_id: TerminologyId {
                    value: "openehr".to_owned(),
                },
                code_string: "238".to_owned(),
                preferred_term: None,
            },
        }
    }

    fn context(location: Option<&str>) -> EventContext {
        EventContext {
            start_time: DvDateTime {
                normal_status: None,
                normal_range: None,
                other_reference_ranges: openehr_base::containers::present(Vec::new()),
                magnitude_status: None,
                accuracy: None,
                value: "2021-01-01T09:00:00".to_owned(),
            },
            end_time: None,
            location: location.map(str::to_owned),
            setting: setting(),
            other_context: None,
            health_care_facility: None,
            participations: openehr_base::containers::present(Vec::new()),
        }
    }

    #[test]
    fn valid_context() {
        assert!(context(Some("ward A3")).invariants().is_empty());
        assert!(context(None).invariants().is_empty());
    }

    #[test]
    fn empty_location_invalid() {
        assert_eq!(
            context(Some("")).invariants()[0].message,
            "Invariant location_valid failed on type EVENT_CONTEXT"
        );
    }
}
