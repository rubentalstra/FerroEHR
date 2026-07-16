//! Surface A1 — OPT 1.4 artefact validity (`I_DEFINITION_ADL14` upload).
//!
//! openEHR formalizes the validity rules a CDR should apply to an *uploaded*
//! archetype/template artefact in the AOM2 validation catalogue
//! (`docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc`) and the AOM2
//! class-definition rule blocks
//! (`AM/docs/AOM2/master04.5-constraint_model-class_definitions.adoc`,
//! `AM/docs/AOM2/master07-terminology_package.adoc`). OPT 1.4 has no normative
//! prose chapter, so the only formalized catalogue of standalone-artefact
//! checks is AOM2/08; those rules are the oracle here, applied to the
//! *flattened* OPT 1.4 tree (`openehr_its::opt14::OperationalTemplate`).
//!
//! This module owns the tree walk (T1: the `C_COMPLEX_OBJECT`/`C_ATTRIBUTE`
//! alternation) and the shared context; the per-*kind of check* rules live in
//! sibling modules along the AOM2/08 catalogue's own section axis:
//!
//! - [`invariants`] — AOM 1.4 constraint-model per-node-kind invariants
//!   (`Existence_set`, `Members_valid`, `Target_path_valid`, VARID/VARDT, VACDF,
//!   VDFAI, STCDC);
//! - [`rm_conformance`] — VCORM/VCARM/VCAEX/VCACA/VCAM + VACMCO over
//!   `openehr_rm::model`;
//! - [`primitive`] — `C_PRIMITIVE` + temporal/duration patterns + the
//!   `C_DOMAIN_TYPE` assumed-value rules;
//! - [`terminology`] — VATID/VTTBK/VTCBK/VTLC + code collection;
//! - [`interval`] — the BASE interval / multiplicity primitives (T20).
//!
//! It does **not** run `valid_value` (that is instance-time — surface B in
//! [`crate::validation`]). Every violation is reported through
//! [`ServiceError::sm`] with `CallStatusType::PreconditionViolation` (→ ITS-REST
//! `400 Bad Request`), carrying the AOM2 rule code in the message. `400` is
//! what the CNF `I_DEFINITION_ADL14` upload/validate suites assert for an
//! invalid OPT (`CNF/tests/platform/robot/I_DEFINITION_ADL14/validate_opt/…`).

mod interval;
mod invariants;
mod primitive;
mod rm_conformance;
mod terminology;

use std::collections::HashSet;

use openehr_its::opt14::{CAttribute, CObject, Intervalofinteger, OperationalTemplate};

use crate::service::CallStatusType;

use crate::service::ServiceError;

/// One artefact-validity violation: the AOM2 rule code + a human detail.
pub(super) struct Violation {
    code: &'static str,
    detail: String,
}

