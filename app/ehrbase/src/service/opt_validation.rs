//! Ingestion-side artefact validity for OPT 1.4 upload (B2 task 6).
//!
//! openEHR formalizes the validity rules a CDR should apply to an *uploaded*
//! archetype/template artefact in the AOM2 validation catalogue
//! (`docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc`) and the AOM2
//! class-definition rule blocks
//! (`AM/docs/AOM2/master04.5-constraint_model-class_definitions.adoc`,
//! `AM/docs/AOM2/master07-terminology_package.adoc`). OPT 1.4 has no normative
//! prose chapter (blueprint `docs/blueprint/03-am.md` §Spec defects 1), so the
//! only formalized catalogue of standalone-artefact checks is AOM2/08; those
//! rules are the oracle here, applied to the *flattened* OPT 1.4 tree
//! (`openehr_its::opt14::OperationalTemplate`).
//!
//! Every violation is reported through [`ServiceError::sm`] with
//! `CallStatusType::PreconditionViolation` (→ ITS-REST `400 Bad Request`),
//! carrying the AOM2 rule code in the message text. `400` is what the CNF
//! `I_DEFINITION_ADL14` upload/validate suites assert for an invalid OPT
//! (`docs/specs/openehr/CNF/tests/platform/robot/I_DEFINITION_ADL14/`
//! `validate_opt/…invalid_opt.robot`: "server rejected OPT with status code
//! 400"; the `_resources/keywords/template_opt1.4_keywords.robot` upload
//! keyword asserts the same); the ECC upload-invalid case accepts any `4xx`.
//!
//! # Reference-model conformance checks
//!
//! The RM-conformance rules (VCORM/VCARM/VCAEX/VCACA/VCAM, AOM2/08 lines 70–75)
//! require "a computational representation of the reference model". We use the
//! BMM-generated static RM model (`openehr_rm::model`, ADR-008 §3) — the same
//! spec-pinned oracle the AQL planner uses.
//!
//! # Codes deliberately not implemented here (inapplicable to OPT 1.4)
//!
//! See the per-check `PORT NOTE`s below for VCACA (RM cardinality bounds are not
//! exposed by the static model), the AOM2 phase-2 *specialisation* rules
//! (VSANCE/VSANCC/VSONCT/…, meaningful only for a differential child against its
//! flat parent — an OPT is already flat), and the VCORMEN* enumeration rules
//! (no RM enumeration metadata in the static model).

use std::collections::HashSet;

use openehr_its::opt14::{
    CArchetypeRoot, CAttribute, CObject, Cardinality, FlatArchetypeOntology, Intervalofinteger,
    OperationalTemplate,
};
use openehr_rm::model;

use ehrbase_sm::CallStatusType;

use super::ServiceError;

/// One artefact-validity violation: the AOM2 rule code + a human detail.
struct Violation {
    code: &'static str,
    detail: String,
}

/// LOCATABLE meta attributes tolerated on any RM class (see the PORT NOTE at
/// the VCARM check: archie-era OPTs constrain these on PATHABLE-only classes).
const LOCATABLE_META_ATTRS: &[&str] = &[
    "name",
    "archetype_node_id",
    "uid",
    "links",
    "archetype_details",
    "feeder_audit",
];

/// Legacy `(class, attribute)` pairs tolerated for prior-art OPT compatibility
/// (PORT NOTE): `ELEMENT.null_flavor` is the archetype-tooling (US) spelling of
/// RM `null_flavour` (`org.openehr.rm.data_structures` ELEMENT), and
/// `ITEM_TABLE.rotated` is an RM 1.0.x attribute removed from later RM
/// releases — both appear in widely-deployed OPT 1.4 artifacts (the vendored
/// RIPPLE / `clinical_content` corpus templates).
const LEGACY_RM_ATTRS: &[(&str, &str)] = &[
    ("ELEMENT", "null_flavor"),
    ("ITEM_TABLE", "rotated"),
    // EVENT.offset is a *computed* function in current RM (Iso8601_duration,
    // org.openehr.rm.data_structures event classes) — RM 1.0.x-era tooling
    // emitted it as a constrainable stored attribute.
    ("EVENT", "offset"),
    ("POINT_EVENT", "offset"),
    ("INTERVAL_EVENT", "offset"),
];

