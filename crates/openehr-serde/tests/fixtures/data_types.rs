//! Fixtures for rm.data_types classes.

use openehr_foundation::interval::interval::Interval;
use openehr_foundation::primitive_types::integer::Integer;
use openehr_foundation::primitive_types::real::Real;
use openehr_foundation::serde_support::TypeTag;
use openehr_foundation::time::iso8601_duration::Iso8601Duration;
use openehr_foundation::time::iso8601_type::Iso8601TypeCore;
use openehr_rm::data_types::basic::dv_boolean::DvBoolean;
use openehr_rm::data_types::basic::dv_identifier::DvIdentifier;
use openehr_rm::data_types::basic::dv_state::DvState;
use openehr_rm::data_types::date_time::dv_date::DvDate;
use openehr_rm::data_types::date_time::dv_duration::DvDuration;
use openehr_rm::data_types::date_time::dv_time::DvTime;
use openehr_rm::data_types::encapsulated::dv_encapsulated::DvEncapsulatedData;
use openehr_rm::data_types::encapsulated::dv_multimedia::DvMultimedia;
use openehr_rm::data_types::encapsulated::dv_parsable::DvParsable;
use openehr_rm::data_types::quantity::dv_count::DvCount;
use openehr_rm::data_types::quantity::dv_interval::DvInterval;
use openehr_rm::data_types::quantity::dv_ordinal::DvOrdinal;
use openehr_rm::data_types::quantity::dv_proportion::DvProportion;
use openehr_rm::data_types::quantity::dv_quantity::DvQuantity;
use openehr_rm::data_types::quantity::dv_scale::DvScale;
use openehr_rm::data_types::quantity::proportion_kind::ProportionKind;
use openehr_rm::data_types::quantity::reference_range::ReferenceRange;
use openehr_rm::data_types::text::dv_paragraph::DvParagraph;
use openehr_rm::data_types::text::term_mapping::{MatchKind, TermMapping};
use openehr_rm::data_types::time_specification::dv_general_time_specification::DvGeneralTimeSpecification;
use openehr_rm::data_types::time_specification::dv_periodic_time_specification::DvPeriodicTimeSpecification;
use openehr_rm::data_types::uri::dv_ehr_uri::DvEhrUri;
use openehr_rm::data_types::uri::dv_uri::{DvUri, DvUriData};

use super::helpers::{
    amount_data, code_phrase, coded, date_time, ordered_data, temporal_data, text,
};
use super::{Vector, vector};

fn count(magnitude: i64) -> DvCount {
    DvCount {
        type_tag: TypeTag::new(),
        amount: amount_data(),
        magnitude,
    }
}

fn parsable(value: &str, formalism: &str) -> DvParsable {
    DvParsable {
        type_tag: TypeTag::new(),
        encapsulated: DvEncapsulatedData {
            charset: None,
            language: None,
        },
        value: value.to_string(),
        formalism: formalism.to_string(),
    }
}

