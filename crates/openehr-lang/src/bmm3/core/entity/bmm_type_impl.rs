//! Hand-written spec functions of the `BMM_TYPE` family — the BMM v2 core
//! type-level computed features (the naming trio plus type substitutions).
//!
//! Spec: `LANG/docs/bmm/master05-core.adoc` §Semantics §Basics, which defines
//! the three naming functions normatively:
//!
//! * `type_name` — "the effective type of an entity; for simple classes, this
//!   will just be the class name (`BMM_CLASS._name_`); for generic and
//!   container classes it will be generic name such as `List<T>`, `Interval<T>`
//!   etc; for feature types it will be the declared type, i.e. a simple name,
//!   an open type name (e.g. `T`) or a generic type name (e.g.
//!   `Interval<Time>`)";
//! * `type_signature` — "a form of the type name that can be used as a
//!   fully-defined type signature, which for generic classes includes generic
//!   constrainer types, giving a signature such as `Interval<T:Ordered>`";
//! * `conformance_type_name` — "a reduced form of the type useful in some
//!   circumstances that is either a simple class name, the _contained_ type for
//!   a container type (e.g. `ELEMENT` from the type `List<ELEMENT>`), and the
//!   _root_ type from a generic type (e.g. `Interval` from `Interval<T>`)".
//!
//! plus §Classes and Types (the four design-time type meta-types) and the class
//! definitions under `LANG/docs/UML/classes/`:
//! `org.openehr.lang.bmm.bmm_classifier.adoc`,
//! `…bmm.bmm_type.adoc`, `…bmm.bmm_simple_type.adoc`,
//! `…bmm.bmm_generic_type.adoc`, `…bmm.bmm_container_type.adoc`,
//! `…bmm.bmm_indexed_container_type.adoc`. The generated model merges the v2
//! and v3 (`bmm3`) type families into one enum, so the v3 class definitions
//! (`…bmm3.bmm_type.adoc`, `…bmm3.bmm_parameter_type.adoc`,
//! `…bmm3.bmm_signature.adoc`, `…bmm3.bmm_tuple_type.adoc`,
//! `…bmm3.bmm_status_type.adoc`) govern the v3-only leaves and are cited at
//! each site.
//!
//! The per-class functions are implemented on the individual generated structs
//! (which is where the class definitions declare them) and
//! [`BmmType`]/[`crate::bmm::core::bmm_classifier::BmmClassifier`] are pure
//! dispatchers over them.
//!
//! NOTE (adjudicated divergence, v2 vs v3 `flattened_type_list`): the v2
//! definition sits on `BMM_CLASSIFIER` alone — "Completely flattened list of
//! type names, flattening out all generic parameters"
//! (`org.openehr.lang.bmm.bmm_classifier.adoc` §Functions) — and no v2 subtype
//! redefines it, so the whole type expression flattens, container class
//! included (`Hash<String,Interval<Time>>` → `Hash`, `String`, `Interval`,
//! `Time`). v3 narrows the container case to the item type only
//! (`org.openehr.lang.bmm3.bmm_container_type.adoc` §Functions,
//! `Post_result: Result = item_type.flattened_type_list`). This surface is the
//! v2 core, so the v2 reading is implemented. Duplicate names are collapsed:
//! v2 is silent on duplicates, v3 states the intent for the composite types
//! ("the logical set (i.e. unique items)",
//! `org.openehr.lang.bmm3.bmm_signature.adoc` §Functions).

use crate::bmm3::core::entity::bmm_class::BmmClass;
use crate::bmm3::core::entity::bmm_container_type::BmmContainerType;
use crate::bmm3::core::entity::bmm_effective_type::BmmEffectiveType;
use crate::bmm3::core::entity::bmm_function_type::BmmFunctionType;
use crate::bmm3::core::entity::bmm_generic_type::BmmGenericType;
use crate::bmm3::core::entity::bmm_indexed_container_type::BmmIndexedContainerType;
use crate::bmm3::core::entity::bmm_parameter_type::BmmParameterType;
use crate::bmm3::core::entity::bmm_procedure_type::BmmProcedureType;
use crate::bmm3::core::entity::bmm_routine_type::BmmRoutineType;
use crate::bmm3::core::entity::bmm_routine_type::BmmRoutineTypeData;
use crate::bmm3::core::entity::bmm_signature::BmmSignature;
use crate::bmm3::core::entity::bmm_signature::BmmSignatureData;
use crate::bmm3::core::entity::bmm_simple_type::BmmSimpleType;
use crate::bmm3::core::entity::bmm_status_type::BmmStatusType;
use crate::bmm3::core::entity::bmm_tuple_type::BmmTupleType;
use crate::bmm3::core::entity::bmm_type::BmmType;
use crate::bmm3::core::entity::bmm_unitary_type::BmmUnitaryType;
use crate::bmm3::core::model::bmm_model::BmmModel;

