//! Hand-written AOM2 `ARCHETYPE_CONSTRAINT` spec functions.
//!
//! Spec source (vendored):
//! `AM/docs/UML/classes/org.openehr.am.aom2.archetype_constraint.adoc`
//! §Attributes + §Functions, with the concrete `is_prohibited` effectings on
//! `org.openehr.am.aom2.c_object.adoc` and `org.openehr.am.aom2.c_attribute.adoc`.

use crate::v2_4::aom2::constraint_model::archetype_constraint::ArchetypeConstraint;
use crate::v2_4::aom2::constraint_model::c_object::CObject;
use crate::v2_4::aom2::constraint_model::c_second_order::CSecondOrder;
use openehr_base::v1_3::foundation_types::interval::multiplicity_interval::MultiplicityInterval;

impl ArchetypeConstraint {
    /// Returns this node's parent, if it has one.
    #[must_use]
    pub fn parent(&self) -> Option<&ArchetypeConstraint> {
        match self {
            Self::ArchetypeSlot(n) => n.parent.as_deref(),
            Self::CAttribute(n) => n.parent.as_deref(),
            Self::CBoolean(n) => n.parent.as_deref(),
            Self::CComplexObject(n) => n.parent(),
            Self::CComplexObjectProxy(n) => n.parent.as_deref(),
            Self::CDate(n) => n.parent.as_deref(),
            Self::CDateTime(n) => n.parent.as_deref(),
            Self::CDuration(n) => n.parent.as_deref(),
            Self::CInteger(n) => n.parent.as_deref(),
            Self::CReal(n) => n.parent.as_deref(),
            Self::CString(n) => n.parent.as_deref(),
            Self::CTerminologyCode(n) => n.parent.as_deref(),
            Self::CTime(n) => n.parent.as_deref(),
        }
    }

    /// Returns this node's second-order constraint parent, if it has one.
    #[must_use]
    pub fn soc_parent(&self) -> Option<&CSecondOrder> {
        match self {
            Self::ArchetypeSlot(n) => n.soc_parent.as_ref(),
            Self::CAttribute(n) => n.soc_parent.as_ref(),
            Self::CBoolean(n) => n.soc_parent.as_ref(),
            Self::CComplexObject(n) => n.soc_parent(),
            Self::CComplexObjectProxy(n) => n.soc_parent.as_ref(),
            Self::CDate(n) => n.soc_parent.as_ref(),
            Self::CDateTime(n) => n.soc_parent.as_ref(),
            Self::CDuration(n) => n.soc_parent.as_ref(),
            Self::CInteger(n) => n.soc_parent.as_ref(),
            Self::CReal(n) => n.soc_parent.as_ref(),
            Self::CString(n) => n.soc_parent.as_ref(),
            Self::CTerminologyCode(n) => n.soc_parent.as_ref(),
            Self::CTime(n) => n.soc_parent.as_ref(),
        }
    }

    /// Returns true if this node is the root of the constraint tree.
    ///
    /// `is_root` (`org.openehr.am.aom2.archetype_constraint.adoc` §Functions):
    /// "True if this node is the root of the tree" — the same page defines
    /// `parent` as present "except in the case of the top of a tree".
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent().is_none()
    }

    /// Returns true if this node has no child nodes.
    ///
    /// `is_leaf` (`org.openehr.am.aom2.archetype_constraint.adoc` §Functions):
    /// "True if this node is a terminal node in the tree structure, i.e. having
    /// no child nodes." The two branching forms are `C_ATTRIBUTE`, whose
    /// children are its `children` objects, and `C_COMPLEX_OBJECT`, whose
    /// children are its `attributes`; every other node type is terminal.
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        match self {
            Self::CAttribute(a) => a.children.as_deref().is_none_or(<[CObject]>::is_empty),
            Self::CComplexObject(o) => o.attributes().is_none_or(<[_]>::is_empty),
            _ => true,
        }
    }

    /// Returns true if a second-order constraint applies at or above this node.
    ///
    /// `is_second_order_constrained`
    /// (`org.openehr.am.aom2.archetype_constraint.adoc` §Functions),
    /// post-condition `soc_parent /= Void or else (parent /= Void and then
    /// parent.is_second_order_constrained)`.
    #[must_use]
    pub fn is_second_order_constrained(&self) -> bool {
        self.soc_parent().is_some()
            || self
                .parent()
                .is_some_and(ArchetypeConstraint::is_second_order_constrained)
    }

    /// Returns true if this node is prohibited.
    ///
    /// `is_prohibited` (`org.openehr.am.aom2.archetype_constraint.adoc`
    /// §Functions) is abstract; it is effected on `C_OBJECT` as `occurrences
    /// /= Void and then occurrences.is_prohibited` and on `C_ATTRIBUTE` as
    /// `existence /= Void and then existence.is_prohibited`.
    #[must_use]
    pub fn is_prohibited(&self) -> bool {
        match self {
            Self::CAttribute(a) => a
                .existence
                .as_ref()
                .is_some_and(MultiplicityInterval::is_prohibited),
            Self::ArchetypeSlot(n) => prohibited(n.occurrences.as_ref()),
            Self::CBoolean(n) => prohibited(n.occurrences.as_ref()),
            Self::CComplexObject(n) => prohibited(n.occurrences()),
            Self::CComplexObjectProxy(n) => prohibited(n.occurrences.as_ref()),
            Self::CDate(n) => prohibited(n.occurrences.as_ref()),
            Self::CDateTime(n) => prohibited(n.occurrences.as_ref()),
            Self::CDuration(n) => prohibited(n.occurrences.as_ref()),
            Self::CInteger(n) => prohibited(n.occurrences.as_ref()),
            Self::CReal(n) => prohibited(n.occurrences.as_ref()),
            Self::CString(n) => prohibited(n.occurrences.as_ref()),
            Self::CTerminologyCode(n) => prohibited(n.occurrences.as_ref()),
            Self::CTime(n) => prohibited(n.occurrences.as_ref()),
        }
    }

    /// Returns this node's path relative to the root of the archetype.
    ///
    /// `path` (`org.openehr.am.aom2.archetype_constraint.adoc` §Functions):
    /// "Path of this node relative to root of archetype", built by walking the
    /// `parent` chain. Path segments alternate `C_ATTRIBUTE.rm_attribute_name`
    /// and `C_OBJECT.node_id` (`org.openehr.am.aom2.archetype.adoc`
    /// §Functions, `physical_paths`); a node whose `node_id` is empty
    /// contributes no predicate, and the root object contributes nothing at all
    /// because a predicate qualifies the attribute segment before it.
    #[must_use]
    pub fn path(&self) -> String {
        let mut segments: Vec<String> = Vec::new();
        let mut node = Some(self);
        while let Some(current) = node {
            let parent = current.parent();
            match current {
                Self::CAttribute(a) => segments.push(format!("/{}", a.rm_attribute_name)),
                other if parent.is_some() => {
                    if let Some(id) = other.object_node_id().filter(|id| !id.is_empty()) {
                        segments.push(format!("[{id}]"));
                    }
                }
                _ => {}
            }
            node = parent;
        }
        segments.reverse();
        segments.concat()
    }

    /// The `node_id` this node carries when it is a `C_OBJECT`.
    fn object_node_id(&self) -> Option<&str> {
        match self {
            Self::CAttribute(_) => None,
            Self::ArchetypeSlot(n) => Some(&n.node_id),
            Self::CBoolean(n) => Some(&n.node_id),
            Self::CComplexObject(n) => Some(n.node_id()),
            Self::CComplexObjectProxy(n) => Some(&n.node_id),
            Self::CDate(n) => Some(&n.node_id),
            Self::CDateTime(n) => Some(&n.node_id),
            Self::CDuration(n) => Some(&n.node_id),
            Self::CInteger(n) => Some(&n.node_id),
            Self::CReal(n) => Some(&n.node_id),
            Self::CString(n) => Some(&n.node_id),
            Self::CTerminologyCode(n) => Some(&n.node_id),
            Self::CTime(n) => Some(&n.node_id),
        }
    }
}

