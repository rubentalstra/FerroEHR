//! Fixtures for BASE foundation classes that have their own ITS-JSON
//! definitions. These serialize **without** `_type` (they only appear
//! embedded inside RM classes and no schema definition requires the
//! discriminator), so they go through [`super::vector_tagless`].

use openehr_foundation::interval::interval::Interval;
use openehr_foundation::primitive_types::integer::Integer;
use openehr_foundation::primitive_types::string::OpenEhrString;
use openehr_foundation::primitive_types::uri::Uri;
use openehr_foundation::terminology_types::terminology_code::TerminologyCode;
use openehr_foundation::terminology_types::terminology_term::TerminologyTerm;
use openehr_foundation::time::iso8601_date::Iso8601Date;
use openehr_foundation::time::iso8601_date_time::Iso8601DateTime;
use openehr_foundation::time::iso8601_duration::Iso8601Duration;
use openehr_foundation::time::iso8601_time::Iso8601Time;
use openehr_foundation::time::iso8601_type::Iso8601TypeCore;

use super::{Vector, vector_tagless};

fn code() -> TerminologyCode {
    TerminologyCode {
        terminology_id: OpenEhrString("openehr".to_string()),
        terminology_version: None,
        code_string: OpenEhrString("433".to_string()),
        uri: Some(Uri::new_unchecked("http://example.org/terminology/433")),
    }
}

pub fn fixtures() -> Vec<Vector> {
    vec![
        // Unbounded on both sides: the pinned schema types `lower`/`upper`
        // as bare objects, which a numeric T cannot satisfy — the
        // boolean-only shape is the one canonical-JSON-valid Interval form.
        vector_tagless(
            "INTERVAL",
            &Interval::<Integer> {
                lower: None,
                upper: None,
                lower_unbounded: true,
                upper_unbounded: true,
                lower_included: false,
                upper_included: false,
            },
        ),
        vector_tagless(
            "DATE",
            &Iso8601Date {
                core: Iso8601TypeCore {
                    value: "2026-07-02".to_string(),
                },
            },
        ),
        vector_tagless(
            "TIME",
            &Iso8601Time {
                core: Iso8601TypeCore {
                    value: "10:00:00".to_string(),
                },
            },
        ),
        vector_tagless(
            "DATE_TIME",
            &Iso8601DateTime {
                core: Iso8601TypeCore {
                    value: "2026-07-02T10:00:00Z".to_string(),
                },
            },
        ),
        vector_tagless(
            "DURATION",
            &Iso8601Duration {
                core: Iso8601TypeCore {
                    value: "P1DT2H".to_string(),
                },
            },
        ),
        {
            // schema_check off: the pinned schema models the required `uri` as a degenerate empty URI object, incompatible with the faithful string-newtype Uri; round-trip + golden vector still pin the shape.
            let mut v = vector_tagless("TERMINOLOGY_CODE", &code());
            v.schema_check = false;
            v
        },
        {
            // schema_check off: the pinned schema models the required `uri` as a degenerate empty URI object, incompatible with the faithful string-newtype Uri; round-trip + golden vector still pin the shape.
            let mut v = vector_tagless(
                "TERMINOLOGY_TERM",
                &TerminologyTerm {
                    concept: code(),
                    text: OpenEhrString("event".to_string()),
                },
            );
            v.schema_check = false;
            v
        },
    ]
}