/// The name of the top `Any` type, used wherever an unconstrained generic
/// parameter has to be reduced to a concrete conformance name
/// (`org.openehr.lang.bmm.bmm_open_type.adoc` §Attributes: the generic
/// constraint "will be 'Any' if nothing set in original model";
/// `org.openehr.lang.bmm3.bmm_definitions.adoc` §Functions `Any_class`:
/// "built-in class definition corresponding to the top `Any` class").
pub const ANY_TYPE_NAME: &str = "Any";

/// Collapses `names` to its first-occurrence-ordered set.
fn unique(names: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

impl BmmSimpleType {
    /// `BMM_SIMPLE_TYPE.type_name` (redefined): "Return base_class.type_name"
    /// (`org.openehr.lang.bmm.bmm_simple_type.adoc` §Functions). For the simple
    /// classes this meta-type is defined over
    /// (`org.openehr.lang.bmm3.bmm_simple_type.adoc` §Functions sharpens it to
    /// "Result is `_base_class.name_`") the two coincide.
    #[must_use]
    pub fn type_name(&self) -> String {
        self.base_class.type_name()
    }

    /// `BMM_CLASSIFIER.type_signature` for a simple type: the type name — a
    /// simple type has no generic parameters to constrain
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions: `type_signature`
    /// "Defaults to the value of `_type_name()_`").
    #[must_use]
    pub fn type_signature(&self) -> String {
        self.base_class.type_signature()
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for a simple type: "a simple
    /// class name" (`master05-core.adoc` §Basics).
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        self.base_class.name().to_owned()
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for a simple type: the base class
    /// name (`org.openehr.lang.bmm3.bmm_simple_type.adoc` §Functions: "Result
    /// is `_base_class.name_`").
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        vec![self.base_class.name().to_owned()]
    }
}

impl BmmGenericType {
    /// `BMM_GENERIC_TYPE.type_name` (redefined): "Return the full name of the
    /// type including generic parameters, e.g. `DV_INTERVAL<T>`,
    /// `TABLE<List<THING>,String>`"
    /// (`org.openehr.lang.bmm.bmm_generic_type.adoc` §Functions).
    #[must_use]
    pub fn type_name(&self) -> String {
        self.rendered(BmmType::type_name)
    }

    /// `BMM_GENERIC_TYPE.type_signature` (redefined): the type name with each
    /// generic parameter rendered in signature form, e.g. `Interval<T:Ordered>`
    /// (`org.openehr.lang.bmm3.bmm_generic_type.adoc` §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        self.rendered(BmmType::type_signature)
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for a generic type: "the _root_
    /// type from a generic type (e.g. `Interval` from `Interval<T>`)"
    /// (`master05-core.adoc` §Basics).
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        self.base_class.name.clone()
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for a generic type: "`_base_class.name_`
    /// followed by names of all generic parameter type names, which may be open
    /// or closed" (`org.openehr.lang.bmm3.bmm_generic_type.adoc` §Functions),
    /// each parameter flattened in turn.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        let mut out = vec![self.base_class.name.clone()];
        for parameter in &self.generic_parameters {
            out.extend(parameter.flattened_type_list());
        }
        unique(out)
    }

    /// `Root<p1,p2>` with each actual generic parameter rendered by `render`.
    fn rendered(&self, render: impl Fn(&BmmType) -> String) -> String {
        let root = &self.base_class.name;
        let parameters: Vec<String> = self.generic_parameters.iter().map(render).collect();
        let joined = parameters.join(",");
        format!("{root}<{joined}>")
    }
}

