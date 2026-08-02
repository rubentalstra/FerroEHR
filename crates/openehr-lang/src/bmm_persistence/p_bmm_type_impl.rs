//! Hand-written spec functions of the `P_BMM_TYPE` family — the declared
//! `as_type_string` rendering.
//!
//! Spec:
//! `LANG/docs/UML/classes/org.openehr.lang.bmm_persistence.p_bmm_type.adoc`
//! §Functions (`as_type_string`: "Formal name of the type for display";
//! "effected in descendants"), with each descendant's parts from its own class
//! doc (`…p_bmm_simple_type.adoc` `type`, `…p_bmm_open_type.adoc` `type`,
//! `…p_bmm_generic_type.adoc` `root_type` + `generic_parameters` +
//! `generic_parameter_defs`, `…p_bmm_container_type.adoc` `container_type` +
//! `type`/`type_def`, `…p_bmm_indexed_container_type.adoc` `index_type`) and
//! the written forms of `LANG/docs/bmm_persistence/master04-syntax.adoc`
//! §Generic Classes (`DV_INTERVAL<DV_QUANTITY>`, `List<Reference<Party>>`) and
//! §Container Properties (`Hash <String, EVENT_ACTION>`).
//!
//! Generic delimiters `'<'`, `','`, `'>'` per
//! `org.openehr.lang.bmm3.bmm_definitions.adoc` §Constants
//! (`Generic_left_delimiter`, `Generic_separator`, `Generic_right_delimiter`) —
//! the same literals [`crate::bmm::core::bmm_type_impl`] renders with,
//! because the generated `BMM_DEFINITIONS` carries no constant for them.

use crate::bmm_persistence::p_bmm_base_type::PBmmBaseType;
use crate::bmm_persistence::p_bmm_container_type::PBmmContainerType;
use crate::bmm_persistence::p_bmm_container_type::PBmmContainerTypeData;
use crate::bmm_persistence::p_bmm_generic_type::PBmmGenericType;
use crate::bmm_persistence::p_bmm_indexed_container_type::PBmmIndexedContainerType;
use crate::bmm_persistence::p_bmm_open_type::PBmmOpenType;
use crate::bmm_persistence::p_bmm_simple_type::PBmmSimpleType;
use crate::bmm_persistence::p_bmm_type::PBmmType;

/// The display form of a container's target type: its `type` name, else the
/// nested `type_def`, else the empty string when the schema states neither
/// (both are `0..1` on `P_BMM_CONTAINER_TYPE`).
fn container_target(r#type: Option<&String>, type_def: Option<&PBmmBaseType>) -> String {
    match (r#type, type_def) {
        (Some(name), _) => name.clone(),
        (None, Some(nested)) => nested.as_type_string(),
        (None, None) => String::new(),
    }
}

impl PBmmSimpleType {
    /// `P_BMM_TYPE.as_type_string`: "Formal name of the type for display"
    /// (`org.openehr.lang.bmm_persistence.p_bmm_type.adoc` §Functions) — for a
    /// simple type the class name itself ("Name of type - must be a simple
    /// class name", `…p_bmm_simple_type.adoc` §Attributes).
    #[must_use]
    pub fn as_type_string(&self) -> String {
        self.r#type.clone()
    }
}

impl PBmmOpenType {
    /// `P_BMM_TYPE.as_type_string` for an open type: the generic parameter name,
    /// "a single letter like 'T', 'G' etc"
    /// (`…p_bmm_open_type.adoc` §Attributes).
    #[must_use]
    pub fn as_type_string(&self) -> String {
        self.r#type.clone()
    }
}

impl PBmmGenericType {
    /// `P_BMM_TYPE.as_type_string` for a generic type: `root_type<p1,p2>`, the
    /// form `master04-syntax.adoc` §Generic Classes writes
    /// (`DV_INTERVAL<DV_QUANTITY>`).
    ///
    /// The string parameters (`generic_parameters`) come first, then the
    /// structural ones (`generic_parameter_defs`) — the two lists are the
    /// simple and complex halves of one parameter list ("use
    /// `_generic_parameters_` for a list of string types; use
    /// `_generic_parameter_defs_` for a list of complex type references",
    /// §Generic Classes), and a schema uses one or the other, never both for
    /// the same position.
    #[must_use]
    pub fn as_type_string(&self) -> String {
        let mut parameters: Vec<String> = self.generic_parameters.clone().unwrap_or_default();
        parameters.extend(
            self.generic_parameter_defs
                .iter()
                .map(PBmmType::as_type_string),
        );
        if parameters.is_empty() {
            return self.root_type.clone();
        }
        let root = &self.root_type;
        let joined = parameters.join(",");
        format!("{root}<{joined}>")
    }
}

