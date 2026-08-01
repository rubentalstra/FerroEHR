//! Hand-written spec functions of the BMM **v3** `BMM_TYPE` family — the type
//! naming surface of the v3 (`org.openehr.lang.bmm3`) generation.
//!
//! Spec: `LANG/docs/bmm3/master06-core-types.adoc` (the v3 type meta-model) plus
//! the class definitions under `LANG/docs/UML/classes/`:
//! `org.openehr.lang.bmm3.bmm_type.adoc` §Functions (`type_name` — "Formal
//! string form of the type as per UML"; `type_signature` — "Signature form of
//! the type name, which for generics includes generic parameter constrainer
//! types E.g. `Interval<T:Ordered>`. Defaults to the value of `_type_name()_`";
//! `flattened_type_list` — "Completely flattened list of type names, flattening
//! out all generic parameters"), `…bmm3.bmm_simple_type.adoc`,
//! `…bmm3.bmm_generic_type.adoc`, `…bmm3.bmm_container_type.adoc`,
//! `…bmm3.bmm_indexed_container_type.adoc`, `…bmm3.bmm_parameter_type.adoc`,
//! `…bmm3.bmm_signature.adoc`, `…bmm3.bmm_status_type.adoc`,
//! `…bmm3.bmm_tuple_type.adoc`, `…bmm3.bmm_effective_type.adoc`,
//! `…bmm3.bmm_unitary_type.adoc`, all §Functions.
//!
//! This is the v3 generation's OWN surface. The v2.x generation
//! (`LANG/docs/bmm/master01-preface.adoc` §History — "the normative,
//! tool-implemented version") carries its own, structurally different one at
//! [`crate::bmm::core::bmm_type_impl`]; the two never share an impl, because
//! `BMM_TYPE`, `BMM_SIMPLE_TYPE`, `BMM_GENERIC_TYPE` and `BMM_CONTAINER_TYPE`
//! are different classes in the two generations
//! (`LANG/docs/bmm3/master00-amendment_record.adoc` SPECLANG-14, "Formalise the
//! BMM v2/v3 split").
//!
//! TODO: only the naming/flattening surface is implemented here. The rest of the
//! v3 type lattice declared by those class definitions — `is_abstract`,
//! `is_open`/`is_closed`/`is_partially_closed`, `effective_base_class`,
//! `unitary_type`, `effective_type`, `is_primitive`, `type_base_name` — and the
//! v3 class/feature/model function surface (`BMM_CLASS.flat_features`,
//! `BMM_MODEL` navigation, the declared invariants) are unimplemented.

use crate::bmm3::core::entity::bmm_class::BmmClass;
use crate::bmm3::core::entity::bmm_container_type::BmmContainerType;
use crate::bmm3::core::entity::bmm_effective_type::BmmEffectiveType;
use crate::bmm3::core::entity::bmm_function_type::BmmFunctionType;
use crate::bmm3::core::entity::bmm_generic_type::BmmGenericType;
use crate::bmm3::core::entity::bmm_indexed_container_type::BmmIndexedContainerType;
use crate::bmm3::core::entity::bmm_model_type::BmmModelType;
use crate::bmm3::core::entity::bmm_parameter_type::BmmParameterType;
use crate::bmm3::core::entity::bmm_procedure_type::BmmProcedureType;
use crate::bmm3::core::entity::bmm_routine_type::BmmRoutineType;
use crate::bmm3::core::entity::bmm_routine_type::BmmRoutineTypeData;
use crate::bmm3::core::entity::bmm_signature::BmmSignature;
use crate::bmm3::core::entity::bmm_signature::BmmSignatureData;
use crate::bmm3::core::entity::bmm_simple_class::BmmSimpleClass;
use crate::bmm3::core::entity::bmm_simple_type::BmmSimpleType;
use crate::bmm3::core::entity::bmm_status_type::BmmStatusType;
use crate::bmm3::core::entity::bmm_tuple_type::BmmTupleType;
use crate::bmm3::core::entity::bmm_type::BmmType;
use crate::bmm3::core::entity::bmm_unitary_type::BmmUnitaryType;
use crate::bmm3::core::entity::range_constrained::bmm_enumeration::BmmEnumeration;

/// The name of the top `Any` type, used wherever an unconstrained generic
/// parameter has to be reduced to a concrete conformance name
/// (`org.openehr.lang.bmm3.bmm_definitions.adoc` §Functions `Any_class`:
/// "built-in class definition corresponding to the top `Any` class").
pub const ANY_TYPE_NAME: &str = "Any";

