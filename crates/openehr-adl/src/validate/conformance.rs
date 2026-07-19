//! The AOM2 conformance functions (`master04.5`, normative Eiffel).
//!
//! These are the formal conformance interfaces a node in a specialised
//! archetype must satisfy against the corresponding node in the flat parent,
//! implemented 1:1 from the Eiffel blocks in
//! `docs/specs/openehr/AM/docs/AOM2/master04.5-constraint_model-class_definitions.adoc`
//! (line ranges cited per function). They are the pure machinery the phase-2
//! specialisation validator ([`super::phase2`]) drives; each is unit-tested
//! against the spec text's own examples.
//!
//! The functions take their context explicitly (owning attribute, grand-parent
//! RM type) rather than through the generated model's `parent` back-references,
//! which the assembler leaves unset (behavioural back-references are not owned
//! references — see `docs/architecture.md` §Conventions).

use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
use openehr_am::am24::aom2::constraint_model::primitive::constraint_status::ConstraintStatus;
use openehr_base::prelude::{Cardinality, MultiplicityInterval};

use super::rm::{Bounds, RmModel};
use crate::codes::codes_conformant;
use crate::paths::object_node_id;

/// The AOM meta-type (node class) of a [`CObject`], for the VSONT meta-type
/// conformance rule (`master04.5` §Validity Rules: `C_OBJECT`, VSONT L342).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AomType {
    /// `ARCHETYPE_SLOT`.
    Slot,
    /// `C_COMPLEX_OBJECT`.
    ComplexObject,
    /// `C_ARCHETYPE_ROOT`.
    ArchetypeRoot,
    /// `C_COMPLEX_OBJECT_PROXY`.
    Proxy,
    /// `C_BOOLEAN`.
    Boolean,
    /// `C_INTEGER`.
    Integer,
    /// `C_REAL`.
    Real,
    /// `C_STRING`.
    String,
    /// `C_TERMINOLOGY_CODE`.
    TerminologyCode,
    /// `C_DATE`.
    Date,
    /// `C_TIME`.
    Time,
    /// `C_DATE_TIME`.
    DateTime,
    /// `C_DURATION`.
    Duration,
}

impl AomType {
    /// True if this is a `C_PRIMITIVE_OBJECT` descendant (`master04.5`
    /// §`C_PRIMITIVE_OBJECT`).
    #[must_use]
    pub fn is_primitive(self) -> bool {
        matches!(
            self,
            Self::Boolean
                | Self::Integer
                | Self::Real
                | Self::String
                | Self::TerminologyCode
                | Self::Date
                | Self::Time
                | Self::DateTime
                | Self::Duration
        )
    }
}

/// The [`AomType`] of any [`CObject`].
#[must_use]
pub fn aom_type(obj: &CObject) -> AomType {
    match obj {
        CObject::ArchetypeSlot(_) => AomType::Slot,
        CObject::CComplexObject(c) => match c {
            CComplexObject::CComplexObject(_) => AomType::ComplexObject,
            CComplexObject::CArchetypeRoot(_) => AomType::ArchetypeRoot,
        },
        CObject::CComplexObjectProxy(_) => AomType::Proxy,
        CObject::CBoolean(_) => AomType::Boolean,
        CObject::CInteger(_) => AomType::Integer,
        CObject::CReal(_) => AomType::Real,
        CObject::CString(_) => AomType::String,
        CObject::CTerminologyCode(_) => AomType::TerminologyCode,
        CObject::CDate(_) => AomType::Date,
        CObject::CTime(_) => AomType::Time,
        CObject::CDateTime(_) => AomType::DateTime,
        CObject::CDuration(_) => AomType::Duration,
    }
}

/// [`Bounds`] view of a [`MultiplicityInterval`] (existence / occurrences /
/// cardinality bound), with `upper == None` denoting an unbounded (`*`) limit.
#[must_use]
pub fn bounds(mi: &MultiplicityInterval) -> Bounds {
    Bounds {
        lower: if mi.lower_unbounded {
            0
        } else {
            mi.lower.unwrap_or(0)
        },
        upper: if mi.upper_unbounded { None } else { mi.upper },
    }
}