impl Violation {
    pub(super) fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// The per-walk validation context: the globally-collected code sets plus
/// whether the artefact declares any `constraint_definitions` at all (which
/// gates VACDF — see [`invariants::check_constraint_ref`]).
pub(super) struct Ctx {
    pub(super) defined_at: HashSet<String>,
    pub(super) defined_ac: HashSet<String>,
    pub(super) has_constraint_defs: bool,
}

/// Validate an uploaded OPT 1.4 artefact against the AOM2/08 standalone-artefact
/// validity rules. The first violation found is returned as a `400` carrying the
/// AOM2 rule code (`"<CODE>: <detail>"`); a fully valid artefact returns `Ok`.
pub(crate) fn validate_opt_artefact(opt: &OperationalTemplate) -> Result<(), ServiceError> {
    check(opt).map_err(|v| {
        ServiceError::sm(
            CallStatusType::PreconditionViolation,
            format!("{}: {}", v.code, v.detail),
        )
    })
}

fn check(opt: &OperationalTemplate) -> Result<(), Violation> {
    // Terminology-side rules first (cheap; no tree recursion needed beyond code
    // collection).
    let ctx = Ctx {
        defined_at: terminology::collect_defined_at_codes(opt),
        defined_ac: terminology::collect_defined_ac_codes(opt),
        has_constraint_defs: terminology::flat_ontologies(opt)
            .iter()
            .any(|o| o.constraint_definitions.iter().any(|s| !s.items.is_empty())),
    };
    terminology::check_term_bindings(opt, &ctx.defined_at)?; // VTTBK
    terminology::check_constraint_bindings(opt, &ctx.defined_ac)?; // VTCBK
    terminology::check_language_consistency(opt)?; // VTLC

    // RM-conformance + structural rules walk the flattened definition tree.
    // The root definition is a `C_ARCHETYPE_ROOT`; its `rm_type_name` is the
    // top RM type (VCORM).
    rm_conformance::check_object_type(
        opt.definition.rm_type_name.as_str(),
        &opt.definition.node_id,
    )?;
    terminology::check_node_id(&opt.definition.node_id, &ctx.defined_at)?; // VATID (root)
    // VARID / VARDT on the root archetype id (ADL1.4 master08 lines 544/556).
    invariants::check_archetype_id(
        &opt.definition.archetype_id.value,
        opt.definition.rm_type_name.as_str(),
    )?;
    for attr in &opt.definition.attributes {
        walk_attribute(attr, opt.definition.rm_type_name.as_str(), &ctx)?;
    }
    Ok(())
}

// ─── tree walk (T1: C_COMPLEX_OBJECT / C_ATTRIBUTE alternation) ──────────────────

/// Recurse into one constrained attribute of an object whose RM type is
/// `parent_rm`.
fn walk_attribute(attr: &CAttribute, parent_rm: &str, ctx: &Ctx) -> Result<(), Violation> {
    let (attr_name, existence, children, cardinality) = match attr {
        CAttribute::CSingleAttribute(a) => (
            a.rm_attribute_name.as_str(),
            &a.existence,
            &a.children,
            None,
        ),
        CAttribute::CMultipleAttribute(a) => (
            a.rm_attribute_name.as_str(),
            &a.existence,
            &a.children,
            Some(&a.cardinality),
        ),
    };

    invariants::check_attribute_name(attr_name, parent_rm)?;
    invariants::check_existence_set(attr_name, parent_rm, existence)?;

    // A single-valued attribute (no cardinality) can hold at most one value.
    if cardinality.is_none() {
        invariants::check_members_valid(attr_name, parent_rm, children)?;
    }

    // RM-conformance checks (VCARM/VCAM/VCAEX) on the resolved RM attribute.
    rm_conformance::check_attribute(attr, attr_name, parent_rm, existence)?;

    // VACMCO / VCOC: occurrences-vs-cardinality (container attributes only).
    if let Some(card) = cardinality {
        rm_conformance::check_cardinality_occurrences(attr_name, parent_rm, card, children)?;
    }

    // Recurse into each child object.
    for child in children {
        walk_object(child, ctx)?;
    }
    Ok(())
}

/// Check one child object node, then recurse into its own attributes.
fn walk_object(obj: &CObject, ctx: &Ctx) -> Result<(), Violation> {
    let rm_type = co_rm_type(obj);
    let node_id = co_node_id(obj);

    // VCORM: object constraint type-name existence. A primitive-object node
    // carries a foundation primitive type name (STRING, INTEGER, …) which is
    // intentionally absent from the RM model, so it is exempt.
    if !matches!(obj, CObject::CPrimitiveObject(_)) {
        rm_conformance::check_object_type(rm_type, node_id)?;
    }

    // VATID: every at-code used as a node_id must be defined in terminology.
    terminology::check_node_id(node_id, &ctx.defined_at)?;

    // AOM 1.4 per-node-kind invariants (the constraint-model class files).
    match obj {
        CObject::CArchetypeRoot(root) => {
            // VARID / VARDT on every flattened slot-filler root.
            invariants::check_archetype_id(&root.archetype_id.value, &root.rm_type_name)?;
        }
        CObject::ArchetypeSlot(slot) => invariants::check_slot(slot)?,
        CObject::ArchetypeInternalRef(r) => invariants::check_internal_ref(r)?,
        CObject::ConstraintRef(r) => invariants::check_constraint_ref(r, ctx)?,
        CObject::CPrimitiveObject(p) => {
            if let Some(item) = &p.item {
                primitive::check_primitive(item, node_id)?;
            }
        }
        CObject::CCodePhrase(c) => {
            invariants::check_code_list(&c.code_list, node_id)?;
            primitive::check_assumed_code(c.assumed_value.as_ref(), &c.code_list, node_id)?;
        }
        CObject::CCodeReference(c) => {
            invariants::check_code_list(&c.code_list, node_id)?;
            primitive::check_assumed_code(c.assumed_value.as_ref(), &c.code_list, node_id)?;
        }
        CObject::CDvOrdinal(c) => primitive::check_dv_ordinal(c, node_id)?,
        CObject::CDvQuantity(c) => primitive::check_dv_quantity(c, node_id)?,
        _ => {}
    }

    // Recurse into a nested C_ARCHETYPE_ROOT's terminology scope-wise via the
    // global set already collected; structurally we just descend its attributes.
    for attr in co_attributes(obj) {
        walk_attribute(attr, rm_type, ctx)?;
    }
    Ok(())
}

// ─── C_OBJECT accessors (shared across the pass) ─────────────────────────────────

pub(super) fn co_rm_type(obj: &CObject) -> &str {
    match obj {
        CObject::ArchetypeInternalRef(o) => &o.rm_type_name,
        CObject::ArchetypeSlot(o) => &o.rm_type_name,
        CObject::ConstraintRef(o) => &o.rm_type_name,
        CObject::CArchetypeRoot(o) => &o.rm_type_name,
        CObject::CCodePhrase(o) => &o.rm_type_name,
        CObject::CCodeReference(o) => &o.rm_type_name,
        CObject::CComplexObject(o) => &o.rm_type_name,
        CObject::CDefinedObject(o) => &o.rm_type_name,
        CObject::CDvOrdinal(o) => &o.rm_type_name,
        CObject::CDvQuantity(o) => &o.rm_type_name,
        CObject::CDvState(o) => &o.rm_type_name,
        CObject::CPrimitiveObject(o) => &o.rm_type_name,
        CObject::TComplexObject(o) => &o.rm_type_name,
    }
}

pub(super) fn co_node_id(obj: &CObject) -> &str {
    match obj {
        CObject::ArchetypeInternalRef(o) => &o.node_id,
        CObject::ArchetypeSlot(o) => &o.node_id,
        CObject::ConstraintRef(o) => &o.node_id,
        CObject::CArchetypeRoot(o) => &o.node_id,
        CObject::CCodePhrase(o) => &o.node_id,
        CObject::CCodeReference(o) => &o.node_id,
        CObject::CComplexObject(o) => &o.node_id,
        CObject::CDefinedObject(o) => &o.node_id,
        CObject::CDvOrdinal(o) => &o.node_id,
        CObject::CDvQuantity(o) => &o.node_id,
        CObject::CDvState(o) => &o.node_id,
        CObject::CPrimitiveObject(o) => &o.node_id,
        CObject::TComplexObject(o) => &o.node_id,
    }
}

pub(super) fn co_occurrences(obj: &CObject) -> &Intervalofinteger {
    match obj {
        CObject::ArchetypeInternalRef(o) => &o.occurrences,
        CObject::ArchetypeSlot(o) => &o.occurrences,
        CObject::ConstraintRef(o) => &o.occurrences,
        CObject::CArchetypeRoot(o) => &o.occurrences,
        CObject::CCodePhrase(o) => &o.occurrences,
        CObject::CCodeReference(o) => &o.occurrences,
        CObject::CComplexObject(o) => &o.occurrences,
        CObject::CDefinedObject(o) => &o.occurrences,
        CObject::CDvOrdinal(o) => &o.occurrences,
        CObject::CDvQuantity(o) => &o.occurrences,
        CObject::CDvState(o) => &o.occurrences,
        CObject::CPrimitiveObject(o) => &o.occurrences,
        CObject::TComplexObject(o) => &o.occurrences,
    }
}

const NO_ATTRS: &[CAttribute] = &[];

pub(super) fn co_attributes(obj: &CObject) -> &[CAttribute] {
    match obj {
        CObject::CArchetypeRoot(o) => &o.attributes,
        CObject::CComplexObject(o) => &o.attributes,
        CObject::TComplexObject(o) => &o.attributes,
        _ => NO_ATTRS,
    }
}

#[cfg(test)]
mod tests;