pub fn fixtures() -> Vec<Vector> {
    vec![
        vector(
            "DV_BOOLEAN",
            &DvBoolean {
                type_tag: TypeTag::new(),
                value: true,
            },
        ),
        vector(
            "DV_IDENTIFIER",
            &DvIdentifier {
                type_tag: TypeTag::new(),
                issuer: Some("NHS".to_string()),
                assigner: None,
                id: "abc-123".to_string(),
                r#type: None,
            },
        ),
        vector(
            "DV_STATE",
            &DvState {
                type_tag: TypeTag::new(),
                value: coded("completed", "openehr", "532"),
                is_terminal: true,
            },
        ),
        vector("DV_TEXT", &text("plain text")),
        vector("DV_CODED_TEXT", &coded("event", "openehr", "433")),
        vector(
            "DV_PARAGRAPH",
            &DvParagraph {
                type_tag: TypeTag::new(),
                items: vec![text("line one")],
            },
        ),
        vector("CODE_PHRASE", &code_phrase("openehr", "433")),
        vector(
            "TERM_MAPPING",
            &TermMapping {
                type_tag: TypeTag::new(),
                match_: MatchKind::Equivalent,
                purpose: None,
                target: code_phrase("SNOMED-CT", "50043002"),
            },
        ),
        vector("DV_COUNT", &count(3)),
        vector(
            "DV_QUANTITY",
            &DvQuantity {
                type_tag: TypeTag::new(),
                amount: amount_data(),
                magnitude: Real(37.2),
                precision: Some(Integer(1)),
                units: "Cel".to_string(),
                units_system: None,
                units_display_name: None,
            },
        ),
        vector(
            "DV_ORDINAL",
            &DvOrdinal {
                type_tag: TypeTag::new(),
                ordered: ordered_data(),
                symbol: coded("Mild", "local", "at0005"),
                value: Integer(1),
            },
        ),
        vector(
            "DV_SCALE",
            &DvScale {
                type_tag: TypeTag::new(),
                ordered: ordered_data(),
                symbol: coded("Moderate", "local", "at0006"),
                value: Real(2.5),
            },
        ),
        vector(
            "DV_PROPORTION",
            &DvProportion {
                type_tag: TypeTag::new(),
                amount: amount_data(),
                numerator: Real(1.0),
                denominator: Real(2.0),
                type_: ProportionKind::Fraction,
                precision: None,
            },
        ),
        vector(
            "DV_INTERVAL",
            &DvInterval::<DvCount> {
                type_tag: TypeTag::new(),
                range: Interval {
                    lower: Some(count(1)),
                    upper: Some(count(10)),
                    lower_unbounded: false,
                    upper_unbounded: false,
                    lower_included: true,
                    upper_included: true,
                },
            },
        ),
        vector(
            "REFERENCE_RANGE",
            &ReferenceRange::<DvCount> {
                type_tag: TypeTag::new(),
                meaning: text("normal"),
                range: DvInterval {
                    type_tag: TypeTag::new(),
                    range: Interval {
                        lower: Some(count(2)),
                        upper: Some(count(8)),
                        lower_unbounded: false,
                        upper_unbounded: false,
                        lower_included: true,
                        upper_included: true,
                    },
                },
            },
        ),
        vector(
            "DV_DATE",
            &DvDate {
                type_tag: TypeTag::new(),
                temporal: temporal_data(),
                value: "2026-07-02".to_string(),
            },
        ),
        vector(
            "DV_TIME",
            &DvTime {
                type_tag: TypeTag::new(),
                temporal: temporal_data(),
                value: "10:00:00".to_string(),
            },
        ),
        vector("DV_DATE_TIME", &date_time("2026-07-02T10:00:00Z")),
        vector(
            "DV_DURATION",
            &DvDuration {
                type_tag: TypeTag::new(),
                accuracy_is_percent: None,
                accuracy: None,
                iso8601: Iso8601Duration {
                    core: Iso8601TypeCore {
                        value: "P1DT2H".to_string(),
                    },
                },
            },
        ),
        vector(
            "DV_PERIODIC_TIME_SPECIFICATION",
            &DvPeriodicTimeSpecification {
                type_tag: TypeTag::new(),
                value: parsable("[20260702T1000;20260702T1030]/(7d)@DW", "HL7:PIVL"),
            },
        ),
        vector(
            "DV_GENERAL_TIME_SPECIFICATION",
            &DvGeneralTimeSpecification {
                type_tag: TypeTag::new(),
                value: parsable("20260702T1000", "HL7:GTS"),
            },
        ),
        vector(
            "DV_MULTIMEDIA",
            &DvMultimedia {
                type_tag: TypeTag::new(),
                encapsulated: DvEncapsulatedData {
                    charset: None,
                    language: None,
                },
                alternate_text: Some("ECG image".to_string()),
                uri: None,
                data: Some(vec![1, 2, 3, 4]),
                media_type: code_phrase("IANA_media-types", "image/png"),
                compression_algorithm: None,
                integrity_check: None,
                integrity_check_algorithm: None,
                thumbnail: None,
                size: 4,
            },
        ),
        vector("DV_PARSABLE", &parsable("<xml/>", "text/xml")),
        vector(
            "DV_URI",
            &DvUri {
                type_tag: TypeTag::new(),
                uri: DvUriData {
                    value: "https://example.org/x".to_string(),
                },
            },
        ),
        vector(
            "DV_EHR_URI",
            &DvEhrUri {
                type_tag: TypeTag::new(),
                uri: DvUriData {
                    value: "ehr://ehr.example.org/ehr1".to_string(),
                },
            },
        ),
    ]
}