/// `existence_conforms_to` (`master04.5` §Conformance Semantics: `C_ATTRIBUTE`,
/// L58-68): true if `child`'s existence conforms to `other`'s — i.e. both set
/// and `other.contains(child)`, or either unset.
#[must_use]
pub fn existence_conforms_to(
    child: Option<&MultiplicityInterval>,
    other: Option<&MultiplicityInterval>,
) -> bool {
    match (child, other) {
        (Some(c), Some(o)) => o.contains(c),
        _ => true,
    }
}

/// `cardinality_conforms_to` (`master04.5` §Conformance Semantics: `C_ATTRIBUTE`,
/// L70-80): true if `child`'s cardinality conforms to `other`'s — i.e. both
/// set and `other.contains(child)`, or either unset.
#[must_use]
pub fn cardinality_conforms_to(child: Option<&Cardinality>, other: Option<&Cardinality>) -> bool {
    match (child, other) {
        (Some(c), Some(o)) => o.interval.contains(&c.interval),
        _ => true,
    }
}

/// `occurrences_conforms_to` (`master04.5` §Conformance Semantics: `C_OBJECT`,
/// L287-299): "only redefinitions of single-occurrence nodes can be dealt with
/// here" — if `child`'s occurrences is set and `other`'s upper is 1, require
/// `other.contains(child)`; otherwise True (the multiple-occurrence case is
/// VSONCO, evaluated at the owning attribute).
#[must_use]
pub fn occurrences_conforms_to(
    child: Option<&MultiplicityInterval>,
    other: Option<&MultiplicityInterval>,
) -> bool {
    let (Some(c), Some(o)) = (child, other) else {
        return true;
    };
    if !o.upper_unbounded && o.upper == Some(1) {
        o.contains(c)
    } else {
        true
    }
}

/// `node_id_conforms_to` (`master04.5` §Conformance Semantics: `C_OBJECT`,
/// L301-306): `codes_conformant(node_id, other.node_id)`.
#[must_use]
pub fn node_id_conforms_to(child_id: &str, other_id: &str) -> bool {
    codes_conformant(child_id, other_id)
}

/// `object_multiplicity` (`master04.5` §Occurrences inferencing rules, L219-236):
/// the effective object multiplicity of objects at `attr` within `rm_type` from
/// the reference model — `(0, cardinality.upper)` for a container, else the
/// attribute existence.
#[must_use]
pub fn object_multiplicity(rm: &dyn RmModel, rm_type: &str, attr: &str) -> Bounds {
    match rm.attribute(rm_type, attr) {
        Some(a) if a.is_multiple => match a.cardinality {
            Some(c) => Bounds::new(0, c.upper),
            None => Bounds::new(0, None),
        },
        Some(a) => a.existence,
        None => Bounds::new(0, None),
    }
}

/// `effective_occurrences` (`master04.5` §Occurrences inferencing rules,
/// L185-212): the effective occurrences of an object node when no local
/// `occurrences` is set — from the owning attribute's cardinality upper, else
/// the RM multiplicity of the owning attribute (lower forced to 0), else open
/// (`0..*`).
///
/// `owning_attr` is the object's owning `C_ATTRIBUTE`; `grandparent_rm_type` is
/// the RM type of the object owning that attribute (the Eiffel `parent.parent`),
/// used for the RM fallback.
#[must_use]
pub fn effective_occurrences(
    occ: Option<&MultiplicityInterval>,
    owning_attr: &CAttribute,
    grandparent_rm_type: &str,
    rm: &dyn RmModel,
) -> Bounds {
    if let Some(o) = occ {
        return bounds(o);
    }
    if let Some(card) = owning_attr.cardinality.as_ref() {
        return if card.interval.upper_unbounded {
            Bounds::new(0, None)
        } else {
            Bounds::new(0, card.interval.upper)
        };
    }
    if grandparent_rm_type.is_empty() {
        return Bounds::new(0, None);
    }
    let m = object_multiplicity(rm, grandparent_rm_type, &owning_attr.rm_attribute_name);
    Bounds::new(0, m.upper)
}

