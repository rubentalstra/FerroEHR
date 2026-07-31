//! Hand-written spec functions of `P_BMM_CLASS` — the declared `is_generic`
//! predicate plus uniform access to the attributes every concrete
//! `P_BMM_CLASS` leaf carries.
//!
//! Spec: `LANG/docs/UML/classes/org.openehr.lang.bmm_persistence.p_bmm_class.adoc`
//! §Attributes (`name`, `ancestors`, `constants`, `properties`, `functions`,
//! `invariants`, `is_abstract`, `is_override`, `generic_parameter_defs`,
//! `source_schema_id`, `uid`, `ancestor_defs`) and §Functions (`is_generic`,
//! whose postcondition is `Result := generic_parameter_defs /= Void`).
//!
//! `P_BMM_CLASS` is emitted as a polymorphic slot over its own least-rich form
//! plus the `P_BMM_ENUMERATION` family, so every attribute read here reaches
//! through both levels; the enumeration-specific attributes live in
//! [`crate::bmm_persistence::p_bmm_enumeration_impl`].

use std::collections::BTreeMap;

use crate::bmm_persistence::p_bmm_class::PBmmClass;
use crate::bmm_persistence::p_bmm_constant::PBmmConstant;
use crate::bmm_persistence::p_bmm_enumeration::PBmmEnumeration;
use crate::bmm_persistence::p_bmm_function::PBmmFunction;
use crate::bmm_persistence::p_bmm_generic_parameter::PBmmGenericParameter;
use crate::bmm_persistence::p_bmm_generic_type::PBmmGenericType;
use crate::bmm_persistence::p_bmm_property::PBmmProperty;

/// Borrows one `P_BMM_CLASS` attribute from whichever concrete leaf a
/// [`PBmmClass`] holds.
macro_rules! class_field {
    ($value:expr, $field:ident) => {
        match $value {
            PBmmClass::PBmmEnumeration(PBmmEnumeration::PBmmEnumerationInteger(leaf)) => {
                &leaf.$field
            }
            PBmmClass::PBmmEnumeration(PBmmEnumeration::PBmmEnumerationString(leaf)) => {
                &leaf.$field
            }
            PBmmClass::PBmmEnumeration(PBmmEnumeration::PBmmEnumeration(leaf)) => &leaf.$field,
            PBmmClass::PBmmClass(leaf) => &leaf.$field,
        }
    };
}

/// Mutably borrows one `P_BMM_CLASS` attribute from whichever concrete leaf a
/// [`PBmmClass`] holds.
macro_rules! class_field_mut {
    ($value:expr, $field:ident) => {
        match $value {
            PBmmClass::PBmmEnumeration(PBmmEnumeration::PBmmEnumerationInteger(leaf)) => {
                &mut leaf.$field
            }
            PBmmClass::PBmmEnumeration(PBmmEnumeration::PBmmEnumerationString(leaf)) => {
                &mut leaf.$field
            }
            PBmmClass::PBmmEnumeration(PBmmEnumeration::PBmmEnumeration(leaf)) => &mut leaf.$field,
            PBmmClass::PBmmClass(leaf) => &mut leaf.$field,
        }
    };
}

impl PBmmClass {
    /// `P_BMM_CLASS.name`: "Name of the class" (class doc §Attributes).
    #[must_use]
    pub fn name(&self) -> &str {
        class_field!(self, name).as_str()
    }

    /// `P_BMM_MODEL_ELEMENT.documentation`: "Optional documentation of this
    /// element"
    /// (`org.openehr.lang.bmm_persistence.p_bmm_model_element.adoc`
    /// §Attributes).
    #[must_use]
    pub fn documentation(&self) -> Option<&str> {
        class_field!(self, documentation).as_deref()
    }

    /// `P_BMM_CLASS.ancestors`: "List of immediate inheritance parents. If
    /// there are generic ancestors, use `_ancestor_defs_` instead" (class doc
    /// §Attributes).
    #[must_use]
    pub fn ancestors(&self) -> &[String] {
        class_field!(self, ancestors).as_slice()
    }

    /// `P_BMM_CLASS.ancestor_defs`: "List of structured inheritance ancestors,
    /// used only in the case of generic inheritance" (class doc §Attributes).
    #[must_use]
    pub fn ancestor_defs(&self) -> &[PBmmGenericType] {
        class_field!(self, ancestor_defs).as_slice()
    }

    /// `P_BMM_CLASS.properties`: "List of attributes defined in this class",
    /// keyed by property name (class doc §Attributes).
    #[must_use]
    pub fn properties(&self) -> Option<&BTreeMap<String, PBmmProperty>> {
        class_field!(self, properties).as_ref()
    }

    /// `P_BMM_CLASS.functions`: "List of functions (routines) defined in this
    /// class, keyed by name" (class doc §Attributes).
    #[must_use]
    pub fn functions(&self) -> Option<&BTreeMap<String, PBmmFunction>> {
        class_field!(self, functions).as_ref()
    }

    /// `P_BMM_CLASS.constants`: "Constants defined in this class, keyed by
    /// name" (class doc §Attributes).
    #[must_use]
    pub fn constants(&self) -> Option<&BTreeMap<String, PBmmConstant>> {
        class_field!(self, constants).as_ref()
    }

