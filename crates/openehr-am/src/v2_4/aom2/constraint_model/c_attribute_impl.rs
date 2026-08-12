//! Hand-written AOM2 `C_ATTRIBUTE` spec functions.
//!
//! Spec source (vendored):
//! `AM/docs/UML/classes/org.openehr.am.aom2.c_attribute.adoc` §Attributes +
//! §Functions.

use crate::v2_4::aom2::constraint_model::c_attribute::CAttribute;
use crate::v2_4::aom2::constraint_model::c_object::CObject;
use openehr_base::v1_3::foundation_types::interval::multiplicity_interval::MultiplicityInterval;

impl CAttribute {
    /// Returns true if there is no effective constraint on this attribute's
    /// children.
    ///
    /// `any_allowed` (`org.openehr.am.aom2.c_attribute.adoc` §Functions),
    /// post-condition `Result := children.is_empty and not is_prohibited`.
    #[must_use]
    pub fn any_allowed(&self) -> bool {
        self.children.as_deref().is_none_or(<[CObject]>::is_empty) && !self.is_prohibited()
    }

    /// Returns true if this attribute is constrained to be mandatory.
    ///
    /// `is_mandatory` (`org.openehr.am.aom2.c_attribute.adoc` §Functions),
    /// post-condition `Result = existence /= Void and then
    /// existence.is_mandatory`, i.e. an `existence` of `1..1`.
    #[must_use]
    pub fn is_mandatory(&self) -> bool {
        self.existence
            .as_ref()
            .is_some_and(MultiplicityInterval::is_mandatory)
    }

    /// Returns true if this attribute is constrained to be absent.
    ///
    /// `is_prohibited` (`org.openehr.am.aom2.c_attribute.adoc` §Functions),
    /// post-condition `Result = existence /= Void and then
    /// existence.is_prohibited`, i.e. an `existence` of `0..0`.
    #[must_use]
    pub fn is_prohibited(&self) -> bool {
        self.existence
            .as_ref()
            .is_some_and(MultiplicityInterval::is_prohibited)
    }

    /// Returns true if this node represents a single-valued attribute.
    ///
    /// `is_single` (`org.openehr.am.aom2.c_attribute.adoc` §Functions):
    /// "Evaluated as not `is_multiple`".
    #[must_use]
    pub fn is_single(&self) -> bool {
        !self.is_multiple
    }

    /// Returns this attribute's path with respect to its owning object.
    ///
    /// `rm_attribute_path` (`org.openehr.am.aom2.c_attribute.adoc` §Functions):
    /// "Path of this attribute with respect to owning `C_OBJECT`, including
    /// differential path where applicable" — the `differential_path` being
    /// "Path to the parent object of this attribute (i.e. doesn't include the
    /// name of this attribute)", so the two concatenate.
    #[must_use]
    pub fn rm_attribute_path(&self) -> String {
        match self.differential_path.as_deref() {
            Some(prefix) if !prefix.is_empty() => {
                format!(
                    "{}/{}",
                    prefix.trim_end_matches('/'),
                    self.rm_attribute_name
                )
            }
            _ => self.rm_attribute_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn attribute(
        existence: Option<MultiplicityInterval>,
        children: Option<Vec<CObject>>,
        differential_path: Option<&str>,
        is_multiple: bool,
    ) -> CAttribute {
        CAttribute {
            parent: None,
            soc_parent: None,
            rm_attribute_name: "items".to_owned(),
            existence,
            children,
            differential_path: differential_path.map(str::to_owned),
            cardinality: None,
            is_multiple,
        }
    }

    #[test]
    fn no_children_and_no_prohibition_allows_anything() {
        assert!(attribute(None, None, None, false).any_allowed());
        assert!(attribute(Some(interval(0, 1)), Some(Vec::new()), None, false).any_allowed());
    }

    #[test]
    fn a_prohibited_attribute_allows_nothing() {
        assert!(!attribute(Some(interval(0, 0)), None, None, false).any_allowed());
    }

    #[test]
    fn existence_decides_mandation_and_prohibition() {
        assert!(attribute(Some(interval(1, 1)), None, None, false).is_mandatory());
        assert!(!attribute(Some(interval(0, 1)), None, None, false).is_mandatory());
        assert!(!attribute(None, None, None, false).is_mandatory());
        assert!(attribute(Some(interval(0, 0)), None, None, false).is_prohibited());
        assert!(!attribute(None, None, None, false).is_prohibited());
    }

    #[test]
    fn single_is_the_negation_of_multiple() {
        assert!(attribute(None, None, None, false).is_single());
        assert!(!attribute(None, None, None, true).is_single());
    }

    #[test]
    fn the_differential_path_prefixes_the_attribute_name() {
        assert_eq!(
            attribute(None, None, None, false).rm_attribute_path(),
            "items"
        );
        assert_eq!(
            attribute(None, None, Some("/data[id2]/events[id3]"), false).rm_attribute_path(),
            "/data[id2]/events[id3]/items"
        );
        assert_eq!(
            attribute(None, None, Some("/data[id2]/"), false).rm_attribute_path(),
            "/data[id2]/items"
        );
    }
}