/// Collapses `names` to its first-occurrence-ordered set — the "logical set
/// (i.e. unique items)" the v3 composite meta-types state for their flattened
/// type lists (`org.openehr.lang.bmm3.bmm_signature.adoc` §Functions).
fn unique(names: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

impl BmmEnumeration {
    /// `BMM_MODEL_ELEMENT.name`: "Name of this model element"
    /// (`org.openehr.lang.bmm3.bmm_model_element.adoc` §Attributes).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::BmmEnumerationInteger(e) => e.name.as_str(),
            Self::BmmEnumerationString(e) => e.name.as_str(),
            Self::BmmEnumeration(e) => e.name.as_str(),
        }
    }
}

impl BmmSimpleClass {
    /// `BMM_MODEL_ELEMENT.name`: "Name of this model element"
    /// (`org.openehr.lang.bmm3.bmm_model_element.adoc` §Attributes).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::BmmEnumeration(e) => e.name(),
            Self::BmmSimpleClass(c) => c.name.as_str(),
        }
    }
}

impl BmmClass {
    /// `BMM_MODEL_ELEMENT.name`: "Name of this model element"
    /// (`org.openehr.lang.bmm3.bmm_model_element.adoc` §Attributes). Note that
    /// unlike UML this is "just the root name, even if the class is generic"
    /// (`org.openehr.lang.bmm3.bmm_class.adoc` §Description NOTE).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::BmmGenericClass(c) => c.name.as_str(),
            Self::BmmSimpleClass(c) => c.name(),
        }
    }
}

impl BmmSimpleType {
    /// `BMM_SIMPLE_TYPE.type_name` (effected): "Result is `_base_class.name_`"
    /// (`org.openehr.lang.bmm3.bmm_simple_type.adoc` §Functions).
    #[must_use]
    pub fn type_name(&self) -> String {
        self.base_class.name().to_owned()
    }

    /// `BMM_TYPE.type_signature`: "Defaults to the value of `_type_name()_`" for
    /// a meta-type with no constrainer (`org.openehr.lang.bmm3.bmm_type.adoc`
    /// §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        self.type_name()
    }

    /// The reduced conformance form of a simple type: the defining class name
    /// (`master06-core-types.adoc` §Simple Type — a simple type "is just a
    /// reference to a class").
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        self.base_class.name().to_owned()
    }

    /// `BMM_SIMPLE_TYPE.flattened_type_list` (effected): "Result is
    /// `_base_class.name_`" (`org.openehr.lang.bmm3.bmm_simple_type.adoc`
    /// §Functions).
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        vec![self.base_class.name().to_owned()]
    }
}

impl BmmGenericType {
    /// `BMM_GENERIC_TYPE.type_name` (effected): "Return the full name of the
    /// type including generic parameters, e.g. `DV_INTERVAL<T>`,
    /// `TABLE<List<THING>,String>`" (`org.openehr.lang.bmm3.bmm_generic_type.adoc`
    /// §Functions).
    #[must_use]
    pub fn type_name(&self) -> String {
        let root = &self.base_class.name;
        let parameters: Vec<String> = self
            .generic_parameters
            .iter()
            .map(BmmUnitaryType::type_name)
            .collect();
        format!("{root}<{}>", parameters.join(","))
    }

    /// `BMM_GENERIC_TYPE.type_signature` (redefined): "Signature form of the
    /// type, which for generics includes generic parameter constrainer types
    /// E.g. `Interval<T:Ordered>`"
    /// (`org.openehr.lang.bmm3.bmm_generic_type.adoc` §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        let root = &self.base_class.name;
        let parameters: Vec<String> = self
            .generic_parameters
            .iter()
            .map(BmmUnitaryType::type_signature)
            .collect();
        format!("{root}<{}>", parameters.join(","))
    }

    /// The reduced conformance form of a generic type: "the _root_ type from a
    /// generic type (e.g. `Interval` from `Interval<T>`)"
    /// (`LANG/docs/bmm/master05-core.adoc` §Semantics §Basics — the v3 chapters
    /// keep the same reduction, `master06-core-types.adoc` §Generic Type).
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        self.base_class.name.clone()
    }

    /// `BMM_GENERIC_TYPE.flattened_type_list` (effected): "Result is
    /// `_base_class.name_` followed by names of all generic parameter type
    /// names, which may be open or closed"
    /// (`org.openehr.lang.bmm3.bmm_generic_type.adoc` §Functions).
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        let mut out = vec![self.base_class.name.clone()];
        for parameter in &self.generic_parameters {
            out.extend(parameter.flattened_type_list());
        }
        unique(out)
    }
}