    /// `P_BMM_CLASS.invariants`: "Invariants defined on this class, as a Hash
    /// of assertion expressions keyed by tag" (class doc §Attributes).
    #[must_use]
    pub fn invariants(&self) -> Option<&BTreeMap<String, String>> {
        class_field!(self, invariants).as_ref()
    }

    /// `P_BMM_CLASS.generic_parameter_defs`: "List of generic parameter
    /// definitions" (class doc §Attributes).
    #[must_use]
    pub fn generic_parameter_defs(&self) -> Option<&BTreeMap<String, PBmmGenericParameter>> {
        class_field!(self, generic_parameter_defs).as_ref()
    }

    /// `P_BMM_CLASS.is_abstract`: "True if this is an abstract type" (class doc
    /// §Attributes) — a `0..1` flag, absent meaning not abstract.
    #[must_use]
    pub fn is_abstract(&self) -> bool {
        class_field!(self, is_abstract).unwrap_or(false)
    }

    /// `P_BMM_CLASS.is_override`: "True if this class definition overrides one
    /// found in an included schema" (class doc §Attributes) — a `0..1` flag,
    /// absent meaning no override.
    #[must_use]
    pub fn is_override(&self) -> bool {
        class_field!(self, is_override).unwrap_or(false)
    }

    /// Records that this class definition overrides one of the same name in an
    /// included schema (`P_BMM_CLASS.is_override`, class doc §Attributes) —
    /// set by [`crate::bmm_persistence::include_resolution::resolve_includes`]
    /// when it detects the collision.
    pub fn set_is_override(&mut self, value: bool) {
        *class_field_mut!(self, is_override) = Some(value);
    }

    /// `P_BMM_CLASS.source_schema_id`: "Reference to original source schema
    /// defining this class. Set during `BMM_SCHEMA` materialise" (class doc
    /// §Attributes).
    #[must_use]
    pub fn source_schema_id(&self) -> &str {
        class_field!(self, source_schema_id).as_str()
    }

    /// `P_BMM_CLASS.uid`: "Unique id generated for later comparison during
    /// merging, in order to detect if two classes are the same. Assigned in
    /// post-load processing" (class doc §Attributes).
    #[must_use]
    pub fn uid(&self) -> i32 {
        *class_field!(self, uid)
    }

    /// `P_BMM_CLASS.is_generic`: "True if this class is a generic class", with
    /// postcondition `Result := generic_parameter_defs /= Void` (class doc
    /// §Functions).
    ///
    /// NOTE (adjudicated): the postcondition tests only for the presence of the
    /// attribute, and an ODIN `generic_parameter_defs = <>` block reads as
    /// present-but-empty; a class with no formal parameter is not a type
    /// generator ("Generic classes are those that have one or more
    /// substitutable generic type parameters",
    /// `LANG/docs/bmm_persistence/master04-syntax.adoc` §Generic Classes), so
    /// an empty map answers `False`.
    #[must_use]
    pub fn is_generic(&self) -> bool {
        self.generic_parameter_defs()
            .is_some_and(|parameters| !parameters.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::bmm_persistence::p_bmm_class::PBmmClass;
    use crate::bmm_persistence::p_bmm_class::PBmmClassData;
    use crate::bmm_persistence::p_bmm_generic_parameter::PBmmGenericParameter;

    /// A bare class definition named `name`, with the given formal generic
    /// parameter declarations.
    fn class(
        name: &str,
        generic_parameter_defs: Option<BTreeMap<String, PBmmGenericParameter>>,
    ) -> PBmmClass {
        PBmmClass::PBmmClass(PBmmClassData {
            documentation: None,
            name: name.to_owned(),
            ancestors: Vec::new(),
            constants: None,
            properties: None,
            functions: None,
            invariants: None,
            is_abstract: None,
            is_override: None,
            generic_parameter_defs,
            source_schema_id: "openehr_test_1.0.0".to_owned(),
            bmm_class: None,
            uid: 1,
            ancestor_defs: Vec::new(),
        })
    }

    /// The formal parameter `T` conforming to `Ordered`.
    fn ordered_t() -> BTreeMap<String, PBmmGenericParameter> {
        [(
            "T".to_owned(),
            PBmmGenericParameter {
                documentation: None,
                name: "T".to_owned(),
                conforms_to_type: Some("Ordered".to_owned()),
                bmm_generic_parameter: None,
            },
        )]
        .into_iter()
        .collect()
    }

    #[test]
    fn absent_flags_read_as_false() {
        let class = class("ELEMENT", None);
        assert!(!class.is_abstract());
        assert!(!class.is_override());
        assert!(!class.is_generic());
        assert_eq!(class.name(), "ELEMENT");
        assert_eq!(class.source_schema_id(), "openehr_test_1.0.0");
        assert_eq!(class.uid(), 1);
        assert!(class.ancestors().is_empty());
        assert!(class.ancestor_defs().is_empty());
        assert!(class.properties().is_none());
        assert!(class.functions().is_none());
        assert!(class.constants().is_none());
        assert!(class.invariants().is_none());
        assert!(class.documentation().is_none());
    }

    #[test]
    fn is_generic_needs_at_least_one_formal_parameter() {
        assert!(!class("Interval", Some(BTreeMap::new())).is_generic());
        assert!(class("Interval", Some(ordered_t())).is_generic());
    }

    #[test]
    fn set_is_override_marks_the_including_definition() {
        let mut class = class("DV_TEXT", None);
        class.set_is_override(true);
        assert!(class.is_override());
    }
}