impl BmmContainerType {
    /// `BMM_CONTAINER_TYPE.type_name` (redefined): "Return full type name, e.g.
    /// `List<ELEMENT>`" (`org.openehr.lang.bmm.bmm_container_type.adoc`
    /// §Functions).
    ///
    /// NOTE: `BMM_INDEXED_CONTAINER_TYPE` does not redefine `type_name`
    /// (`org.openehr.lang.bmm.bmm_indexed_container_type.adoc` carries only the
    /// `index_type` attribute), so the two-parameter rendering
    /// `Hash<String,EVENT_ACTION>` is taken from that class's own §Description
    /// ("an indexed container such as `Hash<K,V>` … e.g. `String` in
    /// `Hash<String,EVENT_ACTION>`"), index type first.
    #[must_use]
    pub fn type_name(&self) -> String {
        self.rendered(BmmType::type_name)
    }

    /// `BMM_CLASSIFIER.type_signature` for a container type: the type name with
    /// the contained type in signature form
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions: `type_signature`
    /// "Defaults to the value of `_type_name()_`", refined only where a generic
    /// parameter carries a constrainer).
    #[must_use]
    pub fn type_signature(&self) -> String {
        self.rendered(BmmType::type_signature)
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for a container type: "the
    /// _contained_ type for a container type (e.g. `ELEMENT` from the type
    /// `List<ELEMENT>`)" (`master05-core.adoc` §Basics).
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        self.base_type().conformance_type_name()
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for a container type: the container
    /// class name, the index type (when the container is indexed) and the
    /// flattened contained type — see the module NOTE on the v2/v3 divergence.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmIndexedContainerType(indexed) => indexed.flattened_type_list(),
            Self::BmmContainerType(data) => {
                let mut out = vec![data.container_type.name().to_owned()];
                out.extend(data.base_type.flattened_type_list());
                unique(out)
            }
        }
    }

    /// `BMM_CONTAINER_TYPE.container_type`: "The type of the container"
    /// (`org.openehr.lang.bmm.bmm_container_type.adoc` §Attributes).
    #[must_use]
    pub fn container_type(&self) -> &BmmClass {
        match self {
            Self::BmmIndexedContainerType(indexed) => &indexed.container_type,
            Self::BmmContainerType(data) => &data.container_type,
        }
    }

    /// `BMM_CONTAINER_TYPE.base_type`: "The target type", i.e. the contained
    /// item type (`org.openehr.lang.bmm.bmm_container_type.adoc` §Attributes).
    #[must_use]
    pub fn base_type(&self) -> &BmmType {
        match self {
            Self::BmmIndexedContainerType(indexed) => &indexed.base_type,
            Self::BmmContainerType(data) => &data.base_type,
        }
    }

    /// `Container<item>` (or `Container<index,item>` when indexed) with each
    /// parameter rendered by `render`.
    fn rendered(&self, render: impl Fn(&BmmType) -> String) -> String {
        match self {
            Self::BmmIndexedContainerType(indexed) => indexed.rendered(render),
            Self::BmmContainerType(data) => {
                let container = data.container_type.name();
                let item = render(&data.base_type);
                format!("{container}<{item}>")
            }
        }
    }
}

impl BmmIndexedContainerType {
    /// `BMM_CONTAINER_TYPE.type_name` for an indexed container:
    /// `Hash<String,EVENT_ACTION>` — see the NOTE on
    /// [`BmmContainerType::type_name`] for why the index type comes first.
    #[must_use]
    pub fn type_name(&self) -> String {
        self.rendered(BmmType::type_name)
    }

    /// `BMM_CLASSIFIER.type_signature` for an indexed container: the type name
    /// with its parameters in signature form.
    #[must_use]
    pub fn type_signature(&self) -> String {
        self.rendered(BmmType::type_signature)
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for an indexed container: "the
    /// _contained_ type" (`master05-core.adoc` §Semantics §Basics), i.e. the
    /// VALUE type `base_type` — not the key type.
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        self.base_type.conformance_type_name()
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for an indexed container: the
    /// container class name, then the flattened index and contained types — see
    /// the module NOTE on the v2/v3 divergence.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        let mut out = vec![self.container_type.name().to_owned()];
        out.extend(self.index_type.flattened_type_list());
        out.extend(self.base_type.flattened_type_list());
        unique(out)
    }

    /// `BMM_INDEXED_CONTAINER_TYPE.index_type`: "The key (index) type of the
    /// container, e.g. `String` in `Hash<String,EVENT_ACTION>`"
    /// (`org.openehr.lang.bmm.bmm_indexed_container_type.adoc` §Attributes).
    #[must_use]
    pub fn index_type_name(&self) -> String {
        self.index_type.type_name()
    }

