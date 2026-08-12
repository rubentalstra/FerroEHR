//! Hand-written AOM2 `C_PRIMITIVE_OBJECT` spec functions.
//!
//! Spec sources (vendored):
//! `AM/docs/UML/classes/org.openehr.am.aom2.c_primitive_object.adoc` §Functions
//! and each concrete descendant's own class page, whose `constraint` attribute
//! names the native type it constrains (e.g. `c_duration.adoc`:
//! `List<Interval<Iso8601_duration>>`).

use crate::v2_4::aom2::constraint_model::c_primitive_object::CPrimitiveObject;

impl CPrimitiveObject {
    /// Returns true if this constraint carries an assumed value.
    ///
    /// `has_assumed_value` (`org.openehr.am.aom2.c_primitive_object.adoc`
    /// §Functions): "True if there is an assumed value", read against the
    /// `0..1` `assumed_value` attribute every concrete descendant carries.
    #[must_use]
    pub fn has_assumed_value(&self) -> bool {
        match self {
            Self::CBoolean(c) => c.assumed_value.is_some(),
            Self::CDate(c) => c.assumed_value.is_some(),
            Self::CDateTime(c) => c.assumed_value.is_some(),
            Self::CDuration(c) => c.assumed_value.is_some(),
            Self::CInteger(c) => c.assumed_value.is_some(),
            Self::CReal(c) => c.assumed_value.is_some(),
            Self::CString(c) => c.assumed_value.is_some(),
            Self::CTerminologyCode(c) => c.assumed_value.is_some(),
            Self::CTime(c) => c.assumed_value.is_some(),
        }
    }

    /// Returns the name of the native type this constraint constrains.
    ///
    /// `constrained_typename` (`org.openehr.am.aom2.c_primitive_object.adoc`
    /// §Functions): for most types the constrainer typename without its `C_`
    /// prefix (`C_INTEGER` → `Integer`), while "for the date/time types the
    /// mapping is different" — the different mapping being each temporal
    /// descendant's own declared `constraint` element type (`Iso8601_date`,
    /// `Iso8601_time`, `Iso8601_date_time`, `Iso8601_duration`) and
    /// `C_TERMINOLOGY_CODE`'s `Terminology_code` `assumed_value` type.
    #[must_use]
    pub fn constrained_typename(&self) -> &'static str {
        match self {
            Self::CBoolean(_) => "Boolean",
            Self::CInteger(_) => "Integer",
            Self::CReal(_) => "Real",
            Self::CString(_) => "String",
            Self::CDate(_) => "Iso8601_date",
            Self::CTime(_) => "Iso8601_time",
            Self::CDateTime(_) => "Iso8601_date_time",
            Self::CDuration(_) => "Iso8601_duration",
            Self::CTerminologyCode(_) => "Terminology_code",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2_4::aom2::constraint_model::primitive::c_date::CDate;
    use crate::v2_4::aom2::constraint_model::primitive::c_integer::CInteger;

    fn integer(assumed_value: Option<f64>) -> CPrimitiveObject {
        CPrimitiveObject::CInteger(CInteger {
            parent: None,
            soc_parent: None,
            rm_type_name: "Integer".to_owned(),
            occurrences: None,
            node_id: "at9999".to_owned(),
            alternative_ids: None,
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value,
            is_enumerated_type_constraint: None,
            constraint: None,
        })
    }

    fn date() -> CPrimitiveObject {
        CPrimitiveObject::CDate(CDate {
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
            pattern_constraint: None,
        })
    }

    #[test]
    fn an_absent_assumed_value_is_no_assumed_value() {
        assert!(!integer(None).has_assumed_value());
        assert!(integer(Some(0.0)).has_assumed_value());
    }

    #[test]
    fn the_simple_types_drop_the_c_prefix() {
        assert_eq!(integer(None).constrained_typename(), "Integer");
    }

    #[test]
    fn the_temporal_types_name_their_iso8601_constraint_type() {
        assert_eq!(date().constrained_typename(), "Iso8601_date");
    }
}