impl BmmContainerType {
    /// `BMM_CONTAINER_TYPE.type_name` (effected): "Return full type name, e.g.
    /// `List<ELEMENT>`" (`org.openehr.lang.bmm3.bmm_container_type.adoc`
    /// §Functions); an indexed container renders both parameters, e.g.
    /// `HashMap<String, ELEMENT>`
    /// (`…bmm3.bmm_indexed_container_type.adoc` §Functions).
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::BmmIndexedContainerType(indexed) => indexed.type_name(),
            Self::BmmContainerType(data) => {
                let container = &data.container_class.name;
                let item = data.item_type.type_name();
                format!("{container}<{item}>")
            }
        }
    }

    /// `BMM_TYPE.type_signature`: the container form with each parameter in
    /// signature form (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        match self {
            Self::BmmIndexedContainerType(indexed) => indexed.type_signature(),
            Self::BmmContainerType(data) => {
                let container = &data.container_class.name;
                let item = data.item_type.type_signature();
                format!("{container}<{item}>")
            }
        }
    }

    /// The reduced conformance form of a container type: "the _contained_ type
    /// for a container type (e.g. `ELEMENT` from the type `List<ELEMENT>`)"
    /// (`LANG/docs/bmm/master05-core.adoc` §Semantics §Basics; the v3 container
    /// meta-type keeps `_item_type_` as the contained type,
    /// `org.openehr.lang.bmm3.bmm_container_type.adoc` §Attributes).
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self {
            Self::BmmIndexedContainerType(indexed) => indexed.item_type.conformance_type_name(),
            Self::BmmContainerType(data) => data.item_type.conformance_type_name(),
        }
    }

    /// `BMM_CONTAINER_TYPE.flattened_type_list` (effected):
    /// `Post_result: Result = item_type.flattened_type_list`
    /// (`org.openehr.lang.bmm3.bmm_container_type.adoc` §Functions) — the v3
    /// reading narrows to the item type, unlike the v2 one which also carries
    /// the container class (see [`crate::bmm::core::bmm_type_impl`]).
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmIndexedContainerType(indexed) => indexed.flattened_type_list(),
            Self::BmmContainerType(data) => unique(data.item_type.flattened_type_list()),
        }
    }
}

impl BmmIndexedContainerType {
    /// `BMM_INDEXED_CONTAINER_TYPE.type_name` (effected): "Return full type
    /// name, e.g. `HashMap<String, ELEMENT>`"
    /// (`org.openehr.lang.bmm3.bmm_indexed_container_type.adoc` §Functions).
    #[must_use]
    pub fn type_name(&self) -> String {
        let container = &self.container_class.name;
        let index = self.index_type.type_name();
        let item = self.item_type.type_name();
        format!("{container}<{index},{item}>")
    }

    /// `BMM_TYPE.type_signature` for an indexed container: the same form with
    /// each parameter in signature form
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        let container = &self.container_class.name;
        let index = self.index_type.type_signature();
        let item = self.item_type.type_signature();
        format!("{container}<{index},{item}>")
    }

    /// `BMM_CONTAINER_TYPE.flattened_type_list` for an indexed container: the
    /// item type's flattened list plus the index type, which is itself a
    /// declared `BMM_SIMPLE_TYPE` of the model
    /// (`org.openehr.lang.bmm3.bmm_indexed_container_type.adoc` §Attributes:
    /// `index_type: BMM_SIMPLE_TYPE`).
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        let mut out = self.index_type.flattened_type_list();
        out.extend(self.item_type.flattened_type_list());
        unique(out)
    }
}