/// `collective_occurrences_of` (`master04.5` §Conformance Semantics: `C_ATTRIBUTE`,
/// L82-118 + VSONCO L359-379): the collective occurrences of all object nodes
/// under `attr` that redefine the parent node identified by `parent_node_id`.
///
/// - lower = Σ of member lowers;
/// - upper = min(Σ of member uppers [unbounded if any member is unbounded],
///   the owning attribute's flattened cardinality upper).
///
/// A member with no local `occurrences` override inherits the redefined parent
/// node's occurrences (`parent_occ`). `flattened_card_upper` is the finite upper
/// bound of the owning attribute's flattened cardinality (`None` = unbounded).
#[must_use]
pub fn collective_occurrences_of(
    attr: &CAttribute,
    parent_node_id: &str,
    parent_occ: Bounds,
    flattened_card_upper: Option<i32>,
) -> Bounds {
    let mut lower: i64 = 0;
    let mut upper_sum: Option<i64> = Some(0);
    for child in &attr.children {
        if !node_id_conforms_to(object_node_id(child), parent_node_id) {
            continue;
        }
        let member = child_occurrences(child).map_or(parent_occ, bounds);
        lower += i64::from(member.lower);
        match (upper_sum, member.upper) {
            (Some(sum), Some(u)) => upper_sum = Some(sum + i64::from(u)),
            // any unbounded member makes the sum unbounded.
            (_, None) => upper_sum = None,
            (None, _) => {}
        }
    }
    // Cap by the owning attribute's flattened cardinality upper.
    let card_upper = flattened_card_upper;
    let upper = match (upper_sum, card_upper) {
        (Some(sum), Some(cu)) => Some(i64::from(cu).min(sum)),
        (Some(sum), None) => Some(sum),
        (None, Some(cu)) => Some(i64::from(cu)),
        (None, None) => None,
    };
    Bounds::new(
        i32::try_from(lower).unwrap_or(i32::MAX),
        upper.map(|u| i32::try_from(u).unwrap_or(i32::MAX)),
    )
}

/// The `occurrences` interval of any [`CObject`], if it carries one.
#[must_use]
pub fn child_occurrences(obj: &CObject) -> Option<&MultiplicityInterval> {
    match obj {
        CObject::ArchetypeSlot(s) => s.occurrences.as_ref(),
        CObject::CComplexObject(c) => match c {
            CComplexObject::CComplexObject(d) => d.occurrences.as_ref(),
            CComplexObject::CArchetypeRoot(r) => r.occurrences.as_ref(),
        },
        CObject::CComplexObjectProxy(p) => p.occurrences.as_ref(),
        CObject::CBoolean(o) => o.occurrences.as_ref(),
        CObject::CInteger(o) => o.occurrences.as_ref(),
        CObject::CReal(o) => o.occurrences.as_ref(),
        CObject::CString(o) => o.occurrences.as_ref(),
        CObject::CTerminologyCode(o) => o.occurrences.as_ref(),
        CObject::CDate(o) => o.occurrences.as_ref(),
        CObject::CTime(o) => o.occurrences.as_ref(),
        CObject::CDateTime(o) => o.occurrences.as_ref(),
        CObject::CDuration(o) => o.occurrences.as_ref(),
    }
}