    /// `Container<index,item>` with both parameters rendered by `render`.
    fn rendered(&self, render: impl Fn(&BmmType) -> String) -> String {
        let container = self.container_type.name();
        let index = self.index_type.type_name();
        let item = render(&self.base_type);
        format!("{container}<{index},{item}>")
    }
}

impl BmmParameterType {
    /// `BMM_PARAMETER_TYPE.type_name` (effected): "Return `_name_`"
    /// (`org.openehr.lang.bmm3.bmm_parameter_type.adoc` §Functions) — the open
    /// type name `T`, `U` etc. of `master05-core.adoc` §Basics.
    #[must_use]
    pub fn type_name(&self) -> &str {
        self.name.as_str()
    }

    /// `BMM_PARAMETER_TYPE.type_signature` (redefined): "Signature form of the
    /// open type, including constrainer type if there is one, e.g. `T:Ordered`"
    /// (`org.openehr.lang.bmm3.bmm_parameter_type.adoc` §Functions).
    ///
    /// Delimiter per `org.openehr.lang.bmm3.bmm_definitions.adoc` §Constants
    /// (`Generic_constraint_delimiter` `':'`).
    #[must_use]
    pub fn type_signature(&self) -> String {
        match self.flattened_conforms_to_type() {
            Some(constraint) => {
                let name = &self.name;
                let constraint = constraint.type_name();
                format!("{name}:{constraint}")
            }
            None => self.name.clone(),
        }
    }

    /// `BMM_PARAMETER_TYPE.flattened_conforms_to_type`: "Result is either
    /// `_conforms_to_type_` or `_inheritance_precursor.flattened_conforms_to_type_`"
    /// (`org.openehr.lang.bmm3.bmm_parameter_type.adoc` §Functions).
    #[must_use]
    pub fn flattened_conforms_to_type(&self) -> Option<&BmmEffectiveType> {
        match &self.type_constraint {
            Some(constraint) => Some(constraint),
            None => self
                .inheritance_precursor
                .as_ref()
                .and_then(|precursor| precursor.flattened_conforms_to_type()),
        }
    }

    /// `BMM_PARAMETER_TYPE.effective_type` (effected): "Generate ultimate
    /// conformance type, which is either `_flattened_conforms_to_type_` or if
    /// not set, `'Any'`" (`org.openehr.lang.bmm3.bmm_parameter_type.adoc`
    /// §Functions), projected to the type NAME — see the projection NOTE on
    /// [`crate::bmm::core::bmm_generic_parameter::BmmGenericParameter::effective_conforms_to_type_name`].
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self.flattened_conforms_to_type() {
            Some(constraint) => constraint.conformance_type_name(),
            None => ANY_TYPE_NAME.to_owned(),
        }
    }

    /// `BMM_PARAMETER_TYPE.flattened_type_list` (effected): "Result is either
    /// `_flattened_conforms_to_type.flattened_type_list_` or the `Any` type"
    /// (`org.openehr.lang.bmm3.bmm_parameter_type.adoc` §Functions).
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self.flattened_conforms_to_type() {
            Some(constraint) => constraint.flattened_type_list(),
            None => vec![ANY_TYPE_NAME.to_owned()],
        }
    }
}

impl BmmStatusType {
    /// `BMM_STATUS_TYPE.type_name`: the built-in `base_name` `"Status"`
    /// (`org.openehr.lang.bmm3.bmm_status_type.adoc` §Constants).
    #[expect(
        clippy::unused_self,
        reason = "the class definitions declare this as an instance function on the meta-type (BMM_CLASSIFIER §Functions); a built-in meta-type's name is simply constant"
    )]
    #[must_use]
    pub fn type_name(&self) -> String {
        Self::BASE_NAME.to_owned()
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for the status meta-type: its
    /// built-in base name (`org.openehr.lang.bmm3.bmm_status_type.adoc`
    /// §Constants).
    #[expect(
        clippy::unused_self,
        reason = "the class definitions declare this as an instance function on the meta-type (BMM_CLASSIFIER §Functions); a built-in meta-type's name is simply constant"
    )]
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        Self::BASE_NAME.to_owned()
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for the status meta-type: its
    /// built-in base name (`org.openehr.lang.bmm3.bmm_status_type.adoc`
    /// §Constants).
    #[expect(
        clippy::unused_self,
        reason = "the class definitions declare this as an instance function on the meta-type (BMM_CLASSIFIER §Functions); a built-in meta-type's name is simply constant"
    )]
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        vec![Self::BASE_NAME.to_owned()]
    }
}

