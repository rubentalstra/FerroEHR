//! Hand-written AOM2 `C_ORDERED` spec functions.
//!
//! Spec source (vendored):
//! `AM/docs/UML/classes/org.openehr.am.aom2.c_ordered.adoc` §Functions, with the
//! temporal descendants redefining `any_allowed` per
//! `org.openehr.am.aom2.c_temporal.adoc` §Functions.

use crate::v2_4::aom2::constraint_model::primitive::c_ordered::COrdered;

impl COrdered {
    /// Returns true if any value of the constrained ordered type would be
    /// allowed.
    ///
    /// `any_allowed` (`org.openehr.am.aom2.c_ordered.adoc` §Functions),
    /// post-condition `Result = constraint.is_empty`. The temporal descendants
    /// redefine it to also require an empty `pattern_constraint`, so this
    /// dispatches to `C_TEMPORAL` for them.
    #[must_use]
    pub fn any_allowed(&self) -> bool {
        let (range_empty, pattern) = match self {
            Self::CDate(c) => (
                c.constraint.as_ref().is_none_or(Vec::is_empty),
                c.pattern_constraint.as_deref(),
            ),
            Self::CDateTime(c) => (
                c.constraint.as_ref().is_none_or(Vec::is_empty),
                c.pattern_constraint.as_deref(),
            ),
            Self::CDuration(c) => (
                c.constraint.as_ref().is_none_or(Vec::is_empty),
                c.pattern_constraint.as_deref(),
            ),
            Self::CTime(c) => (
                c.constraint.as_ref().is_none_or(Vec::is_empty),
                c.pattern_constraint.as_deref(),
            ),
            Self::CInteger(c) => (c.constraint.as_ref().is_none_or(Vec::is_empty), None),
            Self::CReal(c) => (c.constraint.as_ref().is_none_or(Vec::is_empty), None),
        };
        range_empty && pattern.is_none_or(str::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2_4::aom2::constraint_model::primitive::c_date::CDate;
    use crate::v2_4::aom2::constraint_model::primitive::c_integer::CInteger;
    use openehr_base::v1_3::foundation_types::interval::interval::Interval;
    use openehr_base::v1_3::foundation_types::interval::point_interval::PointInterval;

    fn integer(constraint: Option<Vec<Interval<i32>>>) -> COrdered {
        COrdered::CInteger(CInteger {
            parent: None,
            soc_parent: None,
            rm_type_name: "Integer".to_owned(),
            occurrences: None,
            node_id: "at9999".to_owned(),
            alternative_ids: None,
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value: None,
            is_enumerated_type_constraint: None,
            constraint,
        })
    }

    fn date(pattern: Option<&str>) -> COrdered {
        COrdered::CDate(CDate {
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
            constraint: None,
            pattern_constraint: pattern.map(str::to_owned),
        })
    }

    #[test]
    fn an_unstated_range_allows_any_value() {
        assert!(integer(None).any_allowed());
        assert!(integer(Some(Vec::new())).any_allowed());
    }

    #[test]
    fn one_interval_is_already_a_constraint() {
        let point = Interval::PointInterval(PointInterval {
            lower: Some(1),
            upper: Some(1),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        });
        assert!(!integer(Some(vec![point])).any_allowed());
    }

    #[test]
    fn a_temporal_descendant_also_answers_for_its_pattern() {
        assert!(date(None).any_allowed());
        assert!(!date(Some("YYYY-??-XX")).any_allowed());
    }
}