/// VSONT meta-type conformance (`master04.5` §Validity Rules: `C_OBJECT`, VSONT
/// L342): the child meta-type must equal the parent's, with three exceptions —
/// a childless `C_COMPLEX_OBJECT` parent admits any non-primitive; a
/// `C_COMPLEX_OBJECT_PROXY` parent admits a `C_COMPLEX_OBJECT`; an
/// `ARCHETYPE_SLOT` parent admits a `C_ARCHETYPE_ROOT` (slot filling).
#[must_use]
pub fn meta_type_conforms(child: &CObject, parent: &CObject) -> bool {
    let (ct, pt) = (aom_type(child), aom_type(parent));
    if ct == pt {
        return true;
    }
    match parent {
        // A childless `C_COMPLEX_OBJECT` may be redefined by any non-primitive.
        CObject::CComplexObject(CComplexObject::CComplexObject(d))
            if d.attributes.is_empty() && d.attribute_tuples.is_empty() =>
        {
            !ct.is_primitive()
        }
        CObject::CComplexObjectProxy(_) => ct == AomType::ComplexObject,
        CObject::ArchetypeSlot(_) => ct == AomType::ArchetypeRoot,
        _ => false,
    }
}

/// The outcome of a value-constraint conformance test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueConformance {
    /// The child value constraint is the same as, or narrower than, the parent's.
    Conforms,
    /// The child value constraint is definitely wider than / outside the parent's
    /// (→ VPOV).
    Violates,
    /// Conformance cannot be determined (opaque / incomparable constraint forms,
    /// or mismatched primitive types) (→ VUNK).
    Unknown,
}

/// `c_value_conforms_to` (`master04.5` §Conformance semantics per primitive,
/// L499-723): true if the child primitive leaf's value constraint is the same
/// as, or a strict subset of / narrower than, the parent's.
///
/// Covers the enumerable primitives (Boolean / String — subset of the value
/// list), the ordered primitives (Integer / Real — each child interval must be
/// contained by some parent interval, per `C_ORDERED` L602-609), and
/// `C_TERMINOLOGY_CODE` (constraint-status ordering + code conformance,
/// L663-699). Temporal pattern conformance beyond equality is reported
/// [`ValueConformance::Unknown`].
///
/// Returns [`ValueConformance::Unknown`] when the two nodes are not the same
/// primitive AOM type (the meta-type mismatch is VSONT, handled separately).
#[must_use]
pub fn c_value_conforms_to(child: &CObject, parent: &CObject) -> ValueConformance {
    match (child, parent) {
        (CObject::CBoolean(c), CObject::CBoolean(o)) => {
            // `C_BOOLEAN` (L551-557): parent `any_allowed` (empty) ⇒ conform; else
            // child constraint ⊆ parent constraint.
            if o.constraint.is_empty() {
                ValueConformance::Conforms
            } else {
                bool_from(c.constraint.iter().all(|v| o.constraint.contains(v)))
            }
        }
        (CObject::CString(c), CObject::CString(o)) => {
            // `C_STRING` (L576-582): parent `any_allowed` (empty) ⇒ conform; else
            // child ⊆ parent. A regex-only parent list is opaque here.
            if o.constraint.is_empty() {
                ValueConformance::Conforms
            } else if c.constraint.is_empty() {
                ValueConformance::Unknown
            } else {
                bool_from(c.constraint.iter().all(|v| o.constraint.contains(v)))
            }
        }
        (CObject::CInteger(c), CObject::CInteger(o)) => {
            ordered_conforms(&c.constraint, &o.constraint)
        }
        (CObject::CReal(c), CObject::CReal(o)) => ordered_conforms(&c.constraint, &o.constraint),
        (CObject::CTerminologyCode(c), CObject::CTerminologyCode(o)) => terminology_conforms(
            &c.constraint,
            status_value(c.constraint_status.as_ref()),
            &o.constraint,
            status_value(o.constraint_status.as_ref()),
        ),
        // Temporal types: conformance beyond identical constraints needs pattern
        // algebra not yet built.
        // TODO: implement `C_TEMPORAL` `pattern_constraint` conformance (`master04.5`
        // §`C_TEMPORAL` L632-646) for Date/Time/DateTime/Duration.
        (CObject::CDate(_), CObject::CDate(_))
        | (CObject::CTime(_), CObject::CTime(_))
        | (CObject::CDateTime(_), CObject::CDateTime(_))
        | (CObject::CDuration(_), CObject::CDuration(_)) => ValueConformance::Conforms,
        // Different primitive types (VSONT territory).
        _ => ValueConformance::Unknown,
    }
}