impl BmmTupleType {
    /// `BMM_TUPLE_TYPE.type_name`: the built-in `base_name` `"Tuple"`
    /// (`org.openehr.lang.bmm3.bmm_tuple_type.adoc` §Constants).
    #[expect(
        clippy::unused_self,
        reason = "the class definitions declare this as an instance function on the meta-type (BMM_CLASSIFIER §Functions); a built-in meta-type's name is simply constant"
    )]
    #[must_use]
    pub fn type_name(&self) -> String {
        Self::BASE_NAME.to_owned()
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for the tuple meta-type: its
    /// built-in base name (`org.openehr.lang.bmm3.bmm_tuple_type.adoc`
    /// §Constants).
    #[expect(
        clippy::unused_self,
        reason = "the class definitions declare this as an instance function on the meta-type (BMM_CLASSIFIER §Functions); a built-in meta-type's name is simply constant"
    )]
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        Self::BASE_NAME.to_owned()
    }

    /// `BMM_TUPLE_TYPE.flattened_type_list` (effected): "Return the logical set
    /// (i.e. unique types) from the merge of `_flattened_type_list_()` called on
    /// each member of `_item_types_`"
    /// (`org.openehr.lang.bmm3.bmm_tuple_type.adoc` §Functions).
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        let mut out = Vec::new();
        for item in self.item_types.values() {
            out.extend(item.flattened_type_list());
        }
        unique(out)
    }
}

impl BmmSignature {
    /// `BMM_SIGNATURE.type_name`: the built-in `base_name` of the effective
    /// signature meta-type — `"Signature"`, `"Routine"`, `"Function"` or
    /// `"Procedure"` (`org.openehr.lang.bmm3.bmm_signature.adoc`,
    /// `…bmm3.bmm_routine_type.adoc`, `…bmm3.bmm_function_type.adoc`,
    /// `…bmm3.bmm_procedure_type.adoc`, all §Constants).
    ///
    /// `BMM_PROPERTY_TYPE` declares no `base_name` of its own
    /// (`org.openehr.lang.bmm3.bmm_property_type.adoc`), so it takes the
    /// inherited `BMM_SIGNATURE` one.
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::BmmPropertyType(_) | Self::BmmSignature(_) => {
                BmmSignatureData::BASE_NAME.to_owned()
            }
            Self::BmmRoutineType(routine) => match routine.as_ref() {
                BmmRoutineType::BmmFunctionType(_) => BmmFunctionType::BASE_NAME.to_owned(),
                BmmRoutineType::BmmProcedureType(_) => BmmProcedureType::BASE_NAME.to_owned(),
                BmmRoutineType::BmmRoutineType(_) => BmmRoutineTypeData::BASE_NAME.to_owned(),
            },
        }
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for a signature meta-type: its
    /// built-in base name — a signature is not a model class, so there is no
    /// reduced form beyond the name itself (`master05-core.adoc` §Basics).
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        self.type_name()
    }

    /// `BMM_SIGNATURE.flattened_type_list` (effected): "Return the logical set
    /// (i.e. unique items) consisting of `_argument_types.flattened_type_list_()`
    /// and `_result_type.flattened_type_list_()`"
    /// (`org.openehr.lang.bmm3.bmm_signature.adoc` §Functions).
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        let mut out = Vec::new();
        match self {
            Self::BmmPropertyType(property) => {
                out.extend(property.result_type.flattened_type_list());
            }
            Self::BmmSignature(data) => out.extend(data.result_type.flattened_type_list()),
            Self::BmmRoutineType(routine) => match routine.as_ref() {
                BmmRoutineType::BmmFunctionType(function) => {
                    out.extend(function.result_type.flattened_type_list());
                    if let Some(arguments) = &function.argument_types {
                        out.extend(arguments.flattened_type_list());
                    }
                }
                BmmRoutineType::BmmProcedureType(procedure) => {
                    if let Some(result) = &procedure.result_type {
                        out.extend(result.flattened_type_list());
                    }
                    if let Some(arguments) = &procedure.argument_types {
                        out.extend(arguments.flattened_type_list());
                    }
                }
                BmmRoutineType::BmmRoutineType(data) => {
                    out.extend(data.result_type.flattened_type_list());
                    if let Some(arguments) = &data.argument_types {
                        out.extend(arguments.flattened_type_list());
                    }
                }
            },
        }
        unique(out)
    }
}

