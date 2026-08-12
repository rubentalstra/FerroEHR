//! Hand-written AOM2 `C_TEMPORAL` spec functions.
//!
//! Spec source (vendored):
//! `AM/docs/UML/classes/org.openehr.am.aom2.c_temporal.adoc` §Functions.

use crate::v2_4::aom2::constraint_model::primitive::c_temporal::CTemporal;

impl CTemporal {
    /// Returns true if any value of the constrained temporal type would be
    /// allowed.
    ///
    /// `any_allowed` (`org.openehr.am.aom2.c_temporal.adoc` §Functions),
    /// post-condition `Result = precursor and pattern_constraint.is_empty` —
    /// the precursor being `C_ORDERED.any_allowed`, `Result =
    /// constraint.is_empty` (`org.openehr.am.aom2.c_ordered.adoc` §Functions).
    #[must_use]
    pub fn any_allowed(&self) -> bool {
        let (range_empty, pattern) = match self {
            Self::CDate(c) => (
                c.constraint.as_ref().is_none_or(Vec::is_empty),
                &c.pattern_constraint,
            ),
            Self::CDateTime(c) => (
                c.constraint.as_ref().is_none_or(Vec::is_empty),
                &c.pattern_constraint,
            ),
            Self::CDuration(c) => (
                c.constraint.as_ref().is_none_or(Vec::is_empty),
                &c.pattern_constraint,
            ),
            Self::CTime(c) => (
                c.constraint.as_ref().is_none_or(Vec::is_empty),
                &c.pattern_constraint,
            ),
        };
        range_empty && pattern.as_ref().is_none_or(String::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2_4::aom2::constraint_model::primitive::c_date::CDate;
    use openehr_base::v1_3::foundation_types::interval::interval::Interval;
    use openehr_base::v1_3::foundation_types::interval::point_interval::PointInterval;
    use openehr_base::v1_3::foundation_types::time::iso8601_date::Iso8601Date;

    fn date(constraint: Option<Vec<Interval<Iso8601Date>>>, pattern: Option<&str>) -> CTemporal {
        CTemporal::CDate(CDate {
            parent: None,
            soc_parent: None,
            rm_type_name: "DV_DATE".to_owned(),
            occurrences: None,
            node_id: "at9999".to_owned(),
            alternative_ids: None,
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value: None,
            is_enumerated_type_constraint: None,
            constraint,
            pattern_constraint: pattern.map(str::to_owned),
        })
    }

    fn a_range() -> Vec<Interval<Iso8601Date>> {
        vec![Interval::PointInterval(PointInterval {
            lower: Some(Iso8601Date {
                value: "2004-05-20".to_owned(),
            }),
            upper: Some(Iso8601Date {
                value: "2004-05-20".to_owned(),
            }),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        })]
    }

    #[test]
    fn neither_a_range_nor_a_pattern_allows_anything() {
        assert!(date(None, None).any_allowed());
        assert!(date(Some(Vec::new()), None).any_allowed());
    }

    #[test]
    fn either_a_range_or_a_pattern_is_already_a_constraint() {
        assert!(!date(Some(a_range()), None).any_allowed());
        assert!(!date(None, Some("YYYY-??-XX")).any_allowed());
        assert!(!date(Some(a_range()), Some("YYYY-??-XX")).any_allowed());
    }
}