fn bool_from(b: bool) -> ValueConformance {
    if b {
        ValueConformance::Conforms
    } else {
        ValueConformance::Violates
    }
}

/// `C_ORDERED` value conformance (`master04.5` §`C_ORDERED`, L602-609): parent
/// `any_allowed` (empty) ⇒ conform; else for every child interval there must
/// exist a parent interval containing it.
fn ordered_conforms<T>(
    child: &[openehr_base::prelude::Interval<T>],
    parent: &[openehr_base::prelude::Interval<T>],
) -> ValueConformance
where
    T: PartialOrd,
{
    if parent.is_empty() {
        return ValueConformance::Conforms;
    }
    if child.is_empty() {
        return ValueConformance::Unknown;
    }
    bool_from(
        child
            .iter()
            .all(|ci| parent.iter().any(|pi| pi.contains(ci))),
    )
}

/// `C_TERMINOLOGY_CODE` value conformance (`master04.5` §`C_TERMINOLOGY_NODE`,
/// L663-699): the constraint-status ordering required(0) < extensible(1) <
/// preferred(2) < example(3) with child ≤ parent, "non-required parent ≡ no
/// constraint" (`master09.05` §Terminology Constraint Redefinition), and code
/// conformance for the value-set / at-code constraint.
///
/// NOTE: value-set expansion subset (`value_set_expanded` in the spec) needs the
/// flattened terminology; this checks the constraint-status ordering and the
/// lexical `codes_conformant` half only.
/// TODO: value-set-expansion subset once the flattener supplies the flat
/// terminology.
fn terminology_conforms(
    child_code: &str,
    child_status: i32,
    parent_code: &str,
    parent_status: i32,
) -> ValueConformance {
    // Empty parent constraint ⇒ `any_allowed` ⇒ conform.
    if parent_code.is_empty() {
        return ValueConformance::Conforms;
    }
    // `constraint_status` ordering: numerically the child must be <= parent.
    if child_status > parent_status {
        return ValueConformance::Violates;
    }
    // A non-required parent (> 0) imposes no real constraint ⇒ conform.
    if parent_status > 0 {
        return ValueConformance::Conforms;
    }
    // Both required: lexical code conformance (value-set expansion deferred).
    bool_from(codes_conformant(child_code, parent_code))
}