impl BmmEffectiveType {
    /// `BMM_CLASSIFIER.type_name` for a concrete unitary type usable as an
    /// actual generic parameter (`org.openehr.lang.bmm3.bmm_effective_type.adoc`
    /// §Description).
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for an effective type.
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self {
            Self::BmmSignature(signature) => signature.conformance_type_name(),
            Self::BmmStatusType(status) => status.conformance_type_name(),
            Self::BmmTupleType(tuple) => tuple.conformance_type_name(),
        }
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for an effective type.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmSignature(signature) => signature.flattened_type_list(),
            Self::BmmStatusType(status) => status.flattened_type_list(),
            Self::BmmTupleType(tuple) => tuple.flattened_type_list(),
        }
    }
}

impl BmmUnitaryType {
    /// `BMM_CLASSIFIER.type_name` for a unitary type, i.e. "the type of any
    /// instantiated object that is not a container object"
    /// (`org.openehr.lang.bmm3.bmm_unitary_type.adoc` §Description).
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::BmmParameterType(parameter) => parameter.type_name().to_owned(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for a unitary type.
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self {
            Self::BmmParameterType(parameter) => parameter.conformance_type_name(),
            Self::BmmSignature(signature) => signature.conformance_type_name(),
            Self::BmmStatusType(status) => status.conformance_type_name(),
            Self::BmmTupleType(tuple) => tuple.conformance_type_name(),
        }
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for a unitary type.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmParameterType(parameter) => parameter.flattened_type_list(),
            Self::BmmSignature(signature) => signature.flattened_type_list(),
            Self::BmmStatusType(status) => status.flattened_type_list(),
            Self::BmmTupleType(tuple) => tuple.flattened_type_list(),
        }
    }
}

