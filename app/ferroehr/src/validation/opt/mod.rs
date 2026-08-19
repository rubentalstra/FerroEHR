// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

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
//! *flattened* OPT 1.4 tree (`openehr_its::opt14::types::OperationalTemplate`).
//!
//! This module owns the tree walk (T1: the `C_COMPLEX_OBJECT`/`C_ATTRIBUTE`
//! alternation) and the shared context; the per-*kind of check* rules live in
//! sibling modules along the AOM2/08 catalogue's own section axis:
//!
//! - `invariants` — AOM 1.4 constraint-model per-node-kind invariants
//!   (`Existence_set`, `Members_valid`, `Target_path_valid`, VARID/VARDT, VACDF,
//!   VDFAI, STCDC);
//! - `rm_conformance` — VCORM/VCARM/VCAEX/VCACA/VCAM + VACMCO over
//!   `openehr_rm::v1_2::model`;
//! - `primitive` — `C_PRIMITIVE` + temporal/duration patterns + the
//!   `C_DOMAIN_TYPE` assumed-value rules;
//! - `terminology` — VATID/VTTBK/VTCBK/VTLC + code collection;
//! - `interval` — the BASE interval / multiplicity primitives.
//!
//! It does **not** run `valid_value` (that is instance-time — surface B in
//! [`crate::validation`]). Every violation is reported through
//! [`ServiceError::ValidationFailed`] (→ ITS-REST `422`, the `Error` object
//! with the AOM2 rule code in `validationErrors[]`): an AOM2 rule violation is
//! a semantic error on a successfully parsed artefact (the overview status
//! table's `422` row, `docs/specs/openehr/ITS-REST/specifications/docs/
//! overview/Requests_and_responses.md` §HTTP status codes), distinct from the
//! syntactic `400` branch of `responses/400.yaml` that owns not-well-formed
//! content.
//!
//! The check sequence (terminology sets first, then the walk; per node:
//! VCORM → VATID → per-kind invariants → recurse) reports the **first**
//! violation found — the ordering is part of the behavioural contract.

mod interval;
mod invariants;
mod primitive;
mod resource;
mod rm_conformance;
mod terminology;

use std::collections::HashSet;

use openehr_its::opt14::types::{CAttribute, CObject, Intervalofinteger, OperationalTemplate};

use crate::service::error::ServiceError;

/// One artefact-validity violation: the AOM2 rule code + a human detail.
struct RuleViolation {
    code: &'static str,
    detail: String,
}

impl RuleViolation {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// The per-walk validation context: the globally-collected code sets plus
/// whether the artefact declares any `constraint_definitions` at all (which
/// gates VACDF — see `invariants::check_constraint_ref`).
struct Ctx {
    defined_at: HashSet<String>,
    defined_ac: HashSet<String>,
    has_constraint_defs: bool,
}

/// The `C_OBJECT` fields every structural rule interrogates, extracted once
/// per node — the `opt14` `C_OBJECT` family is a closed 13-variant set, so one
/// exhaustive match here replaces a per-field accessor match per rule.
struct NodeView<'a> {
    rm_type: &'a str,
    node_id: &'a str,
    occurrences: &'a Intervalofinteger,
    /// Empty for the leaf kinds (only `C_ARCHETYPE_ROOT` / `C_COMPLEX_OBJECT`
    /// / `T_COMPLEX_OBJECT` carry attribute constraints).
    attributes: &'a [CAttribute],
}

