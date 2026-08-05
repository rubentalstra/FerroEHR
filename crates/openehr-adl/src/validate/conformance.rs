//! The AOM2 conformance functions (`master04.5`, normative Eiffel).
//!
//! These are the formal conformance interfaces a node in a specialised
//! archetype must satisfy against the corresponding node in the flat parent,
//! implemented 1:1 from the Eiffel blocks in
//! `docs/specs/openehr/AM/docs/AOM2/master04.5-constraint_model-class_definitions.adoc`
//! (line ranges cited per function). They are the pure machinery the phase-2
//! specialisation validator (`specialisation`) drives; each is unit-tested
//! against the spec text's own examples.
//!
//! The functions take their context explicitly (owning attribute, grand-parent
//! RM type) rather than through the generated model's `parent` back-references,
//! which the assembler leaves unset.

use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_attribute_tuple::CAttributeTuple;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
use openehr_am::am24::aom2::constraint_model::c_primitive_tuple::CPrimitiveTuple;
use openehr_am::am24::aom2::constraint_model::primitive::constraint_status::ConstraintStatus;
use openehr_base::prelude::{Cardinality, MultiplicityInterval};

use super::rm::RmModel;
use crate::aom::access::{AomType, aom_type, child_occurrences, object_node_id};
use crate::aom::build::primitive_to_cobject;
use crate::aom::interval::{Bounds, bounds};
use crate::codes::codes_conformant;
use crate::paths::locate;

