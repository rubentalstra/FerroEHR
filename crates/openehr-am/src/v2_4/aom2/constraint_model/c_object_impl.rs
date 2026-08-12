//! Hand-written AOM2 `C_OBJECT` spec functions.
//!
//! Spec sources (vendored):
//! `AM/docs/UML/classes/org.openehr.am.aom2.c_object.adoc` §Functions and
//! `AM/docs/AOM2/master07-terminology_package.adoc` §Specialisation Depth.

use crate::v2_4::aom2::constraint_model::c_object::CObject;
use crate::v2_4::aom2::definitions::adl_code_definitions::AdlCodeDefinitionsData;
use openehr_base::v1_3::foundation_types::interval::multiplicity_interval::MultiplicityInterval;

impl CObject {
    /// Returns the `node_id` of this object node.
    #[must_use]
    pub fn node_id(&self) -> &str {
        match self {
            Self::ArchetypeSlot(o) => &o.node_id,
            Self::CBoolean(o) => &o.node_id,
            Self::CComplexObject(o) => o.node_id(),
            Self::CComplexObjectProxy(o) => &o.node_id,
            Self::CDate(o) => &o.node_id,
            Self::CDateTime(o) => &o.node_id,
            Self::CDuration(o) => &o.node_id,
            Self::CInteger(o) => &o.node_id,
            Self::CReal(o) => &o.node_id,
            Self::CString(o) => &o.node_id,
            Self::CTerminologyCode(o) => &o.node_id,
            Self::CTime(o) => &o.node_id,
        }
    }

    /// Returns the `occurrences` of this object node.
    #[must_use]
    pub fn occurrences(&self) -> Option<&MultiplicityInterval> {
        match self {
            Self::ArchetypeSlot(o) => o.occurrences.as_ref(),
            Self::CBoolean(o) => o.occurrences.as_ref(),
            Self::CComplexObject(o) => o.occurrences(),
            Self::CComplexObjectProxy(o) => o.occurrences.as_ref(),
            Self::CDate(o) => o.occurrences.as_ref(),
            Self::CDateTime(o) => o.occurrences.as_ref(),
            Self::CDuration(o) => o.occurrences.as_ref(),
            Self::CInteger(o) => o.occurrences.as_ref(),
            Self::CReal(o) => o.occurrences.as_ref(),
            Self::CString(o) => o.occurrences.as_ref(),
            Self::CTerminologyCode(o) => o.occurrences.as_ref(),
            Self::CTime(o) => o.occurrences.as_ref(),
        }
    }

    /// Returns the specialisation level of this node, from its `node_id`.
    ///
    /// `specialisation_depth` (`org.openehr.am.aom2.c_object.adoc` §Functions):
    /// "The value 0 corresponds to non-specialised, 1 to first-level
    /// specialisation and so on. The level is the same as the number of '.'
    /// characters in the `node_id` code. If `node_id` is not set, the return
    /// value is -1, signifying that the specialisation level should be
    /// determined from the nearest parent `C_OBJECT` node having a `node_id`."
    #[must_use]
    pub fn specialisation_depth(&self) -> i32 {
        specialisation_depth_of(self.node_id())
    }

    /// Returns true if this node is prohibited.
    ///
    /// `is_prohibited` (`org.openehr.am.aom2.c_object.adoc` §Functions),
    /// post-condition `Result = occurrences /= Void and then
    /// occurrences.is_prohibited`, i.e. an `occurrences` of `0..0`.
    #[must_use]
    pub fn is_prohibited(&self) -> bool {
        self.occurrences()
            .is_some_and(MultiplicityInterval::is_prohibited)
    }
}

/// The specialisation depth a `node_id` code encodes, or `-1` when unset.
pub(crate) fn specialisation_depth_of(node_id: &str) -> i32 {
    if node_id.is_empty() {
        return -1;
    }
    i32::try_from(
        node_id
            .matches(AdlCodeDefinitionsData::SPECIALISATION_SEPARATOR)
            .count(),
    )
    .unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2_4::aom2::constraint_model::c_complex_object::{
        CComplexObject, CComplexObjectData,
    };

    fn object(node_id: &str, occurrences: Option<MultiplicityInterval>) -> CObject {
        CObject::CComplexObject(CComplexObject::CComplexObject(CComplexObjectData {
            parent: None,
            soc_parent: None,
            rm_type_name: "ELEMENT".to_owned(),
            occurrences,
            node_id: node_id.to_owned(),
            alternative_ids: None,
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            attributes: None,
            attribute_tuples: None,
        }))
    }

    fn interval(lower: i32, upper: i32) -> MultiplicityInterval {
        MultiplicityInterval {
            lower: Some(lower),
            upper: Some(upper),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        }
    }

    #[test]
    fn the_depth_is_the_separator_count_of_the_node_id() {
        assert_eq!(object("id1", None).specialisation_depth(), 0);
        assert_eq!(object("at0004.1", None).specialisation_depth(), 1);
        assert_eq!(object("at0004.0.1", None).specialisation_depth(), 2);
    }

    #[test]
    fn an_unset_node_id_defers_to_the_nearest_coded_ancestor() {
        assert_eq!(object("", None).specialisation_depth(), -1);
    }

    #[test]
    fn prohibition_is_an_occurrences_of_zero_to_zero() {
        assert!(object("id1", Some(interval(0, 0))).is_prohibited());
        assert!(!object("id1", Some(interval(0, 1))).is_prohibited());
        assert!(!object("id1", None).is_prohibited());
    }
}