impl<'a> NodeView<'a> {
    fn of(obj: &'a CObject) -> Self {
        match obj {
            CObject::ArchetypeInternalRef(o) => {
                Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences)
            }
            CObject::ArchetypeSlot(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::ConstraintRef(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::CArchetypeRoot(o) => Self {
                rm_type: &o.rm_type_name,
                node_id: &o.node_id,
                occurrences: &o.occurrences,
                attributes: &o.attributes,
            },
            CObject::CCodePhrase(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::CCodeReference(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::CComplexObject(o) => Self {
                rm_type: &o.rm_type_name,
                node_id: &o.node_id,
                occurrences: &o.occurrences,
                attributes: &o.attributes,
            },
            CObject::CDefinedObject(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::CDvOrdinal(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::CDvQuantity(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::CDvState(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::CPrimitiveObject(o) => Self::leaf(&o.rm_type_name, &o.node_id, &o.occurrences),
            CObject::TComplexObject(o) => Self {
                rm_type: &o.rm_type_name,
                node_id: &o.node_id,
                occurrences: &o.occurrences,
                attributes: &o.attributes,
            },
        }
    }

    fn leaf(rm_type: &'a str, node_id: &'a str, occurrences: &'a Intervalofinteger) -> Self {
        Self {
            rm_type,
            node_id,
            occurrences,
            attributes: &[],
        }
    }
}

/// The child objects constrained under a `C_ATTRIBUTE` (single- and
/// multiple-valued alike).
fn attribute_children(attr: &CAttribute) -> &[CObject] {
    match attr {
        CAttribute::CSingleAttribute(a) => &a.children,
        CAttribute::CMultipleAttribute(a) => &a.children,
    }
}

/// Validate an uploaded OPT 1.4 artefact against the AOM2/08 standalone-artefact
/// validity rules. The first violation found is returned as a `422` carrying the
/// AOM2 rule code; a fully valid artefact returns `Ok`.
///
/// # Errors
///
/// [`ServiceError::ValidationFailed`] (→ ITS-REST `422` rendering the `Error`
/// object with the rule code in `validationErrors[]`) for the first violation
/// found: an AOM2 rule violation is a semantic error on a successfully parsed
/// artefact (the overview status table's `422` row,
/// `docs/specs/openehr/ITS-REST/specifications/docs/overview/
/// Requests_and_responses.md` §HTTP status codes; no template operation
/// declares `422`, so the semantic branch is register-adjudicated), never the
/// syntactic `400` branch of `responses/400.yaml`.
pub(super) fn validate_opt_artefact(opt: &OperationalTemplate) -> Result<(), ServiceError> {
    check(opt).map_err(|v| {
        ServiceError::ValidationFailed(vec![openehr_base::validate::InvariantViolation::at(
            v.code.to_string(),
            v.detail,
        )])
    })
}

fn check(opt: &OperationalTemplate) -> Result<(), RuleViolation> {
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

    // The RM resource-package invariants over the OPT's own meta-data header
    // run last, so every existing AOM2 refusal keeps its code (the
    // first-violation ordering is part of the behavioural contract).
    resource::check_resource_meta(opt)?;
    Ok(())
}

// ─── tree walk (T1: C_COMPLEX_OBJECT / C_ATTRIBUTE alternation) ──────────────────

/// Recurse into one constrained attribute of an object whose RM type is
/// `parent_rm`.
fn walk_attribute(attr: &CAttribute, parent_rm: &str, ctx: &Ctx) -> Result<(), RuleViolation> {
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
fn walk_object(obj: &CObject, ctx: &Ctx) -> Result<(), RuleViolation> {
    let view = NodeView::of(obj);

    // VCORM: object constraint type-name existence. A primitive-object node
    // carries a foundation primitive type name (STRING, INTEGER, …) which is
    // intentionally absent from the RM model, so it is exempt.
    if !matches!(obj, CObject::CPrimitiveObject(_)) {
        rm_conformance::check_object_type(view.rm_type, view.node_id)?;
    }

    // VATID: every at-code used as a node_id must be defined in terminology.
    terminology::check_node_id(view.node_id, &ctx.defined_at)?;

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
                primitive::check_primitive(item, view.node_id)?;
            }
        }
        CObject::CCodePhrase(c) => {
            invariants::check_code_list(&c.code_list, view.node_id)?;
            primitive::check_assumed_code(c.assumed_value.as_ref(), &c.code_list, view.node_id)?;
        }
        CObject::CCodeReference(c) => {
            invariants::check_code_list(&c.code_list, view.node_id)?;
            primitive::check_assumed_code(c.assumed_value.as_ref(), &c.code_list, view.node_id)?;
        }
        CObject::CDvOrdinal(c) => primitive::check_dv_ordinal(c, view.node_id)?,
        CObject::CDvQuantity(c) => primitive::check_dv_quantity(c, view.node_id)?,
        _ => {}
    }

    // Recurse into a nested C_ARCHETYPE_ROOT's terminology scope-wise via the
    // global set already collected; structurally we just descend its attributes.
    for attr in view.attributes {
        walk_attribute(attr, view.rm_type, ctx)?;
    }
    Ok(())
}