impl PBmmContainerTypeData {
    /// `P_BMM_TYPE.as_type_string` for a container type: `container_type<target>`,
    /// the form `master04-syntax.adoc` §Container Properties writes
    /// (`items: List<ITEM>`).
    #[must_use]
    pub fn as_type_string(&self) -> String {
        let container = &self.container_type;
        let target = container_target(self.r#type.as_ref(), self.type_def.as_ref());
        format!("{container}<{target}>")
    }
}

impl PBmmIndexedContainerType {
    /// `P_BMM_TYPE.as_type_string` for an indexed container type:
    /// `container_type<index_type,target>`, the form `master04-syntax.adoc`
    /// §Container Properties writes (`custom_actions: Hash <String,
    /// EVENT_ACTION>`).
    #[must_use]
    pub fn as_type_string(&self) -> String {
        let container = &self.container_type;
        let index = &self.index_type;
        let target = container_target(self.r#type.as_ref(), self.type_def.as_ref());
        format!("{container}<{index},{target}>")
    }
}

impl PBmmContainerType {
    /// `P_BMM_TYPE.as_type_string` dispatched over the `P_BMM_CONTAINER_TYPE`
    /// forms.
    #[must_use]
    pub fn as_type_string(&self) -> String {
        match self {
            Self::PBmmIndexedContainerType(indexed) => indexed.as_type_string(),
            Self::PBmmContainerType(data) => data.as_type_string(),
        }
    }
}

impl PBmmBaseType {
    /// `P_BMM_TYPE.as_type_string` dispatched over the `P_BMM_BASE_TYPE` forms.
    #[must_use]
    pub fn as_type_string(&self) -> String {
        match self {
            Self::PBmmGenericType(generic) => generic.as_type_string(),
            Self::PBmmOpenType(open) => open.as_type_string(),
            Self::PBmmSimpleType(simple) => simple.as_type_string(),
        }
    }
}

impl PBmmType {
    /// `P_BMM_TYPE.as_type_string`: "Formal name of the type for display"
    /// (class doc §Functions), dispatched over every `P_BMM_TYPE` form.
    #[must_use]
    pub fn as_type_string(&self) -> String {
        match self {
            Self::PBmmContainerType(container) => container.as_type_string(),
            Self::PBmmGenericType(generic) => generic.as_type_string(),
            Self::PBmmOpenType(open) => open.as_type_string(),
            Self::PBmmSimpleType(simple) => simple.as_type_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bmm_persistence::p_bmm_base_type::PBmmBaseType;
    use crate::bmm_persistence::p_bmm_container_type::PBmmContainerType;
    use crate::bmm_persistence::p_bmm_container_type::PBmmContainerTypeData;
    use crate::bmm_persistence::p_bmm_generic_type::PBmmGenericType;
    use crate::bmm_persistence::p_bmm_indexed_container_type::PBmmIndexedContainerType;
    use crate::bmm_persistence::p_bmm_simple_type::PBmmSimpleType;
    use crate::bmm_persistence::p_bmm_type::PBmmType;

    /// A `P_BMM_GENERIC_TYPE` over string parameters.
    fn generic(root: &str, parameters: &[&str]) -> PBmmGenericType {
        PBmmGenericType {
            bmm_type: None,
            value_constraint: None,
            root_type: root.to_owned(),
            generic_parameter_defs: Vec::new(),
            generic_parameters: openehr_base::containers::present(
                parameters.iter().map(|p| (*p).to_owned()).collect(),
            ),
        }
    }

    #[test]
    fn simple_and_generic_forms() {
        let simple = PBmmSimpleType {
            bmm_type: None,
            value_constraint: None,
            r#type: "DV_TEXT".to_owned(),
        };
        assert_eq!(simple.as_type_string(), "DV_TEXT");
        assert_eq!(
            generic("DV_INTERVAL", &["DV_QUANTITY"]).as_type_string(),
            "DV_INTERVAL<DV_QUANTITY>"
        );
        // A root type with no stated parameters renders bare.
        assert_eq!(generic("DV_INTERVAL", &[]).as_type_string(), "DV_INTERVAL");
    }

    #[test]
    fn nested_container_of_generic_renders_the_master04_example() {
        // master04-syntax.adoc §Generic Classes: `careProvider:
        // List<Reference<Party>>`.
        let nested = PBmmContainerType::PBmmContainerType(PBmmContainerTypeData {
            bmm_type: None,
            container_type: "List".to_owned(),
            type_def: Some(PBmmBaseType::PBmmGenericType(generic(
                "Reference",
                &["Party"],
            ))),
            r#type: None,
        });
        assert_eq!(nested.as_type_string(), "List<Reference<Party>>");
    }

    #[test]
    fn indexed_container_names_its_index_type() {
        // master04-syntax.adoc §Container Properties: `custom_actions: Hash
        // <String, EVENT_ACTION>`.
        let indexed = PBmmIndexedContainerType {
            bmm_type: None,
            container_type: "Hash".to_owned(),
            type_def: None,
            r#type: Some("EVENT_ACTION".to_owned()),
            index_type: "String".to_owned(),
        };
        assert_eq!(indexed.as_type_string(), "Hash<String,EVENT_ACTION>");
    }

    #[test]
    fn mixed_generic_parameter_defs_render_after_the_string_parameters() {
        // master04-syntax.adoc §Generic Classes CRAZY_TYPE shape, reduced: a
        // structural parameter beside a string one.
        let mut root = generic("REFERENCE_RANGE", &["Integer"]);
        root.generic_parameter_defs = vec![PBmmType::PBmmGenericType(generic(
            "DV_INTERVAL",
            &["DV_QUANTITY"],
        ))];
        assert_eq!(
            root.as_type_string(),
            "REFERENCE_RANGE<Integer,DV_INTERVAL<DV_QUANTITY>>"
        );
    }
}