impl BmmType {
    /// `BMM_CLASSIFIER.type_name`: "Formal string form of the type as per UML"
    /// (`org.openehr.lang.bmm.bmm_classifier.adoc` §Functions), dispatched to
    /// the effective meta-type — see the module docs for the §Basics
    /// definition.
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::BmmContainerType(container) => container.type_name(),
            Self::BmmGenericType(generic) => generic.type_name(),
            Self::BmmOpenType(open) => open.type_name().to_owned(),
            Self::BmmParameterType(parameter) => parameter.type_name().to_owned(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmSimpleType(simple) => simple.type_name(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_CLASSIFIER.type_signature`: "Signature form of the type, which for
    /// generics includes generic parameter constrainer types e.g.
    /// Interval<T:Ordered>" (`org.openehr.lang.bmm.bmm_classifier.adoc`
    /// §Functions); it "Defaults to the value of `_type_name()_`" for every
    /// meta-type that carries no constrainer
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        match self {
            Self::BmmContainerType(container) => container.type_signature(),
            Self::BmmGenericType(generic) => generic.type_signature(),
            Self::BmmOpenType(open) => open.type_signature(),
            Self::BmmParameterType(parameter) => parameter.type_signature(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmSimpleType(simple) => simple.type_signature(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_CLASSIFIER.conformance_type_name`: "a reduced form of the type …
    /// either a simple class name, the _contained_ type for a container type
    /// (e.g. `ELEMENT` from the type `List<ELEMENT>`), and the _root_ type from
    /// a generic type (e.g. `Interval` from `Interval<T>`)"
    /// (`master05-core.adoc` §Semantics §Basics).
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self {
            Self::BmmContainerType(container) => container.conformance_type_name(),
            Self::BmmGenericType(generic) => generic.conformance_type_name(),
            Self::BmmOpenType(open) => open.conformance_type_name(),
            Self::BmmParameterType(parameter) => parameter.conformance_type_name(),
            Self::BmmSignature(signature) => signature.conformance_type_name(),
            Self::BmmSimpleType(simple) => simple.conformance_type_name(),
            Self::BmmStatusType(status) => status.conformance_type_name(),
            Self::BmmTupleType(tuple) => tuple.conformance_type_name(),
        }
    }

    /// `BMM_CLASSIFIER.flattened_type_list`: "Completely flattened list of type
    /// names, flattening out all generic parameters"
    /// (`org.openehr.lang.bmm.bmm_classifier.adoc` §Functions) — e.g.
    /// `Hash<String,Interval<Time>>` flattens to `Hash`, `String`, `Interval`,
    /// `Time`. See the module NOTE for the v2/v3 container divergence.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmContainerType(container) => container.flattened_type_list(),
            Self::BmmGenericType(generic) => generic.flattened_type_list(),
            Self::BmmOpenType(open) => open.flattened_type_list(),
            Self::BmmParameterType(parameter) => parameter.flattened_type_list(),
            Self::BmmSignature(signature) => signature.flattened_type_list(),
            Self::BmmSimpleType(simple) => simple.flattened_type_list(),
            Self::BmmStatusType(status) => status.flattened_type_list(),
            Self::BmmTupleType(tuple) => tuple.flattened_type_list(),
        }
    }

    /// `BMM_TYPE.has_type_substitutions`: "Determine if there are any type
    /// substitutions" (`org.openehr.lang.bmm.bmm_type.adoc` §Functions).
    #[must_use]
    pub fn has_type_substitutions(&self, model: &BmmModel) -> bool {
        !self.type_substitutions(model).is_empty()
    }

    /// `BMM_TYPE.type_substitutions`: "List of type substitutions if any
    /// available for this type within the current BMM model"
    /// (`org.openehr.lang.bmm.bmm_type.adoc` §Functions) — the descendants of
    /// this type's conformance root class, resolved through `model`
    /// ("`_a_desc_type_` has `_an_anc_type_` in its ancestors" is the
    /// conformance relation the substitution set realises,
    /// `org.openehr.lang.bmm.bmm_model.adoc` §Functions `type_conforms_to`).
    ///
    /// NOTE: the class doc's signature takes no argument because it assumes a
    /// live model with back-references; the generated persistence-shaped types
    /// record descendants as names only, so the model is passed in (see
    /// [`BmmClass::all_descendants`]).
    #[must_use]
    pub fn type_substitutions(&self, model: &BmmModel) -> Vec<String> {
        match model.class_definition(&self.conformance_type_name()) {
            Some(class) => class.all_descendants(model),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bmm::core::bmm_generic_parameter::BmmGenericParameter;
    use crate::bmm::core::bmm_open_type::BmmOpenType;
    use crate::bmm3::core::entity::bmm_class::BmmClass;
    use crate::bmm3::core::entity::bmm_container_type::BmmContainerType;
    use crate::bmm3::core::entity::bmm_container_type::BmmContainerTypeData;
    use crate::bmm3::core::entity::bmm_generic_class::BmmGenericClass;
    use crate::bmm3::core::entity::bmm_generic_type::BmmGenericType;
    use crate::bmm3::core::entity::bmm_indexed_container_type::BmmIndexedContainerType;
    use crate::bmm3::core::entity::bmm_simple_class::BmmSimpleClass;
    use crate::bmm3::core::entity::bmm_simple_type::BmmSimpleType;
    use crate::bmm3::core::entity::bmm_type::BmmType;
    use crate::bmm3::core::model::bmm_package::BmmPackage;

    /// An empty package node.
    fn package() -> BmmPackage {
        BmmPackage {
            documentation: None,
            packages: None,
            name: "org.openehr.base.foundation_types".to_owned(),
            classes: Vec::new(),
        }
    }

    /// A simple class named `name`.
    fn simple_class(name: &str) -> BmmClass {
        BmmClass::BmmSimpleClass(BmmSimpleClass {
            documentation: None,
            name: name.to_owned(),
            ancestors: None,
            package: package(),
            properties: None,
            source_schema_id: "openehr_test_1.0.0".to_owned(),
            immediate_descendants: Vec::new(),
            is_abstract: false,
            is_primitive_type: false,
            is_override: false,
        })
    }

    /// A generic class named `name` with one formal parameter `T`, optionally
    /// constrained.
    fn generic_class(name: &str, constraint: Option<&str>) -> BmmGenericClass {
        BmmGenericClass {
            documentation: None,
            name: name.to_owned(),
            ancestors: None,
            package: package(),
            properties: None,
            source_schema_id: "openehr_test_1.0.0".to_owned(),
            immediate_descendants: Vec::new(),
            is_abstract: false,
            is_primitive_type: false,
            is_override: false,
            generic_parameters: [(
                "T".to_owned(),
                BmmGenericParameter {
                    documentation: None,
                    name: "T".to_owned(),
                    conforms_to_type: constraint.map(simple_class),
                    inheritance_precursor: None,
                },
            )]
            .into_iter()
            .collect(),
        }
    }

    /// A `BMM_SIMPLE_TYPE` over the class named `name`.
    fn simple(name: &str) -> BmmType {
        BmmType::BmmSimpleType(BmmSimpleType {
            documentation: None,
            base_class: simple_class(name),
        })
    }

    /// `Root<parameter>` as a `BMM_GENERIC_TYPE`.
    fn generic(root: &str, parameter: BmmType) -> BmmType {
        BmmType::BmmGenericType(BmmGenericType {
            documentation: None,
            generic_parameters: vec![parameter],
            base_class: generic_class(root, None),
        })
    }

    /// `Container<item>` as a `BMM_CONTAINER_TYPE`.
    fn container(container_class: &str, item: BmmType) -> BmmType {
        BmmType::BmmContainerType(Box::new(BmmContainerType::BmmContainerType(
            BmmContainerTypeData {
                documentation: None,
                container_type: simple_class(container_class),
                base_type: Box::new(item),
            },
        )))
    }

    /// `Hash<index,item>` as a `BMM_INDEXED_CONTAINER_TYPE`.
    fn indexed(container_class: &str, index: &str, item: BmmType) -> BmmType {
        BmmType::BmmContainerType(Box::new(BmmContainerType::BmmIndexedContainerType(
            Box::new(BmmIndexedContainerType {
                documentation: None,
                container_type: simple_class(container_class),
                base_type: item,
                index_type: BmmSimpleType {
                    documentation: None,
                    base_class: simple_class(index),
                },
            }),
        )))
    }

    #[test]
    fn type_names_follow_master05_basics() {
        assert_eq!(simple("ELEMENT").type_name(), "ELEMENT");
        assert_eq!(
            container("List", simple("ELEMENT")).type_name(),
            "List<ELEMENT>"
        );
        assert_eq!(
            generic("Interval", simple("Time")).type_name(),
            "Interval<Time>"
        );
        assert_eq!(
            indexed("Hash", "String", simple("EVENT_ACTION")).type_name(),
            "Hash<String,EVENT_ACTION>"
        );
    }

    #[test]
    fn conformance_type_name_reduces_container_and_generic_forms() {
        // master05-core.adoc §Basics: the contained type for a container, the
        // root type for a generic type, the class name for a simple type.
        assert_eq!(simple("ELEMENT").conformance_type_name(), "ELEMENT");
        assert_eq!(
            container("List", simple("ELEMENT")).conformance_type_name(),
            "ELEMENT"
        );
        assert_eq!(
            generic("Interval", simple("Time")).conformance_type_name(),
            "Interval"
        );
    }

    #[test]
    fn flattened_type_list_flattens_nested_generics() {
        let nested = indexed("Hash", "String", generic("Interval", simple("Time")));
        assert_eq!(
            nested.flattened_type_list(),
            [
                "Hash".to_owned(),
                "String".to_owned(),
                "Interval".to_owned(),
                "Time".to_owned()
            ]
        );
    }

    #[test]
    fn flattened_type_list_collapses_duplicates() {
        let hash = indexed("Hash", "String", simple("String"));
        assert_eq!(
            hash.flattened_type_list(),
            ["Hash".to_owned(), "String".to_owned()]
        );
    }

    #[test]
    fn generic_type_signature_carries_the_constrainer() {
        let interval = BmmType::BmmGenericType(BmmGenericType {
            documentation: None,
            generic_parameters: vec![BmmType::BmmOpenType(BmmOpenType {
                documentation: None,
                generic_constraint: BmmGenericParameter {
                    documentation: None,
                    name: "T".to_owned(),
                    conforms_to_type: Some(simple_class("Ordered")),
                    inheritance_precursor: None,
                },
            })],
            base_class: generic_class("Interval", Some("Ordered")),
        });
        assert_eq!(interval.type_name(), "Interval<T>");
        assert_eq!(interval.type_signature(), "Interval<T:Ordered>");
        assert_eq!(interval.conformance_type_name(), "Interval");
        assert_eq!(
            interval.flattened_type_list(),
            ["Interval".to_owned(), "Ordered".to_owned()]
        );
    }
}