/// `existence_conforms_to` (`master04.5` §Conformance Semantics: `C_ATTRIBUTE`,
/// L58-68): true if `child`'s existence conforms to `other`'s — i.e. both set
/// and `other.contains(child)`, or either unset.
#[must_use]
pub(crate) fn existence_conforms_to(
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
pub(crate) fn cardinality_conforms_to(
    child: Option<&Cardinality>,
    other: Option<&Cardinality>,
) -> bool {
    match (child, other) {
        (Some(c), Some(o)) => o.interval.contains(&c.interval),
        _ => true,
    }
}

/// True if `child`'s occurrences conform to `other`'s.
///
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
pub(crate) fn node_id_conforms_to(child_id: &str, other_id: &str) -> bool {
    codes_conformant(child_id, other_id)
}

/// `object_multiplicity` (`master04.5` §Occurrences inferencing rules, L219-236):
/// the effective object multiplicity of objects at `attr` within `rm_type` from
/// the reference model — `(0, cardinality.upper)` for a container, else the
/// attribute existence.
#[must_use]
pub(crate) fn object_multiplicity(rm: &dyn RmModel, rm_type: &str, attr: &str) -> Bounds {
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
pub(crate) fn effective_occurrences(
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
pub(crate) fn collective_occurrences_of(
    attr: &CAttribute,
    parent_node_id: &str,
    parent_occ: Bounds,
    flattened_card_upper: Option<i32>,
) -> Bounds {
    let mut lower: i64 = 0;
    let mut upper_sum: Option<i64> = Some(0);
    for child in attr.children.iter().flatten() {
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

/// The EFFECTIVE `existence` of an attribute in an **ADL 1.4** text: the stated
/// `existence` if there is one, else the 1.4 default `{1..1}`.
///
/// `ADL1.4/master05-cadl.adoc` §Existence L210: "The default existence
/// constraint, if none is shown, is {1..1}." ADL 1.4 states the default in the
/// formalism itself, so it is supplied as an ACCESSOR over the parsed model — the
/// parsed structure is never rewritten, and an absent `existence` stays absent in
/// the AOM (which is what the 1.4→2 converter and the printer must see).
#[must_use]
pub fn effective_existence_adl14(attr: &CAttribute) -> Bounds {
    attr.existence.as_ref().map_or(BOUNDS_ONE, bounds)
}

/// The EFFECTIVE `occurrences` of an object node in an **ADL 1.4** text.
///
/// - A stated `occurrences` wins.
/// - A `use_node` internal reference with none takes the referenced node's:
///   `ADL1.4/master05-cadl.adoc` §Internal References L515 — "Unlike other node
///   types, if no `occurrences` is mentioned, the value of the `occurrences` is
///   set to that of the referenced node (which if not explicitly mentioned will be
///   the default occurrences)". `root` is the archetype's definition root, against
///   which the proxy's target path resolves; a non-specialised 1.4 archetype is
///   its own flat form, so the path resolves locally.
/// - Otherwise the 1.4 default `{1..1}`: `ADL1.4/master05-cadl.adoc`
///   §Occurrences L316 — "The default occurrences, if none is mentioned, is
///   `{1..1}`".
///
/// As with [`effective_existence_adl14`], this is an accessor: nothing is written
/// back into the parsed model.
#[must_use]
pub(crate) fn effective_occurrences_adl14(root: &CComplexObject, obj: &CObject) -> Bounds {
    if let Some(occ) = child_occurrences(obj) {
        return bounds(occ);
    }
    if let CObject::CComplexObjectProxy(proxy) = obj
        && let Some(target) = locate(root, &proxy.target_path)
        && let Some(occ) = child_occurrences(target)
    {
        return bounds(occ);
    }
    BOUNDS_ONE
}

/// The ADL 1.4 default multiplicity `{1..1}`, shared by the existence
/// (`master05` L210) and occurrences (`master05` L316) defaults.
const BOUNDS_ONE: Bounds = Bounds {
    lower: 1,
    upper: Some(1),
};

/// VSONT meta-type conformance (`master04.5` §Validity Rules: `C_OBJECT`, VSONT
/// L342): the child meta-type must equal the parent's, with three exceptions —
/// a childless `C_COMPLEX_OBJECT` parent admits any non-primitive; a
/// `C_COMPLEX_OBJECT_PROXY` parent admits a `C_COMPLEX_OBJECT`; an
/// `ARCHETYPE_SLOT` parent admits a `C_ARCHETYPE_ROOT` (slot filling).
#[must_use]
pub(crate) fn meta_type_conforms(child: &CObject, parent: &CObject) -> bool {
    let (ct, pt) = (aom_type(child), aom_type(parent));
    if ct == pt {
        return true;
    }
    match parent {
        // A childless `C_COMPLEX_OBJECT` may be redefined by any non-primitive.
        CObject::CComplexObject(CComplexObject::CComplexObject(d))
            if d.attributes.as_ref().is_none_or(Vec::is_empty)
                && d.attribute_tuples.as_ref().is_none_or(Vec::is_empty) =>
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
pub(crate) enum ValueConformance {
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
pub(crate) fn c_value_conforms_to(child: &CObject, parent: &CObject) -> ValueConformance {
    match (child, parent) {
        (CObject::CBoolean(c), CObject::CBoolean(o)) => {
            // `C_BOOLEAN` (L551-557): parent `any_allowed` (empty) ⇒ conform; else
            // child constraint ⊆ parent constraint.
            if o.constraint.as_ref().is_none_or(Vec::is_empty) {
                ValueConformance::Conforms
            } else {
                bool_from(
                    c.constraint
                        .iter()
                        .flatten()
                        .all(|v| o.constraint.as_ref().is_some_and(|oc| oc.contains(v))),
                )
            }
        }
        (CObject::CString(c), CObject::CString(o)) => {
            // `C_STRING` (L576-582): parent `any_allowed` (empty) ⇒ conform; else
            // child ⊆ parent. A regex-only parent list is opaque here.
            if o.constraint.as_ref().is_none_or(Vec::is_empty) {
                ValueConformance::Conforms
            } else if c.constraint.as_ref().is_none_or(Vec::is_empty) {
                ValueConformance::Unknown
            } else {
                bool_from(
                    c.constraint
                        .iter()
                        .flatten()
                        .all(|v| o.constraint.as_ref().is_some_and(|oc| oc.contains(v))),
                )
            }
        }
        (CObject::CInteger(c), CObject::CInteger(o)) => ordered_conforms(
            c.constraint.as_deref().unwrap_or_default(),
            o.constraint.as_deref().unwrap_or_default(),
        ),
        (CObject::CReal(c), CObject::CReal(o)) => ordered_conforms(
            c.constraint.as_deref().unwrap_or_default(),
            o.constraint.as_deref().unwrap_or_default(),
        ),
        (CObject::CTerminologyCode(c), CObject::CTerminologyCode(o)) => terminology_conforms(
            &c.constraint,
            status_value(c.constraint_status.as_ref()),
            &o.constraint,
            status_value(o.constraint_status.as_ref()),
        ),
        // Temporal types (`master04.5` §`C_TEMPORAL` L632-639): `c_value_conforms_to`
        // = precursor (`C_ORDERED` interval conformance) AND
        // (`other.pattern_constraint` empty OR
        // `valid_pattern_constraint_replacement(pattern, other.pattern)`).
        (CObject::CDate(c), CObject::CDate(o)) => temporal_conforms(
            c.pattern_constraint.as_deref(),
            c.constraint.as_ref().is_none_or(Vec::is_empty),
            o.pattern_constraint.as_deref(),
            o.constraint.as_ref().is_none_or(Vec::is_empty),
        ),
        (CObject::CTime(c), CObject::CTime(o)) => temporal_conforms(
            c.pattern_constraint.as_deref(),
            c.constraint.as_ref().is_none_or(Vec::is_empty),
            o.pattern_constraint.as_deref(),
            o.constraint.as_ref().is_none_or(Vec::is_empty),
        ),
        (CObject::CDateTime(c), CObject::CDateTime(o)) => temporal_conforms(
            c.pattern_constraint.as_deref(),
            c.constraint.as_ref().is_none_or(Vec::is_empty),
            o.pattern_constraint.as_deref(),
            o.constraint.as_ref().is_none_or(Vec::is_empty),
        ),
        (CObject::CDuration(c), CObject::CDuration(o)) => temporal_conforms(
            c.pattern_constraint.as_deref(),
            c.constraint.as_ref().is_none_or(Vec::is_empty),
            o.pattern_constraint.as_deref(),
            o.constraint.as_ref().is_none_or(Vec::is_empty),
        ),
        // Different primitive types (VSONT territory).
        _ => ValueConformance::Unknown,
    }
}

/// `C_TEMPORAL` value conformance (`master04.5` §`C_TEMPORAL` L632-639).
///
/// The `precursor` (`C_ORDERED`) interval-containment half is only evaluable
/// when both nodes carry an empty interval constraint — a pattern-constrained
/// temporal carries an empty interval list (`master04.5` §`C_DATE` etc.: "For a
/// pattern constraint or no constraint, use an empty list"), and the generated
/// `Iso8601_*` types provide no ordering to compare non-empty interval bounds,
/// so a non-empty interval constraint is reported [`ValueConformance::Unknown`].
/// The pattern half is: parent pattern empty (`any_allowed`) ⇒ conforms; both
/// present ⇒ [`valid_pattern_constraint_replacement`].
fn temporal_conforms(
    child_pattern: Option<&str>,
    child_intervals_empty: bool,
    parent_pattern: Option<&str>,
    parent_intervals_empty: bool,
) -> ValueConformance {
    if !child_intervals_empty || !parent_intervals_empty {
        // Ordered-interval comparison is not available for the Iso8601 types.
        return ValueConformance::Unknown;
    }
    let parent_pat = parent_pattern.filter(|s| !s.is_empty());
    let Some(pp) = parent_pat else {
        // Parent constrains no pattern ⇒ any_allowed ⇒ conforms.
        return ValueConformance::Conforms;
    };
    let Some(cp) = child_pattern.filter(|s| !s.is_empty()) else {
        // Parent constrains a pattern; child states none ⇒ child is not a strict
        // subset ⇒ cannot confirm a valid narrowing.
        return ValueConformance::Unknown;
    };
    bool_from(valid_pattern_constraint_replacement(cp, pp))
}

/// `valid_pattern_constraint_replacement` (`master04.5` §`C_TEMPORAL`): true if
/// the child ISO 8601 constraint pattern is a valid narrowing of the parent
/// pattern.
///
/// NOTE: the AOM2 spec declares the function signature and the allowed-pattern
/// lists but not the replacement algorithm body — this is our own design, read
/// from the ISO 8601 pattern semantics (`master04.5` §`C_TEMPORAL`
/// definitions): the patterns are position-aligned; a field position is
/// mandatory (a letter `Y/M/D/H/m/s`), optional (`?`) or excluded (`X`/`x`), and
/// a valid replacement narrows each position — parent optional (`?`) admits any
/// child (mandatory, optional, or excluded), parent mandatory admits only a
/// mandatory child, parent excluded admits only an excluded child; literal
/// separators must match and the layouts must be the same length.
fn valid_pattern_constraint_replacement(child: &str, parent: &str) -> bool {
    let (cb, pb) = (child.as_bytes(), parent.as_bytes());
    if cb.len() != pb.len() {
        return false;
    }
    cb.iter()
        .zip(pb.iter())
        .all(|(&c, &p)| position_narrows(c, p))
}

/// Field-presence classes of an ISO 8601 temporal-pattern position.
#[derive(PartialEq, Eq, Clone, Copy)]
enum PatternField {
    /// A mandatory field (a letter `Y/M/D/H/m/s`).
    Mandatory,
    /// An optional field (`?`).
    Optional,
    /// An excluded field (`X`/`x`).
    Excluded,
    /// A literal separator (`-`, `:`, `T`, `P`, `W`, `/`, `.`).
    Separator(u8),
}

fn classify(byte: u8) -> PatternField {
    match byte {
        b'?' => PatternField::Optional,
        b'X' | b'x' => PatternField::Excluded,
        b if b.is_ascii_alphabetic() => PatternField::Mandatory,
        b => PatternField::Separator(b),
    }
}

/// True if a child pattern position validly narrows the parent position.
fn position_narrows(child: u8, parent: u8) -> bool {
    match (classify(child), classify(parent)) {
        // Separators must match exactly.
        (PatternField::Separator(c), PatternField::Separator(p)) => c == p,
        (PatternField::Separator(_), _) | (_, PatternField::Separator(_)) => false,
        // Parent optional admits any child field; parent mandatory admits only a
        // mandatory child; parent excluded admits only an excluded child.
        (_, PatternField::Optional)
        | (PatternField::Mandatory, PatternField::Mandatory)
        | (PatternField::Excluded, PatternField::Excluded) => true,
        _ => false,
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
/// NOTE: this terminology-agnostic core checks the constraint-status ordering
/// and the lexical `codes_conformant` half only. The `value_set_expanded` subset
/// half (`master04.5` §`C_TERMINOLOGY_NODE` L683-690) needs the child + flat
/// parent terminologies, which this function does not receive; it is applied in
/// `specialisation` (`check_terminology_leaf`), which has both flattened
/// terminologies. This lexical core is used for the non-specialisation value
/// path (`c_value_conforms_to`), where no value-set expansion is required.
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
    status.map_or(0, |s| s.value())
}

/// The verdict of the second-order tuple conformance functions (`master04.5`
/// §`C_SECOND_ORDER`, L729-804).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TupleConformance {
    /// The child tuple conforms to its counterpart, has no counterpart in the
    /// parent node (a new second-order constraint), or cannot be disproved.
    Conforms,
    /// The child tuple constrains a different member-attribute group than the
    /// counterpart it overlaps, so no row of it can satisfy the
    /// `count = other.count` precondition of `C_PRIMITIVE_TUPLE.c_conforms_to`.
    GroupMismatch,
    /// A child tuple row carries a member count other than the member-attribute
    /// group's — the same precondition, at the row.
    RowArityMismatch,
    /// Comparable, but some child tuple row narrows no parent row —
    /// `C_ATTRIBUTE_TUPLE.c_conforms_to` is False.
    RowViolates,
}

/// The RM attribute names of a tuple's member attributes, in declaration order.
///
/// Each `C_PRIMITIVE_TUPLE` member corresponds positionally to one of these
/// (`master04.5` §`C_PRIMITIVE_TUPLE`).
#[must_use]
pub(crate) fn tuple_member_names(tuple: &CAttributeTuple) -> Vec<&str> {
    tuple
        .members
        .iter()
        .flatten()
        .map(|a| a.rm_attribute_name.as_str())
        .collect()
}

/// `c_conforms_to` for a `C_ATTRIBUTE_TUPLE` against the tuple set of the
/// corresponding flat-parent node (`master04.5` §`C_SECOND_ORDER`, L756-804).
///
/// The counterpart is the parent tuple constraining the same member-attribute
/// group; declaration order may differ, so members are paired by attribute
/// name. A child tuple sharing no attribute with any parent tuple is a new
/// second-order constraint and conforms.
///
/// `C_ATTRIBUTE_TUPLE.c_conforms_to` requires every child row to conform to
/// some parent row (the second `c_congruent_to` disjunct is subsumed: a
/// congruent row conforms); `C_PRIMITIVE_TUPLE.c_conforms_to` requires equal
/// member counts and, position-wise, `same_type` plus member value conformance.
/// A member pair whose value conformance is undecidable
/// ([`ValueConformance::Unknown`]) leaves the row unrefuted.
#[must_use]
pub(crate) fn tuple_conforms_to(
    child: &CAttributeTuple,
    parent_tuples: &[CAttributeTuple],
) -> TupleConformance {
    let names = tuple_member_names(child);
    // The counterpart is the parent tuple over the same group; failing that,
    // one that overlaps it (the group the child then fails to restate).
    let Some(counterpart) = parent_tuples
        .iter()
        .find(|p| member_order(&names, &tuple_member_names(p)).is_some())
        .or_else(|| {
            parent_tuples
                .iter()
                .find(|p| tuple_member_names(p).iter().any(|n| names.contains(n)))
        })
    else {
        return TupleConformance::Conforms;
    };
    let Some(order) = member_order(&names, &tuple_member_names(counterpart)) else {
        return TupleConformance::GroupMismatch;
    };
    // Rows of the counterpart carrying one member per member attribute; a
    // malformed parent tuple leaves the child unrefuted.
    let parent_rows: Vec<&CPrimitiveTuple> = counterpart
        .tuples
        .iter()
        .flatten()
        .filter(|r| r.members.len() == order.len())
        .collect();
    if parent_rows.is_empty() {
        return TupleConformance::Conforms;
    }
    let mut verdict = TupleConformance::Conforms;
    for row in child.tuples.iter().flatten() {
        if row.members.len() != order.len() {
            return TupleConformance::RowArityMismatch;
        }
        let refuted = parent_rows
            .iter()
            .all(|p| row_conforms_to(row, p, &order) == ValueConformance::Violates);
        if refuted {
            verdict = TupleConformance::RowViolates;
        }
    }
    verdict
}

/// The child→parent member position map of two tuple member-attribute lists, or
/// `None` if the two do not constrain the same attribute group.
fn member_order(names: &[&str], parent_names: &[&str]) -> Option<Vec<usize>> {
    if names.len() != parent_names.len() {
        return None;
    }
    names
        .iter()
        .map(|n| parent_names.iter().position(|p| p == n))
        .collect()
}

/// `c_conforms_to` for one `C_PRIMITIVE_TUPLE` row against a parent row
/// (`master04.5` §`C_SECOND_ORDER`, L783-792), with `order` mapping each child
/// member position to the parent row position of the same attribute.
fn row_conforms_to(
    row: &CPrimitiveTuple,
    parent_row: &CPrimitiveTuple,
    order: &[usize],
) -> ValueConformance {
    let mut worst = ValueConformance::Conforms;
    for (i, member) in row.members.iter().enumerate() {
        let Some(other) = order.get(i).and_then(|j| parent_row.members.get(*j)) else {
            return ValueConformance::Violates;
        };
        let (c, p) = (
            primitive_to_cobject(member.clone()),
            primitive_to_cobject(other.clone()),
        );
        // `same_type` (L785): a member of a different primitive type never
        // conforms.
        if aom_type(&c) != aom_type(&p) {
            return ValueConformance::Violates;
        }
        match c_value_conforms_to(&c, &p) {
            ValueConformance::Conforms => {}
            ValueConformance::Violates => return ValueConformance::Violates,
            ValueConformance::Unknown => worst = ValueConformance::Unknown,
        }
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::access::complex_attributes;
    use crate::assemble::parse_artefact;
    use crate::parse::Dialect;
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

    /// Build the definition root of a tiny 1.4 archetype, for the ADL 1.4
    /// effective-value tests.
    fn definition_of_adl14(src: &str) -> CComplexObject {
        let art = parse_artefact(src, Dialect::Adl14).unwrap();
        match art {
            Archetype::AuthoredArchetype(a) => match *a {
                AuthoredArchetype::AuthoredArchetype(d) => d.definition,
                AuthoredArchetype::Template(t) => t.definition,
                AuthoredArchetype::OperationalTemplate(o) => o.definition,
            },
            Archetype::TemplateOverlay(t) => t.definition,
        }
    }

    /// The ADL 1.4 defaults are EFFECTIVE values, applied by the accessor and
    /// never written back: `ADL1.4/master05-cadl.adoc` §Existence L210 ("The
    /// default existence constraint, if none is shown, is {1..1}") and
    /// §Occurrences L316 ("The default occurrences, if none is mentioned, is
    /// `{1..1}`").
    #[test]
    fn adl14_effective_defaults_are_one_to_one() {
        let root = definition_of_adl14(ADL14_USE_NODE_SRC);
        let items = complex_attributes(&root)
            .iter()
            .find(|a| a.rm_attribute_name == "items")
            .unwrap()
            .clone();
        // The attribute states no existence ⇒ effective {1..1} (L210); the parsed
        // structure keeps it absent.
        assert!(items.existence.is_none());
        assert_eq!(effective_existence_adl14(&items), Bounds::new(1, Some(1)));

        // at0001 states no occurrences ⇒ effective {1..1} (L316).
        let plain = &items.children.as_deref().unwrap_or_default()[0];
        assert!(child_occurrences(plain).is_none());
        assert_eq!(
            effective_occurrences_adl14(&root, plain),
            Bounds::new(1, Some(1))
        );
        // at0002 states {0..2} ⇒ that wins.
        assert_eq!(
            effective_occurrences_adl14(&root, &items.children.as_deref().unwrap_or_default()[1]),
            Bounds::new(0, Some(2))
        );
    }

    /// `ADL1.4/master05-cadl.adoc` §Internal References L515: a `use_node` with no
    /// stated `occurrences` takes the REFERENCED node's — here at0002's {0..2},
    /// not the {1..1} default.
    #[test]
    fn adl14_use_node_inherits_the_referenced_node_occurrences() {
        let root = definition_of_adl14(ADL14_USE_NODE_SRC);
        let items = complex_attributes(&root)
            .iter()
            .find(|a| a.rm_attribute_name == "items")
            .unwrap()
            .clone();
        let proxy = &items.children.as_deref().unwrap_or_default()[2];
        assert!(child_occurrences(proxy).is_none());
        assert_eq!(
            effective_occurrences_adl14(&root, proxy),
            Bounds::new(0, Some(2))
        );
    }

    const ADL14_USE_NODE_SRC: &str = "\
archetype (adl_version=1.4)
\topenEHR-EHR-CLUSTER.effective.v1

concept
\t[at0000]

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"AuthorDraft\">

definition
\tCLUSTER[at0000] matches {
\t\titems cardinality matches {0..*; unordered} matches {
\t\t\tELEMENT[at0001] matches {*}
\t\t\tELEMENT[at0002] occurrences matches {0..2} matches {*}
\t\t\tuse_node ELEMENT /items[at0002]
\t\t}
\t}

ontology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\titems = <
\t\t\t\t[\"at0000\"] = <text=<\"\"> description=<\"\">>
\t\t\t\t[\"at0001\"] = <text=<\"\"> description=<\"\">>
\t\t\t\t[\"at0002\"] = <text=<\"\"> description=<\"\">>
\t\t\t>
\t\t>
\t>
";

    #[test]
    fn node_id_conformance_uses_codes_conformant() {
        assert!(node_id_conforms_to("id3.1", "id3"));
        assert!(!node_id_conforms_to("id4", "id3"));
    }

    /// Build a single-attribute `C_ATTRIBUTE` from a tiny archetype's root, for
    /// the collective-occurrences tests.
    fn attr_of(src: &str, attr_name: &str) -> CAttribute {
        let art = parse_artefact(src, Dialect::Adl2).unwrap();
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

    #[test]
    fn temporal_pattern_replacement() {
        // Parent `yyyy-??-??` (month/day optional): child `yyyy-mm-dd` narrows both
        // to mandatory → valid replacement (master04.5 §C_TEMPORAL).
        assert!(valid_pattern_constraint_replacement(
            "yyyy-mm-dd",
            "yyyy-??-??"
        ));
        // Child may also exclude an optional field.
        assert!(valid_pattern_constraint_replacement(
            "yyyy-mm-XX",
            "yyyy-??-??"
        ));
        // Parent mandatory day cannot be loosened to optional.
        assert!(!valid_pattern_constraint_replacement(
            "yyyy-mm-??",
            "yyyy-mm-dd"
        ));
        // Separator / layout mismatch is not a valid replacement.
        assert!(!valid_pattern_constraint_replacement(
            "yyyy-mm",
            "yyyy-mm-dd"
        ));

        // parent pattern empty ⇒ conforms; child interval-constrained ⇒ unknown.
        assert_eq!(
            temporal_conforms(Some("yyyy-mm-dd"), true, None, true),
            ValueConformance::Conforms
        );
        assert_eq!(
            temporal_conforms(Some("yyyy-mm-dd"), true, Some("yyyy-??-??"), true),
            ValueConformance::Conforms
        );
        assert_eq!(
            temporal_conforms(Some("yyyy-mm-??"), true, Some("yyyy-mm-dd"), true),
            ValueConformance::Violates
        );
        // A non-empty interval constraint is not orderable on the Iso8601 types.
        assert_eq!(
            temporal_conforms(Some("yyyy-mm-dd"), false, Some("yyyy-??-??"), true),
            ValueConformance::Unknown
        );
    }
}