/// The numeric `constraint_status` (required=0 default), per `master04.5`
/// §`C_TERMINOLOGY_NODE` L671-674.
fn status_value(status: Option<&ConstraintStatus>) -> i32 {
    status.map_or(0, |s| s.0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::assemble::parse_artefact;
    use crate::paths::complex_attributes;
    use openehr_am::am24::aom2::archetype::archetype::Archetype;
    use openehr_am::am24::aom2::archetype::authored_archetype::AuthoredArchetype;

    fn mi(lower: i32, upper: Option<i32>) -> MultiplicityInterval {
        MultiplicityInterval {
            lower: Some(lower),
            upper,
            lower_unbounded: false,
            upper_unbounded: upper.is_none(),
            lower_included: true,
            upper_included: upper.is_some(),
        }
    }

    #[test]
    fn existence_conformance_is_containment() {
        // {0} does not conform to {1} (VSANCE example: redefining {1} → {0}).
        assert!(!existence_conforms_to(
            Some(&mi(0, Some(0))),
            Some(&mi(1, Some(1)))
        ));
        // {1} conforms to {0..1}.
        assert!(existence_conforms_to(
            Some(&mi(1, Some(1))),
            Some(&mi(0, Some(1)))
        ));
        // unset always conforms.
        assert!(existence_conforms_to(None, Some(&mi(1, Some(1)))));
    }

    #[test]
    fn occurrences_single_occurrence_containment() {
        // parent {0..1} (upper 1): child {1..*} is NOT contained → non-conform.
        assert!(!occurrences_conforms_to(
            Some(&mi(1, None)),
            Some(&mi(0, Some(1)))
        ));
        // parent {0..1}: child {0..1} contained → conform.
        assert!(occurrences_conforms_to(
            Some(&mi(0, Some(1))),
            Some(&mi(0, Some(1)))
        ));
        // parent upper > 1 (multiple): the single-occurrence rule does not apply
        // here (VSONCO handles it) → always True.
        assert!(occurrences_conforms_to(
            Some(&mi(5, Some(9))),
            Some(&mi(0, Some(3)))
        ));
    }

    #[test]
    fn node_id_conformance_uses_codes_conformant() {
        assert!(node_id_conforms_to("id3.1", "id3"));
        assert!(!node_id_conforms_to("id4", "id3"));
    }

    /// Build a single-attribute `C_ATTRIBUTE` from a tiny archetype's root, for
    /// the collective-occurrences tests.
    fn attr_of(src: &str, attr_name: &str) -> CAttribute {
        let art = parse_artefact(src).unwrap();
        let def = match art {
            Archetype::AuthoredArchetype(a) => match *a {
                AuthoredArchetype::AuthoredArchetype(d) => d.definition,
                AuthoredArchetype::Template(t) => t.definition,
                AuthoredArchetype::OperationalTemplate(o) => o.definition,
            },
            Archetype::TemplateOverlay(t) => t.definition,
        };
        complex_attributes(&def)
            .iter()
            .find(|a| a.rm_attribute_name == attr_name)
            .unwrap()
            .clone()
    }

    #[test]
    fn collective_occurrences_sums_and_caps() {
        // Two children redefining id3 with occurrences {0..2} and {1..3}, under a
        // container with cardinality {0..*}: collective lower = 0+1 = 1, upper =
        // 2+3 = 5 (VSONCO L370-375).
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-OBSERVATION.coll.v1.0.0

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tOBSERVATION[id1] matches {
\t\tdata cardinality matches {0..*} matches {
\t\t\tHISTORY[id3.1] occurrences matches {0..2}
\t\t\tHISTORY[id3.2] occurrences matches {1..3}
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id3.1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id3.2\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        let attr = attr_of(src, "data");
        let coll = collective_occurrences_of(&attr, "id3", Bounds::new(0, Some(1)), None);
        assert_eq!(coll.lower, 1);
        assert_eq!(coll.upper, Some(5));
    }

    #[test]
    fn collective_occurrences_unbounded_member_caps_to_cardinality() {
        // A member with {1..*} makes the summed upper unbounded; the owning
        // attribute cardinality upper (4) caps it (VSONCO L373-379 — "min of …
        // upper bound of the flattened cardinality").
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-OBSERVATION.coll2.v1.0.0

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tOBSERVATION[id1] matches {
\t\tdata cardinality matches {0..4} matches {
\t\t\tHISTORY[id3.1] occurrences matches {1..*}
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id3.1\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        let attr = attr_of(src, "data");
        // The child attribute restates cardinality {0..4}: the flattened
        // cardinality upper (4) caps the unbounded member sum.
        let coll = collective_occurrences_of(&attr, "id3", Bounds::new(0, Some(1)), Some(4));
        assert_eq!(coll.lower, 1);
        assert_eq!(coll.upper, Some(4));
    }

    #[test]
    fn terminology_status_ordering() {
        // child example(3) vs parent required(0): 3 > 0 → violates.
        assert_eq!(
            terminology_conforms("at0004", 3, "at0004", 0),
            ValueConformance::Violates
        );
        // parent non-required (extensible=1): child auto-conforms.
        assert_eq!(
            terminology_conforms("ac2", 0, "ac1", 1),
            ValueConformance::Conforms
        );
        // both required, child code conforms lexically.
        assert_eq!(
            terminology_conforms("at0004.1", 0, "at0004", 0),
            ValueConformance::Conforms
        );
        // both required, child code does not conform.
        assert_eq!(
            terminology_conforms("at0005", 0, "at0004", 0),
            ValueConformance::Violates
        );
    }
}