/// Whether an `occurrences` interval states prohibition (`0..0`).
fn prohibited(occurrences: Option<&MultiplicityInterval>) -> bool {
    occurrences.is_some_and(MultiplicityInterval::is_prohibited)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2_4::aom2::constraint_model::c_attribute::CAttribute;
    use crate::v2_4::aom2::constraint_model::c_attribute_tuple::CAttributeTuple;
    use crate::v2_4::aom2::constraint_model::c_complex_object::{
        CComplexObject, CComplexObjectData,
    };

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

    fn object(
        node_id: &str,
        parent: Option<ArchetypeConstraint>,
        attributes: Option<Vec<CAttribute>>,
        occurrences: Option<MultiplicityInterval>,
    ) -> ArchetypeConstraint {
        ArchetypeConstraint::CComplexObject(Box::new(CComplexObject::CComplexObject(
            CComplexObjectData {
                parent: parent.map(Box::new),
                soc_parent: None,
                rm_type_name: "ELEMENT".to_owned(),
                occurrences,
                node_id: node_id.to_owned(),
                alternative_ids: None,
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                attributes,
                attribute_tuples: None,
            },
        )))
    }

    fn attribute(
        name: &str,
        parent: Option<ArchetypeConstraint>,
        soc_parent: Option<CSecondOrder>,
    ) -> ArchetypeConstraint {
        ArchetypeConstraint::CAttribute(Box::new(CAttribute {
            parent: parent.map(Box::new),
            soc_parent,
            rm_attribute_name: name.to_owned(),
            existence: None,
            children: None,
            differential_path: None,
            cardinality: None,
            is_multiple: true,
        }))
    }

    #[test]
    fn a_node_without_a_parent_is_the_root() {
        assert!(object("id1", None, None, None).is_root());
        assert!(!object("id2", Some(object("id1", None, None, None)), None, None).is_root());
    }

    #[test]
    fn a_node_with_no_children_is_a_leaf() {
        assert!(object("id1", None, None, None).is_leaf());
        assert!(object("id1", None, Some(Vec::new()), None).is_leaf());
        assert!(attribute("items", None, None).is_leaf());
    }

    #[test]
    fn second_order_constraint_is_inherited_up_the_parent_chain() {
        let tuple = CSecondOrder::CAttributeTuple(CAttributeTuple {
            members: None,
            tuples: None,
        });
        let constrained = attribute("items", None, Some(tuple));
        assert!(constrained.is_second_order_constrained());
        let child = object("id2", Some(constrained), None, None);
        assert!(child.is_second_order_constrained());
        assert!(!object("id1", None, None, None).is_second_order_constrained());
    }

    #[test]
    fn prohibition_reads_occurrences_on_objects() {
        assert!(object("id1", None, None, Some(interval(0, 0))).is_prohibited());
        assert!(!object("id1", None, None, Some(interval(0, 1))).is_prohibited());
        assert!(!object("id1", None, None, None).is_prohibited());
    }

    #[test]
    fn the_path_alternates_attribute_names_and_node_ids_from_the_root() {
        let root = object("id1", None, None, None);
        let data = attribute("data", Some(root), None);
        let node = object("id2", Some(data), None, None);
        assert_eq!(node.path(), "/data[id2]");
        assert_eq!(object("id1", None, None, None).path(), "");
    }
}