impl BmmModelType {
    /// `BMM_TYPE.type_name` for a model type (a type defined by a class of the
    /// model — `org.openehr.lang.bmm3.bmm_model_type.adoc` §Description).
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::BmmGenericType(generic) => generic.type_name(),
            Self::BmmSimpleType(simple) => simple.type_name(),
        }
    }

    /// `BMM_TYPE.type_signature` for a model type.
    #[must_use]
    pub fn type_signature(&self) -> String {
        match self {
            Self::BmmGenericType(generic) => generic.type_signature(),
            Self::BmmSimpleType(simple) => simple.type_signature(),
        }
    }

    /// The reduced conformance form of a model type.
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self {
            Self::BmmGenericType(generic) => generic.conformance_type_name(),
            Self::BmmSimpleType(simple) => simple.conformance_type_name(),
        }
    }

    /// `BMM_TYPE.flattened_type_list` for a model type.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmGenericType(generic) => generic.flattened_type_list(),
            Self::BmmSimpleType(simple) => simple.flattened_type_list(),
        }
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
            Self::BmmGenericType(generic) => generic.type_name(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmSimpleType(simple) => simple.type_name(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_TYPE.type_signature` for an effective type
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        match self {
            Self::BmmGenericType(generic) => generic.type_signature(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmSimpleType(simple) => simple.type_signature(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for an effective type.
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self {
            Self::BmmGenericType(generic) => generic.conformance_type_name(),
            Self::BmmSignature(signature) => signature.conformance_type_name(),
            Self::BmmSimpleType(simple) => simple.conformance_type_name(),
            Self::BmmStatusType(status) => status.conformance_type_name(),
            Self::BmmTupleType(tuple) => tuple.conformance_type_name(),
        }
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for an effective type.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmGenericType(generic) => generic.flattened_type_list(),
            Self::BmmSignature(signature) => signature.flattened_type_list(),
            Self::BmmSimpleType(simple) => simple.flattened_type_list(),
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
            Self::BmmGenericType(generic) => generic.type_name(),
            Self::BmmParameterType(parameter) => parameter.type_name().to_owned(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmSimpleType(simple) => simple.type_name(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_TYPE.type_signature` for a unitary type
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        match self {
            Self::BmmGenericType(generic) => generic.type_signature(),
            Self::BmmParameterType(parameter) => parameter.type_signature(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmSimpleType(simple) => simple.type_signature(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_CLASSIFIER.conformance_type_name` for a unitary type.
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self {
            Self::BmmGenericType(generic) => generic.conformance_type_name(),
            Self::BmmParameterType(parameter) => parameter.conformance_type_name(),
            Self::BmmSignature(signature) => signature.conformance_type_name(),
            Self::BmmSimpleType(simple) => simple.conformance_type_name(),
            Self::BmmStatusType(status) => status.conformance_type_name(),
            Self::BmmTupleType(tuple) => tuple.conformance_type_name(),
        }
    }

    /// `BMM_CLASSIFIER.flattened_type_list` for a unitary type.
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmGenericType(generic) => generic.flattened_type_list(),
            Self::BmmParameterType(parameter) => parameter.flattened_type_list(),
            Self::BmmSignature(signature) => signature.flattened_type_list(),
            Self::BmmSimpleType(simple) => simple.flattened_type_list(),
            Self::BmmStatusType(status) => status.flattened_type_list(),
            Self::BmmTupleType(tuple) => tuple.flattened_type_list(),
        }
    }
}
impl BmmType {
    /// `BMM_TYPE.type_name`: "Formal string form of the type as per UML"
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions), dispatched to the
    /// effective meta-type.
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::BmmContainerType(container) => container.type_name(),
            Self::BmmGenericType(generic) => generic.type_name(),
            Self::BmmParameterType(parameter) => parameter.type_name().to_owned(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmSimpleType(simple) => simple.type_name(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// `BMM_TYPE.type_signature`: "Signature form of the type name, which for
    /// generics includes generic parameter constrainer types E.g.
    /// `Interval<T:Ordered>`. Defaults to the value of `_type_name()_`"
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions).
    #[must_use]
    pub fn type_signature(&self) -> String {
        match self {
            Self::BmmContainerType(container) => container.type_signature(),
            Self::BmmGenericType(generic) => generic.type_signature(),
            Self::BmmParameterType(parameter) => parameter.type_signature(),
            Self::BmmSignature(signature) => signature.type_name(),
            Self::BmmSimpleType(simple) => simple.type_signature(),
            Self::BmmStatusType(status) => status.type_name(),
            Self::BmmTupleType(tuple) => tuple.type_name(),
        }
    }

    /// The reduced conformance form of a v3 type
    /// (`LANG/docs/bmm/master05-core.adoc` §Semantics §Basics, whose reduction
    /// the v3 chapters keep — `master06-core-types.adoc` §Type Conformance).
    #[must_use]
    pub fn conformance_type_name(&self) -> String {
        match self {
            Self::BmmContainerType(container) => container.conformance_type_name(),
            Self::BmmGenericType(generic) => generic.conformance_type_name(),
            Self::BmmParameterType(parameter) => parameter.conformance_type_name(),
            Self::BmmSignature(signature) => signature.conformance_type_name(),
            Self::BmmSimpleType(simple) => simple.conformance_type_name(),
            Self::BmmStatusType(status) => status.conformance_type_name(),
            Self::BmmTupleType(tuple) => tuple.conformance_type_name(),
        }
    }

    /// `BMM_TYPE.flattened_type_list`: "Completely flattened list of type names,
    /// flattening out all generic parameters"
    /// (`org.openehr.lang.bmm3.bmm_type.adoc` §Functions).
    #[must_use]
    pub fn flattened_type_list(&self) -> Vec<String> {
        match self {
            Self::BmmContainerType(container) => container.flattened_type_list(),
            Self::BmmGenericType(generic) => generic.flattened_type_list(),
            Self::BmmParameterType(parameter) => parameter.flattened_type_list(),
            Self::BmmSignature(signature) => signature.flattened_type_list(),
            Self::BmmSimpleType(simple) => simple.flattened_type_list(),
            Self::BmmStatusType(status) => status.flattened_type_list(),
            Self::BmmTupleType(tuple) => tuple.flattened_type_list(),
        }
    }
}