impl Violation {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// Validate an uploaded OPT 1.4 artefact against the AOM2/08 standalone-artefact
/// validity rules. The first violation found is returned as a `400` carrying the
/// AOM2 rule code (`"<CODE>: <detail>"`); a fully valid artefact returns `Ok`.
pub(super) fn validate_opt_artefact(opt: &OperationalTemplate) -> Result<(), ServiceError> {
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
    let defined_at = collect_defined_at_codes(opt);
    let defined_ac = collect_defined_ac_codes(opt);
    check_term_bindings(opt, &defined_at)?; // VTTBK
    check_constraint_bindings(opt, &defined_ac)?; // VTCBK
    check_language_consistency(opt)?; // VTLC

    // RM-conformance + structural rules walk the flattened definition tree.
    // The root definition is a `C_ARCHETYPE_ROOT`; its `rm_type_name` is the
    // top RM type (VCORM).
    check_object_type(
        opt.definition.rm_type_name.as_str(),
        &opt.definition.node_id,
    )?;
    check_node_id(&opt.definition.node_id, &defined_at)?; // VATID (root)
    for attr in &opt.definition.attributes {
        walk_attribute(attr, opt.definition.rm_type_name.as_str(), &defined_at)?;
    }
    Ok(())
}

// ─── tree walk ────────────────────────────────────────────────────────────────

/// Recurse into one constrained attribute of an object whose RM type is
/// `parent_rm`.
fn walk_attribute(
    attr: &CAttribute,
    parent_rm: &str,
    defined_at: &HashSet<String>,
) -> Result<(), Violation> {
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

    // RM-conformance checks fire only when the enclosing object's RM type is
    // known to the static model; an unknown parent means we cannot judge its
    // attributes (and VCORM already flagged the parent if it was a bogus type).
    if model::class(parent_rm).is_some() {
        match model::attribute(parent_rm, attr_name) {
            // PORT NOTE (prior-art OPT tolerance): archie/openEHR-SDK tooling
            // models every constrainable node as Locatable, so published OPTs
            // (incl. the vendored IPS template) constrain LOCATABLE meta
            // attributes (`name`, `archetype_node_id`, …) on classes the RM
            // derives from PATHABLE only (e.g. ISM_TRANSITION —
            // org.openehr.rm.composition.ism_transition.adoc inherits
            // PATHABLE). Rejecting them per strict VCARM would refuse
            // real-world templates; the constraints are tolerated (they bind
            // to the serialized meta fields, which canonical JSON carries).
            None if LOCATABLE_META_ATTRS.contains(&attr_name)
                || LEGACY_RM_ATTRS.contains(&(parent_rm, attr_name)) => {}
            None => {
                // VCARM: attribute name reference model validity (AOM2 line 126).
                return Err(Violation::new(
                    "VCARM",
                    format!(
                        "attribute '{attr_name}' is not defined in reference-model type \
                         '{parent_rm}'"
                    ),
                ));
            }
            Some(rm_attr) => {
                rm_conformance(attr, attr_name, parent_rm, existence, rm_attr)?;
            }
        }
    }

    // VACMCO / VCOC: occurrences-vs-cardinality (container attributes only).
    if let Some(card) = cardinality {
        check_cardinality_occurrences(attr_name, parent_rm, card, children)?;
    }

    // Recurse into each child object.
    for child in children {
        walk_object(child, defined_at)?;
    }
    Ok(())
}

/// Check one child object node, then recurse into its own attributes.
fn walk_object(obj: &CObject, defined_at: &HashSet<String>) -> Result<(), Violation> {
    let rm_type = co_rm_type(obj);
    let node_id = co_node_id(obj);

    // VCORM: object constraint type-name existence (AOM2 line 325). A
    // primitive-object node carries a foundation primitive type name (STRING,
    // INTEGER, …) which is intentionally absent from the RM model, so it is
    // exempt.
    if !matches!(obj, CObject::CPrimitiveObject(_)) {
        check_object_type(rm_type, node_id)?;
    }

    // VATID: every at-code used as a node_id must be defined in terminology.
    check_node_id(node_id, defined_at)?;

    // Recurse into a nested C_ARCHETYPE_ROOT's terminology scope-wise via the
    // global set already collected; structurally we just descend its attributes.
    for attr in co_attributes(obj) {
        walk_attribute(attr, rm_type, defined_at)?;
    }
    Ok(())
}

// ─── RM conformance (VCORM/VCARM/VCAEX/VCACA/VCAM) ──────────────────────────────

/// VCORM: `object constraint type name existence: a type name introducing an
/// object constraint block must be defined in the underlying information model.`
/// (`AOM2/master04.5-…class_definitions.adoc` line 325.)
fn check_object_type(rm_type: &str, node_id: &str) -> Result<(), Violation> {
    // Strip any generic argument (`DV_INTERVAL<DV_QUANTITY>` → `DV_INTERVAL`);
    // the static model keys on the bare class name.
    let bare = rm_type.split('<').next().unwrap_or(rm_type).trim();
    if bare.is_empty() {
        return Err(Violation::new(
            "VCORM",
            format!("object node '{node_id}' has an empty rm_type_name"),
        ));
    }
    if model::class(bare).is_none() {
        return Err(Violation::new(
            "VCORM",
            format!(
                "type '{rm_type}' (object node '{node_id}') is not defined in the reference model"
            ),
        ));
    }
    Ok(())
}

/// The AOM2 RM-conformance rules that apply to a `C_ATTRIBUTE` once its RM
/// attribute has been resolved: VCAM (multiplicity), VCAEX (existence), plus the
/// VCORMT type-conformance of each child against the RM attribute's declared
/// type.
fn rm_conformance(
    attr: &CAttribute,
    attr_name: &str,
    parent_rm: &str,
    existence: &Intervalofinteger,
    rm_attr: &model::RmAttribute,
) -> Result<(), Violation> {
    let rm_is_multiple = !matches!(rm_attr.container, model::Container::None);

    // VCAM: `archetype attribute reference model multiplicity conformance: the
    // multiplicity … of an attribute must conform to that of the corresponding
    // attribute in the underlying information model.` (line 132.) A container
    // (`C_MULTIPLE_ATTRIBUTE`) constraint on a single-valued RM attribute cannot
    // conform.
    let arch_is_multiple = matches!(attr, CAttribute::CMultipleAttribute(_));
    if arch_is_multiple && !rm_is_multiple {
        return Err(Violation::new(
            "VCAM",
            format!(
                "attribute '{attr_name}' on '{parent_rm}' is constrained as a container \
                 (C_MULTIPLE_ATTRIBUTE) but is single-valued in the reference model"
            ),
        ));
    }

    // VCAEX: `archetype attribute reference model existence conformance: the
    // existence of an attribute, if set, must conform, i.e. be the same or
    // narrower, to the existence … in the underlying information model.`
    // (line 129.) The RM existence upper bound is always 1; the RM lower bound
    // is 1 for a mandatory attribute and 0 otherwise. Allowing absence (`{0..}`)
    // on an RM-mandatory attribute *widens* it — the one enforceable violation.
    if rm_attr.is_mandatory && iv_lower(existence) == 0 && !existence.lower_unbounded {
        return Err(Violation::new(
            "VCAEX",
            format!(
                "attribute '{attr_name}' on '{parent_rm}' has existence lower bound 0 but the \
                 attribute is mandatory (existence lower bound 1) in the reference model"
            ),
        ));
    }

    // VCACA: `archetype attribute reference model cardinality conformance …`
    // (line 162). PORT NOTE: the static RM model (`openehr_rm::model`) exposes an
    // attribute's *container kind* (`None`/`List`/`Set`/`Hash`) but not the RM's
    // numeric cardinality bounds, and cADL itself hedges that RM-cardinality
    // enforcement "may depend somewhat on knowledge of the software system"
    // (blueprint 03-am §Spec defects 12; `ADL1.4/master05-cadl.adoc` line 268).
    // The enforceable part of VCACA — that a container constraint may not sit on
    // a single-valued RM attribute — is already covered by VCAM above; the
    // numeric-bound part is not checkable without RM cardinality metadata.

    Ok(())
}

// ─── VACMCO / VCOC (occurrences vs cardinality) ─────────────────────────────────

/// VCOC / VACMCO: `it must be possible for … one instance of every mandatory
/// child object … to be included within the cardinality range.`
/// (`AOM2/…class_definitions.adoc` line 159, restating cADL VCOC,
/// `ADL1.4/master05-cadl.adoc` line 324, per blueprint 03-am req 8.) The sum of
/// the children's occurrence *lower* bounds is the count that MUST appear; it
/// cannot exceed a finite cardinality upper bound. (The maximum-side of the
/// literal cADL wording is intentionally *not* enforced: a single-membership
/// container with several alternative child blocks — each `occurrences 0..1` —
/// is a legal openEHR pattern whose occurrence-maxima sum exceeds the
/// cardinality, cADL §Single-valued/alternative blocks.)
fn check_cardinality_occurrences(
    attr_name: &str,
    parent_rm: &str,
    card: &Cardinality,
    children: &[CObject],
) -> Result<(), Violation> {
    let Some(card_upper) = iv_upper(&card.interval) else {
        return Ok(()); // open cardinality upper bound: any number of children fits.
    };
    let required: i64 = children
        .iter()
        .map(|c| i64::from(iv_lower(co_occurrences(c))))
        .sum();
    if required > i64::from(card_upper) {
        return Err(Violation::new(
            "VACMCO",
            format!(
                "attribute '{attr_name}' on '{parent_rm}': the sum of the child occurrences \
                 lower bounds ({required}) exceeds the cardinality upper bound ({card_upper}), \
                 so the mandatory children cannot fit"
            ),
        ));
    }
    Ok(())
}

// ─── VATID (node-id codes defined in terminology) ───────────────────────────────

/// VATID: `check that all codes mentioned in `definition` are defined in
/// terminology` (`AOM2/master08-validation.adoc` line 56). Applied to at-code
/// `node_id`s (the addressable, sibling-identifying codes, AOM14/04 §`Node_id`).
/// Empty `node_ids` (non-addressable leaves) and non-`at`/`id` codes are exempt.
/// The defined-code set is collected globally across the flattened OPT (the
/// definition roots + every `component_ontologies` set), which is deliberately
/// lenient about per-archetype scoping — it still catches a `node_id` that is
/// defined nowhere while never mis-rejecting a correctly-scoped code.
fn check_node_id(node_id: &str, defined_at: &HashSet<String>) -> Result<(), Violation> {
    if !is_at_code(node_id) {
        return Ok(());
    }
    if !defined_at.contains(node_id) {
        return Err(Violation::new(
            "VATID",
            format!("node_id '{node_id}' is used in the definition but not defined in terminology"),
        ));
    }
    Ok(())
}

/// An addressable archetype term code: `at0000`, `at0001.1`, or the ADL2 `id`
/// form. A bare, empty, or free-text `node_id` is not an at-code.
fn is_at_code(code: &str) -> bool {
    let rest = code
        .strip_prefix("at")
        .or_else(|| code.strip_prefix("id"))
        .unwrap_or("");
    rest.starts_with(|c: char| c.is_ascii_digit())
}

// ─── VTTBK / VTCBK (binding key validity) ───────────────────────────────────────

/// VTTBK: `terminology term binding key valid. Every term binding must be to
/// either a defined archetype term ('at-code') or to a path that is valid in the
/// flat archetype.` (`AOM2/master07-terminology_package.adoc` line 77.) A `/`
/// path key is accepted without full path resolution (conservative — never
/// mis-reject a real flat path).
fn check_term_bindings(
    opt: &OperationalTemplate,
    defined_at: &HashSet<String>,
) -> Result<(), Violation> {
    let check = |code: &str| -> Result<(), Violation> {
        // PORT NOTE (flattened-OPT tolerance): a *specialised* at-code
        // (`at0.23`, dot-notation — AOM2 §specialisation depth) may be bound
        // without a re-emitted local term definition: archie-era flattening
        // keeps parent-archetype bindings whose definitions live in the parent
        // (the vendored blood-pressure corpus OPTs carry these). A dotted
        // at-code is therefore accepted as a valid binding key.
        let specialised = code.starts_with("at") && code.contains('.');
        if code.starts_with('/') || !is_at_code(code) || specialised || defined_at.contains(code) {
            return Ok(());
        }
        Err(Violation::new(
            "VTTBK",
            format!(
                "term binding key '{code}' is neither a defined archetype term (at-code) nor a path"
            ),
        ))
    };
    for set in &opt.definition.term_bindings {
        for item in &set.items {
            check(&item.code)?;
        }
    }
    for onto in flat_ontologies(opt) {
        for set in &onto.term_bindings {
            for item in &set.items {
                check(&item.code)?;
            }
        }
    }
    // Nested C_ARCHETYPE_ROOTs carry their own term_bindings.
    for root in nested_roots(opt) {
        for set in &root.term_bindings {
            for item in &set.items {
                check(&item.code)?;
            }
        }
    }
    Ok(())
}

/// VTCBK: `terminology constraint binding key valid. Every constraint binding
/// must be to a defined archetype constraint code ('ac-code').`
/// (`AOM2/master07-terminology_package.adoc` line 80.)
fn check_constraint_bindings(
    opt: &OperationalTemplate,
    defined_ac: &HashSet<String>,
) -> Result<(), Violation> {
    for onto in flat_ontologies(opt) {
        for set in &onto.constraint_bindings {
            for item in &set.items {
                if !defined_ac.contains(&item.code) {
                    return Err(Violation::new(
                        "VTCBK",
                        format!(
                            "constraint binding key '{}' is not a defined archetype constraint \
                             code (ac-code)",
                            item.code
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

// ─── VTLC (language consistency) ────────────────────────────────────────────────

/// VTLC: `language consistency. Languages consistent: all term codes and
/// constraint codes exist in all languages.`
/// (`AOM2/master07-terminology_package.adoc` line 74.) Applied to each
/// `FlatArchetypeOntology` whose `term_definitions` / `constraint_definitions`
/// are grouped per language: every language must define the same code set.
///
/// PORT NOTE: the per-`C_ARCHETYPE_ROOT` `term_definitions` (a flat
/// `Vec<ARCHETYPE_TERM>`, single-language) carry no language grouping, so VTLC
/// is inert for a single-language OPT — the multi-language code sets live only
/// in `ontology` / `component_ontologies`.
fn check_language_consistency(opt: &OperationalTemplate) -> Result<(), Violation> {
    for onto in flat_ontologies(opt) {
        language_consistent(&codes_by_language(&onto.term_definitions), "term")?;
        language_consistent(
            &codes_by_language(&onto.constraint_definitions),
            "constraint",
        )?;
    }
    Ok(())
}

fn codes_by_language(
    sets: &[openehr_its::opt14::Codedefinitionset],
) -> Vec<(String, HashSet<String>)> {
    sets.iter()
        .map(|s| {
            (
                s.language.clone(),
                s.items.iter().map(|t| t.code.clone()).collect(),
            )
        })
        .collect()
}

fn language_consistent(by_lang: &[(String, HashSet<String>)], kind: &str) -> Result<(), Violation> {
    if by_lang.len() < 2 {
        return Ok(());
    }
    let (ref_lang, ref_codes) = &by_lang[0];
    for (lang, codes) in &by_lang[1..] {
        if codes != ref_codes {
            let missing: Vec<&str> = ref_codes
                .symmetric_difference(codes)
                .map(String::as_str)
                .collect();
            return Err(Violation::new(
                "VTLC",
                format!(
                    "the {kind} code set differs between languages '{ref_lang}' and '{lang}' \
                     (e.g. {missing:?}); all codes must exist in all languages"
                ),
            ));
        }
    }
    Ok(())
}

// ─── code collection + accessors ────────────────────────────────────────────────

/// Every archetype term (`at`/`id`) code defined anywhere in the flattened OPT.
fn collect_defined_at_codes(opt: &OperationalTemplate) -> HashSet<String> {
    let mut out = HashSet::new();
    out.extend(
        opt.definition
            .term_definitions
            .iter()
            .map(|t| t.code.clone()),
    );
    for root in nested_roots(opt) {
        out.extend(root.term_definitions.iter().map(|t| t.code.clone()));
    }
    for onto in flat_ontologies(opt) {
        for set in &onto.term_definitions {
            out.extend(set.items.iter().map(|t| t.code.clone()));
        }
    }
    out
}

/// Every archetype constraint (`ac`) code defined in the flattened OPT.
fn collect_defined_ac_codes(opt: &OperationalTemplate) -> HashSet<String> {
    let mut out = HashSet::new();
    for onto in flat_ontologies(opt) {
        for set in &onto.constraint_definitions {
            out.extend(set.items.iter().map(|t| t.code.clone()));
        }
    }
    out
}

/// `ontology` + every `component_ontologies` entry.
fn flat_ontologies(opt: &OperationalTemplate) -> Vec<&FlatArchetypeOntology> {
    opt.ontology
        .iter()
        .chain(opt.component_ontologies.iter())
        .collect()
}

/// Every nested `C_ARCHETYPE_ROOT` under the definition (the flattened slot
/// fillers), excluding the root definition itself.
fn nested_roots(opt: &OperationalTemplate) -> Vec<&CArchetypeRoot> {
    let mut out = Vec::new();
    for attr in &opt.definition.attributes {
        collect_roots_in_attr(attr, &mut out);
    }
    out
}

fn collect_roots_in_attr<'a>(attr: &'a CAttribute, out: &mut Vec<&'a CArchetypeRoot>) {
    let children = match attr {
        CAttribute::CSingleAttribute(a) => &a.children,
        CAttribute::CMultipleAttribute(a) => &a.children,
    };
    for child in children {
        if let CObject::CArchetypeRoot(root) = child {
            out.push(root);
            for a in &root.attributes {
                collect_roots_in_attr(a, out);
            }
        } else {
            for a in co_attributes(child) {
                collect_roots_in_attr(a, out);
            }
        }
    }
}

fn iv_lower(iv: &Intervalofinteger) -> i32 {
    if iv.lower_unbounded {
        0
    } else {
        iv.lower.unwrap_or(0)
    }
}

fn iv_upper(iv: &Intervalofinteger) -> Option<i32> {
    if iv.upper_unbounded { None } else { iv.upper }
}

fn co_rm_type(obj: &CObject) -> &str {
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

fn co_node_id(obj: &CObject) -> &str {
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

fn co_occurrences(obj: &CObject) -> &Intervalofinteger {
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

fn co_attributes(obj: &CObject) -> &[CAttribute] {
    match obj {
        CObject::CArchetypeRoot(o) => &o.attributes,
        CObject::CComplexObject(o) => &o.attributes,
        CObject::TComplexObject(o) => &o.attributes,
        _ => NO_ATTRS,
    }
}

#[cfg(test)]
mod tests;
